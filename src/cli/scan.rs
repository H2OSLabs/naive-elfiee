//! File scanner for `elf init` and `elf scan`.
//!
//! Scans project directory, respects `.elfignore` + `.gitignore`,
//! creates document blocks for each recognized file.

use crate::engine::{EventPoolWithPath, EventStore};
use crate::models::{Command, Event};
use crate::services;
use crate::state::AppState;
use crate::utils::block_type_inference::infer_block_type;
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::path::Path;

/// 编译内嵌的默认 .elfignore
const DEFAULT_ELFIGNORE: &str = include_str!("../../.elfignore");

/// 扫描到的文件信息
#[derive(Debug, Clone)]
pub struct ScannedFile {
    /// 相对路径（用作 block name），如 "src/main.rs"
    pub relative_path: String,
    /// 文件扩展名（不含 .），如 "rs"
    pub extension: String,
    /// 文本文件的完整内容（二进制文件为 None）
    pub content: Option<String>,
}

/// 解析 .elfignore 内容为 pattern 列表
fn parse_elfignore(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.trim_end_matches('/').to_string())
        .collect()
}

/// 扫描项目目录，返回所有符合条件的文件
///
/// 排除规则：.elfignore + .gitignore + hidden files + .elf/ 目录
pub fn scan_project(project_dir: &Path) -> Result<Vec<ScannedFile>, String> {
    let ignore_patterns = parse_elfignore(DEFAULT_ELFIGNORE);

    let mut builder = WalkBuilder::new(project_dir);
    builder
        .hidden(true) // 排除 . 开头文件/目录
        .git_ignore(true) // 尊重 .gitignore
        .max_depth(Some(100));

    // 添加 .elfignore 规则作为 override
    let mut overrides = OverrideBuilder::new(project_dir);
    for pattern in &ignore_patterns {
        // ignore crate 的 override：! 前缀表示排除
        let _ = overrides.add(&format!("!{}", pattern));
    }
    // 始终排除 .elf/ 目录本身
    let _ = overrides.add("!.elf");

    if let Ok(built) = overrides.build() {
        builder.overrides(built);
    }

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.map_err(|e| format!("Scan error: {}", e))?;

        // 只处理文件（跳过目录）
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let abs_path = entry.path();
        let relative = abs_path
            .strip_prefix(project_dir)
            .map_err(|e| format!("Path error: {}", e))?;

        let ext = relative.extension().and_then(|e| e.to_str()).unwrap_or("");

        // 只处理 .elftypes 中认识的扩展名
        if infer_block_type(ext).is_some() {
            // 尝试读取文本内容，失败则为二进制
            let content = std::fs::read_to_string(abs_path).ok();
            files.push(ScannedFile {
                relative_path: relative.to_string_lossy().to_string(),
                extension: ext.to_string(),
                content,
            });
        }
    }

    // 按路径排序，保证确定性
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(files)
}

/// 为扫描到的文件直接写入 core.create events
///
/// 在 engine spawn 之前调用（与 bootstrap events 同阶段）
pub async fn create_blocks_for_files(
    event_pool: &EventPoolWithPath,
    system_id: &str,
    files: &[ScannedFile],
) -> Result<usize, String> {
    if files.is_empty() {
        return Ok(0);
    }

    // 推算 vector clock 起始值
    let existing = EventStore::get_all_events(&event_pool.pool)
        .await
        .map_err(|e| format!("Failed to read events: {}", e))?;
    let base_count = existing.len() as i64;

    let mut events = Vec::new();
    for (i, file) in files.iter().enumerate() {
        let block_id = uuid::Uuid::new_v4().to_string();
        let block_type =
            infer_block_type(&file.extension).unwrap_or_else(|| "document".to_string());

        let mut ts = HashMap::new();
        ts.insert(system_id.to_string(), base_count + (i as i64) + 1);

        let contents = match &file.content {
            Some(text) => serde_json::json!({
                "source": "linked",
                "format": file.extension,
                "content": text
            }),
            None => serde_json::json!({
                "source": "linked",
                "format": file.extension
            }),
        };

        events.push(Event::new(
            block_id,
            format!("{}/core.create", system_id),
            serde_json::json!({
                "name": file.relative_path,
                "type": block_type,
                "owner": system_id,
                "contents": contents,
                "children": {}
            }),
            ts,
        ));
    }

    let count = events.len();
    EventStore::append_events(&event_pool.pool, &events)
        .await
        .map_err(|e| format!("Failed to create file blocks: {}", e))?;

    Ok(count)
}

/// `elf scan [file]` — 扫描并同步文件内容到 Elfiee
///
/// - 无参数：批量扫描全部文件，新文件创建 block + 写内容，已有文件更新内容
/// - 指定文件：单文件同步，找到 block 则更新，没有则创建
pub async fn run(project: &str, file: Option<&str>) -> Result<(), String> {
    let project_dir = Path::new(project)
        .canonicalize()
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;

    if !project_dir.join(".elf").exists() {
        return Err("Not an Elfiee project (no .elf/ directory). Run `elf init` first.".into());
    }

    // 打开项目 + 启动 engine
    let state = AppState::new();
    let file_id = services::project::open_project(project_dir.to_str().unwrap(), &state).await?;
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or("Engine not running")?;

    let system_id = crate::config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());

    if let Some(file_path) = file {
        // 单文件模式
        run_single_file(&project_dir, file_path, &handle, &system_id).await
    } else {
        // 批量模式
        run_batch(&project_dir, &handle, &system_id).await
    }
}

/// 单文件同步：读取文件内容，找到 block 则更新，没有则创建
async fn run_single_file(
    project_dir: &Path,
    file_path: &str,
    handle: &crate::engine::EngineHandle,
    system_id: &str,
) -> Result<(), String> {
    // 计算相对路径
    let abs_path = project_dir.join(file_path);
    let abs_path = if abs_path.exists() {
        abs_path
            .canonicalize()
            .map_err(|e| format!("Failed to resolve file path: {}", e))?
    } else {
        return Err(format!("File not found: {}", file_path));
    };
    let relative = abs_path
        .strip_prefix(project_dir)
        .map_err(|e| format!("File is not inside project directory: {}", e))?;
    let relative_str = relative.to_string_lossy().to_string();

    // 读取文件内容
    let content = std::fs::read_to_string(&abs_path)
        .map_err(|e| format!("Failed to read file (binary?): {}", e))?;

    let ext = relative.extension().and_then(|e| e.to_str()).unwrap_or("");

    // 查找已有 block
    let existing_blocks = services::block::list_blocks(handle, system_id).await;
    let existing_block = existing_blocks.iter().find(|b| b.name == relative_str);

    if let Some(block) = existing_block {
        // 更新内容
        services::document::write_document(handle, system_id, &block.block_id, &content).await?;
        println!(
            "Synced {} → block {} (updated)",
            relative_str,
            &block.block_id[..8]
        );
    } else {
        // 创建新 block
        let block_type = infer_block_type(ext).unwrap_or_else(|| "document".to_string());
        let cmd = Command::new(
            system_id.to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": relative_str,
                "block_type": block_type,
                "source": "linked",
                "format": ext,
            }),
        );
        let events = services::block::execute_command(handle, cmd).await?;
        let block_id = &events[0].entity;

        // 写入内容
        services::document::write_document(handle, system_id, block_id, &content).await?;
        println!(
            "Synced {} → block {} (created)",
            relative_str,
            &block_id[..8]
        );
    }

    Ok(())
}

/// 批量扫描：新文件创建 block + 写内容，已有文件更新内容
async fn run_batch(
    project_dir: &Path,
    handle: &crate::engine::EngineHandle,
    system_id: &str,
) -> Result<(), String> {
    let files = scan_project(project_dir)?;

    let existing_blocks = services::block::list_blocks(handle, system_id).await;
    let existing_map: HashMap<String, String> = existing_blocks
        .iter()
        .map(|b| (b.name.clone(), b.block_id.clone()))
        .collect();

    let mut created = 0;
    let mut updated = 0;

    for file in &files {
        if let Some(block_id) = existing_map.get(&file.relative_path) {
            // 已有 block — 更新内容（仅文本文件）
            if let Some(ref content) = file.content {
                services::document::write_document(handle, system_id, block_id, content).await?;
                updated += 1;
            }
        } else {
            // 新文件 — 创建 block
            let block_type =
                infer_block_type(&file.extension).unwrap_or_else(|| "document".to_string());
            let cmd = Command::new(
                system_id.to_string(),
                "core.create".to_string(),
                "".to_string(),
                serde_json::json!({
                    "name": file.relative_path,
                    "block_type": block_type,
                    "source": "linked",
                    "format": file.extension,
                }),
            );
            let events = services::block::execute_command(handle, cmd).await?;
            let block_id = &events[0].entity;

            // 写入内容（仅文本文件）
            if let Some(ref content) = file.content {
                services::document::write_document(handle, system_id, block_id, content).await?;
            }
            created += 1;
        }
    }

    println!(
        "Scanned {} files: {} created, {} updated, {} unchanged",
        files.len(),
        created,
        updated,
        files.len() - created - updated
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_elfignore() {
        let content = "# comment\nnode_modules\ntarget/\n\n.venv";
        let patterns = parse_elfignore(content);
        assert_eq!(patterns, vec!["node_modules", "target", ".venv"]);
    }

    #[test]
    fn test_scan_project_basic() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // 创建一些文件
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(root.join("README.md"), "# Hello").unwrap();

        let files = scan_project(root).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.relative_path.as_str()).collect();

        assert!(names.contains(&"src/main.rs"));
        assert!(names.contains(&"README.md"));

        // 验证文件内容被读取
        let main_rs = files
            .iter()
            .find(|f| f.relative_path == "src/main.rs")
            .unwrap();
        assert_eq!(main_rs.content.as_deref(), Some("fn main() {}"));

        let readme = files
            .iter()
            .find(|f| f.relative_path == "README.md")
            .unwrap();
        assert_eq!(readme.content.as_deref(), Some("# Hello"));
    }

    #[test]
    fn test_scan_project_ignores_elf_dir() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join(".elf")).unwrap();
        std::fs::write(root.join(".elf/config.toml"), "").unwrap();
        std::fs::write(root.join("main.rs"), "").unwrap();

        let files = scan_project(root).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.relative_path.as_str()).collect();

        assert!(!names.iter().any(|n| n.contains(".elf")));
        assert!(names.contains(&"main.rs"));
    }

    #[test]
    fn test_scan_project_respects_gitignore() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // 初始化 git（ignore crate 需要 .git 目录才启用 .gitignore）
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join(".gitignore"), "build/\n").unwrap();
        std::fs::create_dir_all(root.join("build")).unwrap();
        std::fs::write(root.join("build/output.js"), "").unwrap();
        std::fs::write(root.join("main.rs"), "").unwrap();

        let files = scan_project(root).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.relative_path.as_str()).collect();

        assert!(names.contains(&"main.rs"));
        assert!(!names.iter().any(|n| n.contains("build")));
    }

    #[tokio::test]
    async fn test_create_blocks_for_files() {
        let event_pool = EventStore::create(":memory:").await.unwrap();

        let files = vec![
            ScannedFile {
                relative_path: "src/main.rs".to_string(),
                extension: "rs".to_string(),
                content: Some("fn main() {}".to_string()),
            },
            ScannedFile {
                relative_path: "README.md".to_string(),
                extension: "md".to_string(),
                content: Some("# Hello".to_string()),
            },
        ];

        let count = create_blocks_for_files(&event_pool, "system", &files)
            .await
            .unwrap();
        assert_eq!(count, 2);

        let events = EventStore::get_all_events(&event_pool.pool).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].value["name"], "src/main.rs");
        assert_eq!(events[0].value["contents"]["content"], "fn main() {}");
        assert_eq!(events[1].value["name"], "README.md");
        assert_eq!(events[1].value["contents"]["content"], "# Hello");
    }

    #[tokio::test]
    async fn test_create_blocks_binary_file_no_content() {
        let event_pool = EventStore::create(":memory:").await.unwrap();

        let files = vec![ScannedFile {
            relative_path: "image.png".to_string(),
            extension: "png".to_string(),
            content: None, // 二进制文件
        }];

        let count = create_blocks_for_files(&event_pool, "system", &files)
            .await
            .unwrap();
        assert_eq!(count, 1);

        let events = EventStore::get_all_events(&event_pool.pool).await.unwrap();
        assert!(events[0].value["contents"].get("content").is_none());
        assert_eq!(events[0].value["contents"]["format"], "png");
    }
}

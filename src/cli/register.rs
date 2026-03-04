//! `elf register` — 注册 Agent（Editor + Grants + MCP 配置 + Skill 注入）
//!
//! 做两件事：
//! 1. **向内**：在 eventstore.db 中创建 Editor + Grants
//! 2. **向外**：注入 MCP 配置 + Skill 到 Agent 配置目录

use crate::elf_project::ElfProject;
use crate::services;
use crate::state::AppState;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Agent 的默认权限集（不含 Owner 专属权限）
const DEFAULT_AGENT_CAPS: &[&str] = &[
    "document.read",
    "document.write",
    "task.read",
    "task.write",
    "task.commit",
    "session.append",
    "session.read",
    "core.create",
    "core.link",
    "core.unlink",
    "core.delete",
];

/// Owner 专属权限（不授予普通 Agent）
const _OWNER_ONLY_CAPS: &[&str] = &[
    "core.grant",
    "core.revoke",
    "editor.create",
    "editor.delete",
];

/// 执行 `elf register`
pub async fn run(
    agent_type: &str,
    name: Option<&str>,
    config_dir: Option<&str>,
    project: &str,
    port: u16,
) -> Result<String, String> {
    run_with_caps(
        agent_type,
        name,
        config_dir,
        project,
        DEFAULT_AGENT_CAPS,
        port,
    )
    .await
}

/// 执行 register，指定自定义权限集
pub async fn run_with_caps(
    agent_type: &str,
    name: Option<&str>,
    config_dir: Option<&str>,
    project: &str,
    capabilities: &[&str],
    port: u16,
) -> Result<String, String> {
    let project_dir = crate::utils::safe_canonicalize(Path::new(project))
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;

    // 打开项目
    let state = AppState::new();
    let file_id = services::project::open_project(project_dir.to_str().unwrap(), &state).await?;

    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or("Engine not running after open")?;

    // 获取 system editor ID
    let system_id = crate::config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());

    // 生成 editor_id
    let editor_id = format!("{}-{}", agent_type, &uuid::Uuid::new_v4().to_string()[..8]);
    let display_name = name.unwrap_or(agent_type);

    // 创建 Editor（走 services）
    services::editor::create_editor(
        &handle,
        &system_id,
        display_name,
        Some("Bot"),
        Some(&editor_id),
    )
    .await
    .map_err(|e| format!("Failed to create editor: {}", e))?;

    // 授予权限（走 services）
    for cap_id in capabilities {
        services::grant::grant_permission(&handle, &system_id, &editor_id, cap_id, "*")
            .await
            .map_err(|e| format!("Failed to grant {}: {}", cap_id, e))?;
    }

    // 推断 Agent 配置目录
    let agent_config_dir = config_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| infer_config_dir(agent_type, &project_dir));

    // 注入 MCP 配置（包含项目路径，支持跨路径 Agent）
    inject_mcp_config(&agent_config_dir, &editor_id, port, &project_dir)?;

    // 注入 Skill
    let elf_project = ElfProject::open(&project_dir)?;
    inject_skill(&agent_config_dir, &elf_project)?;

    println!("Registered {} as editor '{}'", agent_type, editor_id);
    println!("  MCP config -> {}", agent_config_dir.display());
    println!("  Skill -> {}/skills/elfiee/", agent_config_dir.display());

    Ok(editor_id)
}

/// 细粒度 Grant 条目（用于模板中指定 per-block 权限）
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GrantEntry {
    pub capability: String,
    /// Block name、id 或 "*"
    pub block: String,
}

/// 执行 register，指定 wildcard caps + 细粒度 grants
///
/// `capabilities` 对所有 block 生效（wildcard），`grants` 对特定 block 生效
pub async fn run_with_grants(
    agent_type: &str,
    name: Option<&str>,
    config_dir: Option<&str>,
    project: &str,
    capabilities: &[&str],
    grants: &[GrantEntry],
    port: u16,
) -> Result<String, String> {
    let project_dir = crate::utils::safe_canonicalize(Path::new(project))
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;

    // 打开项目
    let state = AppState::new();
    let file_id = services::project::open_project(project_dir.to_str().unwrap(), &state).await?;

    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or("Engine not running after open")?;

    let system_id = crate::config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());

    // 生成 editor_id
    let editor_id = format!("{}-{}", agent_type, &uuid::Uuid::new_v4().to_string()[..8]);
    let display_name = name.unwrap_or(agent_type);

    // 创建 Editor（走 services）
    services::editor::create_editor(
        &handle,
        &system_id,
        display_name,
        Some("Bot"),
        Some(&editor_id),
    )
    .await
    .map_err(|e| format!("Failed to create editor: {}", e))?;

    // 授予 wildcard 权限（走 services）
    for cap_id in capabilities {
        services::grant::grant_permission(&handle, &system_id, &editor_id, cap_id, "*")
            .await
            .map_err(|e| format!("Failed to grant {}: {}", cap_id, e))?;
    }

    // 授予细粒度权限（走 services）
    for grant in grants {
        let block_id = super::resolve::resolve_block_id(&handle, &grant.block).await?;
        services::grant::grant_permission(
            &handle,
            &system_id,
            &editor_id,
            &grant.capability,
            &block_id,
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to grant {} on {}: {}",
                grant.capability, grant.block, e
            )
        })?;
    }

    // 推断 Agent 配置目录
    let agent_config_dir = config_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| infer_config_dir(agent_type, &project_dir));

    // 注入 MCP 配置
    inject_mcp_config(&agent_config_dir, &editor_id, port, &project_dir)?;

    // 注入 Skill
    let elf_project = ElfProject::open(&project_dir)?;
    inject_skill(&agent_config_dir, &elf_project)?;

    println!("Registered {} as editor '{}'", agent_type, editor_id);
    println!("  MCP config -> {}", agent_config_dir.display());
    println!("  Skill -> {}/skills/elfiee/", agent_config_dir.display());
    if !grants.is_empty() {
        println!("  Fine-grained grants: {}", grants.len());
    }

    Ok(editor_id)
}

/// 推断 Agent 配置目录
pub(super) fn infer_config_dir(agent_type: &str, project_dir: &Path) -> PathBuf {
    match agent_type {
        "openclaw" | "claude" => project_dir.join(".claude"),
        _ => project_dir.join(".claude"),
    }
}

/// Elfiee MCP 工具名列表（用于自动注入 permissions）
/// 18 个 tool：4 连接 + 7 块操作 + 4 权限 + 2 历史 + 1 通用执行
const ELFIEE_MCP_TOOLS: &[&str] = &[
    // 连接
    "mcp__elfiee__elfiee_auth",
    "mcp__elfiee__elfiee_open",
    "mcp__elfiee__elfiee_close",
    "mcp__elfiee__elfiee_file_list",
    // 块操作
    "mcp__elfiee__elfiee_block_list",
    "mcp__elfiee__elfiee_block_get",
    "mcp__elfiee__elfiee_block_create",
    "mcp__elfiee__elfiee_block_delete",
    "mcp__elfiee__elfiee_block_rename",
    "mcp__elfiee__elfiee_block_link",
    "mcp__elfiee__elfiee_block_unlink",
    // 权限
    "mcp__elfiee__elfiee_grant",
    "mcp__elfiee__elfiee_revoke",
    "mcp__elfiee__elfiee_editor_create",
    "mcp__elfiee__elfiee_editor_delete",
    // 历史 & 时间旅行
    "mcp__elfiee__elfiee_block_history",
    "mcp__elfiee__elfiee_state_at_event",
    // 通用执行
    "mcp__elfiee__elfiee_exec",
];

/// 注入 MCP 配置到 Agent 的配置目录
///
/// 分三个文件注入：
/// 1. `../.mcp.json` — MCP server 连接配置（Claude Code 从项目根读取）
/// 2. `settings.local.json` — env（ELFIEE_EDITOR_ID, ELFIEE_PROJECT）
/// 3. `settings.local.json` — permissions.allow（追加 Elfiee MCP 工具权限）
fn inject_mcp_config(
    config_dir: &Path,
    editor_id: &str,
    port: u16,
    project_path: &Path,
) -> Result<(), String> {
    std::fs::create_dir_all(config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;

    // 1. 注入 MCP server 到 .mcp.json（config_dir 的父目录，即项目根）
    let mcp_json_path = config_dir.parent().unwrap_or(config_dir).join(".mcp.json");

    let mut mcp_config: serde_json::Value = if mcp_json_path.exists() {
        let content = std::fs::read_to_string(&mcp_json_path)
            .map_err(|e| format!("Failed to read .mcp.json: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse .mcp.json: {}", e))?
    } else {
        json!({})
    };

    let mcp_servers = mcp_config
        .as_object_mut()
        .ok_or(".mcp.json is not an object")?
        .entry("mcpServers")
        .or_insert_with(|| json!({}));

    mcp_servers
        .as_object_mut()
        .ok_or("mcpServers is not an object")?
        .insert(
            "elfiee".to_string(),
            json!({
                "type": "sse",
                "url": format!("http://localhost:{}/sse", port)
            }),
        );

    std::fs::write(
        &mcp_json_path,
        serde_json::to_string_pretty(&mcp_config)
            .map_err(|e| format!("Failed to serialize .mcp.json: {}", e))?,
    )
    .map_err(|e| format!("Failed to write .mcp.json: {}", e))?;

    // 2. 注入 env + permissions 到 settings.local.json
    let settings_path = config_dir.join("settings.local.json");

    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))?
    } else {
        json!({})
    };

    // 清理旧的 mcpServers（如果存在于 settings.local.json 中）
    if let Some(obj) = settings.as_object_mut() {
        obj.remove("mcpServers");
    }

    // 设置环境变量
    let env = settings
        .as_object_mut()
        .ok_or("settings is not an object")?
        .entry("env")
        .or_insert_with(|| json!({}));

    let env_obj = env.as_object_mut().ok_or("env is not an object")?;

    env_obj.insert("ELFIEE_EDITOR_ID".to_string(), json!(editor_id));

    env_obj.insert(
        "ELFIEE_PROJECT".to_string(),
        json!(project_path.to_string_lossy()),
    );

    // 追加 Elfiee MCP 工具权限（不删除已有权限）
    let permissions = settings
        .as_object_mut()
        .unwrap()
        .entry("permissions")
        .or_insert_with(|| json!({}));

    let allow = permissions
        .as_object_mut()
        .ok_or("permissions is not an object")?
        .entry("allow")
        .or_insert_with(|| json!([]));

    let allow_arr = allow
        .as_array_mut()
        .ok_or("permissions.allow is not an array")?;

    let existing: std::collections::HashSet<String> = allow_arr
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    for tool in ELFIEE_MCP_TOOLS {
        if !existing.contains(*tool) {
            allow_arr.push(json!(tool));
        }
    }

    // 写回 settings.local.json
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    std::fs::write(&settings_path, content)
        .map_err(|e| format!("Failed to write settings: {}", e))?;

    Ok(())
}

/// 注入 Skill + Scripts 到 Agent 的配置目录
fn inject_skill(config_dir: &Path, elf_project: &ElfProject) -> Result<(), String> {
    let skill_dir = config_dir.join("skills").join("elfiee");
    let scripts_dir = skill_dir.join("scripts");
    std::fs::create_dir_all(&scripts_dir)
        .map_err(|e| format!("Failed to create skill dir: {}", e))?;

    // SKILL.md
    let skill_content = elf_project.read_skill(None);
    std::fs::write(skill_dir.join("SKILL.md"), skill_content)
        .map_err(|e| format!("Failed to write SKILL.md: {}", e))?;

    // scripts/reconcile.sh
    std::fs::write(
        scripts_dir.join("reconcile.sh"),
        crate::elf_project::RECONCILE_SCRIPT,
    )
    .map_err(|e| format!("Failed to write reconcile.sh: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_infer_config_dir() {
        let project = Path::new("/tmp/my-project");
        assert_eq!(
            infer_config_dir("openclaw", project),
            PathBuf::from("/tmp/my-project/.claude")
        );
        assert_eq!(
            infer_config_dir("claude", project),
            PathBuf::from("/tmp/my-project/.claude")
        );
        assert_eq!(
            infer_config_dir("custom", project),
            PathBuf::from("/tmp/my-project/.claude")
        );
    }

    #[test]
    fn test_inject_mcp_config_new() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".claude");
        let project_path = Path::new("/home/user/my-project");

        inject_mcp_config(&config_dir, "test-editor-1234", 47200, project_path).unwrap();

        // 验证 .mcp.json（在 config_dir 的父目录）
        let mcp_json_path = tmp.path().join(".mcp.json");
        assert!(mcp_json_path.exists());
        let mcp_content = std::fs::read_to_string(&mcp_json_path).unwrap();
        let mcp_config: serde_json::Value = serde_json::from_str(&mcp_content).unwrap();
        assert_eq!(
            mcp_config["mcpServers"]["elfiee"]["url"],
            "http://localhost:47200/sse"
        );
        assert_eq!(mcp_config["mcpServers"]["elfiee"]["type"], "sse");

        // 验证 settings.local.json（env + permissions，无 mcpServers）
        let settings_path = config_dir.join("settings.local.json");
        assert!(settings_path.exists());
        let content = std::fs::read_to_string(&settings_path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert!(settings.get("mcpServers").is_none());
        assert_eq!(settings["env"]["ELFIEE_EDITOR_ID"], "test-editor-1234");
        assert_eq!(settings["env"]["ELFIEE_PROJECT"], "/home/user/my-project");

        // 验证 permissions
        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert!(allow.iter().any(|v| v == "mcp__elfiee__elfiee_auth"));
        assert!(allow.iter().any(|v| v == "mcp__elfiee__elfiee_exec"));
        // 9 个 extension tool 已删除，确认不存在
        assert!(!allow
            .iter()
            .any(|v| v == "mcp__elfiee__elfiee_document_read"));
        assert!(!allow.iter().any(|v| v == "mcp__elfiee__elfiee_task_create"));
        assert_eq!(allow.len(), ELFIEE_MCP_TOOLS.len()); // 18 个
    }

    #[test]
    fn test_inject_mcp_config_merge() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&config_dir).unwrap();
        let project_path = Path::new("/home/user/my-project");

        // 预写已有 .mcp.json
        let existing_mcp = json!({
            "mcpServers": {
                "other-server": { "command": "npx", "args": ["other"] }
            }
        });
        std::fs::write(
            tmp.path().join(".mcp.json"),
            serde_json::to_string_pretty(&existing_mcp).unwrap(),
        )
        .unwrap();

        // 预写已有 settings.local.json（含旧 mcpServers 字段）
        let existing_settings = json!({
            "mcpServers": {
                "stale-elfiee": { "type": "sse" }
            },
            "permissions": {
                "allow": [
                    "Bash(cargo test:*)",
                    "WebSearch"
                ]
            },
            "someOtherKey": true
        });
        std::fs::write(
            config_dir.join("settings.local.json"),
            serde_json::to_string_pretty(&existing_settings).unwrap(),
        )
        .unwrap();

        inject_mcp_config(&config_dir, "test-editor-5678", 47200, project_path).unwrap();

        // 验证 .mcp.json 保留已有 + 新增 elfiee
        let mcp_content = std::fs::read_to_string(tmp.path().join(".mcp.json")).unwrap();
        let mcp_config: serde_json::Value = serde_json::from_str(&mcp_content).unwrap();
        assert!(mcp_config["mcpServers"]["other-server"].is_object());
        assert_eq!(
            mcp_config["mcpServers"]["elfiee"]["url"],
            "http://localhost:47200/sse"
        );

        // 验证 settings.local.json 清理了旧 mcpServers
        let content = std::fs::read_to_string(config_dir.join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(settings.get("mcpServers").is_none());
        assert_eq!(settings["someOtherKey"], true);
        assert_eq!(settings["env"]["ELFIEE_PROJECT"], "/home/user/my-project");

        // 验证权限保留 + 追加
        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert!(allow.iter().any(|v| v == "Bash(cargo test:*)"));
        assert!(allow.iter().any(|v| v == "WebSearch"));
        assert!(allow.iter().any(|v| v == "mcp__elfiee__elfiee_auth"));
        assert_eq!(allow.len(), 2 + ELFIEE_MCP_TOOLS.len());
    }

    #[test]
    fn test_inject_mcp_config_no_duplicate_permissions() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".claude");
        let project_path = Path::new("/home/user/my-project");

        // 注入两次
        inject_mcp_config(&config_dir, "editor-1", 47200, project_path).unwrap();
        inject_mcp_config(&config_dir, "editor-2", 47200, project_path).unwrap();

        // permissions 不重复
        let content = std::fs::read_to_string(config_dir.join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), ELFIEE_MCP_TOOLS.len());

        // editor_id 更新为最新
        assert_eq!(settings["env"]["ELFIEE_EDITOR_ID"], "editor-2");

        // .mcp.json 中 elfiee 只有一个条目
        let mcp_content = std::fs::read_to_string(tmp.path().join(".mcp.json")).unwrap();
        let mcp_config: serde_json::Value = serde_json::from_str(&mcp_content).unwrap();
        assert_eq!(
            mcp_config["mcpServers"].as_object().unwrap().len(),
            1 // 只有 elfiee
        );
    }
}

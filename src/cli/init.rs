//! `elf init` — 初始化 .elf/ 项目目录

use crate::elf_project::ElfProject;
use crate::services;
use std::path::Path;

/// 执行 `elf init`
pub async fn run(project: &str) -> Result<(), String> {
    let project_dir = Path::new(project);

    // 确保项目目录存在
    if !project_dir.exists() {
        std::fs::create_dir_all(project_dir)
            .map_err(|e| format!("Failed to create project directory: {}", e))?;
    }

    let project_dir = project_dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;

    // 检查是否已初始化
    if project_dir.join(".elf").exists() {
        return Err(format!(".elf/ already exists at {}", project_dir.display()));
    }

    // 初始化项目（创建 .elf/ + eventstore.db + config.toml + templates/skills/）
    let elf_project = ElfProject::init(&project_dir).await?;

    // 种子 bootstrap events（system editor + wildcard grants）
    let event_pool = elf_project.event_pool().await?;
    services::project::seed_bootstrap_events(&event_pool).await?;

    // 扫描项目文件 → 为每个文件创建 document block
    let files = super::scan::scan_project(&project_dir)?;
    let system_id = crate::config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());
    let block_count = super::scan::create_blocks_for_files(&event_pool, &system_id, &files).await?;

    println!("Initialized .elf/ in {}", project_dir.display());
    println!("  eventstore.db: created");
    println!("  config.toml: created");
    println!("  templates/skills/default.md: created");
    println!(
        "  {} files scanned, {} blocks created",
        files.len(),
        block_count
    );

    Ok(())
}

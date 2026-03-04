//! `elf status` — 查看项目状态

use crate::elf_project::ElfProject;
use crate::engine::EventStore;
use std::path::Path;

/// 执行 `elf status`
pub async fn run(project: &str) -> Result<(), String> {
    let project_path = Path::new(project);

    if !project_path.join(".elf").exists() {
        return Err(format!(
            "Not an Elfiee project (no .elf/ directory at {})",
            project_path.display()
        ));
    }

    let project_path = project_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;

    let elf_project = ElfProject::open(&project_path)?;
    let event_pool = elf_project.event_pool().await?;
    let events = EventStore::get_all_events(&event_pool.pool)
        .await
        .map_err(|e| format!("Failed to read events: {}", e))?;

    // 统计
    let editors = events
        .iter()
        .filter(|e| e.attribute.contains("editor.create"))
        .count();
    let blocks = events
        .iter()
        .filter(|e| e.attribute.contains("core.create"))
        .count();
    let grants = events
        .iter()
        .filter(|e| e.attribute.contains("core.grant"))
        .count();
    let total = events.len();

    println!("Elfiee Project: {}", project_path.display());
    println!("  Config: {}", elf_project.config().project.name);
    println!();
    println!("  Events:  {}", total);
    println!("  Editors: {}", editors);
    println!("  Blocks:  {}", blocks);
    println!("  Grants:  {}", grants);

    Ok(())
}

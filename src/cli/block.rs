//! `elf block` — Block 管理子命令（list / get）

use crate::services;
use crate::state::AppState;
use std::path::Path;

/// 执行 `elf block list`
pub async fn list(project: &str) -> Result<(), String> {
    let project_dir = crate::utils::safe_canonicalize(Path::new(project))
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;

    if !project_dir.join(".elf").exists() {
        return Err("Not an Elfiee project (no .elf/ directory). Run `elf init` first.".into());
    }

    let state = AppState::new();
    let file_id = services::project::open_project(project_dir.to_str().unwrap(), &state).await?;
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or("Engine not running")?;

    let system_id = crate::config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());

    // 通过 services 层获取 CBAC 过滤后的 blocks
    let blocks = services::block::list_blocks(&handle, &system_id).await;

    if blocks.is_empty() {
        println!("No blocks found.");
        return Ok(());
    }

    // 按 name 排序输出
    let mut sorted: Vec<_> = blocks.iter().collect();
    sorted.sort_by_key(|b| &b.name);

    println!("{:<50} {:<10} {:<36} OWNER", "NAME", "TYPE", "ID");
    println!("{}", "-".repeat(110));

    for block in &sorted {
        println!(
            "{:<50} {:<10} {:<36} {}",
            block.name, block.block_type, block.block_id, block.owner
        );
    }

    println!("\nTotal: {} blocks", sorted.len());

    Ok(())
}

/// 执行 `elf block get <block>`
///
/// 查看单个 block 的详细信息（支持 name 或 id）
pub async fn get(project: &str, block: &str) -> Result<(), String> {
    let project_dir = crate::utils::safe_canonicalize(Path::new(project))
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;

    if !project_dir.join(".elf").exists() {
        return Err("Not an Elfiee project (no .elf/ directory). Run `elf init` first.".into());
    }

    let state = AppState::new();
    let file_id = services::project::open_project(project_dir.to_str().unwrap(), &state).await?;
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or("Engine not running")?;

    let system_id = crate::config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());

    // 解析 block name/id
    let block_id = super::resolve::resolve_block_id(&handle, block).await?;

    let block_data = services::block::get_block(&handle, &system_id, &block_id).await?;

    println!("ID:    {}", block_data.block_id);
    println!("Name:  {}", block_data.name);
    println!("Type:  {}", block_data.block_type);
    println!("Owner: {}", block_data.owner);

    if let Some(desc) = &block_data.description {
        println!("Desc:  {}", desc);
    }

    if !block_data.children.is_empty() {
        println!("\nRelations:");
        for (relation, targets) in &block_data.children {
            for target in targets {
                println!("  {} → {}", relation, target);
            }
        }
    }

    println!("\nContents:");
    println!(
        "{}",
        serde_json::to_string_pretty(&block_data.contents).unwrap_or_else(|_| "{}".to_string())
    );

    Ok(())
}

//! `elf grant` — 授予 Editor 对 Block 的 Capability 权限（通过 services 层）

use crate::services;
use crate::state::AppState;
use std::path::Path;

/// 执行 `elf grant <editor_id> <capability> [block]`
///
/// block 支持 name/id 双模解析，默认 "*"（wildcard）
pub async fn run(
    project: &str,
    editor_id: &str,
    capability: &str,
    block: &str,
) -> Result<(), String> {
    let project_dir = Path::new(project)
        .canonicalize()
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

    // 通过 services 层执行
    services::grant::grant_permission(&handle, &system_id, editor_id, capability, &block_id)
        .await
        .map_err(|e| format!("Failed to grant: {}", e))?;

    println!(
        "Granted {} to editor '{}' on block '{}'",
        capability, editor_id, block
    );

    Ok(())
}

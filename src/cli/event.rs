//! `elf event` — 事件查询子命令（list / history / at）
//!
//! 通过 services 层查询事件，CBAC 自动过滤。

use crate::services;
use crate::state::AppState;
use std::collections::HashMap;
use std::path::Path;

/// 执行 `elf event list`
///
/// 列出所有事件，按 CBAC 过滤（editor events 不过滤，block events 按 read 权限过滤）
pub async fn list(project: &str) -> Result<(), String> {
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

    let events = services::event::list_events(&handle, &system_id).await?;

    if events.is_empty() {
        println!("No events found.");
        return Ok(());
    }

    // 构建 block_id → name 映射
    let blocks = handle.get_all_blocks().await;
    let name_map: HashMap<&str, &str> = blocks
        .iter()
        .map(|(id, b)| (id.as_str(), b.name.as_str()))
        .collect();

    println!(
        "{:<24} {:<20} {:<12} {:<20} EVENT_ID",
        "BLOCK", "CAPABILITY", "EDITOR", "CREATED_AT"
    );
    println!("{}", "-".repeat(90));

    for event in &events {
        let (editor, cap) = parse_attribute(&event.attribute);
        let entity_display = name_map
            .get(event.entity.as_str())
            .copied()
            .unwrap_or_else(|| short_id(&event.entity));

        println!(
            "{:<24} {:<20} {:<12} {:<20} {}",
            truncate(entity_display, 24),
            truncate(cap, 20),
            truncate(editor, 12),
            &event.created_at[..event.created_at.len().min(19)],
            short_id(&event.event_id),
        );
    }

    println!("\nTotal: {} events", events.len());

    Ok(())
}

/// 执行 `elf event history <block>`
///
/// 查询指定 block 的事件历史，支持 name/id 双模解析
pub async fn history(project: &str, block: &str) -> Result<(), String> {
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

    let events = services::event::get_block_history(&handle, &system_id, &block_id).await?;

    if events.is_empty() {
        println!("No events found for block '{}'.", block);
        return Ok(());
    }

    println!("Block: {} ({})", block, short_id(&block_id));
    println!();
    println!(
        "{:<20} {:<12} {:<20} EVENT_ID",
        "CAPABILITY", "EDITOR", "CREATED_AT"
    );
    println!("{}", "-".repeat(70));

    for event in &events {
        let (editor, cap) = parse_attribute(&event.attribute);
        println!(
            "{:<20} {:<12} {:<20} {}",
            truncate(cap, 20),
            truncate(editor, 12),
            &event.created_at[..event.created_at.len().min(19)],
            short_id(&event.event_id),
        );
    }

    println!("\nTotal: {} events", events.len());

    Ok(())
}

/// 执行 `elf event at <block> <event_id>`
///
/// 时间旅行：获取指定 event 时刻的 block 状态快照
pub async fn at(project: &str, block: &str, event_id: &str) -> Result<(), String> {
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

    let (block_state, grants) =
        services::event::get_state_at_event(&handle, &system_id, &block_id, event_id).await?;

    println!("Block state at event {}:", event_id);
    println!();
    println!("  ID:    {}", block_state.block_id);
    println!("  Name:  {}", block_state.name);
    println!("  Type:  {}", block_state.block_type);
    println!("  Owner: {}", block_state.owner);

    if let Some(desc) = &block_state.description {
        println!("  Desc:  {}", desc);
    }

    println!();
    println!("Contents:");
    println!(
        "{}",
        serde_json::to_string_pretty(&block_state.contents).unwrap_or_else(|_| "{}".to_string())
    );

    if !grants.is_empty() {
        println!();
        println!("Grants at this point ({}):", grants.len());
        for grant in &grants {
            println!(
                "  {} — {} on {}",
                grant.editor_id, grant.cap_id, grant.block_id
            );
        }
    }

    Ok(())
}

/// 解析 event attribute → (editor_id, cap_id)
/// 格式: "{editor_id}/{cap_id}"，如 "alice/core.create"
fn parse_attribute(attribute: &str) -> (&str, &str) {
    attribute.split_once('/').unwrap_or(("?", attribute))
}

/// UUID 前 8 位作为短 ID
fn short_id(id: &str) -> &str {
    &id[..id.len().min(8)]
}

/// 截断字符串用于表格对齐
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a very long string", 10), "this is...");
    }

    #[test]
    fn test_parse_attribute() {
        assert_eq!(
            parse_attribute("alice/core.create"),
            ("alice", "core.create")
        );
        assert_eq!(parse_attribute("bob/task.write"), ("bob", "task.write"));
        assert_eq!(parse_attribute("no-slash"), ("?", "no-slash"));
    }

    #[test]
    fn test_short_id() {
        assert_eq!(short_id("a1b2c3d4-e5f6-7890"), "a1b2c3d4");
        assert_eq!(short_id("short"), "short");
    }
}

//! Event 服务 — 事件查询 + 时间旅行 + CBAC

use crate::engine::{EngineHandle, StateProjector};
use crate::models::{Block, Event, Grant};

/// 列出所有事件，CBAC 过滤：
/// - Editor events: 不过滤（项目级信息）
/// - Block events: 按 {block_type}.read 权限过滤
pub async fn list_events(handle: &EngineHandle, editor_id: &str) -> Result<Vec<Event>, String> {
    let all_events = handle.get_all_events().await?;
    let all_blocks = handle.get_all_blocks().await;

    let mut filtered = Vec::new();
    for event in all_events {
        // Editor events 是项目级信息，不需要过滤
        if event.entity.starts_with("editor-") {
            filtered.push(event);
            continue;
        }

        // Wildcard entity (*) 不需要过滤
        if event.entity == "*" {
            filtered.push(event);
            continue;
        }

        // Block events: 检查 {block_type}.read
        let read_cap = all_blocks
            .get(&event.entity)
            .map(|block| format!("{}.read", block.block_type))
            .unwrap_or_else(|| "document.read".to_string());

        let has_read = handle
            .check_grant(editor_id.to_string(), read_cap, event.entity.clone())
            .await;

        if has_read {
            filtered.push(event);
        }
    }

    Ok(filtered)
}

/// 查询 block 的事件历史，CBAC 检查 {block_type}.read
pub async fn get_block_history(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
) -> Result<Vec<Event>, String> {
    // 先检查 CBAC
    let block = handle
        .get_block(block_id.to_string())
        .await
        .ok_or_else(|| format!("Block '{}' not found", block_id))?;

    let read_cap = format!("{}.read", block.block_type);
    let has_permission = handle
        .check_grant(editor_id.to_string(), read_cap, block_id.to_string())
        .await;

    if !has_permission {
        return Err(format!(
            "Permission denied: no {}.read on block",
            block.block_type
        ));
    }

    handle.get_events_by_entity(block_id.to_string()).await
}

/// 时间旅行：获取指定 event 时刻的状态快照
///
/// CBAC 检查 {block_type}.read
pub async fn get_state_at_event(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
    event_id: &str,
) -> Result<(Block, Vec<Grant>), String> {
    // 获取所有事件
    let all_events = handle.get_all_events().await?;

    // 找到目标 event 的索引（支持前缀匹配，如短 ID）
    let target_index = resolve_event_index(&all_events, event_id)?;

    // 创建临时 StateProjector，replay 到目标点
    let mut temp_projector = StateProjector::new();
    temp_projector.replay(all_events[..=target_index].to_vec());

    // 获取 block 快照
    let block = temp_projector
        .get_block(block_id)
        .ok_or_else(|| format!("Block '{}' not found at event '{}'", block_id, event_id))?
        .clone();

    // CBAC: 检查当前用户是否有 read 权限
    let read_cap = format!("{}.read", block.block_type);
    let has_permission = handle
        .check_grant(editor_id.to_string(), read_cap, block_id.to_string())
        .await;

    if !has_permission {
        return Err(format!(
            "Permission denied: no {}.read on block",
            block.block_type
        ));
    }

    // 提取 grants
    let mut grants = Vec::new();
    for (eid, cap_id, target_block) in temp_projector.grants.iter_all() {
        grants.push(Grant::new(
            eid.to_string(),
            cap_id.to_string(),
            target_block.to_string(),
        ));
    }

    Ok((block, grants))
}

/// 按精确 ID 或前缀匹配 event，返回索引
///
/// 解析规则：
/// 1. 精确匹配 → 返回
/// 2. 前缀唯一匹配 → 返回（支持短 ID，如 event_id 前 8 位）
/// 3. 前缀多个匹配 → 报错
/// 4. 无匹配 → 报错
fn resolve_event_index(events: &[Event], input: &str) -> Result<usize, String> {
    // 精确匹配
    if let Some(idx) = events.iter().position(|e| e.event_id == input) {
        return Ok(idx);
    }

    // 前缀匹配
    let matches: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.event_id.starts_with(input))
        .map(|(i, _)| i)
        .collect();

    match matches.len() {
        0 => Err(format!("Event '{}' not found", input)),
        1 => Ok(matches[0]),
        n => Err(format!(
            "Ambiguous event prefix '{}': {} events match. Use a longer prefix.",
            input, n
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::registry::CapabilityRegistry;
    use crate::engine::{EventPoolWithPath, EventStore};
    use crate::models::{Command, Event};
    use std::collections::HashMap;

    async fn seed_test_editor(event_pool: &EventPoolWithPath, editor_id: &str) {
        let registry = CapabilityRegistry::new();
        let cap_ids = registry.get_grantable_cap_ids(&[]);
        let mut events = Vec::new();

        let mut ts = HashMap::new();
        ts.insert(editor_id.to_string(), 1);
        events.push(Event::new(
            editor_id.to_string(),
            format!("{}/editor.create", editor_id),
            serde_json::json!({
                "editor_id": editor_id,
                "name": editor_id,
                "editor_type": "Human"
            }),
            ts,
        ));

        for (i, cap_id) in cap_ids.iter().enumerate() {
            let mut grant_ts = HashMap::new();
            grant_ts.insert(editor_id.to_string(), (i + 2) as i64);
            events.push(Event::new(
                "*".to_string(),
                format!("{}/core.grant", editor_id),
                serde_json::json!({
                    "editor": editor_id,
                    "capability": cap_id,
                    "block": "*"
                }),
                grant_ts,
            ));
        }

        EventStore::append_events(&event_pool.pool, &events)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_list_events_cbac() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        // 创建 block
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "Test", "block_type": "document" }),
        );
        handle.process_command(cmd).await.unwrap();

        // alice 能看到所有事件
        let events = list_events(&handle, "alice").await.unwrap();
        assert!(!events.is_empty());

        // bob 只能看到 editor events 和 wildcard events
        let bob_events = list_events(&handle, "bob").await.unwrap();
        // bob 看不到 block events（没有 document.read）
        let block_events: Vec<_> = bob_events
            .iter()
            .filter(|e| !e.entity.starts_with("editor-") && e.entity != "*")
            .collect();
        assert!(block_events.is_empty());

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_get_block_history_cbac() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "Test", "block_type": "document" }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let block_id = &events[0].entity;

        // alice 可以查看历史
        let history = get_block_history(&handle, "alice", block_id).await;
        assert!(history.is_ok());

        // bob 不可以查看历史
        let history = get_block_history(&handle, "bob", block_id).await;
        assert!(history.is_err());

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_get_state_at_event() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        // 创建 block
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "v1", "block_type": "document" }),
        );
        let create_events = handle.process_command(cmd).await.unwrap();
        let block_id = &create_events[0].entity;
        let event_id = &create_events[0].event_id;

        // 修改名称
        let cmd = Command::new(
            "alice".to_string(),
            "core.write".to_string(),
            block_id.clone(),
            serde_json::json!({ "name": "v2" }),
        );
        handle.process_command(cmd).await.unwrap();

        // 时间旅行到 create 时刻
        let (block, _grants) = get_state_at_event(&handle, "alice", block_id, event_id)
            .await
            .unwrap();
        assert_eq!(block.name, "v1");

        // 当前状态应是 v2
        let current = handle.get_block(block_id.clone()).await.unwrap();
        assert_eq!(current.name, "v2");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_get_state_at_event_short_id() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        // 创建 block
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "v1", "block_type": "document" }),
        );
        let create_events = handle.process_command(cmd).await.unwrap();
        let block_id = &create_events[0].entity;
        let event_id = &create_events[0].event_id;

        // 用前 8 位短 ID 查询
        let short = &event_id[..8];
        let (block, _grants) = get_state_at_event(&handle, "alice", block_id, short)
            .await
            .unwrap();
        assert_eq!(block.name, "v1");

        handle.shutdown().await;
    }

    #[test]
    fn test_resolve_event_index() {
        let events = vec![
            Event::new(
                "b1".to_string(),
                "alice/core.create".to_string(),
                serde_json::json!({}),
                HashMap::new(),
            ),
            Event::new(
                "b2".to_string(),
                "alice/core.create".to_string(),
                serde_json::json!({}),
                HashMap::new(),
            ),
        ];

        // 精确匹配
        let idx = resolve_event_index(&events, &events[0].event_id).unwrap();
        assert_eq!(idx, 0);

        // 前缀匹配（前 8 位）
        let short = &events[1].event_id[..8];
        let idx = resolve_event_index(&events, short).unwrap();
        assert_eq!(idx, 1);

        // 不存在
        let result = resolve_event_index(&events, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}

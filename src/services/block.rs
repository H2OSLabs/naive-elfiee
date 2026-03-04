//! Block 服务 — CBAC 过滤的 Block CRUD

use crate::engine::EngineHandle;
use crate::models::{Block, Command, Event};
use std::collections::HashSet;

/// 列出所有 blocks，CBAC 过滤：只返回 owner 或有 grant 的 blocks
pub async fn list_blocks(handle: &EngineHandle, editor_id: &str) -> Vec<Block> {
    let blocks_map = handle.get_all_blocks().await;

    // 获取该 editor 的 grants
    let all_grants = handle.get_all_grants().await;
    let user_grants = all_grants.get(editor_id).cloned().unwrap_or_default();
    let has_wildcard = user_grants.iter().any(|(_, bid)| bid == "*");
    let granted_blocks: HashSet<String> = user_grants
        .into_iter()
        .map(|(_, block_id)| block_id)
        .collect();

    let mut filtered = Vec::new();
    for block in blocks_map.into_values() {
        let is_owner = block.owner == editor_id;
        let has_grant = has_wildcard || granted_blocks.contains(&block.block_id);

        if is_owner || has_grant {
            filtered.push(block);
        }
    }

    filtered
}

/// 获取单个 block，CBAC 检查 {block_type}.read
pub async fn get_block(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
) -> Result<Block, String> {
    let block = handle
        .get_block(block_id.to_string())
        .await
        .ok_or_else(|| "Block not found".to_string())?;

    // CBAC: {block_type}.read
    let read_capability = format!("{}.read", block.block_type);
    let has_permission = handle
        .check_grant(editor_id.to_string(), read_capability, block_id.to_string())
        .await;

    if !has_permission {
        return Err(format!(
            "Permission denied: no {}.read on block",
            block.block_type
        ));
    }

    Ok(block)
}

/// 执行 Command（通过 pipeline，已有 CBAC）
pub async fn execute_command(handle: &EngineHandle, cmd: Command) -> Result<Vec<Event>, String> {
    handle.process_command(cmd).await
}

/// 重命名 block（通过 core.write）
pub async fn rename_block(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
    name: &str,
) -> Result<Vec<Event>, String> {
    let cmd = Command::new(
        editor_id.to_string(),
        "core.write".to_string(),
        block_id.to_string(),
        serde_json::json!({ "name": name }),
    );
    handle.process_command(cmd).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::registry::CapabilityRegistry;
    use crate::engine::{EventPoolWithPath, EventStore};
    use crate::models::Event;
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
    async fn test_list_blocks_cbac_filter() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        // alice 创建 block
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "Test", "block_type": "document" }),
        );
        handle.process_command(cmd).await.unwrap();

        // alice 能看到自己的 block
        let blocks = list_blocks(&handle, "alice").await;
        assert_eq!(blocks.len(), 1);

        // bob 没有 grant，看不到
        let blocks = list_blocks(&handle, "bob").await;
        assert_eq!(blocks.len(), 0);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_get_block_cbac() {
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

        // alice (owner) 可以读取
        let result = get_block(&handle, "alice", block_id).await;
        assert!(result.is_ok());

        // bob (no grants) 不可以读取
        let result = get_block(&handle, "bob", block_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Permission denied"));

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_rename_block() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "Old", "block_type": "document" }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let block_id = &events[0].entity;

        let result = rename_block(&handle, "alice", block_id, "New").await;
        assert!(result.is_ok());

        let block = handle.get_block(block_id.clone()).await.unwrap();
        assert_eq!(block.name, "New");

        handle.shutdown().await;
    }
}

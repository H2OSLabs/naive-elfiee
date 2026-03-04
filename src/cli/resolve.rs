//! Block name/ID 双模解析器
//!
//! 支持通过 name 或 id 查找 block_id：
//! - "*" → wildcard 直通
//! - 精确 id 匹配 → 返回
//! - name 匹配（唯一）→ 返回 id
//! - name 多个匹配 → 报错列出

use crate::engine::EngineHandle;

/// 按 name 或 id 解析 block_id
///
/// 解析规则：
/// 1. "*" → wildcard 直通
/// 2. 精确 id 匹配 → 返回该 id
/// 3. name 唯一匹配 → 返回对应 id
/// 4. name 多个匹配 → 报错，列出所有匹配
/// 5. 无匹配 → 报错
pub async fn resolve_block_id(handle: &EngineHandle, input: &str) -> Result<String, String> {
    // Wildcard 直通
    if input == "*" {
        return Ok("*".to_string());
    }

    let blocks = handle.get_all_blocks().await;

    // 精确 id 匹配
    if blocks.contains_key(input) {
        return Ok(input.to_string());
    }

    // Name 匹配
    let matches: Vec<(&String, &crate::models::Block)> =
        blocks.iter().filter(|(_, b)| b.name == input).collect();

    match matches.len() {
        0 => Err(format!(
            "Block not found: '{}' (not a valid block ID or name)",
            input
        )),
        1 => Ok(matches[0].0.clone()),
        _ => {
            let mut msg = format!(
                "Ambiguous block name '{}': {} blocks match:\n",
                input,
                matches.len()
            );
            for (id, block) in &matches {
                msg.push_str(&format!(
                    "  {} (type: {}, owner: {})\n",
                    id, block.block_type, block.owner
                ));
            }
            msg.push_str("Please use the block ID instead.");
            Err(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::registry::CapabilityRegistry;
    use crate::engine::{spawn_engine, EventStore};
    use crate::models::{Command, Event};
    use std::collections::HashMap;

    /// Seed a test editor with all permissions
    async fn seed_test_editor(event_pool: &crate::engine::EventPoolWithPath, editor_id: &str) {
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
    async fn test_resolve_wildcard() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test".to_string(), event_pool).await.unwrap();

        let result = resolve_block_id(&handle, "*").await;
        assert_eq!(result.unwrap(), "*");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_resolve_by_id() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test".to_string(), event_pool).await.unwrap();

        // 创建一个 block
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "src/main.rs",
                "block_type": "document"
            }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let block_id = events[0].entity.clone();

        // 按 id 解析
        let result = resolve_block_id(&handle, &block_id).await;
        assert_eq!(result.unwrap(), block_id);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_resolve_by_name() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test".to_string(), event_pool).await.unwrap();

        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "src/main.rs",
                "block_type": "document"
            }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let block_id = events[0].entity.clone();

        // 按 name 解析
        let result = resolve_block_id(&handle, "src/main.rs").await;
        assert_eq!(result.unwrap(), block_id);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_resolve_not_found() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test".to_string(), event_pool).await.unwrap();

        let result = resolve_block_id(&handle, "nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));

        handle.shutdown().await;
    }
}

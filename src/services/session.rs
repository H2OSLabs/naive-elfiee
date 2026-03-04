//! Session 服务 — Session 读取/追加

use crate::engine::EngineHandle;
use crate::models::{Block, Command, Event};

/// 读取 session block，CBAC 检查 session.read
pub async fn read_session(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
) -> Result<Block, String> {
    let block = handle
        .get_block(block_id.to_string())
        .await
        .ok_or_else(|| format!("Block '{}' not found", block_id))?;

    if block.block_type != "session" {
        return Err(format!(
            "Block '{}' is type '{}', not 'session'",
            block.name, block.block_type
        ));
    }

    // CBAC: session.read
    let has_permission = handle
        .check_grant(
            editor_id.to_string(),
            "session.read".to_string(),
            block_id.to_string(),
        )
        .await;

    if !has_permission {
        return Err("Permission denied: no session.read".to_string());
    }

    Ok(block)
}

/// 追加 session entry（通过 session.append capability）
pub async fn append_session(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
    entry_type: &str,
    data: serde_json::Value,
) -> Result<Vec<Event>, String> {
    let cmd = Command::new(
        editor_id.to_string(),
        "session.append".to_string(),
        block_id.to_string(),
        serde_json::json!({
            "entry_type": entry_type,
            "data": data
        }),
    );
    handle.process_command(cmd).await
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
    async fn test_read_session_cbac() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "session1", "block_type": "session" }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let block_id = &events[0].entity;

        // alice (owner) 可以读取
        let result = read_session(&handle, "alice", block_id).await;
        assert!(result.is_ok());

        // bob 不可以读取
        let result = read_session(&handle, "bob", block_id).await;
        assert!(result.is_err());

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_read_session_type_check() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        // 创建 document block
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "doc", "block_type": "document" }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let block_id = &events[0].entity;

        let result = read_session(&handle, "alice", block_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not 'session'"));

        handle.shutdown().await;
    }
}

//! Document 服务 — 文档读写 + CBAC

use crate::engine::EngineHandle;
use crate::models::{Block, Command, Event};

/// 读取 document 内容，CBAC 检查 document.read
pub async fn read_document(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
) -> Result<Block, String> {
    let block = handle
        .get_block(block_id.to_string())
        .await
        .ok_or_else(|| format!("Block '{}' not found", block_id))?;

    if block.block_type != "document" {
        return Err(format!(
            "Block '{}' is type '{}', not 'document'",
            block.name, block.block_type
        ));
    }

    // CBAC: document.read
    let has_permission = handle
        .check_grant(
            editor_id.to_string(),
            "document.read".to_string(),
            block_id.to_string(),
        )
        .await;

    if !has_permission {
        return Err("Permission denied: no document.read".to_string());
    }

    Ok(block)
}

/// 写入 document 内容（通过 document.write capability）
pub async fn write_document(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
    content: &str,
) -> Result<Vec<Event>, String> {
    let cmd = Command::new(
        editor_id.to_string(),
        "document.write".to_string(),
        block_id.to_string(),
        serde_json::json!({ "content": content }),
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
    async fn test_read_document_cbac() {
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
            serde_json::json!({ "name": "doc.md", "block_type": "document" }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let block_id = &events[0].entity;

        // alice (owner) 可以读取
        let result = read_document(&handle, "alice", block_id).await;
        assert!(result.is_ok());

        // bob (no grants) 不可以读取
        let result = read_document(&handle, "bob", block_id).await;
        assert!(result.is_err());

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_read_document_type_check() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        // 创建 task block（不是 document）
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "task1", "block_type": "task" }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let block_id = &events[0].entity;

        // 用 read_document 读 task block 应失败
        let result = read_document(&handle, "alice", block_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not 'document'"));

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_write_document() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "doc.md", "block_type": "document" }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let block_id = &events[0].entity;

        let result = write_document(&handle, "alice", block_id, "# Hello").await;
        assert!(result.is_ok());

        let block = handle.get_block(block_id.clone()).await.unwrap();
        assert_eq!(block.contents["content"], "# Hello");

        handle.shutdown().await;
    }
}

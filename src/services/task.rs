//! Task 服务 — Task 创建/读取/写入/提交

use crate::engine::EngineHandle;
use crate::models::{Block, Command, Event};

/// 读取 task block，CBAC 检查 task.read
pub async fn read_task(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
) -> Result<Block, String> {
    let block = handle
        .get_block(block_id.to_string())
        .await
        .ok_or_else(|| format!("Block '{}' not found", block_id))?;

    if block.block_type != "task" {
        return Err(format!(
            "Block '{}' is type '{}', not 'task'",
            block.name, block.block_type
        ));
    }

    // CBAC: task.read
    let has_permission = handle
        .check_grant(
            editor_id.to_string(),
            "task.read".to_string(),
            block_id.to_string(),
        )
        .await;

    if !has_permission {
        return Err("Permission denied: no task.read".to_string());
    }

    Ok(block)
}

/// 创建 task block，返回 (block_id, events)
pub async fn create_task(
    handle: &EngineHandle,
    editor_id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(String, Vec<Event>), String> {
    // 1. 创建 task block
    let cmd = Command::new(
        editor_id.to_string(),
        "core.create".to_string(),
        String::new(),
        serde_json::json!({ "name": name, "block_type": "task" }),
    );
    let events = handle.process_command(cmd).await?;
    let block_id = events
        .first()
        .map(|ev| ev.entity.clone())
        .unwrap_or_default();

    // 2. 如果提供了 description，通过 core.write 设置
    if let Some(desc) = description {
        let write_cmd = Command::new(
            editor_id.to_string(),
            "core.write".to_string(),
            block_id.clone(),
            serde_json::json!({ "description": desc }),
        );
        if let Err(e) = handle.process_command(write_cmd).await {
            log::warn!("Failed to set task description: {}", e);
        }
    }

    Ok((block_id, events))
}

/// 写入 task block（通过 task.write capability）
pub async fn write_task(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
    payload: serde_json::Value,
) -> Result<Vec<Event>, String> {
    let cmd = Command::new(
        editor_id.to_string(),
        "task.write".to_string(),
        block_id.to_string(),
        payload,
    );
    handle.process_command(cmd).await
}

/// 提交 task（通过 task.commit capability）
pub async fn commit_task(
    handle: &EngineHandle,
    editor_id: &str,
    block_id: &str,
) -> Result<Vec<Event>, String> {
    let cmd = Command::new(
        editor_id.to_string(),
        "task.commit".to_string(),
        block_id.to_string(),
        serde_json::json!({}),
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
    async fn test_read_task_cbac() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "task1", "block_type": "task" }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let block_id = &events[0].entity;

        // alice (owner) 可以读取
        let result = read_task(&handle, "alice", block_id).await;
        assert!(result.is_ok());

        // bob 不可以读取
        let result = read_task(&handle, "bob", block_id).await;
        assert!(result.is_err());

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_read_task_type_check() {
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

        // 用 read_task 读 document block 应失败
        let result = read_task(&handle, "alice", block_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not 'task'"));

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_create_task_with_description() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        let (block_id, _events) = create_task(&handle, "alice", "my-task", Some("描述"))
            .await
            .unwrap();

        let block = handle.get_block(block_id).await.unwrap();
        assert_eq!(block.name, "my-task");
        assert_eq!(block.description, Some("描述".to_string()));

        handle.shutdown().await;
    }
}

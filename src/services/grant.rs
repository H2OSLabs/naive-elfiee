//! Grant 服务 — 权限管理 + CBAC 过滤

use crate::engine::EngineHandle;
use crate::models::{Command, Event, Grant};
use std::collections::HashSet;

/// 列出所有 grants，CBAC 过滤
///
/// 规则：
/// - Wildcard grants (*) 始终可见
/// - Block-specific grants: 仅当用户是 owner、有 wildcard grant、或对该 block 有任意 grant 时可见
pub async fn list_grants(handle: &EngineHandle, editor_id: &str) -> Vec<Grant> {
    let grants_map = handle.get_all_grants().await;

    // 获取当前用户的 grants
    let user_grants = grants_map.get(editor_id).cloned().unwrap_or_default();
    let has_wildcard = user_grants.iter().any(|(_, bid)| bid == "*");
    let accessible_blocks: HashSet<String> = user_grants
        .into_iter()
        .map(|(_, block_id)| block_id)
        .collect();

    // 获取当前用户拥有的 blocks
    let blocks_map = handle.get_all_blocks().await;
    let owned_blocks: HashSet<&str> = blocks_map
        .values()
        .filter(|b| b.owner == editor_id)
        .map(|b| b.block_id.as_str())
        .collect();

    let mut grants = Vec::new();
    for (grant_editor_id, grant_list) in grants_map {
        for (cap_id, block_id) in grant_list {
            if block_id == "*"
                || has_wildcard
                || owned_blocks.contains(block_id.as_str())
                || accessible_blocks.contains(&block_id)
            {
                grants.push(Grant::new(grant_editor_id.clone(), cap_id, block_id));
            }
        }
    }

    grants
}

/// 获取 editor 的 grants
pub async fn get_editor_grants(handle: &EngineHandle, editor_id: &str) -> Vec<(String, String)> {
    handle.get_editor_grants(editor_id.to_string()).await
}

/// 获取 block 的 grants
pub async fn get_block_grants(handle: &EngineHandle, block_id: &str) -> Vec<Grant> {
    let grant_list = handle.get_block_grants(block_id.to_string()).await;
    grant_list
        .into_iter()
        .map(|(editor_id, cap_id, block_id)| Grant::new(editor_id, cap_id, block_id))
        .collect()
}

/// 授权
pub async fn grant_permission(
    handle: &EngineHandle,
    grantor_id: &str,
    target_editor: &str,
    capability: &str,
    block_id: &str,
) -> Result<Vec<Event>, String> {
    let cmd = Command::new(
        grantor_id.to_string(),
        "core.grant".to_string(),
        block_id.to_string(),
        serde_json::json!({
            "target_editor": target_editor,
            "capability": capability,
            "target_block": block_id
        }),
    );
    handle.process_command(cmd).await
}

/// 撤权
pub async fn revoke_permission(
    handle: &EngineHandle,
    revoker_id: &str,
    target_editor: &str,
    capability: &str,
    block_id: &str,
) -> Result<Vec<Event>, String> {
    let cmd = Command::new(
        revoker_id.to_string(),
        "core.revoke".to_string(),
        block_id.to_string(),
        serde_json::json!({
            "target_editor": target_editor,
            "capability": capability,
            "target_block": block_id
        }),
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
    async fn test_list_grants_cbac_filter() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        // alice 有 wildcard grants，应该能看到所有 grants
        let grants = list_grants(&handle, "alice").await;
        assert!(!grants.is_empty());

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_grant_and_revoke() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = crate::engine::spawn_engine("test".to_string(), event_pool)
            .await
            .unwrap();

        // 创建 editor bob
        let cmd = Command::new(
            "alice".to_string(),
            "editor.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "Bob", "editor_type": "Bot" }),
        );
        let events = handle.process_command(cmd).await.unwrap();
        let bob_id = events[0].entity.clone();

        // 授权
        let result = grant_permission(&handle, "alice", &bob_id, "document.read", "*").await;
        assert!(result.is_ok());

        // 验证 grant 存在
        let has = handle
            .check_grant(bob_id.clone(), "document.read".to_string(), "*".to_string())
            .await;
        assert!(has);

        // 撤权
        let result = revoke_permission(&handle, "alice", &bob_id, "document.read", "*").await;
        assert!(result.is_ok());

        // 验证 grant 已撤
        let has = handle
            .check_grant(bob_id, "document.read".to_string(), "*".to_string())
            .await;
        assert!(!has);

        handle.shutdown().await;
    }
}

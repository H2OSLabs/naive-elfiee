/// 集成测试：验证 get_block 命令的权限检查
///
/// 测试场景：
/// 1. block owner 可以读取自己的block
/// 2. 非owner但有read grant的editor可以读取block
/// 3. 非owner且没有read grant的editor不能读取block
/// 4. 不同block类型使用不同的read capability
use elfiee_lib::capabilities::registry::CapabilityRegistry;
use elfiee_lib::engine::{EventPoolWithPath, EventStore};
use elfiee_lib::models::{Command, Event};
use elfiee_lib::state::AppState;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_get_block_owner_can_read() {
    // 测试场景1: block owner 可以读取自己的block
    let (state, file_id, block_id) = setup_test_env().await;

    // 使用 owner (system) 读取 block - 直接调用engine的check_grant
    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    // block owner总是被授权
    let has_permission = handle
        .check_grant(
            "system".to_string(),
            "document.read".to_string(),
            block_id.clone(),
        )
        .await;

    assert!(
        has_permission,
        "Block owner should always have read permission"
    );

    // 实际获取block
    let block = handle.get_block(block_id.clone()).await;
    assert!(block.is_some());
    assert_eq!(block.unwrap().block_id, block_id);
}

#[tokio::test]
async fn test_get_block_non_owner_with_grant_can_read() {
    // 测试场景2: 非owner但有read grant的editor可以读取block
    let (state, file_id, block_id) = setup_test_env().await;

    // 创建另一个 editor (alice)
    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    let create_alice_cmd = Command {
        cmd_id: uuid::Uuid::new_v4().to_string(),
        editor_id: "system".to_string(),
        cap_id: "editor.create".to_string(),
        block_id: "alice".to_string(),
        payload: serde_json::json!({
            "editor_id": "alice",
            "name": "Alice"
        }),
        timestamp: chrono::Utc::now(),
    };
    handle.process_command(create_alice_cmd).await.unwrap();

    // alice 没有权限前不能读取
    let has_permission_before = handle
        .check_grant(
            "alice".to_string(),
            "document.read".to_string(),
            block_id.clone(),
        )
        .await;
    assert!(
        !has_permission_before,
        "Alice should not have permission initially"
    );

    // 授予 alice document.read 权限
    let grant_cmd = Command {
        cmd_id: uuid::Uuid::new_v4().to_string(),
        editor_id: "system".to_string(),
        cap_id: "core.grant".to_string(),
        block_id: block_id.clone(),
        payload: serde_json::json!({
            "target_editor": "alice",
            "capability": "document.read",
            "target_block": block_id.clone()
        }),
        timestamp: chrono::Utc::now(),
    };
    handle.process_command(grant_cmd).await.unwrap();

    // alice 有权限后可以读取
    let has_permission_after = handle
        .check_grant(
            "alice".to_string(),
            "document.read".to_string(),
            block_id.clone(),
        )
        .await;

    assert!(
        has_permission_after,
        "Alice should have read permission after grant"
    );

    // 实际获取block
    let block = handle.get_block(block_id.clone()).await;
    assert!(block.is_some());
    assert_eq!(block.unwrap().block_id, block_id);
}

#[tokio::test]
async fn test_get_block_non_owner_without_grant_cannot_read() {
    // 测试场景3: 非owner且没有read grant的editor不能读取block
    let (state, file_id, block_id) = setup_test_env().await;

    // 创建另一个 editor (bob)，但不授予权限
    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    let create_bob_cmd = Command {
        cmd_id: uuid::Uuid::new_v4().to_string(),
        editor_id: "system".to_string(),
        cap_id: "editor.create".to_string(),
        block_id: "bob".to_string(),
        payload: serde_json::json!({
            "editor_id": "bob",
            "name": "Bob"
        }),
        timestamp: chrono::Utc::now(),
    };
    handle.process_command(create_bob_cmd).await.unwrap();

    // bob 尝试读取 block（没有被授予权限）
    let has_permission = handle
        .check_grant(
            "bob".to_string(),
            "document.read".to_string(),
            block_id.clone(),
        )
        .await;

    assert!(
        !has_permission,
        "Bob should not have read permission without grant"
    );
}

#[tokio::test]
async fn test_get_block_different_block_types() {
    // 测试场景4: 不同block类型使用不同的read capability
    let (state, file_id, _document_block_id) = setup_test_env().await;

    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    // 创建 task block
    let create_task_cmd = Command {
        cmd_id: uuid::Uuid::new_v4().to_string(),
        editor_id: "system".to_string(),
        cap_id: "core.create".to_string(),
        block_id: "temp".to_string(),
        payload: serde_json::json!({
            "block_type": "task",
            "name": "Test Task Block"
        }),
        timestamp: chrono::Utc::now(),
    };
    let events = handle.process_command(create_task_cmd).await.unwrap();
    let task_block_id = events[0].entity.clone();

    // 创建 alice editor
    let create_alice_cmd = Command {
        cmd_id: uuid::Uuid::new_v4().to_string(),
        editor_id: "system".to_string(),
        cap_id: "editor.create".to_string(),
        block_id: "alice".to_string(),
        payload: serde_json::json!({
            "editor_id": "alice",
            "name": "Alice"
        }),
        timestamp: chrono::Utc::now(),
    };
    handle.process_command(create_alice_cmd).await.unwrap();

    // 授予 alice document.read 权限（注意是 document，不是 task）
    let grant_doc_cmd = Command {
        cmd_id: uuid::Uuid::new_v4().to_string(),
        editor_id: "system".to_string(),
        cap_id: "core.grant".to_string(),
        block_id: task_block_id.clone(),
        payload: serde_json::json!({
            "target_editor": "alice",
            "capability": "document.read",  // 错误的 capability
            "target_block": task_block_id.clone()
        }),
        timestamp: chrono::Utc::now(),
    };
    handle.process_command(grant_doc_cmd).await.unwrap();

    // alice 尝试读取 task block（有 document.read 但需要 task.read）
    let has_wrong_permission = handle
        .check_grant(
            "alice".to_string(),
            "task.read".to_string(),
            task_block_id.clone(),
        )
        .await;

    assert!(
        !has_wrong_permission,
        "Should not have permission with wrong capability type"
    );

    // 现在授予正确的 task.read 权限
    let grant_task_cmd = Command {
        cmd_id: uuid::Uuid::new_v4().to_string(),
        editor_id: "system".to_string(),
        cap_id: "core.grant".to_string(),
        block_id: task_block_id.clone(),
        payload: serde_json::json!({
            "target_editor": "alice",
            "capability": "task.read",  // 正确的 capability
            "target_block": task_block_id.clone()
        }),
        timestamp: chrono::Utc::now(),
    };
    handle.process_command(grant_task_cmd).await.unwrap();

    // alice 再次尝试读取 task block
    let has_correct_permission = handle
        .check_grant(
            "alice".to_string(),
            "task.read".to_string(),
            task_block_id.clone(),
        )
        .await;

    assert!(
        has_correct_permission,
        "Should have permission with correct capability type"
    );
}

// ========== Helper Functions ==========

/// Seed bootstrap events for a test editor directly to EventStore.
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

/// 设置测试环境：使用内存 EventStore，bootstrap system editor，启动 engine，创建 document block
async fn setup_test_env() -> (Arc<AppState>, String, String) {
    let event_pool = EventStore::create(":memory:")
        .await
        .expect("Failed to create in-memory event store");

    // Seed bootstrap events BEFORE engine spawn
    seed_test_editor(&event_pool, "system").await;

    let file_id = "test-file".to_string();

    let state = Arc::new(AppState::default());
    state
        .engine_manager
        .spawn_engine(file_id.clone(), event_pool)
        .await
        .unwrap();

    let handle = state.engine_manager.get_engine(&file_id).unwrap();

    // 创建一个 document block (owner 是 system)
    let create_block_cmd = Command {
        cmd_id: uuid::Uuid::new_v4().to_string(),
        editor_id: "system".to_string(),
        cap_id: "core.create".to_string(),
        block_id: "".to_string(),
        payload: serde_json::json!({
            "block_type": "document",
            "name": "Test Document Block"
        }),
        timestamp: chrono::Utc::now(),
    };
    let events = handle.process_command(create_block_cmd).await.unwrap();
    let block_id = events[0].entity.clone();

    state.set_active_editor(file_id.clone(), "system".to_string());

    (state, file_id, block_id)
}

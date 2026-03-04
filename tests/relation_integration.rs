/// 集成测试：Relation 系统增强 (I2-01 ~ I2-07)
///
/// 验证逻辑因果图（Logical Causal Graph）功能：
/// - I2-01: RELATION_IMPLEMENT 常量定义
/// - I2-02: core.link 限制为 implement 关系 + 防重复
/// - I2-03: core.unlink 限制为 implement 关系
/// - I2-04: StateProjector 反向索引（parents）
/// - I2-05: DAG 环检测（自环、直接环、间接环）
/// - I2-06: 现有测试 relation type 替换
/// - I2-07: 本文件的集成测试
use elfiee_lib::capabilities::registry::CapabilityRegistry;
use elfiee_lib::engine::{spawn_engine, EventPoolWithPath, EventStore};
use elfiee_lib::models::{Command, Event, RELATION_IMPLEMENT};
use std::collections::HashMap;

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

/// 辅助函数：创建内存 engine
async fn setup_engine() -> elfiee_lib::engine::EngineHandle {
    let event_pool = EventStore::create(":memory:").await.unwrap();
    seed_test_editor(&event_pool, "alice").await;
    spawn_engine("test_relation".to_string(), event_pool)
        .await
        .unwrap()
}

/// 辅助函数：创建一个 block，返回 block_id
async fn create_block(handle: &elfiee_lib::engine::EngineHandle, name: &str) -> String {
    let cmd = Command::new(
        "alice".to_string(),
        "core.create".to_string(),
        "".to_string(),
        serde_json::json!({
            "name": name,
            "block_type": "document"
        }),
    );
    let events = handle.process_command(cmd).await.unwrap();
    events[0].entity.clone()
}

/// 辅助函数：执行 core.link
async fn link_blocks(
    handle: &elfiee_lib::engine::EngineHandle,
    source_id: &str,
    target_id: &str,
) -> Result<(), String> {
    let cmd = Command::new(
        "alice".to_string(),
        "core.link".to_string(),
        source_id.to_string(),
        serde_json::json!({
            "relation": RELATION_IMPLEMENT,
            "target_id": target_id
        }),
    );
    handle.process_command(cmd).await.map(|_| ())
}

/// 辅助函数：执行 core.unlink
async fn unlink_blocks(
    handle: &elfiee_lib::engine::EngineHandle,
    source_id: &str,
    target_id: &str,
) -> Result<(), String> {
    let cmd = Command::new(
        "alice".to_string(),
        "core.unlink".to_string(),
        source_id.to_string(),
        serde_json::json!({
            "relation": RELATION_IMPLEMENT,
            "target_id": target_id
        }),
    );
    handle.process_command(cmd).await.map(|_| ())
}

/// 辅助函数：执行 core.delete
async fn delete_block(
    handle: &elfiee_lib::engine::EngineHandle,
    block_id: &str,
) -> Result<(), String> {
    let cmd = Command::new(
        "alice".to_string(),
        "core.delete".to_string(),
        block_id.to_string(),
        serde_json::json!({}),
    );
    handle.process_command(cmd).await.map(|_| ())
}

// ============================================================================
// I2-02: core.link 测试
// ============================================================================

/// 正常 implement 链接成功
#[tokio::test]
async fn test_link_implement_success() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;
    let b = create_block(&handle, "Block B").await;

    let result = link_blocks(&handle, &a, &b).await;
    assert!(result.is_ok(), "implement link should succeed");

    // 验证 children 已更新
    let block_a = handle.get_block(a.clone()).await.unwrap();
    let children = block_a.children.get(RELATION_IMPLEMENT).unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0], b);

    handle.shutdown().await;
}

/// 非 implement 关系被拒绝
#[tokio::test]
async fn test_link_non_implement_rejected() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;
    let b = create_block(&handle, "Block B").await;

    let cmd = Command::new(
        "alice".to_string(),
        "core.link".to_string(),
        a.clone(),
        serde_json::json!({
            "relation": "references",
            "target_id": b
        }),
    );
    let result = handle.process_command(cmd).await;
    assert!(result.is_err(), "non-implement relation should be rejected");
    assert!(result
        .unwrap_err()
        .contains("Only 'implement' relation is allowed"));

    handle.shutdown().await;
}

/// 同一对 source→target 不重复
#[tokio::test]
async fn test_link_duplicate_prevented() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;
    let b = create_block(&handle, "Block B").await;

    // 第一次链接
    link_blocks(&handle, &a, &b).await.unwrap();

    // 第二次链接相同的对
    let result = link_blocks(&handle, &a, &b).await;
    assert!(result.is_err(), "duplicate link should be rejected");
    assert!(result.unwrap_err().contains("Duplicate link"));

    handle.shutdown().await;
}

// ============================================================================
// I2-05: DAG 环检测测试
// ============================================================================

/// A→B→A 直接环被拒绝
#[tokio::test]
async fn test_link_cycle_direct() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;
    let b = create_block(&handle, "Block B").await;

    // A → B
    link_blocks(&handle, &a, &b).await.unwrap();

    // B → A 应该被拒绝（形成 A→B→A 环）
    let result = link_blocks(&handle, &b, &a).await;
    assert!(result.is_err(), "direct cycle should be rejected");
    assert!(result.unwrap_err().contains("Cycle detected"));

    handle.shutdown().await;
}

/// A→B→C→A 间接环被拒绝
#[tokio::test]
async fn test_link_cycle_indirect() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;
    let b = create_block(&handle, "Block B").await;
    let c = create_block(&handle, "Block C").await;

    // A → B → C
    link_blocks(&handle, &a, &b).await.unwrap();
    link_blocks(&handle, &b, &c).await.unwrap();

    // C → A 应该被拒绝（形成 A→B→C→A 环）
    let result = link_blocks(&handle, &c, &a).await;
    assert!(result.is_err(), "indirect cycle should be rejected");
    assert!(result.unwrap_err().contains("Cycle detected"));

    handle.shutdown().await;
}

/// A→B, A→C, B→D, C→D（菱形 DAG）合法
#[tokio::test]
async fn test_link_dag_valid() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;
    let b = create_block(&handle, "Block B").await;
    let c = create_block(&handle, "Block C").await;
    let d = create_block(&handle, "Block D").await;

    // 菱形 DAG: A→B, A→C, B→D, C→D
    link_blocks(&handle, &a, &b).await.unwrap();
    link_blocks(&handle, &a, &c).await.unwrap();
    link_blocks(&handle, &b, &d).await.unwrap();
    link_blocks(&handle, &c, &d).await.unwrap();

    // 验证所有 children
    let block_a = handle.get_block(a.clone()).await.unwrap();
    let a_children = block_a.children.get(RELATION_IMPLEMENT).unwrap();
    assert_eq!(a_children.len(), 2);

    let block_b = handle.get_block(b.clone()).await.unwrap();
    let b_children = block_b.children.get(RELATION_IMPLEMENT).unwrap();
    assert_eq!(b_children.len(), 1);
    assert_eq!(b_children[0], d);

    handle.shutdown().await;
}

/// A→A 自环被拒绝
#[tokio::test]
async fn test_link_self_cycle() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;

    let result = link_blocks(&handle, &a, &a).await;
    assert!(result.is_err(), "self-cycle should be rejected");
    assert!(result.unwrap_err().contains("Cycle detected"));

    handle.shutdown().await;
}

// ============================================================================
// I2-03: core.unlink 测试
// ============================================================================

/// unlink 后反向索引正确更新（通过验证再次 link 同对不报重复来间接验证）
#[tokio::test]
async fn test_unlink_removes_parent() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;
    let b = create_block(&handle, "Block B").await;

    // Link A → B
    link_blocks(&handle, &a, &b).await.unwrap();

    // 验证 children 存在
    let block_a = handle.get_block(a.clone()).await.unwrap();
    assert!(block_a.children.get(RELATION_IMPLEMENT).is_some());

    // Unlink A → B
    unlink_blocks(&handle, &a, &b).await.unwrap();

    // 验证 children 已清空
    let block_a = handle.get_block(a.clone()).await.unwrap();
    assert!(
        block_a.children.get(RELATION_IMPLEMENT).is_none()
            || block_a.children.get(RELATION_IMPLEMENT).unwrap().is_empty()
    );

    // 可以重新链接（不会报重复）
    let result = link_blocks(&handle, &a, &b).await;
    assert!(result.is_ok(), "should be able to re-link after unlink");

    handle.shutdown().await;
}

/// unlink 非 implement 关系被拒绝
#[tokio::test]
async fn test_unlink_non_implement_rejected() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;

    let cmd = Command::new(
        "alice".to_string(),
        "core.unlink".to_string(),
        a.clone(),
        serde_json::json!({
            "relation": "references",
            "target_id": "some-block"
        }),
    );
    let result = handle.process_command(cmd).await;
    assert!(result.is_err(), "non-implement unlink should be rejected");
    assert!(result
        .unwrap_err()
        .contains("Only 'implement' relation is allowed"));

    handle.shutdown().await;
}

// ============================================================================
// I2-04: 反向索引查询测试
// ============================================================================

/// 查询 block 的所有上游（parents）
#[tokio::test]
async fn test_parents_query() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;
    let b = create_block(&handle, "Block B").await;
    let c = create_block(&handle, "Block C").await;
    let d = create_block(&handle, "Block D").await;

    // A → D, B → D, C → D （D 有三个上游）
    link_blocks(&handle, &a, &d).await.unwrap();
    link_blocks(&handle, &b, &d).await.unwrap();
    link_blocks(&handle, &c, &d).await.unwrap();

    // 目前 EngineHandle 没有直接暴露 get_parents()，
    // 我们通过 StateProjector 的单元测试来验证反向索引。
    // 此处验证 children 正确性，间接确认反向索引维护。
    let block_a = handle.get_block(a.clone()).await.unwrap();
    assert!(block_a
        .children
        .get(RELATION_IMPLEMENT)
        .unwrap()
        .contains(&d));

    let block_b = handle.get_block(b.clone()).await.unwrap();
    assert!(block_b
        .children
        .get(RELATION_IMPLEMENT)
        .unwrap()
        .contains(&d));

    let block_c = handle.get_block(c.clone()).await.unwrap();
    assert!(block_c
        .children
        .get(RELATION_IMPLEMENT)
        .unwrap()
        .contains(&d));

    handle.shutdown().await;
}

/// 查询 block 的所有下游（children）
#[tokio::test]
async fn test_children_query() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;
    let b = create_block(&handle, "Block B").await;
    let c = create_block(&handle, "Block C").await;

    // A → B, A → C （A 有两个下游）
    link_blocks(&handle, &a, &b).await.unwrap();
    link_blocks(&handle, &a, &c).await.unwrap();

    let block_a = handle.get_block(a.clone()).await.unwrap();
    let children = block_a.children.get(RELATION_IMPLEMENT).unwrap();
    assert_eq!(children.len(), 2);
    assert!(children.contains(&b));
    assert!(children.contains(&c));

    handle.shutdown().await;
}

// ============================================================================
// core.delete 清理测试
// ============================================================================

/// 删除 block 后，其作为 source 的 children 关系被清理
/// 通过验证原来被环检测阻止的链接现在可以创建来间接验证
#[tokio::test]
async fn test_delete_cleans_parents() {
    let handle = setup_engine().await;
    let a = create_block(&handle, "Block A").await;
    let b = create_block(&handle, "Block B").await;
    let c = create_block(&handle, "Block C").await;

    // A → B → C
    link_blocks(&handle, &a, &b).await.unwrap();
    link_blocks(&handle, &b, &c).await.unwrap();

    // 此时 C → A 会被环检测阻止
    let result = link_blocks(&handle, &c, &a).await;
    assert!(result.is_err(), "should detect cycle before delete");

    // 删除 B，打断因果链
    delete_block(&handle, &b).await.unwrap();

    // 现在 C → A 不应再检测到环（因为 A→B 链已断）
    // 但需要注意：A 的 children 中仍然引用了已删除的 B
    // 这是允许的——环检测遍历时找不到 B 的 block 数据，自动终止
    // 所以 C → A 现在应该成功
    let result = link_blocks(&handle, &c, &a).await;
    assert!(
        result.is_ok(),
        "should succeed after deleting intermediate block"
    );

    handle.shutdown().await;
}

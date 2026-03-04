//! Tests for Task extension
//!
//! Test categories:
//! - Payload deserialization tests
//! - Basic capability functionality tests
//! - Authorization/CBAC tests
//! - Block type validation tests
//! - Integration workflow tests

use super::*;
use crate::capabilities::grants::GrantsTable;
use crate::capabilities::registry::CapabilityRegistry;
use crate::models::{Block, Command, RELATION_IMPLEMENT};
use std::collections::HashMap;

// ============================================
// Helper functions
// ============================================

fn create_task_block(owner: &str) -> Block {
    let mut block = Block::new(
        "Test Task".to_string(),
        "task".to_string(),
        owner.to_string(),
    );
    block.contents = serde_json::json!({
        "description": "为项目添加 OAuth2 登录",
        "status": "pending",
        "assigned_to": "coder-agent"
    });
    block
}

fn create_task_block_with_children(owner: &str) -> Block {
    let mut block = create_task_block(owner);
    let mut children = HashMap::new();
    children.insert(
        RELATION_IMPLEMENT.to_string(),
        vec!["block-code-1".to_string(), "block-code-2".to_string()],
    );
    block.children = children;
    block
}

// ============================================
// TaskWritePayload Tests
// ============================================

#[test]
fn test_write_payload_deserialize_description_only() {
    let json = serde_json::json!({
        "description": "实现登录功能"
    });
    let payload: TaskWritePayload = serde_json::from_value(json).unwrap();
    assert_eq!(payload.description, Some("实现登录功能".to_string()));
    assert!(payload.status.is_none());
    assert!(payload.assigned_to.is_none());
    assert!(payload.template.is_none());
}

#[test]
fn test_write_payload_deserialize_all_fields() {
    let json = serde_json::json!({
        "description": "实现登录",
        "status": "in_progress",
        "assigned_to": "alice",
        "template": "code-review"
    });
    let payload: TaskWritePayload = serde_json::from_value(json).unwrap();
    assert_eq!(payload.description, Some("实现登录".to_string()));
    assert_eq!(payload.status, Some("in_progress".to_string()));
    assert_eq!(payload.assigned_to, Some("alice".to_string()));
    assert_eq!(payload.template, Some("code-review".to_string()));
}

#[test]
fn test_write_payload_empty_object_accepted() {
    // 空对象可以反序列化（所有字段都有 #[serde(default)]）
    let json = serde_json::json!({});
    let result: Result<TaskWritePayload, _> = serde_json::from_value(json);
    assert!(
        result.is_ok(),
        "Empty object should deserialize (all fields optional)"
    );
    let payload = result.unwrap();
    assert!(payload.description.is_none());
    assert!(payload.status.is_none());
}

// ============================================
// TaskReadPayload Tests
// ============================================

#[test]
fn test_read_payload_deserialize_empty() {
    let json = serde_json::json!({});
    let result: Result<TaskReadPayload, _> = serde_json::from_value(json);
    assert!(result.is_ok(), "Empty object should deserialize for read");
}

// ============================================
// TaskCommitPayload Tests
// ============================================

#[test]
fn test_commit_payload_deserialize_empty() {
    let json = serde_json::json!({});
    let result: Result<TaskCommitPayload, _> = serde_json::from_value(json);
    assert!(result.is_ok(), "Empty object should deserialize for commit");
}

// ============================================
// task.write Functionality Tests
// ============================================

#[test]
fn test_write_description() {
    let registry = CapabilityRegistry::new();
    let cap = registry
        .get("task.write")
        .expect("task.write should be registered");

    let block = Block::new(
        "Test Task".to_string(),
        "task".to_string(),
        "alice".to_string(),
    );

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "description": "为项目添加 OAuth 登录支持"
        }),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_ok(), "Handler should execute successfully");

    let events = result.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity, block.block_id);
    assert_eq!(events[0].attribute, "alice/task.write");

    // 验证 contents.description
    let contents = events[0].value.get("contents").unwrap();
    assert_eq!(
        contents.get("description").unwrap().as_str().unwrap(),
        "为项目添加 OAuth 登录支持"
    );
}

#[test]
fn test_write_status() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    let block = Block::new("Task".to_string(), "task".to_string(), "alice".to_string());

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "status": "in_progress"
        }),
    );

    let events = cap.handler(&cmd, Some(&block)).unwrap();
    let contents = events[0].value.get("contents").unwrap();
    assert_eq!(contents["status"], "in_progress");
}

#[test]
fn test_write_preserves_existing_fields() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    let mut block = Block::new("Task".to_string(), "task".to_string(), "alice".to_string());
    block.contents = serde_json::json!({
        "description": "旧描述",
        "status": "pending",
        "assigned_to": "bob"
    });

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "status": "completed"
        }),
    );

    let events = cap.handler(&cmd, Some(&block)).unwrap();
    let contents = events[0].value.get("contents").unwrap();

    // status 被更新
    assert_eq!(contents["status"], "completed");
    // 其他字段保留
    assert_eq!(contents["description"], "旧描述");
    assert_eq!(contents["assigned_to"], "bob");
}

#[test]
fn test_write_multiple_fields() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    let block = Block::new("Task".to_string(), "task".to_string(), "alice".to_string());

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "description": "新任务",
            "status": "pending",
            "assigned_to": "alice",
            "template": "code-review"
        }),
    );

    let events = cap.handler(&cmd, Some(&block)).unwrap();
    assert_eq!(events.len(), 1);

    let contents = &events[0].value["contents"];
    assert_eq!(contents["description"], "新任务");
    assert_eq!(contents["status"], "pending");
    assert_eq!(contents["assigned_to"], "alice");
    assert_eq!(contents["template"], "code-review");
}

#[test]
fn test_write_empty_payload_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    let block = Block::new("Task".to_string(), "task".to_string(), "alice".to_string());

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("at least one field"));
}

#[test]
fn test_write_wrong_block_type() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    let block = Block::new(
        "Doc".to_string(),
        "document".to_string(),
        "alice".to_string(),
    );

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "description": "内容"
        }),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Expected task block"));
}

#[test]
fn test_write_no_block_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        "nonexistent".to_string(),
        serde_json::json!({ "description": "c" }),
    );

    let result = cap.handler(&cmd, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Block required"));
}

// ============================================
// task.write Authorization Tests
// ============================================

#[test]
fn test_write_authorization_owner() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "alice" || grants_table.has_grant("alice", "task.write", &block.block_id);
    assert!(is_authorized, "Owner should be authorized");
}

#[test]
fn test_write_authorization_non_owner_without_grant() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.write", &block.block_id);
    assert!(
        !is_authorized,
        "Non-owner without grant should not be authorized"
    );
}

#[test]
fn test_write_authorization_non_owner_with_grant() {
    let mut grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    grants_table.add_grant(
        "bob".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.write", &block.block_id);
    assert!(is_authorized, "Non-owner with grant should be authorized");
}

// ============================================
// task.read Functionality Tests
// ============================================

#[test]
fn test_read_basic() {
    let registry = CapabilityRegistry::new();
    let cap = registry
        .get("task.read")
        .expect("task.read should be registered");

    let block = create_task_block("alice");

    let cmd = Command::new(
        "alice".to_string(),
        "task.read".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_ok(), "Handler should execute successfully");

    let events = result.unwrap();
    assert_eq!(events.len(), 0, "task.read is permission-only, no events");
}

#[test]
fn test_read_wrong_block_type() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.read").unwrap();

    let block = Block::new(
        "Doc".to_string(),
        "document".to_string(),
        "alice".to_string(),
    );

    let cmd = Command::new(
        "alice".to_string(),
        "task.read".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Expected task block"));
}

#[test]
fn test_read_no_block_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.read").unwrap();

    let cmd = Command::new(
        "alice".to_string(),
        "task.read".to_string(),
        "nonexistent".to_string(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, None);
    assert!(result.is_err());
}

// ============================================
// task.read Authorization Tests
// ============================================

#[test]
fn test_read_authorization_owner() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "alice" || grants_table.has_grant("alice", "task.read", &block.block_id);
    assert!(is_authorized);
}

#[test]
fn test_read_authorization_non_owner_without_grant() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.read", &block.block_id);
    assert!(!is_authorized);
}

#[test]
fn test_read_authorization_non_owner_with_grant() {
    let mut grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    grants_table.add_grant(
        "bob".to_string(),
        "task.read".to_string(),
        block.block_id.clone(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.read", &block.block_id);
    assert!(is_authorized);
}

// ============================================
// task.commit Functionality Tests
// ============================================

#[test]
fn test_commit_basic_with_downstream() {
    let registry = CapabilityRegistry::new();
    let cap = registry
        .get("task.commit")
        .expect("task.commit should be registered");

    let block = create_task_block_with_children("alice");

    let cmd = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(
        result.is_ok(),
        "Handler should succeed with downstream blocks"
    );

    let events = result.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity, block.block_id);
    assert_eq!(events[0].attribute, "alice/task.commit");

    // 验证 event value（无 target_path，只有 downstream_block_ids）
    let value = &events[0].value;
    assert!(
        value.get("target_path").is_none(),
        "target_path should not be present"
    );
    let downstream: Vec<String> =
        serde_json::from_value(value.get("downstream_block_ids").unwrap().clone()).unwrap();
    assert_eq!(downstream.len(), 2);
    assert!(downstream.contains(&"block-code-1".to_string()));
    assert!(downstream.contains(&"block-code-2".to_string()));
}

#[test]
fn test_commit_no_downstream_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.commit").unwrap();

    // Task block with no children
    let block = create_task_block("alice");

    let cmd = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No downstream blocks"));
}

#[test]
fn test_commit_empty_payload_accepted() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.commit").unwrap();

    let block = create_task_block_with_children("alice");

    let cmd = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_ok(), "Empty payload should be accepted");
}

#[test]
fn test_commit_wrong_block_type() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.commit").unwrap();

    let block = Block::new(
        "Doc".to_string(),
        "document".to_string(),
        "alice".to_string(),
    );

    let cmd = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Expected task block"));
}

#[test]
fn test_commit_allows_repeated_commits() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.commit").unwrap();

    let block = create_task_block_with_children("alice");

    let cmd1 = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );
    assert!(cap.handler(&cmd1, Some(&block)).is_ok());

    let cmd2 = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );
    assert!(cap.handler(&cmd2, Some(&block)).is_ok());
}

// ============================================
// task.commit Authorization Tests
// ============================================

#[test]
fn test_commit_authorization_owner() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "alice" || grants_table.has_grant("alice", "task.commit", &block.block_id);
    assert!(is_authorized);
}

#[test]
fn test_commit_authorization_non_owner_without_grant() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.commit", &block.block_id);
    assert!(!is_authorized);
}

#[test]
fn test_commit_authorization_non_owner_with_grant() {
    let mut grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    grants_table.add_grant(
        "bob".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.commit", &block.block_id);
    assert!(is_authorized);
}

// ============================================
// Integration Workflow Test
// ============================================

#[test]
fn test_full_workflow_write_then_commit() {
    let registry = CapabilityRegistry::new();

    // Step 1: Create task block with downstream
    let mut block = Block::new(
        "Commit Task".to_string(),
        "task".to_string(),
        "alice".to_string(),
    );

    // Step 2: Write structured task fields
    let write_cap = registry.get("task.write").unwrap();
    let write_cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "description": "修复登录 bug",
            "status": "in_progress",
            "assigned_to": "alice"
        }),
    );
    let write_events = write_cap.handler(&write_cmd, Some(&block)).unwrap();
    block.contents = write_events[0].value.get("contents").unwrap().clone();

    // Step 3: Link downstream blocks (simulate)
    let mut children = HashMap::new();
    children.insert(
        RELATION_IMPLEMENT.to_string(),
        vec!["block-fix-1".to_string()],
    );
    block.children = children;

    // Step 4: Commit (empty payload, auto-discover)
    let commit_cap = registry.get("task.commit").unwrap();
    let commit_cmd = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );
    let commit_events = commit_cap.handler(&commit_cmd, Some(&block)).unwrap();
    assert_eq!(commit_events.len(), 1);
    assert_eq!(commit_events[0].attribute, "alice/task.commit");

    // Verify: structured fields preserved after commit
    assert_eq!(
        block.contents.get("description").unwrap().as_str().unwrap(),
        "修复登录 bug"
    );
    assert_eq!(
        block.contents.get("status").unwrap().as_str().unwrap(),
        "in_progress"
    );
    assert_eq!(
        block.contents.get("assigned_to").unwrap().as_str().unwrap(),
        "alice"
    );
}

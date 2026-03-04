//! Tests for Document extension

use super::*;
use crate::capabilities::registry::CapabilityRegistry;
use crate::models::{Block, Command};
use serde_json::json;
use std::collections::HashMap;

fn create_test_block() -> Block {
    Block {
        block_id: "block-123".to_string(),
        name: "Test Document".to_string(),
        block_type: "document".to_string(),
        owner: "alice".to_string(),
        contents: json!({ "format": "md" }),
        children: HashMap::new(),
        description: None,
    }
}

#[test]
fn test_document_write_stores_content() {
    let registry = CapabilityRegistry::new();
    let cap = registry
        .get("document.write")
        .expect("document.write should be registered");
    let block = create_test_block();

    let cmd = Command::new(
        "alice".to_string(),
        "document.write".to_string(),
        block.block_id.clone(),
        json!({ "content": "# Hello World" }),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_ok());

    let events = result.unwrap();
    assert_eq!(events.len(), 1);

    let event = &events[0];
    assert_eq!(event.value["contents"]["content"], "# Hello World");
    // format should be preserved
    assert_eq!(event.value["contents"]["format"], "md");
}

#[test]
fn test_document_write_preserves_existing_fields() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("document.write").unwrap();
    let mut block = create_test_block();
    block.contents = json!({
        "source": "outline",
        "format": "rs",
        "path": "src/main.rs"
    });

    let cmd = Command::new(
        "alice".to_string(),
        "document.write".to_string(),
        block.block_id.clone(),
        json!({ "content": "fn main() {}" }),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_ok());

    let events = result.unwrap();
    let new_contents = &events[0].value["contents"];

    assert_eq!(new_contents["content"], "fn main() {}");
    assert_eq!(new_contents["source"], "outline");
    assert_eq!(new_contents["format"], "rs");
    assert_eq!(new_contents["path"], "src/main.rs");
}

#[test]
fn test_document_write_no_block_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("document.write").unwrap();

    let cmd = Command::new(
        "alice".to_string(),
        "document.write".to_string(),
        "nonexistent".to_string(),
        json!({ "content": "Content" }),
    );

    let result = cap.handler(&cmd, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Block required"));
}

#[test]
fn test_document_write_wrong_block_type() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("document.write").unwrap();

    let block = Block::new("Task".to_string(), "task".to_string(), "alice".to_string());

    let cmd = Command::new(
        "alice".to_string(),
        "document.write".to_string(),
        block.block_id.clone(),
        json!({ "content": "Content" }),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Expected document block"));
}

#[test]
fn test_document_read_returns_empty_events() {
    let registry = CapabilityRegistry::new();
    let cap = registry
        .get("document.read")
        .expect("document.read should be registered");
    let block = create_test_block();

    let cmd = Command::new(
        "alice".to_string(),
        "document.read".to_string(),
        block.block_id.clone(),
        json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 0);
}

#[test]
fn test_document_read_no_block_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("document.read").unwrap();

    let cmd = Command::new(
        "alice".to_string(),
        "document.read".to_string(),
        "nonexistent".to_string(),
        json!({}),
    );

    let result = cap.handler(&cmd, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Block required"));
}

#[test]
fn test_document_read_wrong_block_type() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("document.read").unwrap();

    let block = Block::new("Task".to_string(), "task".to_string(), "alice".to_string());

    let cmd = Command::new(
        "alice".to_string(),
        "document.read".to_string(),
        block.block_id.clone(),
        json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Expected document block"));
}

#[test]
fn test_document_write_payload_deserialization() {
    let json_val = json!({ "content": "some text" });

    let payload: Result<DocumentWritePayload, _> = serde_json::from_value(json_val);
    assert!(payload.is_ok());
    assert_eq!(payload.unwrap().content, "some text");
}

#[test]
fn test_document_read_payload_deserialization() {
    let json_val = json!({});

    let payload: Result<DocumentReadPayload, _> = serde_json::from_value(json_val);
    assert!(payload.is_ok());
}

use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;

use super::DocumentWritePayload;

/// Handler for document.write capability.
///
/// Writes text content to a document block's contents field.
/// The content is stored under the "content" key in the contents object,
/// aligned with data-model.md §5.1 Document Block schema.
#[capability(id = "document.write", target = "document")]
fn handle_document_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for document.write")?;

    if block.block_type != "document" {
        return Err(format!(
            "Expected document block, got '{}'",
            block.block_type
        ));
    }

    let payload: DocumentWritePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for document.write: {}", e))?;

    // Merge content into existing contents, preserving format/path/hash/etc.
    let mut new_contents = if let Some(obj) = block.contents.as_object() {
        obj.clone()
    } else {
        serde_json::Map::new()
    };
    new_contents.insert("content".to_string(), serde_json::json!(payload.content));

    let event = create_event(
        block.block_id.clone(),
        "document.write",
        serde_json::json!({
            "contents": new_contents
        }),
        &cmd.editor_id,
        1, // Placeholder — engine actor updates with correct count
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Block, Command};
    use std::collections::HashMap;

    fn create_test_block() -> Block {
        Block {
            block_id: "block-123".to_string(),
            name: "Test Block".to_string(),
            block_type: "document".to_string(),
            owner: "alice".to_string(),
            contents: serde_json::json!({ "format": "md" }),
            children: HashMap::new(),
            description: None,
        }
    }

    #[test]
    fn test_document_write_basic() {
        let block = create_test_block();

        let cmd = Command::new(
            "alice".to_string(),
            "document.write".to_string(),
            block.block_id.clone(),
            serde_json::json!({ "content": "# Hello World" }),
        );

        let result = handle_document_write(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        let new_contents = &event.value["contents"];
        assert_eq!(new_contents["content"], "# Hello World");
    }

    #[test]
    fn test_document_write_preserves_format() {
        let mut block = create_test_block();
        block.contents = serde_json::json!({
            "source": "outline",
            "format": "rs"
        });

        let cmd = Command::new(
            "alice".to_string(),
            "document.write".to_string(),
            block.block_id.clone(),
            serde_json::json!({ "content": "fn main() {}" }),
        );

        let result = handle_document_write(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        let new_contents = &events[0].value["contents"];

        assert_eq!(new_contents["content"], "fn main() {}");
        assert_eq!(new_contents["source"], "outline");
        assert_eq!(new_contents["format"], "rs");
    }

    #[test]
    fn test_document_write_no_block_fails() {
        let cmd = Command::new(
            "alice".to_string(),
            "document.write".to_string(),
            "nonexistent".to_string(),
            serde_json::json!({ "content": "Content" }),
        );

        let result = handle_document_write(&cmd, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Block required"));
    }
}

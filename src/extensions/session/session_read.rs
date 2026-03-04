use crate::capabilities::core::CapResult;
use crate::models::{Block, Command, Event};
use capability_macros::capability;

/// Handler for session.read capability.
///
/// Permission gate for reading session block contents.
/// Actual data retrieval happens via the query layer (get_block / get_all_blocks).
/// This handler returns an empty event list since reads are side-effect free.
#[capability(id = "session.read", target = "session")]
fn handle_session_read(_cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for session.read")?;

    if block.block_type != "session" {
        return Err(format!(
            "Expected session block, got '{}'",
            block.block_type
        ));
    }

    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Block, Command};
    use serde_json::json;
    use std::collections::HashMap;

    fn create_session_block() -> Block {
        Block {
            block_id: "session-001".to_string(),
            name: "Build Log".to_string(),
            block_type: "session".to_string(),
            owner: "alice".to_string(),
            contents: json!({ "entries": [] }),
            children: HashMap::new(),
            description: None,
        }
    }

    #[test]
    fn test_session_read_success() {
        let block = create_session_block();
        let cmd = Command::new(
            "alice".to_string(),
            "session.read".to_string(),
            block.block_id.clone(),
            json!({}),
        );

        let result = handle_session_read(&cmd, Some(&block));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_session_read_no_block_fails() {
        let cmd = Command::new(
            "alice".to_string(),
            "session.read".to_string(),
            "nonexistent".to_string(),
            json!({}),
        );

        let result = handle_session_read(&cmd, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Block required"));
    }

    #[test]
    fn test_session_read_wrong_type_fails() {
        let block = Block {
            block_id: "doc-001".to_string(),
            name: "A Document".to_string(),
            block_type: "document".to_string(),
            owner: "alice".to_string(),
            contents: json!({}),
            children: HashMap::new(),
            description: None,
        };

        let cmd = Command::new(
            "alice".to_string(),
            "session.read".to_string(),
            block.block_id.clone(),
            json!({}),
        );

        let result = handle_session_read(&cmd, Some(&block));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Expected session block"));
    }
}

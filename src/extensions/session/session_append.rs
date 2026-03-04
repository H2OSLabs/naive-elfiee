use crate::capabilities::core::CapResult;
use crate::models::{Block, Command, Event, EventMode};
use capability_macros::capability;
use std::collections::HashMap;

use super::SessionAppendPayload;

/// Handler for session.append capability.
///
/// Appends an entry to a session block's entries list.
/// Uses EventMode::Append — StateProjector pushes the entry to contents.entries.
///
/// Entry types (per data-model.md §5.3):
/// - `command`: { command, output, exit_code }
/// - `message`: { role, content }
/// - `decision`: { action, related_blocks? }
#[capability(id = "session.append", target = "session")]
fn handle_session_append(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for session.append")?;

    let payload: SessionAppendPayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for session.append: {}", e))?;

    // Construct the entry with metadata
    let entry = serde_json::json!({
        "entry_type": payload.entry_type,
        "data": payload.data,
        "timestamp": crate::utils::time::now_utc(),
    });

    // Build event with Append mode
    let attribute = format!("{}/{}", cmd.editor_id, "session.append");
    let mut timestamp = HashMap::new();
    timestamp.insert(cmd.editor_id.clone(), 1_i64); // Placeholder — engine updates

    let event = Event::new_with_mode(
        block.block_id.clone(),
        attribute,
        serde_json::json!({ "entry": entry }),
        timestamp,
        EventMode::Append,
    );

    Ok(vec![event])
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
    fn test_session_append_command_entry() {
        let block = create_session_block();

        let cmd = Command::new(
            "alice".to_string(),
            "session.append".to_string(),
            block.block_id.clone(),
            json!({
                "entry_type": "command",
                "data": {
                    "command": "cargo test",
                    "output": "test result: ok. 241 passed",
                    "exit_code": 0
                }
            }),
        );

        let result = handle_session_append(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.mode, EventMode::Append);
        assert_eq!(event.value["entry"]["entry_type"], "command");
        assert_eq!(event.value["entry"]["data"]["exit_code"], 0);
    }

    #[test]
    fn test_session_append_message_entry() {
        let block = create_session_block();

        let cmd = Command::new(
            "alice".to_string(),
            "session.append".to_string(),
            block.block_id.clone(),
            json!({
                "entry_type": "message",
                "data": {
                    "role": "agent",
                    "content": "I've implemented the login feature."
                }
            }),
        );

        let result = handle_session_append(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value["entry"]["entry_type"], "message");
        assert_eq!(events[0].value["entry"]["data"]["role"], "agent");
    }

    #[test]
    fn test_session_append_decision_entry() {
        let block = create_session_block();

        let cmd = Command::new(
            "alice".to_string(),
            "session.append".to_string(),
            block.block_id.clone(),
            json!({
                "entry_type": "decision",
                "data": {
                    "action": "approve_merge",
                    "related_blocks": ["block-1", "block-2"]
                }
            }),
        );

        let result = handle_session_append(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value["entry"]["entry_type"], "decision");
    }

    #[test]
    fn test_session_append_no_block_fails() {
        let cmd = Command::new(
            "alice".to_string(),
            "session.append".to_string(),
            "nonexistent".to_string(),
            json!({
                "entry_type": "command",
                "data": { "command": "ls" }
            }),
        );

        let result = handle_session_append(&cmd, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Block required"));
    }
}

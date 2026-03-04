use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, EditorDeletePayload, Event};
use capability_macros::capability;

/// Handler for editor.delete capability.
///
/// Removes an editor identity from the file.
/// This is a system-level operation.
/// Authorization: For now, any editor can delete any editor (MVP).
/// In production, we would restrict this to admins or the editor themselves.
#[capability(id = "editor.delete", target = "system")]
fn handle_editor_delete(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>> {
    // Strongly-typed deserialization
    let payload: EditorDeletePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for editor.delete: {}", e))?;

    // Create event marking editor as deleted
    // Empty payload - deletion is signaled by the event type itself (Event Sourcing semantics)
    let event = create_event(
        payload.editor_id.clone(),
        "editor.delete",
        serde_json::json!({}),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_handle_editor_delete() {
        let cmd = Command {
            cmd_id: "cmd-1".to_string(),
            editor_id: "admin".to_string(),
            cap_id: "editor.delete".to_string(),
            block_id: "system".to_string(),
            payload: serde_json::json!({ "editor_id": "bob" }),
            timestamp: Utc::now(),
        };

        let result = handle_editor_delete(&cmd, None);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, "bob");
        assert_eq!(events[0].attribute, "admin/editor.delete");
    }
}

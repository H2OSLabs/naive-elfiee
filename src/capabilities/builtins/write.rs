use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event, WriteBlockPayload};
use capability_macros::capability;

/// Handler for core.write capability.
///
/// Updates structural fields of a block (name, description).
/// block_type is NOT modifiable — it is determined at init/scan time.
/// At least one field must be provided in the payload.
#[capability(id = "core.write", target = "core/*")]
fn handle_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for core.write")?;

    let payload: WriteBlockPayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for core.write: {}", e))?;

    // At least one field must be provided
    if payload.name.is_none() && payload.description.is_none() {
        return Err("core.write requires at least one of: name, description".to_string());
    }

    let mut value = serde_json::Map::new();

    if let Some(name) = &payload.name {
        value.insert("name".to_string(), serde_json::json!(name));
    }

    if let Some(description) = &payload.description {
        value.insert("description".to_string(), serde_json::json!(description));
    }

    let event = create_event(
        block.block_id.clone(),
        "core.write",
        serde_json::Value::Object(value),
        &cmd.editor_id,
        1, // Placeholder — engine actor updates with correct count
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Block, Command};

    #[test]
    fn test_write_name_only() {
        let block = Block::new(
            "Old Name".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "alice".to_string(),
            "core.write".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "name": "New Name"
            }),
        );

        let result = handle_write(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value["name"], "New Name");
        assert!(events[0].value.get("description").is_none());
    }

    #[test]
    fn test_write_description_only() {
        let block = Block::new(
            "Test".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "alice".to_string(),
            "core.write".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "description": "New Description"
            }),
        );

        let result = handle_write(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events[0].value["description"], "New Description");
        assert!(events[0].value.get("name").is_none());
    }

    #[test]
    fn test_write_both_fields() {
        let block = Block::new(
            "Test".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "alice".to_string(),
            "core.write".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "name": "New Name",
                "description": "New Description"
            }),
        );

        let result = handle_write(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events[0].value["name"], "New Name");
        assert_eq!(events[0].value["description"], "New Description");
    }

    #[test]
    fn test_write_empty_payload_fails() {
        let block = Block::new(
            "Test".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "alice".to_string(),
            "core.write".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        let result = handle_write(&cmd, Some(&block));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least one of"));
    }

    #[test]
    fn test_write_no_block_fails() {
        let cmd = Command::new(
            "alice".to_string(),
            "core.write".to_string(),
            "nonexistent".to_string(),
            serde_json::json!({ "name": "x" }),
        );

        let result = handle_write(&cmd, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Block required"));
    }
}

use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, CreateBlockPayload, Event};
use capability_macros::capability;

/// Handler for core.create capability.
///
/// Creates a new block with name, type, owner, and optional description.
///
/// Note: The block parameter is None for create since the block doesn't exist yet.
#[capability(id = "core.create", target = "core/*")]
fn handle_create(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>> {
    // Strongly-typed deserialization
    let payload: CreateBlockPayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for core.create: {}", e))?;

    // Generate new block ID
    let block_id = uuid::Uuid::new_v4().to_string();

    // Create a single event with full initial state
    // Per README.md Part 2: create events contain the full initial state
    let mut initial_contents = serde_json::json!({ "source": payload.source });

    // Document blocks: inject format into contents
    if payload.block_type == "document" {
        if let Some(fmt) = &payload.format {
            initial_contents["format"] = serde_json::json!(fmt);
        }
    }

    // If caller provided initial contents, merge them
    if let Some(user_contents) = &payload.contents {
        if let Some(obj) = user_contents.as_object() {
            for (k, v) in obj {
                initial_contents[k] = v.clone();
            }
        }
    }

    let mut value = serde_json::json!({
        "name": payload.name,
        "type": payload.block_type,
        "owner": cmd.editor_id,
        "contents": initial_contents,
        "children": {}
    });

    // Include description if provided
    if let Some(desc) = &payload.description {
        value["description"] = serde_json::json!(desc);
    }

    let event = create_event(
        block_id.clone(),
        "core.create", // cap_id
        value,
        &cmd.editor_id,
        1, // Placeholder - engine actor updates with correct count (actor.rs:227)
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Command;

    #[test]
    fn test_create_basic() {
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "document"
            }),
        );

        let result = handle_create(&cmd, None);
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.value["name"], "Test Block");
        assert_eq!(event.value["type"], "document");
        assert_eq!(event.value["owner"], "alice");

        // Verify source is injected into contents
        assert_eq!(event.value["contents"]["source"], "outline");

        // No description when not provided
        assert!(event.value.get("description").is_none());
    }

    #[test]
    fn test_create_with_description() {
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "document",
                "description": "测试描述"
            }),
        );

        let result = handle_create(&cmd, None);
        assert!(result.is_ok());

        let events = result.unwrap();
        let event = &events[0];

        assert_eq!(event.value["description"], "测试描述");
    }

    #[test]
    fn test_create_without_description() {
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "document"
            }),
        );

        let result = handle_create(&cmd, None);
        assert!(result.is_ok());

        let events = result.unwrap();
        let event = &events[0];

        // description field should not exist when not provided
        assert!(event.value.get("description").is_none());
    }
}

/// Capability: task.write
///
/// Writes structured fields to a task block's contents.
/// Contents structure (data-model.md §5.2):
/// ```json
/// { "description": "...", "status": "...", "assigned_to": "...", "template": "..." }
/// ```
/// Only non-None payload fields are merged (partial update).
use super::TaskWritePayload;
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;

#[capability(id = "task.write", target = "task")]
fn handle_task_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for task.write")?;

    if block.block_type != "task" {
        return Err(format!("Expected task block, got '{}'", block.block_type));
    }

    let payload: TaskWritePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for task.write: {}", e))?;

    // At least one field must be provided
    if payload.description.is_none()
        && payload.status.is_none()
        && payload.assigned_to.is_none()
        && payload.template.is_none()
    {
        return Err(
            "task.write requires at least one field (description, status, assigned_to, template)"
                .to_string(),
        );
    }

    // Merge non-None fields into existing contents
    let mut new_contents = if let Some(obj) = block.contents.as_object() {
        obj.clone()
    } else {
        serde_json::Map::new()
    };

    if let Some(desc) = &payload.description {
        new_contents.insert("description".to_string(), serde_json::json!(desc));
    }
    if let Some(status) = &payload.status {
        new_contents.insert("status".to_string(), serde_json::json!(status));
    }
    if let Some(assigned_to) = &payload.assigned_to {
        new_contents.insert("assigned_to".to_string(), serde_json::json!(assigned_to));
    }
    if let Some(template) = &payload.template {
        new_contents.insert("template".to_string(), serde_json::json!(template));
    }

    let event = create_event(
        block.block_id.clone(),
        "task.write",
        serde_json::json!({
            "contents": new_contents
        }),
        &cmd.editor_id,
        1, // Placeholder — engine actor updates with correct count
    );

    Ok(vec![event])
}

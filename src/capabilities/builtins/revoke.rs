use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event, RevokePayload};
use capability_macros::capability;

/// Handler for core.revoke capability.
///
/// Revokes a capability from an editor for a specific block (or wildcard).
#[capability(id = "core.revoke", target = "core/*")]
fn handle_revoke(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>> {
    // Strongly-typed deserialization
    let payload: RevokePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for core.revoke: {}", e))?;

    // Create revoke event
    // Entity is the target block (or "*" for wildcard)
    let event = create_event(
        payload.target_block.clone(),
        "core.revoke", // cap_id
        serde_json::json!({
            "editor": payload.target_editor,
            "capability": payload.capability,
            "block": payload.target_block,
        }),
        &cmd.editor_id,
        1, // Placeholder - engine actor updates with correct count (actor.rs:227)
    );

    Ok(vec![event])
}

use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event, UnlinkBlockPayload, RELATION_IMPLEMENT};
use capability_macros::capability;

/// Handler for core.unlink capability.
///
/// Removes a link between two blocks by removing the target from the relation.
/// Only the `implement` relation type is allowed; other types are rejected.
#[capability(id = "core.unlink", target = "core/*")]
fn handle_unlink(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for core.unlink")?;

    // Strongly-typed deserialization
    let payload: UnlinkBlockPayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for core.unlink: {}", e))?;

    // Restrict to implement relation only
    if payload.relation != RELATION_IMPLEMENT {
        return Err(format!(
            "Only '{}' relation is allowed, got '{}'",
            RELATION_IMPLEMENT, payload.relation
        ));
    }

    // Update children HashMap - remove the target
    let mut new_children = block.children.clone();
    if let Some(targets) = new_children.get_mut(&payload.relation) {
        targets.retain(|id| id != &payload.target_id);
        // Remove the relation key if no targets left
        if targets.is_empty() {
            new_children.remove(&payload.relation);
        }
    }

    // Create event
    let event = create_event(
        block.block_id.clone(),
        "core.unlink", // cap_id
        serde_json::json!({ "children": new_children }),
        &cmd.editor_id,
        1, // Placeholder - engine actor updates with correct count (actor.rs:227)
    );

    Ok(vec![event])
}

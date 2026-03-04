use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event, LinkBlockPayload, RELATION_IMPLEMENT};
use capability_macros::capability;

/// Handler for core.link capability.
///
/// Links two blocks together by adding an `implement` relation to the block's children.
/// Only the `implement` relation type is allowed; other types are rejected.
/// Duplicate links (same source→target) are prevented.
#[capability(id = "core.link", target = "core/*")]
fn handle_link(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for core.link")?;

    // Strongly-typed deserialization
    let payload: LinkBlockPayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for core.link: {}", e))?;

    // Restrict to implement relation only
    if payload.relation != RELATION_IMPLEMENT {
        return Err(format!(
            "Only '{}' relation is allowed, got '{}'",
            RELATION_IMPLEMENT, payload.relation
        ));
    }

    // Prevent duplicate links
    if let Some(existing_targets) = block.children.get(RELATION_IMPLEMENT) {
        if existing_targets.contains(&payload.target_id) {
            return Err(format!(
                "Duplicate link: {} → {} already exists",
                block.block_id, payload.target_id
            ));
        }
    }

    // Update children HashMap
    let mut new_children = block.children.clone();
    new_children
        .entry(payload.relation)
        .or_default()
        .push(payload.target_id);

    // Create event
    let event = create_event(
        block.block_id.clone(),
        "core.link", // cap_id
        serde_json::json!({ "children": new_children }),
        &cmd.editor_id,
        1, // Placeholder - engine actor updates with correct count (actor.rs:227)
    );

    Ok(vec![event])
}

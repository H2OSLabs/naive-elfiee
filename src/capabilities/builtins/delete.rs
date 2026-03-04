use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;

/// Handler for core.delete capability.
///
/// Deletes a block from the active state projection.
///
/// # Event Sourcing Semantics
/// This capability follows pure Event Sourcing principles with two-layer deletion:
///
/// ## Event Store Layer (Soft Delete)
/// - The deletion event is permanently preserved in the immutable event log
/// - Complete history including deleted blocks can be reconstructed via event replay
/// - Enables audit trails and recovery of deleted content
///
/// ## StateProjector Layer (Hard Delete)
/// - The block is removed from the in-memory active state (`blocks` HashMap)
/// - Only current active blocks are kept in the projection for query efficiency
/// - Deleted blocks do not appear in queries of current state
///
/// ## Recovery
/// To recover a deleted block:
/// 1. Query Event Store for all events related to the block
/// 2. Replay events in a temporary StateProjector up to the deletion event
/// 3. Reconstruct the block's last state before deletion
///
/// This design separates concerns:
/// - Event = single source of truth (permanently preserved)
/// - StateProjector = current active state only (rebuildable projection)
#[capability(id = "core.delete", target = "core/*")]
fn handle_delete(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for core.delete")?;

    // Generate deletion event
    // Empty payload - deletion is signaled by the event type itself
    let event = create_event(
        block.block_id.clone(),
        "core.delete",
        serde_json::json!({}), // Empty payload
        &cmd.editor_id,
        1, // Placeholder - updated by engine actor (actor.rs:329)
    );

    Ok(vec![event])
}

use crate::capabilities::grants::GrantsTable;
use crate::models::{Block, Command, Event};
use std::collections::HashMap;

/// Result type for capability execution
pub type CapResult<T> = Result<T, String>;

/// Core trait for capability handlers.
///
/// All capabilities must implement this trait. Use the `#[capability]` macro
/// to avoid boilerplate code.
pub trait CapabilityHandler: Send + Sync {
    /// Unique capability ID (e.g., "core.link", "document.write")
    fn cap_id(&self) -> &str;

    /// Target block type pattern (e.g., "core/*", "document")
    fn target(&self) -> &str;

    /// Check if an editor is authorized to execute this capability.
    ///
    /// This is the sole authorization entry point (pure event-sourcing).
    /// Authorization is derived from two event-sourced layers:
    /// 1. Owner check — block.owner from core.create events
    /// 2. Grant check — from core.grant/core.revoke events via GrantsTable
    ///
    /// When block=None (create-type or wildcard operations), checks wildcard grant.
    /// Every operation requires authorization — no exceptions.
    fn certificator(&self, editor_id: &str, block: Option<&Block>, grants: &GrantsTable) -> bool {
        match block {
            Some(b) => {
                // 1. Owner check: block owner has all capabilities
                if b.owner == editor_id {
                    return true;
                }
                // 2. Grant check (specific block or wildcard)
                grants.has_grant(editor_id, self.cap_id(), &b.block_id)
            }
            None => {
                // No target block (create, wildcard grant/revoke, editor ops)
                // Must have wildcard grant for this capability
                grants.has_grant(editor_id, self.cap_id(), "*")
            }
        }
    }

    /// Execute the capability and return events to be appended.
    ///
    /// This method contains the actual logic of the capability.
    ///
    /// # Arguments
    /// * `cmd` - The command to execute
    /// * `block` - The target block (None for capabilities like core.create that create new blocks)
    fn handler(&self, cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>>;
}

/// Helper function to create a standard Event with vector clock.
///
/// The attribute is automatically formatted as `{editor_id}/{cap_id}` per EAVT spec.
/// This simplifies event creation in capability handlers.
pub fn create_event(
    entity: String,
    cap_id: &str,
    value: serde_json::Value,
    editor_id: &str,
    editor_count: i64,
) -> Event {
    let mut timestamp = HashMap::new();
    timestamp.insert(editor_id.to_string(), editor_count);

    // Format attribute as {editor_id}/{cap_id} per README.md Part 2
    let attribute = format!("{}/{}", editor_id, cap_id);

    Event::new(entity, attribute, value, timestamp)
}

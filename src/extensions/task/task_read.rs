/// Capability: task.read
///
/// Permission gate for reading task block contents.
/// Returns empty events — actual data retrieval happens via query commands
/// (`get_block` / `get_all_blocks`) which bypass the capability handler layer.
///
/// This capability exists for CBAC permission checking.
use crate::capabilities::core::CapResult;
use crate::models::{Block, Command, Event};
use capability_macros::capability;

#[capability(id = "task.read", target = "task")]
fn handle_task_read(_cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for task.read")?;

    if block.block_type != "task" {
        return Err(format!("Expected task block, got '{}'", block.block_type));
    }

    // Permission-only capability — no events generated
    Ok(vec![])
}

use crate::capabilities::core::CapResult;
use crate::models::{Block, Command, Event};
use capability_macros::capability;

/// Handler for document.read capability.
///
/// Permission gate for reading document block contents.
/// Actual data retrieval happens via the query layer (get_block / get_all_blocks).
/// This handler returns an empty event list since reads are side-effect free.
#[capability(id = "document.read", target = "document")]
fn handle_document_read(_cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for document.read")?;

    if block.block_type != "document" {
        return Err(format!(
            "Expected document block, got '{}'",
            block.block_type
        ));
    }

    Ok(vec![])
}

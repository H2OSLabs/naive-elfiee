use serde::{Deserialize, Serialize};

/// Represents a capability grant in the CBAC system.
///
/// Grants define which editors have which capabilities on which blocks.
/// They are projected from grant/revoke events in the EventStore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grant {
    /// The editor who has been granted the capability
    pub editor_id: String,

    /// The capability that has been granted (e.g., "document.write", "core.delete")
    pub cap_id: String,

    /// The target block ID, or "*" for wildcard (all blocks)
    pub block_id: String,
}

impl Grant {
    /// Create a new Grant
    pub fn new(editor_id: String, cap_id: String, block_id: String) -> Self {
        Self {
            editor_id,
            cap_id,
            block_id,
        }
    }

    /// Check if this grant applies to a specific block
    pub fn applies_to_block(&self, block_id: &str) -> bool {
        self.block_id == block_id || self.block_id == "*"
    }
}

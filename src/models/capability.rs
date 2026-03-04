use serde::{Deserialize, Serialize};

/// Metadata for a capability.
/// Actual handler functions are registered separately.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub cap_id: String,
    pub name: String,
    pub target: String, // block_type this capability applies to
}

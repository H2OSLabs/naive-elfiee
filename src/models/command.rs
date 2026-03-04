use crate::utils::time;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub cmd_id: String,
    pub editor_id: String,
    pub cap_id: String,
    pub block_id: String,
    pub payload: serde_json::Value,
    /// UTC timestamp when the command was created (timezone-aware)
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Command {
    pub fn new(
        editor_id: String,
        cap_id: String,
        block_id: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            cmd_id: uuid::Uuid::new_v4().to_string(),
            editor_id,
            cap_id,
            block_id,
            payload,
            timestamp: time::now_utc_datetime(),
        }
    }
}

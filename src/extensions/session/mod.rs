/// Session Extension
///
/// Provides capabilities for session blocks in Elfiee.
/// Session blocks record execution history using append-only semantics.
///
/// ## Capabilities
///
/// - `session.append`: Append an entry to a session block
/// - `session.read`: Read session content (permission gate)
///
/// ## Contents Schema (data-model.md §5.3)
///
/// ```json
/// {
///   "entries": [
///     { "entry_type": "command", "data": { "command": "...", "output": "...", "exit_code": 0 }, "timestamp": "..." },
///     { "entry_type": "message", "data": { "role": "agent", "content": "..." }, "timestamp": "..." },
///     { "entry_type": "decision", "data": { "action": "...", "related_blocks": [...] }, "timestamp": "..." }
///   ]
/// }
/// ```
///
/// ## Entry Types
///
/// - `command`: Command execution record (command + output + exit_code)
/// - `message`: Conversation message (role + content)
/// - `decision`: Decision marker (action + related_blocks)
use serde::{Deserialize, Serialize};

// ============================================================================
// Module Exports
// ============================================================================

pub mod session_append;
pub use session_append::*;

pub mod session_read;
pub use session_read::*;

// ============================================================================
// Payload Definitions
// ============================================================================

/// Payload for session.append capability
///
/// Appends a single entry to a session block's entries list.
/// Uses EventMode::Append — each event adds one entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAppendPayload {
    /// entry 类型：command / message / decision
    pub entry_type: String,
    /// entry 内容（类型相关的 JSON）
    pub data: serde_json::Value,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_session_append_payload_command() {
        let json_val = json!({
            "entry_type": "command",
            "data": {
                "command": "cargo test",
                "output": "ok",
                "exit_code": 0
            }
        });

        let payload: Result<SessionAppendPayload, _> = serde_json::from_value(json_val);
        assert!(payload.is_ok());
        let p = payload.unwrap();
        assert_eq!(p.entry_type, "command");
        assert_eq!(p.data["exit_code"], 0);
    }

    #[test]
    fn test_session_append_payload_message() {
        let json_val = json!({
            "entry_type": "message",
            "data": {
                "role": "human",
                "content": "Please fix the bug"
            }
        });

        let payload: Result<SessionAppendPayload, _> = serde_json::from_value(json_val);
        assert!(payload.is_ok());
        let p = payload.unwrap();
        assert_eq!(p.entry_type, "message");
        assert_eq!(p.data["role"], "human");
    }
}

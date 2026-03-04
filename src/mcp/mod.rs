//! MCP (Model Context Protocol) Server Module
//!
//! This module implements the MCP server for Elfiee, allowing AI agents
//! and other clients to interact with .elf projects through a standardized protocol.
//!
//! ## Architecture
//!
//! Elfiee is a pure backend server exposing MCP SSE and CLI interfaces.
//!
//! ```text
//! +----------------------------+
//! |  Elfiee Core               |
//! |  MCP SSE Server (:47200)   |
//! |  EngineManager             |
//! |  Arc<AppState>             |
//! +----------------------------+
//!       ^         ^
//!       |         |
//!     Agent     CLI
//! ```
//!
//! Each MCP connection has per-connection identity via `elfiee_auth`.
//!
//! ## Available Tools
//!
//! ### Connection Management
//! - `elfiee_auth` - Authenticate this connection (bind editor_id)
//! - `elfiee_open` - Open/create an .elf project
//! - `elfiee_close` - Close an .elf project
//!
//! ### File & Block Operations
//! - `elfiee_file_list` - List open projects
//! - `elfiee_block_list` - List blocks in a project
//! - `elfiee_block_get` - Get block details
//! - `elfiee_block_create` - Create new block
//! - `elfiee_block_delete` - Delete block
//! - `elfiee_block_rename` - Rename block
//! - `elfiee_block_link` - Add block relation
//! - `elfiee_block_unlink` - Remove block relation
//! - `elfiee_document_read/write` - Read/write document content
//! - `elfiee_session_append` - Append session entry
//! - `elfiee_task_create/write/commit/link` - Task operations
//! - `elfiee_grant/revoke` - Permission operations
//! - `elfiee_editor_create/delete` - Editor operations
//! - `elfiee_exec` - Execute any capability

pub mod server;
pub mod transport;

pub use server::ElfieeMcpServer;
pub use transport::{start_mcp_server, MCP_PORT};

use rmcp::ErrorData as McpError;

/// MCP error: connection not authenticated
pub fn not_authenticated() -> McpError {
    McpError::invalid_request(
        "Not authenticated. Call elfiee_auth with your editor_id first.".to_string(),
        None,
    )
}

/// MCP error: project not open
pub fn project_not_open(project: &str) -> McpError {
    McpError::invalid_request(
        format!(
            "Project '{}' is not open. \
            Use elfiee_open to open it, or elfiee_file_list to see currently open projects.",
            project
        ),
        None,
    )
}

/// MCP error: block not found
pub fn block_not_found(block_id: &str) -> McpError {
    McpError::invalid_request(
        format!(
            "Block '{}' not found. Use elfiee_block_list to see available blocks.",
            block_id
        ),
        None,
    )
}

/// MCP error: engine not found
pub fn engine_not_found(file_id: &str) -> McpError {
    McpError::invalid_request(
        format!(
            "Engine not running for file '{}'. \
            The file may have been closed. Use elfiee_file_list to check open files.",
            file_id
        ),
        None,
    )
}

/// MCP error: invalid payload
pub fn invalid_payload(err: impl std::fmt::Display) -> McpError {
    McpError::invalid_params(
        format!(
            "Invalid payload: {}. Check the tool's parameter schema for required fields.",
            err
        ),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ErrorCode;

    #[test]
    fn test_not_authenticated_error() {
        let err = not_authenticated();
        assert_eq!(err.code, ErrorCode::INVALID_REQUEST);
        let msg = err.message.to_string();
        assert!(msg.contains("Not authenticated"));
        assert!(msg.contains("elfiee_auth"));
    }

    #[test]
    fn test_project_not_open_error() {
        let err = project_not_open("/tmp/my_project");
        assert_eq!(err.code, ErrorCode::INVALID_REQUEST);
        let msg = err.message.to_string();
        assert!(msg.contains("/tmp/my_project"));
        assert!(msg.contains("elfiee_open"));
    }

    #[test]
    fn test_block_not_found_error() {
        let err = block_not_found("block-123");
        assert_eq!(err.code, ErrorCode::INVALID_REQUEST);
        let msg = err.message.to_string();
        assert!(msg.contains("block-123"));
        assert!(msg.contains("elfiee_block_list"));
    }

    #[test]
    fn test_engine_not_found_error() {
        let err = engine_not_found("file-abc");
        assert_eq!(err.code, ErrorCode::INVALID_REQUEST);
        let msg = err.message.to_string();
        assert!(msg.contains("file-abc"));
        assert!(msg.contains("elfiee_file_list"));
    }

    #[test]
    fn test_invalid_payload_error() {
        let err = invalid_payload("missing field 'name'");
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        let msg = err.message.to_string();
        assert!(msg.contains("missing field 'name'"));
        assert!(msg.contains("parameter schema"));
    }
}

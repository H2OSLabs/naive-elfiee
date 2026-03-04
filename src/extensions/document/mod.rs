/// Document Extension
///
/// Provides capabilities for document blocks in Elfiee.
/// Unified replacement for the former separate markdown and code extensions.
///
/// ## Capabilities
///
/// - `document.write`: Write text content to a document block
/// - `document.read`: Read document content (permission gate)
///
/// ## Contents Schema (data-model.md §5.1)
///
/// ```json
/// {
///   "format": "md",          // 文件格式标识（创建时必填）
///   "content": "# Hello",    // 文本内容（文本格式时）
///   "path": "src/auth.rs",   // 对应的项目文件路径（可选）
///   "hash": "sha256:...",    // 内容 hash（二进制格式时）
///   "size": 102400,          // 文件大小（二进制格式时）
///   "mime": "image/png"      // MIME 类型（二进制格式时）
/// }
/// ```
use serde::{Deserialize, Serialize};

// ============================================================================
// Module Exports
// ============================================================================

pub mod document_write;
pub use document_write::*;

pub mod document_read;
pub use document_read::*;

// ============================================================================
// Payload Definitions
// ============================================================================

/// Payload for document.write capability
///
/// Contains text content for a document block.
/// Stored in `contents` as `{ "content": "..." }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentWritePayload {
    /// 文本内容
    pub content: String,
}

/// Payload for document.read capability
///
/// document.read is a permission-only capability.
/// No payload fields needed — the empty JSON object `{}` is accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentReadPayload {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests;

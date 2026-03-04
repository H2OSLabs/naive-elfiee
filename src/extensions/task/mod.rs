/// Task Extension
///
/// Provides capabilities for managing task blocks in Elfiee.
///
/// ## Capabilities
///
/// - `task.write`: Write structured fields to a task block
/// - `task.read`: Read task content (permission gate)
/// - `task.commit`: Generate audit event for committing task's downstream blocks
///
/// ## Contents Schema (data-model.md §5.2)
///
/// ```json
/// {
///   "description": "为项目添加 OAuth2 登录",
///   "status": "pending",
///   "assigned_to": "coder-agent",
///   "template": "code-review"
/// }
/// ```
///
/// Title is `block.name`, managed via `core.write`.
use serde::{Deserialize, Serialize};

// ============================================================================
// Module Exports
// ============================================================================

pub mod task_write;
pub use task_write::*;

pub mod task_read;
pub use task_read::*;

pub mod task_commit;
pub use task_commit::*;

// ============================================================================
// Payload Definitions
// ============================================================================

/// Payload for task.write capability
///
/// Updates structured fields of a task block.
/// Only non-None fields are merged into contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWritePayload {
    /// 任务描述
    #[serde(default)]
    pub description: Option<String>,
    /// 任务状态：pending / in_progress / completed / failed
    #[serde(default)]
    pub status: Option<String>,
    /// 分配给的 editor_id
    #[serde(default)]
    pub assigned_to: Option<String>,
    /// 使用的工作模板
    #[serde(default)]
    pub template: Option<String>,
}

/// Payload for task.read capability
///
/// task.read is a permission-only capability.
/// No payload fields needed — the empty JSON object `{}` is accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskReadPayload {}

/// Payload for task.commit capability
///
/// Empty payload — records an audit event marking the task as committed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCommitPayload {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests;

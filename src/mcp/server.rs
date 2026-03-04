//! MCP Server Implementation
//!
//! Uses rmcp's macro system for clean tool definitions.
//! Read operations call services layer for unified CBAC filtering.

use crate::mcp;
use crate::models::Command;
use crate::services;
use crate::state::AppState;
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{
        Annotated, CallToolResult, Content, Implementation, ListResourceTemplatesResult,
        ListResourcesResult, PaginatedRequestParam, RawResource, RawResourceTemplate,
        ReadResourceRequestParam, ReadResourceResult, ResourceContents, ResourcesCapability,
        ServerCapabilities, ServerInfo, ToolsCapability,
    },
    service::{RequestContext, RoleServer},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Elfiee MCP Server
///
/// Provides MCP protocol access to Elfiee's capabilities.
/// Each instance is per-connection — `connection_editor_id` tracks
/// the authenticated editor for this specific MCP connection.
#[derive(Clone)]
pub struct ElfieeMcpServer {
    app_state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
    /// Per-connection editor_id, set by `elfiee_auth` tool.
    /// None = not yet authenticated.
    connection_editor_id: Arc<RwLock<Option<String>>>,
}

// ============================================================================
// Tool Input Structures
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectInput {
    /// Path to the .elf project file
    pub project: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockInput {
    /// Path to the .elf project file
    pub project: String,
    /// ID of the block
    pub block_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockCreateInput {
    /// Path to the .elf project file
    pub project: String,
    /// Name of the new block
    pub name: String,
    /// Block type: document, task, session
    pub block_type: String,
    /// Optional parent block ID to link to
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockRenameInput {
    /// Path to the .elf project file
    pub project: String,
    /// ID of the block to rename
    pub block_id: String,
    /// New name for the block
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockLinkInput {
    /// Path to the .elf project file
    pub project: String,
    /// Parent block ID
    pub parent_id: String,
    /// Child block ID
    pub child_id: String,
    /// Relation type (e.g., 'contains', 'references')
    pub relation: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockUnlinkInput {
    /// Path to the .elf project file
    pub project: String,
    /// Parent block ID
    pub parent_id: String,
    /// Child block ID
    pub child_id: String,
    /// Relation type to remove
    pub relation: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GrantInput {
    /// Path to the .elf project file
    pub project: String,
    /// Block ID to grant permission on
    pub block_id: String,
    /// Editor ID to grant permission to
    pub editor_id: String,
    /// Capability ID to grant (e.g., 'document.write')
    pub cap_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditorInput {
    /// Path to the .elf project file
    pub project: String,
    /// Editor ID
    pub editor_id: String,
    /// Optional display name for the editor
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthInput {
    /// Editor ID to bind to this connection.
    /// Obtain from `elf register` or config.toml `[editor] default`.
    pub editor_id: String,
    /// Project path (optional). If provided, returns the Skill for this project.
    pub project: Option<String>,
    /// Role name (optional). Loads role-specific skill if available.
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StateAtEventInput {
    /// Path to the .elf project file
    pub project: String,
    /// Block ID to get state for
    pub block_id: String,
    /// Event ID to replay up to (time travel target)
    pub event_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenProjectInput {
    /// Path to the .elf project directory.
    /// If it doesn't exist, a new project will be created.
    pub project: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExecInput {
    /// Path to the .elf project file
    pub project: String,
    /// Capability ID (e.g., 'document.write')
    pub capability: String,
    /// Target block ID
    pub block_id: Option<String>,
    /// Capability-specific payload
    pub payload: Option<serde_json::Value>,
}

// ============================================================================
// MCP Server Implementation
// ============================================================================

#[tool_router]
impl ElfieeMcpServer {
    /// Create a new MCP server instance.
    /// Each connection gets its own instance with isolated `connection_editor_id`.
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self {
            app_state,
            tool_router: Self::tool_router(),
            connection_editor_id: Arc::new(RwLock::new(None)),
        }
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Get file_id from project path
    fn get_file_id(&self, project: &str) -> Result<String, McpError> {
        let files = self.app_state.list_open_files();
        for (file_id, path) in &files {
            if path == project {
                return Ok(file_id.clone());
            }
        }
        Err(mcp::project_not_open(project))
    }

    /// Get the authenticated editor_id for this connection.
    ///
    /// Returns error if `elfiee_auth` has not been called yet.
    fn get_connection_editor_id(&self) -> Result<String, McpError> {
        self.connection_editor_id
            .try_read()
            .ok()
            .and_then(|guard| guard.clone())
            .ok_or_else(mcp::not_authenticated)
    }

    /// Get engine handle for a file
    fn get_engine(&self, file_id: &str) -> Result<crate::engine::EngineHandle, McpError> {
        self.app_state
            .engine_manager
            .get_engine(file_id)
            .ok_or_else(|| mcp::engine_not_found(file_id))
    }

    /// Execute a capability and return rich result with updated state
    async fn execute_capability(
        &self,
        project: &str,
        capability: &str,
        block_id: Option<String>,
        payload: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(project)?;
        let editor_id = self.get_connection_editor_id()?;
        let handle = self.get_engine(&file_id)?;

        let target_block_id = block_id.clone().unwrap_or_default();
        let cmd = Command::new(
            editor_id.clone(),
            capability.to_string(),
            target_block_id.clone(),
            payload,
        );

        match services::block::execute_command(&handle, cmd).await {
            Ok(events) => {
                // Notify frontend of state change
                let _ = self.app_state.state_changed_tx.send(file_id.clone());

                let mut result = json!({
                    "ok": true,
                    "capability": capability,
                    "editor": editor_id,
                    "events_committed": events.len(),
                });

                // For create operations, extract the new block_id from events
                if capability == "core.create" {
                    if let Some(ev) = events.first() {
                        result["created_block_id"] = json!(ev.entity);
                        // Fetch the newly created block for full details
                        if let Some(block) = handle.get_block(ev.entity.clone()).await {
                            result["block"] = Self::format_block_summary(&block);
                        }
                    }
                } else if !target_block_id.is_empty() {
                    // For other operations, fetch the updated block state
                    if let Some(block) = handle.get_block(target_block_id.clone()).await {
                        result["block"] = Self::format_block_summary(&block);
                    }
                }

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            Err(e) => {
                let error_msg = e.to_string();
                let hint = Self::error_hint(capability, &error_msg);
                let result = json!({
                    "ok": false,
                    "capability": capability,
                    "error": error_msg,
                    "hint": hint,
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
        }
    }

    /// Format a block into a concise, informative summary
    fn format_block_summary(block: &crate::models::Block) -> serde_json::Value {
        let mut summary = json!({
            "block_id": block.block_id,
            "name": block.name,
            "block_type": block.block_type,
            "owner": block.owner,
        });

        // Content preview (type-specific)
        match block.block_type.as_str() {
            "document" => {
                if let Some(content) = block.contents.get("content").and_then(|v| v.as_str()) {
                    let preview = if content.len() > 200 {
                        format!("{}...", &content[..200])
                    } else {
                        content.to_string()
                    };
                    summary["content_preview"] = json!(preview);
                    summary["content_length"] = json!(content.len());
                }
                if let Some(fmt) = block.contents.get("format").and_then(|v| v.as_str()) {
                    summary["format"] = json!(fmt);
                }
            }
            "task" => {
                if let Some(desc) = block.contents.get("description").and_then(|v| v.as_str()) {
                    let preview = if desc.len() > 200 {
                        format!("{}...", &desc[..200])
                    } else {
                        desc.to_string()
                    };
                    summary["content_preview"] = json!(preview);
                }
                if let Some(status) = block.contents.get("status").and_then(|v| v.as_str()) {
                    summary["status"] = json!(status);
                }
                if let Some(assigned) = block.contents.get("assigned_to").and_then(|v| v.as_str()) {
                    summary["assigned_to"] = json!(assigned);
                }
            }
            "session" => {
                if let Some(entries) = block.contents.get("entries").and_then(|v| v.as_array()) {
                    summary["entry_count"] = json!(entries.len());
                }
            }
            _ => {}
        }

        // Children relations
        if !block.children.is_empty() {
            let relations: serde_json::Value = block
                .children
                .iter()
                .map(|(rel, ids)| (rel.clone(), json!(ids)))
                .collect::<serde_json::Map<String, serde_json::Value>>()
                .into();
            summary["children"] = relations;
        }

        // Description
        if let Some(desc) = &block.description {
            summary["description"] = json!(desc);
        }

        summary
    }

    /// Provide actionable hints for common errors
    fn error_hint(capability: &str, error: &str) -> String {
        let lower = error.to_lowercase();
        if lower.contains("not authorized") || lower.contains("permission") {
            return format!(
                "The current editor lacks '{}' permission. Use elfiee_grant to grant it first.",
                capability
            );
        }
        if lower.contains("not found") {
            return "The target block does not exist. Use elfiee_block_list to see available blocks."
                .to_string();
        }
        if lower.contains("type") && lower.contains("mismatch") {
            return format!(
                "This block's type doesn't support '{}'. Check the block_type with elfiee_block_get.",
                capability
            );
        }
        if lower.contains("payload") || lower.contains("invalid") {
            return "The request payload is malformed. Check the required fields for this tool."
                .to_string();
        }
        "Check the error message above. Use elfiee_block_get to inspect the block's current state."
            .to_string()
    }

    // ========================================================================
    // Connection Management
    // ========================================================================

    /// Authenticate this MCP connection
    #[tool(
        description = "Authenticate this MCP connection by binding an editor_id. Must be called before any write operations (create, write, delete, grant, etc.). The editor_id should be obtained from `elf register` or from your project's config.toml. Optionally pass `project` path and `role` to receive the Skill guide. Read-only operations like elfiee_file_list and elfiee_block_list work without authentication."
    )]
    async fn elfiee_auth(
        &self,
        Parameters(input): Parameters<AuthInput>,
    ) -> Result<CallToolResult, McpError> {
        let mut editor_id = self.connection_editor_id.write().await;
        *editor_id = Some(input.editor_id.clone());

        let mut result = json!({
            "authenticated": true,
            "editor_id": input.editor_id,
            "hint": "You can now perform write operations. Use elfiee_file_list to see open projects, or elfiee_open to open a project."
        });

        // 如果提供了 project，尝试加载 Skill
        if let Some(project_path) = &input.project {
            let project_dir = std::path::Path::new(project_path);
            if project_dir.join(".elf").exists() {
                if let Ok(elf_project) = crate::elf_project::ElfProject::open(project_dir) {
                    let skill = elf_project.read_skill(input.role.as_deref());
                    result["skill"] = json!(skill);
                }
            }
        }

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    /// Open an .elf project
    #[tool(
        description = "Open an .elf project directory. Creates the project if it doesn't exist. Must be called before other operations on the project. Use elfiee_file_list to see already open projects. Returns the Skill guide if available."
    )]
    async fn elfiee_open(
        &self,
        Parameters(input): Parameters<OpenProjectInput>,
    ) -> Result<CallToolResult, McpError> {
        match crate::services::project::open_project(&input.project, &self.app_state).await {
            Ok(file_id) => {
                let mut result = json!({
                    "ok": true,
                    "project": input.project,
                    "file_id": file_id,
                });

                // Include block count
                if let Some(handle) = self.app_state.engine_manager.get_engine(&file_id) {
                    let blocks = handle.get_all_blocks().await;
                    result["block_count"] = json!(blocks.len());
                }

                // Include skill if project has one
                let project_dir = std::path::Path::new(&input.project);
                if project_dir.join(".elf").exists() {
                    if let Ok(elf_project) = crate::elf_project::ElfProject::open(project_dir) {
                        let skill = elf_project.read_skill(None);
                        result["skill"] = json!(skill);
                    }
                }

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "ok": false,
                    "error": e,
                }))
                .unwrap(),
            )])),
        }
    }

    /// Close an .elf project
    #[tool(
        description = "Close an .elf project and release its resources. The project can be reopened later with elfiee_open."
    )]
    async fn elfiee_close(
        &self,
        Parameters(input): Parameters<ProjectInput>,
    ) -> Result<CallToolResult, McpError> {
        match crate::services::project::close_project(&input.project, &self.app_state).await {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "project": input.project,
                    "closed": true,
                }))
                .unwrap(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "ok": false,
                    "error": e,
                }))
                .unwrap(),
            )])),
        }
    }

    // ========================================================================
    // File Operations
    // ========================================================================

    /// List all currently open .elf projects
    #[tool(
        description = "List all currently open .elf project files. Returns file paths and block counts. Use the 'project' path from results as input for other tools. Use elfiee_open to open a project first."
    )]
    async fn elfiee_file_list(&self) -> Result<CallToolResult, McpError> {
        let files = self.app_state.list_open_files();

        if files.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "files": [],
                    "count": 0,
                    "hint": "No .elf projects are currently open. Use elfiee_open to open a project."
                }))
                .unwrap(),
            )]));
        }

        let mut result = Vec::new();
        for (file_id, path) in &files {
            let mut file_info = json!({
                "project": path,
                "file_id": file_id,
            });

            // Add connection's authenticated editor_id
            if let Ok(editor_id) = self.get_connection_editor_id() {
                file_info["connection_editor"] = json!(editor_id);
            }

            // Add block count
            if let Some(handle) = self.app_state.engine_manager.get_engine(file_id) {
                let blocks = handle.get_all_blocks().await;
                file_info["block_count"] = json!(blocks.len());

                // Summarize block types
                let mut type_counts = std::collections::HashMap::new();
                for block in blocks.values() {
                    *type_counts
                        .entry(block.block_type.clone())
                        .or_insert(0usize) += 1;
                }
                if !type_counts.is_empty() {
                    file_info["block_types"] = json!(type_counts);
                }
            }

            result.push(file_info);
        }

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json!({
                "files": result,
                "count": files.len(),
                "hint": "Use the 'project' value as the 'project' parameter for other elfiee tools."
            }))
            .unwrap(),
        )]))
    }

    // ========================================================================
    // Block Operations
    // ========================================================================

    /// List all blocks in a project with summaries (CBAC filtered)
    #[tool(
        description = "List all blocks in a project with type, content preview, relations, and metadata. Results are filtered by your permissions. Use elfiee_block_get for full block details."
    )]
    async fn elfiee_block_list(
        &self,
        Parameters(input): Parameters<ProjectInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let editor_id = self.get_connection_editor_id()?;
        let handle = self.get_engine(&file_id)?;

        let blocks = services::block::list_blocks(&handle, &editor_id).await;
        let result: Vec<serde_json::Value> =
            blocks.iter().map(Self::format_block_summary).collect();

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json!({
                "project": input.project,
                "blocks": result,
                "count": blocks.len(),
            }))
            .unwrap(),
        )]))
    }

    /// Get detailed information about a specific block (CBAC: {block_type}.read)
    #[tool(
        description = "Get full details of a block including all contents, children relations, metadata, and permissions. Requires read permission."
    )]
    async fn elfiee_block_get(
        &self,
        Parameters(input): Parameters<BlockInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let editor_id = self.get_connection_editor_id()?;
        let handle = self.get_engine(&file_id)?;

        match services::block::get_block(&handle, &editor_id, &input.block_id).await {
            Ok(block) => {
                let mut result = json!({
                    "block_id": block.block_id,
                    "name": block.name,
                    "block_type": block.block_type,
                    "owner": block.owner,
                    "contents": block.contents,
                });

                // Children relations
                if block.children.is_empty() {
                    result["children"] = json!({});
                } else {
                    let relations: serde_json::Value = block
                        .children
                        .iter()
                        .map(|(rel, ids)| (rel.clone(), json!(ids)))
                        .collect::<serde_json::Map<String, serde_json::Value>>()
                        .into();
                    result["children"] = relations;
                }

                // Description
                if let Some(desc) = &block.description {
                    result["description"] = json!(desc);
                }

                // Grants on this block
                let grants = services::grant::get_block_grants(&handle, &block.block_id).await;
                if !grants.is_empty() {
                    let grant_list: Vec<serde_json::Value> = grants
                        .iter()
                        .map(|g| json!({ "editor": g.editor_id, "capability": g.cap_id }))
                        .collect();
                    result["grants"] = json!(grant_list);
                }

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "error": e,
                    "hint": "Use elfiee_block_list to see available blocks.",
                }))
                .unwrap(),
            )])),
        }
    }

    /// Create a new block in the project
    #[tool(
        description = "Create a new block (document, task, or session) in the project. Returns the created block with its generated block_id."
    )]
    async fn elfiee_block_create(
        &self,
        Parameters(input): Parameters<BlockCreateInput>,
    ) -> Result<CallToolResult, McpError> {
        let payload = json!({
            "name": input.name,
            "block_type": input.block_type
        });
        self.execute_capability(&input.project, "core.create", input.parent_id, payload)
            .await
    }

    /// Delete a block from the project
    #[tool(
        description = "Soft-delete a block. The block is marked as deleted but its history is preserved in the event store."
    )]
    async fn elfiee_block_delete(
        &self,
        Parameters(input): Parameters<BlockInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.delete",
            Some(input.block_id),
            json!({}),
        )
        .await
    }

    /// Rename a block
    #[tool(description = "Rename a block")]
    async fn elfiee_block_rename(
        &self,
        Parameters(input): Parameters<BlockRenameInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.write",
            Some(input.block_id),
            json!({ "name": input.name }),
        )
        .await
    }

    /// Add a relation between two blocks
    #[tool(description = "Add a relation between two blocks (parent -> child)")]
    async fn elfiee_block_link(
        &self,
        Parameters(input): Parameters<BlockLinkInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.link",
            Some(input.parent_id),
            json!({
                "target_id": input.child_id,
                "relation": input.relation
            }),
        )
        .await
    }

    /// Remove a relation between two blocks
    #[tool(description = "Remove a relation between two blocks")]
    async fn elfiee_block_unlink(
        &self,
        Parameters(input): Parameters<BlockUnlinkInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.unlink",
            Some(input.parent_id),
            json!({
                "target_id": input.child_id,
                "relation": input.relation
            }),
        )
        .await
    }

    // ========================================================================
    // Permission Operations
    // ========================================================================

    /// Grant a capability to an editor on a block
    #[tool(
        description = "Grant a capability (e.g. 'document.write', 'task.write', 'session.append') to an editor on a specific block. The block owner can always perform all operations without explicit grants."
    )]
    async fn elfiee_grant(
        &self,
        Parameters(input): Parameters<GrantInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.grant",
            Some(input.block_id.clone()),
            json!({
                "target_editor": input.editor_id,
                "capability": input.cap_id,
                "target_block": input.block_id
            }),
        )
        .await
    }

    /// Revoke a capability from an editor on a block
    #[tool(
        description = "Revoke a previously granted capability from an editor on a specific block."
    )]
    async fn elfiee_revoke(
        &self,
        Parameters(input): Parameters<GrantInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.revoke",
            Some(input.block_id.clone()),
            json!({
                "target_editor": input.editor_id,
                "capability": input.cap_id,
                "target_block": input.block_id
            }),
        )
        .await
    }

    // ========================================================================
    // Editor Operations
    // ========================================================================

    /// Create a new editor in the project
    #[tool(description = "Create a new editor in the project")]
    async fn elfiee_editor_create(
        &self,
        Parameters(input): Parameters<EditorInput>,
    ) -> Result<CallToolResult, McpError> {
        let name = input.name.unwrap_or_else(|| input.editor_id.clone());
        let payload = json!({
            "name": name,
            "editor_id": input.editor_id,
        });

        self.execute_capability(&input.project, "core.editor_create", None, payload)
            .await
    }

    /// Delete an editor from the project
    #[tool(description = "Delete an editor from the project")]
    async fn elfiee_editor_delete(
        &self,
        Parameters(input): Parameters<EditorInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.editor_delete",
            None,
            json!({ "editor_id": input.editor_id }),
        )
        .await
    }

    // ========================================================================
    // History & Time Travel
    // ========================================================================

    /// Get the event history for a specific block (CBAC: {block_type}.read)
    #[tool(
        description = "Get the full event history for a specific block. Requires {block_type}.read permission (e.g., document.read for document blocks). Returns all events that affected this block, in chronological order."
    )]
    async fn elfiee_block_history(
        &self,
        Parameters(input): Parameters<BlockInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let editor_id = self.get_connection_editor_id()?;
        let handle = self.get_engine(&file_id)?;

        match services::event::get_block_history(&handle, &editor_id, &input.block_id).await {
            Ok(events) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "block_id": input.block_id,
                    "event_count": events.len(),
                    "events": events.iter().map(|e| json!({
                        "event_id": e.event_id,
                        "entity": e.entity,
                        "attribute": e.attribute,
                        "value": e.value,
                        "timestamp": e.timestamp,
                    })).collect::<Vec<_>>(),
                }))
                .unwrap(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "error": e,
                    "hint": "Use elfiee_block_list to see available blocks.",
                }))
                .unwrap(),
            )])),
        }
    }

    /// Time travel: get the state of a block at a specific point in time (CBAC: {block_type}.read)
    #[tool(
        description = "Get the state of a block at a specific point in time by replaying events up to the given event_id. Requires {block_type}.read permission. Returns the block state and grants as they were at that moment."
    )]
    async fn elfiee_state_at_event(
        &self,
        Parameters(input): Parameters<StateAtEventInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let editor_id = self.get_connection_editor_id()?;
        let handle = self.get_engine(&file_id)?;

        match services::event::get_state_at_event(
            &handle,
            &editor_id,
            &input.block_id,
            &input.event_id,
        )
        .await
        {
            Ok((block, grants)) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "block": {
                        "block_id": block.block_id,
                        "name": block.name,
                        "block_type": block.block_type,
                        "owner": block.owner,
                        "description": block.description,
                        "contents": block.contents,
                        "children": block.children,
                    },
                    "grants": grants.iter().map(|g| json!({
                        "editor_id": g.editor_id,
                        "cap_id": g.cap_id,
                        "block_id": g.block_id,
                    })).collect::<Vec<_>>(),
                    "at_event": input.event_id,
                }))
                .unwrap(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "error": e,
                    "hint": "Use elfiee_block_list to get block IDs, then get_all_events to find event IDs.",
                }))
                .unwrap(),
            )])),
        }
    }

    // ========================================================================
    // Generic Execution
    // ========================================================================

    /// Execute any registered capability directly
    #[tool(
        description = "Execute any registered capability. Use for extension operations: document.write, task.write, task.commit, session.append, etc. Also works for core operations (core.create, core.link, core.delete, core.grant, core.revoke). Provide capability name, target block_id, and capability-specific payload."
    )]
    async fn elfiee_exec(
        &self,
        Parameters(input): Parameters<ExecInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            &input.capability,
            input.block_id,
            input.payload.unwrap_or(json!({})),
        )
        .await
    }
}

// ============================================================================
// ServerHandler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::handler::server::ServerHandler for ElfieeMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "elfiee-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "Elfiee MCP Server for .elf project operations. \
                1. Call elfiee_auth with your editor_id to authenticate. \
                2. Call elfiee_open to open a project (or elfiee_file_list to see already open ones). \
                3. Use block/document/task/session tools to interact with the project."
                    .to_string(),
            ),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                resources: Some(ResourcesCapability::default()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let files = self.app_state.list_open_files();
        let mut resources = Vec::new();

        // Helper to wrap raw resource
        let resource = |raw: RawResource| Annotated {
            raw,
            annotations: None,
        };

        // Static resource: list of open files
        resources.push(resource(RawResource {
            uri: "elfiee://files".to_string(),
            name: "Open Files".to_string(),
            description: Some("List of currently open .elf project files".to_string()),
            mime_type: Some("application/json".to_string()),
            size: None,
        }));

        // Dynamic resources: per-project blocks and grants
        for (file_id, path) in &files {
            resources.push(resource(RawResource {
                uri: format!("elfiee://{}/blocks", path),
                name: format!("Blocks in {}", path),
                description: Some(format!("All blocks in project {}", path)),
                mime_type: Some("application/json".to_string()),
                size: None,
            }));

            resources.push(resource(RawResource {
                uri: format!("elfiee://{}/grants", path),
                name: format!("Grants in {}", path),
                description: Some(format!("Permission grants in project {}", path)),
                mime_type: Some("application/json".to_string()),
                size: None,
            }));

            resources.push(resource(RawResource {
                uri: format!("elfiee://{}/events", path),
                name: format!("Events in {}", path),
                description: Some(format!("Event log for project {}", path)),
                mime_type: Some("application/json".to_string()),
                size: None,
            }));

            resources.push(resource(RawResource {
                uri: format!("elfiee://{}/editors", path),
                name: format!("Editors in {}", path),
                description: Some(format!("Editor list for project {}", path)),
                mime_type: Some("application/json".to_string()),
                size: None,
            }));

            resources.push(resource(RawResource {
                uri: format!("elfiee://{}/my-tasks", path),
                name: format!("My Tasks in {}", path),
                description: Some(format!(
                    "Tasks assigned to the current editor in project {}",
                    path
                )),
                mime_type: Some("application/json".to_string()),
                size: None,
            }));

            resources.push(resource(RawResource {
                uri: format!("elfiee://{}/my-grants", path),
                name: format!("My Grants in {}", path),
                description: Some(format!(
                    "Permissions granted to the current editor in project {}",
                    path
                )),
                mime_type: Some("application/json".to_string()),
                size: None,
            }));

            // Individual block resources
            if let Some(handle) = self.app_state.engine_manager.get_engine(file_id) {
                let blocks = handle.get_all_blocks().await;
                for block in blocks.values() {
                    let mime = match block.block_type.as_str() {
                        "document" => "text/plain",
                        _ => "application/json",
                    };
                    resources.push(resource(RawResource {
                        uri: format!("elfiee://{}/block/{}", path, block.block_id),
                        name: block.name.clone(),
                        description: Some(format!("[{}] {}", block.block_type, block.name)),
                        mime_type: Some(mime.to_string()),
                        size: None,
                    }));
                }
            }
        }

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        let template = |raw: RawResourceTemplate| Annotated {
            raw,
            annotations: None,
        };

        Ok(ListResourceTemplatesResult {
            resource_templates: vec![
                template(RawResourceTemplate {
                    uri_template: "elfiee://{project}/blocks".to_string(),
                    name: "Project Blocks".to_string(),
                    description: Some("List all blocks in a project".to_string()),
                    mime_type: Some("application/json".to_string()),
                }),
                template(RawResourceTemplate {
                    uri_template: "elfiee://{project}/block/{block_id}".to_string(),
                    name: "Block Content".to_string(),
                    description: Some("Read a specific block's full content".to_string()),
                    mime_type: Some("application/json".to_string()),
                }),
                template(RawResourceTemplate {
                    uri_template: "elfiee://{project}/grants".to_string(),
                    name: "Project Grants".to_string(),
                    description: Some("Permission grants in a project".to_string()),
                    mime_type: Some("application/json".to_string()),
                }),
                template(RawResourceTemplate {
                    uri_template: "elfiee://{project}/events".to_string(),
                    name: "Event Log".to_string(),
                    description: Some("Event sourcing log for a project".to_string()),
                    mime_type: Some("application/json".to_string()),
                }),
                template(RawResourceTemplate {
                    uri_template: "elfiee://{project}/editors".to_string(),
                    name: "Editors".to_string(),
                    description: Some("List of editors in a project".to_string()),
                    mime_type: Some("application/json".to_string()),
                }),
                template(RawResourceTemplate {
                    uri_template: "elfiee://{project}/my-tasks".to_string(),
                    name: "My Tasks".to_string(),
                    description: Some("Tasks assigned to the current editor".to_string()),
                    mime_type: Some("application/json".to_string()),
                }),
                template(RawResourceTemplate {
                    uri_template: "elfiee://{project}/my-grants".to_string(),
                    name: "My Grants".to_string(),
                    description: Some("Permissions granted to the current editor".to_string()),
                    mime_type: Some("application/json".to_string()),
                }),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri = &request.uri;

        // Handle static resource: elfiee://files
        if uri == "elfiee://files" {
            let files = self.app_state.list_open_files();
            let result: Vec<serde_json::Value> = files
                .iter()
                .map(|(file_id, path)| {
                    json!({
                        "file_id": file_id,
                        "project": path,
                    })
                })
                .collect();

            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri.clone(),
                    mime_type: Some("application/json".to_string()),
                    text: serde_json::to_string_pretty(&json!({
                        "files": result,
                        "count": files.len(),
                    }))
                    .unwrap(),
                }],
            });
        }

        // Parse URI: elfiee://{project}/...
        let stripped = uri
            .strip_prefix("elfiee://")
            .ok_or_else(|| mcp::invalid_payload("URI must start with elfiee://"))?;

        // Find which project this URI refers to
        let files = self.app_state.list_open_files();
        let (file_id, project, remainder) = files
            .iter()
            .filter_map(|(fid, path)| {
                stripped
                    .strip_prefix(path.as_str())
                    .map(|rest| (fid.clone(), path.clone(), rest.to_string()))
            })
            .next()
            .ok_or_else(|| mcp::project_not_open(&format!("(parsed from URI: {})", uri)))?;

        let remainder = remainder.trim_start_matches('/');
        let handle = self.get_engine(&file_id)?;

        // Get editor_id for CBAC (graceful fallback for resources)
        let editor_id = self.get_connection_editor_id().ok();

        match remainder {
                // elfiee://{project}/blocks — CBAC filtered
                "blocks" => {
                    let blocks = if let Some(ref eid) = editor_id {
                        services::block::list_blocks(&handle, eid).await
                    } else {
                        // Unauthenticated: return empty
                        Vec::new()
                    };
                    let result: Vec<serde_json::Value> = blocks
                        .iter()
                        .map(Self::format_block_summary)
                        .collect();

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("application/json".to_string()),
                            text: serde_json::to_string_pretty(&json!({
                                "project": project,
                                "blocks": result,
                                "count": blocks.len(),
                            }))
                            .unwrap(),
                        }],
                    })
                }

                // elfiee://{project}/grants — CBAC filtered
                "grants" => {
                    let grants = if let Some(ref eid) = editor_id {
                        services::grant::list_grants(&handle, eid).await
                    } else {
                        Vec::new()
                    };
                    let grant_list: Vec<serde_json::Value> = grants
                        .iter()
                        .map(|g| json!({
                            "editor_id": g.editor_id,
                            "capability": g.cap_id,
                            "block_id": g.block_id,
                        }))
                        .collect();

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("application/json".to_string()),
                            text: serde_json::to_string_pretty(&json!({
                                "project": project,
                                "grants": grant_list,
                                "count": grant_list.len(),
                            }))
                            .unwrap(),
                        }],
                    })
                }

                // elfiee://{project}/events — CBAC filtered
                "events" => {
                    let events = if let Some(ref eid) = editor_id {
                        services::event::list_events(&handle, eid)
                            .await
                            .map_err(|e| mcp::invalid_payload(format!("Failed to read events: {}", e)))?
                    } else {
                        Vec::new()
                    };
                    let result: Vec<serde_json::Value> = events
                        .iter()
                        .map(|ev| {
                            json!({
                                "event_id": ev.event_id,
                                "entity": ev.entity,
                                "attribute": ev.attribute,
                                "value": ev.value,
                                "timestamp": ev.timestamp,
                                "created_at": ev.created_at,
                            })
                        })
                        .collect();

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("application/json".to_string()),
                            text: serde_json::to_string_pretty(&json!({
                                "project": project,
                                "events": result,
                                "count": result.len(),
                            }))
                            .unwrap(),
                        }],
                    })
                }

                // elfiee://{project}/block/{block_id} — CBAC checked
                rest if rest.starts_with("block/") => {
                    let block_id = rest.strip_prefix("block/").unwrap();
                    let eid = editor_id.clone().unwrap_or_default();

                    match services::block::get_block(&handle, &eid, block_id).await {
                        Ok(block) => {
                            let (text, mime) = match block.block_type.as_str() {
                                "document" => {
                                    let content = block
                                        .contents
                                        .get("content")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    (content.to_string(), "text/plain".to_string())
                                }
                                _ => {
                                    let full = json!({
                                        "block_id": block.block_id,
                                        "name": block.name,
                                        "block_type": block.block_type,
                                        "owner": block.owner,
                                        "contents": block.contents,
                                        "children": block.children,
                                        "description": block.description,
                                    });
                                    (
                                        serde_json::to_string_pretty(&full).unwrap(),
                                        "application/json".to_string(),
                                    )
                                }
                            };

                            Ok(ReadResourceResult {
                                contents: vec![ResourceContents::TextResourceContents {
                                    uri: uri.clone(),
                                    mime_type: Some(mime),
                                    text,
                                }],
                            })
                        }
                        Err(e) => Err(mcp::invalid_payload(e)),
                    }
                }

                // elfiee://{project}/editors — project-level, no block CBAC needed
                "editors" => {
                    let editors = services::editor::list_editors(&handle).await;
                    let result: Vec<serde_json::Value> = editors
                        .iter()
                        .map(|editor| {
                            json!({
                                "editor_id": editor.editor_id,
                                "name": editor.name,
                                "editor_type": format!("{:?}", editor.editor_type),
                            })
                        })
                        .collect();

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("application/json".to_string()),
                            text: serde_json::to_string_pretty(&json!({
                                "project": project,
                                "editors": result,
                                "count": result.len(),
                            }))
                            .unwrap(),
                        }],
                    })
                }

                // elfiee://{project}/my-tasks — CBAC filtered via list_blocks
                "my-tasks" => {
                    let blocks = if let Some(ref eid) = editor_id {
                        services::block::list_blocks(&handle, eid).await
                    } else {
                        Vec::new()
                    };
                    let my_tasks: Vec<serde_json::Value> = blocks
                        .iter()
                        .filter(|b| b.block_type == "task")
                        .filter(|b| {
                            if let Some(ref eid) = editor_id {
                                b.contents
                                    .get("assigned_to")
                                    .and_then(|v| v.as_str())
                                    .is_some_and(|a| a == eid)
                                    || &b.owner == eid
                            } else {
                                false
                            }
                        })
                        .map(Self::format_block_summary)
                        .collect();

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("application/json".to_string()),
                            text: serde_json::to_string_pretty(&json!({
                                "project": project,
                                "editor_id": editor_id,
                                "tasks": my_tasks,
                                "count": my_tasks.len(),
                            }))
                            .unwrap(),
                        }],
                    })
                }

                // elfiee://{project}/my-grants
                "my-grants" => {
                    let my_grants = if let Some(ref eid) = editor_id {
                        services::grant::get_editor_grants(&handle, eid)
                            .await
                            .iter()
                            .map(|(cap_id, block_id)| json!({
                                "editor_id": eid,
                                "cap_id": cap_id,
                                "block_id": block_id,
                            }))
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    };

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("application/json".to_string()),
                            text: serde_json::to_string_pretty(&json!({
                                "project": project,
                                "editor_id": editor_id,
                                "grants": my_grants,
                                "count": my_grants.len(),
                            }))
                            .unwrap(),
                        }],
                    })
                }

                _ => Err(mcp::invalid_payload(format!(
                    "Unknown resource path: '{}'. Valid paths: blocks, block/{{id}}, grants, events, editors, my-tasks, my-grants",
                    remainder
                ))),
            }
    }
}

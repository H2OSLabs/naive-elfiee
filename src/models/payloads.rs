use serde::{Deserialize, Serialize};

// NOTE: Extension-specific payloads should be defined in their respective extension modules.
// This file contains only CORE capability payloads that are part of the base system.

/// Payload for core.create capability
///
/// This payload is used to create a new block with a name and type.
/// block_type 由 Extension 注册决定，核心类型包括 document, task, session。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlockPayload {
    /// The display name for the new block
    pub name: String,
    /// The block type (registered via Extension, e.g. "document", "task", "session")
    pub block_type: String,
    /// The source category of the block ("outline" or "linked")
    #[serde(default = "default_source")]
    pub source: String,
    /// Optional initial contents for the block (type-specific JSON).
    ///
    /// Document: { "format": "rs", "content": "fn main() {}" }
    /// Task: { "description": "实现登录", "status": "pending" }
    /// Session: { "entries": [] }
    #[serde(default)]
    pub contents: Option<serde_json::Value>,
    /// Document 类型的文件格式标识（创建 document block 时必填）。
    /// 例如: "md", "rs", "py", "toml", "png", "pdf"
    #[serde(default)]
    pub format: Option<String>,
    /// Optional block description
    #[serde(default)]
    pub description: Option<String>,
}

fn default_source() -> String {
    "outline".to_string()
}

/// Payload for core.link capability
///
/// This payload is used to create a link (relation) from one block to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkBlockPayload {
    /// The relation type (must be "implement")
    pub relation: String,
    /// The target block ID to link to
    pub target_id: String,
}

/// Payload for core.unlink capability
///
/// This payload is used to remove a link (relation) from one block to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlinkBlockPayload {
    /// The relation type (must be "implement")
    pub relation: String,
    /// The target block ID to unlink
    pub target_id: String,
}

/// Payload for core.grant capability
///
/// This payload is used to grant a capability to an editor for a specific block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantPayload {
    /// The editor ID to grant the capability to
    pub target_editor: String,
    /// The capability ID to grant (e.g., "document.write", "core.delete")
    pub capability: String,
    /// The block ID to grant access to, or "*" for all blocks (wildcard)
    #[serde(default = "default_wildcard")]
    pub target_block: String,
}

/// Payload for core.revoke capability
///
/// This payload is used to revoke a capability from an editor for a specific block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokePayload {
    /// The editor ID to revoke the capability from
    pub target_editor: String,
    /// The capability ID to revoke
    pub capability: String,
    /// The block ID to revoke access from, or "*" for all blocks (wildcard)
    #[serde(default = "default_wildcard")]
    pub target_block: String,
}

/// Payload for core.write capability
///
/// Updates structural fields of a block (name, description).
/// block_type is NOT modifiable — it is determined at init/scan time.
/// At least one field must be provided.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteBlockPayload {
    /// New name for the block (optional)
    #[serde(default)]
    pub name: Option<String>,
    /// New description for the block (optional)
    #[serde(default)]
    pub description: Option<String>,
}

/// Payload for editor.create capability
///
/// This payload is used to create a new editor identity in the file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorCreatePayload {
    /// The display name for the new editor
    pub name: String,
    /// The type of editor (Human or Bot), defaults to Human if not specified
    #[serde(default)]
    pub editor_type: Option<String>,
    /// Optional explicitly provided editor ID (e.g. system editor ID)
    #[serde(default)]
    pub editor_id: Option<String>,
}

/// Payload for editor.delete capability
///
/// This payload is used to delete an editor identity from the file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorDeletePayload {
    /// The editor ID to delete
    pub editor_id: String,
}

/// Default value for target_block field (wildcard)
fn default_wildcard() -> String {
    "*".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_block_payload() {
        let json = serde_json::json!({
            "name": "My Block",
            "block_type": "document",
            "source": "linked",
            "format": "md"
        });
        let payload: CreateBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.name, "My Block");
        assert_eq!(payload.block_type, "document");
        assert_eq!(payload.source, "linked");
        assert_eq!(payload.format, Some("md".to_string()));
        assert!(payload.description.is_none());
        assert!(payload.contents.is_none());
    }

    #[test]
    fn test_create_block_payload_default_source() {
        let json = serde_json::json!({
            "name": "My Block",
            "block_type": "document",
            "format": "md"
        });
        let payload: CreateBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.source, "outline");
    }

    #[test]
    fn test_create_block_payload_with_description() {
        let json = serde_json::json!({
            "name": "My Block",
            "block_type": "document",
            "format": "rs",
            "description": "测试描述"
        });
        let payload: CreateBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.name, "My Block");
        assert_eq!(payload.description, Some("测试描述".to_string()));
    }

    #[test]
    fn test_write_block_payload() {
        let json = serde_json::json!({
            "name": "New Name",
            "description": "New Description"
        });
        let payload: WriteBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.name, Some("New Name".to_string()));
        assert_eq!(payload.description, Some("New Description".to_string()));
    }

    #[test]
    fn test_write_block_payload_partial() {
        let json = serde_json::json!({
            "name": "Only Name"
        });
        let payload: WriteBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.name, Some("Only Name".to_string()));
        assert!(payload.description.is_none());
    }

    #[test]
    fn test_create_block_payload_with_contents() {
        let json = serde_json::json!({
            "name": "auth.rs",
            "block_type": "document",
            "format": "rs",
            "contents": {
                "format": "rs",
                "content": "fn main() {}"
            }
        });
        let payload: CreateBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.format, Some("rs".to_string()));
        assert!(payload.contents.is_some());
        let contents = payload.contents.unwrap();
        assert_eq!(contents["content"], "fn main() {}");
    }

    #[test]
    fn test_create_block_payload_task_type() {
        let json = serde_json::json!({
            "name": "实现登录",
            "block_type": "task",
            "contents": {
                "description": "为项目添加 OAuth2 登录",
                "status": "pending"
            }
        });
        let payload: CreateBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.block_type, "task");
        assert!(payload.format.is_none());
        assert!(payload.contents.is_some());
    }

    #[test]
    fn test_create_block_payload_session_type() {
        let json = serde_json::json!({
            "name": "执行记录",
            "block_type": "session",
            "contents": {
                "entries": []
            }
        });
        let payload: CreateBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.block_type, "session");
        assert!(payload.contents.is_some());
    }

    #[test]
    fn test_link_block_payload() {
        use crate::models::RELATION_IMPLEMENT;

        let json = serde_json::json!({
            "relation": RELATION_IMPLEMENT,
            "target_id": "block-456"
        });
        let payload: LinkBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.relation, RELATION_IMPLEMENT);
        assert_eq!(payload.target_id, "block-456");
    }

    #[test]
    fn test_unlink_block_payload() {
        use crate::models::RELATION_IMPLEMENT;

        let json = serde_json::json!({
            "relation": RELATION_IMPLEMENT,
            "target_id": "block-789"
        });
        let payload: UnlinkBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.relation, RELATION_IMPLEMENT);
        assert_eq!(payload.target_id, "block-789");
    }

    #[test]
    fn test_grant_payload_with_wildcard_default() {
        let json = serde_json::json!({
            "target_editor": "alice",
            "capability": "document.write"
        });
        let payload: GrantPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.target_editor, "alice");
        assert_eq!(payload.capability, "document.write");
        assert_eq!(payload.target_block, "*");
    }

    #[test]
    fn test_grant_payload_with_specific_block() {
        let json = serde_json::json!({
            "target_editor": "bob",
            "capability": "core.delete",
            "target_block": "block-123"
        });
        let payload: GrantPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.target_block, "block-123");
    }

    #[test]
    fn test_revoke_payload() {
        let json = serde_json::json!({
            "target_editor": "charlie",
            "capability": "document.write",
            "target_block": "block-999"
        });
        let payload: RevokePayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.target_editor, "charlie");
        assert_eq!(payload.capability, "document.write");
        assert_eq!(payload.target_block, "block-999");
    }

    #[test]
    fn test_editor_create_payload() {
        let json = serde_json::json!({
            "name": "Alice"
        });
        let payload: EditorCreatePayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.name, "Alice");
        assert!(payload.editor_id.is_none());
    }

    #[test]
    fn test_editor_create_payload_with_id() {
        let json = serde_json::json!({
            "name": "System",
            "editor_id": "sys-123"
        });
        let payload: EditorCreatePayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.name, "System");
        assert_eq!(payload.editor_id, Some("sys-123".to_string()));
    }
}

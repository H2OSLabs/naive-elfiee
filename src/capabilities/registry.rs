use super::core::CapabilityHandler;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing capability handlers.
///
/// Capabilities are registered at initialization and can be looked up by ID.
pub struct CapabilityRegistry {
    handlers: HashMap<String, Arc<dyn CapabilityHandler>>,
}

impl CapabilityRegistry {
    /// Create a new registry with all built-in capabilities registered.
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
        };

        // Register built-in capabilities
        registry.register_builtins();

        // Register extension capabilities
        registry.register_extensions();

        registry
    }

    /// Register a capability handler.
    pub fn register(&mut self, handler: Arc<dyn CapabilityHandler>) {
        self.handlers.insert(handler.cap_id().to_string(), handler);
    }

    /// Get a capability handler by ID.
    pub fn get(&self, cap_id: &str) -> Option<Arc<dyn CapabilityHandler>> {
        self.handlers.get(cap_id).cloned()
    }

    /// Get all registered capability IDs suitable for agent auto-grants.
    ///
    /// Returns all capability IDs except those in `exclude`. Typically used to
    /// exclude owner-only capabilities like `core.grant` and `core.revoke`.
    pub fn get_grantable_cap_ids(&self, exclude: &[&str]) -> Vec<String> {
        self.handlers
            .keys()
            .filter(|id| !exclude.contains(&id.as_str()))
            .cloned()
            .collect()
    }

    /// Register all built-in capabilities (9: 7 core + 2 editor).
    fn register_builtins(&mut self) {
        use super::builtins::*;

        self.register(Arc::new(CoreCreateCapability));
        self.register(Arc::new(CoreWriteCapability));
        self.register(Arc::new(CoreLinkCapability));
        self.register(Arc::new(CoreUnlinkCapability));
        self.register(Arc::new(CoreDeleteCapability));
        self.register(Arc::new(CoreGrantCapability));
        self.register(Arc::new(CoreRevokeCapability));
        self.register(Arc::new(EditorCreateCapability));
        self.register(Arc::new(EditorDeleteCapability));
    }

    /// Register all extension capabilities.
    fn register_extensions(&mut self) {
        use crate::extensions::document::*;
        use crate::extensions::session::*;
        use crate::extensions::task::*;

        // Document extension
        self.register(Arc::new(DocumentWriteCapability));
        self.register(Arc::new(DocumentReadCapability));

        // Task extension
        self.register(Arc::new(TaskWriteCapability));
        self.register(Arc::new(TaskReadCapability));
        self.register(Arc::new(TaskCommitCapability));

        // Session extension
        self.register(Arc::new(SessionAppendCapability));
        self.register(Arc::new(SessionReadCapability));
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let registry = CapabilityRegistry::new();

        // Verify core capabilities are registered (9 builtins)
        assert!(registry.get("core.create").is_some());
        assert!(registry.get("core.write").is_some());
        assert!(registry.get("core.link").is_some());
        assert!(registry.get("core.unlink").is_some());
        assert!(registry.get("core.delete").is_some());
        assert!(registry.get("core.grant").is_some());
        assert!(registry.get("core.revoke").is_some());
        assert!(registry.get("editor.create").is_some());
        assert!(registry.get("editor.delete").is_some());

        // Verify removed capabilities are NOT registered
        assert!(registry.get("core.read").is_none());
        assert!(registry.get("core.rename").is_none());
        assert!(registry.get("core.change_type").is_none());
        assert!(registry.get("core.update_metadata").is_none());

        // Verify extension capabilities are registered
        assert!(registry.get("document.write").is_some());
        assert!(registry.get("document.read").is_some());
        assert!(registry.get("task.write").is_some());
        assert!(registry.get("task.read").is_some());
        assert!(registry.get("task.commit").is_some());
        assert!(registry.get("session.append").is_some());
        assert!(registry.get("session.read").is_some());

        // Verify deleted capabilities are NOT registered
        assert!(registry.get("markdown.write").is_none());
        assert!(registry.get("markdown.read").is_none());
        assert!(registry.get("code.write").is_none());
        assert!(registry.get("code.read").is_none());
        assert!(registry.get("terminal.save").is_none());
        assert!(registry.get("directory.write").is_none());
        assert!(registry.get("agent.create").is_none());
    }

    #[test]
    fn test_capability_lookup() {
        let registry = CapabilityRegistry::new();

        let cap = registry.get("core.link").unwrap();
        assert_eq!(cap.cap_id(), "core.link");
        assert_eq!(cap.target(), "core/*");
    }

    #[test]
    fn test_get_grantable_cap_ids_excludes_specified() {
        let registry = CapabilityRegistry::new();
        let caps = registry.get_grantable_cap_ids(&["core.grant", "core.revoke"]);

        // Should not contain excluded capabilities
        assert!(!caps.contains(&"core.grant".to_string()));
        assert!(!caps.contains(&"core.revoke".to_string()));

        // Should contain other capabilities
        assert!(caps.contains(&"core.create".to_string()));
        assert!(caps.contains(&"core.write".to_string()));
        assert!(caps.contains(&"document.write".to_string()));
        assert!(caps.contains(&"core.delete".to_string()));

        // Total should be all registered minus 2 excluded
        let all_count = registry.get_grantable_cap_ids(&[]).len();
        assert_eq!(caps.len(), all_count - 2);
    }

    #[test]
    fn test_nonexistent_capability() {
        let registry = CapabilityRegistry::new();
        assert!(registry.get("nonexistent.capability").is_none());
    }

    #[test]
    fn test_link_capability_execution() {
        use crate::models::{Block, Command};

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.link").unwrap();

        let block = Block::new(
            "test".to_string(),
            "document".to_string(),
            "editor1".to_string(),
        );
        let cmd = Command::new(
            "editor1".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "block2"
            }),
        );

        let events = cap.handler(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, block.block_id);
        assert_eq!(events[0].attribute, "editor1/core.link");
    }

    #[test]
    fn test_create_capability_execution() {
        use crate::models::{Block, Command};

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.create").unwrap();

        let dummy_block = Block::new(
            "dummy".to_string(),
            "dummy".to_string(),
            "editor1".to_string(),
        );
        let cmd = Command::new(
            "editor1".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "New Block",
                "block_type": "document"
            }),
        );

        let events = cap.handler(&cmd, Some(&dummy_block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].attribute, "editor1/core.create");
        let value = &events[0].value;
        assert_eq!(
            value.get("name").and_then(|v| v.as_str()),
            Some("New Block")
        );
        assert_eq!(value.get("type").and_then(|v| v.as_str()), Some("document"));
        assert_eq!(value.get("owner").and_then(|v| v.as_str()), Some("editor1"));
    }

    #[test]
    fn test_write_capability_execution() {
        use crate::models::{Block, Command};

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.write").unwrap();

        let block = Block::new(
            "test".to_string(),
            "document".to_string(),
            "editor1".to_string(),
        );
        let cmd = Command::new(
            "editor1".to_string(),
            "core.write".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "name": "Updated Name",
                "description": "Updated Description"
            }),
        );

        let events = cap.handler(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].attribute, "editor1/core.write");
        assert_eq!(events[0].value["name"], "Updated Name");
        assert_eq!(events[0].value["description"], "Updated Description");
    }

    #[test]
    fn test_grant_capability_execution() {
        use crate::models::{Block, Command};

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.grant").unwrap();

        let block = Block::new(
            "test".to_string(),
            "document".to_string(),
            "editor1".to_string(),
        );
        let cmd = Command::new(
            "editor1".to_string(),
            "core.grant".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "target_editor": "editor2",
                "capability": "document.write",
                "target_block": block.block_id
            }),
        );

        let events = cap.handler(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].attribute, "editor1/core.grant");
    }

    #[test]
    fn test_unlink_capability_execution() {
        use crate::models::{Block, Command, RELATION_IMPLEMENT};
        use std::collections::HashMap;

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.unlink").unwrap();

        let mut block = Block::new(
            "test".to_string(),
            "document".to_string(),
            "editor1".to_string(),
        );
        let mut children = HashMap::new();
        children.insert(
            RELATION_IMPLEMENT.to_string(),
            vec!["block2".to_string(), "block3".to_string()],
        );
        block.children = children;

        let cmd = Command::new(
            "editor1".to_string(),
            "core.unlink".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "relation": RELATION_IMPLEMENT,
                "target_id": "block2"
            }),
        );

        let events = cap.handler(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, block.block_id);
        assert_eq!(events[0].attribute, "editor1/core.unlink");

        let value_obj = events[0].value.as_object().unwrap();
        let new_children: HashMap<String, Vec<String>> =
            serde_json::from_value(value_obj.get("children").unwrap().clone()).unwrap();
        assert_eq!(new_children.get(RELATION_IMPLEMENT).unwrap().len(), 1);
        assert_eq!(new_children.get(RELATION_IMPLEMENT).unwrap()[0], "block3");
    }

    #[test]
    fn test_revoke_capability_execution() {
        use crate::models::{Block, Command};

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.revoke").unwrap();

        let block = Block::new(
            "test".to_string(),
            "document".to_string(),
            "editor1".to_string(),
        );
        let cmd = Command::new(
            "editor1".to_string(),
            "core.revoke".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "target_editor": "editor2",
                "capability": "document.write",
                "target_block": block.block_id
            }),
        );

        let events = cap.handler(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].attribute, "editor1/core.revoke");
    }

    #[test]
    fn test_authorization_owner_always_authorized() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let grants_table = GrantsTable::new();
        let registry = CapabilityRegistry::new();

        let block = Block::new(
            "Test Block".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "alice".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "block2"
            }),
        );

        assert!(
            block.owner == cmd.editor_id
                || grants_table.has_grant(&cmd.editor_id, "core.link", &block.block_id),
            "Owner should always be authorized"
        );

        let cap = registry.get("core.link").unwrap();
        let result = cap.handler(&cmd, Some(&block));
        assert!(result.is_ok());
    }

    #[test]
    fn test_authorization_non_owner_without_grant_rejected() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let grants_table = GrantsTable::new();

        let block = Block::new(
            "Test Block".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "block2"
            }),
        );

        let is_authorized = block.owner == cmd.editor_id
            || grants_table.has_grant(&cmd.editor_id, "core.link", &block.block_id);

        assert!(!is_authorized);
    }

    #[test]
    fn test_authorization_non_owner_with_specific_grant_authorized() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let mut grants_table = GrantsTable::new();
        let registry = CapabilityRegistry::new();

        let block = Block::new(
            "Test Block".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        grants_table.add_grant(
            "bob".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
        );

        let cmd = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "block2"
            }),
        );

        let is_authorized = block.owner == cmd.editor_id
            || grants_table.has_grant(&cmd.editor_id, "core.link", &block.block_id);
        assert!(is_authorized);

        let cap = registry.get("core.link").unwrap();
        let result = cap.handler(&cmd, Some(&block));
        assert!(result.is_ok());
    }

    #[test]
    fn test_authorization_wildcard_grant_works_for_any_block() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let mut grants_table = GrantsTable::new();
        let registry = CapabilityRegistry::new();

        grants_table.add_grant("bob".to_string(), "core.link".to_string(), "*".to_string());

        let block1 = Block::new(
            "Block 1".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        let block2 = Block::new(
            "Block 2".to_string(),
            "document".to_string(),
            "charlie".to_string(),
        );

        let cmd1 = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block1.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "other_block"
            }),
        );

        let cmd2 = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block2.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "other_block"
            }),
        );

        assert!(grants_table.has_grant(&cmd1.editor_id, "core.link", &block1.block_id));
        assert!(grants_table.has_grant(&cmd2.editor_id, "core.link", &block2.block_id));

        let cap = registry.get("core.link").unwrap();
        assert!(cap.handler(&cmd1, Some(&block1)).is_ok());
        assert!(cap.handler(&cmd2, Some(&block2)).is_ok());
    }

    #[test]
    fn test_authorization_different_capability_not_authorized() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let mut grants_table = GrantsTable::new();

        let block = Block::new(
            "Test Block".to_string(),
            "document".to_string(),
            "alice".to_string(),
        );

        grants_table.add_grant(
            "bob".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
        );

        let cmd = Command::new(
            "bob".to_string(),
            "core.delete".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        let is_authorized = block.owner == cmd.editor_id
            || grants_table.has_grant(&cmd.editor_id, "core.delete", &block.block_id);

        assert!(!is_authorized);
    }
}

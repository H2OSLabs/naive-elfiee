use crate::models::Event;
use std::collections::HashMap;

/// Grants table for Capability-Based Access Control (CBAC).
///
/// Tracks which editors have been granted which capabilities for which blocks.
/// This table is projected from grant/revoke events in the EventStore.
#[derive(Debug, Clone)]
pub struct GrantsTable {
    /// Map: editor_id -> Vec<(cap_id, block_id)>
    grants: HashMap<String, Vec<(String, String)>>,
}

impl GrantsTable {
    /// Create an empty grants table.
    pub fn new() -> Self {
        Self {
            grants: HashMap::new(),
        }
    }

    /// Process a single grant/revoke event and update the table.
    ///
    /// This is the sole entry point for grant/revoke event processing.
    /// Called by StateProjector::apply_event() for `core.grant` and `core.revoke` events.
    pub fn process_event(&mut self, event: &Event) {
        if event.attribute.ends_with("/core.grant") {
            if let Some(obj) = event.value.as_object() {
                let editor = obj.get("editor").and_then(|v| v.as_str()).unwrap_or("");
                let capability = obj.get("capability").and_then(|v| v.as_str()).unwrap_or("");
                let block = obj.get("block").and_then(|v| v.as_str()).unwrap_or("*");

                if !editor.is_empty() && !capability.is_empty() {
                    self.add_grant(
                        editor.to_string(),
                        capability.to_string(),
                        block.to_string(),
                    );
                }
            }
        } else if event.attribute.ends_with("/core.revoke") {
            if let Some(obj) = event.value.as_object() {
                let editor = obj.get("editor").and_then(|v| v.as_str()).unwrap_or("");
                let capability = obj.get("capability").and_then(|v| v.as_str()).unwrap_or("");
                let block = obj.get("block").and_then(|v| v.as_str()).unwrap_or("*");

                if !editor.is_empty() && !capability.is_empty() {
                    self.remove_grant(editor, capability, block);
                }
            }
        }
    }

    /// Add a grant to the table.
    pub fn add_grant(&mut self, editor_id: String, cap_id: String, block_id: String) {
        let entry = self.grants.entry(editor_id).or_default();

        // Avoid duplicates
        let grant_pair = (cap_id, block_id);
        if !entry.contains(&grant_pair) {
            entry.push(grant_pair);
        }
    }

    /// Remove a grant from the table.
    pub fn remove_grant(&mut self, editor_id: &str, cap_id: &str, block_id: &str) {
        if let Some(editor_grants) = self.grants.get_mut(editor_id) {
            editor_grants.retain(|(cap, blk)| !(cap == cap_id && blk == block_id));

            // Clean up empty entries
            if editor_grants.is_empty() {
                self.grants.remove(editor_id);
            }
        }
    }

    /// Remove all grants for a specific editor.
    ///
    /// This is used when an editor is deleted from the system.
    pub fn remove_all_grants_for_editor(&mut self, editor_id: &str) {
        self.grants.remove(editor_id);
    }

    /// Get all grants for a specific editor.
    ///
    /// Returns a reference to the Vec of (cap_id, block_id) tuples, or None if no grants exist.
    pub fn get_grants(&self, editor_id: &str) -> Option<&Vec<(String, String)>> {
        self.grants.get(editor_id)
    }

    /// Iterate over all grants as (editor_id, cap_id, block_id) tuples.
    pub fn iter_all(&self) -> impl Iterator<Item = (&str, &str, &str)> {
        self.grants.iter().flat_map(|(editor_id, pairs)| {
            pairs.iter().map(move |(cap_id, block_id)| {
                (editor_id.as_str(), cap_id.as_str(), block_id.as_str())
            })
        })
    }

    /// Check if an editor has a specific grant.
    ///
    /// Matching order:
    /// 1. Exact match: editor_id has (cap_id, block_id) or (cap_id, "*")
    /// 2. Wildcard editor: "*" has (cap_id, block_id) or (cap_id, "*")
    ///
    /// The wildcard editor_id "*" means "all editors have this grant".
    pub fn has_grant(&self, editor_id: &str, cap_id: &str, block_id: &str) -> bool {
        let check = |grants: &Vec<(String, String)>| {
            grants
                .iter()
                .any(|(cap, blk)| cap == cap_id && (blk == block_id || blk == "*"))
        };

        // 1. Exact match on editor_id
        if let Some(editor_grants) = self.grants.get(editor_id) {
            if check(editor_grants) {
                return true;
            }
        }

        // 2. Wildcard editor_id "*" (grant to all editors)
        if editor_id != "*" {
            if let Some(wildcard_grants) = self.grants.get("*") {
                if check(wildcard_grants) {
                    return true;
                }
            }
        }

        false
    }
}

impl Default for GrantsTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as StdHashMap;

    #[test]
    fn test_add_grant() {
        let mut table = GrantsTable::new();

        table.add_grant(
            "alice".to_string(),
            "document.write".to_string(),
            "block1".to_string(),
        );

        let grants = table.get_grants("alice").unwrap();
        assert_eq!(grants.len(), 1);
        assert_eq!(
            grants[0],
            ("document.write".to_string(), "block1".to_string())
        );
    }

    #[test]
    fn test_add_duplicate_grant() {
        let mut table = GrantsTable::new();

        table.add_grant(
            "alice".to_string(),
            "document.write".to_string(),
            "block1".to_string(),
        );
        table.add_grant(
            "alice".to_string(),
            "document.write".to_string(),
            "block1".to_string(),
        );

        let grants = table.get_grants("alice").unwrap();
        assert_eq!(grants.len(), 1, "Duplicate grants should not be added");
    }

    #[test]
    fn test_remove_grant() {
        let mut table = GrantsTable::new();

        table.add_grant(
            "alice".to_string(),
            "document.write".to_string(),
            "block1".to_string(),
        );
        table.add_grant(
            "alice".to_string(),
            "core.link".to_string(),
            "block2".to_string(),
        );

        table.remove_grant("alice", "document.write", "block1");

        let grants = table.get_grants("alice").unwrap();
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0], ("core.link".to_string(), "block2".to_string()));
    }

    #[test]
    fn test_remove_all_grants_for_editor() {
        let mut table = GrantsTable::new();

        table.add_grant(
            "alice".to_string(),
            "document.write".to_string(),
            "block1".to_string(),
        );
        table.add_grant(
            "alice".to_string(),
            "core.link".to_string(),
            "block2".to_string(),
        );

        table.remove_all_grants_for_editor("alice");

        assert!(table.get_grants("alice").is_none());
        assert!(!table.has_grant("alice", "document.write", "block1"));
    }

    #[test]
    fn test_has_grant_exact_match() {
        let mut table = GrantsTable::new();
        table.add_grant(
            "alice".to_string(),
            "document.write".to_string(),
            "block1".to_string(),
        );

        assert!(table.has_grant("alice", "document.write", "block1"));
        assert!(!table.has_grant("alice", "document.write", "block2"));
        assert!(!table.has_grant("alice", "core.link", "block1"));
    }

    #[test]
    fn test_has_grant_wildcard() {
        let mut table = GrantsTable::new();
        table.add_grant(
            "alice".to_string(),
            "document.write".to_string(),
            "*".to_string(),
        );

        assert!(table.has_grant("alice", "document.write", "block1"));
        assert!(table.has_grant("alice", "document.write", "block2"));
        assert!(table.has_grant("alice", "document.write", "any_block"));
    }

    #[test]
    fn test_process_event_grant_and_revoke() {
        let mut ts1 = StdHashMap::new();
        ts1.insert("alice".to_string(), 1);

        let grant_event = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "document.write",
                "block": "block1"
            }),
            ts1.clone(),
        );

        let mut ts2 = StdHashMap::new();
        ts2.insert("alice".to_string(), 2);

        let revoke_event = Event::new(
            "alice".to_string(),
            "alice/core.revoke".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "document.write",
                "block": "block1"
            }),
            ts2,
        );

        // Test: grant via process_event
        let mut table = GrantsTable::new();
        table.process_event(&grant_event);
        assert!(table.has_grant("bob", "document.write", "block1"));

        // Test: revoke via process_event
        table.process_event(&revoke_event);
        assert!(!table.has_grant("bob", "document.write", "block1"));
    }

    #[test]
    fn test_wildcard_editor_grant() {
        let mut table = GrantsTable::new();
        // Grant directory.write to ALL editors ("*") for a specific block
        table.add_grant(
            "*".to_string(),
            "task.write".to_string(),
            "elf-block".to_string(),
        );

        // Any editor should match
        assert!(table.has_grant("alice", "task.write", "elf-block"));
        assert!(table.has_grant("bob", "task.write", "elf-block"));
        assert!(table.has_grant("system", "task.write", "elf-block"));

        // Wrong capability or block should not match
        assert!(!table.has_grant("alice", "document.write", "elf-block"));
        assert!(!table.has_grant("alice", "task.write", "other-block"));
    }

    #[test]
    fn test_wildcard_editor_revoke() {
        let mut table = GrantsTable::new();
        table.add_grant(
            "*".to_string(),
            "task.write".to_string(),
            "elf-block".to_string(),
        );

        assert!(table.has_grant("alice", "task.write", "elf-block"));

        // Revoke the wildcard grant
        table.remove_grant("*", "task.write", "elf-block");

        assert!(!table.has_grant("alice", "task.write", "elf-block"));
        assert!(!table.has_grant("bob", "task.write", "elf-block"));
    }

    #[test]
    fn test_wildcard_editor_with_exact_grant() {
        let mut table = GrantsTable::new();

        // Exact grant for alice
        table.add_grant(
            "alice".to_string(),
            "document.write".to_string(),
            "block1".to_string(),
        );
        // Wildcard grant for all editors
        table.add_grant(
            "*".to_string(),
            "task.write".to_string(),
            "elf-block".to_string(),
        );

        // alice: exact grant works
        assert!(table.has_grant("alice", "document.write", "block1"));
        // alice: wildcard grant also works
        assert!(table.has_grant("alice", "task.write", "elf-block"));
        // bob: only wildcard grant works
        assert!(table.has_grant("bob", "task.write", "elf-block"));
        assert!(!table.has_grant("bob", "document.write", "block1"));
    }

    #[test]
    fn test_wildcard_editor_combined_with_wildcard_block() {
        let mut table = GrantsTable::new();
        // Grant to all editors on all blocks
        table.add_grant("*".to_string(), "task.write".to_string(), "*".to_string());

        assert!(table.has_grant("alice", "task.write", "any-block"));
        assert!(table.has_grant("bob", "task.write", "other-block"));
        // Wrong capability still doesn't match
        assert!(!table.has_grant("alice", "document.write", "any-block"));
    }
}

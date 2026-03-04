use crate::capabilities::grants::GrantsTable;
use crate::models::{Block, Editor, EditorType, Event, EventMode, RELATION_IMPLEMENT};
use log;
use std::collections::HashMap;

/// Remove `parent_id` from the parent list of each target block.
/// Cleans up empty parent entries from the reverse index.
fn remove_parent_entries(
    parents: &mut HashMap<String, Vec<String>>,
    parent_id: &str,
    targets: &[String],
) {
    for target in targets {
        if let Some(parent_list) = parents.get_mut(target) {
            parent_list.retain(|id| id != parent_id);
            if parent_list.is_empty() {
                parents.remove(target);
            }
        }
    }
}

/// In-memory state projection from events.
///
/// Replays all events to build the current state of blocks, editors, and grants.
/// This is the authoritative source of truth for the engine's current state.
pub struct StateProjector {
    /// All blocks indexed by block_id
    pub blocks: HashMap<String, Block>,

    /// All editors indexed by editor_id
    pub editors: HashMap<String, Editor>,

    /// Grants table for authorization (reuses existing implementation)
    pub grants: GrantsTable,

    /// Vector clock counts for each editor (for conflict detection)
    pub editor_counts: HashMap<String, i64>,

    /// Reverse index: child_block_id → list of parent_block_ids
    ///
    /// Maintained for `implement` relations only (the sole relation type).
    /// Updated on core.link, core.unlink, and core.delete events.
    pub parents: HashMap<String, Vec<String>>,
}

impl StateProjector {
    /// Create a new empty state projector.
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            editors: HashMap::new(),
            grants: GrantsTable::new(),
            editor_counts: HashMap::new(),
            parents: HashMap::new(),
        }
    }

    /// Replay all events to build current state.
    ///
    /// This is called once during engine initialization to rebuild
    /// state from the event store.
    pub fn replay(&mut self, events: Vec<Event>) {
        for event in events {
            self.apply_event(&event);
        }
    }

    /// Apply a single event to state.
    ///
    /// This method updates the in-memory state based on the event type.
    pub fn apply_event(&mut self, event: &Event) {
        // Update editor transaction counts from vector clock
        for (editor_id, count) in &event.timestamp {
            let current = self.editor_counts.entry(editor_id.clone()).or_insert(0);
            *current = (*current).max(*count);
        }

        // Parse attribute format: "{editor_id}/{cap_id}"
        let parts: Vec<&str> = event.attribute.split('/').collect();
        if parts.len() != 2 {
            return; // Invalid attribute format, skip
        }
        let cap_id = parts[1];

        // Handle different event types based on capability
        match cap_id {
            // Block creation
            "core.create" => {
                // Create event should contain full block state
                if let Some(obj) = event.value.as_object() {
                    let block = Block {
                        block_id: event.entity.clone(),
                        name: obj
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        block_type: obj
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        owner: obj
                            .get("owner")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        contents: obj
                            .get("contents")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!({})),
                        children: obj
                            .get("children")
                            .and_then(|v| serde_json::from_value(v.clone()).ok())
                            .unwrap_or_default(),
                        description: obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    };
                    // Build reverse index for initial children
                    if let Some(targets) = block.children.get(RELATION_IMPLEMENT) {
                        for target in targets {
                            self.parents
                                .entry(target.clone())
                                .or_default()
                                .push(block.block_id.clone());
                        }
                    }
                    self.blocks.insert(block.block_id.clone(), block);
                }
            }

            // Block structural write (name, description)
            // block_type is NOT modifiable — determined at init/scan time
            // Must be matched BEFORE the generic .write wildcard below
            "core.write" => {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    if let Some(name) = event.value.get("name").and_then(|v| v.as_str()) {
                        block.name = name.to_string();
                    }
                    if let Some(desc) = event.value.get("description").and_then(|v| v.as_str()) {
                        block.description = Some(desc.to_string());
                    }
                }
            }

            // Block content updates (write, link, save)
            _ if cap_id.ends_with(".write")
                || cap_id.ends_with(".link")
                || cap_id.ends_with(".save") =>
            {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    // 根据 event.mode 决定内容更新策略
                    match event.mode {
                        EventMode::Full => {
                            // Full 模式：合并 contents 对象（当前行为）
                            if let Some(contents) = event.value.get("contents") {
                                if let Some(obj) = block.contents.as_object_mut() {
                                    if let Some(new_contents) = contents.as_object() {
                                        for (k, v) in new_contents {
                                            obj.insert(k.clone(), v.clone());
                                        }
                                    }
                                }
                            }
                        }
                        EventMode::Delta => {
                            // Delta 模式：存储 diff 内容
                            // 当前为 placeholder——实际 diff apply 逻辑在 document extension（Step 5）实现
                            // 此处将 delta 内容存入 contents，供后续处理
                            if let Some(contents) = event.value.get("contents") {
                                if let Some(obj) = block.contents.as_object_mut() {
                                    if let Some(new_contents) = contents.as_object() {
                                        for (k, v) in new_contents {
                                            obj.insert(k.clone(), v.clone());
                                        }
                                    }
                                }
                            }
                            log::debug!(
                                "Delta mode event for block {} — full diff apply deferred to Step 5",
                                event.entity
                            );
                        }
                        EventMode::Ref => {
                            // Ref 模式：存储引用元数据（hash + path + size）
                            // 不含实际二进制内容，Agent 通过 AgentContext 按 hash 获取
                            if let Some(contents) = event.value.get("contents") {
                                block.contents = contents.clone();
                            }
                        }
                        EventMode::Append => {
                            // Append 模式：追加 entry 到 entries 数组
                            // Session Block 使用此模式，每个 event 的 value 是一条 entry
                            if let Some(entry) = event.value.get("entry") {
                                let entries = block.contents.as_object_mut().and_then(|obj| {
                                    obj.entry("entries")
                                        .or_insert_with(|| serde_json::json!([]))
                                        .as_array_mut()
                                });
                                if let Some(arr) = entries {
                                    arr.push(entry.clone());
                                }
                            }
                        }
                    }

                    // Update children if present, maintaining reverse index
                    if let Some(children) = event.value.get("children") {
                        if let Ok(new_children) =
                            serde_json::from_value::<HashMap<String, Vec<String>>>(children.clone())
                        {
                            let old_targets: Vec<String> = block
                                .children
                                .get(RELATION_IMPLEMENT)
                                .cloned()
                                .unwrap_or_default();
                            let new_targets: Vec<String> = new_children
                                .get(RELATION_IMPLEMENT)
                                .cloned()
                                .unwrap_or_default();

                            // Add new parent entries
                            for target in &new_targets {
                                if !old_targets.contains(target) {
                                    self.parents
                                        .entry(target.clone())
                                        .or_default()
                                        .push(event.entity.clone());
                                }
                            }
                            // Remove parent entries for targets no longer linked
                            let removed: Vec<String> = old_targets
                                .iter()
                                .filter(|t| !new_targets.contains(t))
                                .cloned()
                                .collect();
                            remove_parent_entries(&mut self.parents, &event.entity, &removed);

                            block.children = new_children;
                        }
                    }
                }
            }

            "core.unlink" => {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    // Update children, maintaining reverse index
                    if let Some(children) = event.value.get("children") {
                        if let Ok(new_children) =
                            serde_json::from_value::<HashMap<String, Vec<String>>>(children.clone())
                        {
                            let old_targets: Vec<String> = block
                                .children
                                .get(RELATION_IMPLEMENT)
                                .cloned()
                                .unwrap_or_default();
                            let new_targets: Vec<String> = new_children
                                .get(RELATION_IMPLEMENT)
                                .cloned()
                                .unwrap_or_default();

                            // Remove parent entries for targets that were unlinked
                            let removed: Vec<String> = old_targets
                                .iter()
                                .filter(|t| !new_targets.contains(t))
                                .cloned()
                                .collect();
                            remove_parent_entries(&mut self.parents, &event.entity, &removed);

                            block.children = new_children;
                        }
                    }
                }
            }

            // Block deletion
            "core.delete" => {
                // 1. Clean up forward direction: remove this block as a parent of its children
                if let Some(block) = self.blocks.get(&event.entity) {
                    if let Some(targets) = block.children.get(RELATION_IMPLEMENT) {
                        remove_parent_entries(&mut self.parents, &event.entity, targets);
                    }
                }

                // 2. Clean up reverse direction: remove this block from parent blocks' children
                if let Some(parent_ids) = self.parents.remove(&event.entity) {
                    for parent_id in &parent_ids {
                        if let Some(parent_block) = self.blocks.get_mut(parent_id) {
                            if let Some(targets) = parent_block.children.get_mut(RELATION_IMPLEMENT)
                            {
                                targets.retain(|id| id != &event.entity);
                            }
                        }
                    }
                }

                self.blocks.remove(&event.entity);
            }

            // Grant/Revoke — delegate to GrantsTable (single source of event parsing)
            "core.grant" | "core.revoke" => {
                self.grants.process_event(event);
            }

            // Editor creation
            "editor.create" => {
                if let Some(editor_obj) = event.value.as_object() {
                    let editor_id = editor_obj
                        .get("editor_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let name = editor_obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let editor_type_str = editor_obj.get("editor_type").and_then(|v| v.as_str());

                    // editor_id, name, editor_type 均为必填字段
                    if let (false, false, Some(type_str)) =
                        (editor_id.is_empty(), name.is_empty(), editor_type_str)
                    {
                        let editor_type = match type_str {
                            "Bot" => EditorType::Bot,
                            "Human" => EditorType::Human,
                            _ => {
                                log::warn!(
                                    "Unknown editor_type '{}' in editor.create for {}, skipping",
                                    type_str,
                                    editor_id
                                );
                                return;
                            }
                        };

                        self.editors.insert(
                            editor_id.to_string(),
                            Editor {
                                editor_id: editor_id.to_string(),
                                name: name.to_string(),
                                editor_type,
                            },
                        );
                    }
                }
            }

            // Editor deletion
            "editor.delete" => {
                self.editors.remove(&event.entity);
                // Remove all grants for this editor to prevent leaks in GrantsTable
                self.grants.remove_all_grants_for_editor(&event.entity);
            }

            _ => {
                // Unknown capability - ignore for now
            }
        }
    }

    /// Get a block by ID.
    pub fn get_block(&self, block_id: &str) -> Option<&Block> {
        self.blocks.get(block_id)
    }

    /// Get all parent (upstream) block IDs for a given block.
    ///
    /// Returns blocks that have an `implement` relation pointing to this block.
    pub fn get_parents(&self, block_id: &str) -> Vec<String> {
        self.parents.get(block_id).cloned().unwrap_or_default()
    }

    /// Get all child (downstream) block IDs for a given block.
    ///
    /// Returns blocks that this block has an `implement` relation to.
    pub fn get_children(&self, block_id: &str) -> Vec<String> {
        self.blocks
            .get(block_id)
            .and_then(|b| b.children.get(RELATION_IMPLEMENT))
            .cloned()
            .unwrap_or_default()
    }

    /// Get the current transaction count for an editor.
    pub fn get_editor_count(&self, editor_id: &str) -> i64 {
        *self.editor_counts.get(editor_id).unwrap_or(&0)
    }

    /// 将指定 Block 的当前状态序列化为快照 JSON。
    ///
    /// 返回的 JSON 包含完整 Block 状态，可用于 CacheStore.save_snapshot()。
    pub fn to_snapshot_state(&self, block_id: &str) -> Option<serde_json::Value> {
        self.blocks.get(block_id).map(|block| {
            let mut snapshot = serde_json::json!({
                "block_id": block.block_id,
                "name": block.name,
                "block_type": block.block_type,
                "contents": block.contents,
                "children": block.children,
                "owner": block.owner
            });
            if let Some(desc) = &block.description {
                snapshot["description"] = serde_json::json!(desc);
            }
            snapshot
        })
    }

    /// 将所有 Block 的当前状态序列化为快照 HashMap。
    ///
    /// 返回 HashMap<block_id, state_json>，用于关闭时批量保存。
    pub fn all_snapshot_states(&self) -> std::collections::HashMap<String, serde_json::Value> {
        self.blocks
            .keys()
            .filter_map(|id| self.to_snapshot_state(id).map(|state| (id.clone(), state)))
            .collect()
    }

    /// 从快照 JSON 恢复一个 Block 到内存状态。
    ///
    /// 用于启动时从 cache.db 加载快照，跳过全量 event replay。
    pub fn restore_from_snapshot(&mut self, block_id: &str, state: &serde_json::Value) {
        if let Some(obj) = state.as_object() {
            let block = Block {
                block_id: block_id.to_string(),
                name: obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                block_type: obj
                    .get("block_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                owner: obj
                    .get("owner")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                contents: obj
                    .get("contents")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                children: obj
                    .get("children")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default(),
                description: obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            };

            // 恢复 reverse index
            if let Some(targets) = block.children.get(RELATION_IMPLEMENT) {
                for target in targets {
                    self.parents
                        .entry(target.clone())
                        .or_default()
                        .push(block.block_id.clone());
                }
            }

            self.blocks.insert(block_id.to_string(), block);
        }
    }

    /// Check for conflicts using vector clocks (MVP simple version).
    ///
    /// Returns true if the command is based on stale state.
    /// For MVP, we only check the command editor's count.
    pub fn has_conflict(&self, editor_id: &str, expected_count: i64) -> bool {
        let current_count = self.get_editor_count(editor_id);
        expected_count < current_count
    }

    /// 将完整 StateProjector 状态序列化为 JSON（用于 CacheStore 快照）。
    ///
    /// 包含 blocks + editors + grants + editor_counts + parents 五个字段。
    pub fn serialize_full_state(&self) -> serde_json::Value {
        // grants: 从 GrantsTable 提取为 HashMap<editor_id, Vec<(cap_id, block_id)>>
        let mut grants_map: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for (editor_id, cap_id, block_id) in self.grants.iter_all() {
            grants_map
                .entry(editor_id.to_string())
                .or_default()
                .push((cap_id.to_string(), block_id.to_string()));
        }

        serde_json::json!({
            "blocks": self.blocks,
            "editors": self.editors,
            "grants": grants_map,
            "editor_counts": self.editor_counts,
            "parents": self.parents,
        })
    }

    /// 从 JSON 快照恢复完整 StateProjector 状态。
    ///
    /// 与 `serialize_full_state()` 配对使用。恢复失败则保持当前状态不变。
    pub fn restore_full_state(&mut self, state: &serde_json::Value) -> bool {
        let obj = match state.as_object() {
            Some(o) => o,
            None => return false,
        };

        // blocks
        if let Some(blocks_val) = obj.get("blocks") {
            if let Ok(blocks) = serde_json::from_value::<HashMap<String, Block>>(blocks_val.clone())
            {
                self.blocks = blocks;
            } else {
                return false;
            }
        }

        // editors
        if let Some(editors_val) = obj.get("editors") {
            if let Ok(editors) =
                serde_json::from_value::<HashMap<String, Editor>>(editors_val.clone())
            {
                self.editors = editors;
            } else {
                return false;
            }
        }

        // grants
        if let Some(grants_val) = obj.get("grants") {
            if let Ok(grants_map) =
                serde_json::from_value::<HashMap<String, Vec<(String, String)>>>(grants_val.clone())
            {
                let mut table = GrantsTable::new();
                for (editor_id, pairs) in grants_map {
                    for (cap_id, block_id) in pairs {
                        table.add_grant(editor_id.clone(), cap_id, block_id);
                    }
                }
                self.grants = table;
            } else {
                return false;
            }
        }

        // editor_counts
        if let Some(counts_val) = obj.get("editor_counts") {
            if let Ok(counts) = serde_json::from_value::<HashMap<String, i64>>(counts_val.clone()) {
                self.editor_counts = counts;
            } else {
                return false;
            }
        }

        // parents
        if let Some(parents_val) = obj.get("parents") {
            if let Ok(parents) =
                serde_json::from_value::<HashMap<String, Vec<String>>>(parents_val.clone())
            {
                self.parents = parents;
            } else {
                return false;
            }
        }

        true
    }
}

impl Default for StateProjector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as StdHashMap;

    #[test]
    fn test_state_projector_create_block() {
        let mut state = StateProjector::new();

        let mut ts = StdHashMap::new();
        ts.insert("alice".to_string(), 1);

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "type": "document",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            ts,
        );

        state.apply_event(&event);

        assert_eq!(state.blocks.len(), 1);
        let block = state.get_block("block1").unwrap();
        assert_eq!(block.name, "Test Block");
        assert_eq!(block.block_type, "document");
        assert_eq!(block.owner, "alice");
    }

    #[test]
    fn test_state_projector_delete_block() {
        let mut state = StateProjector::new();

        // Create block
        let mut ts1 = StdHashMap::new();
        ts1.insert("alice".to_string(), 1);

        let create_event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "type": "document",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            ts1,
        );

        state.apply_event(&create_event);
        assert_eq!(state.blocks.len(), 1);

        // Delete block
        let mut ts2 = StdHashMap::new();
        ts2.insert("alice".to_string(), 2);

        let delete_event = Event::new(
            "block1".to_string(),
            "alice/core.delete".to_string(),
            serde_json::json!({ "deleted": true }),
            ts2,
        );

        state.apply_event(&delete_event);
        assert_eq!(state.blocks.len(), 0);
    }

    #[test]
    fn test_state_projector_editor_count() {
        let mut state = StateProjector::new();

        let mut ts = StdHashMap::new();
        ts.insert("alice".to_string(), 5);
        ts.insert("bob".to_string(), 3);

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test",
                "type": "document",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            ts,
        );

        state.apply_event(&event);

        assert_eq!(state.get_editor_count("alice"), 5);
        assert_eq!(state.get_editor_count("bob"), 3);
        assert_eq!(state.get_editor_count("charlie"), 0);
    }

    #[test]
    fn test_conflict_detection() {
        let mut state = StateProjector::new();

        let mut ts = StdHashMap::new();
        ts.insert("alice".to_string(), 5);

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test",
                "type": "document",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            ts,
        );

        state.apply_event(&event);

        // Command based on count 5 is ok (current state)
        assert!(!state.has_conflict("alice", 5));

        // Command based on count 4 is a conflict (stale)
        assert!(state.has_conflict("alice", 4));

        // Command based on count 6 is ok (newer)
        assert!(!state.has_conflict("alice", 6));
    }

    #[test]
    fn test_apply_create_event_with_description() {
        let mut state = StateProjector::new();

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "type": "document",
                "owner": "alice",
                "contents": {},
                "children": {},
                "description": "测试描述"
            }),
            {
                let mut ts = std::collections::HashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );

        state.apply_event(&event);

        assert_eq!(state.blocks.len(), 1);
        let block = state.get_block("block1").unwrap();
        assert_eq!(block.name, "Test Block");
        assert_eq!(block.description, Some("测试描述".to_string()));
    }

    #[test]
    fn test_apply_write_event_updates_contents() {
        let mut state = StateProjector::new();

        // 先创建 Block
        let create_event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test",
                "type": "document",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            {
                let mut ts = std::collections::HashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&create_event);

        // 写入内容
        let write_event = Event::new(
            "block1".to_string(),
            "alice/document.write".to_string(),
            serde_json::json!({
                "contents": {
                    "content": "# Hello"
                }
            }),
            {
                let mut ts = std::collections::HashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
        );
        state.apply_event(&write_event);

        let block = state.get_block("block1").unwrap();
        assert_eq!(block.contents["content"], "# Hello");
    }

    #[test]
    fn test_replay_with_description() {
        let mut state = StateProjector::new();

        let events = vec![
            Event::new(
                "block1".to_string(),
                "alice/core.create".to_string(),
                serde_json::json!({
                    "name": "Block 1",
                    "type": "document",
                    "owner": "alice",
                    "contents": {},
                    "children": {},
                    "description": "描述1"
                }),
                {
                    let mut ts = std::collections::HashMap::new();
                    ts.insert("alice".to_string(), 1);
                    ts
                },
            ),
            Event::new(
                "block1".to_string(),
                "alice/document.write".to_string(),
                serde_json::json!({
                    "contents": { "content": "内容" }
                }),
                {
                    let mut ts = std::collections::HashMap::new();
                    ts.insert("alice".to_string(), 2);
                    ts
                },
            ),
        ];

        state.replay(events);

        let block = state.get_block("block1").unwrap();
        assert_eq!(block.description, Some("描述1".to_string()));
        assert_eq!(block.contents["content"], "内容");
    }

    #[test]
    fn test_grant_event_adds_permission() {
        let mut state = StateProjector::new();

        let grant_event = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "document.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );

        state.apply_event(&grant_event);

        // Verify grant was added
        assert!(state.grants.has_grant("bob", "document.write", "block1"));
        assert!(!state.grants.has_grant("bob", "document.read", "block1"));
    }

    #[test]
    fn test_revoke_event_removes_permission() {
        let mut state = StateProjector::new();

        // First grant permission
        let grant_event = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "document.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&grant_event);

        // Verify grant exists
        assert!(state.grants.has_grant("bob", "document.write", "block1"));

        // Now revoke it
        let revoke_event = Event::new(
            "alice".to_string(),
            "alice/core.revoke".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "document.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
        );
        state.apply_event(&revoke_event);

        // CRITICAL: Verify revoke actually removed the grant (bug fix validation)
        assert!(!state.grants.has_grant("bob", "document.write", "block1"));
    }

    #[test]
    fn test_grant_and_revoke_multiple_permissions() {
        let mut state = StateProjector::new();

        // Grant multiple permissions
        let events = vec![
            Event::new(
                "alice".to_string(),
                "alice/core.grant".to_string(),
                serde_json::json!({
                    "editor": "bob",
                    "capability": "document.write",
                    "block": "block1"
                }),
                {
                    let mut ts = StdHashMap::new();
                    ts.insert("alice".to_string(), 1);
                    ts
                },
            ),
            Event::new(
                "alice".to_string(),
                "alice/core.grant".to_string(),
                serde_json::json!({
                    "editor": "bob",
                    "capability": "document.read",
                    "block": "block1"
                }),
                {
                    let mut ts = StdHashMap::new();
                    ts.insert("alice".to_string(), 2);
                    ts
                },
            ),
        ];

        for event in events {
            state.apply_event(&event);
        }

        // Both permissions should exist
        assert!(state.grants.has_grant("bob", "document.write", "block1"));
        assert!(state.grants.has_grant("bob", "document.read", "block1"));

        // Revoke one permission
        let revoke_event = Event::new(
            "alice".to_string(),
            "alice/core.revoke".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "document.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 3);
                ts
            },
        );
        state.apply_event(&revoke_event);

        // Only the revoked one should be removed
        assert!(!state.grants.has_grant("bob", "document.write", "block1"));
        assert!(state.grants.has_grant("bob", "document.read", "block1"));
    }

    #[test]
    fn test_wildcard_grant_applies_to_all_blocks() {
        let mut state = StateProjector::new();

        // Grant with wildcard block
        let grant_event = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "document.read",
                "block": "*"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&grant_event);

        // Should have permission on any block with wildcard
        assert!(state.grants.has_grant("bob", "document.read", "*"));
    }

    #[test]
    fn test_editor_create_event_adds_to_state() {
        let mut state = StateProjector::new();

        let editor_create_event = Event::new(
            "editor-123".to_string(),
            "system/editor.create".to_string(),
            serde_json::json!({
                "editor_id": "editor-123",
                "name": "Alice",
                "editor_type": "Human"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("system".to_string(), 1);
                ts
            },
        );

        state.apply_event(&editor_create_event);

        // Verify editor was added to state
        assert_eq!(state.editors.len(), 1);
        let editor = state.editors.get("editor-123").unwrap();
        assert_eq!(editor.name, "Alice");
        assert_eq!(editor.editor_type, crate::models::EditorType::Human);
    }

    #[test]
    fn test_editor_create_event_with_bot_type() {
        let mut state = StateProjector::new();

        let editor_create_event = Event::new(
            "bot-456".to_string(),
            "system/editor.create".to_string(),
            serde_json::json!({
                "editor_id": "bot-456",
                "name": "CodeReviewer",
                "editor_type": "Bot"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("system".to_string(), 1);
                ts
            },
        );

        state.apply_event(&editor_create_event);

        // Verify bot editor was added
        let editor = state.editors.get("bot-456").unwrap();
        assert_eq!(editor.name, "CodeReviewer");
        assert_eq!(editor.editor_type, crate::models::EditorType::Bot);
    }

    #[test]
    fn test_state_projector_delete_editor() {
        let mut state = StateProjector::new();

        // 1. Create editor
        let editor_id = "editor-123".to_string();
        let create_event = Event::new(
            editor_id.clone(),
            "system/editor.create".to_string(),
            serde_json::json!({
                "editor_id": &editor_id,
                "name": "Alice",
                "editor_type": "Human"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("system".to_string(), 1);
                ts
            },
        );
        state.apply_event(&create_event);

        // 2. Grant some permissions
        let grant_event = Event::new(
            "system".to_string(),
            "system/core.grant".to_string(),
            serde_json::json!({
                "editor": &editor_id,
                "capability": "document.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("system".to_string(), 2);
                ts
            },
        );
        state.apply_event(&grant_event);

        assert_eq!(state.editors.len(), 1);
        assert!(state
            .grants
            .has_grant(&editor_id, "document.write", "block1"));

        // 3. Delete editor
        let delete_event = Event::new(
            editor_id.clone(),
            "system/editor.delete".to_string(),
            serde_json::json!({ "deleted": true }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("system".to_string(), 3);
                ts
            },
        );
        state.apply_event(&delete_event);

        // 4. Verify editor and grants are gone
        assert_eq!(state.editors.len(), 0);
        assert!(state.editors.get(&editor_id).is_none());
        assert!(!state
            .grants
            .has_grant(&editor_id, "document.write", "block1"));
        assert!(state.grants.get_grants(&editor_id).is_none());
    }

    #[test]
    fn test_grant_revoke_with_empty_fields_ignored() {
        let mut state = StateProjector::new();

        // Grant event with empty editor field - should be ignored
        let grant_event = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "",
                "capability": "document.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&grant_event);

        // Should not have added grant with empty editor
        assert!(!state.grants.has_grant("", "document.write", "block1"));

        // Grant event with empty capability - should be ignored
        let grant_event2 = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
        );
        state.apply_event(&grant_event2);

        // Should not have added grant with empty capability
        assert!(!state.grants.has_grant("bob", "", "block1"));
    }

    #[test]
    fn test_apply_core_write_event() {
        let mut state = StateProjector::new();

        // 1. Create a block
        let create_event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test",
                "type": "document",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&create_event);

        let block = state.get_block("block1").unwrap();
        assert_eq!(block.name, "Test");
        assert!(block.description.is_none());

        // 2. Apply core.write event to update name and description
        let write_event = Event::new(
            "block1".to_string(),
            "alice/core.write".to_string(),
            serde_json::json!({
                "name": "New Name",
                "description": "New Description"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
        );
        state.apply_event(&write_event);

        // 3. Verify updates
        let block = state.get_block("block1").unwrap();
        assert_eq!(block.name, "New Name");
        assert_eq!(block.description, Some("New Description".to_string()));
    }

    // ========================================================================
    // Reverse index (parents) tests
    // ========================================================================

    fn create_block_event(entity: &str, name: &str, owner: &str, count: i64) -> Event {
        Event::new(
            entity.to_string(),
            format!("{}/core.create", owner),
            serde_json::json!({
                "name": name,
                "type": "document",
                "owner": owner,
                "contents": {},
                "children": {}
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert(owner.to_string(), count);
                ts
            },
        )
    }

    fn link_event(source: &str, target: &str, editor: &str, count: i64) -> Event {
        let mut children = StdHashMap::new();
        children.insert(RELATION_IMPLEMENT.to_string(), vec![target.to_string()]);
        Event::new(
            source.to_string(),
            format!("{}/core.link", editor),
            serde_json::json!({ "children": children }),
            {
                let mut ts = StdHashMap::new();
                ts.insert(editor.to_string(), count);
                ts
            },
        )
    }

    #[test]
    fn test_parents_after_link() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));

        // Link A → B
        state.apply_event(&link_event("a", "b", "alice", 3));

        assert_eq!(state.get_parents("b"), vec!["a".to_string()]);
        assert_eq!(state.get_children("a"), vec!["b".to_string()]);
        assert!(state.get_parents("a").is_empty());
        assert!(state.get_children("b").is_empty());
    }

    #[test]
    fn test_parents_after_unlink() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));

        // Link A → B
        state.apply_event(&link_event("a", "b", "alice", 3));
        assert_eq!(state.get_parents("b"), vec!["a".to_string()]);

        // Unlink A → B (children becomes empty)
        let unlink_event = Event::new(
            "a".to_string(),
            "alice/core.unlink".to_string(),
            serde_json::json!({ "children": {} }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 4);
                ts
            },
        );
        state.apply_event(&unlink_event);

        assert!(state.get_parents("b").is_empty());
        assert!(state.get_children("a").is_empty());
    }

    #[test]
    fn test_parents_multiple_parents() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));
        state.apply_event(&create_block_event("d", "D", "alice", 3));

        // A → D
        state.apply_event(&link_event("a", "d", "alice", 4));
        // B → D
        state.apply_event(&link_event("b", "d", "alice", 5));

        let parents = state.get_parents("d");
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&"a".to_string()));
        assert!(parents.contains(&"b".to_string()));
    }

    #[test]
    fn test_parents_cleanup_on_delete() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));

        // A → B
        state.apply_event(&link_event("a", "b", "alice", 3));
        assert_eq!(state.get_parents("b"), vec!["a".to_string()]);

        // Delete A
        let delete_event = Event::new(
            "a".to_string(),
            "alice/core.delete".to_string(),
            serde_json::json!({ "deleted": true }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 4);
                ts
            },
        );
        state.apply_event(&delete_event);

        // B should no longer have A as parent
        assert!(state.get_parents("b").is_empty());
        // A should not exist
        assert!(state.get_block("a").is_none());
    }

    #[test]
    fn test_delete_child_cleans_parent_children_map() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));
        state.apply_event(&create_block_event("c", "C", "alice", 3));

        // A → B, A → C
        let mut children = StdHashMap::new();
        children.insert(
            RELATION_IMPLEMENT.to_string(),
            vec!["b".to_string(), "c".to_string()],
        );
        let link_ev = Event::new(
            "a".to_string(),
            "alice/core.link".to_string(),
            serde_json::json!({ "children": children }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 4);
                ts
            },
        );
        state.apply_event(&link_ev);
        assert_eq!(state.get_children("a").len(), 2);

        // Delete B — A's children should shrink to [C]
        let delete_event = Event::new(
            "b".to_string(),
            "alice/core.delete".to_string(),
            serde_json::json!({}),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 5);
                ts
            },
        );
        state.apply_event(&delete_event);

        assert!(state.get_block("b").is_none());
        assert_eq!(state.get_children("a"), vec!["c".to_string()]);
        assert!(state.get_parents("b").is_empty());
    }

    #[test]
    fn test_get_children_convenience() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));
        state.apply_event(&create_block_event("c", "C", "alice", 3));

        // A → B, A → C (build children with both targets)
        let mut children = StdHashMap::new();
        children.insert(
            RELATION_IMPLEMENT.to_string(),
            vec!["b".to_string(), "c".to_string()],
        );
        let link_event = Event::new(
            "a".to_string(),
            "alice/core.link".to_string(),
            serde_json::json!({ "children": children }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 4);
                ts
            },
        );
        state.apply_event(&link_event);

        let children = state.get_children("a");
        assert_eq!(children.len(), 2);
        assert!(children.contains(&"b".to_string()));
        assert!(children.contains(&"c".to_string()));
    }

    // ========================================================================
    // remove_parent_entries helper tests
    // ========================================================================

    #[test]
    fn test_remove_parent_entries_basic() {
        let mut parents: StdHashMap<String, Vec<String>> = StdHashMap::new();
        parents.insert("child1".to_string(), vec!["parent1".to_string()]);
        parents.insert(
            "child2".to_string(),
            vec!["parent1".to_string(), "parent2".to_string()],
        );

        remove_parent_entries(
            &mut parents,
            "parent1",
            &["child1".to_string(), "child2".to_string()],
        );

        // child1 had only parent1, so entry should be removed entirely
        assert!(!parents.contains_key("child1"));
        // child2 still has parent2
        assert_eq!(parents.get("child2").unwrap(), &vec!["parent2".to_string()]);
    }

    #[test]
    fn test_remove_parent_entries_nonexistent_target() {
        let mut parents: StdHashMap<String, Vec<String>> = StdHashMap::new();
        parents.insert("child1".to_string(), vec!["parent1".to_string()]);

        // Should not panic when target doesn't exist in map
        remove_parent_entries(&mut parents, "parent1", &["nonexistent".to_string()]);

        // Original entry unchanged
        assert_eq!(parents.get("child1").unwrap(), &vec!["parent1".to_string()]);
    }

    // ========================================================================
    // Mode-aware apply_event tests
    // ========================================================================

    #[test]
    fn test_write_full_mode_merges_contents() {
        let mut state = StateProjector::new();

        // 创建 Block
        state.apply_event(&create_block_event("block1", "Test", "alice", 1));

        // Full mode write：合并 contents
        let write_event = Event::new_with_mode(
            "block1".to_string(),
            "alice/document.write".to_string(),
            serde_json::json!({
                "contents": {"content": "# Hello"}
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
            EventMode::Full,
        );
        state.apply_event(&write_event);

        let block = state.get_block("block1").unwrap();
        assert_eq!(block.contents["content"], "# Hello");
    }

    #[test]
    fn test_write_ref_mode_replaces_contents() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("block1", "Image", "alice", 1));

        // Ref mode：整个 contents 被替换为 ref 元数据
        let ref_event = Event::new_with_mode(
            "block1".to_string(),
            "alice/document.write".to_string(),
            serde_json::json!({
                "contents": {
                    "hash": "sha256:abc123",
                    "path": "assets/logo.png",
                    "size": 1024
                }
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
            EventMode::Ref,
        );
        state.apply_event(&ref_event);

        let block = state.get_block("block1").unwrap();
        assert_eq!(block.contents["hash"], "sha256:abc123");
        assert_eq!(block.contents["path"], "assets/logo.png");
        assert_eq!(block.contents["size"], 1024);
    }

    #[test]
    fn test_write_append_mode_accumulates_entries() {
        let mut state = StateProjector::new();

        // 创建 Session Block，初始 contents 为空
        let create_event = Event::new(
            "session1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Session",
                "type": "session",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&create_event);

        // Append 第一条 entry
        let append1 = Event::new_with_mode(
            "session1".to_string(),
            "alice/session.write".to_string(),
            serde_json::json!({
                "entry": {"entry_type": "message", "role": "user", "content": "hello"}
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
            EventMode::Append,
        );
        state.apply_event(&append1);

        // Append 第二条 entry
        let append2 = Event::new_with_mode(
            "session1".to_string(),
            "alice/session.write".to_string(),
            serde_json::json!({
                "entry": {"entry_type": "message", "role": "assistant", "content": "hi"}
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 3);
                ts
            },
            EventMode::Append,
        );
        state.apply_event(&append2);

        let block = state.get_block("session1").unwrap();
        let entries = block.contents["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["role"], "user");
        assert_eq!(entries[1]["role"], "assistant");
    }

    #[test]
    fn test_write_delta_mode_stores_diff() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("block1", "Doc", "alice", 1));

        // Delta mode：当前存储 diff 内容（placeholder，Step 5 实现真正的 diff apply）
        let delta_event = Event::new_with_mode(
            "block1".to_string(),
            "alice/document.write".to_string(),
            serde_json::json!({
                "contents": {"diff": "@@ -1 +1 @@\n-old\n+new"}
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
            EventMode::Delta,
        );
        state.apply_event(&delta_event);

        let block = state.get_block("block1").unwrap();
        assert!(block.contents["diff"].as_str().unwrap().contains("@@"));
    }

    // ========================================================================
    // Snapshot serialization/deserialization tests
    // ========================================================================

    #[test]
    fn test_to_snapshot_state() {
        let mut state = StateProjector::new();

        let create_event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "type": "document",
                "owner": "alice",
                "contents": {"content": "# Hello"},
                "children": {},
                "description": "desc"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&create_event);

        let snapshot = state.to_snapshot_state("block1").unwrap();
        assert_eq!(snapshot["block_id"], "block1");
        assert_eq!(snapshot["name"], "Test Block");
        assert_eq!(snapshot["block_type"], "document");
        assert_eq!(snapshot["owner"], "alice");
        assert_eq!(snapshot["contents"]["content"], "# Hello");
        assert_eq!(snapshot["description"], "desc");
    }

    #[test]
    fn test_to_snapshot_state_nonexistent_block() {
        let state = StateProjector::new();
        assert!(state.to_snapshot_state("nonexistent").is_none());
    }

    #[test]
    fn test_all_snapshot_states() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));

        let snapshots = state.all_snapshot_states();
        assert_eq!(snapshots.len(), 2);
        assert!(snapshots.contains_key("a"));
        assert!(snapshots.contains_key("b"));
    }

    #[test]
    fn test_restore_from_snapshot() {
        let mut state = StateProjector::new();

        let snapshot = serde_json::json!({
            "block_id": "block1",
            "name": "Restored Block",
            "block_type": "document",
            "owner": "alice",
            "contents": {"content": "# Restored"},
            "children": {},
            "description": "restored"
        });

        state.restore_from_snapshot("block1", &snapshot);

        let block = state.get_block("block1").unwrap();
        assert_eq!(block.name, "Restored Block");
        assert_eq!(block.block_type, "document");
        assert_eq!(block.owner, "alice");
        assert_eq!(block.contents["content"], "# Restored");
        assert_eq!(block.description, Some("restored".to_string()));
    }

    #[test]
    fn test_restore_from_snapshot_with_children() {
        let mut state = StateProjector::new();

        // 先创建子 Block
        state.apply_event(&create_block_event("child1", "C1", "alice", 1));

        // 恢复带 children 的父 Block
        let snapshot = serde_json::json!({
            "block_id": "parent",
            "name": "Parent",
            "block_type": "document",
            "owner": "alice",
            "contents": {},
            "children": {RELATION_IMPLEMENT: ["child1"]}
        });

        state.restore_from_snapshot("parent", &snapshot);

        // 验证 reverse index
        assert_eq!(state.get_parents("child1"), vec!["parent".to_string()]);
        assert_eq!(state.get_children("parent"), vec!["child1".to_string()]);
    }

    #[test]
    fn test_snapshot_roundtrip() {
        // 创建 → 序列化 → 新 state → 恢复 → 验证一致
        let mut state1 = StateProjector::new();

        let create = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Original",
                "type": "document",
                "owner": "alice",
                "contents": {"content": "# Content"},
                "children": {}
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state1.apply_event(&create);

        let snapshot = state1.to_snapshot_state("block1").unwrap();

        let mut state2 = StateProjector::new();
        state2.restore_from_snapshot("block1", &snapshot);

        let b1 = state1.get_block("block1").unwrap();
        let b2 = state2.get_block("block1").unwrap();
        assert_eq!(b1.name, b2.name);
        assert_eq!(b1.block_type, b2.block_type);
        assert_eq!(b1.owner, b2.owner);
        assert_eq!(b1.contents, b2.contents);
    }
}

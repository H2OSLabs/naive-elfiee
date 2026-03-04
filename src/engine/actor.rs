use crate::capabilities::registry::CapabilityRegistry;
use crate::engine::cache_store::CacheStore;
use crate::engine::event_store::{EventPoolWithPath, EventStore};
use crate::engine::state::StateProjector;
use crate::models::{Block, Command, Editor, Event, LinkBlockPayload, RELATION_IMPLEMENT};
use sqlx::sqlite::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::sync::{mpsc, oneshot};

/// Messages that can be sent to the engine actor.
#[derive(Debug)]
pub enum EngineMessage {
    /// Process a command and return resulting events
    ProcessCommand {
        command: Command,
        response: oneshot::Sender<Result<Vec<Event>, String>>,
    },
    /// Get a block by ID
    GetBlock {
        block_id: String,
        response: oneshot::Sender<Option<Block>>,
    },
    /// Get all blocks
    GetAllBlocks {
        response: oneshot::Sender<HashMap<String, Block>>,
    },
    /// Get all editors
    GetAllEditors {
        response: oneshot::Sender<HashMap<String, Editor>>,
    },
    /// Get all grants as a map: editor_id -> Vec<(cap_id, block_id)>
    GetAllGrants {
        response: oneshot::Sender<HashMap<String, Vec<(String, String)>>>,
    },
    /// Get grants for a specific editor
    GetEditorGrants {
        editor_id: String,
        response: oneshot::Sender<Vec<(String, String)>>,
    },
    /// Get grants for a specific block
    GetBlockGrants {
        block_id: String,
        response: oneshot::Sender<Vec<(String, String, String)>>,
    },
    /// Check if an editor is authorized for a capability on a block
    CheckGrant {
        editor_id: String,
        cap_id: String,
        block_id: String,
        response: oneshot::Sender<bool>,
    },
    /// Get all events
    GetAllEvents {
        response: oneshot::Sender<Result<Vec<Event>, String>>,
    },
    /// Get events for a specific entity (block or editor)
    GetEventsByEntity {
        entity: String,
        response: oneshot::Sender<Result<Vec<Event>, String>>,
    },
    /// Get events after a specific event ID (for incremental replay)
    GetEventsAfterEventId {
        after_event_id: String,
        response: oneshot::Sender<Result<Vec<Event>, String>>,
    },
    /// Get the latest event ID
    GetLatestEventId {
        response: oneshot::Sender<Result<Option<String>, String>>,
    },
    /// Shutdown the actor
    Shutdown,
}

/// CacheStore 中全量 StateProjector 快照的 key。
const CACHE_KEY_PROJECTOR: &str = "__projector__";

/// Actor that processes commands for a single .elf file.
///
/// Each file has its own engine actor, ensuring serial processing of commands
/// for that file. This prevents race conditions and maintains consistency.
pub struct ElfileEngineActor {
    /// Unique identifier for this file
    #[allow(dead_code)]
    file_id: String,

    /// Event pool with database path for temp_dir derivation
    event_pool_with_path: EventPoolWithPath,

    /// Current state projection
    state: StateProjector,

    /// Capability registry
    registry: CapabilityRegistry,

    /// Mailbox for receiving messages
    mailbox: mpsc::UnboundedReceiver<EngineMessage>,

    /// 本机快照缓存池（`:memory:` 测试时为 None）
    cache_pool: Option<SqlitePool>,
}

impl ElfileEngineActor {
    /// Check if linking source → target would create a cycle in the DAG.
    ///
    /// From target, DFS along `implement` children. If we reach source,
    /// a cycle would be formed: source → target → ... → source.
    /// Also rejects self-links (source == target).
    fn check_link_cycle(&self, source_id: &str, target_id: &str) -> Result<(), String> {
        // Self-link is always a cycle
        if source_id == target_id {
            return Err(format!(
                "Cycle detected: linking {} → {} would create a self-cycle",
                source_id, target_id
            ));
        }

        let mut visited = HashSet::new();
        let mut stack = vec![target_id.to_string()];

        while let Some(current) = stack.pop() {
            if current == source_id {
                return Err(format!(
                    "Cycle detected: linking {} → {} would create a cycle",
                    source_id, target_id
                ));
            }
            if visited.insert(current.clone()) {
                if let Some(block) = self.state.get_block(&current) {
                    if let Some(targets) = block.children.get(RELATION_IMPLEMENT) {
                        stack.extend(targets.iter().cloned());
                    }
                }
            }
        }
        Ok(())
    }

    /// Create a new engine actor for a file.
    ///
    /// 启动策略：
    /// 1. 尝试从 CacheStore 加载快照 + 增量 replay（快）
    /// 2. 失败则全量 replay（慢但总是正确）
    pub async fn new(
        file_id: String,
        event_pool_with_path: EventPoolWithPath,
        mailbox: mpsc::UnboundedReceiver<EngineMessage>,
    ) -> Result<Self, String> {
        let registry = CapabilityRegistry::new();
        let mut state = StateProjector::new();

        // 尝试打开本机缓存
        let cache_pool = Self::try_open_cache(&event_pool_with_path.db_path).await;
        let mut used_cache = false;

        // 快速路径：从快照恢复 + 增量 replay
        if let Some(ref cache) = cache_pool {
            if let Ok(Some((cached_event_id, cached_state))) =
                CacheStore::get_latest_snapshot(cache, CACHE_KEY_PROJECTOR).await
            {
                if state.restore_full_state(&cached_state) {
                    // 只 replay 快照之后的增量事件
                    match EventStore::get_events_after_event_id(
                        &event_pool_with_path.pool,
                        &cached_event_id,
                    )
                    .await
                    {
                        Ok(incremental) => {
                            state.replay(incremental);
                            used_cache = true;
                        }
                        Err(_) => {
                            // 快照可能过期，回退到全量 replay
                            state = StateProjector::new();
                        }
                    }
                }
            }
        }

        // 慢路径：全量 replay
        if !used_cache {
            let events = EventStore::get_all_events(&event_pool_with_path.pool)
                .await
                .map_err(|e| format!("Failed to load events from database: {}", e))?;
            state.replay(events);
        }

        Ok(Self {
            file_id,
            event_pool_with_path,
            state,
            registry,
            mailbox,
            cache_pool,
        })
    }

    /// 尝试打开本机缓存数据库。
    ///
    /// 仅当 db_path 符合 `<project>/.elf/eventstore.db` 结构时才启用缓存。
    /// `:memory:` 测试和非标准路径返回 None（不影响功能，只跳过缓存）。
    async fn try_open_cache(db_path: &Path) -> Option<SqlitePool> {
        let db_str = db_path.to_str()?;
        if db_str == ":memory:" || db_str.is_empty() {
            return None;
        }

        // 验证路径结构：parent 必须是 .elf 目录
        let elf_dir = db_path.parent()?;
        if elf_dir.file_name()?.to_str()? != ".elf" {
            return None;
        }

        // db_path = /project/.elf/eventstore.db → project_path = /project
        let project_path = elf_dir.parent()?;
        let cache_path = CacheStore::cache_path_for_project(project_path);
        let cache_str = cache_path.to_str()?;

        CacheStore::create(cache_str).await.ok()
    }

    /// 关闭时保存全量 StateProjector 快照到本机缓存。
    async fn save_cache_snapshot(&self) {
        if let Some(ref cache) = self.cache_pool {
            if let Ok(Some(latest_event_id)) =
                EventStore::get_latest_event_id(&self.event_pool_with_path.pool).await
            {
                let full_state = self.state.serialize_full_state();
                let _ = CacheStore::save_snapshot(
                    cache,
                    CACHE_KEY_PROJECTOR,
                    &latest_event_id,
                    &full_state,
                )
                .await;
            }
        }
    }

    /// Run the actor's main loop.
    ///
    /// This processes messages from the mailbox until a Shutdown message is received.
    pub async fn run(mut self) {
        while let Some(msg) = self.mailbox.recv().await {
            match msg {
                EngineMessage::ProcessCommand { command, response } => {
                    let result = self.process_command(command).await;
                    let _ = response.send(result);
                }
                EngineMessage::GetBlock { block_id, response } => {
                    let block = self.state.get_block(&block_id).cloned();
                    let _ = response.send(block);
                }
                EngineMessage::GetAllBlocks { response } => {
                    let blocks = self.state.blocks.clone();
                    let _ = response.send(blocks);
                }
                EngineMessage::GetAllEditors { response } => {
                    let editors = self.state.editors.clone();
                    let _ = response.send(editors);
                }
                EngineMessage::GetAllGrants { response } => {
                    let mut grants: std::collections::HashMap<String, Vec<(String, String)>> =
                        std::collections::HashMap::new();
                    for (editor_id, cap_id, block_id) in self.state.grants.iter_all() {
                        grants
                            .entry(editor_id.to_string())
                            .or_default()
                            .push((cap_id.to_string(), block_id.to_string()));
                    }
                    let _ = response.send(grants);
                }
                EngineMessage::GetAllEvents { response } => {
                    let events = EventStore::get_all_events(&self.event_pool_with_path.pool)
                        .await
                        .map_err(|e| format!("Failed to get events: {}", e));
                    let _ = response.send(events);
                }
                EngineMessage::GetEventsByEntity { entity, response } => {
                    let events =
                        EventStore::get_events_by_entity(&self.event_pool_with_path.pool, &entity)
                            .await
                            .map_err(|e| format!("Failed to get events by entity: {}", e));
                    let _ = response.send(events);
                }
                EngineMessage::GetEventsAfterEventId {
                    after_event_id,
                    response,
                } => {
                    let events = EventStore::get_events_after_event_id(
                        &self.event_pool_with_path.pool,
                        &after_event_id,
                    )
                    .await
                    .map_err(|e| format!("Failed to get events after event: {}", e));
                    let _ = response.send(events);
                }
                EngineMessage::GetLatestEventId { response } => {
                    let result = EventStore::get_latest_event_id(&self.event_pool_with_path.pool)
                        .await
                        .map_err(|e| format!("Failed to get latest event id: {}", e));
                    let _ = response.send(result);
                }
                EngineMessage::GetEditorGrants {
                    editor_id,
                    response,
                } => {
                    let grants = self
                        .state
                        .grants
                        .get_grants(&editor_id)
                        .cloned()
                        .unwrap_or_default();
                    let _ = response.send(grants);
                }
                EngineMessage::GetBlockGrants { block_id, response } => {
                    // Get all grants and filter those that apply to this block
                    let mut block_grants = Vec::new();
                    for (editor_id, cap_id, target_block) in self.state.grants.iter_all() {
                        if target_block == block_id || target_block == "*" {
                            block_grants.push((
                                editor_id.to_string(),
                                cap_id.to_string(),
                                target_block.to_string(),
                            ));
                        }
                    }
                    let _ = response.send(block_grants);
                }
                EngineMessage::CheckGrant {
                    editor_id,
                    cap_id,
                    block_id,
                    response,
                } => {
                    // Pure event-sourcing auth: owner check + grants check
                    let authorized = self
                        .state
                        .get_block(&block_id)
                        .map(|b| b.owner == editor_id)
                        .unwrap_or(false)
                        || self.state.grants.has_grant(&editor_id, &cap_id, &block_id);
                    let _ = response.send(authorized);
                }
                EngineMessage::Shutdown => {
                    // 关闭前保存快照到本机缓存
                    self.save_cache_snapshot().await;
                    break;
                }
            }
        }
    }

    /// Process a command and return resulting events.
    ///
    /// This is the core command processing logic:
    /// 1. Get capability handler
    /// 2. Get block (None for create, Some for others)
    /// 3. Check authorization (certificator)
    /// 4. Execute handler
    /// 5. Update vector clock
    /// 6. Check for conflicts (MVP simple version)
    /// 7. Commit events to EventStore
    /// 8. Apply events to StateProjector
    async fn process_command(&mut self, cmd: Command) -> Result<Vec<Event>, String> {
        // 1. Get capability handler
        let handler = self
            .registry
            .get(&cmd.cap_id)
            .ok_or_else(|| format!("Unknown capability: {}", cmd.cap_id))?;

        // 2. Get block (None for create operations, Some for others)
        // System-level operations like core.create, editor.create, editor.delete don't require a block.
        // Wildcard grants/revokes (block_id = "*") also skip block lookup since "*" is not a real block.
        let block_opt = if cmd.cap_id == "core.create"
            || cmd.cap_id == "editor.create"
            || cmd.cap_id == "editor.delete"
        {
            None
        } else if (cmd.cap_id == "core.grant" || cmd.cap_id == "core.revoke") && cmd.block_id == "*"
        {
            // Wildcard grant/revoke: no specific block to look up.
            // Authorization is handled by the caller (services layer).
            None
        } else {
            Some(
                self.state
                    .get_block(&cmd.block_id)
                    .ok_or_else(|| format!("Block not found: {}", cmd.block_id))?
                    .clone(), // Clone so we can modify it
            )
        };

        // 3. Check authorization (certificator) — always called, no exceptions
        // Every operation requires authorization (CBAC: "每个操作都需要授权")
        if !handler.certificator(&cmd.editor_id, block_opt.as_ref(), &self.state.grants) {
            return Err(format!(
                "Authorization failed: {} does not have permission for {} on block {}",
                cmd.editor_id, cmd.cap_id, cmd.block_id
            ));
        }

        // 3.5. DAG cycle detection for core.link
        if cmd.cap_id == "core.link" {
            let payload: LinkBlockPayload = serde_json::from_value(cmd.payload.clone())
                .map_err(|e| format!("Invalid payload for cycle check: {}", e))?;
            self.check_link_cycle(&cmd.block_id, &payload.target_id)?;
        }

        // 4. Execute handler
        let mut events = handler.handler(&cmd, block_opt.as_ref())?;

        // 5. Update vector clock
        // Get the full current vector clock state and increment the current editor's count
        let mut full_timestamp = self.state.editor_counts.clone();
        let current_count = *full_timestamp.get(&cmd.editor_id).unwrap_or(&0);
        let new_count = current_count + 1;
        full_timestamp.insert(cmd.editor_id.clone(), new_count);

        for event in &mut events {
            event.timestamp = full_timestamp.clone();
        }

        // 6. Check for conflicts (MVP simple version)
        // For MVP, we just log if there's a potential conflict but don't reject
        // In production, this would trigger merge/resolution logic
        if self.state.has_conflict(&cmd.editor_id, current_count) {
            log::warn!(
                "Potential conflict detected for editor {} (expected: {}, current: {})",
                cmd.editor_id,
                current_count,
                self.state.get_editor_count(&cmd.editor_id)
            );
        }

        // 7. Persist events to database
        EventStore::append_events(&self.event_pool_with_path.pool, &events)
            .await
            .map_err(|e| format!("Failed to persist events to database: {}", e))?;

        // 8. Apply events to StateProjector
        for event in &events {
            self.state.apply_event(event);
        }

        Ok(events)
    }
}

/// Handle for interacting with an engine actor.
///
/// This provides an async API for sending messages to the actor.
#[derive(Clone, Debug)]
pub struct EngineHandle {
    sender: mpsc::UnboundedSender<EngineMessage>,
}

impl EngineHandle {
    /// Create a new handle with the given sender.
    pub fn new(sender: mpsc::UnboundedSender<EngineMessage>) -> Self {
        Self { sender }
    }

    /// Process a command and return resulting events.
    pub async fn process_command(&self, command: Command) -> Result<Vec<Event>, String> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(EngineMessage::ProcessCommand {
                command,
                response: tx,
            })
            .map_err(|_| "Engine actor has shut down".to_string())?;

        rx.await
            .map_err(|_| "Engine actor did not respond".to_string())?
    }

    /// Get a block by ID.
    pub async fn get_block(&self, block_id: String) -> Option<Block> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(EngineMessage::GetBlock {
                block_id,
                response: tx,
            })
            .ok()?;

        rx.await.ok()?
    }

    /// Get all blocks.
    pub async fn get_all_blocks(&self) -> HashMap<String, Block> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetAllBlocks { response: tx })
            .is_err()
        {
            return HashMap::new();
        }

        rx.await.unwrap_or_default()
    }

    /// Get all editors.
    pub async fn get_all_editors(&self) -> HashMap<String, Editor> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetAllEditors { response: tx })
            .is_err()
        {
            return HashMap::new();
        }

        rx.await.unwrap_or_default()
    }

    /// Get all grants.
    ///
    /// Returns a map of editor_id -> Vec<(cap_id, block_id)>
    pub async fn get_all_grants(&self) -> HashMap<String, Vec<(String, String)>> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetAllGrants { response: tx })
            .is_err()
        {
            return HashMap::new();
        }

        rx.await.unwrap_or_default()
    }

    /// Get grants for a specific editor.
    ///
    /// Returns Vec<(cap_id, block_id)> for the given editor.
    pub async fn get_editor_grants(&self, editor_id: String) -> Vec<(String, String)> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetEditorGrants {
                editor_id,
                response: tx,
            })
            .is_err()
        {
            return Vec::new();
        }

        rx.await.unwrap_or_default()
    }

    /// Get grants for a specific block.
    ///
    /// Returns Vec<(editor_id, cap_id, block_id)> for all grants that apply to this block.
    pub async fn get_block_grants(&self, block_id: String) -> Vec<(String, String, String)> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetBlockGrants {
                block_id,
                response: tx,
            })
            .is_err()
        {
            return Vec::new();
        }

        rx.await.unwrap_or_default()
    }

    /// Check if an editor is authorized for a capability on a block.
    pub async fn check_grant(&self, editor_id: String, cap_id: String, block_id: String) -> bool {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::CheckGrant {
                editor_id,
                cap_id,
                block_id,
                response: tx,
            })
            .is_err()
        {
            return false;
        }

        rx.await.unwrap_or(false)
    }

    /// Get all events.
    ///
    /// Returns all events from the event store for this file.
    pub async fn get_all_events(&self) -> Result<Vec<Event>, String> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetAllEvents { response: tx })
            .is_err()
        {
            return Err("Engine actor has shut down".to_string());
        }

        rx.await
            .map_err(|_| "Engine actor did not respond".to_string())?
    }

    /// Get events for a specific entity (block or editor).
    pub async fn get_events_by_entity(&self, entity: String) -> Result<Vec<Event>, String> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(EngineMessage::GetEventsByEntity {
                entity,
                response: tx,
            })
            .map_err(|_| "Engine actor has shut down".to_string())?;

        rx.await
            .map_err(|_| "Engine actor did not respond".to_string())?
    }

    /// Get events after a specific event ID.
    pub async fn get_events_after_event_id(
        &self,
        after_event_id: String,
    ) -> Result<Vec<Event>, String> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(EngineMessage::GetEventsAfterEventId {
                after_event_id,
                response: tx,
            })
            .map_err(|_| "Engine actor has shut down".to_string())?;

        rx.await
            .map_err(|_| "Engine actor did not respond".to_string())?
    }

    /// Get the latest event ID.
    pub async fn get_latest_event_id(&self) -> Result<Option<String>, String> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(EngineMessage::GetLatestEventId { response: tx })
            .map_err(|_| "Engine actor has shut down".to_string())?;

        rx.await
            .map_err(|_| "Engine actor did not respond".to_string())?
    }

    /// Shutdown the engine actor.
    pub async fn shutdown(&self) {
        let _ = self.sender.send(EngineMessage::Shutdown);
    }
}

/// Spawn a new engine actor for a file.
///
/// Returns a handle for interacting with the actor.
pub async fn spawn_engine(
    file_id: String,
    event_pool_with_path: EventPoolWithPath,
) -> Result<EngineHandle, String> {
    let (tx, rx) = mpsc::unbounded_channel();

    let actor = ElfileEngineActor::new(file_id.clone(), event_pool_with_path, rx).await?;

    // Spawn the actor on tokio runtime
    tokio::spawn(async move {
        actor.run().await;
    });

    Ok(EngineHandle::new(tx))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seed bootstrap events for a test editor directly to EventStore.
    /// Writes editor.create + wildcard core.grant events for all registered capabilities.
    /// Must be called BEFORE spawn_engine() so the engine replays these during init.
    async fn seed_test_editor(event_pool: &EventPoolWithPath, editor_id: &str) {
        let registry = CapabilityRegistry::new();
        let cap_ids = registry.get_grantable_cap_ids(&[]);

        let mut events = Vec::new();

        let mut ts = HashMap::new();
        ts.insert(editor_id.to_string(), 1);

        // editor.create event
        events.push(Event::new(
            editor_id.to_string(),
            format!("{}/editor.create", editor_id),
            serde_json::json!({
                "editor_id": editor_id,
                "name": editor_id,
                "editor_type": "Human"
            }),
            ts,
        ));

        // Wildcard grants for all capabilities
        for (i, cap_id) in cap_ids.iter().enumerate() {
            let mut grant_ts = HashMap::new();
            grant_ts.insert(editor_id.to_string(), (i + 2) as i64);

            events.push(Event::new(
                "*".to_string(),
                format!("{}/core.grant", editor_id),
                serde_json::json!({
                    "editor": editor_id,
                    "capability": cap_id,
                    "block": "*"
                }),
                grant_ts,
            ));
        }

        EventStore::append_events(&event_pool.pool, &events)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_engine_actor_creation() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test_file".to_string(), event_pool)
            .await
            .expect("Failed to spawn engine");

        // Test that we can get all blocks (should be empty initially)
        let blocks = handle.get_all_blocks().await;
        assert_eq!(blocks.len(), 0);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_create_block() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "document"
            }),
        );

        let events = handle
            .process_command(cmd)
            .await
            .expect("Failed to create block");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].attribute, "alice/core.create");

        let blocks = handle.get_all_blocks().await;
        assert_eq!(blocks.len(), 1);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_authorization_owner() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // Create a block owned by alice
        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Alice's Block",
                "block_type": "document"
            }),
        );

        let create_events = handle
            .process_command(create_cmd)
            .await
            .expect("Failed to create block");
        let block_id = &create_events[0].entity;

        // Alice (owner) should be able to link
        let link_cmd = Command::new(
            "alice".to_string(),
            "core.link".to_string(),
            block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "other_block"
            }),
        );

        let result = handle.process_command(link_cmd).await;
        assert!(result.is_ok(), "Owner should be authorized");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_authorization_non_owner_rejected() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        // Only bootstrap alice — bob has no grants
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // Create a block owned by alice
        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Alice's Block",
                "block_type": "document"
            }),
        );

        let create_events = handle
            .process_command(create_cmd)
            .await
            .expect("Failed to create block");
        let block_id = &create_events[0].entity;

        // Bob (non-owner, no grants) should NOT be able to link
        let link_cmd = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "other_block"
            }),
        );

        let result = handle.process_command(link_cmd).await;
        assert!(
            result.is_err(),
            "Non-owner without grants should be rejected"
        );
        assert!(result.unwrap_err().contains("Authorization failed"));

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_authorization_with_grant() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        // Only bootstrap alice — bob gets grant from alice below
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // Create a block owned by alice
        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Alice's Block",
                "block_type": "document"
            }),
        );

        let create_events = handle
            .process_command(create_cmd)
            .await
            .expect("Failed to create block");
        let block_id = &create_events[0].entity;

        // Alice grants bob core.link on the specific block
        let grant_cmd = Command::new(
            "alice".to_string(),
            "core.grant".to_string(),
            block_id.clone(),
            serde_json::json!({
                "target_editor": "bob",
                "capability": "core.link",
                "target_block": block_id
            }),
        );

        handle
            .process_command(grant_cmd)
            .await
            .expect("Failed to grant permission");

        // Bob should now be able to link
        let link_cmd = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "other_block"
            }),
        );

        let result = handle.process_command(link_cmd).await;
        assert!(result.is_ok(), "User with grant should be authorized");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_vector_clock_updates() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        let cmd1 = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Block 1",
                "block_type": "document"
            }),
        );

        let events1 = handle
            .process_command(cmd1)
            .await
            .expect("Failed to create block 1");

        let cmd2 = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Block 2",
                "block_type": "document"
            }),
        );

        let events2 = handle
            .process_command(cmd2)
            .await
            .expect("Failed to create block 2");

        // Verify vector clock increments between commands (offset from bootstrap events)
        let clock1 = *events1[0].timestamp.get("alice").unwrap();
        let clock2 = *events2[0].timestamp.get("alice").unwrap();
        assert_eq!(clock2, clock1 + 1, "Vector clock should increment by 1");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_get_block() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "document"
            }),
        );

        let events = handle
            .process_command(create_cmd)
            .await
            .expect("Failed to create block");
        let block_id = &events[0].entity;

        let block = handle
            .get_block(block_id.clone())
            .await
            .expect("Block should exist");

        assert_eq!(block.name, "Test Block");
        assert_eq!(block.block_type, "document");
        assert_eq!(block.owner, "alice");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_create_block_with_description() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "测试文档",
                "block_type": "document",
                "description": "这是一个测试文档"
            }),
        );

        let events = handle
            .process_command(cmd)
            .await
            .expect("Failed to create block");

        assert_eq!(events.len(), 1);

        let block_id = events[0].entity.clone();
        let block = handle.get_block(block_id).await.unwrap();

        assert_eq!(block.name, "测试文档");
        assert_eq!(block.description, Some("这是一个测试文档".to_string()));

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_write_updates_contents() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test",
                "block_type": "document"
            }),
        );

        let events = handle.process_command(create_cmd).await.unwrap();
        let block_id = events[0].entity.clone();

        let write_cmd = Command::new(
            "alice".to_string(),
            "document.write".to_string(),
            block_id.clone(),
            serde_json::json!({
                "content": "# Hello World"
            }),
        );

        let result = handle.process_command(write_cmd).await;
        assert!(result.is_ok());

        let block = handle.get_block(block_id).await.unwrap();
        assert_eq!(block.contents["content"], "# Hello World");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_description_persists_after_replay() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("events.db");
        let event_pool = EventStore::create(db_path.to_str().unwrap()).await.unwrap();
        seed_test_editor(&event_pool, "alice").await;

        // First engine: create block
        {
            let handle = spawn_engine("test_file".to_string(), event_pool.clone())
                .await
                .unwrap();

            let cmd = Command::new(
                "alice".to_string(),
                "core.create".to_string(),
                "".to_string(),
                serde_json::json!({
                    "name": "持久化测试",
                    "block_type": "document",
                    "description": "测试持久化"
                }),
            );

            handle.process_command(cmd).await.unwrap();
            handle.shutdown().await;
        }

        // Second engine: replay events and verify
        {
            let handle = spawn_engine("test_file".to_string(), event_pool.clone())
                .await
                .unwrap();

            let blocks = handle.get_all_blocks().await;
            assert_eq!(blocks.len(), 1);

            let block = blocks.values().next().unwrap();
            assert_eq!(block.name, "持久化测试");
            assert_eq!(block.description, Some("测试持久化".to_string()));

            handle.shutdown().await;
        }
    }

    #[tokio::test]
    async fn test_wildcard_grant_succeeds() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test_file".to_string(), event_pool)
            .await
            .unwrap();

        // Create an editor (alice has editor.create wildcard grant)
        let create_editor_cmd = Command::new(
            "alice".to_string(),
            "editor.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "Bob", "editor_type": "Bot" }),
        );
        let events = handle.process_command(create_editor_cmd).await.unwrap();
        let bob_id = events[0].entity.clone();

        // Wildcard grant (alice has core.grant wildcard grant)
        let grant_cmd = Command::new(
            "alice".to_string(),
            "core.grant".to_string(),
            "*".to_string(),
            serde_json::json!({
                "target_editor": bob_id,
                "capability": "document.read",
                "target_block": "*"
            }),
        );
        let result = handle.process_command(grant_cmd).await;
        assert!(
            result.is_ok(),
            "Wildcard grant should succeed, got: {:?}",
            result.err()
        );

        let has_grant = handle
            .check_grant(bob_id.clone(), "document.read".to_string(), "*".to_string())
            .await;
        assert!(has_grant, "Bob should have wildcard document.read grant");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_wildcard_revoke_succeeds() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        seed_test_editor(&event_pool, "alice").await;
        let handle = spawn_engine("test_file".to_string(), event_pool)
            .await
            .unwrap();

        // Create an editor
        let create_editor_cmd = Command::new(
            "alice".to_string(),
            "editor.create".to_string(),
            "".to_string(),
            serde_json::json!({ "name": "Bob", "editor_type": "Bot" }),
        );
        let events = handle.process_command(create_editor_cmd).await.unwrap();
        let bob_id = events[0].entity.clone();

        // Grant first
        let grant_cmd = Command::new(
            "alice".to_string(),
            "core.grant".to_string(),
            "*".to_string(),
            serde_json::json!({
                "target_editor": bob_id,
                "capability": "document.write",
                "target_block": "*"
            }),
        );
        handle.process_command(grant_cmd).await.unwrap();

        // Revoke
        let revoke_cmd = Command::new(
            "alice".to_string(),
            "core.revoke".to_string(),
            "*".to_string(),
            serde_json::json!({
                "target_editor": bob_id,
                "capability": "document.write",
                "target_block": "*"
            }),
        );
        let result = handle.process_command(revoke_cmd).await;
        assert!(
            result.is_ok(),
            "Wildcard revoke should succeed, got: {:?}",
            result.err()
        );

        let has_grant = handle
            .check_grant(
                bob_id.clone(),
                "document.write".to_string(),
                "*".to_string(),
            )
            .await;
        assert!(
            !has_grant,
            "Bob should no longer have wildcard document.write grant"
        );

        handle.shutdown().await;
    }
}

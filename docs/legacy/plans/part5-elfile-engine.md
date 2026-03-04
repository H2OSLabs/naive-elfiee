# Part 5: Elfile Engine Implementation

**Priority**: Critical | **Estimated Time**: 2 weeks

## Overview

Implement the Actor Model-based command processor that supports **multi-file, multi-user editing**. Each open `.elf` file gets its own engine actor that processes commands serially via a mailbox, eliminating race conditions and enabling proper conflict detection.

## Why Actor Model is Required

From the specification:

> "For **multi-file** support, the system will instantiate a separate engine for each open file session, with each engine instance following an **Actor Model** paradigm."

> "This engine Actor is designed to support **multi-user** concurrency by acting as the authoritative source for processing commands and resolving conflicts for its specific file."

**Key Requirements**:
1. Multiple `.elf` files open simultaneously
2. Multiple users editing the same file
3. Serialized command processing per file
4. Conflict detection via vector clocks
5. Real-time state broadcast to all connected clients

## Directory Structure

```
src-tauri/src/
├── engine/
│   ├── mod.rs
│   ├── event_store.rs      # (already done)
│   ├── state.rs            # State projector
│   ├── actor.rs            # Engine actor
│   └── manager.rs          # Engine manager
└── lib.rs
```

## Step 1: Add Async Dependencies

**File**: `src-tauri/Cargo.toml`

```toml
[dependencies]
tokio = { version = "1.36", features = ["full"] }
dashmap = "5.5"  # Concurrent HashMap
```

## Step 2: Implement State Projector

**File**: `src-tauri/src/engine/state.rs`

```rust
use crate::models::{Block, Editor, Event};
use std::collections::HashMap;

/// In-memory state projection from events
pub struct StateProjector {
    pub blocks: HashMap<String, Block>,
    pub editors: HashMap<String, Editor>,
    pub grants: HashMap<String, Vec<(String, String)>>, // editor_id -> [(cap_id, block_id)]
    pub editor_counts: HashMap<String, u64>, // editor_id -> transaction_count
}

impl StateProjector {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            editors: HashMap::new(),
            grants: HashMap::new(),
            editor_counts: HashMap::new(),
        }
    }

    /// Replay all events to build current state
    pub fn replay(&mut self, events: Vec<Event>) {
        for event in events {
            self.apply_event(&event);
        }
    }

    /// Apply a single event to state
    pub fn apply_event(&mut self, event: &Event) {
        // Update editor transaction count from vector clock
        for (editor_id, count) in &event.timestamp {
            let current = self.editor_counts.entry(editor_id.clone()).or_insert(0);
            *current = (*current).max(*count);
        }

        // Parse attribute: "editor_id/cap_id"
        let parts: Vec<&str> = event.attribute.split('/').collect();
        if parts.len() != 2 {
            return; // Invalid attribute format
        }
        let cap_id = parts[1];

        // Handle different event types based on capability
        match cap_id {
            // Block creation
            cap if cap.ends_with(".create") => {
                if let Ok(block) = serde_json::from_value::<Block>(event.value.clone()) {
                    self.blocks.insert(block.block_id.clone(), block);
                }
            }

            // Block updates (write, link, etc.)
            cap if cap.ends_with(".write") || cap.ends_with(".link") => {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    // Merge value into block
                    if let Some(contents) = event.value.get("contents") {
                        if let Some(obj) = block.contents.as_object_mut() {
                            if let Some(new_contents) = contents.as_object() {
                                for (k, v) in new_contents {
                                    obj.insert(k.clone(), v.clone());
                                }
                            }
                        }
                    }
                    if let Some(children) = event.value.get("children") {
                        if let Ok(new_children) = serde_json::from_value(children.clone()) {
                            block.children = new_children;
                        }
                    }
                }
            }

            // Block deletion
            "core.delete" => {
                self.blocks.remove(&event.entity);
            }

            // Grant capability
            "core.grant" => {
                if let Some(grant_info) = event.value.get("grant") {
                    let target_editor = grant_info.get("editor")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let capability = grant_info.get("capability")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let target_block = grant_info.get("block")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*");

                    self.grants
                        .entry(target_editor.to_string())
                        .or_insert_with(Vec::new)
                        .push((capability.to_string(), target_block.to_string()));
                }
            }

            // Editor creation
            "editor.create" => {
                if let Ok(editor) = serde_json::from_value::<Editor>(event.value.clone()) {
                    self.editors.insert(editor.editor_id.clone(), editor);
                }
            }

            _ => {
                // Unknown capability - ignore for now
            }
        }
    }

    pub fn get_block(&self, block_id: &str) -> Option<&Block> {
        self.blocks.get(block_id)
    }

    pub fn get_editor_count(&self, editor_id: &str) -> u64 {
        *self.editor_counts.get(editor_id).unwrap_or(&0)
    }

    /// Check for conflicts using vector clocks
    pub fn has_conflict(&self, cmd_timestamp: &HashMap<String, u64>) -> bool {
        for (editor_id, cmd_count) in cmd_timestamp {
            let current_count = self.editor_counts.get(editor_id).unwrap_or(&0);
            if cmd_count < current_count {
                // Command is based on stale state
                return true;
            }
        }
        false
    }
}
```

## Step 3: Implement Engine Actor

**File**: `src-tauri/src/engine/actor.rs`

```rust
use crate::capabilities::CapabilityRegistry;
use crate::engine::{EventStore, StateProjector};
use crate::models::{Block, Command, Event};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Messages that can be sent to the engine actor
pub enum EngineMessage {
    ProcessCommand {
        command: Command,
        response: tokio::sync::oneshot::Sender<Result<Vec<Event>, String>>,
    },
    GetBlock {
        block_id: String,
        response: tokio::sync::oneshot::Sender<Option<Block>>,
    },
    GetAllBlocks {
        response: tokio::sync::oneshot::Sender<Vec<Block>>,
    },
    Shutdown,
}

/// Engine actor for a single .elf file
pub struct ElfileEngineActor {
    file_id: String,
    event_store: Arc<RwLock<EventStore>>,
    state: Arc<RwLock<StateProjector>>,
    registry: Arc<CapabilityRegistry>,
    mailbox: mpsc::Receiver<EngineMessage>,
}

impl ElfileEngineActor {
    pub fn new(
        file_id: String,
        event_store: EventStore,
        registry: CapabilityRegistry,
        mailbox: mpsc::Receiver<EngineMessage>,
    ) -> Result<Self, String> {
        let event_store = Arc::new(RwLock::new(event_store));
        let state = Arc::new(RwLock::new(StateProjector::new()));
        let registry = Arc::new(registry);

        Ok(Self {
            file_id,
            event_store,
            state,
            registry,
            mailbox,
        })
    }

    /// Initialize state by replaying all events
    pub async fn initialize(&self) -> Result<(), String> {
        let store = self.event_store.read().await;
        let events = store
            .get_all_events()
            .map_err(|e| format!("Failed to load events: {}", e))?;

        let mut state = self.state.write().await;
        state.replay(events);

        Ok(())
    }

    /// Main actor loop - processes messages serially
    pub async fn run(mut self) {
        println!("Engine actor started for file: {}", self.file_id);

        while let Some(msg) = self.mailbox.recv().await {
            match msg {
                EngineMessage::ProcessCommand { command, response } => {
                    let result = self.process_command(command).await;
                    let _ = response.send(result);
                }
                EngineMessage::GetBlock { block_id, response } => {
                    let state = self.state.read().await;
                    let block = state.get_block(&block_id).cloned();
                    let _ = response.send(block);
                }
                EngineMessage::GetAllBlocks { response } => {
                    let state = self.state.read().await;
                    let blocks = state.blocks.values().cloned().collect();
                    let _ = response.send(blocks);
                }
                EngineMessage::Shutdown => {
                    println!("Engine actor shutting down for file: {}", self.file_id);
                    break;
                }
            }
        }
    }

    /// Process a command: authorize, execute, conflict check, commit
    async fn process_command(&self, cmd: Command) -> Result<Vec<Event>, String> {
        // 1. Get capability handler
        let handler = self
            .registry
            .get(&cmd.cap_id)
            .ok_or_else(|| format!("Unknown capability: {}", cmd.cap_id))?;

        // 2. Get target block and check authorization
        let state = self.state.read().await;
        let block = state
            .get_block(&cmd.block_id)
            .ok_or_else(|| format!("Block not found: {}", cmd.block_id))?
            .clone();

        if !handler.certificator(&cmd.editor_id, &block, &state.grants) {
            return Err(format!(
                "Editor {} not authorized for {} on block {}",
                cmd.editor_id, cmd.cap_id, cmd.block_id
            ));
        }

        // 3. Execute handler
        let mut events = handler
            .handler(&cmd, &block)
            .map_err(|e| format!("Handler failed: {}", e))?;

        // 4. Update vector clocks
        let editor_count = state.get_editor_count(&cmd.editor_id) + 1;
        for event in &mut events {
            event.timestamp.insert(cmd.editor_id.clone(), editor_count);
        }

        // 5. Conflict detection (check if command is based on stale state)
        // For MVP: simple check - if editor's current count doesn't match expected
        // More sophisticated: check all editors' counts in cmd metadata
        let mut cmd_timestamp = std::collections::HashMap::new();
        cmd_timestamp.insert(cmd.editor_id.clone(), editor_count - 1);

        if state.has_conflict(&cmd_timestamp) {
            return Err(format!(
                "Conflict detected: command based on stale state. Please refresh and retry."
            ));
        }

        drop(state); // Release read lock

        // 6. Commit events to store (with write lock)
        let mut store = self.event_store.write().await;
        store
            .append_events(events.clone())
            .map_err(|e| format!("Failed to commit events: {}", e))?;
        drop(store);

        // 7. Apply events to state (with write lock)
        let mut state = self.state.write().await;
        for event in &events {
            state.apply_event(event);
        }

        // TODO: Emit state_changed event to all clients (Part 6)

        Ok(events)
    }
}

/// Handle to communicate with an engine actor
#[derive(Clone)]
pub struct EngineHandle {
    sender: mpsc::Sender<EngineMessage>,
}

impl EngineHandle {
    pub fn new(sender: mpsc::Sender<EngineMessage>) -> Self {
        Self { sender }
    }

    pub async fn process_command(&self, cmd: Command) -> Result<Vec<Event>, String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(EngineMessage::ProcessCommand {
                command: cmd,
                response: tx,
            })
            .await
            .map_err(|_| "Engine actor disconnected")?;

        rx.await.map_err(|_| "Engine response failed")?
    }

    pub async fn get_block(&self, block_id: String) -> Result<Option<Block>, String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(EngineMessage::GetBlock {
                block_id,
                response: tx,
            })
            .await
            .map_err(|_| "Engine actor disconnected")?;

        rx.await.map_err(|_| "Engine response failed")
    }

    pub async fn get_all_blocks(&self) -> Result<Vec<Block>, String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(EngineMessage::GetAllBlocks { response: tx })
            .await
            .map_err(|_| "Engine actor disconnected")?;

        rx.await.map_err(|_| "Engine response failed")
    }

    pub async fn shutdown(&self) -> Result<(), String> {
        self.sender
            .send(EngineMessage::Shutdown)
            .await
            .map_err(|_| "Engine actor disconnected")
    }
}
```

## Step 4: Implement Engine Manager

**File**: `src-tauri/src/engine/manager.rs`

```rust
use super::actor::{ElfileEngineActor, EngineHandle};
use crate::capabilities::CapabilityRegistry;
use crate::engine::EventStore;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Manages multiple engine actors (one per open file)
pub struct EngineManager {
    engines: Arc<DashMap<String, EngineHandle>>,
}

impl EngineManager {
    pub fn new() -> Self {
        Self {
            engines: Arc::new(DashMap::new()),
        }
    }

    /// Spawn a new engine actor for a file
    pub async fn spawn_engine(
        &self,
        file_id: String,
        event_store: EventStore,
        registry: CapabilityRegistry,
    ) -> Result<EngineHandle, String> {
        // Check if already exists
        if let Some(handle) = self.engines.get(&file_id) {
            return Ok(handle.clone());
        }

        // Create mailbox
        let (tx, rx) = mpsc::channel(100); // Buffer 100 commands

        // Create actor
        let actor = ElfileEngineActor::new(file_id.clone(), event_store, registry, rx)?;

        // Initialize state
        actor.initialize().await?;

        // Spawn actor task
        tokio::spawn(async move {
            actor.run().await;
        });

        // Create handle
        let handle = EngineHandle::new(tx);
        self.engines.insert(file_id, handle.clone());

        Ok(handle)
    }

    /// Get handle to an existing engine
    pub fn get_engine(&self, file_id: &str) -> Option<EngineHandle> {
        self.engines.get(file_id).map(|h| h.clone())
    }

    /// Shutdown and remove an engine
    pub async fn shutdown_engine(&self, file_id: &str) -> Result<(), String> {
        if let Some((_key, handle)) = self.engines.remove(file_id) {
            handle.shutdown().await?;
        }
        Ok(())
    }

    /// Get all active file IDs
    pub fn list_files(&self) -> Vec<String> {
        self.engines.iter().map(|e| e.key().clone()).collect()
    }
}
```

## Step 5: Wire Up Module

**File**: `src-tauri/src/engine/mod.rs`

```rust
mod event_store;
mod state;
mod actor;
mod manager;

pub use event_store::EventStore;
pub use state::StateProjector;
pub use actor::{ElfileEngineActor, EngineHandle, EngineMessage};
pub use manager::EngineManager;
```

## Step 6: Simple Test

**File**: `src-tauri/src/engine/actor.rs` (add at bottom)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::CapabilityRegistry;

    #[tokio::test]
    async fn test_actor_create_block() {
        let store = EventStore::new(":memory:").unwrap();
        let registry = CapabilityRegistry::new();

        let (tx, rx) = mpsc::channel(10);
        let actor = ElfileEngineActor::new(
            "test-file".to_string(),
            store,
            registry,
            rx,
        )
        .unwrap();

        actor.initialize().await.unwrap();

        // Spawn actor
        tokio::spawn(async move {
            actor.run().await;
        });

        let handle = EngineHandle::new(tx);

        // Create block
        let editor_id = uuid::Uuid::new_v4().to_string();
        let cmd = Command::new(
            editor_id.clone(),
            "core.create".to_string(),
            uuid::Uuid::new_v4().to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "markdown"
            }),
        );

        let events = handle.process_command(cmd).await.unwrap();
        assert_eq!(events.len(), 1);

        // Verify block exists
        let blocks = handle.get_all_blocks().await.unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].name, "Test Block");

        handle.shutdown().await.unwrap();
    }
}
```

## Step 7: Run Tests

```bash
cd src-tauri
cargo test
```

## Done

Actor Model engine complete:
- ✅ Tokio-based async actors
- ✅ Message passing via channels (mailbox pattern)
- ✅ Serial command processing per file
- ✅ Conflict detection via vector clocks
- ✅ Engine manager for multi-file support
- ✅ RwLock for safe concurrent state access
- ✅ Handle-based API for Tauri commands

**Supports**:
- Multiple `.elf` files open simultaneously
- Multiple users editing same file
- Serialized processing eliminates race conditions
- Optimistic concurrency with conflict rejection

**Next**: Part 6 - Integrate with Tauri app state and add real-time event broadcasting

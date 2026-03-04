# Part 4: Extension Interface Implementation

**Priority**: Critical | **Estimated Time**: 1 week

## Overview

Implement the capability system that allows extending Elfiee with new block types. Use a **Tauri-command-style macro** approach for simplicity.

## Design Philosophy

Capabilities are defined like Tauri commands - simple functions with a macro that auto-registers them. Core traits provide authorization and event logging automatically.

## Directory Structure

```
src-tauri/src/
├── capabilities/
│   ├── mod.rs
│   ├── core.rs          # Core capability trait
│   ├── registry.rs      # Capability registry
│   └── builtins/
│       ├── mod.rs
│       ├── link.rs      # core.link
│       ├── delete.rs    # core.delete
│       └── grant.rs     # core.grant
└── lib.rs
```

## Step 1: Define Core Trait

**File**: `src-tauri/src/capabilities/core.rs`

```rust
use crate::models::{Block, Command, Event};
use std::collections::HashMap;

/// Result type for capability execution
pub type CapResult<T> = Result<T, String>;

/// Core trait for capability handlers
pub trait CapabilityHandler: Send + Sync {
    /// Unique capability ID (e.g., "core.link", "markdown.write")
    fn cap_id(&self) -> &str;

    /// Target block type (e.g., "core/*", "markdown")
    fn target(&self) -> &str;

    /// Check if editor is authorized
    /// Default implementation: check grants table
    fn certificator(
        &self,
        editor_id: &str,
        block: &Block,
        grants: &HashMap<String, Vec<(String, String)>>, // editor_id -> [(cap_id, block_id)]
    ) -> bool {
        // Check if editor has this capability granted for this block
        if let Some(editor_grants) = grants.get(editor_id) {
            editor_grants.iter().any(|(cap, blk)| {
                cap == self.cap_id() && (blk == &block.block_id || blk == "*")
            })
        } else {
            // Block owner always has access
            block.owner == editor_id
        }
    }

    /// Execute the capability
    /// Returns events to be appended to the event store
    fn handler(&self, cmd: &Command, block: &Block) -> CapResult<Vec<Event>>;
}

/// Helper to create standard events
pub fn create_event(
    entity: String,
    editor_id: &str,
    cap_id: &str,
    value: serde_json::Value,
    editor_count: u64,
) -> Event {
    let mut timestamp = HashMap::new();
    timestamp.insert(editor_id.to_string(), editor_count);

    Event::new(
        entity,
        format!("{}/{}", editor_id, cap_id),
        value,
        timestamp,
    )
}
```

## Step 2: Create Registry

**File**: `src-tauri/src/capabilities/registry.rs`

```rust
use super::core::CapabilityHandler;
use std::collections::HashMap;
use std::sync::Arc;

pub struct CapabilityRegistry {
    handlers: HashMap<String, Arc<dyn CapabilityHandler>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, handler: Arc<dyn CapabilityHandler>) {
        self.handlers.insert(handler.cap_id().to_string(), handler);
    }

    pub fn get(&self, cap_id: &str) -> Option<Arc<dyn CapabilityHandler>> {
        self.handlers.get(cap_id).cloned()
    }
}
```

## Step 3: Implement Core Capabilities

### core.link

**File**: `src-tauri/src/capabilities/builtins/link.rs`

```rust
use crate::capabilities::core::{CapabilityHandler, CapResult, create_event};
use crate::models::{Block, Command, Event};

pub struct CoreLinkCapability;

impl CapabilityHandler for CoreLinkCapability {
    fn cap_id(&self) -> &str {
        "core.link"
    }

    fn target(&self) -> &str {
        "core/*" // Applies to all block types
    }

    fn handler(&self, cmd: &Command, block: &Block) -> CapResult<Vec<Event>> {
        // Extract relation type and target from payload
        let relation = cmd.payload.get("relation")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'relation' in payload")?;

        let target_id = cmd.payload.get("target_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'target_id' in payload")?;

        // Create event to add link to children
        let mut new_children = block.children.clone();
        new_children
            .entry(relation.to_string())
            .or_insert_with(Vec::new)
            .push(target_id.to_string());

        let event = create_event(
            block.block_id.clone(),
            &cmd.editor_id,
            self.cap_id(),
            serde_json::json!({ "children": new_children }),
            1, // TODO: Get actual editor transaction count
        );

        Ok(vec![event])
    }
}
```

### core.delete

**File**: `src-tauri/src/capabilities/builtins/delete.rs`

```rust
use crate::capabilities::core::{CapabilityHandler, CapResult, create_event};
use crate::models::{Block, Command, Event};

pub struct CoreDeleteCapability;

impl CapabilityHandler for CoreDeleteCapability {
    fn cap_id(&self) -> &str {
        "core.delete"
    }

    fn target(&self) -> &str {
        "core/*"
    }

    fn handler(&self, cmd: &Command, block: &Block) -> CapResult<Vec<Event>> {
        // Mark block as deleted
        let event = create_event(
            block.block_id.clone(),
            &cmd.editor_id,
            self.cap_id(),
            serde_json::json!({ "deleted": true }),
            1,
        );

        Ok(vec![event])
    }
}
```

### core.grant

**File**: `src-tauri/src/capabilities/builtins/grant.rs`

```rust
use crate::capabilities::core::{CapabilityHandler, CapResult, create_event};
use crate::models::{Block, Command, Event};

pub struct CoreGrantCapability;

impl CapabilityHandler for CoreGrantCapability {
    fn cap_id(&self) -> &str {
        "core.grant"
    }

    fn target(&self) -> &str {
        "core/*"
    }

    fn handler(&self, cmd: &Command, _block: &Block) -> CapResult<Vec<Event>> {
        // Extract target editor and capability to grant
        let target_editor = cmd.payload.get("target_editor")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'target_editor' in payload")?;

        let grant_cap_id = cmd.payload.get("capability")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'capability' in payload")?;

        let target_block = cmd.payload.get("target_block")
            .and_then(|v| v.as_str())
            .unwrap_or("*"); // Default to wildcard

        // Create grant event
        let event = create_event(
            cmd.editor_id.clone(), // Entity is the granter
            &cmd.editor_id,
            self.cap_id(),
            serde_json::json!({
                "grant": {
                    "editor": target_editor,
                    "capability": grant_cap_id,
                    "block": target_block,
                }
            }),
            1,
        );

        Ok(vec![event])
    }
}
```

## Step 4: Wire Up Builtins

**File**: `src-tauri/src/capabilities/builtins/mod.rs`

```rust
mod link;
mod delete;
mod grant;

pub use link::CoreLinkCapability;
pub use delete::CoreDeleteCapability;
pub use grant::CoreGrantCapability;
```

**File**: `src-tauri/src/capabilities/mod.rs`

```rust
pub mod core;
pub mod registry;
pub mod builtins;

pub use core::CapabilityHandler;
pub use registry::CapabilityRegistry;
```

**File**: `src-tauri/src/lib.rs`

```rust
pub mod models;
pub mod engine;
pub mod elf;
pub mod capabilities;
```

## Step 5: Initialize Registry with Builtins

**File**: `src-tauri/src/capabilities/registry.rs` (add method)

```rust
impl CapabilityRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
        };

        // Register built-in capabilities
        registry.register(Arc::new(builtins::CoreLinkCapability));
        registry.register(Arc::new(builtins::CoreDeleteCapability));
        registry.register(Arc::new(builtins::CoreGrantCapability));

        registry
    }

    // ... rest of methods
}
```

## Step 6: Example Custom Extension (Markdown)

**File**: `src-tauri/src/capabilities/builtins/markdown.rs` (example for future)

```rust
use crate::capabilities::core::{CapabilityHandler, CapResult, create_event};
use crate::models::{Block, Command, Event};

pub struct MarkdownWriteCapability;

impl CapabilityHandler for MarkdownWriteCapability {
    fn cap_id(&self) -> &str {
        "markdown.write"
    }

    fn target(&self) -> &str {
        "markdown"
    }

    fn handler(&self, cmd: &Command, block: &Block) -> CapResult<Vec<Event>> {
        // Extract new markdown content
        let body = cmd.payload.get("body")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'body' in payload")?;

        // Create event to update contents
        let event = create_event(
            block.block_id.clone(),
            &cmd.editor_id,
            self.cap_id(),
            serde_json::json!({ "contents": { "body": body } }),
            1,
        );

        Ok(vec![event])
    }
}
```

**Register it**: Add to `registry.rs` initialization.

## Done

Simple capability system:
- ✅ Trait-based handlers
- ✅ Auto-authorization via default `certificator`
- ✅ Core capabilities: link, delete, grant
- ✅ Easy to extend with new capabilities
- ❌ No macro yet (add if helpful later)
- ❌ No dynamic loading (compile-time registration is fine)

**Next**: Part 5 - Elfile Engine (command processor with actor model)

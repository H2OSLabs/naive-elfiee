# Part 1: Core Data Models Implementation

**Priority**: Critical | **Estimated Time**: 1 week

## Overview

Implement the fundamental data structures that form the foundation of Elfiee. Keep it simple - just Rust structs with serde serialization.

## Directory Structure

```
src-tauri/src/
├── models/
│   ├── mod.rs
│   ├── block.rs
│   ├── editor.rs
│   ├── capability.rs
│   ├── command.rs
│   └── event.rs
└── lib.rs
```

## Step 1: Add Dependencies to Cargo.toml

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

## Step 2: Implement Block Model

**File**: `src-tauri/src/models/block.rs`

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: serde_json::Value,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,
}

impl Block {
    pub fn new(name: String, block_type: String, owner: String) -> Self {
        Self {
            block_id: uuid::Uuid::new_v4().to_string(),
            name,
            block_type,
            contents: serde_json::json!({}),
            children: HashMap::new(),
            owner,
        }
    }
}
```

**That's it. No complex builders, no validation yet. Just data.**

## Step 3: Implement Editor Model

**File**: `src-tauri/src/models/editor.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Editor {
    pub editor_id: String,
    pub name: String,
}

impl Editor {
    pub fn new(name: String) -> Self {
        Self {
            editor_id: uuid::Uuid::new_v4().to_string(),
            name,
        }
    }
}
```

## Step 4: Implement Capability Model

**File**: `src-tauri/src/models/capability.rs`

```rust
use serde::{Deserialize, Serialize};

/// Metadata for a capability.
/// Actual handler functions are registered separately.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub cap_id: String,
    pub name: String,
    pub target: String, // block_type this capability applies to
}
```

**Note**: We'll handle the actual `certificator` and `handler` functions in Part 4 (Extension Interface). For now, just the data.

## Step 5: Implement Command Model

**File**: `src-tauri/src/models/command.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub cmd_id: String,
    pub editor_id: String,
    pub cap_id: String,
    pub block_id: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Command {
    pub fn new(
        editor_id: String,
        cap_id: String,
        block_id: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            cmd_id: uuid::Uuid::new_v4().to_string(),
            editor_id,
            cap_id,
            block_id,
            payload,
            timestamp: chrono::Utc::now(),
        }
    }
}
```

## Step 6: Implement Event Model

**File**: `src-tauri/src/models/event.rs`

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub entity: String,
    pub attribute: String,
    pub value: serde_json::Value,
    pub timestamp: HashMap<String, u64>, // Vector clock
}

impl Event {
    pub fn new(
        entity: String,
        attribute: String,
        value: serde_json::Value,
        timestamp: HashMap<String, u64>,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            entity,
            attribute,
            value,
            timestamp,
        }
    }
}
```

## Step 7: Wire Up Module Exports

**File**: `src-tauri/src/models/mod.rs`

```rust
mod block;
mod editor;
mod capability;
mod command;
mod event;

pub use block::Block;
pub use editor::Editor;
pub use capability::Capability;
pub use command::Command;
pub use event::Event;
```

**File**: `src-tauri/src/lib.rs`

```rust
pub mod models;
```

## Step 8: Create TypeScript Type Definitions

**File**: `src/types/models.ts`

```typescript
export interface Block {
  block_id: string;
  name: string;
  block_type: string;
  contents: Record<string, any>;
  children: Record<string, string[]>;
  owner: string;
}

export interface Editor {
  editor_id: string;
  name: string;
}

export interface Capability {
  cap_id: string;
  name: string;
  target: string;
}

export interface Command {
  cmd_id: string;
  editor_id: string;
  cap_id: string;
  block_id: string;
  payload: Record<string, any>;
  timestamp: string;
}

export interface Event {
  event_id: string;
  entity: string;
  attribute: string;
  value: Record<string, any>;
  timestamp: Record<string, number>;
}
```

## Step 9: Test Compilation

```bash
cd src-tauri
cargo build
```

**Expected**: No errors. Models compile successfully.

## Done

That's it. No over-engineering:
- ✅ Simple structs with serde
- ✅ Basic constructors
- ✅ TypeScript mirrors for frontend
- ❌ No validation logic (add later if needed)
- ❌ No builder patterns (YAGNI)
- ❌ No complex trait hierarchies

**Next**: Part 2 - Event Structure (implement the event store)

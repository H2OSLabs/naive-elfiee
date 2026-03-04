# Extension Development Guide

This guide explains how to create block-type extensions for the Elfiee capability system.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Kernel vs Extension Capabilities](#kernel-vs-extension-capabilities)
3. [Creating a New Extension](#creating-a-new-extension)
4. [The capability Proc Macro](#the-capability-proc-macro)
5. [Payload Types and Validation](#payload-types-and-validation)
6. [Registration in CapabilityRegistry](#registration-in-capabilityregistry)
7. [Testing Patterns](#testing-patterns)
8. [Complete Reference: Existing Extensions](#complete-reference-existing-extensions)

---

## Architecture Overview

Elfiee uses a capability-based architecture where all state changes are captured as immutable events (EAVT schema). The capability system has two layers:

```
CapabilityRegistry
  |
  +-- Built-in (kernel) capabilities  (9 total)
  |     core.create, core.write, core.link, core.unlink, core.delete
  |     core.grant, core.revoke, editor.create, editor.delete
  |
  +-- Extension capabilities  (7 total)
        document.write, document.read
        task.write, task.read, task.commit
        session.append, session.read
```

Every capability is a struct implementing the `CapabilityHandler` trait, registered in `CapabilityRegistry` at startup. The `#[capability]` proc macro generates the trait implementation automatically.

### Key Concepts

- **Block**: The fundamental data unit with `block_id`, `name`, `block_type`, `owner`, `contents` (JSON), and `children` (relation graph).
- **Capability**: An operation that can be performed on blocks. Each has a `certificator` (authorization check) and `handler` (event production).
- **Event**: An immutable EAVT record. The `attribute` field is formatted as `{editor_id}/{cap_id}`.
- **CBAC**: Capability-Based Access Control. Authorization is checked before every handler execution: owner check first, then grant check.

---

## Kernel vs Extension Capabilities

### Kernel (Core) Capabilities

Defined in `src/capabilities/builtins/`. These operate on **all block types** (target `"core/*"`):

| Capability | Purpose | Payload Type |
|---|---|---|
| `core.create` | Create a new block | `CreateBlockPayload` |
| `core.write` | Update block name/description | `WriteBlockPayload` |
| `core.link` | Add a DAG relation | `LinkBlockPayload` |
| `core.unlink` | Remove a DAG relation | `UnlinkBlockPayload` |
| `core.delete` | Soft-delete a block | (empty) |
| `core.grant` | Grant a capability to an editor | `GrantPayload` |
| `core.revoke` | Revoke a capability from an editor | `RevokePayload` |
| `editor.create` | Create a new editor identity | `EditorCreatePayload` |
| `editor.delete` | Delete an editor identity | `EditorDeletePayload` |

Core payloads are defined in `src/models/payloads.rs`.

### Extension Capabilities

Defined in `src/extensions/{extension_name}/`. These operate on **specific block types**:

| Capability | Block Type | Purpose |
|---|---|---|
| `document.write` | `document` | Write text content |
| `document.read` | `document` | Permission gate for reading |
| `task.write` | `task` | Update structured task fields |
| `task.read` | `task` | Permission gate for reading |
| `task.commit` | `task` | Audit event for committing downstream blocks |
| `session.append` | `session` | Append an entry (command/message/decision) |
| `session.read` | `session` | Permission gate for reading |

Extension payloads are defined in the extension's own `mod.rs`.

---

## Creating a New Extension

### Step 1: Create the Directory Structure

Create a new directory under `src/extensions/`:

```
src/extensions/my_extension/
  mod.rs              # Module definition, payload types, test module
  my_write.rs         # Write capability handler
  my_read.rs          # Read capability handler (permission gate)
  tests.rs            # Comprehensive tests
```

### Step 2: Define Payload Types in mod.rs

Extension-specific payloads belong in the extension's `mod.rs`, not in `models/payloads.rs`.

```rust
// src/extensions/my_extension/mod.rs

/// My Extension
///
/// Provides capabilities for my_type blocks in Elfiee.
///
/// ## Capabilities
///
/// - `my_extension.write`: Write data to a my_type block
/// - `my_extension.read`: Read permission gate for my_type blocks

use serde::{Deserialize, Serialize};

// Module exports
pub mod my_write;
pub use my_write::*;

pub mod my_read;
pub use my_read::*;

// Payload definitions

/// Payload for my_extension.write capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyWritePayload {
    /// Required field
    pub content: String,
    /// Optional field with serde default
    #[serde(default)]
    pub metadata: Option<String>,
}

/// Payload for my_extension.read capability (permission-only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyReadPayload {}

// Tests
#[cfg(test)]
mod tests;
```

### Step 3: Implement the Write Handler

```rust
// src/extensions/my_extension/my_write.rs

use super::MyWritePayload;
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;

/// Handler for my_extension.write capability.
///
/// Writes content to a my_type block.
#[capability(id = "my_extension.write", target = "my_type")]
fn handle_my_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    // 1. Require block
    let block = block.ok_or("Block required for my_extension.write")?;

    // 2. Validate block type
    if block.block_type != "my_type" {
        return Err(format!(
            "Expected my_type block, got '{}'",
            block.block_type
        ));
    }

    // 3. Deserialize typed payload
    let payload: MyWritePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for my_extension.write: {}", e))?;

    // 4. Merge into existing contents (preserve other fields)
    let mut new_contents = if let Some(obj) = block.contents.as_object() {
        obj.clone()
    } else {
        serde_json::Map::new()
    };
    new_contents.insert("content".to_string(), serde_json::json!(payload.content));
    if let Some(meta) = &payload.metadata {
        new_contents.insert("metadata".to_string(), serde_json::json!(meta));
    }

    // 5. Create event
    let event = create_event(
        block.block_id.clone(),
        "my_extension.write",
        serde_json::json!({ "contents": new_contents }),
        &cmd.editor_id,
        1, // Placeholder -- engine actor updates with correct vector clock
    );

    Ok(vec![event])
}
```

### Step 4: Implement the Read Handler

Read capabilities are **permission gates** -- they return empty events. Actual data retrieval happens via the query layer (`get_block` / `get_all_blocks`), which is CBAC-filtered by the services layer.

```rust
// src/extensions/my_extension/my_read.rs

use crate::capabilities::core::CapResult;
use crate::models::{Block, Command, Event};
use capability_macros::capability;

/// Handler for my_extension.read capability.
///
/// Permission gate for reading my_type block contents.
/// Returns empty events -- reads are side-effect free.
#[capability(id = "my_extension.read", target = "my_type")]
fn handle_my_read(_cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for my_extension.read")?;

    if block.block_type != "my_type" {
        return Err(format!(
            "Expected my_type block, got '{}'",
            block.block_type
        ));
    }

    Ok(vec![])
}
```

### Step 5: Register the Extension

#### 5a. Add to `src/extensions/mod.rs`

```rust
pub mod document;
pub mod session;
pub mod task;
pub mod my_extension;  // Add this line
```

#### 5b. Register in `src/capabilities/registry.rs`

Add your capabilities to the `register_extensions` method:

```rust
fn register_extensions(&mut self) {
    use crate::extensions::document::*;
    use crate::extensions::session::*;
    use crate::extensions::task::*;
    use crate::extensions::my_extension::*;  // Add this

    // ... existing registrations ...

    // My Extension
    self.register(Arc::new(MyExtensionWriteCapability));
    self.register(Arc::new(MyExtensionReadCapability));
}
```

The struct names are auto-generated by the `#[capability]` macro from the capability ID:
- `my_extension.write` generates `MyExtensionWriteCapability`
- `my_extension.read` generates `MyExtensionReadCapability`

---

## The capability Proc Macro

The `#[capability]` proc macro (defined in `capability-macros/src/lib.rs`) eliminates boilerplate by generating:

1. A public struct (e.g., `MyExtensionWriteCapability`)
2. A `CapabilityHandler` trait implementation with `cap_id()`, `target()`, and `handler()` methods
3. The original handler function is preserved as-is

### Macro Syntax

```rust
#[capability(id = "namespace.action", target = "block_type")]
fn handle_something(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    // ...
}
```

### Parameters

| Parameter | Required | Description | Examples |
|---|---|---|---|
| `id` | Yes | Unique capability ID in `namespace.action` format | `"document.write"`, `"task.commit"` |
| `target` | Yes | Block type this capability applies to | `"document"`, `"task"`, `"core/*"` |

### Struct Name Derivation

The struct name is derived from the `id` by PascalCasing each dot-separated segment and appending `Capability`:

| Capability ID | Generated Struct |
|---|---|
| `core.create` | `CoreCreateCapability` |
| `document.write` | `DocumentWriteCapability` |
| `task.commit` | `TaskCommitCapability` |
| `session.append` | `SessionAppendCapability` |
| `my_extension.write` | `MyExtensionWriteCapability` |

### Handler Function Signature

The handler function **must** have this exact signature:

```rust
fn handler_name(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>>
```

- `cmd: &Command` -- Contains `editor_id`, `cap_id`, `block_id`, and `payload` (JSON).
- `block: Option<&Block>` -- The target block. `None` for create-type operations.
- Returns `CapResult<Vec<Event>>` -- Either a vector of events to append, or an error string.

### What Gets Generated

For `#[capability(id = "document.write", target = "document")]`:

```rust
pub struct DocumentWriteCapability;

impl crate::capabilities::core::CapabilityHandler for DocumentWriteCapability {
    fn cap_id(&self) -> &str { "document.write" }
    fn target(&self) -> &str { "document" }
    fn handler(
        &self,
        cmd: &crate::models::Command,
        block: Option<&crate::models::Block>,
    ) -> crate::capabilities::core::CapResult<Vec<crate::models::Event>> {
        handle_document_write(cmd, block)
    }
}
```

The default `certificator` implementation (from the `CapabilityHandler` trait) handles authorization automatically: owner always authorized, then grant-based check, then wildcard grant check.

---

## Payload Types and Validation

### Rules

1. **Every capability with input must have a typed payload struct** with `#[derive(Debug, Clone, Serialize, Deserialize)]`.
2. **Extension payloads** go in the extension's `mod.rs`.
3. **Core payloads** go in `src/models/payloads.rs`.
4. **Never use manual JSON parsing** -- always deserialize into a typed struct.

### Validation Patterns

**Required fields:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentWritePayload {
    pub content: String,  // Required -- deserialization fails if missing
}
```

**Optional fields with `#[serde(default)]`:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWritePayload {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<String>,
}
```

**Business logic validation in handler:**
```rust
// At least one field must be provided
if payload.description.is_none()
    && payload.status.is_none()
    && payload.assigned_to.is_none()
{
    return Err(
        "task.write requires at least one field (description, status, assigned_to)"
            .to_string(),
    );
}
```

**Permission-only capabilities (empty payload):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentReadPayload {}
```

### Event Creation

Use the `create_event` helper to produce correctly formatted events:

```rust
use crate::capabilities::core::create_event;

let event = create_event(
    block.block_id.clone(),  // entity: what this event is about
    "my_extension.write",    // cap_id: auto-formatted as {editor_id}/{cap_id}
    serde_json::json!({      // value: the new state
        "contents": new_contents
    }),
    &cmd.editor_id,          // editor who performed the action
    1,                       // vector clock placeholder (engine actor updates)
);
```

### Write vs Read Events

- **Write capabilities** produce events with `entity = block.block_id` and contain the updated state.
- **Read capabilities** return `Ok(vec![])` -- they are permission gates only. No events are generated.

### Append-Mode Events (Session Pattern)

For append-only semantics (like session entries), use `Event::new_with_mode` with `EventMode::Append`:

```rust
use crate::models::{Event, EventMode};

let event = Event::new_with_mode(
    block.block_id.clone(),
    format!("{}/{}", cmd.editor_id, "session.append"),
    serde_json::json!({ "entry": entry }),
    timestamp,
    EventMode::Append,  // StateProjector pushes entry to contents.entries
);
```

---

## Registration in CapabilityRegistry

The `CapabilityRegistry` (at `src/capabilities/registry.rs`) manages all capability handlers. Registration happens in two methods:

- `register_builtins()` -- 9 kernel capabilities
- `register_extensions()` -- Extension capabilities

To add a new extension:

```rust
fn register_extensions(&mut self) {
    use crate::extensions::document::*;
    use crate::extensions::session::*;
    use crate::extensions::task::*;
    use crate::extensions::my_extension::*;

    // Document extension (2)
    self.register(Arc::new(DocumentWriteCapability));
    self.register(Arc::new(DocumentReadCapability));

    // Task extension (3)
    self.register(Arc::new(TaskWriteCapability));
    self.register(Arc::new(TaskReadCapability));
    self.register(Arc::new(TaskCommitCapability));

    // Session extension (2)
    self.register(Arc::new(SessionAppendCapability));
    self.register(Arc::new(SessionReadCapability));

    // My extension (2)
    self.register(Arc::new(MyExtensionWriteCapability));
    self.register(Arc::new(MyExtensionReadCapability));
}
```

### Capability Discovery

The registry provides lookup methods:

```rust
let registry = CapabilityRegistry::new();

// Look up by ID
let cap = registry.get("document.write").unwrap();
assert_eq!(cap.cap_id(), "document.write");
assert_eq!(cap.target(), "document");

// Get all grantable capability IDs (excluding owner-only)
let grantable = registry.get_grantable_cap_ids(&["core.grant", "core.revoke"]);
```

---

## Testing Patterns

### 1. Payload Deserialization Tests

Test that payloads serialize/deserialize correctly:

```rust
#[test]
fn test_payload_deserialization() {
    let json = serde_json::json!({ "content": "hello" });
    let payload: MyWritePayload = serde_json::from_value(json).unwrap();
    assert_eq!(payload.content, "hello");
    assert!(payload.metadata.is_none());
}

#[test]
fn test_empty_read_payload() {
    let json = serde_json::json!({});
    let payload: Result<MyReadPayload, _> = serde_json::from_value(json);
    assert!(payload.is_ok());
}
```

### 2. Handler Functionality Tests

Test via the `CapabilityRegistry` to verify correct registration:

```rust
#[test]
fn test_write_basic() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("my_extension.write")
        .expect("my_extension.write should be registered");

    let block = Block::new(
        "Test Block".to_string(),
        "my_type".to_string(),
        "alice".to_string(),
    );

    let cmd = Command::new(
        "alice".to_string(),
        "my_extension.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({ "content": "hello" }),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_ok());

    let events = result.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity, block.block_id);
    assert_eq!(events[0].attribute, "alice/my_extension.write");
    assert_eq!(events[0].value["contents"]["content"], "hello");
}
```

### 3. Block Type Validation Tests

```rust
#[test]
fn test_write_wrong_block_type() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("my_extension.write").unwrap();

    let block = Block::new("Doc".to_string(), "document".to_string(), "alice".to_string());

    let cmd = Command::new(
        "alice".to_string(),
        "my_extension.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({ "content": "hello" }),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Expected my_type block"));
}
```

### 4. Missing Block Tests

```rust
#[test]
fn test_write_no_block_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("my_extension.write").unwrap();

    let cmd = Command::new(
        "alice".to_string(),
        "my_extension.write".to_string(),
        "nonexistent".to_string(),
        serde_json::json!({ "content": "hello" }),
    );

    let result = cap.handler(&cmd, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Block required"));
}
```

### 5. State Preservation Tests

Verify that write operations preserve existing fields in `contents`:

```rust
#[test]
fn test_write_preserves_existing_fields() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("my_extension.write").unwrap();

    let mut block = Block::new("Test".to_string(), "my_type".to_string(), "alice".to_string());
    block.contents = serde_json::json!({
        "existing_field": "preserved",
        "content": "old content"
    });

    let cmd = Command::new(
        "alice".to_string(),
        "my_extension.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({ "content": "new content" }),
    );

    let events = cap.handler(&cmd, Some(&block)).unwrap();
    let new_contents = &events[0].value["contents"];

    assert_eq!(new_contents["content"], "new content");
    assert_eq!(new_contents["existing_field"], "preserved");
}
```

### 6. Authorization Tests

Test CBAC behavior (handler does not check auth directly -- that is the certificator's job):

```rust
#[test]
fn test_authorization_owner() {
    let grants_table = GrantsTable::new();
    let block = Block::new("Test".to_string(), "my_type".to_string(), "alice".to_string());

    let is_authorized = block.owner == "alice"
        || grants_table.has_grant("alice", "my_extension.write", &block.block_id);
    assert!(is_authorized, "Owner should always be authorized");
}

#[test]
fn test_authorization_non_owner_without_grant() {
    let grants_table = GrantsTable::new();
    let block = Block::new("Test".to_string(), "my_type".to_string(), "alice".to_string());

    let is_authorized = block.owner == "bob"
        || grants_table.has_grant("bob", "my_extension.write", &block.block_id);
    assert!(!is_authorized);
}

#[test]
fn test_authorization_non_owner_with_grant() {
    let mut grants_table = GrantsTable::new();
    let block = Block::new("Test".to_string(), "my_type".to_string(), "alice".to_string());

    grants_table.add_grant(
        "bob".to_string(),
        "my_extension.write".to_string(),
        block.block_id.clone(),
    );

    let is_authorized = block.owner == "bob"
        || grants_table.has_grant("bob", "my_extension.write", &block.block_id);
    assert!(is_authorized);
}
```

### Running Tests

```bash
cd /path/to/elfiee && cargo test
```

---

## Complete Reference: Existing Extensions

### Document Extension (`src/extensions/document/`)

- **Block type**: `document`
- **Capabilities**: `document.write`, `document.read`
- **Contents schema**: `{ "format": "md", "content": "# Hello", "path": "src/auth.rs" }`
- **Write payload**: `{ "content": "..." }` (required)
- **Read**: Permission-only gate, returns empty events

### Task Extension (`src/extensions/task/`)

- **Block type**: `task`
- **Capabilities**: `task.write`, `task.read`, `task.commit`
- **Contents schema**: `{ "description": "...", "status": "pending", "assigned_to": "...", "template": "..." }`
- **Write payload**: All fields optional, but at least one required
- **Read**: Permission-only gate
- **Commit**: Validates that `implement` relation has downstream blocks, produces audit event

### Session Extension (`src/extensions/session/`)

- **Block type**: `session`
- **Capabilities**: `session.append`, `session.read`
- **Contents schema**: `{ "entries": [...] }`
- **Append payload**: `{ "entry_type": "command|message|decision", "data": {...} }`
- **Uses `EventMode::Append`** -- each event adds one entry to the list
- **Read**: Permission-only gate

---

## Best Practices

1. **Naming**: Extension = lowercase with underscores (`my_extension`). Capability IDs = `extension.action`. Struct names are auto-generated.
2. **Type safety**: Always use typed payload structs. Never parse JSON manually.
3. **State preservation**: When updating `contents`, clone existing fields and merge. Never overwrite unrelated fields.
4. **Error messages**: Return descriptive error strings. Include the expected vs actual block type, missing fields, etc.
5. **Read capabilities**: Always implement as permission-only gates returning empty events. Actual data flows through the query layer.
6. **Testing**: Cover payload deserialization, basic functionality, wrong block type, missing block, state preservation, and authorization.

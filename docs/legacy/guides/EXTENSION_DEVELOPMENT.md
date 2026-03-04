# Extension Development Guide

This guide explains how to create custom block types and capabilities for the Elfiee system.

## Table of Contents

1. [Overview](#overview)
2. [Creating a New Extension](#creating-a-new-extension)
3. [Defining Capabilities](#defining-capabilities)
4. [Registering Your Extension](#registering-your-extension)
5. [Testing Your Extension](#testing-your-extension)
6. [Authorization and CBAC](#authorization-and-cbac)
7. [Best Practices](#best-practices)
8. [Complete Example: Markdown Extension](#complete-example-markdown-extension)

## Overview

Elfiee uses a capability-based architecture where:
- **Blocks** are the fundamental data units (like documents, notes, tasks)
- **Capabilities** are operations that can be performed on blocks
- **Extensions** provide domain-specific capabilities for custom block types

The system uses event sourcing with EAVT (Entity-Attribute-Value-Timestamp) schema, where all changes are recorded as immutable events.

## Creating a New Extension

### Step 1: Create Extension Directory Structure

Create a new directory under `src/extensions/` for your extension:

```bash
mkdir -p src/extensions/my_extension
```

Your extension should contain:
- `mod.rs` - Module definition and documentation
- Individual capability files (e.g., `my_capability.rs`)

### Step 2: Define Your Extension Module

Create `src/extensions/my_extension/mod.rs`:

```rust
/// My Extension for Elfiee.
///
/// Description of what your extension does.
///
/// ## Capabilities
///
/// - `my_extension.capability1`: Description
/// - `my_extension.capability2`: Description

pub mod my_capability;

// Re-export capabilities for registration
pub use my_capability::*;

#[cfg(test)]
mod tests {
    // Your tests here
}
```

## Defining Capabilities

### Basic Capability Structure

Use the `#[capability]` macro to define a capability handler:

```rust
use crate::capabilities::core::{CapResult, create_event};
use crate::models::{Block, Command, Event};
use capability_macros::capability;

/// Handler for my_extension.do_something capability.
///
/// Detailed description of what this capability does.
#[capability(id = "my_extension.do_something", target = "my_block_type")]
fn handle_do_something(cmd: &Command, block: &Block) -> CapResult<Vec<Event>> {
    // 1. Extract parameters from cmd.payload
    let param = cmd.payload.get("param")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'param' in payload")?;

    // 2. Perform validation
    if param.is_empty() {
        return Err("Parameter cannot be empty".into());
    }

    // 3. Compute new state
    let mut new_contents = if let Some(obj) = block.contents.as_object() {
        obj.clone()
    } else {
        serde_json::Map::new()
    };
    new_contents.insert("my_field".to_string(), serde_json::json!(param));

    // 4. Create event
    let event = create_event(
        block.block_id.clone(),        // Entity (usually block_id)
        "my_extension.do_something",   // Capability ID
        serde_json::json!({ "contents": new_contents }),  // New state
        &cmd.editor_id,                // Who performed the action
        1,                             // TODO: Use actual vector clock
    );

    // 5. Return event(s)
    Ok(vec![event])
}
```

### Capability Macro Parameters

- `id`: Unique capability identifier (format: `extension_name.capability_name`)
- `target`: Block type this capability applies to (use `"*"` for all types)

### Event Creation Guidelines

The `create_event` helper automatically formats the event attribute as `{editor_id}/{cap_id}`:

```rust
let event = create_event(
    entity,        // String - What this event is about (usually block_id)
    cap_id,        // &str - Capability ID
    value,         // serde_json::Value - The new state or data
    editor_id,     // &str - Who performed this action
    editor_count,  // u64 - Vector clock count (TODO: implement properly)
);
```

### Read vs Write Capabilities

**Write Capabilities** (modify block state):
```rust
// Entity is the block being modified
let event = create_event(
    block.block_id.clone(),
    "my_extension.write",
    serde_json::json!({ "contents": new_contents }),
    &cmd.editor_id,
    1,
);
```

**Read Capabilities** (observe without modifying):
```rust
// Entity is the reader (cmd.editor_id)
let event = create_event(
    cmd.editor_id.clone(),     // Note: entity is the reader
    "my_extension.read",
    serde_json::json!({
        "block_id": block.block_id,
        "data": extracted_data
    }),
    &cmd.editor_id,
    1,
);
```

## Registering Your Extension

### Step 1: Add Extension to `src/extensions/mod.rs`

```rust
pub mod markdown;
pub mod my_extension;  // Add your extension
```

### Step 2: Register Capabilities in Registry

Edit `src/capabilities/registry.rs` to include your extension:

```rust
/// Register all extension capabilities.
fn register_extensions(&mut self) {
    use crate::extensions::markdown::*;
    use crate::extensions::my_extension::*;  // Add this

    // Existing registrations
    self.register(Arc::new(MarkdownWriteCapability));
    self.register(Arc::new(MarkdownReadCapability));

    // Register your capabilities
    self.register(Arc::new(MyCapability1));
    self.register(Arc::new(MyCapability2));
}
```

### Step 3: Update `src/lib.rs` (if needed)

The `extensions` module should already be exported in `lib.rs`:

```rust
pub mod extensions;
```

## Testing Your Extension

### Unit Tests

Add tests to your extension's `mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::capabilities::CapabilityRegistry;
    use crate::models::{Block, Command};

    #[test]
    fn test_my_capability_basic() {
        let registry = CapabilityRegistry::new();
        let cap = registry.get("my_extension.do_something")
            .expect("capability should be registered");

        let block = Block::new(
            "Test Block".to_string(),
            "my_block_type".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "alice".to_string(),
            "my_extension.do_something".to_string(),
            block.block_id.clone(),
            serde_json::json!({ "param": "test_value" }),
        );

        let events = cap.handler(&cmd, &block).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].attribute, "alice/my_extension.do_something");

        // Verify the event contains expected data
        let value = &events[0].value;
        // Add your assertions here
    }

    #[test]
    fn test_my_capability_validation() {
        let registry = CapabilityRegistry::new();
        let cap = registry.get("my_extension.do_something").unwrap();

        let block = Block::new(
            "Test Block".to_string(),
            "my_block_type".to_string(),
            "alice".to_string(),
        );

        // Test with invalid input
        let cmd = Command::new(
            "alice".to_string(),
            "my_extension.do_something".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),  // Missing required parameter
        );

        let result = cap.handler(&cmd, &block);
        assert!(result.is_err());
    }
}
```

### Run Tests

```bash
cargo test
```

## Authorization and CBAC

Elfiee uses Capability-Based Access Control (CBAC) with a `GrantsTable`:

### Authorization Model

1. **Owner Always Authorized**: Block owners can perform any capability on their blocks
2. **Grant-Based Authorization**: Non-owners need explicit grants
3. **Wildcard Grants**: `"*"` block_id grants permission on all blocks

### Implementing Authorization Checks

While capability handlers don't directly check authorization (separation of concerns), you should test authorization logic:

```rust
#[test]
fn test_authorization_owner() {
    use crate::capabilities::grants::GrantsTable;

    let grants_table = GrantsTable::new();
    let block = Block::new(
        "Test".to_string(),
        "my_type".to_string(),
        "alice".to_string(),
    );

    let cmd = Command::new(
        "alice".to_string(),
        "my_extension.do_something".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    // Check authorization
    let is_authorized = block.owner == cmd.editor_id
        || grants_table.has_grant(
            &cmd.editor_id,
            "my_extension.do_something",
            &block.block_id
        );

    assert!(is_authorized, "Owner should be authorized");
}

#[test]
fn test_authorization_non_owner_with_grant() {
    use crate::capabilities::grants::GrantsTable;

    let mut grants_table = GrantsTable::new();
    let block = Block::new(
        "Test".to_string(),
        "my_type".to_string(),
        "alice".to_string(),
    );

    // Grant permission to bob
    grants_table.add_grant(
        "bob".to_string(),
        "my_extension.do_something".to_string(),
        block.block_id.clone(),
    );

    let cmd = Command::new(
        "bob".to_string(),
        "my_extension.do_something".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let is_authorized = block.owner == cmd.editor_id
        || grants_table.has_grant(
            &cmd.editor_id,
            "my_extension.do_something",
            &block.block_id
        );

    assert!(is_authorized, "User with grant should be authorized");
}
```

### Grant Management

Grants are managed through the `core.grant` and `core.revoke` capabilities:

```rust
// Grant capability to a user
let grant_cmd = Command::new(
    "alice".to_string(),  // Granter (must be owner)
    "core.grant".to_string(),
    block.block_id.clone(),
    serde_json::json!({
        "target_editor": "bob",
        "capability": "my_extension.do_something",
        "target_block": block.block_id,  // Or "*" for wildcard
    }),
);

// Revoke capability from a user
let revoke_cmd = Command::new(
    "alice".to_string(),
    "core.revoke".to_string(),
    block.block_id.clone(),
    serde_json::json!({
        "target_editor": "bob",
        "capability": "my_extension.do_something",
        "target_block": block.block_id,
    }),
);
```

## Best Practices

### 1. Naming Conventions

- Extension names: lowercase with underscores (`my_extension`)
- Capability IDs: `extension_name.capability_name`
- Capability struct names: PascalCase ending with `Capability`

### 2. Error Handling

Return descriptive error messages:

```rust
// Good
return Err("Missing 'content' in payload".into());

// Bad
return Err("Error".into());
```

### 3. Preserve Existing State

When updating block contents, preserve fields you don't modify:

```rust
let mut new_contents = if let Some(obj) = block.contents.as_object() {
    obj.clone()  // Start with existing contents
} else {
    serde_json::Map::new()
};
new_contents.insert("my_field".to_string(), serde_json::json!(new_value));
// Other fields are preserved
```

### 4. Documentation

- Document what each capability does
- Explain required payload fields
- Describe the event structure created
- Provide usage examples

### 5. Testing

Write tests for:
- Basic functionality
- Input validation
- Error cases
- Authorization scenarios
- State preservation

## Complete Example: Markdown Extension

The Markdown extension is a complete reference implementation. Here's its structure:

```
src/extensions/markdown/
├── mod.rs              # Module definition and tests
├── markdown_write.rs   # Write markdown content
└── markdown_read.rs    # Read markdown content
```

### markdown_write.rs

```rust
use crate::capabilities::core::{CapResult, create_event};
use crate::models::{Block, Command, Event};
use capability_macros::capability;

#[capability(id = "markdown.write", target = "markdown")]
fn handle_markdown_write(cmd: &Command, block: &Block) -> CapResult<Vec<Event>> {
    let markdown_content = cmd.payload.get("content")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'content' in payload")?;

    let mut new_contents = if let Some(obj) = block.contents.as_object() {
        obj.clone()
    } else {
        serde_json::Map::new()
    };
    new_contents.insert("markdown".to_string(), serde_json::json!(markdown_content));

    let event = create_event(
        block.block_id.clone(),
        "markdown.write",
        serde_json::json!({ "contents": new_contents }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

### markdown_read.rs

```rust
use crate::capabilities::core::{CapResult, create_event};
use crate::models::{Block, Command, Event};
use capability_macros::capability;

#[capability(id = "markdown.read", target = "markdown")]
fn handle_markdown_read(cmd: &Command, block: &Block) -> CapResult<Vec<Event>> {
    let markdown_content = block.contents.get("markdown")
        .ok_or("No markdown content found in block")?;

    let event = create_event(
        cmd.editor_id.clone(),  // Entity is reader for read operations
        "markdown.read",
        serde_json::json!({
            "block_id": block.block_id,
            "content": markdown_content
        }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

### Usage Example

```rust
use elfiee::models::{Block, Command};
use elfiee::capabilities::CapabilityRegistry;

// Create a markdown block
let mut block = Block::new(
    "My Document".to_string(),
    "markdown".to_string(),
    "alice".to_string(),
);

let registry = CapabilityRegistry::new();

// Write markdown
let write_cmd = Command::new(
    "alice".to_string(),
    "markdown.write".to_string(),
    block.block_id.clone(),
    serde_json::json!({
        "content": "# Hello World\n\nThis is markdown."
    }),
);

let cap = registry.get("markdown.write").unwrap();
let events = cap.handler(&write_cmd, &block).unwrap();

// Apply the event to update block state (in a real system)
// block.apply_events(&events);

// Read markdown
let read_cmd = Command::new(
    "alice".to_string(),
    "markdown.read".to_string(),
    block.block_id.clone(),
    serde_json::json!({}),
);

let cap = registry.get("markdown.read").unwrap();
let read_events = cap.handler(&read_cmd, &block).unwrap();
```

## Next Steps

1. Study the Markdown extension implementation in `src/extensions/markdown/`
2. Review the core capabilities in `src/capabilities/builtins/`
3. Create your own extension following this guide
4. Add comprehensive tests
5. Consider contributing your extension back to the project

For more information, see:
- [README.md](../../README.md) - Project overview and architecture
- [Part 4: Extension Interface](../plans/part4-extension-interface.md) - Technical specification
- [Part 7: Content Schema Proposal](../plans/part7-content-schema-proposal.md) - Future schema design
- [Capability Macros](../../src-tauri/capability-macros/) - Macro implementation details

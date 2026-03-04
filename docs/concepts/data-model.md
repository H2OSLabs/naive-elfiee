# Data Model

Elfiee's data model consists of 6 core entities that work together to provide event-sourced, capability-controlled block management.

## Entity Overview

```text
Editor ---sends---> Command ---dispatched to---> Capability
                                                     |
                                                 produces
                                                     |
                                                     v
                    Block <---projected from---   Event
                      ^                              |
                      |                          stored in
                      +--- Grant (CBAC) ---      EventStore
```

## Block

The fundamental content unit. Every piece of managed content is a block.

```rust
struct Block {
    block_id: String,          // UUID
    name: String,              // Display name
    block_type: String,        // "document" | "task" | "session"
    contents: serde_json::Value, // Type-specific JSON payload
    children: HashMap<String, Vec<String>>, // DAG relations
    owner: String,             // Creator's editor_id
    metadata: BlockMetadata,   // Timestamps + custom fields
}
```

### Three Block Types

| Type | Purpose | Contents Schema |
|------|---------|----------------|
| `document` | Markdown documents, notes | `{ "markdown": "..." }` |
| `task` | Units of work with commit semantics | `{ "markdown": "..." }` |
| `session` | Append-only conversation logs | `{ "entries": [...] }` |

### Block DAG

Blocks form a directed acyclic graph via the `children` field. The only allowed relation type is `implement`:

```text
A.children["implement"] = [B, C]
```

This means A's changes cause B and C to need changes. Examples: PRD -> Task -> Code, Design -> Implementation.

The engine maintains a reverse index (`parents`) for efficient upward traversal.

## Event

An immutable record of a state change, following the **EAVT** (Entity-Attribute-Value-Timestamp) model.

```rust
struct Event {
    event_id: String,                    // UUID
    entity: String,                      // What changed (block_id or editor_id)
    attribute: String,                   // "{editor_id}/{cap_id}"
    value: serde_json::Value,            // New state or delta
    timestamp: HashMap<String, i64>,     // Vector clock
    created_at: String,                  // Wall clock (ISO 8601)
}
```

See [event-system.md](event-system.md) for content modes and conflict handling.

## Editor

A user or agent identity. Human and bot editors are architecturally equal (Socialware principle).

```rust
struct Editor {
    editor_id: String,     // UUID
    name: String,          // Display name
    editor_type: EditorType, // Human | Bot
}
```

Both types interact through the same MCP tools and are subject to the same CBAC rules.

## Command

An intent sent by an editor to perform an operation.

```rust
struct Command {
    editor_id: String,            // Who is requesting
    cap_id: String,               // Which capability to invoke
    block_id: String,             // Target block (or empty for create)
    payload: serde_json::Value,   // Typed input data
}
```

Commands are routed by `cap_id` to the matching `Capability` in the registry.

## Grant

A CBAC permission entry mapping an editor to a capability on a block.

```rust
struct Grant {
    editor_id: String,  // Who has the permission
    cap_id: String,     // What they can do (e.g., "document.write")
    block_id: String,   // Which block ("*" for wildcard)
}
```

Grants are themselves stored as events (`core.grant` / `core.revoke`), making permissions fully auditable.

## Capability

Defines an operation with two pure functions:

| Function | Purpose |
|----------|---------|
| `certificator` | Checks authorization: owner check -> grants table check |
| `handler` | Executes logic, returns `Vec<Event>` (no I/O) |

```rust
trait CapabilityHandler: Send + Sync {
    fn cap_id(&self) -> &str;
    fn target(&self) -> &str;
    fn certificator(&self, editor_id: &str, block: &Block, grants: &...) -> bool;
    fn handler(&self, cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>>;
}
```

Capabilities are registered in the `CapabilityRegistry` at engine startup. The engine looks up capabilities by `cap_id` when processing commands.

## Typed Payloads

Every capability that accepts input has a corresponding Rust struct:

| Payload | Capability |
|---------|-----------|
| `CreateBlockPayload` | `core.create` |
| `RenamePayload` | `core.rename` |
| `LinkBlockPayload` | `core.link` |
| `UnlinkBlockPayload` | `core.unlink` |
| `GrantPayload` | `core.grant` |
| `RevokePayload` | `core.revoke` |
| `EditorCreatePayload` | `editor.create` |
| `EditorDeletePayload` | `editor.delete` |
| `TaskWritePayload` | `task.write` |
| `TaskCommitPayload` | `task.commit` |

Extension payloads are defined in their own modules; core payloads live in `models/payloads.rs`.

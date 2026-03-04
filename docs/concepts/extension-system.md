# Extension System

Elfiee separates its core kernel from domain-specific functionality through an extension system. The kernel handles event sourcing, CBAC, and the actor model. Extensions add support for specific block types.

## Kernel vs Extensions

```text
+------------------------------------------+
|              Extensions                   |
|  +----------+ +--------+ +-----------+   |
|  | Document | | Task   | | Session   |   |
|  +----------+ +--------+ +-----------+   |
+------------------------------------------+
|                Kernel                     |
|  +--------+ +------+ +-----+ +-------+  |
|  |  ES    | | CBAC | | DAG | | Actor |  |
|  +--------+ +------+ +-----+ +-------+  |
+------------------------------------------+
```

### Kernel Responsibilities

| Component | What It Does |
|-----------|-------------|
| Event Sourcing (ES) | Event storage, replay, vector clocks |
| CBAC | Authorization checks, grants table projection |
| Block DAG | `implement` relations, parent/child index |
| Actor Model | Serial command processing, EngineManager |

### Extension Responsibilities

| Component | What It Does |
|-----------|-------------|
| Schema | Define `contents` JSON structure for a block type |
| Capabilities | Implement `CapabilityHandler` for read/write operations |
| Payloads | Define typed Rust structs for command input |

## Built-in Extensions

Elfiee ships with 3 built-in extensions, one per block type:

### Document Extension

Provides read/write for markdown document blocks.

| Capability | Description |
|-----------|-------------|
| `document.write` | Write markdown content to a document block |
| `document.read` | Read document content (permission gate + audit) |

**Contents schema:**
```json
{ "markdown": "# Title\n\nContent here..." }
```

### Task Extension

Manages task blocks with commit semantics for bridging to Git.

| Capability | Description |
|-----------|-------------|
| `task.write` | Write markdown content to a task block |
| `task.read` | Read task content (permission gate + audit) |
| `task.commit` | Record commit intent, triggering Git operations |

**Contents schema:**
```json
{ "markdown": "# Implement login\n\nRequirements..." }
```

Task status is implicit from event history:
- No `task.commit` event = Pending
- Has downstream blocks via `implement` = In Progress
- Has `task.commit` event = Committed

### Session Extension

Append-only conversation logs for agent-human interactions.

| Capability | Description |
|-----------|-------------|
| `session.append` | Append an entry to the session log |
| `session.read` | Read session entries (permission gate + audit) |

**Contents schema:**
```json
{
  "entries": [
    { "role": "user", "content": "Review this code", "timestamp": "..." },
    { "role": "assistant", "content": "I found 3 issues...", "timestamp": "..." }
  ]
}
```

## Creating an Extension

An extension is a Rust module under `src/extensions/` that provides:

### 1. Module Structure

```text
src/extensions/my_extension/
  mod.rs               # Module definition, payload types, tests
  my_extension_write.rs  # Write capability handler
  my_extension_read.rs   # Read capability handler
```

### 2. Payload Definition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyWritePayload {
    pub content: String,
}
```

### 3. Capability Handler

```rust
#[capability(id = "my_ext.write", target = "my_type")]
fn handle_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required")?;
    let payload: MyWritePayload = serde_json::from_value(cmd.payload.clone())?;

    // Pure computation -- NO I/O allowed
    let event = create_event(
        block.block_id.clone(),
        "my_ext.write",
        serde_json::json!({ "contents": { "data": payload.content } }),
        &cmd.editor_id,
        1,
    );
    Ok(vec![event])
}
```

### 4. Registration

Register capabilities in `CapabilityRegistry::register_extensions()`:

```rust
fn register_extensions(&mut self) {
    use crate::extensions::my_extension::*;
    self.register(Arc::new(MyExtWriteCapability));
    self.register(Arc::new(MyExtReadCapability));
}
```

## Handler Constraints

Capability handlers must be **pure functions**:

- **No I/O**: No file reads, no network calls, no database access.
- **No side effects**: Only produce events from the input command and block state.
- **Deterministic**: Same input always produces the same output.

This ensures that:
1. Handlers are trivially testable.
2. Event replay is deterministic.
3. The engine remains the single point of I/O (SQLite writes).

If an operation needs I/O (e.g., exporting files, running Git), the I/O is performed by the service layer or CLI layer *after* the capability handler produces its event. This is called the **Split Pattern**: the handler produces an audit event, and the caller performs the I/O.

## Extension vs Core Capabilities

| Aspect | Core Capabilities | Extension Capabilities |
|--------|------------------|----------------------|
| Scope | All block types | Specific block type |
| Examples | `core.create`, `core.link`, `core.grant` | `document.write`, `task.commit` |
| Location | `src/capabilities/builtins/` | `src/extensions/{name}/` |
| Target | `"*"` (any block type) | Specific type (e.g., `"task"`) |

# Engine Architecture

The engine is the core of Elfiee, responsible for command processing, event persistence, and state management. It uses the Actor model to handle concurrency without locks.

## Actor Model

Each `.elf` project gets a dedicated `EngineActor` running in its own tokio task. Commands are sent to the actor's mailbox (mpsc channel) and processed serially, eliminating data races.

```text
  MCP Client 1 ---+
                   |
  MCP Client 2 ---+--> EngineHandle --[mpsc]--> EngineActor (project-a)
                   |                              |
  CLI command  ---+                               +-- StateProjector
                                                  +-- EventStore
                                                  +-- CapabilityRegistry

  MCP Client 3 -------> EngineHandle --[mpsc]--> EngineActor (project-b)
```

### Key Properties

- **Isolation**: Each project's actor is independent. Project A's commands never block Project B.
- **Serial processing**: Within a project, commands execute one at a time. No concurrent state mutation.
- **Lock-free**: Concurrency between projects is achieved through message passing, not mutexes.

## Command Processing Pipeline

When a command arrives, the actor executes 7 steps:

```text
1. RECEIVE    Command from mailbox (mpsc channel)
      |
2. LOOKUP     Capability from CapabilityRegistry by cap_id
      |
3. AUTHORIZE  Call capability.certificator(editor, block, grants)
      |       Reject if unauthorized
4. EXECUTE    Call capability.handler(cmd, block)
      |       Produces Vec<Event> (pure function, no I/O)
5. CONFLICT   Compare command's vector clock with state's clock
      |       Reject if stale (OCC)
6. COMMIT     Atomic append events to eventstore.db
      |
7. PROJECT    Apply events to StateProjector (in-memory state)
      |
      +----> Broadcast events via tokio broadcast channel
```

### Step Details

**Step 2 -- Lookup**: The `CapabilityRegistry` maps `cap_id` strings to `CapabilityHandler` trait objects. If the capability is not found, the command is rejected.

**Step 3 -- Authorize**: The default certificator checks owner status first, then the GrantsTable. Custom capabilities can override this logic.

**Step 4 -- Execute**: Handlers are pure functions. They receive the command and current block state, and return events describing what changed. No database writes, no file I/O.

**Step 5 -- Conflict**: The engine compares vector clocks. If another editor committed events after the command was issued, the command is rejected to prevent lost updates.

**Step 6 -- Commit**: Events are written to SQLite atomically. If the write fails, the command fails and state remains unchanged.

**Step 7 -- Project**: Events are applied to the in-memory `StateProjector`, updating blocks, editors, grants, and vector clocks.

## EngineManager

The `EngineManager` manages multiple actors (one per project) using a `DashMap` for thread-safe concurrent access:

```rust
struct EngineManager {
    engines: Arc<DashMap<String, EngineHandle>>,
}
```

Operations:

| Method | Purpose |
|--------|---------|
| `spawn_engine(file_id, pool)` | Start a new actor for a project |
| `get_engine(file_id)` | Get handle to an existing actor |
| `shutdown_engine(file_id)` | Stop an actor and remove it |
| `shutdown_all()` | Stop all actors (app shutdown) |

## EngineHandle

The `EngineHandle` wraps the mpsc sender with typed async methods:

| Method | Returns |
|--------|---------|
| `process_command(cmd)` | `Result<Vec<Event>, String>` |
| `get_block(block_id)` | `Option<Block>` |
| `get_all_blocks()` | `HashMap<String, Block>` |
| `get_all_editors()` | `HashMap<String, Editor>` |
| `get_all_grants()` | `HashMap<String, Vec<(String, String)>>` |
| `get_editor_grants(editor_id)` | `Vec<(String, String)>` |
| `check_grant(editor, cap, block)` | `bool` |
| `get_all_events()` | `Result<Vec<Event>, String>` |
| `shutdown()` | `()` |

Each method sends an `EngineMessage` to the actor's mailbox and awaits the response via a oneshot channel.

## StateProjector

The `StateProjector` maintains the current in-memory state by replaying events:

```rust
struct StateProjector {
    blocks: HashMap<String, Block>,
    editors: HashMap<String, Editor>,
    grants: GrantsTable,
    editor_counts: HashMap<String, i64>,  // Vector clocks
    parents: HashMap<String, Vec<String>>, // Reverse DAG index
    system_editor_id: Option<String>,
}
```

### Event Application

The `apply_event` method dispatches based on the capability ID extracted from the attribute:

- `core.create` -- Insert new block
- `core.delete` -- Remove block and clean up DAG
- `core.link` / `core.unlink` -- Update children + parents index
- `core.rename` -- Update block name
- `core.grant` / `core.revoke` -- Update GrantsTable
- `editor.create` / `editor.delete` -- Update editors map
- `document.write`, `task.write` -- Update block contents
- `session.append` -- Append to entries list

### Startup Replay

On engine startup:

1. Load all events from `eventstore.db`.
2. Optionally load cached state from `cache.db`.
3. Replay events (or only events after cache point) through `StateProjector`.
4. Engine is ready to process commands.

## CacheStore

The `CacheStore` (`cache.db`) stores block snapshots for fast startup:

- Written periodically after event application.
- On startup, loads cached blocks and replays only newer events.
- Fully disposable -- deleting `cache.db` triggers a full replay from `eventstore.db`.

This optimization keeps startup time constant regardless of event log size.

## EngineMessage

All communication with the actor goes through typed messages:

```rust
enum EngineMessage {
    ProcessCommand { command, response },
    GetBlock { block_id, response },
    GetAllBlocks { response },
    GetAllEditors { response },
    GetAllGrants { response },
    GetEditorGrants { editor_id, response },
    GetBlockGrants { block_id, response },
    CheckGrant { editor_id, cap_id, block_id, response },
    GetAllEvents { response },
    Shutdown,
}
```

Each variant carries a `oneshot::Sender` for the response, enabling async request-response over the mpsc channel.

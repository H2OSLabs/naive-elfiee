# Event System

All state in Elfiee is derived from an append-only log of immutable events stored in SQLite. There is no mutable state -- the current state is always a projection of the event history.

## EAVT Model

Every event follows the Entity-Attribute-Value-Timestamp schema:

| Field | Type | Description |
|-------|------|-------------|
| `event_id` | UUID | Unique identifier |
| `entity` | String | What changed -- usually a `block_id`, sometimes an `editor_id` |
| `attribute` | String | Who did what -- format: `"{editor_id}/{cap_id}"` |
| `value` | JSON | The payload describing the change |
| `timestamp` | Map<String, i64> | Vector clock for conflict detection |
| `created_at` | String | Wall clock time (ISO 8601) |

### Attribute Format

The attribute encodes both the actor and the action:

```text
"alice/core.create"      -- alice created a block
"bot-001/document.write" -- bot-001 wrote document content
"alice/core.grant"       -- alice granted a permission
```

This allows efficient filtering by editor or by capability.

## Four Content Modes

The `value` field supports four semantic modes that determine how events are applied during projection:

| Mode | Semantics | Use Case |
|------|-----------|----------|
| `full` | Replace entire contents | `document.write`, `task.write` |
| `delta` | Merge into existing contents | `core.rename` (updates name only) |
| `ref` | Reference to external data | `task.commit` (records downstream IDs) |
| `append` | Add entry to a list | `session.append` (adds log entry) |

### Full Mode Example

```json
{
  "contents": { "markdown": "# New content\n\nFull replacement." },
  "metadata": { "updated_at": "2025-01-15T10:00:00Z" }
}
```

### Delta Mode Example

```json
{
  "name": "Renamed Block",
  "metadata": { "description": "Updated description" }
}
```

### Append Mode Example

```json
{
  "entry": {
    "role": "user",
    "content": "Please review the code.",
    "timestamp": "2025-01-15T10:00:00Z"
  }
}
```

## Vector Clock

Each event carries a vector clock `HashMap<String, i64>` tracking the transaction count per editor. This enables **Optimistic Concurrency Control (OCC)**:

1. Editor sends a command with their last-known vector clock.
2. Engine compares it with the current state's vector clock.
3. If stale (another editor committed in between), the command is rejected.
4. If current, the command proceeds and increments the editor's counter.

```text
Event 1: { "alice": 1 }              -- alice's first action
Event 2: { "alice": 1, "bob": 1 }    -- bob's first action
Event 3: { "alice": 2, "bob": 1 }    -- alice's second action
```

## Event Store

Events are persisted to `eventstore.db` (SQLite) inside the `.elf/` directory. The store provides:

- **Append**: Write one or more events atomically.
- **Read all**: Load all events for replay during engine startup.
- **Query by entity/attribute**: Filter events for history views.

The database schema:

```sql
CREATE TABLE events (
    event_id   TEXT PRIMARY KEY,
    entity     TEXT NOT NULL,
    attribute  TEXT NOT NULL,
    value      TEXT NOT NULL,      -- JSON string
    timestamp  TEXT NOT NULL,      -- JSON string of vector clock
    created_at TEXT NOT NULL       -- ISO 8601
);
```

## Snapshot Mechanism (CacheStore)

Replaying the full event log on every startup becomes expensive as the log grows. The **CacheStore** (`cache.db`, stored alongside `.elf/`) provides block snapshots:

- After applying events, the engine periodically writes block state to `cache.db`.
- On next startup, the engine loads from the cache and only replays events after the cache point.
- The cache is disposable -- deleting it forces a full replay with no data loss.

## Event vs Git

Elfiee's event log and Git serve complementary roles:

| Aspect | Event Log | Git |
|--------|-----------|-----|
| Granularity | Per-operation (every write, rename, grant) | Per-commit (human-chosen checkpoints) |
| Scope | Block-level changes within `.elf/` | Project-wide file changes |
| Purpose | State reconstruction, audit trail, CBAC | Version control, collaboration, branching |
| Storage | `eventstore.db` (SQLite) | `.git/` |

The `task.commit` capability bridges the two: it records an audit event in the log and can trigger a Git commit of downstream block content.

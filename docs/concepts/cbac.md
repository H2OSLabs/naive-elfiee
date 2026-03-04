# Capability-Based Access Control (CBAC)

Elfiee uses a fine-grained permission model where every operation requires explicit authorization. Permissions are not role-based -- they are granted per `(editor, capability, block)` triple.

## Authorization Flow

When a command arrives at the engine, authorization follows a two-layer check:

```text
Command arrives
      |
      v
  Owner check: Is editor the block owner?
      |
     YES --> AUTHORIZED
      |
      NO
      |
      v
  Grants check: Does GrantsTable have (editor, cap_id, block_id)?
      |
     YES --> AUTHORIZED
      |
      NO
      |
      v
  Wildcard check: Does GrantsTable have (editor, cap_id, "*")?
      |
     YES --> AUTHORIZED
      |
      NO --> REJECTED
```

### Layer 1: Owner Check

The block creator (`block.owner`) is always authorized for any capability on their own blocks. No grant entry is needed.

### Layer 2: GrantsTable Check

For non-owners, the `GrantsTable` is consulted. It checks for:

1. **Exact match**: `(editor_id, cap_id, block_id)` -- permission on a specific block.
2. **Wildcard match**: `(editor_id, cap_id, "*")` -- permission on all blocks.

## GrantsTable

The grants table is an in-memory projection built from `core.grant` and `core.revoke` events:

```rust
struct GrantsTable {
    // editor_id -> Vec<(cap_id, block_id)>
    grants: HashMap<String, Vec<(String, String)>>,
}
```

Key operations:

| Method | Purpose |
|--------|---------|
| `add_grant(editor, cap, block)` | Add a permission entry |
| `remove_grant(editor, cap, block)` | Remove a permission entry |
| `has_grant(editor, cap, block)` | Check if permission exists |
| `get_editor_grants(editor)` | List all grants for an editor |
| `from_events(events)` | Rebuild table from event history |

## Task-Granularity Permission Isolation

CBAC enables task-level isolation. An agent working on Task A can be granted `document.write` only on blocks linked to that task, while having no access to Task B's blocks.

```text
Task A (owner: alice)
  |-- implement --> doc-1   (bot-001 granted document.write on doc-1)
  |-- implement --> doc-2   (bot-001 granted document.write on doc-2)

Task B (owner: bob)
  |-- implement --> doc-3   (bot-001 has NO access)
```

This prevents agents from accidentally modifying unrelated work.

## Grant and Revoke as Events

Permission changes are themselves events, providing a full audit trail:

### core.grant Event

```json
{
  "entity": "<grant-target-block-id>",
  "attribute": "alice/core.grant",
  "value": {
    "editor": "bot-001",
    "capability": "document.write",
    "block": "doc-1"
  }
}
```

### core.revoke Event

```json
{
  "entity": "<revoke-target-block-id>",
  "attribute": "alice/core.revoke",
  "value": {
    "editor": "bot-001",
    "capability": "document.write",
    "block": "doc-1"
  }
}
```

Only the block owner can grant or revoke permissions on their blocks.

## Bootstrap via Wildcard Grants

When a new agent is registered with `elf register`, it receives wildcard grants for common capabilities:

```text
(bot-001, document.read, *)
(bot-001, document.write, *)
(bot-001, task.read, *)
(bot-001, task.write, *)
(bot-001, session.read, *)
(bot-001, session.append, *)
(bot-001, core.create, *)
...
```

Owner-only capabilities (`core.grant`, `core.revoke`) are excluded from auto-grants. The owner can later narrow these wildcards to specific blocks.

## System Editor

The system editor ID (configured in `~/.elf/config.json`) is always authorized for all operations, acting as a superuser. This is used for bootstrap operations and administrative tasks.

## Certificator Implementation

The default `certificator` in `CapabilityHandler`:

```rust
fn certificator(
    &self,
    editor_id: &str,
    block: &Block,
    grants: &HashMap<String, Vec<(String, String)>>,
) -> bool {
    // Owner always has access
    if block.owner == editor_id {
        return true;
    }
    // Check grants table
    if let Some(editor_grants) = grants.get(editor_id) {
        editor_grants.iter().any(|(cap, blk)| {
            cap == self.cap_id() && (blk == &block.block_id || blk == "*")
        })
    } else {
        false
    }
}
```

Individual capabilities can override this for custom authorization logic if needed.

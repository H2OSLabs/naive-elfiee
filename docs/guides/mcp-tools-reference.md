# MCP Tools Reference

Elfiee exposes an MCP SSE server (default port 47200) that agents connect to for all operations. This document covers every MCP tool and resource.

## Server Info

| Property | Value |
|---|---|
| Server name | `elfiee-mcp` |
| Transport | SSE (Server-Sent Events) |
| SSE endpoint | `GET /sse` |
| Message endpoint | `POST /message` |
| Default port | `47200` |
| Identity model | Per-connection (each SSE connection gets its own `ElfieeMcpServer` instance) |

## Connection Flow

```
1. Client connects via SSE at http://127.0.0.1:47200/sse
2. Call elfiee_auth with editor_id        --> Authenticates this connection
3. Call elfiee_open with project path     --> Opens a project
4. Use block/document/task tools          --> Operate on the project
5. (Optional) Call elfiee_close           --> Release project resources
```

Read-only tools (`elfiee_file_list`, `elfiee_block_list`) work without authentication, but write operations require `elfiee_auth` first.

---

## Tools

### Connection Management

#### elfiee_auth

Authenticate this MCP connection by binding an editor_id. Must be called before any write operations.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `editor_id` | string | Yes | Editor ID to bind (from `elf register` or `config.toml`) |
| `project` | string | No | Project path. If provided, returns the Skill guide |
| `role` | string | No | Role name. Loads role-specific skill if available |

**Returns:**

```json
{
  "authenticated": true,
  "editor_id": "claude-a1b2c3d4",
  "hint": "You can now perform write operations...",
  "skill": "# Elfiee Skill Guide\n..."
}
```

The `skill` field is only included when `project` is provided and a skill document exists.

---

#### elfiee_open

Open an `.elf` project directory. Creates the project if it does not exist. Must be called before other operations on the project.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project directory |

**Returns:**

```json
{
  "ok": true,
  "project": "/home/user/my-project",
  "file_id": "abc123",
  "block_count": 15,
  "skill": "# Elfiee Skill Guide\n..."
}
```

---

#### elfiee_close

Close an `.elf` project and release its resources.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |

**Returns:**

```json
{
  "ok": true,
  "project": "/home/user/my-project",
  "closed": true
}
```

---

### File Operations

#### elfiee_file_list

List all currently open `.elf` project files with block counts and type summaries. Works without authentication.

**Parameters:** None.

**Returns:**

```json
{
  "files": [
    {
      "project": "/home/user/my-project",
      "file_id": "abc123",
      "connection_editor": "claude-a1b2c3d4",
      "block_count": 15,
      "block_types": {
        "document": 12,
        "task": 2,
        "session": 1
      }
    }
  ],
  "count": 1,
  "hint": "Use the 'project' value as the 'project' parameter for other elfiee tools."
}
```

---

### Block Operations

#### elfiee_block_list

List all blocks in a project with type, content preview, relations, and metadata. Results are CBAC-filtered by the authenticated editor's permissions.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |

**Returns:**

```json
{
  "project": "/home/user/my-project",
  "blocks": [
    {
      "block_id": "a1b2c3d4-...",
      "name": "src/main.rs",
      "block_type": "document",
      "owner": "system",
      "format": "rs",
      "content_preview": "fn main() {\n    println...",
      "content_length": 142
    },
    {
      "block_id": "e5f6a7b8-...",
      "name": "login-task",
      "block_type": "task",
      "owner": "alice",
      "status": "in_progress",
      "assigned_to": "claude-a1b2c3d4",
      "content_preview": "Implement OAuth2 login..."
    }
  ],
  "count": 2
}
```

Block summaries include type-specific previews:
- **document**: `content_preview`, `content_length`, `format`
- **task**: `content_preview` (from description), `status`, `assigned_to`
- **session**: `entry_count`

---

#### elfiee_block_get

Get full details of a specific block including all contents, children relations, metadata, and permissions. Requires `{block_type}.read` permission.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `block_id` | string | Yes | ID of the block |

**Returns:**

```json
{
  "block_id": "a1b2c3d4-...",
  "name": "src/main.rs",
  "block_type": "document",
  "owner": "system",
  "contents": {
    "format": "rs",
    "content": "fn main() {\n    println!(\"Hello\");\n}"
  },
  "children": {
    "implement": ["b2c3d4e5-..."]
  },
  "description": "Main entry point",
  "grants": [
    { "editor": "claude-a1b2c3d4", "capability": "document.write" }
  ]
}
```

---

#### elfiee_block_create

Create a new block (document, task, or session). Returns the created block with its generated `block_id`.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `name` | string | Yes | Display name for the new block |
| `block_type` | string | Yes | Block type: `document`, `task`, or `session` |
| `parent_id` | string | No | Parent block ID to link to via `implement` relation |

**Returns:**

```json
{
  "ok": true,
  "capability": "core.create",
  "editor": "claude-a1b2c3d4",
  "events_committed": 1,
  "created_block_id": "f1e2d3c4-...",
  "block": {
    "block_id": "f1e2d3c4-...",
    "name": "new-task",
    "block_type": "task",
    "owner": "claude-a1b2c3d4"
  }
}
```

---

#### elfiee_block_delete

Soft-delete a block. The block is marked as deleted but its history is preserved in the event store.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `block_id` | string | Yes | ID of the block to delete |

**Returns:**

```json
{
  "ok": true,
  "capability": "core.delete",
  "editor": "claude-a1b2c3d4",
  "events_committed": 1
}
```

---

#### elfiee_block_rename

Rename a block (updates the block's `name` field via `core.write`).

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `block_id` | string | Yes | ID of the block to rename |
| `name` | string | Yes | New name for the block |

**Returns:**

```json
{
  "ok": true,
  "capability": "core.write",
  "editor": "claude-a1b2c3d4",
  "events_committed": 1,
  "block": { "block_id": "...", "name": "new-name", ... }
}
```

---

#### elfiee_block_link

Add a relation between two blocks (parent -> child).

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `parent_id` | string | Yes | Parent block ID |
| `child_id` | string | Yes | Child block ID |
| `relation` | string | Yes | Relation type (e.g., `implement`) |

**Returns:**

```json
{
  "ok": true,
  "capability": "core.link",
  "editor": "claude-a1b2c3d4",
  "events_committed": 1,
  "block": { "block_id": "...", "children": { "implement": ["child-id"] } }
}
```

---

#### elfiee_block_unlink

Remove a relation between two blocks.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `parent_id` | string | Yes | Parent block ID |
| `child_id` | string | Yes | Child block ID |
| `relation` | string | Yes | Relation type to remove |

**Returns:**

```json
{
  "ok": true,
  "capability": "core.unlink",
  "editor": "claude-a1b2c3d4",
  "events_committed": 1,
  "block": { "block_id": "...", "children": {} }
}
```

---

### Permission Operations

#### elfiee_grant

Grant a capability to an editor on a specific block. The block owner can always perform all operations without explicit grants.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `block_id` | string | Yes | Block ID to grant permission on |
| `editor_id` | string | Yes | Editor ID to grant permission to |
| `cap_id` | string | Yes | Capability ID (e.g., `document.write`, `task.read`) |

**Returns:**

```json
{
  "ok": true,
  "capability": "core.grant",
  "editor": "alice",
  "events_committed": 1
}
```

---

#### elfiee_revoke

Revoke a previously granted capability from an editor on a specific block.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `block_id` | string | Yes | Block ID to revoke permission on |
| `editor_id` | string | Yes | Editor ID to revoke permission from |
| `cap_id` | string | Yes | Capability ID to revoke |

**Returns:**

```json
{
  "ok": true,
  "capability": "core.revoke",
  "editor": "alice",
  "events_committed": 1
}
```

---

### Editor Operations

#### elfiee_editor_create

Create a new editor in the project.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `editor_id` | string | Yes | Editor ID for the new editor |
| `name` | string | No | Display name (defaults to `editor_id` if not provided) |

**Returns:**

```json
{
  "ok": true,
  "capability": "core.editor_create",
  "editor": "system",
  "events_committed": 1
}
```

---

#### elfiee_editor_delete

Delete an editor from the project.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `editor_id` | string | Yes | Editor ID to delete |
| `name` | string | No | (unused) |

**Returns:**

```json
{
  "ok": true,
  "capability": "core.editor_delete",
  "editor": "system",
  "events_committed": 1
}
```

---

### History and Time Travel

#### elfiee_block_history

Get the full event history for a specific block. Requires `{block_type}.read` permission (e.g., `document.read` for document blocks). Returns all events that affected this block in chronological order.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `block_id` | string | Yes | ID of the block |

**Returns:**

```json
{
  "block_id": "a1b2c3d4-...",
  "event_count": 3,
  "events": [
    {
      "event_id": "evt-001",
      "entity": "a1b2c3d4-...",
      "attribute": "system/core.create",
      "value": { "name": "src/main.rs", "type": "document", ... },
      "timestamp": { "system": 1 }
    },
    {
      "event_id": "evt-002",
      "entity": "a1b2c3d4-...",
      "attribute": "system/document.write",
      "value": { "contents": { "content": "fn main() {}" } },
      "timestamp": { "system": 2 }
    }
  ]
}
```

---

#### elfiee_state_at_event

Time travel: get the state of a block at a specific point in time by replaying events up to the given `event_id`. Requires `{block_type}.read` permission.

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `block_id` | string | Yes | Block ID to get state for |
| `event_id` | string | Yes | Event ID to replay up to |

**Returns:**

```json
{
  "block": {
    "block_id": "a1b2c3d4-...",
    "name": "src/main.rs",
    "block_type": "document",
    "owner": "system",
    "description": null,
    "contents": { "format": "rs", "content": "fn main() {}" },
    "children": {}
  },
  "grants": [
    { "editor_id": "claude-a1b2c3d4", "cap_id": "document.write", "block_id": "a1b2c3d4-..." }
  ],
  "at_event": "evt-002"
}
```

---

### Generic Execution

#### elfiee_exec

Execute any registered capability directly. Use for extension operations (`document.write`, `task.write`, `task.commit`, `session.append`, etc.) and also for core operations (`core.create`, `core.link`, `core.delete`, `core.grant`, `core.revoke`).

**Parameters:**

| Parameter | Type | Required | Description |
|---|---|---|---|
| `project` | string | Yes | Path to the `.elf` project |
| `capability` | string | Yes | Capability ID (e.g., `document.write`, `task.commit`) |
| `block_id` | string | No | Target block ID (required for most capabilities) |
| `payload` | object | No | Capability-specific JSON payload (defaults to `{}`) |

**Returns:**

Same response format as the specific tool for the capability. On success:

```json
{
  "ok": true,
  "capability": "document.write",
  "editor": "claude-a1b2c3d4",
  "events_committed": 1,
  "block": { ... }
}
```

On error:

```json
{
  "ok": false,
  "capability": "document.write",
  "error": "Not authorized for document.write on block a1b2c3d4-...",
  "hint": "The current editor lacks 'document.write' permission. Use elfiee_grant to grant it first."
}
```

### Payload Reference for elfiee_exec

#### document.write

```json
{ "content": "# Hello World\n\nThis is the document content." }
```

#### task.write

All fields optional, but at least one required:

```json
{
  "description": "Implement OAuth2 login",
  "status": "in_progress",
  "assigned_to": "claude-a1b2c3d4",
  "template": "code-review"
}
```

Valid status values: `pending`, `in_progress`, `completed`, `failed`.

#### task.commit

Empty payload. Requires the task block to have downstream blocks via `implement` relation:

```json
{}
```

#### session.append

```json
{
  "entry_type": "command",
  "data": {
    "command": "cargo test",
    "output": "test result: ok. 287 passed",
    "exit_code": 0
  }
}
```

Entry types:
- `command`: `{ "command": "...", "output": "...", "exit_code": 0 }`
- `message`: `{ "role": "agent|human", "content": "..." }`
- `decision`: `{ "action": "...", "related_blocks": ["block-id-1", ...] }`

#### core.create

```json
{
  "name": "New Block",
  "block_type": "document",
  "source": "outline",
  "format": "md",
  "description": "Optional description",
  "contents": { "format": "md", "content": "# Title" }
}
```

#### core.write

```json
{
  "name": "Updated Name",
  "description": "Updated description"
}
```

#### core.link / core.unlink

```json
{
  "relation": "implement",
  "target_id": "child-block-id"
}
```

#### core.grant / core.revoke

```json
{
  "target_editor": "bob",
  "capability": "document.write",
  "target_block": "*"
}
```

#### editor.create

```json
{
  "name": "Alice",
  "editor_type": "Human",
  "editor_id": "alice-custom-id"
}
```

#### editor.delete

```json
{
  "editor_id": "alice-custom-id"
}
```

---

## Error Handling

All tools return structured JSON on error. Error responses include contextual hints:

```json
{
  "ok": false,
  "capability": "document.write",
  "error": "Not authorized for document.write on block a1b2c3d4-...",
  "hint": "The current editor lacks 'document.write' permission. Use elfiee_grant to grant it first."
}
```

### Common Error Hints

| Error Pattern | Hint |
|---|---|
| Not authorized / permission | "Use elfiee_grant to grant it first." |
| Not found | "Use elfiee_block_list to see available blocks." |
| Type mismatch | "Check the block_type with elfiee_block_get." |
| Invalid payload | "Check the required fields for this tool." |

### MCP-Level Errors

| Error | Cause | Resolution |
|---|---|---|
| Not authenticated | `elfiee_auth` not called | Call `elfiee_auth` with your `editor_id` |
| Project not open | Project path not loaded | Call `elfiee_open` with the project path |
| Engine not found | File was closed | Call `elfiee_file_list` to check, then `elfiee_open` |
| Invalid payload | Malformed parameters | Check the tool's parameter schema |

---

## MCP Resources

Elfiee exposes MCP resources that clients can subscribe to. Resources are automatically updated when state changes (via `resources/list_changed` notifications).

### Static Resource

#### elfiee://files

List of currently open `.elf` project files.

```json
{
  "files": [
    { "file_id": "abc123", "project": "/home/user/my-project" }
  ],
  "count": 1
}
```

### Per-Project Resources

All URIs follow the pattern `elfiee://{project}/{resource}` where `{project}` is the project path.

#### elfiee://{project}/blocks

All blocks in the project (CBAC-filtered). Returns block summaries with type-specific previews.

```json
{
  "project": "/home/user/my-project",
  "blocks": [ { "block_id": "...", "name": "...", "block_type": "...", ... } ],
  "count": 15
}
```

#### elfiee://{project}/block/{block_id}

Full content of a specific block (CBAC-checked).

- **Document blocks**: Returns plain text content (`text/plain`)
- **Other blocks**: Returns full JSON representation (`application/json`)

#### elfiee://{project}/grants

All permission grants in the project (CBAC-filtered).

```json
{
  "project": "/home/user/my-project",
  "grants": [
    { "editor_id": "claude-a1b2c3d4", "capability": "document.write", "block_id": "*" }
  ],
  "count": 12
}
```

#### elfiee://{project}/events

Full event log for the project (CBAC-filtered).

```json
{
  "project": "/home/user/my-project",
  "events": [
    {
      "event_id": "evt-001",
      "entity": "a1b2c3d4-...",
      "attribute": "system/core.create",
      "value": { ... },
      "timestamp": { "system": 1 },
      "created_at": "2025-01-15T10:30:00Z"
    }
  ],
  "count": 42
}
```

#### elfiee://{project}/editors

List of editors in the project.

```json
{
  "project": "/home/user/my-project",
  "editors": [
    { "editor_id": "system", "name": "system", "editor_type": "Human" },
    { "editor_id": "claude-a1b2c3d4", "name": "claude", "editor_type": "Bot" }
  ],
  "count": 2
}
```

#### elfiee://{project}/my-tasks

Tasks assigned to or owned by the authenticated editor (CBAC-filtered).

```json
{
  "project": "/home/user/my-project",
  "editor_id": "claude-a1b2c3d4",
  "tasks": [
    {
      "block_id": "e5f6a7b8-...",
      "name": "login-task",
      "block_type": "task",
      "status": "in_progress",
      "assigned_to": "claude-a1b2c3d4"
    }
  ],
  "count": 1
}
```

#### elfiee://{project}/my-grants

Permissions granted to the authenticated editor.

```json
{
  "project": "/home/user/my-project",
  "editor_id": "claude-a1b2c3d4",
  "grants": [
    { "editor_id": "claude-a1b2c3d4", "cap_id": "document.write", "block_id": "*" },
    { "editor_id": "claude-a1b2c3d4", "cap_id": "task.write", "block_id": "*" }
  ],
  "count": 11
}
```

### Resource Templates

The server also exposes resource templates for client discovery:

| URI Template | Description |
|---|---|
| `elfiee://{project}/blocks` | List all blocks in a project |
| `elfiee://{project}/block/{block_id}` | Read a specific block's full content |
| `elfiee://{project}/grants` | Permission grants in a project |
| `elfiee://{project}/events` | Event sourcing log for a project |
| `elfiee://{project}/editors` | List of editors in a project |
| `elfiee://{project}/my-tasks` | Tasks assigned to the current editor |
| `elfiee://{project}/my-grants` | Permissions granted to the current editor |

---

## Tool Summary Table

| Tool | Auth Required | Description |
|---|---|---|
| `elfiee_auth` | No | Authenticate this connection |
| `elfiee_open` | No | Open/create a project |
| `elfiee_close` | No | Close a project |
| `elfiee_file_list` | No | List open projects |
| `elfiee_block_list` | Yes | List blocks (CBAC filtered) |
| `elfiee_block_get` | Yes | Get block details |
| `elfiee_block_create` | Yes | Create a new block |
| `elfiee_block_delete` | Yes | Soft-delete a block |
| `elfiee_block_rename` | Yes | Rename a block |
| `elfiee_block_link` | Yes | Add block relation |
| `elfiee_block_unlink` | Yes | Remove block relation |
| `elfiee_grant` | Yes | Grant a capability |
| `elfiee_revoke` | Yes | Revoke a capability |
| `elfiee_editor_create` | Yes | Create an editor |
| `elfiee_editor_delete` | Yes | Delete an editor |
| `elfiee_block_history` | Yes | Get block event history |
| `elfiee_state_at_event` | Yes | Time-travel to event state |
| `elfiee_exec` | Yes | Execute any capability |

**Total: 18 tools**

---

## Notification System

The MCP server pushes `resources/list_changed` notifications to connected clients whenever state changes. This allows clients to re-fetch resources to get updated block/grant/event data without polling.

The notification fan-out subscribes to an internal broadcast channel (`state_changed_tx`). When any capability execution succeeds, a notification is sent to all connected clients.

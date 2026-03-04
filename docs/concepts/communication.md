# Communication Architecture

Elfiee communicates exclusively through MCP (Model Context Protocol) over SSE (Server-Sent Events). This is the only external protocol. The server is headless -- there is no GUI.

## Single Process, Single Port

```text
elf serve --port 47200
  |
  +-- SSE endpoint:  GET  /sse       (client connects here)
  +-- POST endpoint: POST /message   (client sends commands here)
  |
  +-- EngineManager (shared across all connections)
       +-- EngineActor (project-a)
       +-- EngineActor (project-b)
```

All agents connect to the same port. Each connection gets an independent `ElfieeMcpServer` instance with its own identity context.

## Per-Connection Identity

Every MCP connection starts unauthenticated. The agent must call `elfiee_auth` to establish identity:

```text
Agent connects via SSE
      |
      v
  ElfieeMcpServer created (no identity yet)
      |
      v
  Agent calls elfiee_auth({ editor_id: "bot-001" })
      |
      v
  Server binds editor_id to this connection
      |
      v
  All subsequent operations use this editor_id for CBAC
```

After authentication, the agent calls `elfiee_open` to open a project, then uses capability tools.

## MCP Tools

Tools exposed by the MCP server (all prefixed with `elfiee_`):

### Lifecycle

| Tool | Purpose |
|------|---------|
| `elfiee_auth` | Authenticate with editor_id. Returns skill document. |
| `elfiee_open` | Open a project. Returns project info + skill. |
| `elfiee_close` | Close a project. |

### Block Operations

| Tool | Purpose |
|------|---------|
| `elfiee_block_list` | List all blocks (CBAC filtered) |
| `elfiee_block_get` | Get block details |
| `elfiee_block_create` | Create a new block |
| `elfiee_block_delete` | Delete a block |
| `elfiee_block_rename` | Rename a block |
| `elfiee_block_link` | Add implement relation |
| `elfiee_block_unlink` | Remove implement relation |

### Content Operations

| Tool | Purpose |
|------|---------|
| `elfiee_document_read` | Read document content |
| `elfiee_document_write` | Write document content |
| `elfiee_task_read` | Read task content |
| `elfiee_task_write` | Write task content |
| `elfiee_task_commit` | Commit task to Git |
| `elfiee_session_read` | Read session entries |
| `elfiee_session_append` | Append session entry |

### Permission Operations

| Tool | Purpose |
|------|---------|
| `elfiee_grant` | Grant capability to editor |
| `elfiee_revoke` | Revoke capability from editor |

### Editor Operations

| Tool | Purpose |
|------|---------|
| `elfiee_editor_create` | Create new editor |
| `elfiee_editor_delete` | Delete editor |

### Generic

| Tool | Purpose |
|------|---------|
| `elfiee_exec` | Execute any capability by cap_id (escape hatch) |

## MCP Resources

The server also exposes resources for read-only access:

| URI Pattern | Content |
|-------------|---------|
| `elfiee://files` | List of open projects |
| `elfiee://{project}/blocks` | All blocks in a project |
| `elfiee://{project}/block/{id}` | Single block details |
| `elfiee://{project}/grants` | All grants |
| `elfiee://{project}/events` | Event history |
| `elfiee://{project}/editors` | All editors |
| `elfiee://{project}/my-tasks` | Tasks owned by current editor |
| `elfiee://{project}/my-grants` | Grants for current editor |

## Broadcast Channel

After events are committed to the event store, the engine broadcasts them via a tokio broadcast channel. This allows:

- MCP SSE connections to push event notifications to connected agents.
- Multiple listeners to receive the same events without polling.

```text
EngineActor --commit events--> EventStore
      |
      +--broadcast--> MCP Connection 1 (SSE push)
      +--broadcast--> MCP Connection 2 (SSE push)
```

## Connection Lifecycle

```text
1. Agent opens SSE connection to /sse
2. Server creates ElfieeMcpServer instance
3. Agent sends elfiee_auth({ editor_id })
4. Agent sends elfiee_open({ project: "/path/to/project" })
5. Agent performs operations (block CRUD, content read/write, etc.)
6. Agent sends elfiee_close({ project }) when done
7. SSE connection closes (or agent disconnects)
```

Connections are stateful but lightweight. Reconnection requires re-authentication. Server state (engines, event stores) persists across connections.

## No Orchestration

Elfiee is strictly passive:

- It never initiates connections to agents.
- It never spawns agent processes.
- It never sends commands to agents.
- All communication flows inward: Agent -> Elfiee.

Orchestration of multi-agent workflows is the responsibility of an external coordinator that itself connects to Elfiee as an agent.

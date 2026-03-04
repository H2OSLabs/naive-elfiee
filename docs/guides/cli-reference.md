# Elfiee CLI Reference

The `elf` command-line tool provides project initialization, agent registration, file scanning, block management, event inspection, permission management, and MCP server operations.

## Quick Reference

```
elf init [project]                              # Initialize .elf/ project
elf register [agent_type] [options]             # Register an agent
elf unregister <editor_id> [options]            # Unregister an agent
elf serve [--port 47200] [--project <path>]     # Start MCP SSE server
elf run <template> [--project <path>] [--port]  # Run a Socialware workflow
elf status [project]                            # Show project status
elf scan [file] [--project <path>]              # Scan and sync files
elf block list [--project <path>]               # List all blocks
elf block get <block> [--project <path>]        # Show block details
elf event list [--project <path>]               # List all events
elf event history <block> [--project <path>]    # Show block event history
elf event at <block> <event_id> [--project]     # Time-travel to event state
elf grant <editor> <cap> [block] [--project]    # Grant a capability
elf revoke <editor> <cap> [block] [--project]   # Revoke a capability
```

---

## elf init

Initialize a new `.elf/` project directory.

### Usage

```
elf init [project]
```

### Arguments

| Argument | Default | Description |
|---|---|---|
| `project` | `.` (current directory) | Path to the project directory |

### What It Does

1. Creates the `.elf/` directory structure:
   - `eventstore.db` -- SQLite event store (source of truth)
   - `config.toml` -- Project configuration
   - `templates/skills/default.md` -- Default agent skill document
2. Seeds bootstrap events (system editor + wildcard grants)
3. Scans project files and creates document blocks for each recognized file type (respects `.elfignore` and `.gitignore`)

### Examples

```bash
# Initialize current directory
elf init

# Initialize a specific project
elf init /home/user/my-project

# Initialize a new directory (created if it does not exist)
elf init /home/user/new-project
```

### Errors

- Fails if `.elf/` already exists in the target directory.

---

## elf register

Register an agent by creating an Editor identity with permissions, injecting MCP configuration, and installing the Skill document.

### Usage

```
elf register [agent_type] [--name <name>] [--config-dir <dir>] [--project <path>] [--port <port>]
```

### Arguments

| Argument | Default | Description |
|---|---|---|
| `agent_type` | `claude` | Agent type: `claude`, `openclaw`, or `custom` |

### Options

| Option | Default | Description |
|---|---|---|
| `--name <name>` | Same as `agent_type` | Display name for the editor |
| `--config-dir <dir>` | Auto-inferred | Agent configuration directory |
| `--project <path>` | `.` | Project path |
| `--port <port>` | `47200` | MCP server port for config injection |

### What It Does

**Inward (event store):**
1. Creates a Bot editor in the event store with a generated `editor_id` (format: `{agent_type}-{uuid8}`)
2. Grants default capabilities (wildcard `*` on all blocks):
   - `document.read`, `document.write`
   - `task.read`, `task.write`, `task.commit`
   - `session.append`, `session.read`
   - `core.create`, `core.link`, `core.unlink`, `core.delete`

**Outward (agent config):**
3. Writes `.mcp.json` in the project root with the Elfiee SSE server URL
4. Writes `settings.local.json` in the config directory with:
   - `env.ELFIEE_EDITOR_ID` -- The generated editor ID
   - `env.ELFIEE_PROJECT` -- The project path
   - `permissions.allow` -- All 18 Elfiee MCP tool permissions
5. Installs `skills/elfiee/SKILL.md` and `skills/elfiee/scripts/reconcile.sh`

**Owner-only capabilities** (`core.grant`, `core.revoke`, `editor.create`, `editor.delete`) are NOT granted to agents.

### Config Directory Inference

| Agent Type | Inferred Config Dir |
|---|---|
| `claude` | `{project}/.claude` |
| `openclaw` | `{project}/.claude` |
| `custom` | `{project}/.claude` |

### Examples

```bash
# Register a Claude agent in current project
elf register claude

# Register with a custom name
elf register openclaw --name "code-reviewer"

# Register with explicit config directory and port
elf register claude --config-dir /home/user/.claude --port 47300

# Register in a specific project
elf register claude --project /home/user/my-project
```

---

## elf unregister

Remove an agent by deleting its Editor identity (with cascading grant removal) and cleaning up injected configurations.

### Usage

```
elf unregister <editor_id> [--config-dir <dir>] [--project <path>]
```

### Arguments

| Argument | Required | Description |
|---|---|---|
| `editor_id` | Yes | The editor ID to unregister |

### Options

| Option | Default | Description |
|---|---|---|
| `--config-dir <dir>` | Auto-inferred (`.claude`) | Agent configuration directory |
| `--project <path>` | `.` | Project path |

### What It Does

**Inward:**
1. Deletes the editor from the event store (cascading: all grants for this editor are also removed)

**Outward:**
2. Removes `elfiee` from `.mcp.json` (preserves other MCP servers)
3. Removes `ELFIEE_*` env variables from `settings.local.json`
4. Removes `mcp__elfiee__*` permissions from `settings.local.json` (preserves other permissions)
5. Deletes `skills/elfiee/` directory

### Examples

```bash
# Unregister by editor ID
elf unregister claude-a1b2c3d4

# Unregister with explicit config directory
elf unregister claude-a1b2c3d4 --config-dir /home/user/.claude
```

---

## elf serve

Start the MCP SSE server for agent connections.

### Usage

```
elf serve [--port <port>] [--project <path>]
```

### Options

| Option | Default | Description |
|---|---|---|
| `--port <port>` | `47200` | SSE server port |
| `--project <path>` | (none) | Pre-load a project at startup |

### What It Does

1. Starts the MCP SSE server on `http://127.0.0.1:{port}`
2. Optionally pre-loads a project (opens the event store and spawns the engine)
3. Runs until Ctrl+C

### Connection Flow

After a client connects:
1. Client calls `elfiee_auth` with their `editor_id`
2. Client calls `elfiee_open` to open a project
3. Client uses block/document/task tools to operate

### Endpoints

| Method | Path | Description |
|---|---|---|
| GET | `/sse` | SSE connection endpoint |
| POST | `/message` | MCP message endpoint |

### Examples

```bash
# Start server with defaults
elf serve

# Start on a custom port with a pre-loaded project
elf serve --port 47300 --project /home/user/my-project
```

---

## elf run

Run a Socialware workflow template: register all defined roles and start the MCP server.

### Usage

```
elf run <template> [--project <path>] [--port <port>]
```

### Arguments

| Argument | Required | Description |
|---|---|---|
| `template` | Yes | Template name (TOML workflow file) |

### Options

| Option | Default | Description |
|---|---|---|
| `--project <path>` | `.` | Project path |
| `--port <port>` | `47200` | MCP server port |

### Template Resolution

Templates are searched in this order:
1. **Project-level**: `.elf/templates/workflows/{name}.toml`
2. **Built-in**: Currently only `code-review` is built in

### Template Format (TOML)

```toml
[socialware]
name = "Code Review"
namespace = "code-review"
description = "Two-agent code review workflow"

[[roles]]
id = "coder"
agent_type = "claude"
capabilities = ["document.read", "document.write", "task.read", "task.write", "core.create", "core.link"]

[[roles]]
id = "reviewer"
agent_type = "claude"
capabilities = ["document.read", "task.read", "task.write", "task.commit"]
# Optional fine-grained per-block grants:
# [[roles.grants]]
# capability = "task.write"
# block = "review-task"
```

### What It Does

1. Initializes the project if `.elf/` does not exist
2. Registers each role defined in the template (Editor + Grants + MCP config)
3. Starts the MCP server
4. Prints agent startup commands

Elfiee only registers roles and starts the server. Task/session creation and orchestration are handled by an external Coordinator via MCP tools.

### Examples

```bash
# Run the built-in code-review workflow
elf run code-review

# Run a custom template
elf run my-workflow --project /home/user/my-project --port 47300
```

---

## elf status

Display project status including event, editor, block, and grant counts.

### Usage

```
elf status [project]
```

### Arguments

| Argument | Default | Description |
|---|---|---|
| `project` | `.` | Project path |

### Output

```
Elfiee Project: /home/user/my-project
  Config: my-project

  Events:  42
  Editors: 3
  Blocks:  15
  Grants:  28
```

### Examples

```bash
elf status
elf status /home/user/my-project
```

---

## elf scan

Scan project files and synchronize them as document blocks. New files create blocks; existing files update content.

### Usage

```
elf scan [file] [--project <path>]
```

### Arguments

| Argument | Default | Description |
|---|---|---|
| `file` | (none) | Specific file to sync. Omit for batch scan |

### Options

| Option | Default | Description |
|---|---|---|
| `--project <path>` | `.` | Project path |

### Behavior

**Batch mode** (no file argument):
- Scans all files in the project directory
- Respects `.elfignore` and `.gitignore` exclusion rules
- Hidden files/directories are excluded
- Only files with extensions recognized in `.elftypes` are processed
- New files: creates a document block and writes content
- Existing files: updates the document block content

**Single file mode** (file argument provided):
- Reads the specified file
- If a block with the same name exists: updates its content
- If no matching block exists: creates a new block and writes content

### File Type Recognition

File extensions are mapped to block types via the `.elftypes` configuration. Common mappings include source code extensions (`rs`, `py`, `ts`, `js`, etc.) and document formats (`md`, `txt`, `toml`, `json`, etc.).

### Examples

```bash
# Batch scan all files
elf scan

# Sync a single file
elf scan src/main.rs

# Scan in a specific project
elf scan --project /home/user/my-project
```

---

## elf block list

List all blocks in the project (CBAC-filtered).

### Usage

```
elf block list [--project <path>]
```

### Options

| Option | Default | Description |
|---|---|---|
| `--project <path>` | `.` | Project path |

### Output

```
NAME                                               TYPE       ID                                   OWNER
--------------------------------------------------------------------------------------------------------------
README.md                                          document   a1b2c3d4-...                         system
src/main.rs                                        document   e5f6a7b8-...                         system
login-task                                         task       c9d0e1f2-...                         alice

Total: 3 blocks
```

### Examples

```bash
elf block list
elf block list --project /home/user/my-project
```

---

## elf block get

Show detailed information about a single block, including its contents and relations.

### Usage

```
elf block get <block> [--project <path>]
```

### Arguments

| Argument | Required | Description |
|---|---|---|
| `block` | Yes | Block name or block ID |

### Options

| Option | Default | Description |
|---|---|---|
| `--project <path>` | `.` | Project path |

### Block Resolution

The `block` argument supports dual-mode resolution:
- **By ID**: Exact match against `block_id`
- **By name**: Exact match against `block.name`. Fails if multiple blocks share the same name (use the block ID instead)
- **Wildcard**: `*` passes through as-is

### Output

```
ID:    a1b2c3d4-e5f6-7890-abcd-ef1234567890
Name:  src/main.rs
Type:  document
Owner: system

Relations:
  implement -> b2c3d4e5-f6a7-8901-bcde-f23456789012

Contents:
{
  "format": "rs",
  "content": "fn main() {\n    println!(\"Hello\");\n}"
}
```

### Examples

```bash
elf block get src/main.rs
elf block get a1b2c3d4-e5f6-7890-abcd-ef1234567890
elf block get README.md --project /home/user/my-project
```

---

## elf event list

List all events in the project (CBAC-filtered).

### Usage

```
elf event list [--project <path>]
```

### Options

| Option | Default | Description |
|---|---|---|
| `--project <path>` | `.` | Project path |

### Output

```
BLOCK                    CAPABILITY           EDITOR       CREATED_AT           EVENT_ID
------------------------------------------------------------------------------------------
src/main.rs              core.create          system       2025-01-15T10:30:00  a1b2c3d4
src/main.rs              document.write       system       2025-01-15T10:30:01  e5f6a7b8
login-task               task.write           alice        2025-01-15T11:00:00  c9d0e1f2

Total: 3 events
```

### Examples

```bash
elf event list
elf event list --project /home/user/my-project
```

---

## elf event history

Show the full event history for a specific block.

### Usage

```
elf event history <block> [--project <path>]
```

### Arguments

| Argument | Required | Description |
|---|---|---|
| `block` | Yes | Block name or block ID |

### Options

| Option | Default | Description |
|---|---|---|
| `--project <path>` | `.` | Project path |

### Examples

```bash
elf event history src/main.rs
elf event history a1b2c3d4 --project /home/user/my-project
```

---

## elf event at

Time-travel: view the state of a block at a specific point in history by replaying events up to the given event ID.

### Usage

```
elf event at <block> <event_id> [--project <path>]
```

### Arguments

| Argument | Required | Description |
|---|---|---|
| `block` | Yes | Block name or block ID |
| `event_id` | Yes | Target event ID (replay up to this event) |

### Options

| Option | Default | Description |
|---|---|---|
| `--project <path>` | `.` | Project path |

### Output

```
Block state at event a1b2c3d4:

  ID:    e5f6a7b8-...
  Name:  src/main.rs
  Type:  document
  Owner: system

Contents:
{
  "format": "rs",
  "content": "fn main() {}"
}

Grants at this point (2):
  alice -- document.write on e5f6a7b8-...
  bob -- document.read on e5f6a7b8-...
```

### Examples

```bash
elf event at src/main.rs a1b2c3d4
elf event at src/main.rs a1b2c3d4 --project /home/user/my-project
```

---

## elf grant

Grant a capability to an editor on a block.

### Usage

```
elf grant <editor_id> <capability> [block] [--project <path>]
```

### Arguments

| Argument | Required | Default | Description |
|---|---|---|---|
| `editor_id` | Yes | -- | The editor ID to grant to |
| `capability` | Yes | -- | The capability ID (e.g., `document.write`, `task.read`) |
| `block` | No | `*` (wildcard) | Block name, block ID, or `*` for all blocks |

### Options

| Option | Default | Description |
|---|---|---|
| `--project <path>` | `.` | Project path |

### Available Capabilities

**Core**: `core.create`, `core.write`, `core.link`, `core.unlink`, `core.delete`, `core.grant`, `core.revoke`

**Editor**: `editor.create`, `editor.delete`

**Extensions**: `document.read`, `document.write`, `task.read`, `task.write`, `task.commit`, `session.append`, `session.read`

### Examples

```bash
# Grant document.write on all blocks (wildcard)
elf grant claude-a1b2c3d4 document.write

# Grant task.write on a specific block
elf grant claude-a1b2c3d4 task.write login-task

# Grant using block ID
elf grant claude-a1b2c3d4 document.read e5f6a7b8-f6a7-8901-bcde-f23456789012
```

---

## elf revoke

Revoke a previously granted capability from an editor on a block.

### Usage

```
elf revoke <editor_id> <capability> [block] [--project <path>]
```

### Arguments

| Argument | Required | Default | Description |
|---|---|---|---|
| `editor_id` | Yes | -- | The editor ID to revoke from |
| `capability` | Yes | -- | The capability ID to revoke |
| `block` | No | `*` (wildcard) | Block name, block ID, or `*` for all blocks |

### Options

| Option | Default | Description |
|---|---|---|
| `--project <path>` | `.` | Project path |

### Examples

```bash
# Revoke document.write wildcard grant
elf revoke claude-a1b2c3d4 document.write

# Revoke task.write on a specific block
elf revoke claude-a1b2c3d4 task.write login-task
```

---

## Global Behavior Notes

### Project Path Resolution

All commands that accept a `--project` option (or positional `project` argument) canonicalize the path to an absolute path. The default is `.` (current working directory).

### System Editor

Most CLI commands operate as the `system` editor (obtained from the global Elfiee config). The system editor has wildcard grants on all capabilities and is created during `elf init`.

### Block Name/ID Resolution

Commands that accept a `block` argument support dual-mode resolution:

1. `*` -- Wildcard, passes through directly
2. **Exact ID match** -- If the input matches a `block_id`, returns it immediately
3. **Name match** -- If exactly one block has a matching `name`, returns its ID
4. **Ambiguous name** -- If multiple blocks share the name, returns an error listing all matches with their IDs

### Error Handling

All commands print errors to stderr and exit with code 1 on failure. Common error conditions:
- `.elf/` directory not found (run `elf init` first)
- Block not found (check with `elf block list`)
- Permission denied (check grants with `elf event list`)

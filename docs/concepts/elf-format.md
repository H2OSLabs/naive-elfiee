# .elf Project Format

An Elfiee project is marked by a `.elf/` directory at the project root, similar to how Git uses `.git/`. It is a plain directory -- not an archive.

## Directory Structure

```text
my-project/
  .elf/                       # Elfiee project directory
    eventstore.db             # Canonical event log (SQLite) -- source of truth
    config.toml               # Project configuration
    templates/
      skills/
        default.md            # Default agent skill document
  cache.db                    # Block snapshot cache (disposable)
  src/                        # User's project files
  ...
```

### Key Files

| File | Location | Purpose |
|------|----------|---------|
| `eventstore.db` | `.elf/` | Append-only SQLite database of all events. The single source of truth. |
| `config.toml` | `.elf/` | Project config: Git mode, project name, etc. |
| `cache.db` | Project root | CacheStore for fast startup. Disposable -- can be regenerated from events. |
| `templates/skills/` | `.elf/templates/skills/` | Skill documents injected to agents on registration. |

### Why Not an Archive?

The `.elf/` directory approach provides:

- **Git-friendly**: `eventstore.db` changes are trackable; no binary blob churn.
- **Always consistent**: Events are written directly to SQLite; no save/load cycle needed.
- **Inspectable**: Users can examine the database with standard SQLite tools.

## CLI Commands

The `elf` binary provides project management commands:

### Project Lifecycle

| Command | Description |
|---------|-------------|
| `elf init [path]` | Initialize a `.elf/` directory in the target project |
| `elf status [path]` | Show project status: blocks, editors, events count |
| `elf scan [path]` | Scan project files, creating blocks based on `.elftypes` mapping |

### Agent Registration

| Command | Description |
|---------|-------------|
| `elf register <type> [--name NAME] [--config-dir DIR]` | Register an agent editor |

Registration performs:
1. Create an `Editor` entity (type: Bot) in the event store.
2. Grant wildcard capabilities (excluding owner-only ones like `core.grant`).
3. Inject MCP configuration into the agent's config directory.
4. Copy the skill document to `{config-dir}/skills/elfiee/SKILL.md`.

### MCP Server

| Command | Description |
|---------|-------------|
| `elf serve [--port PORT] [--project PATH]` | Start the headless MCP SSE server |

This is the primary entry point for agent interaction. Default port is 47200. See [communication.md](communication.md) for protocol details.

### Workflow

| Command | Description |
|---------|-------------|
| `elf run <template> [--project PATH] [--port PORT]` | Execute a TOML workflow template |

Parses a declarative workflow, registers participants, seeds tasks and grants, then starts the MCP server. See [agent-building.md](agent-building.md) for template format.

### Permissions

| Command | Description |
|---------|-------------|
| `elf grant <editor> <capability> [--block BLOCK]` | Grant a capability to an editor |
| `elf revoke <editor> <capability> [--block BLOCK]` | Revoke a capability from an editor |

### Block Management

| Command | Description |
|---------|-------------|
| `elf block list [--project PATH]` | List all blocks |

## Agent Registration Flow

```text
elf register claude --name "Claude Code" --config-dir ~/.claude
    |
    v
1. Create Editor { name: "Claude Code", type: Bot }
    |
    v
2. Grant wildcard capabilities:
   (claude, document.*, *)
   (claude, task.*, *)
   (claude, session.*, *)
   (claude, core.create, *)
   (claude, core.link, *)
   ...
    |
    v
3. Write MCP config to ~/.claude/mcp.json:
   { "elfiee": { "url": "http://127.0.0.1:47200/sse" } }
    |
    v
4. Copy skill to ~/.claude/skills/elfiee/SKILL.md
```

## .elftypes Mapping

The `.elftypes` file maps file extensions to block types for `elf scan`:

```text
# Extension -> block type mapping
.md = document
.txt = document
.py = document
.rs = document
.toml = document
```

When scanning, files matching these patterns are automatically created as blocks of the corresponding type.

## config.toml

```toml
[project]
name = "my-project"

[git]
enabled = true
```

The config file controls project-level settings. Git integration determines whether `task.commit` triggers actual Git operations.

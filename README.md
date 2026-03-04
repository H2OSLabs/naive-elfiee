# Elfiee

**An event-sourced, capability-controlled block management system.**

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-322%2B-brightgreen.svg)]()

## What is Elfiee?

Elfiee is a **passive EventWeaver** -- a pure Rust MCP (Model Context Protocol) server for managing `.elf` projects. It records every decision as an immutable event, enforces fine-grained permissions before execution, and structures project content as a typed block DAG.

**Four Pillars:**

1. **Event Sourcing** -- All changes captured as an immutable event log (EAVT model) with complete history and time-travel queries.
2. **CBAC (Capability-Based Access Control)** -- Fine-grained permission model per editor/block/capability. No grant = no access.
3. **Block DAG** -- Typed blocks (document, task, session) linked by directed relations forming a structured content graph.
4. **Agent Template (Socialware)** -- Declarative role/permission/workflow definitions via TOML templates, instantiated as event streams.

**Design Principles:**

- Elfiee is **NOT an orchestrator** -- agents call Elfiee through MCP tools; Elfiee never spawns agents or sends commands to them.
- **Human and Agent editors are equal** (Socialware principle) -- both interact through the same identity model and permission system.

## Architecture

```
                    ┌─────────────────────────────────────────┐
                    │            Transport Layer               │
                    │                                          │
                    │  ┌─────────────┐    ┌─────────────┐     │
                    │  │  MCP Server │    │     CLI      │     │
                    │  │  (mcp/)     │    │   (cli/)     │     │
                    │  │  SSE :47200 │    │   elf binary │     │
                    │  └──────┬──────┘    └──────┬───────┘     │
                    └─────────┼──────────────────┼─────────────┘
                              │                  │
                              └────────┬─────────┘
                                       │
                    ┌──────────────────▼──────────────────────┐
                    │           Services Layer                 │
                    │                                          │
                    │  CBAC filtering + business logic         │
                    │  project | block | document | task       │
                    │  session | editor | event | grant        │
                    └──────────────────┬──────────────────────┘
                                       │
                    ┌──────────────────▼──────────────────────┐
                    │              Engine                      │
                    │                                          │
                    │  Actor model (tokio channels)            │
                    │  Event sourcing (SQLite eventstore.db)   │
                    │  State projection (in-memory replay)     │
                    │  Capability registry + Grants table      │
                    └─────────────────────────────────────────┘
```

Both the MCP SSE server and CLI route through a unified **services layer** that enforces CBAC permissions and business logic. The **engine** uses an actor model with one `EngineActor` per project, processing commands serially via tokio channels. Vector clocks enable conflict detection across concurrent editors.

## Quick Start

### Build

```bash
cargo build --release
```

The `elf` binary will be at `target/release/elf`.

### Initialize a Project

```bash
elf init /path/to/project
```

Creates a `.elf/` directory with `eventstore.db`, `cache.db`, and `config.toml`.

### Register an Agent

```bash
elf register claude --project /path/to/project
```

Creates an editor, grants default capabilities, injects MCP configuration into the agent's config directory, and writes the skill guide. Supported agent types: `claude`, `openclaw`, `custom`.

### Scan Project Files

```bash
elf scan --project /path/to/project
```

Discovers project files (respecting `.elfignore` and `.gitignore`) and creates document blocks for each, inferring block types from `.elftypes` mappings.

### Start the MCP Server

```bash
elf serve --port 47200 --project /path/to/project
```

Starts the MCP SSE server. Agents connect and follow this workflow:

1. `elfiee_auth` -- Authenticate with an editor_id (returns skill guide)
2. `elfiee_open` -- Open a project
3. Use block/document/task/session tools to read and write
4. `elfiee_close` -- Close project when done

### Check Project Status

```bash
elf status /path/to/project
```

Displays block counts, editor list, event statistics, and permission summary.

## .elf Project Format

`.elf/` is a directory (analogous to `.git/`), not an archive:

```
project/
├── .elf/
│   ├── eventstore.db         # Canonical event log (SQLite) -- source of truth
│   ├── cache.db              # Block snapshot cache for fast replay
│   ├── config.toml           # Project configuration
│   └── templates/
│       ├── skills/
│       │   └── default.md    # Default agent skill document
│       └── workflows/
│           └── *.toml        # Socialware workflow templates
├── src/                      # Your project files
└── ...
```

Events are written directly to SQLite on every operation. There is no explicit "save" step. Git handles project-level versioning alongside `.elf/`.

## Block Types

| Type | Capabilities | Purpose |
|------|-------------|---------|
| **document** | `document.write`, `document.read` | File-backed content blocks. Each document maps to a project file path. |
| **task** | `task.write`, `task.read`, `task.commit` | Structured work items with status tracking (pending, in_progress, completed, cancelled). |
| **session** | `session.append`, `session.read` | Append-only activity logs for commands, outputs, and agent observations. |

All block types inherit core capabilities: `core.create`, `core.write`, `core.link`, `core.unlink`, `core.delete`.

Blocks are connected via named relations (e.g., `implement`), forming a directed acyclic graph that represents the structural relationships within a project.

## MCP Tools

Elfiee exposes **18 MCP tools** over SSE transport (default port 47200):

| Tool | Description |
|------|-------------|
| `elfiee_auth` | Authenticate this MCP connection with an editor_id. Returns skill guide. |
| `elfiee_open` | Open an .elf project (creates if it does not exist). |
| `elfiee_close` | Close a project and release resources. |
| `elfiee_file_list` | List all currently open projects with block counts. |
| `elfiee_block_list` | List blocks in a project (CBAC filtered). |
| `elfiee_block_get` | Get full block details including contents, relations, and metadata. |
| `elfiee_block_create` | Create a new block (document, task, or session). |
| `elfiee_block_delete` | Soft-delete a block (history preserved in event store). |
| `elfiee_block_rename` | Rename a block. |
| `elfiee_block_link` | Add a directed relation between two blocks. |
| `elfiee_block_unlink` | Remove a relation between two blocks. |
| `elfiee_grant` | Grant a capability to an editor on a specific block. |
| `elfiee_revoke` | Revoke a previously granted capability. |
| `elfiee_editor_create` | Create a new editor in the project. |
| `elfiee_editor_delete` | Delete an editor from the project. |
| `elfiee_block_history` | Get the full event history for a block (requires read permission). |
| `elfiee_state_at_event` | Time-travel: get block state at a specific event (replay up to that point). |
| `elfiee_exec` | Execute any registered capability by name with a custom payload. |

### MCP Resources (8 types)

| Resource URI | Description |
|-------------|-------------|
| `elfiee://files` | List of currently open projects |
| `elfiee://{project}/blocks` | All blocks in a project |
| `elfiee://{project}/block/{block_id}` | Single block with full content |
| `elfiee://{project}/grants` | Permission grants table |
| `elfiee://{project}/events` | Complete event log |
| `elfiee://{project}/editors` | Editor list |
| `elfiee://{project}/my-tasks` | Tasks assigned to the authenticated editor |
| `elfiee://{project}/my-grants` | Permissions granted to the authenticated editor |

## CLI Reference

The `elf` binary provides **14 subcommands** for local project management:

```
elf init [project]                              # Initialize .elf/ directory
elf register <type> [--name] [--project]        # Register an agent
elf unregister <editor_id> [--project]          # Unregister an agent
elf serve [--port 47200] [--project]            # Start MCP SSE server
elf run <template> [--project] [--port]         # Run a Socialware workflow
elf status [project]                            # Show project status
elf scan [file] [--project]                     # Scan files into document blocks
elf block list [--project]                      # List all blocks
elf block get <block> [--project]               # Get full block details
elf event list [--project]                      # List all events (CBAC filtered)
elf event history <block> [--project]           # Block event history
elf event at <block> <event_id> [--project]     # Time-travel: state at event
elf grant <editor> <cap> [block] [--project]    # Grant a capability
elf revoke <editor> <cap> [block] [--project]   # Revoke a capability
```

### Usage Examples

```bash
# Initialize and set up a project
elf init .
elf register claude --project . --port 47200
elf scan --project .

# Inspect project state
elf status .
elf block list --project .
elf event list --project .

# Manage permissions
elf grant <editor_id> document.write "*" --project .
elf revoke <editor_id> task.commit "*" --project .

# View block history and time-travel
elf event history my-block --project .
elf event at my-block <event_id> --project .

# Run a workflow template
elf run tdd-flow --project . --port 47200

# Start the server for agent connections
elf serve --port 47200 --project .
```

## Development

### Build and Test

```bash
# Run all tests (322+ tests: unit, integration, doc-tests)
cargo test

# Run clippy linter (must pass with zero warnings)
cargo clippy

# Format code
cargo fmt --all

# Build the CLI binary
cargo build --bin elf

# Run MCP server in development
cargo run --bin elf -- serve --port 47200
```

### Project Structure

```
elfiee/
├── Cargo.toml               # Workspace root
├── src/
│   ├── lib.rs               # Library root (elfiee_lib)
│   ├── bin/elf.rs            # CLI binary entry point
│   ├── cli/                  # CLI subcommand implementations
│   ├── mcp/                  # MCP SSE server (server.rs, transport.rs)
│   ├── services/             # Business logic + CBAC filtering
│   ├── engine/               # Actor model + event sourcing
│   │   ├── actor.rs          # EngineActor (command processing loop)
│   │   ├── manager.rs        # EngineManager (multi-project lifecycle)
│   │   ├── state.rs          # StateProjector (event replay)
│   │   ├── event_store.rs    # SQLite event persistence
│   │   └── cache_store.rs    # Block snapshot cache
│   ├── models/               # Data models (Block, Editor, Event, Grant)
│   ├── capabilities/         # CBAC system (registry, grants, core, builtins)
│   ├── extensions/           # Block-type extensions (document, task, session)
│   ├── elf_project/          # .elf/ directory management + config
│   └── utils/                # Block type inference, path validation, time
├── tests/                    # Integration tests
├── templates/                # Workflow and skill templates
├── capability-macros/        # Proc-macro crate for capability registration
├── .elfignore                # Default ignore patterns for elf scan
├── .elftypes                 # File extension to block type mapping
└── docs/                     # Documentation (see docs/README.md)
```

### Key Crates

| Crate | Purpose |
|-------|---------|
| `sqlx` | SQLite interface for event store and cache |
| `rmcp` | MCP server implementation (SSE transport) |
| `tokio` | Async runtime for the actor model |
| `clap` | CLI argument parsing |
| `axum` | HTTP server underpinning SSE transport |
| `serde` / `serde_json` | JSON serialization for events and payloads |
| `dashmap` | Concurrent hashmap for engine manager |
| `uuid` | ID generation for blocks, editors, and events |
| `ignore` | .gitignore/.elfignore-aware file scanning |
| `schemars` | JSON Schema generation for MCP tool inputs |

### Contributing

Do not make modifications directly on `main` or `dev` branches. Always create a new branch and open a PR to merge into `dev`.

### Git Hooks

To ensure each commit runs `make fmt` automatically, the repository ships with a local pre-commit hook:

1. Run this once to enable the hooks directory:
   ```bash
   git config core.hooksPath .githooks
   ```
2. After that, every `git commit` will invoke `make fmt`. If formatting adjusts files, the hook prints a reminder so you can review, stage the changes, and re-run the commit.

The hook temporarily stashes unstaged changes (using `git stash --keep-index` semantics) so partially staged files are formatted safely. Your workspace is restored at the end of the hook run. If Git reports a conflict while restoring, manually run `git stash pop` to recover the saved changes.

## Documentation

See [docs/README.md](docs/README.md) for the full documentation index, including:

- **Architecture concepts** -- System design from data models to agent workflows
- **Developer guides** -- Extension development, CLI reference, MCP tools reference
- **Plans** -- Completed and active refactoring plans
- **Legacy** -- Historical Phase 1 and Phase 2 documentation

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

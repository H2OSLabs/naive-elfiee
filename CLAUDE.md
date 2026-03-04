# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## CRITICAL: .elf/ File Rules

**NEVER** use filesystem commands (cat, ls, rm, echo, etc.) on `.elf/` directory contents.
All `.elf/` mutations MUST go through Elfiee MCP tools or the `elf` CLI.
Violating this rule breaks event sourcing integrity and CBAC security.

## Project Overview

**Elfiee** is a **passive EventWeaver** — an event-sourced, capability-controlled block management system. The four core pillars:

1. **Event Sourcing**: All changes captured as immutable event log with complete history
2. **CBAC (Capability-Based Access Control)**: Fine-grained permission model per editor/block/capability
3. **Block DAG**: Typed blocks (document, task, session) linked by `implement` relations
4. **Agent Template**: Declarative role/permission/workflow definitions (Socialware)

Elfiee is a **pure passive MCP Server (EventWeaver)** — it does NOT orchestrate agents. Agents call Elfiee through MCP tools; Elfiee never spawns agents or sends commands to them. Human and Agent editors are equal (Socialware principle).

## Technology Stack

**Headless backend**: `elf` CLI + MCP SSE server.

- **Language**: Rust
- **Communication**:
  - **MCP SSE**: Primary external protocol (`elf serve` on port 47200), all agents connect here
  - **CLI**: `elf` binary for terminal operations (init, register, scan, grant, revoke, etc.)

## Architecture Overview

### Two-Layer Architecture

Both transport layers route through a unified **services** layer:

```
┌─────────────┐  ┌─────────────┐
│  MCP Server │  │    CLI       │
│  (mcp/)     │  │  (cli/)      │
└──────┬──────┘  └──────┬──────┘
       │                │
       └────────┬───────┘
                │
         ┌──────▼──────┐
         │  Services   │  ← CBAC filtering + business logic
         │ (services/) │
         └──────┬──────┘
                │
         ┌──────▼──────┐
         │   Engine    │  ← Actor model, event sourcing
         │ (engine/)   │
         └─────────────┘
```

### Services Layer (`src/services/`)

Encapsulates CBAC filtering and business logic. All read operations filter by editor permissions:

| Module | Purpose |
|--------|---------|
| `project` | Open/close/list projects, seed bootstrap events |
| `block` | List/get/rename blocks (CBAC filtered) |
| `document` | Read/write document content (type-checked + CBAC) |
| `task` | Read/write/commit task blocks (type-checked + CBAC) |
| `session` | Read/append session entries (type-checked + CBAC) |
| `editor` | List/get editor info |
| `event` | List events, block history, state-at-event (CBAC) |
| `grant` | List/grant/revoke permissions |

### Core Data Models

The system uses strongly-typed entities:

- **Block**: Fundamental content unit with `block_id`, `block_type` (document/task/session), `contents` (JSON), and `children` (relation graph)
- **Editor**: User or agent with `editor_id` — human and bot types are equal
- **Capability**: Defines actions with `certificator` (authorization) and `handler` (execution)
- **CapabilitiesGrant**: CBAC table mapping `editor_id` + `cap_id` + `block_id`

### Event Structure (EAVT)

All state changes stored as Events in `eventstore.db`:

- **Entity**: ID of changed entity (block_id/editor_id)
- **Attribute**: Change descriptor `"{editor_id}/{cap_id}"`
- **Value**: JSON payload with **event modes**: `full`, `delta`, `ref`, `append`
- **Timestamp**: Vector clock `Record<editor_id, transaction_count>` for conflict resolution

### .elf Project Format

`.elf/` is a directory (like `.git/`), **not a ZIP archive**:

```
project/
├── .elf/                    # Elfiee project directory
│   ├── eventstore.db        # Canonical event log (SQLite) — source of truth
│   ├── cache.db             # CacheStore: block snapshots for fast replay
│   ├── config.toml          # Project config (Git mode, etc.)
│   └── templates/
│       └── skills/
│           └── default.md   # Default agent skill document
├── src/                     # Project source files
└── ...
```

`save` is a no-op — events are written directly to SQLite. Git handles versioning.

### Engine (Actor Model)

Each `.elf` project has a dedicated **EngineActor** processing commands serially via tokio channels:

1. Receive `Command` from mailbox (mpsc channel)
2. Load `Capability`, `Block`, `Editor` from state projection
3. **Authorize**: Call `Capability.certificator()` → reject if fail
4. **Execute**: Call `Capability.handler()` → produce events
5. **Conflict Check**: Compare vector clocks → reject if stale
6. **Commit**: Atomic append to `eventstore.db`
7. **Project**: Apply events to in-memory state
8. **Notify**: Broadcast via tokio broadcast channel

**EngineHandle** wraps the mpsc channel with async API methods (`process_command`, `get_block`, `get_all_blocks`, `get_all_editors`, `get_all_grants`, `get_editor_grants`, etc.).

### Capabilities

**Built-in (core)**:
- `core.create` — Create new blocks
- `core.write` — Update block metadata (name, description)
- `core.link` / `core.unlink` — DAG relations
- `core.delete` — Soft-delete blocks
- `core.grant` / `core.revoke` — Permission management
- `editor.create` / `editor.delete` — Editor lifecycle

**Extension capabilities** (per block type):
- `document.write` / `document.read` — Document content
- `task.write` / `task.read` / `task.commit` — Task management
- `session.append` / `session.read` — Session logging

**Authorization model**: Owner always authorized → Grant-based → Wildcard grants (`block_id = "*"`)

### MCP Server

Per-connection identity model:
- Each MCP connection gets an independent `ElfieeMcpServer` instance
- `elfiee_auth` tool authenticates with `editor_id`
- `elfiee_open` / `elfiee_close` manage project lifecycle
- All operations go through services layer with CBAC

## File Organization

```
elfiee/
├── CLAUDE.md                # This file
├── Cargo.toml               # Rust project manifest
├── src/                     # Rust source code
│   ├── lib.rs               # Library root
│   ├── bin/
│   │   └── elf.rs           # CLI binary entry point
│   ├── engine/              # Core engine
│   │   ├── actor.rs         # EngineActor (command processing)
│   │   ├── manager.rs       # EngineManager (multi-project)
│   │   ├── state.rs         # StateProjector (event replay)
│   │   ├── event_store.rs   # SQLite event persistence
│   │   └── cache_store.rs   # Block snapshot cache
│   ├── models/              # Data models
│   │   ├── block.rs         # Block entity
│   │   ├── editor.rs        # Editor entity
│   │   ├── event.rs         # Event (with EventMode)
│   │   ├── grant.rs         # Grant entry
│   │   └── payloads.rs      # Core payload types
│   ├── capabilities/        # CBAC system
│   │   ├── registry.rs      # CapabilityRegistry
│   │   ├── grants.rs        # GrantsTable
│   │   ├── core.rs          # Core capability definitions
│   │   └── builtins/        # Built-in capability handlers
│   ├── extensions/          # Block-type extensions
│   │   ├── document/        # document.write, document.read
│   │   ├── task/            # task.write, task.read, task.commit
│   │   └── session/         # session.append, session.read
│   ├── services/            # Business logic + CBAC filtering
│   │   ├── project.rs       # Project open/close/list
│   │   ├── block.rs         # Block CRUD (CBAC filtered)
│   │   ├── document.rs      # Document read/write
│   │   ├── task.rs          # Task read/write/commit
│   │   ├── session.rs       # Session read/append
│   │   ├── editor.rs        # Editor info
│   │   ├── event.rs         # Event history
│   │   └── grant.rs         # Permission management
│   ├── mcp/                 # MCP SSE server
│   │   ├── server.rs        # ElfieeMcpServer (per-connection)
│   │   ├── transport.rs     # SSE transport layer
│   │   └── mod.rs           # Server startup
│   ├── cli/                 # CLI subcommands
│   │   ├── init.rs          # elf init
│   │   ├── register.rs      # elf register
│   │   ├── scan.rs          # elf scan
│   │   ├── run.rs           # elf run <template>
│   │   ├── status.rs        # elf status
│   │   ├── block.rs         # elf block list
│   │   ├── grant.rs         # elf grant
│   │   ├── revoke.rs        # elf revoke
│   │   └── resolve.rs       # Block name/id resolution
│   ├── elf_project/         # .elf/ directory management
│   │   ├── mod.rs           # ElfProject (init/open/skill)
│   │   └── config.rs        # ProjectConfig (TOML)
│   ├── config.rs            # System config (editor ID)
│   ├── state.rs             # AppState (shared state)
│   └── utils/               # Utilities
│       ├── block_type_inference.rs  # .elftypes parser
│       ├── path_validator.rs
│       └── time.rs
├── tests/                   # Integration tests
├── templates/               # Workflow/Skill templates (TOML/MD)
├── capability-macros/       # Proc-macro crate for capabilities
├── .elfignore               # Default ignore patterns for scan
├── .elftypes                # Extension → block type mapping
└── docs/                    # Documentation
    ├── concepts/            # Architecture docs (9 core docs)
    ├── guides/              # Developer guides
    ├── plans/               # Planning documents
    └── legacy/              # Historical Phase 1/2 docs
```

## Key Crates

- `sqlx` — SQLite interface for event store
- `serde`, `serde_json` — JSON serialization
- `uuid` — ID generation
- `rmcp` — MCP server implementation (SSE transport)
- `tokio` — Async runtime for actor model
- `dashmap` — Concurrent hashmap for engine manager
- `clap` — CLI argument parsing
- `ignore` — .gitignore/.elfignore-aware file scanning

## Development Commands

```bash
# Run tests
cargo test

# Run clippy
cargo clippy

# Run headless MCP server
cargo run --bin elf -- serve --port 47200

# Initialize a project
cargo run --bin elf -- init /path/to/project

# Register an agent
cargo run --bin elf -- register openclaw --project /path/to/project

# Scan files into Elfiee
cargo run --bin elf -- scan --project /path/to/project
```

## Capability Payload Types (CRITICAL)

**HARD RULE**: For every capability that accepts input, define a typed Rust payload struct with `#[derive(Serialize, Deserialize)]`. NEVER use manual JSON parsing.

**Payload Location Rules**:
- **Extension-specific** payloads: Define in `src/extensions/{extension_name}/mod.rs`
- **Core** payloads: Define in `src/models/payloads.rs`

## Documentation

- **Architecture concepts**: `docs/concepts/` (9 core architecture docs)
- **Developer guides**: `docs/guides/` (extension development, CLI reference, MCP tools reference)
- **Plans**: `docs/plans/` (completed and in-progress plans)
- **Legacy docs**: `docs/legacy/` (historical Phase 1/2 docs, changelogs)

**For contributors**: Do not make modifications directly on main/dev branches. Always create a new branch and open a PR to merge to dev.

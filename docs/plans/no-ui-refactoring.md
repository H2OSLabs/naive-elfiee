# No-UI Refactoring: Pure Backend Project

**Status: COMPLETED**

## Overview

Elfiee was originally built as a Tauri desktop application with a React/TypeScript frontend and a Rust backend under `src-tauri/`. This refactoring removed all GUI-related code and restructured the project into a standard pure Rust project. Elfiee is now a headless backend exposing an MCP SSE server and a CLI binary.

## What Was Removed

### Frontend Layer
- **React frontend** (`src/` directory): App.tsx, components, pages, and all UI code
- **Build tooling**: Vite, Tailwind CSS, PostCSS, Prettier
- **Package management**: `package.json`, `pnpm-lock.yaml`, `node_modules/`
- **TypeScript configuration**: `tsconfig.json`, `tsconfig.node.json`
- **Auto-generated bindings**: `src/bindings.ts` (tauri-specta output)
- **Frontend documentation**: `docs/guides/FRONTEND_DEVELOPMENT.md`

### Tauri Desktop Framework
- **Tauri IPC commands** (`src-tauri/src/commands/`): Thin wrappers that called the engine via Tauri's invoke system
- **Tauri events** (`src-tauri/src/events.rs`): Frontend event emission layer
- **Tauri dependencies** in `Cargo.toml`: `tauri`, `tauri-specta`, `specta`
- **Tauri configuration**: `tauri.conf.json`, capability files, icon assets

### Deleted Extensions
- **Terminal extension** (`extensions/terminal/`): PTY management, terminal init/execute/save/close. Terminal operations are now the agent's responsibility.
- **Agent extension** (`extensions/agent/`): Agent lifecycle and MCP config injection. Replaced by the CLI `elf register` / `elf unregister` commands.
- **Code extension** (`extensions/code/`): Code read/write operations. Merged into the document extension.
- **Directory extension** (`extensions/directory/`): Directory management, fs scanning, import/export, elf metadata. Directory scanning is now handled by `elf scan`; metadata moved to the elf_project module.

### Other Removals
- **Snapshot system** (`utils/snapshot.rs`): Rendered markdown preview of all blocks. Replaced by MCP resources and CLI queries.
- **Archive format** (`elf/archive.rs`): ZIP-based `.elf` file packaging. The `.elf/` format is now a directory (like `.git/`).
- **Change type capability** (`builtins/change_type.rs`): Block type conversion. Removed as an unnecessary complexity.

## What Was Kept and Promoted

### Core Engine (unchanged)
- **Actor model** (`engine/actor.rs`): EngineActor processes commands serially via tokio channels
- **State projector** (`engine/state.rs`): Event replay to reconstruct in-memory state
- **Event store** (`engine/event_store.rs`): SQLite-backed append-only event log
- **Cache store** (`engine/cache_store.rs`): Block snapshot cache for fast replay
- **Engine manager** (`engine/manager.rs`): Multi-project engine lifecycle

### Services Layer (unchanged)
All CBAC-filtered business logic modules were preserved:
- `project`, `block`, `document`, `task`, `session`, `editor`, `event`, `grant`

### Communication Layer (promoted)
- **MCP SSE server** (`mcp/`): 18 tools and 8 resource types. Now the primary external protocol.
- **CLI** (`cli/` + `bin/elf.rs`): 14 subcommands for terminal operations. Now the primary local interface.

### Extensions (kept 3 of 7)
- **Document extension**: `document.write`, `document.read`
- **Task extension**: `task.write`, `task.read`, `task.commit`
- **Session extension**: `session.append`, `session.read`

### Capabilities System (unchanged)
- **Registry** (`capabilities/registry.rs`): Capability registration and lookup
- **Grants table** (`capabilities/grants.rs`): CBAC permission management
- **Core capabilities** (`capabilities/core.rs`): `core.create`, `core.write`, `core.link`, `core.unlink`, `core.delete`, `core.grant`, `core.revoke`, `editor.create`, `editor.delete`

## Architecture Changes

### Before (Tauri-era)
```
elfiee/
├── src/                     # React frontend
│   ├── bindings.ts          # Auto-generated Tauri IPC bindings
│   ├── components/
│   └── ...
├── src-tauri/               # Rust backend
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # Tauri app entry
│       ├── commands/        # Tauri IPC command handlers
│       ├── engine/
│       └── ...
├── package.json
├── vite.config.ts
└── ...
```

Three transport layers: Tauri IPC, MCP SSE, CLI

### After (Pure Rust)
```
elfiee/
├── Cargo.toml               # Root crate manifest
├── src/
│   ├── lib.rs               # Library root
│   ├── bin/elf.rs            # CLI entry point
│   ├── cli/                  # CLI subcommands
│   ├── mcp/                  # MCP SSE server
│   ├── services/             # Business logic + CBAC
│   ├── engine/               # Actor model + event sourcing
│   ├── models/               # Data models
│   ├── capabilities/         # CBAC system
│   ├── extensions/           # document, task, session
│   ├── elf_project/          # .elf/ directory management
│   └── utils/
├── tests/                    # Integration tests
├── templates/                # Workflow and skill templates
└── capability-macros/        # Proc-macro crate
```

Two transport layers: MCP SSE, CLI. Both route through the unified services layer.

## Step-by-Step Process

1. **Delete frontend files**: Removed `src/` (React), `node_modules/`, `dist/`, `.cursor/`, 11 frontend config files, and frontend documentation.
2. **Promote src-tauri/**: Moved `Cargo.toml`, `Cargo.lock`, `src/`, `tests/`, `templates/`, `capability-macros/` from `src-tauri/` to the project root. Deleted `src-tauri/`.
3. **Update configuration**: Rewrote `Makefile` for pure Rust, simplified GitHub Actions CI (removed pnpm/Node.js/webkit2gtk), cleaned `.gitignore`.
4. **Update documentation**: Rewrote `CLAUDE.md` to reflect the new two-layer architecture, removed all frontend references.
5. **Verification**: All tests pass, zero clippy warnings, CLI builds successfully.

## Results

- **322 tests passing** (287 unit + 8 project + 4 block-permission + 10 service + 12 relation + 1 doc-test)
- **0 clippy warnings**
- **Clean root directory** with standard Rust project layout
- **No broken imports**: `include_str!` paths preserved because relative relationships between `src/` and `templates/` were maintained during the move

## Change Summary

| Action | Target |
|--------|--------|
| **Deleted** | React frontend (`src/`), `node_modules/`, `dist/`, `.cursor/`, 11 frontend config files, `FRONTEND_DEVELOPMENT.md` |
| **Deleted** | Terminal, Agent, Code, Directory extensions; Snapshot system; Archive format |
| **Moved** | `src-tauri/*` to project root, then deleted `src-tauri/` |
| **Modified** | `Makefile`, `.github/workflows/pr-tests.yml`, `.gitignore`, `CLAUDE.md` |
| **Preserved** | Engine, Services, MCP, CLI, 3 extensions (document/task/session), Capabilities, Models |

# Architecture Overview

Elfiee is a **passive EventWeaver** -- an event-sourced, capability-controlled block management system. It serves as a pure MCP Server that agents call into; Elfiee never orchestrates agents or sends commands outward.

## Four Pillars

| Pillar | Role |
|--------|------|
| **Event Sourcing** | All state changes are immutable events in SQLite. The event log is the single source of truth. |
| **CBAC** | Capability-Based Access Control. Every operation is gated by a `(editor, capability, block)` grant triple. |
| **Block DAG** | Typed blocks (document, task, session) connected by `implement` relations forming a directed acyclic graph. |
| **Agent Template (Socialware)** | Declarative TOML workflows defining participants, permissions, and task assignments. Human and bot editors are equal. |

## Two-Layer Transport

Elfiee exposes two transport layers that both route through a shared services layer:

```text
+--------------+  +-----------+
|  MCP Server  |  |    CLI    |
|  (mcp/)      |  |  (cli/)   |
+------+-------+  +-----+-----+
       |                 |
       +--------+--------+
                |
         +------v------+
         |  Services   |  <-- CBAC filtering + business logic
         | (services/) |
         +------+------+
                |
         +------v------+
         |   Engine    |  <-- Actor model, event sourcing
         | (engine/)   |
         +-------------+
```

- **MCP SSE Server** (`elf serve`): Primary protocol. Agents connect via SSE on port 47200.
- **CLI** (`elf init`, `elf register`, etc.): Human-facing terminal commands.

Both call into the **Services** layer, which enforces CBAC and delegates to the **Engine**.

## Services Layer

Every read operation is filtered by editor permissions. Every write operation passes through capability authorization.

| Service | Purpose |
|---------|---------|
| `project` | Open/close/list projects, seed bootstrap events |
| `block` | List/get/rename blocks (CBAC filtered) |
| `document` | Read/write document content (type-checked + CBAC) |
| `task` | Read/write/commit task blocks (type-checked + CBAC) |
| `session` | Read/append session entries (type-checked + CBAC) |
| `editor` | List/get editor info |
| `event` | List events, block history, state-at-event (CBAC) |
| `grant` | List/grant/revoke permissions |

## Engine

The engine uses the **Actor model**: one `.elf` project = one `EngineActor` processing commands serially via a tokio mpsc channel. This eliminates data races without locks.

See [engine.md](engine.md) for the full pipeline.

## Key Design Principles

1. **Passive only** -- Agents call Elfiee; Elfiee never calls agents.
2. **Communication is unidirectional** -- Agent -> Elfiee, never the reverse.
3. **Human = Bot** -- Both are `Editor` entities with the same identity model (Socialware).
4. **No I/O in handlers** -- Capability handlers are pure functions producing events.
5. **Event log is canonical** -- Current state is always derivable by replaying events.

## Sub-Document Index

| Document | Topic |
|----------|-------|
| [data-model.md](data-model.md) | Core entities: Block, Event, Editor, Command, Grant, Capability |
| [event-system.md](event-system.md) | EAVT model, content modes, snapshots, Git relationship |
| [cbac.md](cbac.md) | Capability-Based Access Control, authorization flow |
| [elf-format.md](elf-format.md) | `.elf/` project directory structure, CLI commands |
| [engine.md](engine.md) | Actor model, command pipeline, StateProjector, CacheStore |
| [communication.md](communication.md) | MCP SSE protocol, per-connection identity |
| [extension-system.md](extension-system.md) | Kernel vs extensions, built-in extensions |
| [agent-building.md](agent-building.md) | Editor identity, task state machine, workflows, skills |

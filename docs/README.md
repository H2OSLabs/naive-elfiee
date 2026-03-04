# Elfiee Documentation

This directory contains all documentation for the Elfiee project.

## Sections

### [concepts/](concepts/)

Architecture concept documents covering the full system design (9 core docs):

- **[architecture-overview](concepts/architecture-overview.md)** -- System overview and dependency graph
- **[data-model](concepts/data-model.md)** -- Block, Editor, Capability, Grant entities
- **[event-system](concepts/event-system.md)** -- EAVT event structure and content modes
- **[cbac](concepts/cbac.md)** -- Capability-Based Access Control model
- **[elf-format](concepts/elf-format.md)** -- `.elf/` project directory format
- **[extension-system](concepts/extension-system.md)** -- Extension interface and block-type extensions
- **[engine](concepts/engine.md)** -- Actor model engine and command processing
- **[communication](concepts/communication.md)** -- MCP SSE transport and CLI architecture
- **[agent-building](concepts/agent-building.md)** -- Agent registration, skills, and workflows

### [guides/](guides/)

Developer guides for working with the codebase:

- **extension-development.md** -- How to create new block-type extensions (payload schemas, capability registration, handler/certificator patterns)
- **cli-reference.md** -- Full CLI command reference with usage examples for all `elf` subcommands
- **mcp-tools-reference.md** -- Detailed reference for all 18 MCP tools and 8 resource types, including input schemas and example payloads

### [plans/](plans/)

Planning documents for current and completed work:

- **[no-ui-refactoring](plans/no-ui-refactoring.md)** -- Completed: removing Tauri frontend and promoting to a pure Rust project

### [legacy/](legacy/)

Historical documentation from Phase 1 and Phase 2 development:

- **legacy/concepts/** -- Original Phase 1 architecture concepts
- **legacy/plans/** -- Original implementation plans (parts 1-7)
- **legacy/guides/** -- Legacy developer guides (EXTENSION_DEVELOPMENT.md, DATA_FLOW_STANDARD.md)
- **legacy/mvp/** -- MVP phase documentation (phase 1 and phase 2)
- **legacy/mvp/frame/changelogs/** -- Refactoring changelogs (L0-L5)
- **legacy/analyze/** -- Code analysis and review documents
- **legacy/testing/** -- Concurrency testing guide

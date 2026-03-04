# Elfiee Documentation

Welcome to the Elfiee documentation! This directory contains all the technical documentation for the Elfiee project.

## Documentation Structure

### 📘 [concepts/](concepts/) - Core Concepts

Understanding the fundamental architecture and design philosophy of Elfiee.

- **[ARCHITECTURE_OVERVIEW.md](concepts/ARCHITECTURE_OVERVIEW.md)** - High-level architecture overview
  - Three core principles (Block-based, Event Sourcing, Capability-driven)
  - Core entities and command processing flow
  - .elf file format structure

- **[ENGINE_CONCEPTS.md](concepts/ENGINE_CONCEPTS.md)** - Engine design philosophy
  - Why Actor Model for concurrency
  - Core engine components (EngineManager, ElfileEngineActor, StateProjector, EventStore)
  - Command processing workflow
  - Event sourcing and state projection

### 📗 [guides/](guides/) - Development Guides

Practical guides for developers working with Elfiee.

- **[EXTENSION_DEVELOPMENT.md](guides/EXTENSION_DEVELOPMENT.md)** - Extension development guide
  - Creating custom block types
  - Defining capabilities with `#[capability]` macro
  - Payload types and JSON schemas
  - Authorization and CBAC
  - Complete example: Markdown extension

- **[FRONTEND_DEVELOPMENT.md](guides/FRONTEND_DEVELOPMENT.md)** - Frontend development guide
  - Working with Tauri Specta v2 for type-safe bindings
  - Auto-generated TypeScript types from Rust
  - Strongly-typed payload system
  - Using the Tauri client wrapper
  - Adding new backend commands

### 📙 [plans/](plans/) - Development Planning

Project planning documents and implementation status.

- **[STATUS.md](plans/STATUS.md)** - Current implementation status
  - Progress overview (100% + post-MVP enhancements)
  - Completed parts 1-6 details
  - Test statistics (60 tests passing)
  - Architecture and dependencies

- **[IMPLEMENTATION_PLAN.md](plans/IMPLEMENTATION_PLAN.md)** - Original implementation plan
  - Six-part MVP roadmap
  - Timeline and milestones

- **[engine-architecture.md](plans/engine-architecture.md)** - Detailed engine architecture
  - Design decisions and rationale
  - Concurrency model
  - Performance characteristics
  - Trade-offs analysis

**Part-specific documentation:**
- [part1-core-models.md](plans/part1-core-models.md) - Core data models
- [part2-event-structure.md](plans/part2-event-structure.md) - EAVT event schema
- [part3-elf-file-format.md](plans/part3-elf-file-format.md) - ZIP-based file format
- [part4-extension-interface.md](plans/part4-extension-interface.md) - Extension system and CBAC
- [part5-elfile-engine.md](plans/part5-elfile-engine.md) - Actor-based engine
- [part5-completion-summary.md](plans/part5-completion-summary.md) - Part 5 summary
- [part6-tauri-app.md](plans/part6-tauri-app.md) - Tauri application interface
- [part7-content-schema-proposal.md](plans/part7-content-schema-proposal.md) - Future design proposals

## Recommended Reading Order

### For New Contributors

1. Start with **[ARCHITECTURE_OVERVIEW.md](concepts/ARCHITECTURE_OVERVIEW.md)** to understand the big picture
2. Read **[STATUS.md](plans/STATUS.md)** to see what's been implemented
3. Review **[ENGINE_CONCEPTS.md](concepts/ENGINE_CONCEPTS.md)** for engine design details
4. Choose a development guide based on your focus:
   - Backend extensions: **[EXTENSION_DEVELOPMENT.md](guides/EXTENSION_DEVELOPMENT.md)**
   - Frontend development: **[FRONTEND_DEVELOPMENT.md](guides/FRONTEND_DEVELOPMENT.md)**

### For Understanding Specific Topics

- **Event Sourcing**: [ENGINE_CONCEPTS.md](concepts/ENGINE_CONCEPTS.md) + [part2-event-structure.md](plans/part2-event-structure.md)
- **Actor Model**: [ENGINE_CONCEPTS.md](concepts/ENGINE_CONCEPTS.md) + [engine-architecture.md](plans/engine-architecture.md)
- **CBAC System**: [part4-extension-interface.md](plans/part4-extension-interface.md)
- **File Format**: [part3-elf-file-format.md](plans/part3-elf-file-format.md)
- **Type Safety**: [FRONTEND_DEVELOPMENT.md](guides/FRONTEND_DEVELOPMENT.md) (Capability Payload Types section)

## Quick Links

### Key Architectural Decisions

- **Why Actor Model?** → [ENGINE_CONCEPTS.md](concepts/ENGINE_CONCEPTS.md#1-为何采用-actor-模型-why-the-actor-model)
- **Why Event Sourcing?** → [engine-architecture.md](plans/engine-architecture.md#why-event-sourcing)
- **Why sqlx over rusqlite?** → [engine-architecture.md](plans/engine-architecture.md#why-sqlx-over-rusqlite)
- **Why Tauri Specta?** → [FRONTEND_DEVELOPMENT.md](guides/FRONTEND_DEVELOPMENT.md#overview)

### Implementation Details

- **Command Processing Flow** → [ENGINE_CONCEPTS.md](concepts/ENGINE_CONCEPTS.md#3-命令处理的完整流程)
- **State Projection** → [ENGINE_CONCEPTS.md](concepts/ENGINE_CONCEPTS.md#4-状态投影-state-projection)
- **Capability System** → [EXTENSION_DEVELOPMENT.md](guides/EXTENSION_DEVELOPMENT.md#defining-capabilities)
- **Payload Type System** → [FRONTEND_DEVELOPMENT.md](guides/FRONTEND_DEVELOPMENT.md#capability-payload-types)

### Current Status

- **Test Coverage**: 60 tests (100% passing)
- **MVP Status**: Complete + Post-MVP Enhancements
- **Latest Update**: 2025-10-28
- **See**: [STATUS.md](plans/STATUS.md)

## Contributing

When contributing to Elfiee:

1. **Read the guides** relevant to your contribution area
2. **Follow the patterns** established in existing code
3. **Write tests** for new functionality (TDD workflow)
4. **Use strongly-typed payloads** for new capabilities
5. **Update documentation** when making architectural changes

## Documentation Maintenance

This documentation is actively maintained. If you find errors or outdated information:

1. Check [STATUS.md](plans/STATUS.md) for the latest implementation status
2. Verify against the actual codebase in `src-tauri/src/`
3. Submit corrections via pull request

---

**Last Updated**: 2025-10-28
**Project Status**: MVP Complete + Type Safety Enhancements ✅

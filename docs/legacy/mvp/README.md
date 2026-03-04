# Elfiee MVP Technical Specification & Release Summary (v0.1.0)

This document provides a comprehensive overview of the first major milestone for Elfiee. It details the architectural decisions, functional implementations, and development disciplines established during the MVP phase, including the recent "Meiji Restoration" refactorings.

---

## 1. Core Architectural Pillars

### 1.1 Pure Event Sourcing (ES)
Elfiee is built on the principle that **Events are the Single Source of Truth**.
- **Event Store**: Every mutation is captured as an immutable event in a SQLite database (`_eventstore.db`) residing within the `.elf` (ZIP) container.
- **State Projection**: The current state is not stored; it is projected. The Backend Engine Actor replays the event log into an in-memory `StateProjector` to build the current view of blocks, editors, and grants.
- **Immutability**: Once an event is committed, it can never be changed or deleted, ensuring a perfect audit trail for both human and AI collaborators.

### 1.2 Flat Storage & Orthogonal Indexing
Unlike traditional editors that use nested JSON trees, Elfiee employs a **Flat Storage** model.
- **Object Space**: All Blocks exist as equals in a `HashMap<UUID, Block>`.
- **Structural Orthogonality**: 
    - **System View (VFS)**: Managed by `Directory` blocks which act as indices (mapping paths to IDs).
    - **User View (Knowledge Graph)**: Managed by semantic links in `block.children`.
- **Benefit**: This eliminates the "moving a folder breaks all links" problem and allows a single block to exist in multiple paths simultaneously (Hard-link semantics).

### 1.3 Capability-Based Access Control (CBAC)
Elfiee abandons traditional Role-Based Access Control (RBAC) in favor of **Capabilities**.
- **Granular Permissions**: Authorization is defined as a triplet: `(EditorID, CapabilityID, BlockID)`.
- **Dynamic Granting**: Permissions are granted via events (`core.grant`), allowing for fine-grained control (e.g., "Bot-A can write to Block-B but only read Block-C").
- **Backend Enforcement**: The Frontend is "untrusted." Every action must be authorized by the backend `certificator` before an event is generated.

---

## 2. Functional Modules

### 2.1 Dashboard & File Management
- **Container Format**: The `.elf` format is a ZIP archive containing a SQLite database for events and a temporary directory for assets.
- **Project Lifecycle**: Full support for creating, opening, renaming, and exporting projects.
- **Atomic Operations**: File saving ensures that the in-memory state and the on-disk SQLite database are synchronized and packaged correctly.

### 2.2 User & Identity System
- **Editor Identities**: Differentiates between `Human` and `Bot` (AI Agent) types.
- **Multi-user Workflow**: Supports real-time switching of the "Active Editor" in the UI, with all subsequent actions correctly attributed in the event log.
- **Persistent System Identity**: A global system editor ID is maintained in the user's local config for administrative tasks (like initializing the default outline).

### 2.3 Virtual File System (VFS) - Directory Extension
- **Reference Semantics**: Inspired by the Unix inode system. A directory entry is a pointer to a Block ID.
- **Reference vs. Ownership**: Deleting a file from a directory only removes the reference (dentry). The underlying Block is only eligible for Garbage Collection if it becomes unreachable from all roots.
- **Recursive Operations**: Renaming a folder recursively updates all virtual paths of its children without mutating the children blocks themselves.

### 2.4 MyST Markdown & Code Extensions
- **Professional Rendering**: Integrated `myst-parser` for scientific-grade Markdown, supporting directives, roles, and complex math.
- **Code Execution Flow**: Code blocks support multi-language syntax highlighting and a UI-to-Backend execution bridge (mocked in MVP, ready for Terminal Extension).
- **Projection Discipline**: The frontend handles display-layer issues like text encoding and "mojibake" (乱码) cleaning, ensuring the rendered output is always pristine.

### 2.5 Timeline & Time Travel
- **Dual-Clock System**:
    - **Vector Clocks**: Used for logic ordering and conflict detection in decentralized scenarios.
    - **Wall Clock (RFC 3339)**: All business timestamps are unified to UTC strings with timezone info for reliable UI display and auditing.
- **State Reconstruction**: The `get_state_at_event` command allows the UI to "travel back in time" by replaying events up to a specific point, creating a historical snapshot.
- **Restore Logic**: Users can restore a block to any historical state, which generates a new `write` event, maintaining the integrity of the event log.

---

## 3. Development Discipline & Quality

### 3.1 Frontend-Backend Boundaries
- **Logic Centricity**: All business rules (filename validation, duplicate checking, type inference) are implemented in Rust.
- **Passive Frontend**: The React application is a pure projection. It collects input and displays state but contains zero business logic. It handles backend errors via a standardized `Result<T, E>` bridge.

### 3.2 Store Access Protocol (Zustand)
To ensure reactivity and maintainability, the following store protocols are enforced:
1.  **Declarative (Hooks)**: Use selectors `useAppStore(state => ...)` for rendering to ensure UI updates.
2.  **Imperative (Actions)**: Use `useAppStore.getState().action()` inside `useEffect` or event handlers to avoid closure stale-traps and redundant re-renders.
3.  **Data Integrity**: Store state must mirror backend structures (e.g., using `Map` for file indices).

### 3.3 Testing Methodology
- **Backend (Rust)**: 192+ tests verify capability handlers, path validators, and actor concurrency.
- **Frontend (TS)**: 81+ tests using a standardized global mock provided in `setup.ts`. 
- **Hoisted Mocking**: Utilizes `vi.hoisted` to solve Vitest's module hoisting issues, ensuring that the Store instance used by components is the same one controlled by the test cases.

---

## 4. Recent Refactor Highlights ("Meiji Restoration")
- **Time Unification**: Migrated all timestamps from naive types to RFC 3339 strings.
- **Validation Consolidation**: Deleted redundant validation code from the frontend; the backend is now the sole validator.
- **Rebase Integrity**: Successfully merged the `feat/timeline` branch into the new flat-storage architecture while maintaining 100% test passing rate.

---
*Elfiee Team - December 29, 2025*

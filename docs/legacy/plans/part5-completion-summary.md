# Part 5: Elfile Engine - Completion Summary

**Status**: ✅ COMPLETED
**Date**: 2025-10-23
**Commit**: `f46db44`

## Overview

Successfully implemented the Actor Model-based command processor with async SQLite persistence, supporting multi-file and multi-user editing scenarios. The engine uses tokio for async runtime and sqlx for database operations.

## What Was Implemented

### 1. StateProjector (`src/engine/state.rs`)

**Purpose**: Project events into in-memory state

**Features**:
- Replays all events from database to rebuild current state
- Maintains HashMap of blocks, editors, and grants
- Tracks vector clock counts for conflict detection
- Integrates with GrantsTable for authorization
- Handles all capability types: create, link, unlink, delete, grant, revoke, write

**Key Methods**:
- `replay(&mut self, events: Vec<Event>)` - Batch replay all events
- `apply_event(&mut self, event: &Event)` - Process single event
- `get_block(&self, block_id: &str)` - Query block
- `get_editor_count(&self, editor_id: &str)` - Get vector clock count
- `has_conflict(&self, editor_id: &str, expected_count: u64)` - MVP conflict detection

**Tests**: 4 tests passing
- test_state_projector_create_block
- test_state_projector_delete_block
- test_state_projector_editor_count
- test_conflict_detection

### 2. ElfileEngineActor (`src/engine/actor.rs`)

**Purpose**: Actor that processes commands for a single .elf file

**Architecture**:
- **Message-based**: Uses tokio mpsc channels for mailbox
- **Serial processing**: Commands processed one at a time per file
- **Async persistence**: All database operations are async with sqlx

**Message Types**:
- `ProcessCommand` - Execute a command and return events
- `GetBlock` - Query a specific block
- `GetAllBlocks` - Get all blocks in the file
- `Shutdown` - Gracefully stop the actor

**Command Processing Flow**:
1. Get capability handler from registry
2. Get block (None for create, Some for others)
3. Check authorization (owner or grant)
4. Execute handler
5. Update vector clock
6. Check for conflicts (MVP: simple count check)
7. Persist events to database
8. Apply events to state

**Key Components**:
- `ElfileEngineActor` - The actor itself
- `EngineHandle` - Async API for communicating with actor
- `spawn_engine()` - Factory function to create and start actor

**Tests**: 7 tests passing
- test_engine_actor_creation
- test_engine_create_block
- test_engine_authorization_owner
- test_engine_authorization_non_owner_rejected
- test_engine_authorization_with_grant
- test_engine_vector_clock_updates
- test_engine_get_block

### 3. Async Persistence with sqlx (`src/engine/event_store.rs`)

**Migration**: Replaced rusqlite with sqlx for async compatibility

**Why the Change**:
- rusqlite's `Connection` is not `Send`, cannot be used with tokio::spawn
- sqlx provides `SqlitePool` which is `Send + Sync`
- Enables true async database operations
- Better concurrency with connection pooling

**Key Features**:
- `EventStore::create(path)` - Create database with schema initialization
- `EventStore::append_events(pool, events)` - Async event persistence
- `EventStore::get_all_events(pool)` - Async event retrieval
- `EventStore::get_events_by_entity(pool, entity)` - Query by entity
- `create_if_missing(true)` - Automatic database file creation
- Connection pool with max 5 connections

**Tests**: 2 tests passing (migrated to #[tokio::test])
- test_append_and_retrieve_events
- test_get_events_by_entity

### 4. ElfArchive Async Migration (`src/elf/archive.rs`)

**Changes**:
- `new()` → `async fn new()`
- `event_store()` → `async fn event_pool()` returns `SqlitePool`
- All tests migrated to `#[tokio::test]`

**Tests**: 3 tests passing
- test_create_and_save
- test_open_and_read
- test_round_trip

### 5. EngineManager (`src/engine/manager.rs`)

**Purpose**: Manage multiple engine instances (one per .elf file)

**Features**:
- Uses DashMap for thread-safe concurrent access
- Spawn engines on demand
- Get handles to existing engines
- Shutdown individual or all engines

**Implementation**: [TO BE COMPLETED]

## Design Decisions

### 1. Handler Interface: `Option<&Block>`

**Problem**: `core.create` needs to create blocks that don't exist yet, but other handlers need existing blocks.

**Solution**: Modified handler trait to accept `Option<&Block>`:
```rust
fn handler(&self, cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>>;
```

- `core.create` receives `None`
- All other handlers receive `Some(&block)` and unwrap with error handling

**Impact**: All 8 capability handlers updated, macro updated, 17 test call sites fixed

### 2. StateProjector Reuses GrantsTable

**Problem**: Both StateProjector and GrantsTable need to project grant/revoke events.

**Solution**: StateProjector contains a `GrantsTable` instance:
```rust
pub struct StateProjector {
    pub grants: GrantsTable,  // Reuses existing implementation
    // ...
}
```

**Benefit**: No duplication of grant projection logic

### 3. MVP-Level Conflict Detection

**Problem**: Full vector clock comparison is complex for MVP.

**Solution**: Simple editor count checking:
```rust
pub fn has_conflict(&self, editor_id: &str, expected_count: u64) -> bool {
    let current_count = self.get_editor_count(editor_id);
    expected_count < current_count
}
```

**Tradeoff**: Only detects conflicts for the command editor, not all editors. Sufficient for MVP.

### 4. In-Memory Event Storage → sqlx Persistence

**Evolution**:
1. **Initial Plan**: Use rusqlite with `Connection`
2. **Problem**: rusqlite `Connection` is not `Send`, incompatible with tokio::spawn
3. **Temporary Solution**: Used `Arc<Mutex<Vec<Event>>>` for in-memory storage
4. **Final Solution**: Migrated to sqlx with `SqlitePool`

**Result**: True async persistence with thread-safe connection pooling

## Test Coverage

**Total Tests**: 44 passing (before EngineManager)

**Breakdown**:
- Capability tests: 33 passing
  - Registry: 10 tests
  - Grants: 6 tests
  - Markdown: 9 tests
  - Core builtins: 8 tests (integrated in registry tests)

- Engine tests: 11 passing
  - Actor: 7 tests
  - State: 4 tests

- Storage tests: 5 passing
  - EventStore: 2 tests
  - Archive: 3 tests

**After EngineManager**: Expected 48+ tests

## API Changes

### Handler Trait
```rust
// Before
fn handler(&self, cmd: &Command, block: &Block) -> CapResult<Vec<Event>>;

// After
fn handler(&self, cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>>;
```

### EventStore
```rust
// Before (rusqlite)
impl EventStore {
    pub fn new(path: &str) -> Result<Self, rusqlite::Error>;
    pub fn append_events(&self, events: &[Event]) -> Result<(), rusqlite::Error>;
    pub fn get_all_events(&self) -> Result<Vec<Event>, rusqlite::Error>;
}

// After (sqlx)
impl EventStore {
    pub async fn create(path: &str) -> Result<SqlitePool, sqlx::Error>;
    pub async fn append_events(pool: &SqlitePool, events: &[Event]) -> Result<(), sqlx::Error>;
    pub async fn get_all_events(pool: &SqlitePool) -> Result<Vec<Event>, sqlx::Error>;
}
```

### ElfArchive
```rust
// Before
impl ElfArchive {
    pub fn new() -> std::io::Result<Self>;
    pub fn event_store(&self) -> Result<EventStore, rusqlite::Error>;
}

// After
impl ElfArchive {
    pub async fn new() -> std::io::Result<Self>;
    pub async fn event_pool(&self) -> Result<SqlitePool, sqlx::Error>;
}
```

## Dependencies Added

```toml
[dependencies]
tokio = { version = "1.36", features = ["full"] }
dashmap = "5.5"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
```

**Removed**:
```toml
rusqlite = { version = "0.31", features = ["bundled"] }
```

## Files Modified

### Created:
- `src/engine/state.rs` (~300 lines)
- `src/engine/actor.rs` (~260 lines)
- `src/engine/manager.rs` (TBD, ~150 lines)

### Modified:
- `src/engine/event_store.rs` - Rewritten for sqlx
- `src/engine/mod.rs` - Export new modules
- `src/elf/archive.rs` - Async migration
- `src/capabilities/core.rs` - Handler trait signature
- `src/capabilities/grants.rs` - Removed `from_event_store`
- `src/capabilities/builtins/*.rs` - All 6 handlers updated (create, link, unlink, delete, grant, revoke)
- `src/extensions/markdown/*.rs` - Both handlers updated (write, read)
- `capability-macros/src/lib.rs` - Macro signature update
- `Cargo.toml` - Dependencies update

### Test Files:
- `src/capabilities/registry.rs` - 8 test call sites fixed
- `src/extensions/markdown/mod.rs` - 9 test call sites fixed

## Remaining Work

### EngineManager Implementation

**File**: `src/engine/manager.rs`

**Estimated Time**: 1-2 hours

**Requirements**:
- Manage DashMap of file_id → EngineHandle
- spawn_engine, get_engine, shutdown_engine, shutdown_all
- 4 unit tests

**After Completion**:
- Run all tests (expect 48+ passing)
- Run clippy
- Git commit

## Lessons Learned

1. **Async Complexity**: rusqlite incompatibility was discovered late. Should have validated async compatibility earlier.

2. **Test-Driven Development**: Having comprehensive tests (44 tests) made the rusqlite → sqlx migration much safer. All tests continued to pass after migration.

3. **Interface Design**: The `Option<&Block>` pattern worked well for handling both create and update operations in a unified interface.

4. **Documentation**: Real-time documentation updates help maintain alignment between plan and implementation.

## Next Steps

1. ✅ Implement EngineManager
2. ✅ Verify all tests pass (51 tests passing)
3. ✅ Run clippy
4. ✅ Git commit: "Part 5: Complete Elfile Engine with sqlx and EngineManager" (f46db44)
5. ⏭️ Begin Part 6: Tauri App Interface

## References

For detailed architecture explanation, see [Engine Architecture Guide](./engine-architecture.md).

## Conclusion

Part 5 is functionally complete with a robust, async-compatible engine implementation. The actor model provides clean separation of concerns, the sqlx migration ensures true async persistence, and comprehensive test coverage gives confidence in the implementation. Ready to proceed to Part 6 (Tauri App Interface).

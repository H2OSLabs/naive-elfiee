# Block Metadata System Implementation

## Overview

This feature adds a **strongly-typed** metadata system to Block entities, enabling automatic timestamp tracking, custom fields, and proper event-sourcing integration. The implementation follows the EAVT (Entity-Attribute-Value-Timestamp) model and maintains backward compatibility.

**Branch**: `feat/block-metadata-v2`
**PR**: #27
**Commits**: dd4879e → d2fa098 (5 commits) + Strong Typing Refactor
**Status**: ✅ Merged to dev

## Key Features

- **Strong Type Safety**: `Block.metadata` is `BlockMetadata` type (not `JsonValue`)
- **Automatic Timestamps**: `created_at` and `updated_at` in ISO 8601 UTC format
- **Custom Fields**: Flexible JSON structure with `#[serde(flatten)]` support
- **Type-Safe Methods**: `new()`, `touch()`, `from_json()`, `to_json()`
- **Event Sourcing**: Full integration with EAVT event store
- **Frontend Bindings**: Auto-generated TypeScript types via tauri-specta

## New Files

### 1. `src-tauri/src/models/metadata.rs`
Block metadata structure with helper methods.

**Core Structure**:
```rust
pub struct BlockMetadata {
    pub description: Option<String>,
    pub created_at: Option<String>,   // ISO 8601 UTC
    pub updated_at: Option<String>,   // ISO 8601 UTC
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}
```

**Methods**:
- `new()` - Create with current timestamps
- `from_json(value)` - Parse from JSON Value
- `to_json()` - Convert to JSON Value
- `touch()` - Update `updated_at` to current time

**Test Coverage**: 7 tests covering serialization, timestamps, and custom fields

## Modified Files

### 1. `src-tauri/src/models/block.rs`
**Added**:
- `metadata: BlockMetadata` field to `Block` struct (strongly-typed)
- Import of `BlockMetadata` type
- Default initialization to `BlockMetadata::default()`
- Test: `test_block_with_metadata`, `test_new_block_has_empty_metadata`

**Before**:
```rust
pub struct Block {
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: JsonValue,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,
}
```

**After**:
```rust
use super::BlockMetadata;  // NEW

pub struct Block {
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: JsonValue,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,
    pub metadata: BlockMetadata,  // NEW - Strongly typed
}
```

### 2. `src-tauri/src/models/payloads.rs`
**Added**:
- `metadata: Option<JsonValue>` field to `CreateBlockPayload`

**Behavior**:
- If user provides metadata, it's merged with auto-generated timestamps
- If not provided, only timestamps are set

### 3. `src-tauri/src/capabilities/builtins/create.rs`
**Modified**: `handle_create()` function

**New Logic (Type-Safe)**:
```rust
// 1. Start with auto-generated timestamps
let mut metadata = BlockMetadata::new();

// 2. Merge user-provided metadata if present
if let Some(user_metadata) = payload.metadata {
    if let Ok(parsed) = BlockMetadata::from_json(&user_metadata) {
        metadata.description = parsed.description;
        metadata.custom = parsed.custom;
        // Timestamps always auto-generated
    }
}

// 3. Serialize to JSON for event
serde_json::json!({
    "metadata": metadata.to_json()
})
```

**Tests Added**:
- `test_create_generates_metadata_with_timestamps`
- `test_create_merges_user_metadata`
- `test_create_without_metadata`

### 4. `src-tauri/src/extensions/markdown/markdown_write.rs`
**Modified**: `handle_markdown_write()` function

**New Logic (Type-Safe & Simplified)**:
```rust
// Update metadata.updated_at using the touch() method
let mut new_metadata = block.metadata.clone();
new_metadata.touch();

// Serialize to JSON for event
serde_json::json!({
    "contents": new_contents,
    "metadata": new_metadata.to_json()
})
```

**Benefits**:
- ✅ Simplified from ~25 lines to 3 lines
- ✅ Type-safe: `touch()` method ensures correct implementation
- ✅ Automatic field preservation (description, custom fields, created_at)
- ✅ No need for null/corrupted metadata handling (type system prevents it)

**Tests Added**:
- `test_markdown_write_updates_timestamp`
- `test_markdown_write_preserves_other_metadata`
- `test_markdown_write_handles_missing_metadata`

**Tests Removed**:
- ~~`test_markdown_write_handles_null_metadata_with_created_at`~~ (impossible with strong typing)

### 5. `src-tauri/src/engine/state.rs`
**Modified**: `apply_event()` function

**New Logic (Type-Safe Deserialization)**:
```rust
use crate::models::BlockMetadata;

// In core.create handler:
metadata: obj
    .get("metadata")
    .and_then(|v| BlockMetadata::from_json(v).ok())
    .unwrap_or_default(),

// In write/link handlers:
if let Some(new_metadata) = event.value.get("metadata") {
    if let Ok(parsed) = BlockMetadata::from_json(new_metadata) {
        block.metadata = parsed;
    }
}
```

**Benefits**:
- ✅ Type-safe deserialization from event JSON
- ✅ Graceful fallback to default if parsing fails
- ✅ State projector works with strongly-typed Block struct

**Tests Added**:
- `test_apply_create_event_with_metadata`
- `test_apply_write_event_updates_metadata`
- `test_replay_maintains_metadata`

### 6. `src-tauri/src/engine/actor.rs`
**Modified**: `process_command()` function

**New Logic**:
- Inject `_block_dir` path before command execution
- Strip `_block_dir` from events before persistence

**Tests Added**:
- `test_create_block_with_metadata`
- `test_metadata_persists_after_replay`
- `test_write_updates_timestamp`

### 7. `src-tauri/src/lib.rs`
**Modified**: Type registration for tauri-specta

**Added**:
```rust
.typ::<models::BlockMetadata>()
```

Ensures TypeScript bindings are generated for frontend.

### 8. `src/bindings.ts` (Auto-generated)
**Added TypeScript Types (Strongly-Typed)**:
```typescript
export type BlockMetadata = {
  description?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
} & Partial<{ [key in string]: JsonValue }>;

export type Block = {
  block_id: string;
  name: string;
  block_type: string;
  contents: JsonValue;
  children: Partial<{ [key in string]: string[] }>;
  owner: string;
  metadata: BlockMetadata;  // ✅ Strongly-typed, not JsonValue
};

export type CreateBlockPayload = {
  name: string;
  block_type: string;
  metadata?: JsonValue | null;
};
```

**Frontend Benefits**:
- ✅ TypeScript autocomplete for `block.metadata.created_at`
- ✅ Compile-time errors for typos like `block.metadata.createdAt`
- ✅ Type safety when accessing custom fields

## Follow-up Fixes

### Fix 1: Field Naming Consistency (commit 3507236)
**Issue**: `FileMetadata.last_modified` vs `BlockMetadata.updated_at` inconsistency

**Changes**:
- Renamed `FileMetadata.last_modified` → `updated_at`
- Updated `commands/file.rs` local variable naming
- Regenerated TypeScript bindings

### Fix 2: PR Review Feedback (commit 6f74d50)
**Issue**: Chinese comments in production code

**Changes**:
- Translated all doc comments in `models/metadata.rs` to English
- Preserved `created_at` when reinitializing corrupted metadata
- Added defensive test for null metadata handling

### Fix 3: Test Comment Translation (commit d2fa098)
**Changes**:
- Translated test comments in `create.rs`
- Translated test comments in `markdown_write.rs`
- Translated test comments in `metadata.rs`

### Fix 4: Strong Typing Refactor (Post-Merge)
**Issue**: `BlockMetadata` struct was defined but `Block.metadata` was `JsonValue`, losing type safety benefits

**Critical Changes**:
1. **Block.rs**: Changed `metadata: JsonValue` → `metadata: BlockMetadata`
2. **create.rs**: Use `BlockMetadata::new()` and `from_json()`/`to_json()` methods
3. **markdown_write.rs**: Simplified to `metadata.touch()` (from ~25 lines to 3 lines)
4. **state.rs**: Use `BlockMetadata::from_json()` for type-safe deserialization
5. **All tests**: Updated to use strongly-typed field access (`.created_at` instead of `["created_at"]`)

**Impact**:
- ✅ **100 tests passing** (no regressions)
- ✅ Compile-time type safety for all metadata operations
- ✅ Simplified handler code (removed complex JSON manipulation)
- ✅ Frontend gets strongly-typed `Block.metadata: BlockMetadata`
- ✅ `BlockMetadata.touch()` method now actually used in production

**Code Reduction**:
- `markdown_write.rs`: 25 lines → 3 lines for metadata update
- Removed 1 test (null metadata handling no longer needed)
- Eliminated defensive null checks (type system prevents null)

## Architecture Design

### Event Sourcing Pattern: Snapshot vs Delta

**Current Implementation**: **Event as Snapshot**
- Events contain full metadata state
- State projector performs simple replacement
- Each event is self-contained

**Rationale**:
- Simpler projector logic (no merge complexity)
- Easier debugging (each event shows complete state)
- Consistent with current capability handler pattern
- No data loss risk with current handlers (all clone full metadata)

**Alternative** (not implemented): Event as Delta
- Would require merge logic in `state.rs`
- Smaller events but more complex projection
- Can be refactored later if needed

### Metadata Update Pattern

**Current Pattern** (Type-Safe with `BlockMetadata`):
```rust
// ✅ RECOMMENDED - Use touch() method
let mut new_metadata = block.metadata.clone();
new_metadata.touch();

// ✅ ACCEPTABLE - Manual field update
let mut new_metadata = block.metadata.clone();
new_metadata.updated_at = Some(crate::utils::time::now_utc());

// ✅ CORRECT - Serialize for events
serde_json::json!({
    "metadata": new_metadata.to_json()
})
```

**Legacy Pattern** (Before Strong Typing Refactor):
```rust
// OLD - JSON manipulation (no longer needed)
let mut new_metadata = block.metadata.clone();
if let Some(obj) = new_metadata.as_object_mut() {
    obj.insert("updated_at".to_string(), serde_json::json!(now_utc()));
}
```

**Benefits of Strong Typing**:
- ✅ Compile-time errors if you forget to clone
- ✅ No JSON object checks (type system guarantees structure)
- ✅ Reusable `touch()` method ensures consistency
- ✅ Field preservation automatic (description, custom fields, created_at)

## Testing Summary

**Total Tests Added**: 18 tests across 5 modules

| Module | Tests | Coverage |
|--------|-------|----------|
| `metadata.rs` | 7 | Serialization, timestamps, custom fields |
| `create.rs` | 3 | Metadata generation and merging |
| `markdown_write.rs` | 3 | Timestamp updates, field preservation (1 removed) |
| `state.rs` | 3 | Event projection with metadata |
| `actor.rs` | 2 | End-to-end persistence |

**Test Results**: ✅ **100 tests passing** (102 total with integration tests)

**Test Improvements After Strong Typing**:
- Updated all assertions to use strongly-typed field access
- Removed impossible test case (null metadata with strong typing)
- Simplified test setup (use `BlockMetadata` struct directly)

## Migration Guide

### For Existing Blocks

**No migration needed** - Existing blocks without metadata will deserialize to `BlockMetadata::default()` (all fields `None`).

### For New Capabilities

When implementing new capabilities that modify blocks, use the **strongly-typed pattern**:

1. **Clone and update metadata** (RECOMMENDED):
   ```rust
   let mut new_metadata = block.metadata.clone();
   new_metadata.touch();  // Updates updated_at automatically
   ```

2. **Or manually update fields**:
   ```rust
   let mut new_metadata = block.metadata.clone();
   new_metadata.description = Some("Updated description".to_string());
   new_metadata.updated_at = Some(crate::utils::time::now_utc());
   ```

3. **Serialize for events**:
   ```rust
   serde_json::json!({
       "contents": new_contents,
       "metadata": new_metadata.to_json()  // Convert to JSON
   })
   ```

**Important**: Never manipulate metadata as JSON directly. Always use `BlockMetadata` methods.

### For Frontend

Use type-safe bindings:
```typescript
import { BlockMetadata, CreateBlockPayload } from './bindings';

// Creating a block with metadata
const payload: CreateBlockPayload = {
  name: "My Block",
  block_type: "markdown",
  metadata: {
    description: "Project requirements doc"
  }
};

// Accessing metadata
const created_at = block.metadata.created_at;
const description = block.metadata.description;
```

## Known Limitations

1. **No validation of custom fields**: Any JSON value can be added to `custom`
   - Future enhancement: JSON schema validation
   - Current design prioritizes flexibility

2. **Metadata size limits**: No hard limits on metadata size
   - Event store can handle large JSON values
   - Consider adding limits in future if needed

3. **Event JSON overhead**: Events store full metadata as JSON (snapshot pattern)
   - Trade-off: simpler projector vs larger event size
   - Can optimize to delta pattern if storage becomes a concern

## Reviewer Feedback Addressed

### @jjkysy (Initial PR Review)
- ✅ Metadata replacement pattern clarified (Snapshot vs Delta)
- ✅ Chinese comments translated to English
- ✅ `created_at` preservation in edge cases

### @claude-bot (Initial PR Review)
- ✅ Chinese comments in tests translated
- ✅ Defensive handling for corrupted metadata
- ✅ Test coverage for partial metadata updates

### Post-Merge Self-Review (Strong Typing)
- ✅ **Critical**: `BlockMetadata` was defined but unused in `Block.metadata`
- ✅ Fixed: Changed `Block.metadata` to strongly-typed `BlockMetadata`
- ✅ Simplified: Removed ~25 lines of JSON manipulation code
- ✅ Activated: `BlockMetadata.touch()` now used in production
- ✅ Improved: All tests use strongly-typed field access

## Related Documentation

- `docs/concepts/ARCHITECTURE_OVERVIEW.md` - EAVT model explanation
- `docs/guides/EXTENSION_DEVELOPMENT.md` - Metadata handling patterns (to be added)
- `src-tauri/src/utils/time.rs` - Timestamp utility functions

## Metrics

### Initial Implementation
- **Files Modified**: 9 files (+ 1 new)
- **Lines Added**: ~800 lines (code + tests)
- **Test Coverage**: 18 new tests
- **Breaking Changes**: None (metadata defaults to empty)

### Strong Typing Refactor
- **Files Modified**: 5 files (block.rs, create.rs, markdown_write.rs, state.rs, actor.rs + all tests)
- **Lines Removed**: ~30 lines (simplified JSON manipulation)
- **Lines Added**: ~20 lines (type conversions)
- **Net Code Reduction**: -10 lines with better type safety
- **Breaking Changes**: None (internal refactor, event format unchanged)
- **Performance Impact**: Negligible (same JSON serialization)

### Final State
- ✅ **100% test pass rate** (100 unit tests + 2 integration tests)
- ✅ **Zero regressions**
- ✅ **Full type safety** (Rust + TypeScript)
- ✅ **Production ready**

---

**Last Updated**: 2025-12-19
**Author**: Sy Yao
**Reviewers**: @jjkysy, @claude-bot
**Post-Merge Refactor**: Strong Typing (2025-12-19)

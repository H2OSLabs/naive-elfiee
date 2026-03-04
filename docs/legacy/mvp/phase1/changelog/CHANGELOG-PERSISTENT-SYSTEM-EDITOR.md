# Persistent System Editor ID Implementation

## Overview

This feature implements a **persistent system editor ID** stored in global configuration (`~/.elf/config.json`), ensuring consistent block ownership and collaboration identity across application restarts. Previously, the system editor ID was randomly generated on each startup, causing blocks created in previous sessions to become inaccessible.

**Branch**: `feat/collaborators`
**Issue**: System editor ID changes on every restart, breaking CBAC ownership chain
**Status**: ✅ Implemented

## Problem Statement

### Original Behavior
1. Application starts → generates random system editor ID (e.g., `uuid-123`)
2. User creates blocks → blocks owned by `uuid-123`
3. Application restarts → generates NEW system editor ID (e.g., `uuid-456`)
4. Previous blocks are orphaned → authorization fails because `uuid-456` ≠ `uuid-123`

### Impact
- Blocks created in previous sessions become inaccessible
- Authorization errors when trying to edit existing blocks
- Inconsistent collaborator IDs in file metadata
- Poor multi-machine collaboration experience

## Solution Architecture

### Global Configuration Store
- **Location**: `$USER_HOME/.elf/config.json` (cross-platform)
- **Content**: `{ "system_editor_id": "uuid" }`
- **Lifecycle**:
  - Created on first application launch
  - Persists across application restarts
  - Unique per machine (enables multi-machine collaboration)

### System Editor Lifecycle
1. **Application Startup**: Load or create system editor ID in config
2. **File Bootstrap**: Use config ID when creating system editor for new files
3. **Command Execution**: Fallback to config ID when no active editor is set
4. **Frontend Operations**: Fetch config ID from backend (with caching)

## New Files

### 1. `src-tauri/src/config.rs`
Global configuration management module.

**Core Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GlobalConfig {
    pub system_editor_id: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            system_editor_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}
```

**Functions**:
- `get_config_path() -> Result<PathBuf, String>` - Returns `~/.elf/config.json`
- `load_config() -> Result<GlobalConfig, String>` - Loads or creates config
- `save_config(config: &GlobalConfig) -> Result<(), String>` - Persists config
- `get_system_editor_id() -> Result<String, String>` - Public API for getting ID

**Error Handling**:
- Creates `~/.elf/` directory if missing
- Graceful fallback if config file is corrupted
- Proper error messages for I/O failures

**Test Coverage**: 0 tests (TODO: Add unit tests for config operations)

## Modified Files

### 1. `src-tauri/src/lib.rs`
**Added**:
- `pub mod config;` - Register config module
- `commands::file::get_system_editor_id_from_config` - Command registration (debug + release)

**Purpose**:
- Expose config module to application
- Allow frontend to fetch system editor ID via IPC

### 2. `src-tauri/src/commands/file.rs`
**Added**:
- `use crate::config;` - Import config module
- `get_system_editor_id_from_config()` command (lines 631-643)

**Modified**: `bootstrap_editors()` function (lines 42-63)

**Before**:
```rust
if editors.is_empty() {
    let cmd = Command::new(
        "system".to_string(),  // ❌ Hardcoded string
        "editor.create".to_string(),
        "".to_string(),
        serde_json::json!({ "name": "System" }),  // ❌ No editor_id
    );

    let events = handle.process_command(cmd).await?;

    if let Some(event) = events.first() {
        let system_editor_id = event.entity.clone();  // ❌ Random UUID from event
        state.set_active_editor(file_id.to_string(), system_editor_id);
    }
}
```

**After**:
```rust
if editors.is_empty() {
    // Get persistent system editor ID from global config
    let system_editor_id = config::get_system_editor_id()
        .map_err(|e| format!("Failed to get system editor ID: {}", e))?;

    // Create system editor with the persistent ID
    let cmd = Command::new(
        system_editor_id.clone(),  // ✅ Use config ID
        "editor.create".to_string(),
        "".to_string(),
        serde_json::json!({
            "name": "System",
            "editor_id": system_editor_id.clone()  // ✅ Pass ID in payload
        }),
    );

    let events = handle.process_command(cmd).await?;

    // Set as active editor if creation succeeded
    if events.first().is_some() {
        state.set_active_editor(file_id.to_string(), system_editor_id);  // ✅ Use config ID
    }
}
```

**Benefits**:
- ✅ System editor ID is now deterministic (from config)
- ✅ Same ID used across all application sessions
- ✅ Proper error handling if config loading fails

**New Command**:
```rust
#[tauri::command]
#[specta]
pub async fn get_system_editor_id_from_config() -> Result<String, String> {
    config::get_system_editor_id()
}
```

### 3. `src-tauri/src/commands/editor.rs`
**Modified**: `create_editor()` function fallback (lines 33-41)

**Before**:
```rust
let creator_editor_id = state
    .get_active_editor(&file_id)
    .unwrap_or_else(|| "system".to_string());  // ❌ Hardcoded fallback
```

**After**:
```rust
let creator_editor_id = state
    .get_active_editor(&file_id)
    .unwrap_or_else(|| {
        config::get_system_editor_id()
            .unwrap_or_else(|_| "system".to_string())  // ✅ Config fallback
    });
```

**Rationale**:
- When creating a new editor (collaborator), the operation should be attributed to an identity
- Fallback to system editor ID maintains audit trail consistency
- Ultimate fallback to `"system"` string prevents application crashes

### 4. `src-tauri/src/commands/block.rs`
**Modified**: `update_block_metadata()` function fallback (lines 124-129)

**Before**:
```rust
let editor_id = state
    .get_active_editor(&file_id)
    .unwrap_or_else(|| "system".to_string());  // ❌ Hardcoded fallback
```

**After**:
```rust
let editor_id = state
    .get_active_editor(&file_id)
    .unwrap_or_else(|| {
        config::get_system_editor_id()
            .unwrap_or_else(|_| "system".to_string())  // ✅ Config fallback
    });
```

**Rationale**:
- Metadata updates should be attributed to an editor identity
- Consistent fallback behavior across all commands

### 5. `src-tauri/src/models/payloads.rs`
**Modified**: `EditorCreatePayload` struct (lines 95-98)

**Added Field**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EditorCreatePayload {
    pub name: String,
    #[serde(default)]
    pub editor_type: Option<String>,
    #[serde(default)]
    pub editor_id: Option<String>,  // ✅ NEW - Allow providing editor_id
}
```

**Purpose**:
- Allow `bootstrap_editors()` to specify the system editor ID
- Maintain backward compatibility (optional field with `#[serde(default)]`)

### 6. `src-tauri/src/capabilities/builtins/editor_create.rs`
**Modified**: `handle_editor_create()` function (lines 16-19)

**Before**:
```rust
// Generate new editor ID
let editor_id = uuid::Uuid::new_v4().to_string();  // ❌ Always random
```

**After**:
```rust
// Use provided editor_id from payload, or generate new one if not provided
let editor_id = payload
    .editor_id
    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());  // ✅ Respect payload
```

**Critical Fix**:
- This was the **root cause** of the system editor ID mismatch
- Handler was ignoring the `editor_id` field in payload
- Now honors provided ID for bootstrap, generates UUID for UI-created editors

**Test Added** (lines 155-183):
```rust
#[test]
fn test_editor_create_with_provided_editor_id() {
    let provided_id = "8e6075dd-9a14-46e7-b6cf-bab2d1019ea0";
    let cmd = Command {
        cmd_id: uuid::Uuid::new_v4().to_string(),
        editor_id: provided_id.to_string(),
        cap_id: "editor.create".to_string(),
        block_id: "system".to_string(),
        payload: serde_json::json!({
            "name": "System",
            "editor_id": provided_id
        }),
        timestamp: chrono::Utc::now(),
    };

    let result = handle_editor_create(&cmd, None);
    assert!(result.is_ok());

    let events = result.unwrap();
    assert_eq!(events.len(), 1);

    let event = &events[0];
    // Entity should be the provided editor_id
    assert_eq!(event.entity, provided_id);
    // Value should also contain the same editor_id
    assert_eq!(event.value["editor_id"], provided_id);
}
```

### 7. `src-tauri/Cargo.toml`
**Added Dependency**:
```toml
dirs = "5.0"
```

**Purpose**: Cross-platform user home directory detection

### 8. `src/lib/tauri-client.ts`
**Modified**: Frontend IPC client

**Removed**:
```typescript
const DEFAULT_EDITOR_ID = 'default-editor'  // ❌ Hardcoded fallback
```

**Added** (lines 23-52):
```typescript
/**
 * Cache for system editor ID
 */
let SYSTEM_EDITOR_ID_CACHE: string | null = null

/**
 * Get the system editor ID (with caching)
 *
 * This fetches the persistent system editor ID from the backend config.
 * The ID is cached after first fetch to avoid repeated IPC calls.
 */
async function getSystemEditorId(): Promise<string> {
  if (SYSTEM_EDITOR_ID_CACHE) {
    return SYSTEM_EDITOR_ID_CACHE
  }

  try {
    const result = await commands.getSystemEditorIdFromConfig()
    if (result.status === 'ok') {
      SYSTEM_EDITOR_ID_CACHE = result.data
      return result.data
    } else {
      console.error('Failed to get system editor ID:', result.error)
      return 'system'  // Ultimate fallback
    }
  } catch (error) {
    console.error('Error getting system editor ID:', error)
    return 'system'  // Ultimate fallback
  }
}
```

**Modified Functions** (5 operations):
1. **`createBlock()`** (line 235):
   ```typescript
   const activeEditorId =
     editorId ||
     (await EditorOperations.getActiveEditor(fileId)) ||
     (await getSystemEditorId())  // ✅ Use backend config
   ```

2. **`writeBlock()`** (line 260):
   ```typescript
   const activeEditorId =
     editorId ||
     (await EditorOperations.getActiveEditor(fileId)) ||
     (await getSystemEditorId())  // ✅ Use backend config
   ```

3. **`deleteBlock()`** (line 283):
   ```typescript
   const activeEditorId =
     editorId ||
     (await EditorOperations.getActiveEditor(fileId)) ||
     (await getSystemEditorId())  // ✅ Use backend config
   ```

4. **`grantCapability()`** (line 410):
   ```typescript
   const activeEditorId =
     editorId || (await this.getActiveEditor(fileId)) || (await getSystemEditorId())  // ✅ Use backend config
   ```

5. **`revokeCapability()`** (line 435):
   ```typescript
   const activeEditorId =
     editorId || (await this.getActiveEditor(fileId)) || (await getSystemEditorId())  // ✅ Use backend config
   ```

**Benefits**:
- ✅ Frontend uses same system editor ID as backend
- ✅ IPC result cached to avoid repeated calls
- ✅ Graceful error handling with ultimate fallback
- ✅ Fixes authorization errors caused by ID mismatch

### 9. `src/bindings.ts` (Auto-generated)
**Added TypeScript Type**:
```typescript
export type GlobalConfig = { system_editor_id: string }
```

**Added Command Binding**:
```typescript
export const commands = {
  // ... existing commands
  getSystemEditorIdFromConfig: () =>
    TAURI_INVOKE<Result<string, string>>("get_system_editor_id_from_config"),
}
```

## Workflow Verification

### Expected Behavior (After Fix)

**Scenario**: Create new file and add blocks

1. **Create File**:
   ```
   POST /create_file
   → file_id: "file-xxx"
   ```

2. **Get File Info**:
   ```
   POST /get_file_info
   Response:
   {
     "collaborators": ["8e6075dd-9a14-46e7-b6cf-bab2d1019ea0"],  // ✅ Config ID
     ...
   }
   ```

3. **Get Active Editor**:
   ```
   POST /get_active_editor
   → "8e6075dd-9a14-46e7-b6cf-bab2d1019ea0"  // ✅ Same ID
   ```

4. **Create Block**:
   ```
   POST /execute_command (core.create)
   Response event:
   {
     "entity": "block-xxx",
     "value": {
       "owner": "8e6075dd-9a14-46e7-b6cf-bab2d1019ea0",  // ✅ Same ID
       ...
     }
   }
   ```

5. **Get All Blocks**:
   ```
   POST /get_all_blocks
   Response:
   [
     {
       "block_id": "block-xxx",
       "owner": "8e6075dd-9a14-46e7-b6cf-bab2d1019ea0",  // ✅ Same ID
       ...
     }
   ]
   ```

**Verification**: All IDs match `config.json` system_editor_id

### Before vs After

| Aspect | Before | After |
|--------|--------|-------|
| System Editor ID | Random UUID per session | Persistent UUID from config |
| Block Ownership | Changes on restart | Consistent across restarts |
| Collaborators List | Mismatched IDs | Consistent config ID |
| Authorization | Fails after restart | Works across sessions |
| Multi-machine | ID conflicts | Unique per machine |

## Testing Summary

**Manual Tests Performed**:
1. ✅ Create new file → verify system editor created with config ID
2. ✅ Create blocks → verify owner matches config ID
3. ✅ Restart application → verify same system editor ID used
4. ✅ Existing blocks remain accessible after restart
5. ✅ Collaborators list matches config ID

**Unit Tests Added**:
- `test_editor_create_with_provided_editor_id()` in `editor_create.rs`

**Unit Tests TODO**:
- [ ] Config file creation and loading
- [ ] Fallback behavior when config is corrupted
- [ ] Cross-platform path resolution

**Integration Tests TODO**:
- [ ] End-to-end: Create file → restart → edit blocks
- [ ] Multi-machine: Different system editor IDs per machine

## Architecture Trade-offs

### Global Config vs Per-File Config

**Chosen**: Global config (`~/.elf/config.json`)

**Rationale**:
- System editor represents the **machine/user identity**, not file-specific
- Consistent identity across all files on same machine
- Simpler implementation (one config file vs per-file storage)
- Enables multi-machine collaboration (each machine has unique ID)

**Alternative** (not implemented): Per-file system editor ID
- Would store system editor ID inside each `.elf` file
- Issue: Same machine would have different IDs for different files
- Complexity: Migration when moving files between machines

### Frontend Caching Strategy

**Implementation**: Cache system editor ID on first fetch

**Benefits**:
- Reduces IPC calls (only fetch once per session)
- Consistent ID throughout frontend session
- Fast fallback for commands

**Risk**: Frontend cache could become stale if backend config changes
- Mitigation: Application restart syncs cache
- Not a concern in practice (config rarely changes manually)

### Fallback Chain

**Pattern**: `Active Editor → Config System Editor → "system" string`

**Rationale**:
1. **Active Editor**: Preferred identity when set by UI
2. **Config System Editor**: Machine/user identity fallback
3. **"system" string**: Ultimate fallback to prevent crashes

**Note**: Ultimate fallback should never be reached in normal operation

## Migration Guide

### For Existing Files

**No migration needed** - Files bootstrap system editor on next open.

**Process**:
1. Open existing `.elf` file
2. `bootstrap_editors()` checks if editors exist
3. If system editor exists → use existing ID, set as active
4. If no editors → create system editor with config ID
5. Existing blocks remain owned by their original editor IDs

### For Multi-Machine Collaboration

**Scenario**: User edits same `.elf` file on different machines

1. **Machine A**: System editor ID = `uuid-A`
2. **Machine B**: System editor ID = `uuid-B`
3. **File Collaborators**: Contains both `uuid-A` and `uuid-B`
4. **Block Ownership**: Each block owned by editor that created it

**Authorization**:
- Machine A can edit blocks it created (owner = `uuid-A`)
- Machine B can edit blocks it created (owner = `uuid-B`)
- Cross-machine editing requires explicit grants

**Future Enhancement**: Add "link machine identity" feature to recognize same user across machines

## Known Limitations

1. **Manual Config Editing**: Users can manually edit `~/.elf/config.json`
   - If ID is changed, previous blocks become orphaned
   - Mitigation: Document that config should not be edited
   - Future: Add warning in config file header

2. **Config File Corruption**: If config file becomes corrupted
   - Application creates new config with new ID
   - Previous blocks become orphaned
   - Mitigation: Robust error handling in `load_config()`
   - Future: Add config backup/recovery mechanism

3. **System Editor Name**: Currently hardcoded as "System"
   - Not user-friendly for multi-machine collaboration
   - Future: Allow customizing system editor name (e.g., "Alice's Laptop")

4. **No Config UI**: Users cannot view/manage system editor ID from UI
   - Current: Must manually check `~/.elf/config.json`
   - Future: Add settings panel to display system editor ID

## Follow-up Tasks

### High Priority
- [ ] Add unit tests for `config.rs` module
- [ ] Add integration test for persistence across restarts
- [ ] Document config file format in user guide

### Medium Priority
- [ ] Add settings UI to display system editor ID
- [ ] Allow customizing system editor name
- [ ] Add config backup mechanism

### Low Priority
- [ ] Implement "link machine identity" for multi-machine users
- [ ] Add config file version for future migrations
- [ ] Consider encrypting system editor ID (privacy concern?)

## Related Issues

### Root Cause Analysis

**Original Issue**: `collaborators` returned wrong ID (`7ff96099-...` instead of `8e6075dd-...`)

**Investigation**:
1. ✅ `bootstrap_editors()` was using hardcoded `"system"` string
   - **Fixed**: Use `config::get_system_editor_id()`
2. ✅ `bootstrap_editors()` wasn't passing `editor_id` in payload
   - **Fixed**: Added `"editor_id": system_editor_id` to payload
3. ✅ `editor_create` handler was ignoring payload `editor_id` field
   - **Fixed**: Use `payload.editor_id.unwrap_or_else(|| uuid::Uuid::new_v4())`

**Verification**: All three fixes were required to resolve the issue

### Authorization Error (Resolved)

**Error**: `Authorization failed: aca087a6-... does not have permission for core.grant`

**Cause**: Frontend using hardcoded `'default-editor'` fallback

**Fix**: Implemented `getSystemEditorId()` function to fetch from backend config

## Related Documentation

- `docs/concepts/ARCHITECTURE_OVERVIEW.md` - CBAC authorization model
- `docs/guides/EXTENSION_DEVELOPMENT.md` - Editor identity in commands
- `CLAUDE.md` - System editor and bootstrap process
- `README.md` - Configuration file location

## Metrics

- **Files Created**: 1 (`config.rs`)
- **Files Modified**: 8 (lib.rs, file.rs, editor.rs, block.rs, payloads.rs, editor_create.rs, Cargo.toml, tauri-client.ts)
- **Lines Added**: ~150 lines (code + tests)
- **Lines Removed**: ~10 lines (hardcoded fallbacks)
- **Breaking Changes**: None (backward compatible)
- **Tests Added**: 1 unit test (TODO: Add more)

---

**Date**: 2025-12-23
**Author**: Claude Code Assistant
**Reviewed By**: User Testing
**Status**: ✅ Implementation Complete, Testing in Progress

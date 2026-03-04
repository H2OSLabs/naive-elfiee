# Directory Extension TDD Process Checklist

> **Version**: v1.0 (2025-12-23)
> **Status**: In Progress
> **Goal**: Detailed TDD execution tracking for Directory Extension

---

## Phase 0: Core Capabilities (Prerequisites)

- [x] **0.1 Implement `core.rename`**
    - [x] Create test cases (`builtins::rename::tests`)
    - [x] Implement Handler (`handle_rename`)
    - [x] Register in `builtins/mod.rs` & `registry.rs`
    - [x] Update `StateProjector` to handle `core.rename` event
    - [x] Verify tests pass

- [x] **0.2 Implement `core.change_type`**
    - [x] Create test cases (`builtins::change_type::tests`)
    - [x] Implement Handler (`handle_change_type`)
    - [x] Register in `builtins/mod.rs` & `registry.rs`
    - [x] Update `StateProjector` to handle `core.change_type` event
    - [x] Verify tests pass

---

## Phase 1: Utility Modules

- [x] **1.1 File System Scanner (`utils/fs_scanner.rs`)**
    - [x] Write tests for scanning logic (filtering, limits)
    - [x] Implement `scan_directory` function
    - [x] Verify tests pass

- [x] **1.2 Path Validator (`utils/path_validator.rs`)**
    - [x] Write tests for path security (symlinks, sensitive dirs)
    - [x] Implement `is_safe_path` function
    - [x] Verify tests pass

- [x] **1.3 Block Type Inference (`utils/block_type_inference.rs`)**
    - [x] Write tests for extension mapping
    - [x] Implement `infer_block_type` function
    - [x] Verify tests pass

- [x] **1.4 Module Registration**
    - [x] Register all utils in `utils/mod.rs` and `lib.rs`
    - [x] Add dependencies to `Cargo.toml` (`walkdir`, `regex`)

--- 

## Phase 2: Skeleton Generation

- [x] **2.1 Generate Extension**
    - [x] Run `elfiee-ext-gen create ...`
    - [x] Verify file structure
    - [x] Verify compilation (`cargo check`)

---

## Phase 3: Basic Capabilities

- [x] **3.1 `directory.create`**
    - [x] Define `DirectoryCreatePayload`
    - [x] Write tests (`test_create_basic`)
    - [x] Implement Handler
    - [x] Verify tests pass

- [x] **3.2 `directory.delete`**
    - [x] Define `DirectoryDeletePayload`
    - [x] Write tests (`test_delete_basic`)
    - [x] Implement Handler (Recursive delete)
    - [x] Verify tests pass

- [x] **3.3 `directory.rename`**
    - [x] Define `DirectoryRenamePayload`
    - [x] Write tests (`test_rename_basic`)
    - [x] Implement Handler (Sync with Block.name)
    - [x] Verify tests pass

---

## Phase 4: Advanced Capabilities

- [x] **4.1 `directory.import`**
    - [x] Define `DirectoryImportPayload`
    - [x] Write tests (`test_import_basic`)
    - [x] Implement Handler (Scan -> Create Events)
    - [x] Verify tests pass

- [x] **4.2 `directory.refresh`**
    - [x] Define `DirectoryRefreshPayload`
    - [x] Write tests (`test_refresh_basic`)
    - [x] Implement Handler (Diff Algorithm)
    - [x] Verify tests pass

- [x] **4.3 `directory.export` (Tauri Command)**
    - [x] Implement `directory.export` capability (Audit only)
    - [x] Implement `commands::checkout::checkout_workspace` (IO logic)
    - [x] Write test for checkout logic (in `checkout.rs`)

- [x] **4.4 Bug Fixes & Hardening**
    - [x] Fix path matching bug ("foo" vs "foobar")
    - [x] Improve error handling (replace `.ok()`)
    - [x] Standardize content field access (`text` -> `markdown`)
    - [x] Add path matching isolation test

---

## Phase 5: StateProjector Extension (Removed)

- [x] **5.1 Handle Directory Events**
    - [x] ~Update `StateProjector` for `directory.write`~ (Auto-supported by `*.write` wildcard)
    - [x] Verify state updates correctly (Verified by `test_full_workflow`)

---

## Phase 6: Integration & Final Verification
- [x] **6.1 Full Test Suite**
    - [x] Run all unit tests (Verified by user)
    - [x] Validate with `elfiee-ext-gen validate`
- [x] **6.2 Binding Generation**
    - [x] `cargo build` to generate TypeScript types
- [x] **6.3 Frontend Implementation**
    - [x] Implement `buildTreeFromEntries` logic
    - [x] Implement `VfsTree` component
    - [x] Integrate with `FilePanel`
    - [x] Complete `tauri-client` and `app-store` extensions
    - [x] Verify all frontend tests pass
- [x] **6.4 Merge & Commit**

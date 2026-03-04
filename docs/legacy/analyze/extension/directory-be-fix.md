# Directory Extension Backend 修复计划

本文档记录 PR review 中发现的问题及修复计划。

---

## 问题总览

| ID | 类别 | 优先级 | 问题 | 预计时间 |
|----|------|--------|------|---------|
| 1 | Architecture | P0 | directory.list 路径处理不一致 | 30 分钟 |
| 2 | Code Quality | P0 | 代码重复（176 行） | 40 分钟 |
| 3 | Security | P0 | TOCTOU 竞态条件漏洞 | 30 分钟 |
| 4 | Security | P0 | 缺少 Symlink 攻击防护 | 20 分钟 |
| 5 | Performance | P1 | Pattern Matching 递归深度问题 | 20 分钟 |
| 6 | Code Quality | P1 | 错误消息不一致 | 15 分钟 |
| 7 | Documentation | P1 | directory.watch 功能限制未说明 | 10 分钟 |

**总计**:
- **Critical (P0)**: 2 小时
- **推荐 (P1)**: 45 分钟
- **可选 (P2)**: 30 分钟

---

## 🔴 P0: Critical Issues（必须修复）

### 问题 1: directory.list 路径处理不一致

**位置**: `src-tauri/src/extensions/directory/directory_list.rs:37`

**问题描述**:

当前 `directory.list` 将 `payload.root` 作为**绝对路径**处理：

```rust
// 当前代码（第 37 行）
let path = Path::new(&payload.root);
```

而其他 capabilities（create, delete, rename）都将 payload 中的路径作为**相对于 block.contents.root 的路径**处理：

```rust
// directory_create.rs:45-52 的模式
let root = block.contents.get("root").and_then(|v| v.as_str())?;
let full_path = Path::new(root).join(&payload.path);
```

**影响**:
- 破坏了用户的心智模型（不同 capability 行为不一致）
- 可能导致安全问题（绕过 root 限制）

**修复方案**:

```rust
// src-tauri/src/extensions/directory/directory_list.rs
// 修改 handle_list 函数（第 33-48 行）

#[capability(id = "directory.list", target = "directory")]
fn handle_list(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: DirectoryListPayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid DirectoryListPayload: {}", e))?;
    let block = block.ok_or("Block is required for directory.list")?;

    // Step 1: Get root from block.contents
    let root = block
        .contents
        .get("root")
        .and_then(|v| v.as_str())
        .ok_or("Block.contents must have 'root' field")?;

    // Step 2: Join payload.root as relative path (CHANGED)
    let full_path = Path::new(root).join(&payload.root);

    // Step 3: Canonicalize for security check
    let canonical_path = full_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize path '{}': {}", payload.root, e))?;

    let canonical_root = Path::new(root)
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize root: {}", e))?;

    // Step 4: Verify path is within root
    if !canonical_path.starts_with(&canonical_root) {
        return Err(format!(
            "Path '{}' is outside the root directory",
            payload.root
        ));
    }

    // ... 其余逻辑保持不变
}
```

**同时修改**:
- `directory_refresh.rs` 使用相同逻辑（第 55-69 行）

**测试影响**:
- 需要更新 `tests.rs` 中的 `test_list_basic` 和 `test_refresh_basic`
- `payload.root` 从绝对路径改为相对路径（如 `"."` 表示当前目录）

---

### 问题 2: 代码重复

**位置**:
- `src-tauri/src/extensions/directory/directory_list.rs:89-175`
- `src-tauri/src/extensions/directory/directory_refresh.rs:89-175`

**问题描述**:

两个文件包含完全相同的 176 行代码：
- `read_dir_single()` 函数（46 行）
- `read_dir_recursive()` 函数（130 行）

**修复方案**:

**步骤 1**: 创建共享模块 `src-tauri/src/extensions/directory/utils.rs`

```rust
//! Shared utilities for directory operations

use std::fs;
use std::path::Path;

/// Reads a single directory level without recursion
///
/// # Arguments
/// * `path` - Directory path to read
/// * `include_hidden` - Whether to include hidden files (starting with '.')
/// * `entries` - Mutable vector to append entries to
pub(super) fn read_dir_single(
    path: &Path,
    include_hidden: bool,
    entries: &mut Vec<serde_json::Value>,
) -> Result<(), String> {
    let dir_entries = fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in dir_entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        // Skip hidden files if not requested
        if !include_hidden && name.starts_with('.') {
            continue;
        }

        let metadata = entry
            .metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let path_relative = entry.path();
        let path_str = path_relative.to_string_lossy();

        let entry_json = serde_json::json!({
            "name": name,
            "path": entry.path().strip_prefix(path)
                .unwrap_or(&entry.path())
                .to_string_lossy(),
            "full_path": path_str,
            "is_directory": metadata.is_dir(),
            "size": if metadata.is_file() { Some(metadata.len()) } else { None },
            "modified": metadata.modified().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| {
                        chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                            .unwrap()
                            .to_rfc3339()
                    })
            }),
        });

        entries.push(entry_json);
    }

    Ok(())
}

/// Recursively reads directory tree
///
/// # Arguments
/// * `path` - Directory path to read
/// * `current_depth` - Current recursion depth (starts at 0)
/// * `max_depth` - Maximum depth to recurse (None = unlimited)
/// * `include_hidden` - Whether to include hidden files
/// * `entries` - Mutable vector to append entries to
pub(super) fn read_dir_recursive(
    path: &Path,
    current_depth: usize,
    max_depth: Option<usize>,
    include_hidden: bool,
    entries: &mut Vec<serde_json::Value>,
) -> Result<(), String> {
    // Check depth limit
    if let Some(max) = max_depth {
        if current_depth > max {
            return Ok(());
        }
    }

    let dir_entries = fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory at depth {}: {}", current_depth, e))?;

    for entry in dir_entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        // Skip hidden files if not requested
        if !include_hidden && name.starts_with('.') {
            continue;
        }

        let metadata = entry
            .metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let entry_path = entry.path();
        let path_str = entry_path.to_string_lossy();

        let entry_json = serde_json::json!({
            "name": name,
            "path": entry_path.strip_prefix(path)
                .unwrap_or(&entry_path)
                .to_string_lossy(),
            "full_path": path_str,
            "is_directory": metadata.is_dir(),
            "size": if metadata.is_file() { Some(metadata.len()) } else { None },
            "modified": metadata.modified().ok().and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| {
                        chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                            .unwrap()
                            .to_rfc3339()
                    })
            }),
        });

        entries.push(entry_json);

        // Recurse into subdirectories
        if metadata.is_dir() {
            read_dir_recursive(
                &entry_path,
                current_depth + 1,
                max_depth,
                include_hidden,
                entries,
            )?;
        }
    }

    Ok(())
}
```

**步骤 2**: 在 `src-tauri/src/extensions/directory/mod.rs` 中声明模块

```rust
// 在文件开头添加
mod utils;

// 导出内容保持不变
pub use directory_list::DirectoryListCapability;
// ...
```

**步骤 3**: 在 `directory_list.rs` 和 `directory_refresh.rs` 中使用

```rust
// 在文件开头添加
use super::utils::{read_dir_single, read_dir_recursive};

// 删除原有的 read_dir_single 和 read_dir_recursive 函数定义（第 89-175 行）
```

**验证**:
```bash
cargo test -p elfiee --test directory_tests
```

---

### 问题 3: TOCTOU 竞态条件漏洞

**位置**: `src-tauri/src/extensions/directory/directory_create.rs:70-93`

**问题描述**:

当前代码存在 Time-of-check to Time-of-use (TOCTOU) 漏洞：

```rust
// Step 6: Check if path already exists
if full_path.exists() {  // ← 检查时间点
    return Err(format!("Path '{}' already exists", payload.path));
}

// Step 7: Create file or directory
if payload.is_directory {
    fs::create_dir(&full_path)  // ← 使用时间点（中间有时间窗口）
        .map_err(|e| format!("Failed to create directory: {}", e))?;
} else {
    fs::write(&full_path, &payload.content)
        .map_err(|e| format!("Failed to create file: {}", e))?;
}

// Step 8: Perform final canonicalization for security verification
let canonical_path = full_path
    .canonicalize()  // ← 安全检查在写入之后！
    .map_err(|e| format!("Failed to canonicalize created path: {}", e))?;
```

**攻击场景**:
1. 攻击者监听文件系统
2. 在 `exists()` 检查和 `fs::write()` 之间，创建一个 symlink
3. 应用程序写入到攻击者指定的位置

**修复方案**:

```rust
// src-tauri/src/extensions/directory/directory_create.rs
// 修改 handle_create 函数（第 70-103 行）

// Step 6: Security check on PARENT directory BEFORE any file creation
if let Some(parent) = full_path.parent() {
    // Ensure parent exists
    if !parent.exists() {
        return Err(format!(
            "Parent directory for '{}' does not exist",
            payload.path
        ));
    }

    // Canonicalize parent and verify it's within root
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize parent directory: {}", e))?;

    if !canonical_parent.starts_with(&canonical_root) {
        return Err(format!(
            "Path '{}' attempts to escape root directory",
            payload.path
        ));
    }
}

// Step 7: Check if path already exists
if full_path.exists() {
    return Err(format!("Path '{}' already exists", payload.path));
}

// Step 8: Now safe to create file/directory
if payload.is_directory {
    fs::create_dir(&full_path)
        .map_err(|e| format!("Failed to create directory: {}", e))?;
} else {
    fs::write(&full_path, &payload.content)
        .map_err(|e| format!("Failed to create file: {}", e))?;
}

// Step 9: Final verification (paranoia check)
let canonical_path = full_path
    .canonicalize()
    .map_err(|e| format!("Failed to verify created path: {}", e))?;

if !canonical_path.starts_with(&canonical_root) {
    // Security violation detected - rollback
    let rollback_result = if payload.is_directory {
        fs::remove_dir(&full_path)
    } else {
        fs::remove_file(&full_path)
    };

    if let Err(e) = rollback_result {
        eprintln!("Failed to rollback insecure creation: {}", e);
    }

    return Err(format!(
        "Security violation: created path '{}' is outside root directory",
        payload.path
    ));
}

// Step 10: Create event (rest remains unchanged)
// ...
```

**关键改进**:
1. **父目录预检查**: 在创建文件前验证父目录的安全性
2. **原子性检查**: 减少 TOCTOU 窗口
3. **事后回滚**: 如果最终验证失败，删除已创建的文件

---

### 问题 4: 缺少 Symlink 攻击防护

**位置**:
- `src-tauri/src/extensions/directory/directory_delete.rs`
- `src-tauri/src/extensions/directory/directory_rename.rs`

**问题描述**:

delete 和 rename 操作没有检查目标是否为 symlink，攻击者可以：
1. 创建 symlink 指向系统敏感文件
2. 通过 delete 操作删除敏感文件
3. 通过 rename 操作移动敏感文件

**修复方案 A: directory_delete.rs**

```rust
// src-tauri/src/extensions/directory/directory_delete.rs
// 在 handle_delete 函数中添加（第 52 行之后，在删除操作之前）

// Step 5: Check for symlink (security)
let metadata = fs::symlink_metadata(&full_path)
    .map_err(|e| format!("Failed to read path metadata: {}", e))?;

if metadata.is_symlink() {
    return Err(format!(
        "Deleting symlinks is not supported for security reasons. \
         Path '{}' is a symbolic link.",
        payload.path
    ));
}

// Step 6: Verify path exists (existing check - line 55)
if !full_path.exists() {
    return Err(format!("Path '{}' does not exist", payload.path));
}

// Step 7: Delete file or directory (existing logic)
// ...
```

**修复方案 B: directory_rename.rs**

```rust
// src-tauri/src/extensions/directory/directory_rename.rs
// 在 handle_rename 函数中添加（第 88 行之后，在重命名操作之前）

// Step 8: Check for symlink in old path (security)
let old_metadata = fs::symlink_metadata(&old_full_path)
    .map_err(|e| format!("Failed to read old path metadata: {}", e))?;

if old_metadata.is_symlink() {
    return Err(format!(
        "Renaming symlinks is not supported for security reasons. \
         Path '{}' is a symbolic link.",
        payload.old_path
    ));
}

// Step 9: Verify old path exists (existing check - line 91)
if !old_full_path.exists() {
    return Err(format!("Old path '{}' does not exist", payload.old_path));
}

// Step 10: Perform rename (existing logic)
// ...
```

**关键点**:
- 使用 `fs::symlink_metadata()` 而非 `fs::metadata()`
- `symlink_metadata()` 检查 symlink 本身，不跟随链接
- `metadata()` 会跟随 symlink，无法检测到链接

---

## 🟡 P1: Medium Priority（推荐修复）

### 问题 5: Pattern Matching 递归深度问题

**位置**: `src-tauri/src/extensions/directory/directory_search.rs:187-226`

**问题描述**:

当前递归实现对于复杂模式（如 `a*a*a*a*b`）可能导致：
- 栈溢出
- 性能退化（指数级回溯）

**修复方案**:

```rust
// src-tauri/src/extensions/directory/directory_search.rs

// Step 1: 在 handle_search 函数中添加早期验证（第 33 行之后）
let payload: DirectorySearchPayload = serde_json::from_value(cmd.payload.clone())
    .map_err(|e| format!("Invalid DirectorySearchPayload: {}", e))?;

// Validate pattern is not empty
if payload.pattern.trim().is_empty() {
    return Err("Search pattern must not be empty".into());
}

// Limit number of wildcards to prevent performance issues
let wildcard_count = payload.pattern.chars().filter(|&c| c == '*').count();
if wildcard_count > 10 {
    return Err(format!(
        "Search pattern contains too many wildcards ({}). Maximum allowed is 10.",
        wildcard_count
    ));
}

// Step 2: 修改 matches_pattern_impl 添加深度限制（第 187 行开始）
fn matches_pattern_impl(
    filename: &[char],
    f_idx: usize,
    pattern: &[char],
    p_idx: usize,
    depth: usize,  // 新增参数
) -> bool {
    // Prevent deep recursion
    if depth > 100 {
        return false;
    }

    if p_idx == pattern.len() {
        return f_idx == filename.len();
    }
    if f_idx == filename.len() {
        return pattern[p_idx..].iter().all(|&c| c == '*');
    }

    match pattern[p_idx] {
        '*' => {
            // Try skipping the wildcard
            if matches_pattern_impl(filename, f_idx, pattern, p_idx + 1, depth + 1) {
                return true;
            }
            // Try consuming one character
            matches_pattern_impl(filename, f_idx + 1, pattern, p_idx, depth + 1)
        }
        '?' => {
            matches_pattern_impl(filename, f_idx + 1, pattern, p_idx + 1, depth + 1)
        }
        c => {
            if filename[f_idx] == c {
                matches_pattern_impl(filename, f_idx + 1, pattern, p_idx + 1, depth + 1)
            } else {
                false
            }
        }
    }
}

// Step 3: 更新 matches_pattern 调用（第 180 行）
fn matches_pattern(filename: &str, pattern: &str) -> bool {
    let filename_chars: Vec<char> = filename.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();
    matches_pattern_impl(&filename_chars, 0, &pattern_chars, 0, 0)  // 初始 depth = 0
}
```

**注释**: 对于生产环境，建议使用成熟的 `glob` 或 `globset` crate。

---

### 问题 6: 错误消息不一致

**位置**: 多个文件中的错误消息

**问题描述**:

部分错误消息暴露了内部完整路径，可能造成信息泄露：

```rust
// directory_create.rs:64 - 暴露完整路径
return Err(format!("Parent directory '{}' does not exist", parent.display()));
```

**修复方案**:

统一原则：**错误消息只使用用户提供的相对路径**

```rust
// directory_create.rs:64
// 修改前
return Err(format!("Parent directory '{}' does not exist", parent.display()));

// 修改后
return Err(format!("Parent directory for '{}' does not exist", payload.path));
```

**需要检查的文件**:
- `directory_create.rs`: 所有错误消息
- `directory_delete.rs`: 所有错误消息
- `directory_rename.rs`: 所有错误消息
- `directory_list.rs`: 所有错误消息
- `directory_refresh.rs`: 所有错误消息

**检查清单**:
```bash
# 搜索可能暴露完整路径的错误消息
rg "full_path|canonical_path|display\(\)" src-tauri/src/extensions/directory/
```

---

### 问题 7: directory.watch 功能限制未说明

**位置**: `src-tauri/src/extensions/directory/directory_watch.rs`

**问题描述**:

当前实现只是设置一个标志，没有实际的文件系统监听功能，但文档没有说明这个限制。

**修复方案**:

```rust
// src-tauri/src/extensions/directory/directory_watch.rs
// 在 capability 宏之前添加详细文档注释

/// Enables or disables directory watching
///
/// **Current Implementation**: This capability only sets a `watch_enabled` flag in the block's
/// contents. It does NOT implement actual filesystem monitoring yet.
///
/// **Future Plan**: When `watch_enabled` is true, the system will:
/// 1. Use the `notify` crate to monitor filesystem events for the directory
/// 2. Emit events when files are created, modified, or deleted
/// 3. Automatically trigger `directory.refresh` when changes are detected
/// 4. Notify connected clients via Tauri events for real-time updates
///
/// **Current Behavior**:
/// - Setting `watch_enabled: true` stores the flag in `block.contents`
/// - No actual filesystem watching occurs
/// - Clients must manually call `directory.refresh` to update the listing
///
/// **Integration Requirements** (for future implementation):
/// - Add `notify` crate dependency
/// - Create a watcher manager in the Engine
/// - Map file events to Elfiee events
/// - Handle watcher lifecycle (start/stop on open/close)
///
/// # Payload
/// ```json
/// {
///   "watch_enabled": true  // or false to disable
/// }
/// ```
///
/// # Returns
/// Event with updated `watch_enabled` status in block contents
///
/// # Example
/// ```rust
/// let payload = DirectoryWatchPayload { watch_enabled: true };
/// let events = handle_watch(&command, Some(&block))?;
/// // block.contents.watch_enabled is now true
/// ```
#[capability(id = "directory.watch", target = "directory")]
fn handle_watch(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    // ... existing implementation
}
```

**额外建议**: 在 `DEVELOPMENT_GUIDE.md` 中添加 "Known Limitations" 章节，说明 watch 功能的当前状态。

---

## 🧪 测试补充计划

### 当前测试架构

根据 `src-tauri/src/extensions/directory/tests.rs`，现有测试分为 4 类：

```
Total: 36 tests
├── Payload Tests (7) ............ 验证 JSON 反序列化
├── Functionality Tests (7) ...... 验证基本功能
├── Authorization Tests (21) ..... 验证 CBAC 授权
└── Workflow Test (1) ............ 验证完整流程
```

### 新增测试归类方案

PR review 建议的测试应**归入现有类别**，避免创建新的测试类别：

| 新测试类型 | 归入类别 | 数量 | 说明 |
|-----------|---------|------|------|
| Symlink 攻击测试 | **Functionality Tests** | 3 | 增强现有功能测试的安全覆盖 |
| TOCTOU 竞态测试 | **Workflow Test** | 2 | 测试并发场景，属于工作流范畴 |
| Pattern 边界测试 | **Functionality Tests** | 4 | 增强 search 功能的边界覆盖 |
| Path 规范化测试 | **Functionality Tests** | 3 | 增强 create/list 的边界覆盖 |
| Edge Case 测试 | **Functionality Tests** | 4 | 增强各 capability 的边界覆盖 |
| 资源耗尽测试 | **新增: Performance Tests** | 3 | 标记为 `#[ignore]`，按需运行 |

**设计原则**:
1. **不增加新的测试类别**（除了性能测试）
2. **Functionality Tests 扩展**：从 7 个增加到 21 个（每个 capability 3 个测试：基本 + 安全 + 边界）
3. **Workflow Tests 扩展**：从 1 个增加到 3 个（基本流程 + 并发场景）
4. **Performance Tests**：独立标记，不计入常规测试

### 测试文件结构（修改后）

```rust
// src-tauri/src/extensions/directory/tests.rs

// ============================================================================
// PART 1: Payload Tests (7 tests) - 保持不变
// ============================================================================

// ============================================================================
// PART 2: Functionality Tests (21 tests) - 从 7 个扩展到 21 个
// ============================================================================

// 每个 capability 3 个测试：基本 + 安全 + 边界

// --- directory.list (3 tests) ---
#[test]
fn test_list_basic() { /* 现有测试 */ }

#[test]
fn test_list_path_traversal_blocked() {
    // 新增：测试 payload.root = "../../../etc" 被拒绝
}

#[test]
fn test_list_max_depth_zero() {
    // 新增：测试 max_depth = Some(0) 的边界行为
}

// --- directory.create (3 tests) ---
#[test]
fn test_create_basic() { /* 现有测试 */ }

#[test]
fn test_create_rejects_symlink_parent() {
    // 新增：父目录为 symlink 时被拒绝
}

#[test]
fn test_create_normalizes_dot_slash_path() {
    // 新增：payload.path = "./././test.txt" 正确处理
}

// --- directory.delete (3 tests) ---
#[test]
fn test_delete_basic() { /* 现有测试 */ }

#[test]
fn test_delete_rejects_symlink() {
    // 新增：拒绝删除 symlink
}

#[test]
fn test_delete_empty_dir_without_recursive() {
    // 新增：recursive=false 可以删除空目录
}

// --- directory.rename (3 tests) ---
#[test]
fn test_rename_basic() { /* 现有测试 */ }

#[test]
fn test_rename_rejects_symlink() {
    // 新增：拒绝重命名 symlink
}

#[test]
fn test_rename_normalizes_parent_ref() {
    // 新增：payload.new_path = "foo/../bar.txt" 正确处理
}

// --- directory.refresh (3 tests) ---
#[test]
fn test_refresh_basic() { /* 现有测试 */ }

#[test]
fn test_refresh_detects_external_changes() {
    // 新增：外部修改文件系统后 refresh 能检测到
}

#[test]
fn test_refresh_with_different_config() {
    // 新增：与上次 list 不同的 include_hidden 配置
}

// --- directory.watch (3 tests) ---
#[test]
fn test_watch_basic() { /* 现有测试 */ }

#[test]
fn test_watch_flag_persists() {
    // 新增：验证 watch_enabled 标志持久化
}

#[test]
fn test_watch_no_actual_monitoring() {
    // 新增：文档化当前限制（不会自动触发事件）
}

// --- directory.search (3 tests) ---
#[test]
fn test_search_basic() { /* 现有测试 */ }

#[test]
fn test_search_multiple_consecutive_wildcards() {
    // 新增：pattern = "***test***" 不会栈溢出
}

#[test]
fn test_search_rejects_too_many_wildcards() {
    // 新增：pattern 包含 11 个 * 被拒绝
}

// ============================================================================
// PART 3: Authorization Tests (21 tests) - 保持不变
// ============================================================================

// ============================================================================
// PART 4: Workflow Tests (3 tests) - 从 1 个扩展到 3 个
// ============================================================================

#[test]
fn test_full_workflow() { /* 现有测试 */ }

#[test]
fn test_concurrent_create_same_path() {
    // 新增：测试 TOCTOU 防护
    // 两个线程同时创建同一文件，只有一个成功
}

#[test]
fn test_concurrent_create_delete() {
    // 新增：一个线程创建文件，另一个删除父目录
    // 验证最终状态一致性
}

// ============================================================================
// PART 5: Performance Tests (3 tests) - 新增，标记为 #[ignore]
// ============================================================================

#[test]
#[ignore]
fn test_list_deeply_nested_directory() {
    // 创建 1000 层嵌套目录
    // 验证递归 list 不会栈溢出
    // 验证性能 < 5 秒
}

#[test]
#[ignore]
fn test_list_large_directory() {
    // 创建 10,000 个文件的目录
    // 验证 list 能成功处理
    // 验证内存使用合理
}

#[test]
#[ignore]
fn test_recursive_delete_deep_tree() {
    // 创建深度嵌套目录树
    // 验证递归删除成功
    // 验证无资源泄漏
}
```

### 测试数量汇总

| 类别 | 当前 | 修改后 | 新增 |
|-----|------|--------|------|
| Payload Tests | 7 | 7 | 0 |
| Functionality Tests | 7 | 21 | +14 |
| Authorization Tests | 21 | 21 | 0 |
| Workflow Tests | 1 | 3 | +2 |
| Performance Tests | 0 | 3 | +3 |
| **总计** | **36** | **55** | **+19** |

### 运行测试

```bash
# 运行所有测试（不包括性能测试）
cargo test -p elfiee --test directory_tests

# 运行性能测试（需要明确指定）
cargo test -p elfiee --test directory_tests -- --ignored

# 运行所有测试（包括性能测试）
cargo test -p elfiee --test directory_tests -- --include-ignored
```

### 测试优先级

1. **P0 (必须)**: 修复问题 1-4 后，现有 36 个测试必须全部通过
2. **P1 (推荐)**: 添加 14 个安全和边界测试（Functionality 扩展）
3. **P2 (推荐)**: 添加 2 个并发测试（Workflow 扩展）
4. **P3 (可选)**: 添加 3 个性能测试（Performance Tests）

---

## 实施步骤

### Phase 1: Critical Fixes（2 小时）

**目标**: 修复所有 P0 问题，确保安全性

```bash
# 步骤 1: 创建 utils 模块（问题 2）
touch src-tauri/src/extensions/directory/utils.rs
# 编辑 utils.rs，移动共享函数
# 修改 mod.rs 声明模块
# 修改 directory_list.rs 和 directory_refresh.rs 导入

# 步骤 2: 修复路径一致性（问题 1）
# 编辑 directory_list.rs handle_list 函数
# 编辑 directory_refresh.rs handle_refresh 函数
# 更新相关测试

# 步骤 3: 修复 TOCTOU（问题 3）
# 编辑 directory_create.rs handle_create 函数

# 步骤 4: 添加 Symlink 防护（问题 4）
# 编辑 directory_delete.rs handle_delete 函数
# 编辑 directory_rename.rs handle_rename 函数

# 验证
cargo test -p elfiee --test directory_tests
```

### Phase 2: Quality Improvements（45 分钟）

**目标**: 修复 P1 问题，提升代码质量

```bash
# 步骤 5: 优化 Pattern Matching（问题 5）
# 编辑 directory_search.rs 添加验证和深度限制

# 步骤 6: 统一错误消息（问题 6）
# 搜索所有暴露完整路径的错误消息并修复
rg "full_path|canonical_path|display\(\)" src-tauri/src/extensions/directory/

# 步骤 7: 文档化 Watch 限制（问题 7）
# 编辑 directory_watch.rs 添加详细注释

# 验证
cargo test -p elfiee --test directory_tests
cargo clippy --all-targets
```

### Phase 3: Testing & Documentation（2.5 小时）

**目标**: 添加测试覆盖，更新文档

```bash
# 步骤 8: 添加安全测试（14 个 Functionality Tests）
# 编辑 tests.rs，添加 symlink、边界测试

# 步骤 9: 添加并发测试（2 个 Workflow Tests）
# 编辑 tests.rs，添加 TOCTOU 并发场景

# 步骤 10: 添加性能测试（3 个 Performance Tests，可选）
# 编辑 tests.rs，添加 #[ignore] 标记的测试

# 步骤 11: 更新文档（问题 8）
# 编辑 DEVELOPMENT_GUIDE.md

# 最终验证
cargo test -p elfiee --test directory_tests
cargo test -p elfiee --test directory_tests -- --include-ignored
```

---

## 验收标准

### Code Changes
- [ ] 所有 P0 问题已修复
- [ ] 所有 P1 问题已修复（推荐）
- [ ] 代码通过 `cargo clippy` 检查
- [ ] 代码格式化 `cargo fmt`

### Testing
- [ ] 所有现有测试通过（36 个）
- [ ] 新增安全测试通过（14 个，P1）
- [ ] 新增并发测试通过（2 个，P1）
- [ ] 性能测试可运行（3 个，P2 可选）
- [ ] 最终测试数量：52+ 个（不含性能测试）

### Documentation
- [ ] directory.watch 限制已文档化
- [ ] directory-be-fix.md 完成（本文档）

### Security Review
- [ ] TOCTOU 漏洞已修复
- [ ] Symlink 攻击已防护
- [ ] 路径遍历防护已验证
- [ ] 错误消息不泄露内部路径

---

## 相关文档

- [Directory Extension Progress](./directory-progress.md) - 开发进度跟踪
- [Directory Extension Guide](./directory/DEVELOPMENT_GUIDE.md) - 使用指南
- [Extension Development Guide](../../guides/EXTENSION_DEVELOPMENT.md) - 扩展开发通用指南
- [Frontend Development Guide](./directory-fe.md) - 前端开发指南（stashed）

---

## 修复记录

| 日期 | 问题 | 状态 | 提交 |
|-----|------|------|------|
| TBD | 问题 1 | Pending | - |
| TBD | 问题 2 | Pending | - |
| TBD | 问题 3 | Pending | - |
| TBD | 问题 4 | Pending | - |
| TBD | 问题 5 | Pending | - |
| TBD | 问题 6 | Pending | - |
| TBD | 问题 7 | Pending | - |
| TBD | 问题 8 | Pending | - |

---

**下一步**: 按照 Phase 1 → Phase 2 → Phase 3 顺序执行修复

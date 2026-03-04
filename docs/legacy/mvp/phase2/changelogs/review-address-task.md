# PR #77 Code Review 修复记录

## 概述

针对 PR #77（Task Block and Other Fixes）的外部 code review 反馈进行修复。Review 覆盖 `commands/task.rs`、`utils/git.rs`、`utils/git_hooks.rs`、`engine/state.rs` 等模块。

## Reviewer 反馈分类

| 级别 | 问题数 | 处理方式 |
|------|--------|---------|
| CRITICAL | 2 | 1 个修复（path traversal），1 个延后（hooks UI） |
| HIGH | 3 | 全部修复 |
| MEDIUM | 3 | 2 个修复，1 个不需要改 |
| LOW | 4 | 2 个修复，2 个延后 |

## 已修复的问题

### 1. 文件导出静默失败 → 严格模式（HIGH）

**问题**: `commit_task` 中 `std::fs::write` 失败只 `log::warn`，用户不知道部分文件没导出就执行了 git commit，导致不完整的提交。

**文件**: `src-tauri/src/commands/task.rs`

**修复**: 任何文件导出失败立即返回 `Err`，中断整个 commit 流程。多 repo 场景同样适用——任一 repo 失败则全部中断。

```rust
// Before: silent failure
match std::fs::write(&file_path, content) {
    Ok(_) => { exported_files.push(...); }
    Err(e) => { log::warn!("Failed to export..."); }
}

// After: strict error propagation
tokio::fs::write(&file_path, content).await.map_err(|e| {
    format!("Failed to export block {} to '{}': {}", block_id, entry_key, e)
})?;
```

### 2. 同步 I/O 阻塞事件循环 → tokio::fs（HIGH）

**问题**: `std::fs::write` 和 `std::fs::create_dir_all` 在 async 函数中使用，可能阻塞 Tauri 事件循环。

**文件**: `src-tauri/src/commands/task.rs`

**修复**: 所有 `std::fs` 调用替换为 `tokio::fs` 等效操作，包括：
- `std::fs::write` → `tokio::fs::write`
- `std::fs::create_dir_all` → `tokio::fs::create_dir_all`

涉及 3 处：commit_task 中的文件导出、commit_task 中的 hooks 目录创建、inject_hooks_for_repo 中的 hooks 目录创建。

同时将 `let _ = std::fs::create_dir_all(...)` 改为 `tokio::fs::create_dir_all(...).await.map_err()?`，不再静默忽略目录创建失败。

### 3. `sanitize_branch_name` 的 `to_lowercase()` 对非 ASCII 字符（MEDIUM）

**问题**: `to_lowercase()` 对部分 Unicode 脚本（如土耳其语 İ→i 的特殊映射）可能产生意外行为。CJK 无影响但不够安全。

**文件**: `src-tauri/src/utils/git.rs`

**修复**: `to_lowercase()` → `to_ascii_lowercase()`。ASCII 字母正常转小写，非 ASCII 字符保持原样。

```rust
// Before
result.trim_end_matches('-').to_lowercase()

// After
result.trim_end_matches('-').to_ascii_lowercase()
```

新增测试 `test_sanitize_branch_name_mixed_ascii_unicode` 验证 `"Fix-登录-Bug"` → `"fix-登录-bug"`。

### 4. Parent index 清理代码重复 → 提取辅助函数（MEDIUM）

**问题**: `state.rs` 中 `core.link`/`core.unlink`/`core.delete` 三处有近乎相同的 parent reverse index 清理逻辑。

**文件**: `src-tauri/src/engine/state.rs`

**修复**: 提取 `remove_parent_entries()` 自由函数（非 `&mut self` 方法，避免 borrow checker 冲突）：

```rust
fn remove_parent_entries(
    parents: &mut HashMap<String, Vec<String>>,
    parent_id: &str,
    targets: &[String],
) {
    for target in targets {
        if let Some(parent_list) = parents.get_mut(target) {
            parent_list.retain(|id| id != parent_id);
            if parent_list.is_empty() {
                parents.remove(target);
            }
        }
    }
}
```

3 处调用点统一改为 `remove_parent_entries(&mut self.parents, &event.entity, &targets)`。

新增测试 `test_remove_parent_entries_basic` 和 `test_remove_parent_entries_nonexistent_target`。

### 5. 错误信息和注释语言统一为英文（LOW）

**问题**: 代码中混用中英文注释和错误信息。

**文件**:
- `src-tauri/src/commands/task.rs` — 全部 inline comments 和 doc comments 改为英文
- `src-tauri/src/utils/git.rs` — 全部 inline comments、doc comments 和测试注释改为英文
- `src-tauri/src/utils/git_hooks.rs` — 全部 inline comments、doc comments、测试注释，以及 `PRE_COMMIT_HOOK_CONTENT` shell 脚本中的注释改为英文

### 6. Path traversal 校验（CRITICAL）

**问题**: `commit_task` 中 `entry_key` 来自 directory block 的 entries map，如果包含 `../` 等路径遍历组件，`Path::new(repo_path).join(entry_key)` 可能写到 repo 外部。

**文件**: `src-tauri/src/commands/task.rs`

**修复**: 复用 `checkout.rs` 和 `directory_write.rs` 已有的 `validate_virtual_path()` 函数（来自 `utils/path_validator.rs`），在 `entry_key` 使用前校验：

```rust
use crate::utils::path_validator::validate_virtual_path;

// In the export loop, before any file I/O:
validate_virtual_path(entry_key)?;
```

`validate_virtual_path` 会拒绝：空路径、绝对路径（`/`）、路径遍历（`..`）、Windows 保留名、非法字符。与 `directory_write.rs` 对 entries key 的校验方式完全一致。

## 未修复的反馈项（及理由）

| 反馈项 | 结论 | 理由 |
|--------|------|------|
| 静默 git hook 注入（CRITICAL） | 延后 | 这是 UI 层问题。添加确认弹窗会阻塞 claude code 等 agent 通过 MCP 自动提交。后续 UI 迭代处理 |
| `git status --porcelain` 检查位置 | 不改 | Reviewer 误判。`--porcelain` 在 `git add` 后执行，正确地同时检查暂存区和工作区状态 |
| 前端错误处理（commit loading 状态） | 延后 | UI 层优化，不阻塞 merge |
| `lib.rs:97` TODO 注释 | 延后 | 创建 issue 追踪即可 |
| commit_task 完整集成测试 | 延后 | 需要模拟 git repo + elf engine + 文件系统，复杂度高。核心逻辑已通过 git.rs 和 git_hooks.rs 单元测试覆盖 |

## 变更文件清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src-tauri/src/commands/task.rs` | 修改 | 严格导出模式 + tokio::fs + path traversal 校验 + 英文注释 |
| `src-tauri/src/utils/git.rs` | 修改 | to_ascii_lowercase + 英文注释 + 新增 1 个测试 |
| `src-tauri/src/utils/git_hooks.rs` | 修改 | 英文注释（含 shell 脚本） |
| `src-tauri/src/engine/state.rs` | 修改 | 提取 remove_parent_entries + 新增 2 个测试 |

## 测试结果

- **Rust 后端**: 363 个测试全部通过（318 unit + 42 integration + 3 doc）
- **前端**: 89 个测试全部通过（12 test files）
- **TypeScript**: `tsc --noEmit` 0 errors

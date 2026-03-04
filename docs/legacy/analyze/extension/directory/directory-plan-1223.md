# Directory Extension 实施计划

> **版本**: v1.0 (2025-12-23)
> **状态**: 实施阶段
> **依赖文档**: [directory-redesign1223.md](./directory-redesign1223.md)
> **目标**: 使用 elfiee-ext-gen 工具逐步实现 Directory Extension

---

## 目录

- [1. 前置条件检查](#1-前置条件检查)
- [2. 分支策略](#2-分支策略)
- [3. Phase 0: Core Capabilities 补充](#3-phase-0-core-capabilities-补充)
- [4. Phase 1: 工具模块开发](#4-phase-1-工具模块开发)
- [5. Phase 2: 使用 elfiee-ext-gen 生成骨架](#5-phase-2-使用-elfiee-ext-gen-生成骨架)
- [6. Phase 3: 实现基础 Capabilities](#6-phase-3-实现基础-capabilities)
- [7. Phase 4: 实现高级 Capabilities](#7-phase-4-实现高级-capabilities)
- [8. Phase 5: StateProjector 扩展](#8-phase-5-stateprojector-扩展)
- [9. Phase 6: 集成测试与验证](#9-phase-6-集成测试与验证)
- [10. 里程碑与检查点](#10-里程碑与检查点)

---

## 1. 前置条件检查

### 1.1 缺失的 Core Capabilities

根据设计文档 1.4 节，Directory Extension 依赖以下 Core Capabilities：

| Capability | 当前状态 | 位置 | 说明 |
|-----------|---------|------|------|
| `core.create` | ✅ 已存在 | `src-tauri/src/capabilities/builtins/create.rs` | 创建 Block |
| `core.delete` | ✅ 已存在 | `src-tauri/src/capabilities/builtins/delete.rs` | 删除 Block |
| `core.update_metadata` | ✅ 已存在 | `src-tauri/src/capabilities/builtins/update_metadata.rs` | 更新 metadata |
| `core.rename` | ❌ 不存在 | - | 重命名 Block.name |
| `core.change_type` | ❌ 不存在 | - | 修改 Block.block_type |

### 1.2 StateProjector 支持情况

当前 `StateProjector` (src-tauri/src/engine/state.rs) 支持的事件类型：

- ✅ `core.create` (line 65)
- ✅ `core.delete` (line 162)
- ✅ `core.update_metadata` (line 167)
- ✅ `*.write` / `*.link` (line 113)
- ✅ `core.unlink` (line 150)
- ✅ `core.grant` / `core.revoke` (line 189)
- ❌ `core.rename` - 需要新增
- ❌ `core.change_type` - 需要新增
- ❌ `directory.*` - 需要新增

### 1.3 依赖的 Rust Crates

需要在 `src-tauri/Cargo.toml` 中确认以下依赖：

```toml
[dependencies]
walkdir = "2"          # 目录遍历
regex = "1"            # 文件过滤模式匹配
```

检查命令：
```bash
cd src-tauri
grep -E "walkdir|regex" Cargo.toml
```

---

## 2. 分支策略

### 2.1 当前分支状态

```bash
# 当前分支
git branch --show-current
# 输出: feat/extension-directory-redesign
```

### 2.2 分支管理方案

由于需要实现 Core Capabilities，采用以下分支策略：

```
dev
 └── feat/core-rename-change-type (新分支)
       ├── 实现 core.rename
       ├── 实现 core.change_type
       ├── 更新 StateProjector
       └── 测试通过后合并到 dev

dev (包含上述更新)
 └── feat/extension-directory-redesign (当前分支，rebase)
       ├── 实现 Directory Extension
       └── 完成后合并到 dev
```

**执行步骤**：

```bash
# Step 1: 创建 core capabilities 分支
git checkout dev
git pull origin dev
git checkout -b feat/core-rename-change-type

# Step 2: 实现 core.rename 和 core.change_type (见 Phase 0)

# Step 3: 合并到 dev
git checkout dev
git merge feat/core-rename-change-type
git push origin dev

# Step 4: rebase directory 分支
git checkout feat/extension-directory-redesign
git rebase dev

# Step 5: 开始实现 Directory Extension (见后续 Phase)
```

---

## 3. Phase 0: Core Capabilities 补充

### 3.1 实现 `core.rename`

#### 文件创建

```bash
# 在分支 feat/core-rename-change-type 上执行
cd src-tauri/src/capabilities/builtins
touch rename.rs
```

#### 代码实现

**文件**: `src-tauri/src/capabilities/builtins/rename.rs`

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Payload for core.rename capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RenamePayload {
    /// New name for the block
    pub name: String,
}

/// Handler for core.rename capability.
///
/// Updates the name field of a block.
#[capability(id = "core.rename", target = "core/*")]
fn handle_rename(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for core.rename")?;

    let payload: RenamePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for core.rename: {}", e))?;

    // Validate name is not empty
    if payload.name.trim().is_empty() {
        return Err("Block name cannot be empty".to_string());
    }

    let event = create_event(
        block.block_id.clone(),
        "core.rename",
        serde_json::json!({ "name": payload.name }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Block;

    #[test]
    fn test_rename_block() {
        let block = Block::new(
            "Old Name".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "alice".to_string(),
            "core.rename".to_string(),
            block.block_id.clone(),
            serde_json::json!({ "name": "New Name" }),
        );

        let events = handle_rename(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value["name"], "New Name");
    }

    #[test]
    fn test_rename_empty_name_fails() {
        let block = Block::new(
            "Old Name".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "alice".to_string(),
            "core.rename".to_string(),
            block.block_id.clone(),
            serde_json::json!({ "name": "" }),
        );

        let result = handle_rename(&cmd, Some(&block));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Block name cannot be empty");
    }

    #[test]
    fn test_rename_requires_block() {
        let cmd = Command::new(
            "alice".to_string(),
            "core.rename".to_string(),
            "block-123".to_string(),
            serde_json::json!({ "name": "New Name" }),
        );

        let result = handle_rename(&cmd, None);
        assert!(result.is_err());
    }
}
```

#### 注册到模块

**文件**: `src-tauri/src/capabilities/builtins/mod.rs`

```rust
// 在文件开头添加
mod rename;

// 在 pub use 部分添加
pub use rename::CoreRenameCapability;
```

#### 注册到 Payload 文件

**文件**: `src-tauri/src/models/payloads.rs`

在文件末尾添加：

```rust
/// Payload for core.rename capability
///
/// This payload is used to rename a block.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RenamePayload {
    /// The new name for the block
    pub name: String,
}
```

#### 更新 StateProjector

**文件**: `src-tauri/src/engine/state.rs`

在 `apply_event` 方法的 `match cap_id` 块中添加（约 line 217 之前）：

```rust
// Block name update
"core.rename" => {
    if let Some(block) = self.blocks.get_mut(&event.entity) {
        if let Some(name) = event.value.get("name").and_then(|v| v.as_str()) {
            block.name = name.to_string();
        }
    }
}
```

#### 注册到 CapabilityRegistry

**文件**: `src-tauri/src/capabilities/registry.rs`

在 `register_core_capabilities` 方法中添加：

```rust
use crate::capabilities::builtins::CoreRenameCapability;

// 在方法体内添加
self.register(Arc::new(CoreRenameCapability));
```

#### 运行测试

```bash
cd src-tauri
cargo test builtins::rename::tests -- --nocapture
```

预期输出：所有测试通过。

---

### 3.2 实现 `core.change_type`

#### 文件创建

```bash
cd src-tauri/src/capabilities/builtins
touch change_type.rs
```

#### 代码实现

**文件**: `src-tauri/src/capabilities/builtins/change_type.rs`

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Payload for core.change_type capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChangeTypePayload {
    /// New block type (e.g., "markdown", "code")
    pub block_type: String,
}

/// Handler for core.change_type capability.
///
/// Changes the block_type field of a block.
/// WARNING: This does not validate that the new type is compatible with existing contents.
#[capability(id = "core.change_type", target = "core/*")]
fn handle_change_type(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for core.change_type")?;

    let payload: ChangeTypePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for core.change_type: {}", e))?;

    // Validate block_type is not empty
    if payload.block_type.trim().is_empty() {
        return Err("Block type cannot be empty".to_string());
    }

    let event = create_event(
        block.block_id.clone(),
        "core.change_type",
        serde_json::json!({ "block_type": payload.block_type }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Block;

    #[test]
    fn test_change_type() {
        let block = Block::new(
            "Test".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "alice".to_string(),
            "core.change_type".to_string(),
            block.block_id.clone(),
            serde_json::json!({ "block_type": "code" }),
        );

        let events = handle_change_type(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value["block_type"], "code");
    }

    #[test]
    fn test_change_type_empty_fails() {
        let block = Block::new(
            "Test".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        let cmd = Command::new(
            "alice".to_string(),
            "core.change_type".to_string(),
            block.block_id.clone(),
            serde_json::json!({ "block_type": "" }),
        );

        let result = handle_change_type(&cmd, Some(&block));
        assert!(result.is_err());
    }

    #[test]
    fn test_change_type_requires_block() {
        let cmd = Command::new(
            "alice".to_string(),
            "core.change_type".to_string(),
            "block-123".to_string(),
            serde_json::json!({ "block_type": "code" }),
        );

        let result = handle_change_type(&cmd, None);
        assert!(result.is_err());
    }
}
```

#### 注册步骤

按照与 `core.rename` 相同的步骤：

1. 更新 `mod.rs`
2. 添加到 `payloads.rs`
3. 更新 `StateProjector` (添加 `"core.change_type"` 分支)
4. 注册到 `CapabilityRegistry`

**StateProjector 新增代码** (src-tauri/src/engine/state.rs):

```rust
// Block type change
"core.change_type" => {
    if let Some(block) = self.blocks.get_mut(&event.entity) {
        if let Some(block_type) = event.value.get("block_type").and_then(|v| v.as_str()) {
            block.block_type = block_type.to_string();
        }
    }
}
```

#### 运行测试

```bash
cd src-tauri
cargo test builtins::change_type::tests -- --nocapture
cargo test engine::state::tests -- --nocapture
```

---

### 3.3 提交并合并 Core Capabilities

```bash
# 确保所有测试通过
cd src-tauri
cargo test

# 提交更改
git add .
git commit -m "feat(core): add core.rename and core.change_type capabilities

- Implement core.rename for updating Block.name
- Implement core.change_type for updating Block.block_type
- Update StateProjector to handle new event types
- Add comprehensive tests for both capabilities"

# 合并到 dev
git checkout dev
git merge feat/core-rename-change-type
git push origin dev

# rebase directory 分支
git checkout feat/extension-directory-redesign
git rebase dev
```

**检查点**：执行 `cargo test` 无错误，所有测试通过。

---

## 4. Phase 1: 工具模块开发

在生成 Directory Extension 骨架之前，先实现独立的工具函数模块。

### 4.1 创建工具目录结构

```bash
cd src-tauri/src
mkdir -p utils
touch utils/mod.rs
touch utils/fs_scanner.rs
touch utils/path_validator.rs
touch utils/block_type_inference.rs
```

### 4.2 实现文件系统扫描器

**文件**: `src-tauri/src/utils/fs_scanner.rs`

```rust
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 文件信息结构
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// 绝对路径
    pub absolute_path: PathBuf,
    /// 相对路径（相对于扫描根目录）
    pub relative_path: String,
    /// 文件名
    pub file_name: String,
    /// 文件扩展名（不含点）
    pub extension: String,
    /// 文件大小（字节）
    pub size: u64,
    /// 是否为目录
    pub is_directory: bool,
}

/// 扫描选项
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// 最大递归深度
    pub max_depth: usize,
    /// 是否跟随符号链接
    pub follow_symlinks: bool,
    /// 是否忽略隐藏文件
    pub ignore_hidden: bool,
    /// 忽略的目录名称列表
    pub ignore_patterns: Vec<String>,
    /// 最大文件大小（字节）
    pub max_file_size: u64,
    /// 最大文件数量
    pub max_files: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: 100,
            follow_symlinks: false,
            ignore_hidden: true,
            ignore_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".DS_Store".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10 MB
            max_files: 10_000,
        }
    }
}

/// 扫描目录并返回文件列表
pub fn scan_directory(root: &Path, options: &ScanOptions) -> Result<Vec<FileInfo>, String> {
    let mut files = Vec::new();
    let mut count = 0;

    let walker = WalkDir::new(root)
        .max_depth(options.max_depth)
        .follow_links(options.follow_symlinks);

    for entry in walker {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        // 检查文件数量限制
        count += 1;
        if count > options.max_files {
            return Err(format!(
                "Too many files (limit: {})",
                options.max_files
            ));
        }

        // 跳过隐藏文件
        if options.ignore_hidden {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }
        }

        // 跳过忽略的目录
        let should_skip = options.ignore_patterns.iter().any(|pattern| {
            path.components()
                .any(|c| c.as_os_str().to_string_lossy() == *pattern)
        });
        if should_skip {
            continue;
        }

        let metadata = entry
            .metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        // 跳过大文件
        if metadata.is_file() && metadata.len() > options.max_file_size {
            log::warn!("Skipping large file: {:?} ({} bytes)", path, metadata.len());
            continue;
        }

        let relative_path = path
            .strip_prefix(root)
            .map_err(|e| format!("Failed to strip prefix: {}", e))?
            .to_string_lossy()
            .to_string();

        // 跳过根目录本身
        if relative_path.is_empty() {
            continue;
        }

        let file_info = FileInfo {
            absolute_path: path.to_path_buf(),
            relative_path: relative_path.clone(),
            file_name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            extension: path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            size: metadata.len(),
            is_directory: metadata.is_dir(),
        };

        files.push(file_info);
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let options = ScanOptions::default();

        let files = scan_directory(temp_dir.path(), &options).unwrap();
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_scan_with_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("file2.md"), "content").unwrap();

        let options = ScanOptions::default();
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.file_name == "file1.txt"));
        assert!(files.iter().any(|f| f.file_name == "file2.md"));
    }

    #[test]
    fn test_scan_ignores_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".hidden"), "content").unwrap();
        fs::write(temp_dir.path().join("visible.txt"), "content").unwrap();

        let options = ScanOptions::default();
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name, "visible.txt");
    }

    #[test]
    fn test_scan_respects_ignore_patterns() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("node_modules")).unwrap();
        fs::write(temp_dir.path().join("node_modules/package.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("main.js"), "code").unwrap();

        let options = ScanOptions::default();
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name, "main.js");
    }
}
```

### 4.3 实现路径安全验证器

**文件**: `src-tauri/src/utils/path_validator.rs`

```rust
use std::fs;
use std::path::Path;

/// 检查路径是否安全（防止路径遍历攻击）
pub fn is_safe_path(path: &Path) -> Result<(), String> {
    // 1. 解析符号链接，获取规范路径
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;

    // 2. 拒绝系统敏感目录
    let forbidden = ["/etc", "/sys", "/proc", "/dev", "/bin", "/sbin"];
    for forbidden_dir in &forbidden {
        if canonical.starts_with(forbidden_dir) {
            return Err(format!(
                "Access to system directory is forbidden: {}",
                forbidden_dir
            ));
        }
    }

    // 3. 检测符号链接
    let metadata = fs::symlink_metadata(&canonical)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;

    if metadata.is_symlink() {
        return Err("Symbolic links are not allowed".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let result = is_safe_path(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_nonexistent_path() {
        let path = Path::new("/nonexistent/path/12345");
        let result = is_safe_path(path);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_forbidden_directory() {
        let path = Path::new("/etc/passwd");
        if path.exists() {
            let result = is_safe_path(path);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .contains("Access to system directory is forbidden"));
        }
    }
}
```

### 4.4 实现 Block 类型推断器

**文件**: `src-tauri/src/utils/block_type_inference.rs`

```rust
/// 根据文件扩展名推断 Block 类型
pub fn infer_block_type(extension: &str) -> Option<String> {
    let ext = extension.to_lowercase();

    match ext.as_str() {
        // Markdown
        "md" | "markdown" => Some("markdown".to_string()),

        // Code
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "h" | "hpp" | "java"
        | "go" | "rb" | "php" | "swift" | "kt" | "cs" | "scala" => Some("code".to_string()),

        // Config/Data
        "json" | "toml" | "yaml" | "yml" | "xml" | "ini" | "conf" => Some("code".to_string()),

        // Shell scripts
        "sh" | "bash" | "zsh" | "fish" => Some("code".to_string()),

        // Web
        "html" | "htm" | "css" | "scss" | "sass" | "less" => Some("code".to_string()),

        // SQL
        "sql" => Some("code".to_string()),

        // 未支持的类型
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_extensions() {
        assert_eq!(infer_block_type("md"), Some("markdown".to_string()));
        assert_eq!(infer_block_type("markdown"), Some("markdown".to_string()));
        assert_eq!(infer_block_type("MD"), Some("markdown".to_string()));
    }

    #[test]
    fn test_code_extensions() {
        assert_eq!(infer_block_type("rs"), Some("code".to_string()));
        assert_eq!(infer_block_type("py"), Some("code".to_string()));
        assert_eq!(infer_block_type("js"), Some("code".to_string()));
        assert_eq!(infer_block_type("json"), Some("code".to_string()));
    }

    #[test]
    fn test_unsupported_extensions() {
        assert_eq!(infer_block_type("png"), None);
        assert_eq!(infer_block_type("jpg"), None);
        assert_eq!(infer_block_type("pdf"), None);
        assert_eq!(infer_block_type("exe"), None);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(infer_block_type("RS"), Some("code".to_string()));
        assert_eq!(infer_block_type("Py"), Some("code".to_string()));
    }
}
```

### 4.5 注册工具模块

**文件**: `src-tauri/src/utils/mod.rs`

```rust
pub mod block_type_inference;
pub mod fs_scanner;
pub mod path_validator;

pub use block_type_inference::infer_block_type;
pub use fs_scanner::{scan_directory, FileInfo, ScanOptions};
pub use path_validator::is_safe_path;
```

**文件**: `src-tauri/src/lib.rs`

在文件开头添加：

```rust
pub mod utils;
```

### 4.6 添加依赖

**文件**: `src-tauri/Cargo.toml`

在 `[dependencies]` 部分添加：

```toml
walkdir = "2"
tempfile = "3"  # 用于测试
```

### 4.7 运行工具模块测试

```bash
cd src-tauri
cargo test utils -- --nocapture
```

**检查点**：所有工具模块测试通过。

---

## 5. Phase 2: 使用 elfiee-ext-gen 生成骨架

### 5.1 生成 Directory Extension

```bash
# 在项目根目录执行
cd /home/yaosh/projects/elfiee

# 生成 Directory Extension
elfiee-ext-gen create \
  -n directory \
  -b directory \
  -c import,export,refresh,create,delete,rename
```

**预期输出**：

```
✅ Extension 'directory' created successfully!

Generated files:
  src-tauri/src/extensions/directory/mod.rs
  src-tauri/src/extensions/directory/directory_import.rs
  src-tauri/src/extensions/directory/directory_export.rs
  src-tauri/src/extensions/directory/directory_refresh.rs
  src-tauri/src/extensions/directory/directory_create.rs
  src-tauri/src/extensions/directory/directory_delete.rs
  src-tauri/src/extensions/directory/directory_rename.rs
  src-tauri/src/extensions/directory/tests.rs
  src-tauri/src/extensions/directory/DEVELOPMENT_GUIDE.md
```

### 5.2 验证生成的文件结构

```bash
cd src-tauri
tree src/extensions/directory
```

**预期输出**：

```
src/extensions/directory/
├── DEVELOPMENT_GUIDE.md
├── directory_create.rs
├── directory_delete.rs
├── directory_export.rs
├── directory_import.rs
├── directory_refresh.rs
├── directory_rename.rs
├── mod.rs
└── tests.rs
```

### 5.3 初步编译检查

```bash
cd src-tauri
cargo check
```

**预期**：编译通过（虽然handler包含`todo!()`）。

---

## 6. Phase 3: 实现基础 Capabilities

按照复杂度从低到高的顺序实现。

### 6.1 实现 `directory.create`

#### Step 1: 定义 Payload

**文件**: `src-tauri/src/extensions/directory/mod.rs`

找到 `DirectoryCreatePayload`，替换为：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryCreatePayload {
    /// 内部虚拟路径（例如 "docs/README.md"）
    pub path: String,

    /// 类型: "file" | "directory"
    #[serde(rename = "type")]
    pub entry_type: String,

    /// 初始内容（仅文件需要，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Block 类型（仅文件需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_type: Option<String>,
}
```

#### Step 2: 实现 Handler

**文件**: `src-tauri/src/extensions/directory/directory_create.rs`

替换 `todo!()` 为实际逻辑：

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use crate::utils::time::now_utc;
use capability_macros::capability;
use serde_json::json;

use super::DirectoryCreatePayload;

#[capability(id = "directory.create", target = "directory")]
fn handle_directory_create(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for directory.create")?;

    let payload: DirectoryCreatePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for directory.create: {}", e))?;

    // Validate path
    if payload.path.is_empty() || payload.path.starts_with('/') {
        return Err("Invalid path format".to_string());
    }

    // Parse current directory contents
    let mut contents: serde_json::Map<String, serde_json::Value> = block
        .contents
        .as_object()
        .cloned()
        .unwrap_or_default();

    let entries = contents
        .get_mut("entries")
        .and_then(|v| v.as_object_mut())
        .ok_or("Invalid directory structure")?;

    // Check if path already exists
    if entries.contains_key(&payload.path) {
        return Err(format!("Path already exists: {}", payload.path));
    }

    let mut events = Vec::new();

    if payload.entry_type == "file" {
        // Create file block
        let file_block_id = uuid::Uuid::new_v4().to_string();
        let file_name = payload
            .path
            .split('/')
            .last()
            .unwrap_or(&payload.path)
            .to_string();
        let block_type = payload.block_type.unwrap_or_else(|| "markdown".to_string());

        // Event 1: Create file block
        events.push(create_event(
            file_block_id.clone(),
            "core.create",
            json!({
                "name": file_name,
                "type": block_type,
                "owner": cmd.editor_id,
                "contents": {
                    "text": payload.content.unwrap_or_default()
                },
                "children": {},
                "metadata": {}
            }),
            &cmd.editor_id,
            1,
        ));

        // Event 2: Add entry to directory
        entries.insert(
            payload.path.clone(),
            json!({
                "id": file_block_id,
                "type": "file",
                "source": "outline",
                "updated_at": now_utc()
            }),
        );
    } else {
        // Create directory entry (no block)
        let dir_id = format!("dir-{}", uuid::Uuid::new_v4());
        entries.insert(
            payload.path.clone(),
            json!({
                "id": dir_id,
                "type": "directory",
                "source": "outline",
                "updated_at": now_utc()
            }),
        );
    }

    // Event 3: Update directory block contents
    events.push(create_event(
        block.block_id.clone(),
        "directory.write",
        json!({ "contents": { "entries": entries } }),
        &cmd.editor_id,
        1,
    ));

    Ok(events)
}
```

#### Step 3: 更新测试

**文件**: `src-tauri/src/extensions/directory/tests.rs`

找到 `test_create_functionality`，更新测试数据：

```rust
#[test]
fn test_create_functionality() {
    let mut block = Block::new(
        "Test Directory".to_string(),
        "directory".to_string(),
        "alice".to_string(),
    );
    block.contents = json!({
        "root_path": "/",
        "entries": {}
    });

    let cmd = Command::new(
        "alice".to_string(),
        "directory.create".to_string(),
        block.block_id.clone(),
        json!({
            "path": "test.md",
            "type": "file",
            "content": "Hello",
            "block_type": "markdown"
        }),
    );

    let events = handle_directory_create(&cmd, Some(&block)).unwrap();
    assert_eq!(events.len(), 2); // core.create + directory.write

    // Verify core.create event
    assert_eq!(events[0].attribute.split('/').nth(1), Some("core.create"));

    // Verify directory.write event
    assert_eq!(events[1].entity, block.block_id);
    let entries = events[1].value["contents"]["entries"].as_object().unwrap();
    assert!(entries.contains_key("test.md"));
}
```

#### Step 4: 运行测试

```bash
cd src-tauri
cargo test directory::tests::test_create -- --nocapture
```

**检查点**：`directory.create` 的所有测试通过。

---

### 6.2 实现 `directory.delete`

#### Step 1: 定义 Payload

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryDeletePayload {
    /// 要删除的虚拟路径
    pub path: String,
}
```

#### Step 2: 实现 Handler

**文件**: `src-tauri/src/extensions/directory/directory_delete.rs`

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;
use serde_json::json;

use super::DirectoryDeletePayload;

#[capability(id = "directory.delete", target = "directory")]
fn handle_directory_delete(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for directory.delete")?;

    let payload: DirectoryDeletePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload: {}", e))?;

    let contents: serde_json::Map<String, serde_json::Value> = block
        .contents
        .as_object()
        .cloned()
        .unwrap_or_default();

    let entries = contents
        .get("entries")
        .and_then(|v| v.as_object())
        .ok_or("Invalid directory structure")?;

    let entry = entries
        .get(&payload.path)
        .ok_or(format!("Path not found: {}", payload.path))?;

    let mut events = Vec::new();

    if entry["type"] == "directory" {
        // Recursively delete children
        let children: Vec<_> = entries
            .iter()
            .filter(|(path, _)| path.starts_with(&payload.path))
            .collect();

        for (_, child_entry) in children {
            if child_entry["type"] == "file" {
                let child_id = child_entry["id"].as_str().unwrap();
                events.push(create_event(
                    child_id.to_string(),
                    "core.delete",
                    json!({}),
                    &cmd.editor_id,
                    1,
                ));
            }
        }
    } else {
        // Delete single file
        let file_id = entry["id"].as_str().ok_or("Missing block ID")?;
        events.push(create_event(
            file_id.to_string(),
            "core.delete",
            json!({}),
            &cmd.editor_id,
            1,
        ));
    }

    // Update directory entries
    let mut new_entries = entries.clone();
    let paths_to_remove: Vec<_> = new_entries
        .keys()
        .filter(|k| k.starts_with(&payload.path))
        .cloned()
        .collect();

    for path in paths_to_remove {
        new_entries.remove(&path);
    }

    events.push(create_event(
        block.block_id.clone(),
        "directory.write",
        json!({ "contents": { "entries": new_entries } }),
        &cmd.editor_id,
        1,
    ));

    Ok(events)
}
```

#### Step 3: 运行测试

```bash
cargo test directory::tests::test_delete -- --nocapture
```

---

### 6.3 实现 `directory.rename`

#### Step 1: 定义 Payload

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryRenamePayload {
    /// 旧路径
    pub old_path: String,
    /// 新路径
    pub new_path: String,
}
```

#### Step 2: 实现 Handler

**文件**: `src-tauri/src/extensions/directory/directory_rename.rs`

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;
use serde_json::json;

use super::DirectoryRenamePayload;

#[capability(id = "directory.rename", target = "directory")]
fn handle_directory_rename(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for directory.rename")?;

    let payload: DirectoryRenamePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload: {}", e))?;

    let contents = block.contents.as_object().ok_or("Invalid contents")?;
    let entries = contents
        .get("entries")
        .and_then(|v| v.as_object())
        .ok_or("Invalid entries")?;

    if !entries.contains_key(&payload.old_path) {
        return Err("Old path not found".to_string());
    }
    if entries.contains_key(&payload.new_path) {
        return Err("New path already exists".to_string());
    }

    let entry = &entries[&payload.old_path];
    let mut events = Vec::new();

    if entry["type"] == "file" {
        // Rename file block
        let file_id = entry["id"].as_str().ok_or("Missing block ID")?;
        let new_filename = payload
            .new_path
            .split('/')
            .last()
            .unwrap_or(&payload.new_path);

        events.push(create_event(
            file_id.to_string(),
            "core.rename",
            json!({ "name": new_filename }),
            &cmd.editor_id,
            1,
        ));
    } else {
        // Rename directory: update all children paths
        let children: Vec<_> = entries
            .iter()
            .filter(|(path, _)| path.starts_with(&payload.old_path))
            .collect();

        for (child_path, child_entry) in children {
            if child_entry["type"] == "file" {
                let new_child_path = child_path.replace(&payload.old_path, &payload.new_path);
                let new_filename = new_child_path.split('/').last().unwrap_or("");

                let file_id = child_entry["id"].as_str().unwrap();
                events.push(create_event(
                    file_id.to_string(),
                    "core.rename",
                    json!({ "name": new_filename }),
                    &cmd.editor_id,
                    1,
                ));
            }
        }
    }

    // Update directory entries
    let mut new_entries = entries.clone();
    let paths_to_rename: Vec<_> = new_entries
        .keys()
        .filter(|k| k.starts_with(&payload.old_path))
        .cloned()
        .collect();

    for old in paths_to_rename {
        if let Some(entry) = new_entries.remove(&old) {
            let new = old.replace(&payload.old_path, &payload.new_path);
            new_entries.insert(new, entry);
        }
    }

    events.push(create_event(
        block.block_id.clone(),
        "directory.write",
        json!({ "contents": { "entries": new_entries } }),
        &cmd.editor_id,
        1,
    ));

    Ok(events)
}
```

#### Step 3: 运行测试

```bash
cargo test directory::tests::test_rename -- --nocapture
```

**检查点**：基础 3 个 Capabilities 测试通过。

---

## 7. Phase 4: 实现高级 Capabilities

### 7.1 实现 `directory.import`

这是最复杂的 Capability。

#### Step 1: 定义 Payload

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryImportPayload {
    /// 外部真实文件系统路径（来源）
    pub source_path: String,

    /// 内部虚拟路径前缀（目标）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
}
```

#### Step 2: 实现 Handler

**文件**: `src-tauri/src/extensions/directory/directory_import.rs`

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use crate::utils::{infer_block_type, is_safe_path, scan_directory, ScanOptions};
use crate::utils::time::now_utc;
use capability_macros::capability;
use serde_json::json;
use std::fs;
use std::path::Path;

use super::DirectoryImportPayload;

#[capability(id = "directory.import", target = "directory")]
fn handle_directory_import(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for directory.import")?;

    let payload: DirectoryImportPayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload: {}", e))?;

    // Validate source path
    let source = Path::new(&payload.source_path);
    is_safe_path(source)?;

    if !source.exists() {
        return Err("Source path does not exist".to_string());
    }

    // Scan directory
    let options = ScanOptions::default();
    let files = scan_directory(source, &options)?;

    let target_prefix = payload.target_path.unwrap_or_else(|| "/".to_string());
    let mut events = Vec::new();
    let mut new_entries = serde_json::Map::new();

    // Parse existing entries
    if let Some(obj) = block.contents.as_object() {
        if let Some(entries) = obj.get("entries").and_then(|v| v.as_object()) {
            new_entries = entries.clone();
        }
    }

    for file_info in files {
        if file_info.is_directory {
            // Add directory entry
            let virtual_path = format!(
                "{}/{}",
                target_prefix.trim_end_matches('/'),
                file_info.relative_path
            );
            let dir_id = format!("dir-{}", uuid::Uuid::new_v4());

            new_entries.insert(
                virtual_path,
                json!({
                    "id": dir_id,
                    "type": "directory",
                    "source": "linked",
                    "updated_at": now_utc()
                }),
            );
        } else {
            // Infer block type
            let block_type = match infer_block_type(&file_info.extension) {
                Some(t) => t,
                None => {
                    log::warn!("Skipping unsupported file: {:?}", file_info.absolute_path);
                    continue;
                }
            };

            // Read file content
            let content = fs::read_to_string(&file_info.absolute_path)
                .map_err(|e| format!("Failed to read file: {}", e))?;

            // Create block
            let file_block_id = uuid::Uuid::new_v4().to_string();

            events.push(create_event(
                file_block_id.clone(),
                "core.create",
                json!({
                    "name": file_info.file_name,
                    "type": block_type,
                    "owner": cmd.editor_id,
                    "contents": {
                        "text": content,
                        "language": file_info.extension
                    },
                    "children": {},
                    "metadata": {}
                }),
                &cmd.editor_id,
                1,
            ));

            // Add entry
            let virtual_path = format!(
                "{}/{}",
                target_prefix.trim_end_matches('/'),
                file_info.relative_path
            );

            new_entries.insert(
                virtual_path,
                json!({
                    "id": file_block_id,
                    "type": "file",
                    "source": "linked",
                    "external_path": file_info.absolute_path.to_string_lossy(),
                    "updated_at": now_utc()
                }),
            );
        }
    }

    // Update directory block
    events.push(create_event(
        block.block_id.clone(),
        "directory.write",
        json!({ "contents": { "entries": new_entries } }),
        &cmd.editor_id,
        1,
    ));

    // Update metadata with external root path
    events.push(create_event(
        block.block_id.clone(),
        "core.update_metadata",
        json!({
            "metadata": {
                "is_repo": true,
                "external_root_path": payload.source_path,
                "last_import": now_utc()
            }
        }),
        &cmd.editor_id,
        1,
    ));

    Ok(events)
}
```

#### Step 3: 运行测试

```bash
cargo test directory::tests::test_import -- --nocapture
```

---

### 7.2 实现 `directory.export`

**架构调整**：
由于 Engine Capability 无法访问其他 Blocks 的内容，`export` 功能拆分为两部分实现：
1. **Engine Capability (`directory.export`)**: 负责权限检查和审计记录。
2. **Tauri Command (`export_directory`)**: 负责实际的 IO 操作。

#### Step 1: 实现 Capability (权限检查站)

**文件**: `src-tauri/src/extensions/directory/directory_export.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryExportPayload {
    pub target_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

#[capability(id = "directory.export", target = "directory")]
fn handle_directory_export(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required")?;
    // 仅用于生成审计日志，实际导出由 Tauri Command 执行
    Ok(vec![create_event(
        block.block_id.clone(),
        "directory.export",
        cmd.payload.clone(), // 记录导出参数
        &cmd.editor_id,
        1,
    )])
}
```

#### Step 2: 实现 Tauri Command (IO 执行者)

**文件**: `src-tauri/src/commands/directory.rs` (需新建或追加)

```rust
#[tauri::command]
pub async fn export_directory(
    state: State<'_, AppState>,
    file_id: String,
    block_id: String,
    payload: DirectoryExportPayload,
) -> Result<(), String> {
    let handle = state.engine_manager.get_engine(&file_id).ok_or("File not open")?;
    let editor_id = state.get_active_editor(&file_id).ok_or("No active editor")?;

    // 1. 调用 Engine 执行权限检查和审计
    let cmd = Command::new(
        editor_id,
        "directory.export".to_string(),
        block_id.clone(),
        serde_json::to_value(&payload).unwrap(),
    );
    handle.process_command(cmd).await?; // 如果无权，这里会报错返回

    // 2. 权限通过，开始导出 IO
    // 遍历 Directory Block 内容
    // 循环调用 handle.get_block() 获取子文件内容
    // 写入 target_path
    
    // 注意：在遍历子文件时，如果 get_block 返回权限错误，应跳过该文件（内容受控）
    
    Ok(())
}
```

---

### 7.3 实现 `directory.refresh`

这是最复杂的 Capability，包含完整的 Diff 算法。

#### Step 1: 定义 Payload

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryRefreshPayload {
    /// 可选：强制指定刷新源路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}
```

#### Step 2: 实现 Handler（核心 Diff 逻辑）

```rust
#[capability(id = "directory.refresh", target = "directory")]
fn handle_directory_refresh(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for directory.refresh")?;
    let payload: DirectoryRefreshPayload = serde_json::from_value(cmd.payload.clone())?;

    // 获取 external_root_path
    let external_root = payload
        .source_path
        .or_else(|| {
            block
                .metadata
                .custom
                .get("external_root_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .ok_or("No external_root_path found")?;

    // 重新扫描外部目录
    let source = Path::new(&external_root);
    let options = ScanOptions::default();
    let current_files = scan_directory(source, &options)?;

    // 解析旧 entries
    let old_entries = block.contents["entries"]
        .as_object()
        .ok_or("Invalid entries")?;

    let mut events = Vec::new();

    // Diff 算法：检测新增和修改
    for file_info in &current_files {
        let virtual_path = file_info.relative_path.clone();

        match old_entries.get(&virtual_path) {
            None => {
                // 新增文件：生成 core.create 事件
            }
            Some(entry) => {
                // 检查修改时间，生成 core.update 或 core.change_type 事件
            }
        }
    }

    // Diff 算法：检测删除
    let current_paths: std::collections::HashSet<_> =
        current_files.iter().map(|f| &f.relative_path).collect();

    for (old_path, old_entry) in old_entries {
        if !current_paths.contains(old_path) {
            // 生成 core.delete 事件
        }
    }

    Ok(events)
}
```

---

## 8. Phase 5: StateProjector 扩展

### 8.1 添加 Directory 事件处理

**文件**: `src-tauri/src/engine/state.rs`

在 `apply_event` 方法的 `match cap_id` 块中添加（约 line 217 之前）：

```rust
// Directory-specific events
"directory.write" => {
    if let Some(block) = self.blocks.get_mut(&event.entity) {
        // Merge contents
        if let Some(new_contents) = event.value.get("contents") {
            if let Some(obj) = block.contents.as_object_mut() {
                if let Some(new_obj) = new_contents.as_object() {
                    for (k, v) in new_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
        }
    }
}

"directory.export" => {
    // Export events are informational, no state change needed
}
```

### 8.3 权限模型说明 (MVP)

在 MVP 阶段，我们将采用 **"结构可见，内容受控" (Structure Visible, Content Controlled)** 的策略：

1.  **结构可见**：只要用户有权读取 Directory Block，就能看到完整的文件树索引（Directory Entries）。这意味着文件名和路径对所有有权访问该项目的人可见。
2.  **内容受控**：
    *   **读取**：当用户点击文件尝试读取内容时，后端会检查对该特定 Content Block 的 `core.read` 权限。无权则报错。
    *   **导出**：在执行 `export_directory` 时，程序会跳过那些当前用户无权读取的 Content Block，确保敏感内容不会被导出。

---

## 9. Phase 6: 集成测试与验证

### 9.1 运行完整测试套件

```bash
cd src-tauri

# 运行所有 directory 测试
cargo test directory -- --nocapture

# 使用 elfiee-ext-gen guide 检查进度
cd ..
elfiee-ext-gen guide directory
```

### 9.2 验证注册

```bash
elfiee-ext-gen validate directory
```

**预期输出**：

```
✅ Extension 'directory' validation passed!

Checks:
  ✅ Module exported in src/extensions/mod.rs
  ✅ Registered in CapabilityRegistry
  ✅ All capabilities have tests
```

### 9.3 生成 TypeScript 绑定

```bash
cd src-tauri
cargo build
```

**验证**: 检查 `src/bindings.ts` 是否包含所有 Directory Payloads。

### 9.4 手动集成测试

创建测试文件 `src-tauri/tests/integration_directory.rs`:

```rust
#[tokio::test]
async fn test_directory_import_export_workflow() {
    // 1. 创建临时测试目录
    // 2. 创建 directory block
    // 3. 执行 import
    // 4. 验证 entries
    // 5. 修改内部文件
    // 6. 执行 export
    // 7. 验证外部文件
}
```

---

## 10. 里程碑与检查点

### Milestone 1: Core Capabilities 完成 ✅
- [ ] `core.rename` 实现并测试通过
- [ ] `core.change_type` 实现并测试通过
- [ ] StateProjector 支持新事件
- [ ] 合并到 dev 分支

### Milestone 2: 工具模块完成 ✅
- [ ] `fs_scanner` 实现并测试通过
- [ ] `path_validator` 实现并测试通过
- [ ] `block_type_inference` 实现并测试通过
- [ ] 所有工具测试通过

### Milestone 3: 骨架生成完成 ✅
- [ ] 使用 elfiee-ext-gen 生成 directory extension
- [ ] 文件结构验证
- [ ] 编译检查通过

### Milestone 4: 基础 Capabilities 完成 ✅
- [ ] `directory.create` 实现并测试通过
- [ ] `directory.delete` 实现并测试通过
- [ ] `directory.rename` 实现并测试通过

### Milestone 5: 高级 Capabilities 完成 ✅
- [ ] `directory.import` 实现并测试通过
- [ ] `directory.export` 实现并测试通过
- [ ] `directory.refresh` 实现并测试通过

### Milestone 6: 集成与验证完成 ✅
- [ ] StateProjector 扩展完成
- [ ] 所有单元测试通过
- [ ] 集成测试通过
- [ ] elfiee-ext-gen validate 通过
- [ ] TypeScript 绑定生成成功

### 最终检查清单

```bash
# 1. 所有测试通过
cd src-tauri
cargo test

# 2. 验证扩展
cd ..
elfiee-ext-gen validate directory

# 3. 编译成功
cd src-tauri
cargo build

# 4. 前端类型验证
cd ../src
npx tsc --noEmit

# 5. 提交
git add .
git commit -m "feat(directory): implement Directory Extension with 6 capabilities"
git push origin feat/extension-directory-redesign
```

---

## 附录

### A. 常见问题

**Q1**: elfiee-ext-gen 生成的 Payload 字段不符合设计？
**A**: 手动编辑 `mod.rs` 中的 Payload 定义，生成器只提供模板。

**Q2**: StateProjector 不支持自定义事件？
**A**: 在 `state.rs` 的 `match cap_id` 中添加对应分支。

**Q3**: export 需要访问其他 Blocks？
**A**: 当前 capability macro 不支持，需要在 handler 内部通过全局状态访问。

### B. 参考命令速查

```bash
# 切换分支
git checkout -b <branch-name>

# 生成扩展
elfiee-ext-gen create -n <name> -b <type> -c <cap1>,<cap2>

# 运行测试
cargo test <module>::tests -- --nocapture

# 验证扩展
elfiee-ext-gen validate <extension>

# 查看指南
elfiee-ext-gen guide <extension>

# 编译
cargo build

# 格式化
cargo fmt

# Lint
cargo clippy
```

---

**文档结束**

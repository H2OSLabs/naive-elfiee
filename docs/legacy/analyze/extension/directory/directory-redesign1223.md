# Directory Extension 重新设计文档

> **版本**: v2.0 (2025-12-23 重新设计)
> **状态**: 设计阶段
> **作者**: Claude Code
> **目的**: 基于当前前端需求，重新设计Directory Extension的架构和实现

---

## 目录

- [1. 概述](#1-概述)
- [2. 核心定位与设计理念](#2-核心定位与设计理念)
- [3. 架构设计](#3-架构设计)
- [4. 数据结构定义](#4-数据结构定义)
- [5. Capabilities详细设计](#5-capabilities详细设计)
- [6. 工作流示例](#6-工作流示例)
- [7. 实施计划](#7-实施计划)
- [8. 测试策略](#8-测试策略)
- [9. 安全性考虑](#9-安全性考虑)
- [10. 未来扩展](#10-未来扩展)

---

## 1. 概述

### 1.1 背景

Directory Extension 对应前端编辑页面左侧功能区的 **Outline** 和 **Linked Repo** 两个区域，目的是形成一个可操作的虚拟工作区和文件树管理系统。

### 1.2 核心目标

1. **虚拟文件系统管理**：在 `.elf` 内部维护一个与外部隔离的文件系统
2. **多项目支持**：类似 VSCode Multi-root Workspace，支持同时管理多个项目
3. **内外隔离**：所有操作仅影响内部状态，通过 export 主动同步到外部
4. **Block化管理**：所有文件都转化为 Block，享受 Event Sourcing 的版本控制能力

### 1.3 设计原则

- ✅ **内容存DB优先**：文本类文件（Markdown、Code）内容存入 Event Store，而非文件系统
- ✅ **扁平索引，前端转树**：后端存储扁平路径映射，前端负责树状渲染
- ✅ **路径-名称同步**：Rename 操作同时更新 Directory 索引和 Block.name
- ✅ **级联删除**：删除操作清理索引和对应的 Blocks，避免孤儿数据
- ✅ **全量同步**：Refresh 采用 Mirror Sync 策略，完全同步外部状态

### 1.4 前置依赖

在开发 Directory Extension 之前，需要先在 Core 模块中实现以下基础 Capabilities：

1. **`core.rename`**: 用于修改 Block 的 `name` 字段
   - Payload: `{ "name": "new_name" }`
   - Event Attribute: `name`

2. **`core.change_type`**: 用于修改 Block 的 `block_type` 字段
   - Payload: `{ "block_type": "new_type" }`
   - Event Attribute: `block_type`

---

## 2. 核心定位与设计理念

### 2.1 两种文件来源

#### Outline（内部原生）
- 用户在 `.elf` 内部直接创建的文件和文件夹
- 这些是原生的 Block，不关联任何外部路径
- 用途：笔记、临时文件、知识管理

#### Linked Repo（外部关联）
- 从外部文件系统导入的项目
- 导入时 **复制** 到内部（非引用），与外部隔离
- 可通过 `refresh` 手动同步外部变更
- 可通过 `export` 将内部修改写回外部

### 2.2 与 VSCode 的类比

| VSCode 功能 | Elfiee 对应 | 说明 |
|------------|------------|------|
| Add Folder to Workspace | `core.create` + `directory.import` | 添加项目到工作区 |
| Explorer Tree | Directory Block 的 entries | 文件树索引 |
| 文件编辑 | Block 内容修改（通过对应 Extension） | Markdown/Code 编辑 |
| Refresh Explorer | `directory.refresh` | 重新扫描外部目录 |
| Copy Path | DirectoryEntry.external_path | 外部路径映射 |

### 2.3 内外隔离哲学

```
┌─────────────────────────────────────────────────────────────┐
│                    外部文件系统                              │
│  /Users/me/projects/my-app/                                 │
│    ├── src/main.rs                                          │
│    └── README.md                                            │
└─────────────────────────────────────────────────────────────┘
                         │
                    import (复制)
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Elfiee 内部虚拟文件系统                    │
│  Directory Block (repo-1)                                   │
│    metadata.custom.external_root_path: "/Users/.../my-app" │
│    entries:                                                 │
│      "src/main.rs" → Block-A (内容在 Event Store)           │
│      "README.md"   → Block-B (内容在 Event Store)           │
└─────────────────────────────────────────────────────────────┘
                         │
                    export (写回)
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    外部文件系统                              │
│  /output/my-app-v2/                                         │
│    ├── src/main.rs  (覆盖)                                  │
│    └── README.md    (覆盖)                                  │
└─────────────────────────────────────────────────────────────┘
```

**关键点**：
- Import 后，内外完全独立
- 用户在 Elfiee 内编辑，不影响外部
- 通过 `refresh + export` 实现手动双向同步
- 避免了复杂的实时同步和冲突检测

---

## 3. 架构设计

### 3.1 实体关系

```
┌──────────────────────────────────────────────────────────────┐
│                     Directory Block                          │
│  - block_id: "repo-1"                                        │
│  - block_type: "directory"                                   │
│  - name: "My Project"                                        │
│  - metadata.custom.external_root_path: "/external/project"  │
│  - contents:                                                 │
│      root_path: "/"                                          │
│      entries: HashMap<Path, DirectoryEntry>                  │
└──────────────────────────────────────────────────────────────┘
                         │
                         │ 索引关系
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ Content     │  │ Content     │  │ Content     │
│ Block-A     │  │ Block-B     │  │ Block-C     │
│             │  │             │  │             │
│ type:       │  │ type:       │  │ type:       │
│ markdown    │  │ code        │  │ code        │
│             │  │             │  │             │
│ contents:   │  │ contents:   │  │ contents:   │
│ {text: ...} │  │ {text: ...} │  │ {text: ...} │
└─────────────┘  └─────────────┘  └─────────────┘
```

### 3.2 两层路径映射

#### 项目级映射（Directory Block → 外部根目录）
存储位置：`Block.metadata.custom.external_root_path`

```json
{
  "block_id": "repo-1",
  "block_type": "directory",
  "metadata": {
    "custom": {
      "is_repo": true,
      "external_root_path": "/Users/me/projects/my-project"
    }
  }
}
```

**用途**：
- `directory.refresh` 读取此路径，重新扫描整个项目
- 前端显示项目的来源路径

#### 文件级映射（虚拟路径 → 外部文件路径）
存储位置：`DirectoryBlockContent.entries[path].external_path`

```json
{
  "entries": {
    "src/main.rs": {
      "id": "block-uuid-1",
      "type": "file",
      "source": "linked",
      "external_path": "/Users/me/projects/my-project/src/main.rs",
      "updated_at": "2025-12-22T10:00:00Z"
    }
  }
}
```

**用途**：
- `directory.export` 根据此路径写回文件
- `directory.refresh` 对比外部文件的修改时间

---

## 4. 数据结构定义

### 4.1 Directory Block Content（存储在 Event Store）

```rust
// src-tauri/src/extensions/directory/models.rs

use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

/// Directory Block 的 contents 字段结构
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryBlockContent {
    /// 虚拟根路径，通常是 "/"
    pub root_path: String,

    /// 文件索引表：虚拟路径 -> 文件条目
    /// 采用扁平存储，前端负责转换为树状结构
    pub entries: HashMap<String, DirectoryEntry>,
}

/// 单个文件/文件夹条目
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryEntry {
    /// 对应的 Block ID
    /// - 对于文件：指向实际的 Content Block (markdown/code)
    /// - 对于文件夹：可以是 null 或虚拟 ID
    pub id: String,

    /// 条目类型: "file" | "directory"
    #[serde(rename = "type")]
    pub entry_type: String,

    /// 来源标识: "linked" (外部导入) | "outline" (内部创建)
    pub source: String,

    /// 外部真实路径（仅 linked 且是文件时存在）
    /// 用于 refresh 和 export 操作
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_path: Option<String>,

    /// 最后更新时间（RFC3339 格式）
    pub updated_at: String,
}
```

### 4.2 Block Type 推断规则

导入外部文件时，根据扩展名推断 Block 类型：

| 文件扩展名 | Block Type | 说明 |
|-----------|-----------|------|
| `.md`, `.markdown` | `markdown` | Markdown 文件 |
| `.rs`, `.py`, `.js`, `.ts`, `.c`, `.cpp`, `.java`, `.go`, `.json`, `.toml`, `.yaml`, etc. | `code` | 代码文件（MVP阶段） |
| `.png`, `.jpg`, `.gif`, `.svg` | ❌ 暂不支持 | 图片（未来扩展） |
| 其他 | ❌ 暂不支持 | 二进制文件（未来扩展） |

**MVP 策略**：
- 仅支持文本文件（Markdown 和 Code）
- 代码文件暂时只读或纯文本编辑（待 Code Extension 开发）
- **默认过滤**：自动忽略 `.git`, `node_modules`, `target`, `dist`, `.DS_Store` 等常见非源码/大文件目录
- 遇到不支持的文件类型，跳过或记录警告

### 4.3 Block Metadata 扩展

对于 Linked Repo 类型的 Directory Block，扩展 metadata：

```rust
// 示例：存储在 Block.metadata.custom
{
  "is_repo": true,
  "external_root_path": "/Users/me/projects/my-project",
  "last_refresh": "2025-12-22T10:00:00Z"
}
```

---

## 5. Capabilities详细设计

### 5.1 `directory.import` - 导入外部项目

#### 功能描述
从外部文件系统导入文件/目录到指定的 Directory Block 中。

#### Payload 定义

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryImportPayload {
    /// 目标 Directory Block 的 ID（必须已存在）
    pub block_id: String,

    /// 外部真实文件系统路径（来源）
    /// 例如: "/Users/me/projects/my-app"
    pub source_path: String,

    /// 内部虚拟路径前缀（目标）
    /// None 或 "/" 表示导入到根目录
    /// 例如: "libs/external" 表示导入到该路径下
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
}
```

#### Handler 逻辑

```rust
// 伪代码
async fn handle_import(
    cmd: Command,
    state: &State,
) -> Result<Vec<Event>, String> {
    let payload: DirectoryImportPayload = parse_payload(cmd.payload)?;

    // 1. 验证 Directory Block 存在
    let dir_block = state.get_block(&payload.block_id)?;
    ensure!(dir_block.block_type == "directory");

    // 2. 验证外部路径安全性
    let source = canonicalize(&payload.source_path)?;
    ensure!(source.exists() && is_safe_path(&source));

    // 3. 递归扫描外部目录
    let files = scan_directory(&source, &ScanOptions {
        max_depth: 100,
        follow_symlinks: false,
        ignore_hidden: true,
        // MVP 默认忽略名单
        ignore_patterns: vec!["node_modules", ".git", "target", "dist", "build", ".DS_Store"],
    })?;

    let mut events = Vec::new();
    let target_prefix = payload.target_path.unwrap_or("/".to_string());

    for file_info in files {
        // 4. 根据扩展名推断 Block 类型
        let block_type = infer_block_type(&file_info.extension)?;
        if block_type.is_none() {
            warn!("Skipping unsupported file: {}", file_info.path);
            continue;
        }

        // 5. 读取文件内容
        let content = read_file_content(&file_info.path)?;

        // 6. 生成 core.create Event（创建 Content Block）
        let block_id = generate_uuid();
        let virtual_path = join_path(&target_prefix, &file_info.relative_path);

        events.push(Event {
            entity: block_id.clone(),
            attribute: format!("{}/{}", cmd.editor_id, "core.create"),
            value: json!({
                "block_type": block_type.unwrap(),
                "name": file_info.file_name,
                "contents": {
                    "text": content,
                    "language": file_info.extension, // for code blocks
                },
            }),
            timestamp: generate_timestamp(),
        });

        // 7. 生成 directory.add_entry Event（更新索引）
        events.push(Event {
            entity: payload.block_id.clone(),
            attribute: format!("{}/{}", cmd.editor_id, "directory.add_entry"),
            value: json!({
                "path": virtual_path,
                "entry": {
                    "id": block_id,
                    "type": "file",
                    "source": "linked",
                    "external_path": file_info.path,
                    "updated_at": now_utc(),
                }
            }),
            timestamp: generate_timestamp(),
        });
    }

    // 8. 更新 Directory Block 的 metadata（记录外部根路径）
    events.push(Event {
        entity: payload.block_id.clone(),
        attribute: format!("{}/{}", cmd.editor_id, "core.update_metadata"),
        value: json!({
            "custom": {
                "is_repo": true,
                "external_root_path": payload.source_path,
                "last_import": now_utc(),
            }
        }),
        timestamp: generate_timestamp(),
    });

    Ok(events)
}
```

#### 边界情况处理

1. **路径安全**：
   - 使用 `canonicalize()` 解析符号链接
   - 拒绝访问系统敏感目录（/etc, /sys, etc.）
   - 使用 `symlink_metadata()` 检测符号链接，拒绝跨越

2. **大文件处理**：
   - 限制单文件大小（如 10MB）
   - 超过限制则跳过并记录警告

3. **嵌套深度**：
   - 限制目录递归深度（防止栈溢出）

4. **文件数量**：
   - 限制单次导入文件数量（如 10,000 个）

---

### 5.2 `directory.export` - 导出到外部

#### 功能描述
将 Directory Block 中的文件/目录导出到外部文件系统。

#### Payload 定义

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryExportPayload {
    /// 源 Directory Block 的 ID
    pub block_id: String,

    /// 目标外部路径（写入位置）
    /// 例如: "/Users/me/output/exported-project"
    pub target_path: String,

    /// 内部虚拟路径（可选，仅导出子目录）
    /// None 表示导出整个项目
    /// 例如: "src" 表示仅导出 src 目录
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}
```

#### Handler 逻辑

```rust
async fn handle_export(
    cmd: Command,
    state: &State,
) -> Result<Vec<Event>, String> {
    let payload: DirectoryExportPayload = parse_payload(cmd.payload)?;

    // 1. 获取 Directory Block
    let dir_block = state.get_block(&payload.block_id)?;
    let contents: DirectoryBlockContent =
        serde_json::from_value(dir_block.contents)?;

    // 2. 验证目标路径安全性
    let target = canonicalize_or_create(&payload.target_path)?;
    ensure!(is_safe_path(&target));

    // 3. 过滤需要导出的条目
    let source_prefix = payload.source_path.unwrap_or("/".to_string());
    let entries_to_export: Vec<_> = contents.entries
        .iter()
        .filter(|(path, _)| path.starts_with(&source_prefix))
        .collect();

    // 4. 逐个导出文件
    for (virtual_path, entry) in entries_to_export {
        if entry.entry_type == "directory" {
            // 创建目录
            let dir_path = target.join(virtual_path);
            fs::create_dir_all(&dir_path)?;
        } else {
            // 导出文件
            let content_block = state.get_block(&entry.id)?;
            let text = extract_text_content(&content_block.contents)?;

            let file_path = target.join(virtual_path);
            fs::create_dir_all(file_path.parent().unwrap())?;
            fs::write(&file_path, text)?;
        }
    }

    // 5. 生成 Event（记录导出操作）
    Ok(vec![Event {
        entity: payload.block_id,
        attribute: format!("{}/{}", cmd.editor_id, "directory.export"),
        value: json!({
            "target_path": payload.target_path,
            "file_count": entries_to_export.len(),
            "exported_at": now_utc(),
        }),
        timestamp: generate_timestamp(),
    }])
}
```

#### 边界情况处理

1. **文件覆盖**：直接覆盖同名文件（用户已确认）
2. **权限问题**：捕获写入错误，回滚部分写入的文件
3. **路径长度**：Windows 260 字符限制检测

---

### 5.3 `directory.refresh` - 重新同步外部

#### 功能描述
重新扫描外部目录，采用 Mirror Sync 策略同步内外状态。

#### Payload 定义

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryRefreshPayload {
    /// 目标 Directory Block 的 ID
    pub block_id: String,

    /// 可选：强制指定刷新源路径
    /// 通常不需要，会自动读取 Block.metadata.custom.external_root_path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}
```

#### Handler 逻辑

```rust
async fn handle_refresh(
    cmd: Command,
    state: &State,
) -> Result<Vec<Event>, String> {
    let payload: DirectoryRefreshPayload = parse_payload(cmd.payload)?;

    // 1. 获取 Directory Block 和外部根路径
    let dir_block = state.get_block(&payload.block_id)?;
    let external_root = payload.source_path
        .or_else(|| dir_block.metadata.custom.get("external_root_path"))
        .ok_or("No external_root_path found")?;

    let old_contents: DirectoryBlockContent =
        serde_json::from_value(dir_block.contents)?;

    // 2. 重新扫描外部目录
    let current_files = scan_directory(&external_root, &ScanOptions::default())?;

    // 3. Diff 算法：计算增删改
    let mut events = Vec::new();
    let mut new_entries = old_contents.entries.clone();

    // 3.1 检测新增和修改
    for file_info in &current_files {
        let virtual_path = file_info.relative_path.clone();

        match old_entries.get(&virtual_path) {
            None => {
                // 新增文件：创建新 Block
                let block_id = generate_uuid();
                let content = read_file_content(&file_info.path)?;

                events.push(create_block_event(block_id, content, file_info));
                events.push(add_entry_event(&payload.block_id, virtual_path, block_id));
            }
            Some(entry) => {
                // 已存在：检查是否修改
                if is_file_modified(&file_info, &entry.updated_at)? {
                    let content = read_file_content(&file_info.path)?;
                    events.push(update_block_event(&entry.id, content));
                    events.push(update_entry_timestamp_event(&payload.block_id, virtual_path));

                    // 检查类型是否变更 (例如 main.rs -> main.md)
                    let new_type = infer_block_type(&file_info.extension)?;
                    let old_block = state.get_block(&entry.id)?;
                    if let Some(nt) = new_type {
                        if nt != old_block.block_type {
                            events.push(Event {
                                entity: entry.id.clone(),
                                attribute: format!("{}/{}", cmd.editor_id, "core.change_type"),
                                value: json!({ "block_type": nt }),
                                timestamp: generate_timestamp(),
                            });
                        }
                    }
                }
            }
        }
    }

    // 3.2 检测删除
    let current_paths: HashSet<_> = current_files.iter()
        .map(|f| f.relative_path.clone())
        .collect();

    for (old_path, old_entry) in &old_contents.entries {
        if !current_paths.contains(old_path) {
            // 外部已删除：删除内部 Block
            events.push(delete_block_event(&old_entry.id));
            events.push(remove_entry_event(&payload.block_id, old_path));
        }
    }

    // 4. 更新 last_refresh 时间戳
    events.push(update_metadata_event(&payload.block_id, "last_refresh", now_utc()));

    Ok(events)
}
```

#### Diff 策略

| 场景 | 外部状态 | 内部状态 | 操作 |
|-----|---------|---------|------|
| 新增文件 | ✅ 存在 | ❌ 不存在 | `core.create` + `directory.add_entry` |
| 删除文件 | ❌ 不存在 | ✅ 存在 | `core.delete` + `directory.remove_entry` |
| 修改文件 | ✅ 修改时间新 | ✅ 存在 | `core.update` + 更新 entry.updated_at |
| 未变化 | ✅ 修改时间相同 | ✅ 存在 | 无操作 |

---

### 5.4 `directory.create` - 内部创建文件/文件夹

#### 功能描述
在 Directory Block 内部创建新的文件或文件夹。

#### Payload 定义

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryCreatePayload {
    /// 目标 Directory Block ID
    pub block_id: String,

    /// 内部虚拟路径（例如 "docs/README.md"）
    pub path: String,

    /// 类型: "file" | "directory"
    #[serde(rename = "type")]
    pub entry_type: String,

    /// 初始内容（仅文件需要，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Block 类型（仅文件需要）
    /// 例如: "markdown", "code"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_type: Option<String>,
}
```

#### Handler 逻辑

```rust
async fn handle_create(
    cmd: Command,
    state: &State,
) -> Result<Vec<Event>, String> {
    let payload: DirectoryCreatePayload = parse_payload(cmd.payload)?;

    // 1. 验证路径格式
    ensure!(!payload.path.is_empty());
    ensure!(!payload.path.starts_with('/'));

    // 2. 检查是否已存在
    let dir_block = state.get_block(&payload.block_id)?;
    let contents: DirectoryBlockContent =
        serde_json::from_value(dir_block.contents)?;

    ensure!(!contents.entries.contains_key(&payload.path), "Path already exists");

    let mut events = Vec::new();

    if payload.entry_type == "file" {
        // 3. 创建文件 Block
        let block_id = generate_uuid();
        let file_name = extract_filename(&payload.path);
        let block_type = payload.block_type.unwrap_or("markdown".to_string());

        events.push(Event {
            entity: block_id.clone(),
            attribute: format!("{}/{}", cmd.editor_id, "core.create"),
            value: json!({
                "block_type": block_type,
                "name": file_name,
                "contents": {
                    "text": payload.content.unwrap_or_default(),
                },
            }),
            timestamp: generate_timestamp(),
        });

        // 4. 添加到 Directory 索引
        events.push(Event {
            entity: payload.block_id,
            attribute: format!("{}/{}", cmd.editor_id, "directory.add_entry"),
            value: json!({
                "path": payload.path,
                "entry": {
                    "id": block_id,
                    "type": "file",
                    "source": "outline",
                    "updated_at": now_utc(),
                }
            }),
            timestamp: generate_timestamp(),
        });
    } else {
        // 5. 创建文件夹（仅索引，无 Block）
        events.push(Event {
            entity: payload.block_id,
            attribute: format!("{}/{}", cmd.editor_id, "directory.add_entry"),
            value: json!({
                "path": payload.path,
                "entry": {
                    "id": format!("dir-{}", generate_short_id()),
                    "type": "directory",
                    "source": "outline",
                    "updated_at": now_utc(),
                }
            }),
            timestamp: generate_timestamp(),
        });
    }

    Ok(events)
}
```

---

### 5.5 `directory.delete` - 删除文件/文件夹

#### 功能描述
从 Directory Block 删除文件或文件夹，级联删除对应的 Blocks。

#### Payload 定义

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryDeletePayload {
    /// Directory Block ID
    pub block_id: String,

    /// 要删除的虚拟路径
    pub path: String,
}
```

#### Handler 逻辑

```rust
async fn handle_delete(
    cmd: Command,
    state: &State,
) -> Result<Vec<Event>, String> {
    let payload: DirectoryDeletePayload = parse_payload(cmd.payload)?;

    // 1. 获取 Directory Block
    let dir_block = state.get_block(&payload.block_id)?;
    let contents: DirectoryBlockContent =
        serde_json::from_value(dir_block.contents)?;

    let entry = contents.entries.get(&payload.path)
        .ok_or("Path not found")?;

    let mut events = Vec::new();

    if entry.entry_type == "directory" {
        // 2. 递归删除子项
        let children: Vec<_> = contents.entries
            .iter()
            .filter(|(path, _)| path.starts_with(&payload.path))
            .collect();

        for (child_path, child_entry) in children {
            if child_entry.entry_type == "file" {
                // 删除子 Block
                events.push(Event {
                    entity: child_entry.id.clone(),
                    attribute: format!("{}/{}", cmd.editor_id, "core.delete"),
                    value: json!({}),
                    timestamp: generate_timestamp(),
                });
            }

            // 删除索引条目
            events.push(Event {
                entity: payload.block_id.clone(),
                attribute: format!("{}/{}", cmd.editor_id, "directory.remove_entry"),
                value: json!({ "path": child_path }),
                timestamp: generate_timestamp(),
            });
        }
    } else {
        // 3. 删除单个文件
        events.push(Event {
            entity: entry.id.clone(),
            attribute: format!("{}/{}", cmd.editor_id, "core.delete"),
            value: json!({}),
            timestamp: generate_timestamp(),
        });

        events.push(Event {
            entity: payload.block_id,
            attribute: format!("{}/{}", cmd.editor_id, "directory.remove_entry"),
            value: json!({ "path": payload.path }),
            timestamp: generate_timestamp(),
        });
    }

    Ok(events)
}
```

---

### 5.6 `directory.rename` - 重命名/移动

#### 功能描述
重命名或移动文件/文件夹，同步更新 Block.name。

#### Payload 定义

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryRenamePayload {
    /// Directory Block ID
    pub block_id: String,

    /// 旧路径
    pub old_path: String,

    /// 新路径
    pub new_path: String,
}
```

#### Handler 逻辑

```rust
async fn handle_rename(
    cmd: Command,
    state: &State,
) -> Result<Vec<Event>, String> {
    let payload: DirectoryRenamePayload = parse_payload(cmd.payload)?;

    // 1. 验证路径
    let dir_block = state.get_block(&payload.block_id)?;
    let contents: DirectoryBlockContent =
        serde_json::from_value(dir_block.contents)?;

    ensure!(contents.entries.contains_key(&payload.old_path));
    ensure!(!contents.entries.contains_key(&payload.new_path));

    let entry = contents.entries.get(&payload.old_path).unwrap();
    let mut events = Vec::new();

    if entry.entry_type == "file" {
        // 2. 更新 Block 的 name 字段
        let new_filename = extract_filename(&payload.new_path);

        events.push(Event {
            entity: entry.id.clone(),
            attribute: format!("{}/{}", cmd.editor_id, "core.rename"),
            value: json!({
                "name": new_filename,
            }),
            timestamp: generate_timestamp(),
        });
    } else {
        // 3. 文件夹：批量更新子路径
        let children: Vec<_> = contents.entries
            .iter()
            .filter(|(path, _)| path.starts_with(&payload.old_path))
            .collect();

        for (child_path, child_entry) in children {
            let new_child_path = child_path.replace(&payload.old_path, &payload.new_path);

            events.push(Event {
                entity: payload.block_id.clone(),
                attribute: format!("{}/{}", cmd.editor_id, "directory.rename_entry"),
                value: json!({
                    "old_path": child_path,
                    "new_path": new_child_path,
                }),
                timestamp: generate_timestamp(),
            });

            // 如果是文件，更新 Block.name
            if child_entry.entry_type == "file" {
                events.push(Event {
                    entity: child_entry.id.clone(),
                    attribute: format!("{}/{}", cmd.editor_id, "core.rename"),
                    value: json!({
                        "name": extract_filename(&new_child_path),
                    }),
                    timestamp: generate_timestamp(),
                });
            }
        }
    }

    // 4. 更新索引中的路径 Key
    events.push(Event {
        entity: payload.block_id,
        attribute: format!("{}/{}", cmd.editor_id, "directory.rename_entry"),
        value: json!({
            "old_path": payload.old_path,
            "new_path": payload.new_path,
        }),
        timestamp: generate_timestamp(),
    });

    Ok(events)
}
```

---

## 6. 工作流示例

### 6.1 用户添加外部项目

**前端操作**：
1. 用户点击 "Add Project"
2. 选择外部目录：`/Users/me/my-rust-app`
3. 输入项目名称："My Rust App"

**后端处理**：
```rust
// Step 1: 创建 Directory Block
Command {
    cmd_id: "cmd-1",
    editor_id: "user-alice",
    block_id: None,
    capability: "core.create",
    payload: {
        "block_type": "directory",
        "name": "My Rust App",
        "contents": {
            "root_path": "/",
            "entries": {}
        },
        "metadata": {
            "custom": {
                "is_repo": true,
                "external_root_path": "/Users/me/my-rust-app"
            }
        }
    }
}

// Step 2: 导入文件
Command {
    cmd_id: "cmd-2",
    editor_id: "user-alice",
    block_id: "repo-1",  // 上一步创建的 Block ID
    capability: "directory.import",
    payload: {
        "block_id": "repo-1",
        "source_path": "/Users/me/my-rust-app",
        "target_path": null
    }
}
```

**生成的 Events**：
- `core.create` x1（Directory Block）
- `core.create` x50（假设有 50 个文件）
- `directory.add_entry` x50
- `core.update_metadata` x1

**前端显示**：
```
📁 My Rust App
  ├── 📄 Cargo.toml
  ├── 📄 README.md
  └── 📁 src
      ├── 📄 main.rs
      └── 📄 lib.rs
```

### 6.2 用户刷新项目

**场景**：用户在外部修改了 `main.rs`，在 Elfiee 中点击刷新。

**后端处理**：
```rust
Command {
    cmd_id: "cmd-3",
    editor_id: "user-alice",
    block_id: "repo-1",
    capability: "directory.refresh",
    payload: {
        "block_id": "repo-1"
    }
}
```

**Diff 结果**：
- `src/main.rs` 修改 → 生成 `core.update` Event
- `src/utils.rs` 新增 → 生成 `core.create` + `directory.add_entry`
- `README.md` 删除 → 生成 `core.delete` + `directory.remove_entry`

### 6.3 用户导出修改

**场景**：用户在 Elfiee 内修改了多个文件，想导出到新目录。

**后端处理**：
```rust
Command {
    cmd_id: "cmd-4",
    editor_id: "user-alice",
    block_id: "repo-1",
    capability: "directory.export",
    payload: {
        "block_id": "repo-1",
        "target_path": "/Users/me/output/my-app-v2"
    }
}
```

**结果**：
- 在 `/Users/me/output/my-app-v2` 创建完整的文件树
- 所有文件内容来自 Block 的当前状态

---

## 7. 实施计划

### 7.1 开发阶段

#### Phase 1: 数据结构和基础设施（1-2天）
- [ ] 定义 Rust 数据结构（`models.rs`）
- [ ] 实现 Block Type 推断工具函数
- [ ] 实现路径安全验证工具
- [ ] 实现目录扫描工具（`scan_directory`）
- [ ] 添加 Specta 类型导出

#### Phase 2: 核心 Capabilities（3-5天）
- [ ] `directory.import` Handler + Tests
- [ ] `directory.create` Handler + Tests
- [ ] `directory.delete` Handler + Tests
- [ ] `directory.rename` Handler + Tests

#### Phase 3: 高级 Capabilities（2-3天）
- [ ] `directory.refresh` Handler + Diff 算法 + Tests
- [ ] `directory.export` Handler + Tests

#### Phase 4: 集成和优化（1-2天）
- [ ] 注册到 CapabilityRegistry
- [ ] 前端 TypeScript 绑定验证
- [ ] 性能测试（大项目导入）
- [ ] 文档编写

### 7.2 使用 elfiee-ext-gen

```bash
cd /home/yaosh/projects/elfiee

# 生成骨架（如果需要）
elfiee-ext-gen create \
  -n directory \
  -b directory \
  -c import,export,refresh,create,delete,rename

# 或手动创建文件结构
```

**建议**：由于 Directory Extension 逻辑复杂，**建议手动创建**，不使用生成器。

### 7.3 文件结构

```
src-tauri/src/extensions/directory/
├── mod.rs                  # 模块导出和 Capability 定义
├── models.rs               # 数据结构定义
├── handlers/
│   ├── mod.rs
│   ├── import.rs           # directory.import Handler
│   ├── export.rs           # directory.export Handler
│   ├── refresh.rs          # directory.refresh Handler
│   ├── create.rs           # directory.create Handler
│   ├── delete.rs           # directory.delete Handler
│   └── rename.rs           # directory.rename Handler
├── utils/
│   ├── mod.rs
│   ├── scanner.rs          # 目录扫描工具
│   ├── path_validator.rs  # 路径安全验证
│   └── block_type.rs       # Block 类型推断
└── tests/
    ├── mod.rs
    ├── import_tests.rs
    ├── refresh_tests.rs
    └── ...
```

---

## 8. 测试策略

### 8.1 单元测试

每个 Capability Handler 至少包含：

1. **正常流程测试**
   - 导入单个文件
   - 导入包含子目录的项目
   - 创建文件/文件夹
   - 重命名文件/文件夹
   - 删除文件/文件夹

2. **边界条件测试**
   - 空目录导入
   - 超大文件跳过
   - 深层嵌套目录
   - 特殊字符文件名
   - 路径长度限制

3. **错误处理测试**
   - 不存在的路径
   - 权限不足
   - 路径已存在
   - 不安全的路径（符号链接）

4. **授权测试**
   - 非所有者无权限
   - 授予权限后可操作
   - 撤销权限后禁止

### 8.2 集成测试

```rust
#[tokio::test]
async fn test_import_export_roundtrip() {
    // 1. 创建测试目录和文件
    let temp_dir = create_temp_project();

    // 2. 创建 Directory Block
    let block_id = create_directory_block().await;

    // 3. 导入
    execute_command(DirectoryImportPayload {
        block_id: block_id.clone(),
        source_path: temp_dir.path(),
        target_path: None,
    }).await.unwrap();

    // 4. 修改内部文件
    let file_block_id = get_entry_block_id(&block_id, "src/main.rs");
    update_block_content(&file_block_id, "new content").await;

    // 5. 导出
    let export_dir = TempDir::new();
    execute_command(DirectoryExportPayload {
        block_id,
        target_path: export_dir.path(),
        source_path: None,
    }).await.unwrap();

    // 6. 验证
    assert_eq!(
        fs::read_to_string(export_dir.path().join("src/main.rs")),
        "new content"
    );
}
```

### 8.3 性能测试

```rust
#[tokio::test]
async fn test_import_large_project() {
    // 导入包含 10,000 个文件的项目
    let start = Instant::now();

    execute_import(large_project_path).await.unwrap();

    let duration = start.elapsed();
    assert!(duration < Duration::from_secs(30)); // 30秒内完成
}
```

---

## 9. 安全性考虑

### 9.1 路径遍历攻击防护

```rust
fn is_safe_path(path: &Path) -> Result<bool, String> {
    let canonical = path.canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;

    // 1. 拒绝系统敏感目录
    let forbidden = ["/etc", "/sys", "/proc", "/dev"];
    if forbidden.iter().any(|p| canonical.starts_with(p)) {
        return Err("Access to system directories is forbidden".to_string());
    }

    // 2. 检测符号链接
    let metadata = fs::symlink_metadata(&canonical)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;

    if metadata.is_symlink() {
        return Err("Symbolic links are not allowed".to_string());
    }

    Ok(true)
}
```

### 9.2 TOCTOU 竞态条件

```rust
// ❌ 错误：检查和操作分离
if path.exists() {  // Time of Check
    fs::remove_file(path)?;  // Time of Use
}

// ✅ 正确：原子操作
fs::remove_file(path).ok();  // 失败也不影响
```

### 9.3 资源限制

```rust
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
const MAX_FILES_PER_IMPORT: usize = 10_000;
const MAX_DEPTH: usize = 100;

### 9.3 资源限制与过滤

1. **文件大小限制**：超过 10MB 的文件自动跳过，防止 Event Store 过大。
2. **数量限制**：单次导入超过 10,000 个文件报错，防止内存溢出。
3. **目录过滤**：
   - 默认忽略所有以 `.` 开头的隐藏文件/文件夹（除 `.elf` 内部逻辑需要外）。
   - 默认忽略常见构建目录：`node_modules`, `target`, `dist`, `bin`, `obj`。
   - 这不仅是为了安全，更是为了保证 `.elf` 文件的轻量化。

fn scan_directory(path: &Path, options: &ScanOptions) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    let mut count = 0;

    for entry in WalkDir::new(path).max_depth(options.max_depth) {
        count += 1;
        if count > MAX_FILES_PER_IMPORT {
            return Err("Too many files".into());
        }

        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.len() > MAX_FILE_SIZE {
            warn!("Skipping large file: {:?}", entry.path());
            continue;
        }

        files.push(FileInfo::from_entry(entry));
    }

    Ok(files)
}
```

---

## 10. 未来扩展

### 10.1 图片和二进制文件支持

```rust
// 扩展 DirectoryEntry
pub struct DirectoryEntry {
    // ... 现有字段

    /// 二进制文件的存储路径（相对于 block-{uuid} 目录）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
}

// Import 时的处理
if is_binary_file(&file_info) {
    // 存储到 Block 目录
    let binary_path = format!("assets/{}", file_info.file_name);
    copy_to_block_dir(&block_id, &binary_path, &file_info.path)?;

    // 在 Event 中只记录路径
    entry.binary_path = Some(binary_path);
}
```

### 10.2 增量同步优化

当前 `refresh` 采用全量扫描，未来可优化为基于文件监听的增量同步：

```rust
// 使用 notify crate
let (tx, rx) = channel();
let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

watcher.watch(&external_root, RecursiveMode::Recursive)?;

for event in rx {
    match event {
        Event::Create(path) => handle_external_create(path),
        Event::Modify(path) => handle_external_modify(path),
        Event::Remove(path) => handle_external_remove(path),
    }
}
```

### 10.3 .gitignore 支持

```rust
use ignore::WalkBuilder;

let walker = WalkBuilder::new(&source_path)
    .git_ignore(true)
    .build();

for result in walker {
    let entry = result?;
    // 自动跳过 .gitignore 中的文件
}
```

### 10.4 智能冲突检测

在 `refresh` 时检测三方冲突（内部修改 + 外部修改）：

```rust
if internal_hash != external_hash && internal_modified > last_refresh {
    // 三方冲突：内外都修改了
    return Err(ConflictError {
        path,
        internal_version,
        external_version,
        suggested_action: "Manual merge required",
    });
}
```

---

## 11. 参考文档

- [Elfiee Architecture Overview](../../concepts/ARCHITECTURE_OVERVIEW.md)
- [Extension Development Guide](../../guides/EXTENSION_DEVELOPMENT.md)
- [Frontend Development Guide](../../guides/FRONTEND_DEVELOPMENT.md)
- [旧版 Directory Extension 设计](./directory.md)
- [旧版 Directory Extension 修复清单](./directory-be-fix.md)

---

## 12. 变更日志

| 日期 | 版本 | 变更内容 |
|-----|------|---------|
| 2025-12-23 | v2.0 | 完全重新设计，基于虚拟文件系统理念 |
| 2024-xx-xx | v1.0 | 初版设计（已废弃） |

---

**文档结束**

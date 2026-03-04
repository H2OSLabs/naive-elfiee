# Directory Extension 执行方案

**版本**: v4.0-final
**日期**: 2025-01-19
**状态**: 最终执行方案

---

## 1. 核心定位

Directory Extension 是 Elfiee 的文件系统管理器，对标 VSCode 的 Explorer。

**核心功能**：
- 挂载项目目录到 Directory Block
- 批量扫描文件，导入为 Blocks
- 提供文件树视图（基于 indexed_files）
- 创建/删除/重命名文件（同步操作文件系统和 Blocks）
- 检测外部编辑并同步到 Elfiee
- 批量导出 Blocks 到文件系统

**架构特性**：
- 一个 .elf 文件可包含多个 Directory Blocks
- 每个 Directory Block 管理一个项目目录
- root 路径持久化在 Block.contents 中
- indexed_files 维护文件路径 → Block ID 映射

---

## 2. 工作流程

### 2.1 初始化 Directory Block

```
前端: 新建 .elf 文件
  ↓
前端: 调用 core.create
  payload: { name: "项目A", block_type: "directory" }
  ↓
后端: 创建 directory Block
  contents: {}  // 空
  ↓
前端: 调用 directory.root
  payload: {
    root: "/home/user/project-a",
    recursive: true,
    include_hidden: false,
    max_depth: null
  }
  ↓
后端: 挂载目录，持久化配置
  contents: {
    "root": "/home/user/project-a",
    "recursive": true,
    "include_hidden": false,
    "max_depth": null,
    "entries": [],
    "last_updated": "2025-01-19T10:00:00Z",
    "watch_enabled": false
  }
  ↓
前端: 调用 directory.scan
  payload: {}  // 从 contents.root 读取
  ↓
后端: 批量导入所有文件为 Blocks
  - 遍历 contents.root 目录
  - 为每个文件创建 markdown Block
  - 记录 indexed_files 映射
  ↓
前端: 从 indexed_files 构建文件树
```

### 2.2 编辑和保存

```
用户: 在 Elfiee 中编辑 README.md
  ↓
前端: 调用 markdown.write
  ↓
后端: 生成 Event（更新 Block.contents.markdown）
  ↓
前端: 添加 block_id 到本地 dirtyBlocks Set
  ↓
用户: 点击 "Save All"
  ↓
前端: 调用 directory.exportall
  payload: {
    exports: [
      { block_id: "block-markdown-001", content: "..." }
    ]
  }
  ↓
后端:
  - 从 contents.root 读取项目根目录
  - 从 indexed_files 查找文件路径
  - 写入外部文件
  - 更新 last_modified
  ↓
前端: 清空 dirtyBlocks
```

### 2.3 外部编辑同步

```
用户: 在 VSCode 中修改了 README.md
  ↓
用户: 在 Elfiee 中点击 "Refresh"
  ↓
前端: 调用 directory.refresh
  payload: {}
  ↓
后端:
  - 从 contents.root 读取项目根目录
  - 遍历 indexed_files
  - 比对文件 mtime
  - 如果 mtime 不匹配:
    - 读取新内容
    - 生成 markdown.write Event
    - 更新 last_modified
  - 生成 directory.refresh Event
  ↓
返回:
  Event 1: directory.refresh（检测结果 + 更新 indexed_files）
  Event 2-N: markdown.write（更新 Block 内容）
```

**权限分离**：
- alice 有 directory.refresh 权限 → Event 1 成功
- alice 没有 markdown.write 权限 → Event 2 失败
- 结果：检测到变更，但无法同步

### 2.4 多目录场景

```
workspace.elf 包含:
  ├── directory-block-1
  │     contents.root: "/home/user/project-a"
  │     indexed_files: { "README.md": "block-md-1", ... }
  │
  ├── directory-block-2
  │     contents.root: "/home/user/project-b"
  │     indexed_files: { "main.py": "block-md-2", ... }
  │
  ├── block-md-1 (project-a 的 README.md)
  └── block-md-2 (project-b 的 main.py)
```

---

## 3. 数据结构

### 3.1 Directory Block.contents

```rust
{
  // 持久化字段
  "root": "/home/user/my-project",           // 项目根目录（绝对路径）
  "recursive": true,                          // 默认递归配置
  "include_hidden": false,                    // 默认隐藏文件配置
  "max_depth": null,                          // 默认深度限制
  "indexed_files": {
    "README.md": {
      "block_id": "block-markdown-001",
      "last_modified": "2025-01-19T10:00:00Z"
    },
    "src/main.rs": {
      "block_id": "block-markdown-002",
      "last_modified": "2025-01-19T11:00:00Z"
    }
  },
  "entries": [],                              // list 结果缓存
  "last_updated": "2025-01-19T12:00:00Z",    // 最后更新时间
  "watch_enabled": false,                     // 文件监听状态

  // Engine 运行时注入（不持久化）
  "_block_dir": "/tmp/elf-xxx/block-dir-001/"
}
```

**字段说明**：
- `root`: 必需，由 directory.root 初始化
- `recursive`, `include_hidden`, `max_depth`: directory.root 设置的默认值
- `indexed_files`: 文件路径（相对于root）→ FileEntry 映射
- `entries`: directory.list 的缓存结果（旧设计，可保留）
- `last_updated`: RFC3339 格式字符串
- `watch_enabled`: 预留字段

### 3.2 FileEntry 结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FileEntry {
    /// 对应的 Block ID
    pub block_id: String,

    /// 外部文件最后修改时间（RFC3339 格式）
    /// 用于 refresh 时检测外部变更
    pub last_modified: String,
}
```

**时间获取方式**：
```rust
use std::fs;

let last_modified = fs::metadata(path)
    .and_then(|m| m.modified())
    .ok()
    .map(|t| {
        let datetime: chrono::DateTime<chrono::Utc> = t.into();
        datetime.to_rfc3339()
    })
    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
```

**变更检测**：
```rust
let current_modified = get_file_mtime(path)?;
if current_modified != entry.last_modified {
    // 文件已变更
}
```

---

## 4. Capability 设计

### 4.1 Capability 清单

| ID | 职责 | 依赖 root | Event 数量 | 优先级 |
|----|------|----------|-----------|--------|
| `directory.root` | 挂载目录，持久化 root | payload 传入 | 1 | P0 |
| `directory.scan` | 批量导入文件为 Blocks | contents.root | N+1 | P0 |
| `directory.list` | 列出 indexed_files | contents.root | 1 (Read) | P0 |
| `directory.exportall` | 批量导出到文件 | contents.root | N+1 | P0 |
| `directory.refresh` | 检测变更并同步 | contents.root | N+1 | P0 |
| `directory.create` | 创建文件 + Block | contents.root | 2 | P1 |
| `directory.delete` | 删除文件 + Block | contents.root | 2 | P1 |
| `directory.rename` | 重命名文件 + Block | contents.root | 2 | P1 |
| `directory.search` | 搜索文件 | contents.root | 1 (Read) | P2 |

---

### 4.2 directory.root

**功能**：挂载项目目录到 Directory Block，持久化 root 路径和默认配置。

**Payload**（保留现有设计）：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryRootPayload {
    /// 项目根目录绝对路径
    pub root: String,

    /// 默认是否递归
    #[serde(default = "default_true")]
    pub recursive: bool,

    /// 默认是否包含隐藏文件
    #[serde(default)]
    pub include_hidden: bool,

    /// 默认最大深度
    #[serde(default)]
    pub max_depth: Option<usize>,
}

fn default_true() -> bool { true }
```

**执行逻辑**（保留现有实现）：
```rust
#[capability(id = "directory.root", target = "directory")]
pub fn handle_root(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: DirectoryRootPayload = serde_json::from_value(cmd.payload.clone())?;
    let block = block.ok_or("Block required for directory.root capability")?;

    // 验证路径
    let trimmed_root = payload.root.trim();
    if trimmed_root.is_empty() {
        return Err("Root path cannot be empty".into());
    }

    let path = Path::new(trimmed_root);
    if !path.exists() {
        return Err("Root path does not exist".into());
    }
    if !path.is_dir() {
        return Err("Root path must be a directory".into());
    }

    // 规范化路径
    let canonical_root = path.canonicalize()
        .map_err(|e| format!("Failed to canonicalize root: {}", e))?;

    // 创建 Event
    let value = serde_json::json!({
        "root": canonical_root.to_string_lossy(),
        "recursive": payload.recursive,
        "include_hidden": payload.include_hidden,
        "max_depth": payload.max_depth,
        "entries": [],
        "last_updated": chrono::Utc::now().to_rfc3339(),
        "watch_enabled": false,
    });

    let event = create_event(
        block.block_id.clone(),
        "directory.root",
        value,
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

**Event 数量**：1

---

### 4.3 directory.scan

**功能**：批量扫描 contents.root 目录，导入所有文件为 Blocks。

**Payload**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryScanPayload {
    /// 是否递归扫描（覆盖 contents.recursive）
    #[serde(default)]
    pub recursive: Option<bool>,

    /// 是否包含隐藏文件（覆盖 contents.include_hidden）
    #[serde(default)]
    pub include_hidden: Option<bool>,

    /// 最大递归深度（覆盖 contents.max_depth）
    #[serde(default)]
    pub max_depth: Option<usize>,
}
```

**执行逻辑**：
```rust
#[capability(id = "directory.scan", target = "directory")]
pub fn handle_scan(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: DirectoryScanPayload = serde_json::from_value(cmd.payload.clone())?;
    let block = block.ok_or("Block required for directory.scan")?;

    // 从 contents 读取 root
    let root = block.contents.get("root")
        .and_then(|v| v.as_str())
        .ok_or("Directory block has no root. Call directory.root first.")?;

    let project_root = Path::new(root);

    // 读取默认配置或使用 payload 覆盖
    let recursive = payload.recursive.unwrap_or_else(|| {
        block.contents.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    });

    let include_hidden = payload.include_hidden.unwrap_or_else(|| {
        block.contents.get("include_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    });

    let max_depth = payload.max_depth.or_else(|| {
        block.contents.get("max_depth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
    });

    let mut events = vec![];
    let mut indexed_files = HashMap::new();

    // 遍历项目目录
    for entry in walkdir::WalkDir::new(project_root)
        .max_depth(max_depth.unwrap_or(usize::MAX))
        .follow_links(false)
    {
        let entry = entry.map_err(|e| format!("Failed to read directory: {}", e))?;

        if !entry.file_type().is_file() {
            continue;
        }

        // 计算相对路径
        let relative_path = entry.path()
            .strip_prefix(project_root)
            .map_err(|_| "Failed to strip prefix")?
            .to_string_lossy()
            .to_string();

        // 过滤隐藏文件
        if !include_hidden && relative_path.starts_with('.') {
            continue;
        }

        // 读取文件内容
        let content = fs::read_to_string(entry.path())
            .map_err(|e| format!("Failed to read {}: {}", relative_path, e))?;

        // MVP: 所有文件都映射到 markdown
        let block_type = "markdown";

        // 创建 Block（core.create Event）
        let new_block_id = uuid::Uuid::new_v4().to_string();
        events.push(create_event(
            new_block_id.clone(),
            "core.create",
            serde_json::json!({
                "name": entry.file_name().to_string_lossy(),
                "type": block_type,
                "owner": cmd.editor_id,
                "contents": {
                    "markdown": content
                }
            }),
            &cmd.editor_id,
            1
        ));

        // 获取文件 mtime
        let last_modified = entry.metadata()
            .and_then(|m| m.modified())
            .ok()
            .map(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                datetime.to_rfc3339()
            })
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

        // 记录文件映射
        indexed_files.insert(relative_path, FileEntry {
            block_id: new_block_id,
            last_modified,
        });
    }

    // 更新 directory block（directory.scan Event）
    let mut new_contents = block.contents.clone();
    if let Some(obj) = new_contents.as_object_mut() {
        obj.insert("indexed_files".to_string(), serde_json::to_value(&indexed_files)?);
        obj.insert("last_scanned".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
    }

    events.push(create_event(
        block.block_id.clone(),
        "directory.scan",
        serde_json::json!({ "contents": new_contents }),
        &cmd.editor_id,
        1
    ));

    Ok(events)
}
```

**Event 数量**：
- N 个 `core.create` Events
- 1 个 `directory.scan` Event
- **总计：N + 1**

---

### 4.4 directory.list

**功能**：列出 indexed_files，支持过滤。

**Payload**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryListPayload {
    /// 文件名模式（可选）
    #[serde(default)]
    pub pattern: Option<String>,
}
```

**执行逻辑**：
```rust
#[capability(id = "directory.list", target = "directory")]
pub fn handle_list(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: DirectoryListPayload = serde_json::from_value(cmd.payload.clone())?;
    let block = block.ok_or("Block required for directory.list")?;

    // 从 contents 读取 indexed_files
    let indexed_files: HashMap<String, FileEntry> = block.contents
        .get("indexed_files")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // 过滤
    let mut filtered = indexed_files.clone();
    if let Some(pattern) = &payload.pattern {
        filtered.retain(|path, _| path.contains(pattern));
    }

    // 生成 Read Event
    let event = create_event(
        cmd.editor_id.clone(),
        "directory.list",
        serde_json::json!({
            "directory_block_id": block.block_id,
            "files": filtered
        }),
        &cmd.editor_id,
        1
    );

    Ok(vec![event])
}
```

**Event 数量**：1 个 Read Event

---

### 4.5 directory.exportall

**功能**：批量导出 Blocks 到外部文件。

**Payload**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryExportallPayload {
    /// 要导出的项目列表
    pub exports: Vec<ExportItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ExportItem {
    /// Block ID
    pub block_id: String,

    /// 内容（前端提供）
    pub content: String,
}
```

**执行逻辑**：
```rust
#[capability(id = "directory.exportall", target = "directory")]
pub fn handle_exportall(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: DirectoryExportallPayload = serde_json::from_value(cmd.payload.clone())?;
    let block = block.ok_or("Block required for directory.export-all")?;

    // 从 contents 读取 root
    let root = block.contents.get("root")
        .and_then(|v| v.as_str())
        .ok_or("Directory block has no root")?;

    let project_root = Path::new(root);

    // 从 contents 读取 indexed_files
    let mut indexed_files: HashMap<String, FileEntry> = block.contents
        .get("indexed_files")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or("No indexed_files found")?;

    let mut events = vec![];

    for export_item in &payload.exports {
        // 查找文件路径
        let file_path = indexed_files.iter()
            .find(|(_, entry)| entry.block_id == export_item.block_id)
            .map(|(path, _)| path.clone())
            .ok_or_else(|| format!("Block {} not in indexed_files", export_item.block_id))?;

        // 写入外部文件
        let abs_path = project_root.join(&file_path);
        fs::write(&abs_path, &export_item.content)
            .map_err(|e| format!("Failed to write {}: {}", file_path, e))?;

        // 更新 last_modified
        let new_modified = fs::metadata(&abs_path)
            .and_then(|m| m.modified())
            .ok()
            .map(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                datetime.to_rfc3339()
            })
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

        if let Some(entry) = indexed_files.get_mut(&file_path) {
            entry.last_modified = new_modified.clone();
        }

        // 生成 export-block Event
        events.push(create_event(
            block.block_id.clone(),
            "directory.export-block",
            serde_json::json!({
                "block_id": export_item.block_id,
                "file_path": file_path,
                "new_modified": new_modified
            }),
            &cmd.editor_id,
            1
        ));
    }

    // 更新 indexed_files
    let mut new_contents = block.contents.clone();
    if let Some(obj) = new_contents.as_object_mut() {
        obj.insert("indexed_files".to_string(), serde_json::to_value(&indexed_files)?);
    }

    events.push(create_event(
        block.block_id.clone(),
        "directory.update-index",
        serde_json::json!({ "contents": new_contents }),
        &cmd.editor_id,
        1
    ));

    Ok(events)
}
```

**Event 数量**：N + 1（N 个 export-block + 1 个 update-index）

---

### 4.6 directory.refresh

**功能**：检测外部文件变更，同步到 Elfiee。

**Payload**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryRefreshPayload {
    /// 可选：指定文件路径（默认全部刷新）
    #[serde(default)]
    pub file_path: Option<String>,
}
```

**执行逻辑**：
```rust
#[capability(id = "directory.refresh", target = "directory")]
pub fn handle_refresh(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: DirectoryRefreshPayload = serde_json::from_value(cmd.payload.clone())?;
    let block = block.ok_or("Block required for directory.refresh")?;

    // 从 contents 读取 root
    let root = block.contents.get("root")
        .and_then(|v| v.as_str())
        .ok_or("Directory block has no root")?;

    let project_root = Path::new(root);

    // 从 contents 读取 indexed_files
    let mut indexed_files: HashMap<String, FileEntry> = block.contents
        .get("indexed_files")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or("No indexed_files found")?;

    let mut events = vec![];
    let mut detected_changes = vec![];

    for (file_path, entry) in &indexed_files {
        // 如果指定了 file_path，只刷新该文件
        if let Some(ref target) = payload.file_path {
            if file_path != target {
                continue;
            }
        }

        let abs_path = project_root.join(file_path);

        // 读取文件内容
        let content = match fs::read_to_string(&abs_path) {
            Ok(c) => c,
            Err(_) => continue,  // 文件已删除，跳过
        };

        // 获取当前 mtime
        let current_modified = fs::metadata(&abs_path)
            .and_then(|m| m.modified())
            .ok()
            .map(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                datetime.to_rfc3339()
            })
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

        // 比对 mtime
        if current_modified != entry.last_modified {
            detected_changes.push(serde_json::json!({
                "file_path": file_path,
                "block_id": entry.block_id,
                "old_modified": entry.last_modified,
                "new_modified": current_modified
            }));

            // 生成 markdown.write Event（MVP 所有文件都是 markdown）
            events.push(create_event(
                entry.block_id.clone(),
                "markdown.write",
                serde_json::json!({
                    "contents": {
                        "markdown": content
                    }
                }),
                &cmd.editor_id,
                1
            ));

            // 更新 indexed_files 中的 last_modified
            if let Some(file_entry) = indexed_files.get_mut(file_path) {
                file_entry.last_modified = current_modified;
            }
        }
    }

    // 生成 directory.refresh Event
    let mut new_contents = block.contents.clone();
    if let Some(obj) = new_contents.as_object_mut() {
        obj.insert("indexed_files".to_string(), serde_json::to_value(&indexed_files)?);
    }

    events.push(create_event(
        block.block_id.clone(),
        "directory.refresh",
        serde_json::json!({
            "contents": new_contents,
            "detected_changes": detected_changes,
            "refreshed_at": chrono::Utc::now().to_rfc3339()
        }),
        &cmd.editor_id,
        1
    ));

    Ok(events)
}
```

**Event 数量**：N + 1
- N 个 `markdown.write` Events
- 1 个 `directory.refresh` Event

---

### 4.7 directory.create

**功能**：创建外部文件并创建对应 Block。

**Payload**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryCreatePayload {
    /// 文件路径（相对于 contents.root）
    pub path: String,

    /// 类型："file" | "dir"
    pub item_type: String,

    /// 文件内容（可选，默认空）
    #[serde(default)]
    pub content: String,
}
```

**执行逻辑**：
```rust
#[capability(id = "directory.create", target = "directory")]
pub fn handle_create(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: DirectoryCreatePayload = serde_json::from_value(cmd.payload.clone())?;
    let block = block.ok_or("Block required for directory.create")?;

    // 从 contents 读取 root
    let root = block.contents.get("root")
        .and_then(|v| v.as_str())
        .ok_or("Directory block has no root")?;

    let project_root = Path::new(root);
    let abs_path = project_root.join(&payload.path);

    // 创建外部文件或目录
    match payload.item_type.as_str() {
        "file" => {
            fs::write(&abs_path, &payload.content)
                .map_err(|e| format!("Failed to create file: {}", e))?;
        }
        "dir" => {
            fs::create_dir_all(&abs_path)
                .map_err(|e| format!("Failed to create directory: {}", e))?;

            // 目录不创建 Block，只记录创建操作
            return Ok(vec![create_event(
                block.block_id.clone(),
                "directory.create-dir",
                serde_json::json!({ "path": payload.path }),
                &cmd.editor_id,
                1
            )]);
        }
        _ => return Err("Invalid item_type, must be 'file' or 'dir'".into())
    }

    // 创建 Block（仅文件）
    let new_block_id = uuid::Uuid::new_v4().to_string();

    // 获取 mtime
    let last_modified = fs::metadata(&abs_path)
        .and_then(|m| m.modified())
        .ok()
        .map(|t| {
            let datetime: chrono::DateTime<chrono::Utc> = t.into();
            datetime.to_rfc3339()
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    // 更新 indexed_files
    let mut indexed_files: HashMap<String, FileEntry> = block.contents
        .get("indexed_files")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    indexed_files.insert(payload.path.clone(), FileEntry {
        block_id: new_block_id.clone(),
        last_modified,
    });

    let mut new_contents = block.contents.clone();
    if let Some(obj) = new_contents.as_object_mut() {
        obj.insert("indexed_files".to_string(), serde_json::to_value(&indexed_files)?);
    }

    let events = vec![
        // Event 1: core.create
        create_event(
            new_block_id.clone(),
            "core.create",
            serde_json::json!({
                "name": Path::new(&payload.path).file_name().unwrap().to_string_lossy(),
                "type": "markdown",
                "owner": cmd.editor_id,
                "contents": {
                    "markdown": payload.content
                }
            }),
            &cmd.editor_id,
            1
        ),
        // Event 2: directory.create
        create_event(
            block.block_id.clone(),
            "directory.create",
            serde_json::json!({
                "contents": new_contents,
                "path": payload.path,
                "block_id": new_block_id
            }),
            &cmd.editor_id,
            1
        )
    ];

    Ok(events)
}
```

**Event 数量**：2

---

### 4.8 directory.delete

**功能**：删除外部文件并软删除对应 Block。

**Payload**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryDeletePayload {
    /// 文件路径（相对于 contents.root）
    pub path: String,

    /// 是否递归删除目录
    #[serde(default)]
    pub recursive: bool,
}
```

**执行逻辑**：
```rust
#[capability(id = "directory.delete", target = "directory")]
pub fn handle_delete(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: DirectoryDeletePayload = serde_json::from_value(cmd.payload.clone())?;
    let block = block.ok_or("Block required for directory.delete")?;

    // 从 contents 读取 root
    let root = block.contents.get("root")
        .and_then(|v| v.as_str())
        .ok_or("Directory block has no root")?;

    let project_root = Path::new(root);
    let abs_path = project_root.join(&payload.path);

    // 从 indexed_files 获取 block_id
    let mut indexed_files: HashMap<String, FileEntry> = block.contents
        .get("indexed_files")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or("No indexed_files found")?;

    let entry = indexed_files.get(&payload.path)
        .ok_or_else(|| format!("File {} not in indexed_files", payload.path))?
        .clone();

    // 删除外部文件
    let metadata = fs::metadata(&abs_path)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;

    if metadata.is_dir() {
        if !payload.recursive {
            return Err("Cannot delete directory without recursive flag".into());
        }
        fs::remove_dir_all(&abs_path)
            .map_err(|e| format!("Failed to delete directory: {}", e))?;
    } else {
        fs::remove_file(&abs_path)
            .map_err(|e| format!("Failed to delete file: {}", e))?;
    }

    // 从 indexed_files 移除
    indexed_files.remove(&payload.path);

    let mut new_contents = block.contents.clone();
    if let Some(obj) = new_contents.as_object_mut() {
        obj.insert("indexed_files".to_string(), serde_json::to_value(&indexed_files)?);
    }

    let events = vec![
        // Event 1: core.delete（软删除 Block）
        create_event(
            entry.block_id.clone(),
            "core.delete",
            serde_json::json!({ "deleted": true }),
            &cmd.editor_id,
            1
        ),
        // Event 2: directory.delete
        create_event(
            block.block_id.clone(),
            "directory.delete",
            serde_json::json!({
                "contents": new_contents,
                "path": payload.path,
                "block_id": entry.block_id
            }),
            &cmd.editor_id,
            1
        )
    ];

    Ok(events)
}
```

**Event 数量**：2

**说明**：`core.delete` 是软删除，只标记 `deleted: true`，Block ID 仍然有效，可以继续写 Event。

---

### 4.9 directory.rename

**功能**：重命名外部文件并更新对应 Block。

**Payload**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryRenamePayload {
    /// 旧路径（相对于 contents.root）
    pub old_path: String,

    /// 新路径（相对于 contents.root）
    pub new_path: String,
}
```

**执行逻辑**：
```rust
#[capability(id = "directory.rename", target = "directory")]
pub fn handle_rename(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: DirectoryRenamePayload = serde_json::from_value(cmd.payload.clone())?;
    let block = block.ok_or("Block required for directory.rename")?;

    // 从 contents 读取 root
    let root = block.contents.get("root")
        .and_then(|v| v.as_str())
        .ok_or("Directory block has no root")?;

    let project_root = Path::new(root);
    let old_abs = project_root.join(&payload.old_path);
    let new_abs = project_root.join(&payload.new_path);

    // 重命名外部文件
    fs::rename(&old_abs, &new_abs)
        .map_err(|e| format!("Failed to rename file: {}", e))?;

    // 更新 indexed_files
    let mut indexed_files: HashMap<String, FileEntry> = block.contents
        .get("indexed_files")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or("No indexed_files found")?;

    let entry = indexed_files.remove(&payload.old_path)
        .ok_or_else(|| format!("File {} not in indexed_files", payload.old_path))?;

    let new_modified = fs::metadata(&new_abs)
        .and_then(|m| m.modified())
        .ok()
        .map(|t| {
            let datetime: chrono::DateTime<chrono::Utc> = t.into();
            datetime.to_rfc3339()
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    indexed_files.insert(payload.new_path.clone(), FileEntry {
        block_id: entry.block_id.clone(),
        last_modified: new_modified,
    });

    let mut new_contents = block.contents.clone();
    if let Some(obj) = new_contents.as_object_mut() {
        obj.insert("indexed_files".to_string(), serde_json::to_value(&indexed_files)?);
    }

    let new_name = Path::new(&payload.new_path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let events = vec![
        // Event 1: 更新 Block.name
        create_event(
            entry.block_id.clone(),
            "core.update-name",
            serde_json::json!({ "name": new_name }),
            &cmd.editor_id,
            1
        ),
        // Event 2: directory.rename
        create_event(
            block.block_id.clone(),
            "directory.rename",
            serde_json::json!({
                "contents": new_contents,
                "old_path": payload.old_path,
                "new_path": payload.new_path,
                "block_id": entry.block_id
            }),
            &cmd.editor_id,
            1
        )
    ];

    Ok(events)
}
```

**Event 数量**：2

---

### 4.10 directory.search

**功能**：搜索文件名或内容。

**Payload**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectorySearchPayload {
    /// 搜索模式（支持通配符）
    pub pattern: String,

    /// 是否递归搜索
    #[serde(default = "default_true")]
    pub recursive: bool,

    /// 是否包含隐藏文件
    #[serde(default)]
    pub include_hidden: bool,
}

fn default_true() -> bool { true }
```

**执行逻辑**：复用现有 `directory_search.rs` 实现，从 contents.root 读取项目根目录。

**Event 数量**：1 个 Read Event

---

## 5. CBAC 权限模型

| Capability | Owner | Collaborator | Guest |
|-----------|-------|--------------|-------|
| `directory.root` | ✅ | ❌ | ❌ |
| `directory.scan` | ✅ | ❌ | ❌ |
| `directory.list` | ✅ | ✅（权限过滤） | ✅（权限过滤） |
| `directory.exportall` | ✅ | ✅（需 grant） | ❌ |
| `directory.refresh` | ✅ | ✅（需 grant，部分同步） | ❌ |
| `directory.create` | ✅ | ✅（需 grant） | ❌ |
| `directory.delete` | ✅ | ✅（需 grant） | ❌ |
| `directory.rename` | ✅ | ✅（需 grant） | ❌ |
| `directory.search` | ✅ | ✅ | ✅ |

---

## 6. 实现要点

### 6.1 复用现有代码

**可复用**：
- ✅ `utils.rs` - read_dir_single, read_dir_recursive
- ✅ `directory_search.rs` - 搜索功能
- ✅ `directory_root.rs` - 完整保留

**需要重写**：
- `directory_list.rs` - 改为读取 indexed_files
- `directory_refresh.rs` - 改为 mtime 检测
- `directory_create/delete/rename.rs` - 从 contents.root 读取路径

### 6.2 时间格式统一

所有 Block.contents 中的时间字段使用 RFC3339 字符串：
```rust
chrono::Utc::now().to_rfc3339()  // "2025-01-19T10:00:00Z"
```

### 6.3 类型映射

MVP 硬编码：
```rust
let block_type = "markdown";  // 所有文件都映射到 markdown
```

### 6.4 前端 Dirty 状态管理

```typescript
const dirtyBlocks = new Set<string>();

// markdown.write 后
dirtyBlocks.add("block-markdown-001");

// 导出时
await invoke('execute_command', {
  cap_id: 'directory.export-all',
  payload: {
    exports: Array.from(dirtyBlocks).map(id => ({
      block_id: id,
      content: getBlockContent(id)
    }))
  }
});

dirtyBlocks.clear();
```

---

## 7. 测试覆盖率目标

| 测试类型 | 数量 | 覆盖率目标 |
|---------|------|-----------|
| Payload 测试 | 9 | 100% |
| 功能测试 | 9 | >90% |
| 授权测试 | 27 | 100% |
| 工作流测试 | 4 | >80% |
| **总计** | **49** | **>90%** |

---

## 8. 依赖

```toml
[dependencies]
walkdir = "2"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
```

---

**状态**: ✅ 最终执行方案完成
**下一步**: 使用 elfiee-ext-gen 重新生成 Directory Extension

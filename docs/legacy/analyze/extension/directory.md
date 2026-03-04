# Directory Extension Development Plan

## 1. 功能概述

### 核心理念

`directory` extension 直接操作文件系统，提供类似 VSCode Explorer 的文件管理功能。

**核心概念**：
- **directory block** = 代表文件系统中某个具体目录（如 `/home/user/projects`）
- **直接文件系统操作** = 所有 CRUD 操作直接修改文件系统（创建/删除/重命名实际文件）
- **Block 只存储视图** = `Block.contents` 缓存最近的 `list` 结果，不存储文件内容
- **类比 VSCode** = 类似 VSCode 的 New File/Delete/Rename 操作

### 能力清单

所有能力一次性实现（因 generator 不支持增量添加）：

| Capability | 用途 | 文件系统操作 |
|-----------|------|-------------|
| `directory.list` | **查**：列出目录内容 | 读取文件系统结构（非递归为默认） |
| `directory.create` | **增**：创建空文件/目录 | `fs::write(path, "")` 或 `fs::create_dir()` |
| `directory.delete` | **删**：删除文件/目录 | `fs::remove_file()` 或 `fs::remove_dir_all()` |
| `directory.rename` | **改**：重命名或移动 | `fs::rename(old, new)` |
| `directory.refresh` | 重新扫描目录，更新缓存 | 重新执行 `list` 逻辑 |
| `directory.watch` | 启用/禁用文件系统监听 | 配置 `watch_enabled` 标志 |
| `directory.search` | 搜索文件名或内容 | 遍历文件系统 + 模式匹配 |

**重要说明**：
- `create` 创建的文件默认为**空文件**（类似 VSCode New File）
- `delete` 删除目录时必须设置 `recursive=true`，会删除目录下所有内容
- `list` 默认 `recursive=false`（仅单层），避免大目录性能问题
- `watch` 仅设置标志，不实现真正的文件系统监听（预留接口）
- `search` 支持文件名模糊匹配

---

## 2. Payload 设计

### 2.1 DirectoryListPayload

```rust
/// List 能力：列出目录内容
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryListPayload {
    /// 可选：相对于 block root 的子路径，默认为 root
    pub path: Option<String>,

    /// 是否递归列出子目录
    #[serde(default)]
    pub recursive: bool,

    /// 是否包含隐藏文件（以 . 开头）
    #[serde(default)]
    pub include_hidden: bool,

    /// 最大递归深度，None 表示无限制
    #[serde(default)]
    pub max_depth: Option<usize>,
}
```

**字段推断说明**：
- `path`: 允许查询子路径，灵活性高
- `recursive`: 布尔标志，默认 false（单层）
- `include_hidden`: 布尔标志，默认 false
- `max_depth`: 防止深度遍历性能问题

### 2.2 DirectoryCreatePayload

```rust
/// Create 能力：创建文件或子目录
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryCreatePayload {
    /// 相对于 block root 的路径
    pub path: String,

    /// 创建类型："file" 或 "dir"
    pub item_type: String,

    /// 文件初始内容（可选，默认空文件）
    /// 仅 item_type = "file" 时有效
    #[serde(default)]
    pub content: Option<String>,
}
```

**字段说明**：
- `content`: 可选字段，默认 `None`
  - `None` → 创建空文件（类似 VSCode New File）
  - `Some("")` → 创建空文件
  - `Some("text")` → 创建带初始内容的文件

**验证规则**：
- `item_type` 必须为 `"file"` 或 `"dir"`
- `path` 不能为空或包含 `..`（安全考虑）
- `path` 不能已存在（避免覆盖）

### 2.3 DirectoryDeletePayload

```rust
/// Delete 能力：删除文件或子目录
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryDeletePayload {
    /// 相对于 block root 的路径
    pub path: String,

    /// 是否递归删除目录（删除目录必须为 true）
    #[serde(default)]
    pub recursive: bool,
}
```

**安全考虑**：
- 删除目录必须设置 `recursive = true`
- 不允许删除 root（`path` 不能为空）
- 检查路径是否在 block root 范围内

### 2.4 DirectoryRenamePayload

```rust
/// Rename 能力：重命名或移动文件/目录
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryRenamePayload {
    /// 旧路径（相对于 block root）
    pub old_path: String,

    /// 新路径（相对于 block root）
    pub new_path: String,
}
```

**行为说明**：
- 同目录内改名：`old_path = "file.txt"`, `new_path = "renamed.txt"`
- 移动到子目录：`old_path = "file.txt"`, `new_path = "subdir/file.txt"`

### 2.5 DirectoryRefreshPayload

```rust
/// Refresh 能力：重新扫描目录，更新缓存
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryRefreshPayload {
    /// 是否递归刷新（默认 false）
    #[serde(default)]
    pub recursive: bool,
}
```

**字段说明**：
- `recursive`: 是否递归刷新子目录

### 2.6 DirectoryWatchPayload

```rust
/// Watch 能力：启用/禁用文件系统监听
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryWatchPayload {
    /// 是否启用监听
    pub enabled: bool,
}
```

**字段说明**：
- `enabled`: `true` 启用监听，`false` 禁用监听
- 当前仅设置 `Block.contents.watch_enabled` 标志，不实现真正的文件系统监听

### 2.7 DirectorySearchPayload

```rust
/// Search 能力：搜索文件名或内容
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectorySearchPayload {
    /// 搜索模式（文件名模糊匹配）
    pub pattern: String,

    /// 是否递归搜索子目录（默认 true）
    #[serde(default = "default_true")]
    pub recursive: bool,

    /// 是否包含隐藏文件（默认 false）
    #[serde(default)]
    pub include_hidden: bool,
}

fn default_true() -> bool {
    true
}
```

**字段说明**：
- `pattern`: 搜索模式，支持通配符（如 `*.rs`、`test_*`）
- `recursive`: 默认 `true`，递归搜索所有子目录
- `include_hidden`: 默认 `false`，不搜索隐藏文件

**验证规则**：
- `pattern` 不能为空

---

## 3. Block Contents 结构设计

每个 `directory` block 的 `contents` 字段应包含：

```json
{
  "root": "/home/user/projects",
  "entries": [
    {
      "name": "file1.txt",
      "type": "file",
      "size": 1024,
      "modified": "2025-11-02T10:00:00Z",
      "path": "file1.txt"
    },
    {
      "name": "subdir",
      "type": "dir",
      "path": "subdir",
      "children": [
        {
          "name": "nested.txt",
          "type": "file",
          "size": 512,
          "modified": "2025-11-02T09:00:00Z",
          "path": "subdir/nested.txt"
        }
      ]
    }
  ],
  "last_updated": "2025-11-02T10:30:00Z",
  "watch_enabled": false
}
```

**字段说明**：
- `root`: 块代表的根目录绝对路径
- `entries`: 最后一次 `list` 操作的结果（缓存）
- `last_updated`: 最后更新时间（**RFC3339 格式**，通过 `chrono::Utc::now().to_rfc3339()` 生成）
- `modified`: 文件修改时间（**RFC3339 格式**）
- `watch_enabled`: 是否启用文件系统监听（P1 功能）

**时间戳标准**：
- **项目统一使用** `chrono` crate
- **格式**：RFC3339（例如 `"2025-11-02T10:30:00Z"`）
- **生成方式**：`chrono::Utc::now().to_rfc3339()`
- **依赖**：已在 `Cargo.toml` 中声明 `chrono = { version = "0.4", features = ["serde"] }`

---

## 4. Handler 实现要点

### 4.1 directory.list Handler

**实现步骤**：

1. **反序列化 Payload**
   ```rust
   let payload: DirectoryListPayload = serde_json::from_value(cmd.payload.clone())
       .map_err(|e| format!("Invalid payload: {}", e))?;
   ```

2. **确定目标路径**
   ```rust
   let block = block.ok_or("Block required for directory.list")?;
   let root = block.contents.get("root")
       .and_then(|v| v.as_str())
       .ok_or("Missing 'root' in block contents")?;

   let target_path = if let Some(sub) = &payload.path {
       PathBuf::from(root).join(sub)
   } else {
       PathBuf::from(root)
   };
   ```

3. **验证路径安全性**
   ```rust
   // 检查路径是否在 root 范围内
   let canonical_target = target_path.canonicalize()
       .map_err(|e| format!("Invalid path: {}", e))?;
   let canonical_root = PathBuf::from(root).canonicalize()
       .map_err(|e| format!("Invalid root: {}", e))?;

   if !canonical_target.starts_with(&canonical_root) {
       return Err("Path outside root directory".to_string());
   }
   ```

4. **递归列出目录**
   ```rust
   fn list_entries(
       path: &Path,
       recursive: bool,
       include_hidden: bool,
       current_depth: usize,
       max_depth: Option<usize>,
   ) -> Result<Vec<serde_json::Value>, String> {
       if let Some(max) = max_depth {
           if current_depth >= max {
               return Ok(vec![]);
           }
       }

       let mut entries = vec![];
       for entry in fs::read_dir(path)
           .map_err(|e| format!("Failed to read directory: {}", e))?
       {
           let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
           let file_name = entry.file_name().to_string_lossy().to_string();

           // 过滤隐藏文件
           if !include_hidden && file_name.starts_with('.') {
               continue;
           }

           let metadata = entry.metadata()
               .map_err(|e| format!("Failed to read metadata: {}", e))?;

           let modified = metadata.modified()
               .ok()
               .map(|t| {
                   let datetime: chrono::DateTime<chrono::Utc> = t.into();
                   datetime.to_rfc3339()
               });

           let mut entry_obj = serde_json::json!({
               "name": file_name,
               "type": if metadata.is_dir() { "dir" } else { "file" },
               "path": entry.path().strip_prefix(root).unwrap().to_string_lossy(),
               "size": metadata.len(),
               "modified": modified,
           });

           // 递归处理子目录
           if recursive && metadata.is_dir() {
               let children = list_entries(
                   &entry.path(),
                   true,
                   include_hidden,
                   current_depth + 1,
                   max_depth,
               )?;
               entry_obj["children"] = serde_json::json!(children);
           }

           entries.push(entry_obj);
       }

       Ok(entries)
   }
   ```

5. **生成事件**
   ```rust
   let entries = list_entries(&target_path, payload.recursive, payload.include_hidden, 0, payload.max_depth)?;

   let event = create_event(
       block.block_id.clone(),
       "directory.list",
       serde_json::json!({
           "root": root,
           "entries": entries,
           "last_updated": chrono::Utc::now().to_rfc3339(),
       }),
       &cmd.editor_id,
       1,
   );

   Ok(vec![event])
   ```

### 4.2 directory.create Handler

**关键步骤**：

1. 反序列化并验证 `item_type`
2. 构建目标路径并检查安全性
3. 检查路径是否已存在
4. 根据 `item_type` 创建文件或目录
   ```rust
   match payload.item_type.as_str() {
       "file" => {
           // 默认创建空文件（类似 VSCode New File）
           let content = payload.content.unwrap_or_default();
           fs::write(&target_path, content)
               .map_err(|e| format!("Failed to create file: {}", e))?;
       }
       "dir" => {
           fs::create_dir_all(&target_path)
               .map_err(|e| format!("Failed to create directory: {}", e))?;
       }
       _ => return Err(format!("Invalid item_type: {}", payload.item_type)),
   }
   ```
5. 生成事件记录创建操作
   ```rust
   let event = create_event(
       block.block_id.clone(),
       "directory.create",
       serde_json::json!({
           "path": payload.path,
           "item_type": payload.item_type,
           "timestamp": chrono::Utc::now().to_rfc3339(),
       }),
       &cmd.editor_id,
       1,
   );
   ```

### 4.3 directory.delete Handler

**关键步骤**：

1. 验证路径存在且在 root 范围内
2. 检查是否为目录，若是则要求 `recursive = true`
   ```rust
   if metadata.is_dir() && !payload.recursive {
       return Err("Cannot delete directory without recursive flag".to_string());
   }
   ```
3. 执行删除
   ```rust
   if metadata.is_dir() {
       fs::remove_dir_all(&target_path)
   } else {
       fs::remove_file(&target_path)
   }.map_err(|e| format!("Failed to delete: {}", e))?;
   ```

### 4.4 directory.rename Handler

**关键步骤**：

1. 验证 `old_path` 存在
2. 验证 `new_path` 不存在
3. 验证两个路径都在 root 范围内
4. 执行重命名
   ```rust
   fs::rename(&old_full_path, &new_full_path)
       .map_err(|e| format!("Failed to rename: {}", e))?;
   ```

---

## 5. 测试策略

### 5.1 自动生成的测试（由 elfiee-ext-gen 提供）

Generator 为每个 capability 自动生成以下测试骨架：

#### Payload 测试（每个能力 1 个，共 7 个）
```rust
#[test]
fn test_list_payload_deserialize() {
    let json = serde_json::json!({
        "path": "/",
        "recursive": true,
        "include_hidden": false,
        "max_depth": 3
    });

    let result: Result<DirectoryListPayload, _> = serde_json::from_value(json);
    assert!(result.is_ok());

    let payload = result.unwrap();
    assert_eq!(payload.path, Some("/".to_string()));
    assert_eq!(payload.recursive, true);
}
```

#### 功能测试骨架（每个能力 1 个，共 7 个）
```rust
#[test]
fn test_list_basic() {
    // TODO: 创建临时目录和文件
    // TODO: 调用 handler
    // TODO: 验证返回的 events 包含正确的 entries
}
```

**说明**：基础功能测试，验证正常路径。开发者需要在此基础上补充边界条件测试。

#### 授权测试（每个能力 3 个，共 21 个，自动通过）
- `test_list_authorization_owner` - 验证 owner 总是有权限
- `test_list_authorization_non_owner_without_grant` - 验证非 owner 无授权时被拒绝
- `test_list_authorization_non_owner_with_grant` - 验证非 owner 获得授权后可执行

**说明**：授权测试自动通过，无需修改。

#### 工作流测试骨架（整个扩展 1 个）
```rust
#[test]
fn test_full_workflow() {
    // TODO: 模拟完整流程
    // 1. list 列出目录
    // 2. create 创建文件
    // 3. list 再次列出，验证新文件出现
    // 4. rename 重命名文件
    // 5. delete 删除文件
    // 6. list 验证文件消失
}
```

**说明**：基础工作流测试，验证多个能力的协作。开发者需要在此基础上补充复杂场景测试。

**自动生成总计**：7 payload + 7 功能 + 21 授权 + 1 工作流 = **36 个测试**

### 5.2 需要补充的测试

生成的测试模板只包含基础测试骨架，开发者需要补充以下测试：

#### 功能测试补充（边界条件，可变数量）

在每个 capability 的基础功能测试（`test_xxx_basic`）之外，补充边界条件测试：

```rust
// list 的边界条件测试
#[test]
fn test_list_with_hidden_files() {
    // 测试 include_hidden 标志
}

#[test]
fn test_list_max_depth() {
    // 测试递归深度限制
}

// create 的边界条件测试
#[test]
fn test_create_duplicate_file() {
    // 测试创建已存在的文件应失败
}

// delete 的边界条件测试
#[test]
fn test_delete_non_empty_directory_without_recursive() {
    // 测试删除非空目录必须设置 recursive
}

// 安全性测试
#[test]
fn test_path_traversal_attack() {
    // 测试 path = "../../../etc/passwd" 应被拒绝
}
```

**预计补充**：约 5-10 个边界条件测试

#### 工作流测试补充（复杂场景，可变数量）

在基础工作流测试（`test_full_workflow`）之外，补充复杂的多步骤场景：

```rust
#[test]
fn test_create_nested_structure() {
    // 创建复杂的目录结构
    // subdir1/
    //   subdir2/
    //     file.txt
    // 验证 list 能正确显示嵌套结构
}

#[test]
fn test_rename_to_existing_name() {
    // 测试重命名到已存在的名称应失败
}

#[test]
fn test_refresh_after_external_change() {
    // 外部修改文件系统后，refresh 能正确更新缓存
}

#[test]
fn test_search_with_pattern() {
    // 创建多个文件，用 search 验证模式匹配
}
```

**预计补充**：约 3-5 个复杂工作流测试

---

**最终测试总数**：36（自动生成）+ 8-15（补充）= **约 44-51 个测试**

---

## 6. 使用 Generator 的开发步骤

### ⚠️ 重要：Generator 覆盖行为

**核心提示**：
- ⚠️ `elfiee-ext-gen create` 会**完全覆盖**扩展目录下的所有文件
- ⚠️ 包括已实现的 handler 代码和测试
- ✅ 初次生成时一次性确定所有 P0 capabilities，避免后续覆盖
- ✅ 每次运行 `create` 前先 `git commit` 保存进度

**详细说明**：参见 `elfiee-ext-gen/docs/progress.md` → "⚠️ 重要提示：`create` 命令覆盖行为"章节

---

### Step 1: 生成扩展骨架

```bash
# 在项目根目录运行（不是 src-tauri 目录）
cd /home/yaosh/projects/elfiee

# 生成所有 7 个能力（一次性完成）
elfiee-ext-gen create \
  -n directory \
  -b directory \
  -c list,create,delete,rename,refresh,watch,search
```

**预期输出**：
```
✅ Generated files:
  - src-tauri/src/extensions/directory/mod.rs
  - src-tauri/src/extensions/directory/directory_list.rs
  - src-tauri/src/extensions/directory/directory_create.rs
  - src-tauri/src/extensions/directory/directory_delete.rs
  - src-tauri/src/extensions/directory/directory_rename.rs
  - src-tauri/src/extensions/directory/directory_refresh.rs
  - src-tauri/src/extensions/directory/directory_watch.rs
  - src-tauri/src/extensions/directory/directory_search.rs
  - src-tauri/src/extensions/directory/tests.rs
  - src-tauri/src/extensions/directory/DEVELOPMENT_GUIDE.md

✅ Updated registrations:
  - src-tauri/src/extensions/mod.rs
  - src-tauri/src/capabilities/registry.rs
  - src-tauri/src/lib.rs
```

### Step 2: 运行初始测试（预期失败）

```bash
cd /home/yaosh/projects/elfiee/src-tauri

cargo test directory::tests --lib
```

**预期结果**：
- ✅ 授权测试通过（3 × 7 = 21 个）
- ❌ Payload 测试失败（7 个）- 因为字段未定义
- ❌ 功能测试失败（7 个）- 因为有 `todo!()`
- ❌ 工作流测试失败（1 个）- 因为有 `todo!()`

**总计**：21 passed, 15 failed（共 36 个自动生成的测试）

### Step 3: 查看开发指南

```bash
cat src/extensions/directory/DEVELOPMENT_GUIDE.md
```

或使用 guide 命令（回到项目根目录）：
```bash
cd /home/yaosh/projects/elfiee
elfiee-ext-gen guide directory
```

### Step 4: 定义 Payload 字段（mod.rs）

编辑 `src/extensions/directory/mod.rs`，根据第 2 节的设计，定义所有 Payload 结构体。

**修改前**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryListPayload {
    /// Generic data field - replace with specific fields
    pub data: serde_json::Value,
}
```

**修改后**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DirectoryListPayload {
    pub path: Option<String>,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default)]
    pub max_depth: Option<usize>,
}
```

对所有 7 个 Payload 重复此操作（list, create, delete, rename, refresh, watch, search）。

### Step 5: 更新测试中的 JSON 示例（tests.rs）

编辑 `src/extensions/directory/tests.rs`，为每个 Payload 测试提供正确的 JSON：

```rust
#[test]
fn test_list_payload_deserialize() {
    let json = serde_json::json!({
        "path": Some("/subdir"),
        "recursive": true,
        "include_hidden": false,
        "max_depth": 3,
    });

    let result: Result<DirectoryListPayload, _> = serde_json::from_value(json);
    assert!(result.is_ok());

    let payload = result.unwrap();
    assert_eq!(payload.path, Some("/subdir".to_string()));
    assert_eq!(payload.recursive, true);
}
```

**运行测试验证**：
```bash
cd /home/yaosh/projects/elfiee/src-tauri
cargo test directory::tests::test_list_payload_deserialize
```

**预期**: ✅ 测试通过

### Step 6: 实现 Handler 逻辑

按照第 4 节的实现要点，逐个实现 capability handler。

**推荐顺序**：
1. `directory.list` - 最基础，其他能力依赖它验证结果
2. `directory.create` - 创建测试数据
3. `directory.delete` - 清理测试数据
4. `directory.rename` - 需要 create/delete 验证
5. `directory.refresh` - 复用 list 逻辑
6. `directory.watch` - 简单的标志设置
7. `directory.search` - 最复杂，需要模式匹配

**每完成一个 handler，立即测试**：
```bash
cargo test directory::tests::test_list_basic
cargo test directory::tests::test_create_basic
cargo test directory::tests::test_refresh_basic
# ... 依次类推
```

### Step 7: 完善功能测试

编辑 `tests.rs`，为每个功能测试补充具体逻辑：

```rust
#[test]
fn test_list_basic() {
    use tempfile::TempDir;
    use std::fs;

    // 1. 创建临时目录和测试文件
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file1.txt"), "content1").unwrap();
    fs::create_dir(temp.path().join("subdir")).unwrap();
    fs::write(temp.path().join("subdir/file2.txt"), "content2").unwrap();

    // 2. 创建 block
    let mut block = Block::new(
        "Test Directory".to_string(),
        "directory".to_string(),
        "alice".to_string(),
    );
    block.contents = serde_json::json!({
        "root": temp.path().to_string_lossy(),
    });

    // 3. 构建 Command
    let cmd = Command::new(
        "alice".to_string(),
        "directory.list".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "recursive": true,
            "include_hidden": false,
        }),
    );

    // 4. 获取 capability 并执行
    let registry = CapabilityRegistry::new();
    let cap = registry.get("directory.list").unwrap();
    let events = cap.handler(&cmd, Some(&block)).unwrap();

    // 5. 验证结果
    assert_eq!(events.len(), 1);
    let entries = events[0].value.get("entries").unwrap();
    assert!(entries.as_array().unwrap().len() >= 2); // file1.txt + subdir
}
```

### Step 8: 实现工作流测试

编辑 `tests.rs` 中的 `test_full_workflow`，模拟完整的 CRUD 流程：

```rust
#[test]
fn test_full_workflow() {
    use tempfile::TempDir;

    let temp = TempDir::new().unwrap();
    let registry = CapabilityRegistry::new();

    let mut block = Block::new(
        "Workflow Test".to_string(),
        "directory".to_string(),
        "alice".to_string(),
    );
    block.contents = serde_json::json!({
        "root": temp.path().to_string_lossy(),
    });

    // Step 1: List - 空目录
    let list_cmd = Command::new(
        "alice".to_string(),
        "directory.list".to_string(),
        block.block_id.clone(),
        serde_json::json!({ "recursive": false }),
    );
    let events = registry.get("directory.list").unwrap()
        .handler(&list_cmd, Some(&block)).unwrap();
    let entries = events[0].value.get("entries").unwrap().as_array().unwrap();
    assert_eq!(entries.len(), 0);

    // Step 2: Create - 创建文件
    let create_cmd = Command::new(
        "alice".to_string(),
        "directory.create".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "path": "test.txt",
            "item_type": "file",
            "content": "Hello",
        }),
    );
    registry.get("directory.create").unwrap()
        .handler(&create_cmd, Some(&block)).unwrap();

    // Step 3: List - 验证文件出现
    let events = registry.get("directory.list").unwrap()
        .handler(&list_cmd, Some(&block)).unwrap();
    let entries = events[0].value.get("entries").unwrap().as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["name"], "test.txt");

    // Step 4: Rename - 重命名
    let rename_cmd = Command::new(
        "alice".to_string(),
        "directory.rename".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "old_path": "test.txt",
            "new_path": "renamed.txt",
        }),
    );
    registry.get("directory.rename").unwrap()
        .handler(&rename_cmd, Some(&block)).unwrap();

    // Step 5: Delete - 删除文件
    let delete_cmd = Command::new(
        "alice".to_string(),
        "directory.delete".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "path": "renamed.txt",
            "recursive": false,
        }),
    );
    registry.get("directory.delete").unwrap()
        .handler(&delete_cmd, Some(&block)).unwrap();

    // Step 6: List - 验证文件消失
    let events = registry.get("directory.list").unwrap()
        .handler(&list_cmd, Some(&block)).unwrap();
    let entries = events[0].value.get("entries").unwrap().as_array().unwrap();
    assert_eq!(entries.len(), 0);
}
```

### Step 9: 运行完整测试套件

```bash
cd /home/yaosh/projects/elfiee/src-tauri

# 运行所有 directory 测试
cargo test directory::tests --lib
```

**预期**：
- ✅ 自动生成的测试全部通过（36/36）
  - 7 payload + 7 功能 + 21 授权 + 1 工作流
- ✅ 补充的边界条件测试通过（5-10 个）
- ✅ 补充的复杂工作流测试通过（3-5 个）

**最终总计**：约 44-51 个测试全部通过

### Step 10: 验证扩展完整性

```bash
cd /home/yaosh/projects/elfiee

elfiee-ext-gen validate directory
```

**验证内容**：
- ✅ 模块导出正确（`src/extensions/mod.rs`）
- ✅ Capability 已注册（`src/capabilities/registry.rs`）
- ✅ Specta 类型已注册（`src/lib.rs`）
- ✅ 所有 Payload 结构体已定义

### Step 11: 在 Tauri 应用中测试

```bash
cd /home/yaosh/projects/elfiee

# 运行开发服务器
pnpm tauri dev
```

在前端调用测试：
```typescript
import { invoke } from '@tauri-apps/api/core';

// 列出目录
const result = await invoke('execute_command', {
  cmd: {
    cmd_id: crypto.randomUUID(),
    editor_id: 'user1',
    cap_id: 'directory.list',
    block_id: 'block-uuid',
    payload: {
      recursive: true,
      include_hidden: false,
      max_depth: 3,
    },
  },
});
```

---

## 7. 参考资料

### 系统对比

| 系统 | List API | Create API | Delete API | Rename API |
|------|----------|-----------|-----------|-----------|
| **VSCode** | `workspace.fs.readDirectory()` | `workspace.fs.createDirectory()` | `workspace.fs.delete()` | `workspace.fs.rename()` |
| **Node.js** | `fs.readdir()` | `fs.mkdir()` / `fs.writeFile()` | `fs.rm()` / `fs.unlink()` | `fs.rename()` |
| **Elfiee** | `directory.list` | `directory.create` | `directory.delete` | `directory.rename` |

### 安全考虑

1. **路径遍历攻击防护**
   - 使用 `canonicalize()` 标准化路径
   - 验证最终路径是否在 root 范围内
   - 拒绝包含 `..` 的路径

2. **权限检查**
   - CBAC 授权在 Engine 层自动处理
   - 可选：在 handler 中添加额外的文件系统权限检查

3. **资源限制**
   - `max_depth` 防止深度遍历导致性能问题
   - 考虑添加 `max_entries` 限制返回条目数

---

## 8. 实现范围

所有能力一次性实现（因 generator 不支持增量添加）：

| Capability | 实现内容 |
|-----------|---------|
| `directory.list` | 读取文件系统结构（支持 recursive/hidden/max_depth） |
| `directory.create` | 创建空文件或目录 |
| `directory.delete` | 删除文件或目录 |
| `directory.rename` | 重命名或移动文件/目录 |
| `directory.refresh` | 重新扫描目录，更新 `Block.contents` 缓存 |
| `directory.watch` | 设置 `watch_enabled` 标志（预留接口）|
| `directory.search` | 文件名模式匹配搜索 |

**技术要求**：
- ✅ 时间戳统一使用 `chrono::Utc::now().to_rfc3339()`
- ✅ 创建文件默认为空文件（`content: Option<String>` 默认 `None`）
- ✅ `list` 默认 `recursive=false`（性能优化）
- ✅ 路径安全检查（防止路径遍历攻击）
- ✅ `rename` 只操作文件系统，不维护缓存（由 `refresh` 负责）
- ✅ 完整测试覆盖：
  - 自动生成：36 个（7 payload + 7 功能 + 21 授权 + 1 工作流）
  - 补充测试：约 8-15 个（边界条件 + 复杂工作流）
  - 总计：约 44-51 个

**验收标准**：
- [ ] 所有测试通过 `cargo test directory::tests`（约 44-51 个）
- [ ] `elfiee-ext-gen validate directory` 通过
- [ ] 在 Tauri 应用中手动测试所有能力

---

**开发进度跟踪**：详见 `docs/analyze/extension/directory-progress.md`

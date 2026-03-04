# Engine架构修复：EventPoolWithPath方案

## 1. 背景与问题

### 1.1 问题描述

当前架构中，Extension的Capability Handler需要访问文件系统来处理heavyweight内容（如图片、视频、代码文件等），但无法获取临时目录（temp_dir）路径。

**根本原因**：
- Handler签名固定：`fn(&Command, Option<&Block>) -> CapResult<Vec<Event>>`
- 无额外参数传递runtime context
- Block.contents只能存储可持久化的数据，不能存储每次变化的temp_dir路径
- Engine持有的`SqlitePool`是抽象层，不暴露数据库文件路径

### 1.2 影响范围

需要文件系统访问的Block类型：
- **Directory Block**: 浏览和管理文件结构
- **Image/Video Block**: 保存二进制媒体文件
- **Code Workspace Block**: 管理源代码文件
- **Terminal Block**: 在沙箱环境中执行命令
- **Database Block**: 独立的SQLite数据库文件

### 1.3 临时目录的特性

```
temp_dir/                         # 每次打开路径都不同（如 /tmp/xyz789/）
├── events.db                     # 事件日志数据库
├── block-{uuid1}/                # Block1的专属目录
│   ├── image.png
│   └── data.json
├── block-{uuid2}/                # Block2的专属目录
│   ├── src/
│   │   └── main.rs
│   └── Cargo.toml
└── block-{uuid3}/                # Block3的专属目录
    └── notes.md
```

**关键特性**：
- ✅ temp_dir路径每次打开elf文件时都会变化（由`tempfile::TempDir`生成）
- ✅ 不能持久化到events.db中
- ✅ 必须在runtime时动态注入到Handler

---

## 2. 解决方案：EventPoolWithPath

### 2.1 核心思想

在创建`SqlitePool`时就记录数据库文件路径，封装为`EventPoolWithPath`结构体，沿着调用链传递给Engine，Engine在调用Handler前注入**block专属目录**（`_block_dir`）到Block。

**关键设计原则**：
> **注入Block专属目录而非整个temp_dir，确保Handler只能访问自己的文件，符合最小权限原则。**
>
> **跨Block操作通过Capability调用实现，而非直接文件系统访问。**

### 2.2 方案架构

```
┌─────────────────────────────────────────────────────────────┐
│  1. EventStore::create(db_path)                             │
│     └─ 返回 EventPoolWithPath { pool, db_path }           │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  2. archive.event_pool()                                    │
│     └─ 返回 EventPoolWithPath                              │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  3. commands/file.rs (create_file/open_file)                │
│     └─ 接收 EventPoolWithPath                              │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  4. manager.spawn_engine(file_id, event_pool_with_path)    │
│     └─ 传递 EventPoolWithPath                              │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  5. actor.spawn(file_id, event_pool_with_path)             │
│     └─ 传递 EventPoolWithPath                              │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  6. ElfileEngineActor                                       │
│     └─ 持有 event_pool_with_path                           │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  7. process_command()                                       │
│     ├─ temp_dir = event_pool_with_path.db_path.parent()   │
│     ├─ block_dir = temp_dir/block-{block_id}/             │
│     ├─ 注入 block.contents["_block_dir"] = block_dir      │
│     └─ 调用 handler(&cmd, &block)                         │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  8. Capability Handler                                      │
│     ├─ 读取 block.contents["_block_dir"]                  │
│     ├─ 直接使用 block_dir（不需手动计算）                 │
│     └─ 只操作自己的目录，不访问其他block                   │
└─────────────────────────────────────────────────────────────┘
```

### 2.3 优势

- ✅ **最小修改**：只是参数类型变化（`SqlitePool` → `EventPoolWithPath`）
- ✅ **零开销**：在创建时记录路径，后续直接使用，不需要运行时查询数据库
- ✅ **类型安全**：编译时即可确保db_path存在
- ✅ **向后兼容**：不影响现有Event结构，temp_dir不会持久化
- ✅ **透明传递**：Handler无需感知传递机制，只需读取`_temp_dir`字段

---

## 3. 架构变更

### 3.1 新增数据结构

**位置**：`src-tauri/src/engine/event_store.rs`

```rust
use std::path::PathBuf;
use sqlx::SqlitePool;

/// Event pool with database file path
///
/// This structure wraps SqlitePool with the database file path,
/// allowing the engine to derive temp_dir at runtime.
pub struct EventPoolWithPath {
    /// SQLite connection pool for event storage
    pub pool: SqlitePool,

    /// Path to the events.db file (e.g., /tmp/xyz789/events.db)
    pub db_path: PathBuf,
}
```

### 3.2 修改清单

| 文件 | 修改内容 | 说明 |
|------|----------|------|
| `src/engine/event_store.rs` | 定义`EventPoolWithPath`结构体 | 新增 |
| `src/engine/event_store.rs` | `EventStore::create`返回类型改为`EventPoolWithPath` | 修改返回值 |
| `src/elf/archive.rs` | `event_pool()`返回类型改为`EventPoolWithPath` | 修改返回值 |
| `src/commands/file.rs` | `create_file`/`open_file`接收`EventPoolWithPath` | 修改变量类型 |
| `src/engine/manager.rs` | `spawn_engine`参数类型改为`EventPoolWithPath` | 修改参数 |
| `src/engine/actor.rs` | `ElfileEngineActor`字段类型改为`EventPoolWithPath` | 修改字段 |
| `src/engine/actor.rs` | `spawn`参数类型改为`EventPoolWithPath` | 修改参数 |
| `src/engine/actor.rs` | `process_command`注入`_temp_dir`到Block | 新增逻辑 |
| `src/engine/actor.rs` | 所有`self.event_pool`改为`self.event_pool_with_path.pool` | 批量替换 |

---

## 4. 实施细节

### 4.1 EventStore修改

**文件**：`src-tauri/src/engine/event_store.rs`

#### 4.1.1 定义新结构体

```rust
use std::path::PathBuf;

/// Event pool with database file path
pub struct EventPoolWithPath {
    pub pool: SqlitePool,
    pub db_path: PathBuf,
}
```

#### 4.1.2 修改create方法

```rust
impl EventStore {
    pub async fn create(path: &str) -> Result<EventPoolWithPath, sqlx::Error> {
        let connection_string = if path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            // Ensure parent directory exists
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent).map_err(sqlx::Error::Io)?;
            }
            format!("sqlite://{}", path)
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::from_str(&connection_string)?
                    .create_if_missing(true),
            )
            .await?;

        // Initialize schema
        Self::init_schema(&pool).await?;

        // Return both pool and path
        Ok(EventPoolWithPath {
            pool,
            db_path: PathBuf::from(path),
        })
    }
}
```

**关键变化**：
- 返回类型从`Result<SqlitePool, ...>`改为`Result<EventPoolWithPath, ...>`
- 在返回时封装pool和db_path

---

### 4.2 Archive修改

**文件**：`src-tauri/src/elf/archive.rs`

```rust
use crate::engine::EventPoolWithPath; // 添加import

impl ElfArchive {
    pub async fn event_pool(&self) -> Result<EventPoolWithPath, sqlx::Error> {
        EventStore::create(self.db_path.to_str().unwrap()).await
    }
}
```

**关键变化**：
- 返回类型从`Result<SqlitePool, ...>`改为`Result<EventPoolWithPath, ...>`
- 无需修改内部逻辑

---

### 4.3 Engine Actor修改

**文件**：`src-tauri/src/engine/actor.rs`

#### 4.3.1 修改结构体字段

```rust
use crate::engine::EventPoolWithPath;

pub struct ElfileEngineActor {
    file_id: String,
    event_pool_with_path: EventPoolWithPath,  // 改这里
    state: StateProjector,
    registry: CapabilityRegistry,
    mailbox: mpsc::UnboundedReceiver<EngineMessage>,
}
```

#### 4.3.2 修改spawn方法

```rust
pub fn spawn(
    file_id: String,
    event_pool_with_path: EventPoolWithPath,  // 改参数类型
) -> (ElfileEngineHandle, JoinHandle<()>) {
    let (tx, rx) = mpsc::unbounded_channel();

    let mut actor = ElfileEngineActor {
        file_id: file_id.clone(),
        event_pool_with_path,  // 改字段名
        state: StateProjector::new(),
        registry: CapabilityRegistry::new(),
        mailbox: rx,
    };

    let handle = tokio::spawn(async move {
        actor.run().await;
    });

    (ElfileEngineHandle { tx }, handle)
}
```

#### 4.3.3 修改process_command注入_block_dir

```rust
async fn process_command(&mut self, cmd: Command) -> Result<Vec<Event>> {
    // 1. 获取handler
    let handler = self.registry.get(&cmd.cap_id)?;

    // 2. 获取block
    let mut block_opt = if cmd.cap_id == "core.create" || cmd.cap_id == "editor.create" {
        None
    } else {
        Some(
            self.state.get_block(&cmd.block_id)
                .ok_or_else(|| format!("Block not found: {}", cmd.block_id))?
                .clone()  // 克隆以便修改
        )
    };

    // 3. 注入 _block_dir（新增逻辑）
    if let Some(ref mut block) = block_opt {
        let temp_dir = self.event_pool_with_path.db_path
            .parent()
            .ok_or("Invalid db_path: no parent directory")?;

        // 关键：注入block专属目录，而非整个temp_dir
        let block_dir = temp_dir.join(format!("block-{}", block.block_id));

        // 确保block目录存在
        std::fs::create_dir_all(&block_dir)
            .map_err(|e| format!("Failed to create block directory: {}", e))?;

        // 注入到contents（临时的，不会持久化）
        if let Some(obj) = block.contents.as_object_mut() {
            obj.insert("_block_dir".to_string(), serde_json::json!(block_dir.to_string_lossy()));
        }
    }

    // 4. Authorization check
    let editor = self.state.get_editor(&cmd.editor_id)
        .ok_or_else(|| format!("Editor not found: {}", cmd.editor_id))?;

    if !handler.authorize(&cmd, block_opt.as_ref(), &editor, &self.state.grants)? {
        return Err(format!("Authorization failed for {}", cmd.cap_id).into());
    }

    // 5. Execute handler（block现在包含_block_dir）
    let mut events = handler.handler(&cmd, block_opt.as_ref())?;

    // 6. 特殊处理：core.create时注入_block_dir到新block
    if cmd.cap_id == "core.create" {
        let temp_dir = self.event_pool_with_path.db_path.parent()?;

        for event in &mut events {
            if event.attribute.ends_with("/core.create") {
                // 注入新block的专属目录
                if let Some(contents) = event.value.get_mut("contents") {
                    if let Some(obj) = contents.as_object_mut() {
                        let block_dir = temp_dir.join(format!("block-{}", event.entity));
                        std::fs::create_dir_all(&block_dir)?;
                        obj.insert("_block_dir".to_string(), json!(block_dir.to_string_lossy()));
                    }
                }
            }
        }
    }

    // ... 后续vector clock, conflict detection, persist等逻辑不变
}
```

**关键变化**：
- 在调用handler前，从`db_path.parent()`获取temp_dir，再计算block专属目录
- 临时注入到`block.contents["_block_dir"]`（只给block自己的目录）
- 创建block目录（如果不存在）
- core.create时特殊处理：注入新block的_block_dir
- Handler返回的events不包含`_block_dir`（因为是临时注入的runtime信息）

#### 4.3.4 批量替换event_pool引用

所有使用`self.event_pool`的地方改为`self.event_pool_with_path.pool`：

```rust
// 原来
EventStore::append_events(&self.event_pool, &events).await?;
EventStore::get_all_events(&self.event_pool).await?;

// 改为
EventStore::append_events(&self.event_pool_with_path.pool, &events).await?;
EventStore::get_all_events(&self.event_pool_with_path.pool).await?;
```

---

### 4.4 Engine Manager修改

**文件**：`src-tauri/src/engine/manager.rs`

```rust
use crate::engine::EventPoolWithPath;

impl EngineManager {
    pub async fn spawn_engine(
        &self,
        file_id: String,
        event_pool_with_path: EventPoolWithPath,  // 改参数类型
    ) -> Result<(), String> {
        if self.engines.contains_key(&file_id) {
            return Err(format!("Engine for file {} already exists", file_id));
        }

        let (handle, join_handle) = ElfileEngineActor::spawn(
            file_id.clone(),
            event_pool_with_path  // 传递新类型
        );

        self.engines.insert(
            file_id.clone(),
            EngineInstance { handle, join_handle }
        );
        Ok(())
    }
}
```

---

### 4.5 Commands修改

**文件**：`src-tauri/src/commands/file.rs`

```rust
// create_file
pub async fn create_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let file_id = format!("file-{}", uuid::Uuid::new_v4());
    let archive = ElfArchive::new().await?;
    archive.save(Path::new(&path))?;

    // 改这里：接收EventPoolWithPath
    let event_pool_with_path = archive.event_pool().await?;

    // 改这里：传递EventPoolWithPath
    state.engine_manager
        .spawn_engine(file_id.clone(), event_pool_with_path)
        .await?;

    state.files.insert(
        file_id.clone(),
        FileInfo {
            archive: Arc::new(archive),
            path: PathBuf::from(&path),
        },
    );

    bootstrap_editors(&file_id, &state).await?;
    Ok(file_id)
}

// open_file
pub async fn open_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let file_id = format!("file-{}", uuid::Uuid::new_v4());
    let archive = ElfArchive::open(Path::new(&path))?;

    // 改这里：接收EventPoolWithPath
    let event_pool_with_path = archive.event_pool().await?;

    // 改这里：传递EventPoolWithPath
    state.engine_manager
        .spawn_engine(file_id.clone(), event_pool_with_path)
        .await?;

    state.files.insert(
        file_id.clone(),
        FileInfo {
            archive: Arc::new(archive),
            path: PathBuf::from(&path),
        },
    );

    bootstrap_editors(&file_id, &state).await?;
    Ok(file_id)
}
```

---

### 4.6 Archive递归保存/解压（关键修复）

**问题描述**：当前实现只保存/解压`events.db`，**所有block目录下的文件都会丢失**！

**文件**：`src-tauri/src/elf/archive.rs`

#### 4.6.1 当前问题

```rust
// ❌ 当前save()只保存events.db
pub fn save(&self, elf_path: &Path) -> std::io::Result<()> {
    // ...
    zip.start_file("events.db", options)?;
    // 只保存数据库，block-{uuid}/目录被忽略
    // ...
}

// ❌ 当前open()只解压events.db
pub fn open(elf_path: &Path) -> std::io::Result<Self> {
    // ...
    let mut db_file = archive.by_name("events.db")?;
    // 只解压数据库，block-{uuid}/目录丢失
    // ...
}
```

**影响**：
```
用户操作流程：
1. 创建directory block，在_block_dir下创建file.txt
   → temp_dir/block-{id}/file.txt 创建成功
2. 保存elf
   → ❌ 只保存events.db，file.txt丢失
3. 重新打开elf
   → ❌ 只解压events.db，block-{id}/目录是空的
4. directory.list调用
   → ❌ 报错"文件不存在"

结论：所有heavyweight内容都无法持久化！
```

#### 4.6.2 添加walkdir依赖

**文件**：`src-tauri/Cargo.toml`

```toml
[dependencies]
# ... 现有依赖
walkdir = "2"  # 用于递归遍历目录
```

#### 4.6.3 修改save()方法

```rust
use walkdir::WalkDir;

impl ElfArchive {
    pub fn save(&self, elf_path: &Path) -> std::io::Result<()> {
        let file = File::create(elf_path)?;
        let mut zip = ZipWriter::new(file);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let temp_path = self.temp_dir.path();

        // 递归遍历temp_dir，保存所有文件
        for entry in WalkDir::new(temp_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // 只处理文件，跳过目录
            if !path.is_file() {
                continue;
            }

            // 计算相对路径（相对于temp_path）
            let relative_path = path.strip_prefix(temp_path)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            // 转换为字符串路径（zip内部路径）
            let zip_path = relative_path.to_string_lossy();

            // 添加文件到zip
            zip.start_file(zip_path.as_ref(), options)?;
            let mut file = File::open(path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        }

        zip.finish()?;
        Ok(())
    }
}
```

**关键变化**：
- 使用`WalkDir::new(temp_path)`递归遍历整个temp_dir
- 保存所有文件（包括`events.db`和所有`block-{uuid}/`下的文件）
- 使用相对路径存储，确保可移植性

#### 4.6.4 修改open()方法

```rust
impl ElfArchive {
    pub fn open(elf_path: &Path) -> std::io::Result<Self> {
        let file = File::open(elf_path)?;
        let mut archive = ZipArchive::new(file)?;

        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("events.db");

        // 解压所有文件
        for i in 0..archive.len() {
            let mut zip_file = archive.by_index(i)?;
            let outpath = temp_dir.path().join(zip_file.name());

            // 创建父目录（如果需要）
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // 解压文件
            if zip_file.is_file() {
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut zip_file, &mut outfile)?;
            } else if zip_file.is_dir() {
                // 创建目录
                std::fs::create_dir_all(&outpath)?;
            }
        }

        Ok(Self { temp_dir, db_path })
    }
}
```

**关键变化**：
- 遍历zip中的所有条目（`archive.len()`）
- 自动创建必要的父目录
- 解压所有文件和目录，保持结构

#### 4.6.5 测试用例

**文件**：`src-tauri/src/elf/archive.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_save_and_open_with_block_files() {
        // 1. 创建archive
        let archive = ElfArchive::new().await.unwrap();
        let temp_path = archive.temp_path();

        // 2. 创建block目录和文件
        let block_dir = temp_path.join("block-test-123");
        fs::create_dir_all(&block_dir).unwrap();
        fs::write(block_dir.join("file1.txt"), "Hello").unwrap();
        fs::write(block_dir.join("file2.md"), "# World").unwrap();

        // 创建子目录
        let sub_dir = block_dir.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(sub_dir.join("nested.json"), r#"{"key": "value"}"#).unwrap();

        // 3. 添加一些events到数据库
        let pool = archive.event_pool().await.unwrap();
        let events = vec![Event::new(
            "block-test-123".to_string(),
            "test".to_string(),
            serde_json::json!({"data": "test"}),
            std::collections::HashMap::new(),
        )];
        EventStore::append_events(&pool.pool, &events).await.unwrap();

        // 4. 保存到elf文件
        let temp_elf = tempfile::NamedTempFile::new().unwrap();
        archive.save(temp_elf.path()).unwrap();

        // 5. 重新打开elf文件
        let opened = ElfArchive::open(temp_elf.path()).unwrap();
        let opened_temp = opened.temp_path();

        // 6. 验证文件都存在
        let opened_block_dir = opened_temp.join("block-test-123");
        assert!(opened_block_dir.exists(), "Block directory should exist");
        assert!(opened_block_dir.join("file1.txt").exists(), "file1.txt should exist");
        assert!(opened_block_dir.join("file2.md").exists(), "file2.md should exist");
        assert!(opened_block_dir.join("subdir/nested.json").exists(), "nested.json should exist");

        // 7. 验证文件内容
        let content1 = fs::read_to_string(opened_block_dir.join("file1.txt")).unwrap();
        assert_eq!(content1, "Hello");

        let content2 = fs::read_to_string(opened_block_dir.join("file2.md")).unwrap();
        assert_eq!(content2, "# World");

        let content3 = fs::read_to_string(opened_block_dir.join("subdir/nested.json")).unwrap();
        assert_eq!(content3, r#"{"key": "value"}"#);

        // 8. 验证数据库也正常
        let opened_pool = opened.event_pool().await.unwrap();
        let retrieved_events = EventStore::get_all_events(&opened_pool.pool).await.unwrap();
        assert_eq!(retrieved_events.len(), 1);
        assert_eq!(retrieved_events[0].entity, "block-test-123");
    }

    #[tokio::test]
    async fn test_multiple_blocks_save_and_open() {
        let archive = ElfArchive::new().await.unwrap();
        let temp_path = archive.temp_path();

        // 创建多个block目录
        for i in 1..=3 {
            let block_dir = temp_path.join(format!("block-{}", i));
            fs::create_dir_all(&block_dir).unwrap();
            fs::write(block_dir.join("data.txt"), format!("Block {} data", i)).unwrap();
        }

        // 保存并重新打开
        let temp_elf = tempfile::NamedTempFile::new().unwrap();
        archive.save(temp_elf.path()).unwrap();
        let opened = ElfArchive::open(temp_elf.path()).unwrap();

        // 验证所有block目录都存在
        for i in 1..=3 {
            let block_dir = opened.temp_path().join(format!("block-{}", i));
            assert!(block_dir.exists());
            let content = fs::read_to_string(block_dir.join("data.txt")).unwrap();
            assert_eq!(content, format!("Block {} data", i));
        }
    }

    #[tokio::test]
    async fn test_empty_blocks_directory() {
        // 测试空的block目录也能正确处理
        let archive = ElfArchive::new().await.unwrap();
        let temp_path = archive.temp_path();

        // 创建空目录
        let block_dir = temp_path.join("block-empty");
        fs::create_dir_all(&block_dir).unwrap();

        let temp_elf = tempfile::NamedTempFile::new().unwrap();
        archive.save(temp_elf.path()).unwrap();
        let opened = ElfArchive::open(temp_elf.path()).unwrap();

        // 空目录可能不会被保存（这是正常的，因为只保存文件）
        // 但重新创建时会自动创建目录
    }
}
```

---

## 5. 测试计划

### 5.1 单元测试

#### 5.1.1 EventStore测试

**文件**：`src-tauri/src/engine/event_store.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_pool_with_path_creation() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();

        let result = EventStore::create(db_path).await.unwrap();

        // 验证pool可用
        assert!(result.pool.is_closed() == false);

        // 验证db_path正确
        assert_eq!(result.db_path.to_str().unwrap(), db_path);

        // 验证可以从db_path推导temp_dir
        let temp_dir = result.db_path.parent().unwrap();
        assert!(temp_dir.exists());
    }

    #[tokio::test]
    async fn test_temp_dir_derivation() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("events.db");

        let result = EventStore::create(db_path.to_str().unwrap()).await.unwrap();

        // 验证可以推导回temp_dir
        let derived_temp_dir = result.db_path.parent().unwrap();
        assert_eq!(derived_temp_dir, temp_dir.path());
    }
}
```

#### 5.1.2 Engine注入测试

**文件**：`src-tauri/src/engine/actor.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_temp_dir_injection() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("events.db");
        let event_pool_with_path = EventStore::create(db_path.to_str().unwrap())
            .await
            .unwrap();

        let (handle, _) = ElfileEngineActor::spawn(
            "test-file".to_string(),
            event_pool_with_path,
        );

        // 创建一个block
        let create_cmd = Command::new(
            "system".to_string(),
            "core.create".to_string(),
            "test-block".to_string(),
            serde_json::json!({
                "block_type": "test",
                "name": "Test Block",
            }),
        );

        let events = handle.process_command(create_cmd).await.unwrap();
        assert!(!events.is_empty());

        // 获取block并验证_temp_dir注入（需要通过其他capability测试）
        // 注意：core.create不会接收block，所以需要后续命令测试
    }

    #[tokio::test]
    async fn test_temp_dir_in_handler() {
        // 创建测试block
        // 执行需要temp_dir的capability
        // 验证handler能读取到_temp_dir
        // 验证_temp_dir值正确
    }
}
```

### 5.2 集成测试

#### 5.2.1 完整流程测试

**文件**：`src-tauri/tests/integration/temp_dir_flow.rs`

```rust
#[tokio::test]
async fn test_full_temp_dir_flow() {
    // 1. 创建elf文件
    let temp_elf = tempfile::NamedTempFile::new().unwrap();
    let archive = ElfArchive::new().await.unwrap();
    archive.save(temp_elf.path()).unwrap();

    // 2. 打开elf文件（解压到新temp_dir）
    let archive2 = ElfArchive::open(temp_elf.path()).unwrap();
    let event_pool_with_path = archive2.event_pool().await.unwrap();

    // 3. 验证temp_dir路径不同（每次打开都是新的临时目录）
    let temp_dir1 = archive.temp_path();
    let temp_dir2 = archive2.temp_path();
    assert_ne!(temp_dir1, temp_dir2);

    // 4. 验证db_path正确
    assert!(event_pool_with_path.db_path.ends_with("events.db"));

    // 5. 验证可以从db_path推导temp_dir
    let derived_temp_dir = event_pool_with_path.db_path.parent().unwrap();
    assert_eq!(derived_temp_dir, temp_dir2);
}
```

#### 5.2.2 Engine启动测试

**文件**：`src-tauri/tests/integration/engine_spawn.rs`

```rust
#[tokio::test]
async fn test_engine_spawn_with_path() {
    let manager = EngineManager::new();
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("events.db");

    let event_pool_with_path = EventStore::create(db_path.to_str().unwrap())
        .await
        .unwrap();

    // Spawn engine
    manager.spawn_engine("test-file".to_string(), event_pool_with_path)
        .await
        .unwrap();

    // 验证engine正常运行
    let handle = manager.get_engine("test-file").unwrap();

    // 创建block并验证可以正常操作
    let cmd = Command::new(
        "system".to_string(),
        "core.create".to_string(),
        uuid::Uuid::new_v4().to_string(),
        serde_json::json!({
            "block_type": "test",
            "name": "Test",
        }),
    );

    let events = handle.process_command(cmd).await.unwrap();
    assert!(!events.is_empty());
}
```

### 5.3 回归测试

确保现有功能不受影响：

- ✅ 所有现有的EventStore测试
- ✅ 所有现有的Engine测试
- ✅ 所有现有的Capability测试
- ✅ Frontend-Backend集成测试

---

## 6. Extension使用指南

### 6.1 访问block专属目录

Extension的Capability Handler可以通过`block.contents["_block_dir"]`访问Block专属目录：

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;
use std::fs;
use std::path::Path;

#[capability(id = "example.write_file", target = "example")]
pub fn handle_write_file(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required")?;
    let payload: ExamplePayload = serde_json::from_value(cmd.payload.clone())?;

    // 1. 从Engine注入的字段读取block专属目录
    let block_dir = block.contents
        .get("_block_dir")
        .and_then(|v| v.as_str())
        .ok_or("Missing _block_dir (should be injected by Engine)")?;

    // 2. 直接操作文件（相对于block_dir）
    // 注意：不需要手动计算block_dir，Engine已经提供了专属目录
    let file_path = Path::new(block_dir).join(&payload.filename);
    fs::write(&file_path, &payload.content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    // 3. 记录元数据到event（不包含_block_dir）
    let value = serde_json::json!({
        "filename": payload.filename,
        "size": payload.content.len(),
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(vec![create_event(
        block.block_id.clone(),
        "example.write_file",
        value,
        &cmd.editor_id,
        cmd.lamport_ts,
    )])
}
```

**关键点**：
- ✅ Engine注入`_block_dir`指向`temp_dir/block-{block_id}/`
- ✅ Handler直接使用，无需手动计算block专属目录
- ✅ 天然隔离，无法访问其他block的文件
- ✅ 符合最小权限原则

### 6.2 路径作用域与隔离

#### 6.2.1 Block专属目录

每个Block都有自己的专属目录：

```
temp_dir/
├── events.db
├── block-{uuid1}/          ← Block1的专属目录
│   ├── file1.txt
│   └── subdir/
│       └── file2.md
└── block-{uuid2}/          ← Block2的专属目录
    ├── image.png
    └── data.json
```

**默认行为**：
- 每个Block的Handler只能访问自己的`_block_dir`
- Engine只注入Block专属目录，不暴露整个temp_dir
- 这提供了**物理隔离**和**最小权限**，防止不同Block之间的文件冲突

#### 6.2.2 安全模型：Capability-Based设计

**核心设计原则**：
> **Handler只访问自己的`_block_dir`，跨Block操作通过Capability调用实现，而非直接文件访问。**

**错误的跨Block访问模式**（❌ 不要这样做）：
```rust
// ❌ 错误：试图获取其他block的_block_dir并直接访问文件
#[capability(id = "doc.embed_image_wrong", target = "document")]
fn handle_embed_image_wrong(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    // ❌ 试图获取其他block的路径（不可行，Engine只注入当前block的_block_dir）
    let target_block = /* 无法获取 */;
    let target_dir = target_block.contents.get("_block_dir")?;  // ❌ 无法做到

    // ❌ 试图直接读取其他block的文件
    let image_data = fs::read(Path::new(target_dir).join("image.png"))?;  // ❌ 绕过CBAC

    // 问题：
    // 1. 绕过了target block的访问控制
    // 2. 无审计日志
    // 3. 违反封装性
}
```

**正确的跨Block访问模式**（✅ 应该这样做）：
```rust
// ✅ 正确：记录引用关系，由前端协调Capability调用
#[capability(id = "doc.embed_image", target = "document")]
fn handle_embed_image(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required")?;
    let payload: EmbedImagePayload = serde_json::from_value(cmd.payload.clone())?;

    // 1. 验证link关系
    if !block.children
        .get("images")
        .map(|ids| ids.contains(&payload.image_block_id))
        .unwrap_or(false)
    {
        return Err("Image block is not linked".into());
    }

    // 2. 只记录引用关系，不直接访问文件
    //    实际的图片数据由前端通过调用image block的capability获取
    Ok(vec![create_event(
        block.block_id.clone(),
        "doc.embed_image",
        json!({
            "image_block_id": payload.image_block_id,
            "position": payload.position,
            "caption": payload.caption,
        }),
        &cmd.editor_id,
        cmd.lamport_ts,
    )])
}
```

**前端协调多个Capability调用**：
```typescript
// 场景：在文档中嵌入图片

// 步骤1：创建引用关系（doc block的capability）
await executeCommand({
    cap_id: "doc.embed_image",
    block_id: "doc1",
    payload: {
        image_block_id: "image2",
        position: { x: 100, y: 200 }
    }
});

// 步骤2：渲染时，调用image block的读取capability获取实际数据
const imageData = await executeCommand({
    cap_id: "image.read",
    block_id: "image2",  // 直接调用image2的capability
    payload: {}
});

// image2的handler会：
// - 检查调用者是否有image.read权限（CBAC检查）
// - 从自己的_block_dir读取image.png
// - 返回图片数据
// - 记录访问日志到events
```

**优势对比**：

| 方面 | 直接文件访问（❌） | Capability调用（✅） |
|------|-------------------|---------------------|
| **权限检查** | 绕过CBAC | 每次调用都检查 |
| **审计日志** | 无记录 | 完整的Event日志 |
| **封装性** | 暴露内部实现 | Block控制访问策略 |
| **灵活性** | 硬编码文件路径 | 可改变存储方式 |
| **安全性** | 可能越权访问 | 最小权限原则 |

#### 6.2.3 典型Extension的路径范围

| Extension | 默认路径范围 | 跨Block访问 |
|-----------|-------------|-------------|
| **Directory** | `_block_dir`（自己的block目录） | 通过link关系+Capability调用 |
| **Terminal** | `_block_dir`作为工作目录 | 沙箱隔离，通过Capability调用 |
| **Code Workspace** | `_block_dir/src/` | 只能访问自己的源码目录 |
| **Image Block** | `_block_dir/image.png` | 通过`image.read` capability提供数据 |
| **Database Block** | `_block_dir/data.db` | 独立数据库，通过SQL capability |

### 6.3 最佳实践

1. **总是验证_block_dir存在**：
   ```rust
   let block_dir = block.contents.get("_block_dir")
       .and_then(|v| v.as_str())
       .ok_or("Missing _block_dir (should be injected by Engine)")?;
   ```

2. **直接使用block_dir，无需手动计算**：
   ```rust
   // ✅ 正确：直接使用Engine注入的_block_dir
   let block_dir = Path::new(block.contents.get("_block_dir")?.as_str()?);
   let file_path = block_dir.join(&payload.path);

   // ❌ 错误：手动计算（不需要，且无法获取temp_dir）
   let temp_dir = /* 无法获取 */;
   let block_dir = Path::new(temp_dir).join(format!("block-{}", block.block_id));
   ```

3. **相对路径操作**：
   ```rust
   // ✅ 好：相对于block_dir的路径
   let file_path = block_dir.join(&payload.relative_path);

   // ❌ 差：绝对路径或逃逸路径
   let file_path = Path::new(&payload.absolute_path);
   ```

4. **路径安全检查**：
   ```rust
   // 防止路径遍历攻击（../ 逃逸）
   if payload.path.contains("..") {
       return Err("Path cannot contain '..'".into());
   }

   let full_path = block_dir.join(&payload.path);

   // 验证路径仍在block_dir内
   if !full_path.starts_with(&block_dir) {
       return Err("Path escapes block directory".into());
   }
   ```

5. **元数据记录**：
   ```rust
   // 只记录相对路径和元数据，不记录绝对路径
   let value = serde_json::json!({
       "files": [
           { "path": "file1.txt", "size": 1024 },
           { "path": "subdir/file2.md", "size": 512 }
       ],
       "updated_at": chrono::Utc::now().to_rfc3339(),
   });
   ```

---

## 7. 总结

### 7.1 核心变更

- ✅ 创建`EventPoolWithPath`封装pool和db_path
- ✅ 沿着调用链传递`EventPoolWithPath`
- ✅ Engine在`process_command`时注入`_block_dir`到Block（block专属目录）
- ✅ Handler从`block.contents["_block_dir"]`读取并直接使用
- ✅ 跨Block操作通过Capability调用，而非直接文件访问

### 7.2 设计原则

1. **最小侵入**：只改变参数类型，逻辑不变
2. **零开销**：在创建时记录路径，后续直接使用
3. **类型安全**：编译时保证db_path存在
4. **向后兼容**：不影响Event结构，`_block_dir`不持久化
5. **最小权限**：只注入block专属目录，不暴露整个temp_dir
6. **Capability-Based**：跨Block操作通过Capability调用，符合CBAC设计
7. **安全隔离**：物理隔离+权限检查双重保护

### 7.3 后续工作

1. **实施修改**：按照本文档逐步修改代码
2. **编写测试**：完成单元测试和集成测试
3. **Archive增强**：实现递归保存/解压temp_dir的完整功能
4. **Extension迁移**：更新directory等extension使用`_block_dir`
5. **Generator模板**：更新`elfiee-ext-gen/template`中的handler示例
6. **文档更新**：更新Extension开发指南

### 7.4 Generator模板影响

**结论**：**Generator模板无需大幅修改**

**原因**：
- 我们只修改了Engine内部的参数传递（`SqlitePool` → `EventPoolWithPath`）
- Handler签名保持不变：`fn(&Command, Option<&Block>) -> CapResult<Vec<Event>>`
- Handler只需知道如何从`block.contents["_block_dir"]`读取路径

**建议的模板更新**：
```rust
// elfiee-ext-gen/template/capability_handler.rs
#[capability(id = "{{ext}}.{{cap}}", target = "{{type}}")]
pub fn handle_{{cap}}(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required")?;
    let payload: {{PayloadType}} = serde_json::from_value(cmd.payload.clone())?;

    // 如果需要文件系统访问，添加以下代码：
    // let block_dir = block.contents
    //     .get("_block_dir")
    //     .and_then(|v| v.as_str())
    //     .ok_or("Missing _block_dir")?;
    //
    // let file_path = Path::new(block_dir).join(&payload.path);
    // fs::write(&file_path, &payload.content)?;

    // 你的业务逻辑...

    Ok(vec![create_event(
        block.block_id.clone(),
        "{{ext}}.{{cap}}",
        json!({ /* 你的数据 */ }),
        &cmd.editor_id,
        cmd.lamport_ts,
    )])
}
```

---

**文档版本**：v2.0
**创建日期**：2025-01-13
**更新日期**：2025-01-13
**作者**：Claude Code
**状态**：待实施
**关键变更**：采用`_block_dir`方案，强化Capability-Based设计

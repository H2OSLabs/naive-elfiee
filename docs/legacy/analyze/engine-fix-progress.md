# Engine架构修复：实施进度跟踪

## 文档说明

本文档基于 `engine-fix.md` 的设计方案，提供详细的TDD实施计划。

**TDD原则**：
1. **红色阶段（Red）**：先编写失败的测试
2. **绿色阶段（Green）**：编写最小代码使测试通过
3. **重构阶段（Refactor）**：优化代码，保持测试通过

**状态标识**：
- `[ ]` 未开始
- `[进行中]` 正在进行
- `[✓]` 已完成
- `[阻塞]` 被阻塞

---

## 任务总览

| 任务 | 状态 | 预计工时 | 实际工时 | 依赖 |
|------|------|----------|----------|------|
| 任务1: EventPoolWithPath结构体 | [✓] | 2h | ~1.5h | 无 |
| 任务2: Archive递归保存/解压 | [✓] | 4h | ~2h | 无 |
| 任务3: Engine注入_block_dir | [✓] | 4h | ~1h | 任务1 |
| 任务4: Commands层传递参数 | [✓] | 2h | ~0.5h | 任务1, 任务3 |
| 任务5: 集成测试与验证 | [进行中] | 3h | - | 任务1-4 |
| **总计** | | **15h** | **~5h** | |

---

## 任务1: EventPoolWithPath结构体 `[✓]`

### 目标
创建`EventPoolWithPath`结构体，封装`SqlitePool`和`db_path`，沿着调用链传递。

### 依赖
- 无

### 涉及文件
- `src-tauri/src/engine/event_store.rs`
- `src-tauri/src/elf/archive.rs`

---

### 1.1 编写测试（Red） `[✓]`

**文件**: `src-tauri/src/engine/event_store.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_event_pool_with_path_creation() {
        // 测试：EventPoolWithPath应该同时包含pool和db_path
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();

        let result = EventStore::create(db_path).await.unwrap();

        // 验证pool可用
        assert!(!result.pool.is_closed());

        // 验证db_path正确
        assert_eq!(result.db_path.to_str().unwrap(), db_path);

        // 验证可以从db_path推导temp_dir
        let temp_dir = result.db_path.parent().unwrap();
        assert!(temp_dir.exists());
    }

    #[tokio::test]
    async fn test_temp_dir_derivation() {
        // 测试：应该能从db_path推导回temp_dir
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("events.db");

        let result = EventStore::create(db_path.to_str().unwrap()).await.unwrap();

        // 验证可以推导回temp_dir
        let derived_temp_dir = result.db_path.parent().unwrap();
        assert_eq!(derived_temp_dir, temp_dir.path());
    }

    #[tokio::test]
    async fn test_event_pool_with_path_memory_db() {
        // 测试：内存数据库应该使用特殊路径
        let result = EventStore::create(":memory:").await.unwrap();

        assert!(!result.pool.is_closed());
        assert_eq!(result.db_path.to_str().unwrap(), ":memory:");
    }
}
```

**预期结果**: ❌ 编译失败（`EventPoolWithPath`不存在）

**验证命令**:
```bash
cd src-tauri && cargo test test_event_pool_with_path --lib
```

---

### 1.2 实现结构体（Green） `[✓]`

**文件**: `src-tauri/src/engine/event_store.rs`

```rust
use std::path::PathBuf;
use sqlx::SqlitePool;

/// Event pool with database file path
///
/// This structure wraps SqlitePool with the database file path,
/// allowing the engine to derive temp_dir at runtime.
#[derive(Clone)]
pub struct EventPoolWithPath {
    /// SQLite connection pool for event storage
    pub pool: SqlitePool,

    /// Path to the events.db file (e.g., /tmp/xyz789/events.db)
    pub db_path: PathBuf,
}

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

**验证命令**:
```bash
cd src-tauri && cargo test test_event_pool_with_path --lib
```

**预期结果**: ✅ 所有测试通过

---

### 1.3 更新Archive.event_pool() `[✓]`

**文件**: `src-tauri/src/elf/archive.rs`

```rust
use crate::engine::EventPoolWithPath;

impl ElfArchive {
    pub async fn event_pool(&self) -> Result<EventPoolWithPath, sqlx::Error> {
        EventStore::create(self.db_path.to_str().unwrap()).await
    }
}
```

**测试更新**:
```rust
#[tokio::test]
async fn test_archive_returns_event_pool_with_path() {
    let archive = ElfArchive::new().await.unwrap();
    let event_pool_with_path = archive.event_pool().await.unwrap();

    // 验证返回的是EventPoolWithPath
    assert!(!event_pool_with_path.pool.is_closed());
    assert!(event_pool_with_path.db_path.ends_with("events.db"));
}
```

**验证命令**:
```bash
cd src-tauri && cargo test test_archive_returns_event_pool_with_path --lib
```

---

### 1.4 重构与代码审查 `[✓]`

**检查清单**:
- [x] 所有测试通过
- [x] 代码符合Rust最佳实践
- [x] 添加了必要的文档注释
- [x] 没有编译警告
- [x] 类型安全（编译时保证db_path存在）

**验证命令**:
```bash
cd src-tauri && cargo test && cargo clippy && cargo fmt --check
```

---

## 任务2: Archive递归保存/解压 `[ ]`

### 目标
修复`Archive::save()`和`Archive::open()`，支持递归保存/解压整个temp_dir，包括所有block目录。

### 依赖
- 无（独立任务）

### 涉及文件
- `src-tauri/Cargo.toml`
- `src-tauri/src/elf/archive.rs`

---

### 2.1 添加依赖 `[ ]`

**文件**: `src-tauri/Cargo.toml`

```toml
[dependencies]
walkdir = "2"
```

**验证命令**:
```bash
cd src-tauri && cargo check
```

---

### 2.2 编写测试（Red） `[ ]`

**文件**: `src-tauri/src/elf/archive.rs`

```rust
#[cfg(test)]
mod archive_recursive_tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_save_and_open_with_block_files() {
        // 测试：保存和重新打开应该保留所有block文件

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

        // 3. 添加events到数据库
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
        // 测试：多个block目录都应该被保存和恢复
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
    async fn test_deep_nested_directories() {
        // 测试：深层嵌套目录应该正确保存和恢复
        let archive = ElfArchive::new().await.unwrap();
        let temp_path = archive.temp_path();

        let deep_path = temp_path
            .join("block-deep")
            .join("level1")
            .join("level2")
            .join("level3");
        fs::create_dir_all(&deep_path).unwrap();
        fs::write(deep_path.join("deep.txt"), "deep content").unwrap();

        let temp_elf = tempfile::NamedTempFile::new().unwrap();
        archive.save(temp_elf.path()).unwrap();
        let opened = ElfArchive::open(temp_elf.path()).unwrap();

        let opened_deep = opened.temp_path()
            .join("block-deep")
            .join("level1")
            .join("level2")
            .join("level3")
            .join("deep.txt");
        assert!(opened_deep.exists());
        let content = fs::read_to_string(opened_deep).unwrap();
        assert_eq!(content, "deep content");
    }
}
```

**预期结果**: ❌ 测试失败（文件未被保存/解压）

**验证命令**:
```bash
cd src-tauri && cargo test archive_recursive_tests --lib
```

---

### 2.3 实现save()递归保存（Green） `[ ]`

**文件**: `src-tauri/src/elf/archive.rs`

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

**验证命令**:
```bash
cd src-tauri && cargo test test_save_and_open_with_block_files --lib
```

**预期结果**: 部分通过（save成功，但open失败）

---

### 2.4 实现open()递归解压（Green） `[ ]`

**文件**: `src-tauri/src/elf/archive.rs`

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

**验证命令**:
```bash
cd src-tauri && cargo test archive_recursive_tests --lib
```

**预期结果**: ✅ 所有测试通过

---

### 2.5 重构与优化 `[ ]`

**优化点**:
- [ ] 添加进度回调（可选，用于大文件）
- [ ] 处理符号链接（安全检查）
- [ ] 优化缓冲区大小

**检查清单**:
- [ ] 所有测试通过
- [ ] 处理边界情况（空目录、大文件等）
- [ ] 性能测试（大量文件场景）
- [ ] 错误处理完善

**验证命令**:
```bash
cd src-tauri && cargo test && cargo clippy
```

---

## 任务3: Engine注入_block_dir `[ ]`

### 目标
修改Engine的`process_command`方法，在调用handler前注入`_block_dir`到Block。

### 依赖
- 任务1（需要EventPoolWithPath）

### 涉及文件
- `src-tauri/src/engine/actor.rs`
- `src-tauri/src/engine/manager.rs`

---

### 3.1 编写测试（Red） `[ ]`

**文件**: `src-tauri/src/engine/actor.rs`

```rust
#[cfg(test)]
mod block_dir_injection_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_block_dir_injection_on_existing_block() {
        // 测试：process_command应该注入_block_dir到现有block
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("events.db");
        let event_pool_with_path = EventStore::create(db_path.to_str().unwrap())
            .await
            .unwrap();

        let (handle, _join) = ElfileEngineActor::spawn(
            "test-file".to_string(),
            event_pool_with_path,
        );

        // 创建一个block
        let create_cmd = Command::new(
            "system".to_string(),
            "core.create".to_string(),
            uuid::Uuid::new_v4().to_string(),
            serde_json::json!({
                "block_type": "test",
                "name": "Test Block",
            }),
        );

        let events = handle.process_command(create_cmd).await.unwrap();
        let block_id = events[0].entity.clone();

        // 创建一个测试capability来验证_block_dir
        // （需要先实现一个测试用的capability）
        let test_cmd = Command::new(
            "system".to_string(),
            "test.verify_block_dir".to_string(),
            block_id.clone(),
            serde_json::json!({}),
        );

        let result = handle.process_command(test_cmd).await;
        assert!(result.is_ok());

        // 验证_block_dir存在
        let block = handle.get_block(block_id).await.unwrap();
        assert!(block.contents.get("_block_dir").is_some());

        let block_dir_str = block.contents["_block_dir"].as_str().unwrap();
        let block_dir_path = Path::new(block_dir_str);

        // 验证路径格式正确
        assert!(block_dir_path.ends_with(format!("block-{}", block_id)));

        // 验证目录被创建
        assert!(block_dir_path.exists());
        assert!(block_dir_path.is_dir());
    }

    #[tokio::test]
    async fn test_block_dir_injection_on_core_create() {
        // 测试：core.create应该在event中注入_block_dir
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("events.db");
        let event_pool_with_path = EventStore::create(db_path.to_str().unwrap())
            .await
            .unwrap();

        let (handle, _join) = ElfileEngineActor::spawn(
            "test-file".to_string(),
            event_pool_with_path,
        );

        let create_cmd = Command::new(
            "system".to_string(),
            "core.create".to_string(),
            uuid::Uuid::new_v4().to_string(),
            serde_json::json!({
                "block_type": "test",
                "name": "Test Block",
            }),
        );

        let events = handle.process_command(create_cmd).await.unwrap();

        // 验证event中包含_block_dir
        let event = &events[0];
        assert!(event.value.get("contents").is_some());

        let contents = event.value["contents"].as_object().unwrap();
        assert!(contents.get("_block_dir").is_some());

        let block_dir = contents["_block_dir"].as_str().unwrap();
        assert!(Path::new(block_dir).exists());
    }
}
```

**预期结果**: ❌ 测试失败（_block_dir未注入）

**验证命令**:
```bash
cd src-tauri && cargo test block_dir_injection_tests --lib
```

---

### 3.2 修改Actor结构体（Green） `[ ]`

**文件**: `src-tauri/src/engine/actor.rs`

```rust
use crate::engine::EventPoolWithPath;

pub struct ElfileEngineActor {
    file_id: String,
    event_pool_with_path: EventPoolWithPath,  // 改这里
    state: StateProjector,
    registry: CapabilityRegistry,
    mailbox: mpsc::UnboundedReceiver<EngineMessage>,
}

pub fn spawn(
    file_id: String,
    event_pool_with_path: EventPoolWithPath,  // 改参数
) -> (ElfileEngineHandle, JoinHandle<()>) {
    let (tx, rx) = mpsc::unbounded_channel();

    let mut actor = ElfileEngineActor {
        file_id: file_id.clone(),
        event_pool_with_path,  // 改这里
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

---

### 3.3 修改process_command注入_block_dir（Green） `[ ]`

**文件**: `src-tauri/src/engine/actor.rs`

```rust
async fn process_command(&mut self, cmd: Command) -> Result<Vec<Event>, String> {
    let handler = self.registry.get(&cmd.cap_id)?;

    // 获取block
    let mut block_opt = if cmd.cap_id == "core.create" || cmd.cap_id == "editor.create" {
        None
    } else {
        Some(
            self.state.get_block(&cmd.block_id)
                .ok_or_else(|| format!("Block not found: {}", cmd.block_id))?
                .clone()
        )
    };

    // 注入 _block_dir（新增逻辑）
    if let Some(ref mut block) = block_opt {
        let temp_dir = self.event_pool_with_path.db_path
            .parent()
            .ok_or("Invalid db_path: no parent directory")?;

        // 关键：注入block专属目录
        let block_dir = temp_dir.join(format!("block-{}", block.block_id));

        // 确保block目录存在
        std::fs::create_dir_all(&block_dir)
            .map_err(|e| format!("Failed to create block directory: {}", e))?;

        // 注入到contents（临时的，不会持久化）
        if let Some(obj) = block.contents.as_object_mut() {
            obj.insert("_block_dir".to_string(), serde_json::json!(block_dir.to_string_lossy()));
        }
    }

    // Authorization check
    let editor = self.state.get_editor(&cmd.editor_id)
        .ok_or_else(|| format!("Editor not found: {}", cmd.editor_id))?;

    if !handler.authorize(&cmd, block_opt.as_ref(), &editor, &self.state.grants)? {
        return Err(format!("Authorization failed for {}", cmd.cap_id));
    }

    // Execute handler（block现在包含_block_dir）
    let mut events = handler.handler(&cmd, block_opt.as_ref())?;

    // 特殊处理：core.create时注入_block_dir到新block
    if cmd.cap_id == "core.create" {
        let temp_dir = self.event_pool_with_path.db_path.parent()
            .ok_or("Invalid db_path")?;

        for event in &mut events {
            if event.attribute.ends_with("/core.create") {
                // 注入新block的专属目录
                if let Some(contents) = event.value.get_mut("contents") {
                    if let Some(obj) = contents.as_object_mut() {
                        let block_dir = temp_dir.join(format!("block-{}", event.entity));
                        std::fs::create_dir_all(&block_dir)
                            .map_err(|e| format!("Failed to create block directory: {}", e))?;
                        obj.insert("_block_dir".to_string(), serde_json::json!(block_dir.to_string_lossy()));
                    }
                }
            }
        }
    }

    // ... 后续vector clock, persist等逻辑
    Ok(events)
}
```

---

### 3.4 批量替换event_pool引用 `[ ]`

在`actor.rs`中，将所有`self.event_pool`改为`self.event_pool_with_path.pool`：

```bash
# 查找所有使用event_pool的地方
grep -n "self.event_pool" src-tauri/src/engine/actor.rs

# 手动替换每个引用
```

**验证命令**:
```bash
cd src-tauri && cargo test block_dir_injection_tests --lib
```

**预期结果**: ✅ 所有测试通过

---

### 3.5 更新Manager `[ ]`

**文件**: `src-tauri/src/engine/manager.rs`

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

**验证命令**:
```bash
cd src-tauri && cargo test --lib
```

---

### 3.6 重构与代码审查 `[ ]`

**检查清单**:
- [ ] 所有测试通过
- [ ] _block_dir在runtime注入，不持久化
- [ ] block目录自动创建
- [ ] core.create特殊处理正确
- [ ] 没有引入性能问题

---

## 任务4: Commands层传递参数 `[ ]`

### 目标
更新`commands/file.rs`，传递`EventPoolWithPath`给Engine。

### 依赖
- 任务1（EventPoolWithPath）
- 任务3（Engine接受EventPoolWithPath）

### 涉及文件
- `src-tauri/src/commands/file.rs`

---

### 4.1 编写集成测试（Red） `[ ]`

**文件**: `src-tauri/tests/integration/file_commands.rs` (新建)

```rust
use elfiee::*;

#[tokio::test]
async fn test_create_file_with_block_dir() {
    let state = AppState::new();
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let path = temp_file.path().to_string_lossy().to_string();

    // 创建文件
    let file_id = commands::create_file(path.clone(), tauri::State::from(&state))
        .await
        .unwrap();

    // 创建block
    let block_id = uuid::Uuid::new_v4().to_string();
    let cmd = Command::new(
        "system".to_string(),
        "core.create".to_string(),
        block_id.clone(),
        serde_json::json!({
            "block_type": "test",
            "name": "Test",
        }),
    );

    commands::execute_command(file_id.clone(), cmd, tauri::State::from(&state))
        .await
        .unwrap();

    // 验证block有_block_dir
    let block = commands::get_block(file_id, block_id, tauri::State::from(&state))
        .await
        .unwrap();

    assert!(block.contents.get("_block_dir").is_some());
}
```

**预期结果**: ❌ 编译失败（类型不匹配）

---

### 4.2 更新create_file和open_file（Green） `[ ]`

**文件**: `src-tauri/src/commands/file.rs`

```rust
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

**验证命令**:
```bash
cd src-tauri && cargo test test_create_file_with_block_dir
```

**预期结果**: ✅ 测试通过

---

### 4.3 回归测试 `[ ]`

**验证所有现有tests**:
```bash
cd src-tauri && cargo test
```

**检查清单**:
- [ ] 所有现有测试通过
- [ ] 新测试通过
- [ ] 没有引入新的警告

---

## 任务5: 集成测试与验证 `[ ]`

### 目标
端到端测试，验证完整流程：创建文件 → 保存 → 重新打开 → 文件还在。

### 依赖
- 任务1-4全部完成

---

### 5.1 编写端到端测试 `[ ]`

**文件**: `src-tauri/tests/integration/end_to_end.rs` (新建)

```rust
use elfiee::*;
use std::fs;

#[tokio::test]
async fn test_end_to_end_block_file_persistence() {
    // 完整流程测试：创建 → 操作 → 保存 → 重新打开 → 验证

    let state = AppState::new();
    let temp_elf = tempfile::NamedTempFile::new().unwrap();
    let elf_path = temp_elf.path().to_string_lossy().to_string();

    // ========== 步骤1: 创建elf文件 ==========
    let file_id = commands::create_file(elf_path.clone(), tauri::State::from(&state))
        .await
        .unwrap();

    // ========== 步骤2: 创建directory block ==========
    let block_id = uuid::Uuid::new_v4().to_string();
    let create_cmd = Command::new(
        "system".to_string(),
        "core.create".to_string(),
        block_id.clone(),
        serde_json::json!({
            "block_type": "directory",
            "name": "Test Directory",
        }),
    );

    commands::execute_command(file_id.clone(), create_cmd, tauri::State::from(&state))
        .await
        .unwrap();

    // ========== 步骤3: 在block目录下创建文件 ==========
    // 获取block查看_block_dir
    let block = commands::get_block(file_id.clone(), block_id.clone(), tauri::State::from(&state))
        .await
        .unwrap();

    let block_dir = block.contents["_block_dir"].as_str().unwrap();

    // 在block目录创建测试文件
    fs::write(Path::new(block_dir).join("test-file.txt"), "Hello from test!").unwrap();
    fs::create_dir_all(Path::new(block_dir).join("subdir")).unwrap();
    fs::write(Path::new(block_dir).join("subdir/nested.md"), "# Nested").unwrap();

    // ========== 步骤4: 保存elf文件 ==========
    commands::save_file(file_id.clone(), tauri::State::from(&state))
        .await
        .unwrap();

    // ========== 步骤5: 关闭文件 ==========
    commands::close_file(file_id.clone(), tauri::State::from(&state))
        .await
        .unwrap();

    // ========== 步骤6: 重新打开elf文件 ==========
    let new_file_id = commands::open_file(elf_path.clone(), tauri::State::from(&state))
        .await
        .unwrap();

    // ========== 步骤7: 验证block还存在 ==========
    let reopened_block = commands::get_block(new_file_id.clone(), block_id.clone(), tauri::State::from(&state))
        .await
        .unwrap();

    assert_eq!(reopened_block.block_type, "directory");
    assert_eq!(reopened_block.name, "Test Directory");

    // ========== 步骤8: 验证文件还存在 ==========
    let reopened_block_dir = reopened_block.contents["_block_dir"].as_str().unwrap();
    let test_file = Path::new(reopened_block_dir).join("test-file.txt");
    let nested_file = Path::new(reopened_block_dir).join("subdir/nested.md");

    assert!(test_file.exists(), "test-file.txt should exist after reopen");
    assert!(nested_file.exists(), "subdir/nested.md should exist after reopen");

    // ========== 步骤9: 验证文件内容 ==========
    let content1 = fs::read_to_string(test_file).unwrap();
    assert_eq!(content1, "Hello from test!");

    let content2 = fs::read_to_string(nested_file).unwrap();
    assert_eq!(content2, "# Nested");

    println!("✅ 端到端测试通过：文件成功持久化并恢复");
}

#[tokio::test]
async fn test_multiple_blocks_persistence() {
    // 测试多个block的文件都能正确持久化
    let state = AppState::new();
    let temp_elf = tempfile::NamedTempFile::new().unwrap();
    let elf_path = temp_elf.path().to_string_lossy().to_string();

    let file_id = commands::create_file(elf_path.clone(), tauri::State::from(&state))
        .await
        .unwrap();

    // 创建3个block，每个都有文件
    let mut block_ids = Vec::new();
    for i in 1..=3 {
        let block_id = uuid::Uuid::new_v4().to_string();
        let create_cmd = Command::new(
            "system".to_string(),
            "core.create".to_string(),
            block_id.clone(),
            serde_json::json!({
                "block_type": "directory",
                "name": format!("Block {}", i),
            }),
        );

        commands::execute_command(file_id.clone(), create_cmd, tauri::State::from(&state))
            .await
            .unwrap();

        // 获取block并创建文件
        let block = commands::get_block(file_id.clone(), block_id.clone(), tauri::State::from(&state))
            .await
            .unwrap();

        let block_dir = block.contents["_block_dir"].as_str().unwrap();
        fs::write(
            Path::new(block_dir).join(format!("file{}.txt", i)),
            format!("Content {}", i)
        ).unwrap();

        block_ids.push(block_id);
    }

    // 保存、关闭、重新打开
    commands::save_file(file_id.clone(), tauri::State::from(&state)).await.unwrap();
    commands::close_file(file_id.clone(), tauri::State::from(&state)).await.unwrap();
    let new_file_id = commands::open_file(elf_path, tauri::State::from(&state)).await.unwrap();

    // 验证所有block的文件都存在
    for (i, block_id) in block_ids.iter().enumerate() {
        let block = commands::get_block(new_file_id.clone(), block_id.clone(), tauri::State::from(&state))
            .await
            .unwrap();

        let block_dir = block.contents["_block_dir"].as_str().unwrap();
        let file_path = Path::new(block_dir).join(format!("file{}.txt", i + 1));

        assert!(file_path.exists());
        let content = fs::read_to_string(file_path).unwrap();
        assert_eq!(content, format!("Content {}", i + 1));
    }

    println!("✅ 多block持久化测试通过");
}
```

**验证命令**:
```bash
cd src-tauri && cargo test end_to_end --test integration
```

---

### 5.2 性能测试（可选） `[ ]`

**文件**: `src-tauri/benches/archive_bench.rs` (新建)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_save_many_files(c: &mut Criterion) {
    c.bench_function("save 100 files", |b| {
        b.iter(|| {
            // 创建archive
            // 创建100个block，每个10个文件
            // 保存
        });
    });
}

criterion_group!(benches, bench_save_many_files);
criterion_main!(benches);
```

---

### 5.3 最终验证清单 `[ ]`

**功能验证**:
- [ ] ✅ EventPoolWithPath正确传递db_path
- [ ] ✅ Archive递归保存所有block目录
- [ ] ✅ Archive递归解压所有block目录
- [ ] ✅ Engine注入_block_dir到现有block
- [ ] ✅ core.create注入_block_dir到新block
- [ ] ✅ Commands层正确传递参数
- [ ] ✅ 端到端流程：创建→保存→打开→文件还在

**质量验证**:
- [ ] ✅ 所有单元测试通过
- [ ] ✅ 所有集成测试通过
- [ ] ✅ 无编译警告
- [ ] ✅ Clippy检查通过
- [ ] ✅ 代码格式化正确

**文档验证**:
- [ ] ✅ 代码注释完整
- [ ] ✅ engine-fix.md准确
- [ ] ✅ 本文档与实现同步

---

## 验证命令汇总

```bash
# 1. 运行所有测试
cd src-tauri && cargo test

# 2. 检查代码质量
cd src-tauri && cargo clippy -- -D warnings

# 3. 格式化代码
cd src-tauri && cargo fmt

# 4. 构建项目
cd src-tauri && cargo build

# 5. 运行Tauri开发服务器
pnpm tauri dev

# 6. 完整验证
cd src-tauri && cargo test && cargo clippy && cargo fmt --check && cargo build
```

---

## 进度跟踪

### 已完成 `[✓]`

（暂无）

### 进行中 `[进行中]`

（暂无）

### 待开始 `[ ]`

- 任务1: EventPoolWithPath结构体
- 任务2: Archive递归保存/解压
- 任务3: Engine注入_block_dir
- 任务4: Commands层传递参数
- 任务5: 集成测试与验证

### 阻塞 `[阻塞]`

（暂无）

---

## 备注

### TDD最佳实践

1. **先写测试**：明确期望行为
2. **最小实现**：只写通过测试的代码
3. **持续重构**：保持测试通过的同时优化代码
4. **快速反馈**：频繁运行测试
5. **小步前进**：每次改动都有测试覆盖

### 常见问题

**Q: 测试一直失败怎么办？**
A: 检查测试本身是否正确，是否有假设错误。

**Q: 如何处理依赖问题？**
A: 按照任务依赖顺序执行，确保前置任务完成。

**Q: 重构时测试挂了？**
A: 立即回滚，找到最后一个绿色状态，重新小步重构。

---

**文档版本**: v1.0
**创建日期**: 2025-01-13
**最后更新**: 2025-01-13
**状态**: 待开始

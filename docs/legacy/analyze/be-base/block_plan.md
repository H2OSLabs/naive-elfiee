# Block 元数据扩展开发计划

## 文档信息

- **文档版本**: 1.0
- **创建日期**: 2025-12-17
- **开发模式**: TDD（测试驱动开发）
- **预计工时**: 11.5 小时

---

## 📋 任务总结

**核心目标**：为 Block 添加 `metadata` 字段，支持存储描述（description）和时间戳（created_at、updated_at）。

**实现路径**：
1. 创建统一的时间戳工具（`utils/time.rs`）
2. 定义 `BlockMetadata` 结构（`models/metadata.rs`）
3. 修改 Block 模型添加 `metadata` 字段
4. 修改 `core.create` Capability，自动生成时间戳
5. 修改 `markdown.write` Capability，自动更新 `updated_at`
6. 修改 StateProjector，处理 metadata 字段
7. 更新类型绑定，前端获得 TypeScript 类型

**核心原则**：
- ✅ 所有修改通过 Event Sourcing（不存在内存直接修改）
- ✅ TDD 开发模式（测试先行）
- ✅ UTC + 时区的时间戳方案
- ✅ metadata 灵活扩展（JSON 格式）
- ✅ status 字段预留（注释中说明，不实现）

---

## 🔍 当前代码分析

### 1. Block 当前结构

```rust
// src-tauri/src/models/block.rs
pub struct Block {
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: serde_json::Value,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,
    // ❌ 缺少 metadata 字段
}
```

### 2. CreateBlockPayload 当前结构

```rust
// src-tauri/src/models/payloads.rs
pub struct CreateBlockPayload {
    pub name: String,
    pub block_type: String,
    // ❌ 缺少 metadata 字段
}
```

### 3. core.create Handler 当前逻辑

```rust
// src-tauri/src/capabilities/builtins/create.rs
let event = create_event(
    block_id.clone(),
    "core.create",
    serde_json::json!({
        "name": payload.name,
        "type": payload.block_type,
        "owner": cmd.editor_id,
        "contents": {},
        "children": {}
        // ❌ 缺少 "metadata"
    }),
    &cmd.editor_id,
    1,
);
```

### 4. markdown.write Handler 当前逻辑

```rust
// src-tauri/src/extensions/markdown/markdown_write.rs
let event = create_event(
    block.block_id.clone(),
    "markdown.write",
    serde_json::json!({ "contents": new_contents }),
    // ❌ 不更新 updated_at
    &cmd.editor_id,
    1,
);
```

### 5. StateProjector 当前逻辑

```rust
// src-tauri/src/engine/state.rs
"core.create" => {
    let block = Block {
        block_id: event.entity.clone(),
        name: obj.get("name")...,
        block_type: obj.get("type")...,
        owner: obj.get("owner")...,
        contents: obj.get("contents")...,
        children: obj.get("children")...,
        // ❌ 缺少 metadata
    };
    self.blocks.insert(block.block_id.clone(), block);
}
```

### 6. 时间戳使用情况

**已有使用**：
- `commands/file.rs` - FileMetadata 使用 chrono::DateTime::to_rfc3339_opts
- `Cargo.toml` - chrono 依赖已存在

**格式示例**：
```rust
chrono::DateTime::<chrono::Utc>::from(t)
    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
// 输出："2025-12-17T02:30:00Z"
```

---

## 📝 详细任务清单

### 阶段 1：基础设施（3.5h）

#### 任务 1.1：创建时间戳工具模块（1h）

**文件**：`src-tauri/src/utils/mod.rs`（新建）
**文件**：`src-tauri/src/utils/time.rs`（新建）

**实现内容**：

```rust
// src-tauri/src/utils/mod.rs
pub mod time;

// src-tauri/src/utils/time.rs
use chrono::{DateTime, Utc};

/// 生成当前 UTC 时间戳（ISO 8601 格式）
///
/// 格式："2025-12-17T02:30:00Z"
///
/// # 示例
/// ```
/// let timestamp = now_utc();
/// assert!(timestamp.ends_with('Z'));
/// ```
pub fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// 解析 ISO 8601 时间戳并转换为 UTC
///
/// # 参数
/// * `timestamp` - ISO 8601 格式的时间戳字符串
///
/// # 返回
/// * `Ok(DateTime<Utc>)` - 解析成功
/// * `Err(String)` - 解析失败，返回错误信息
///
/// # 示例
/// ```
/// let dt = parse_to_utc("2025-12-17T02:30:00Z").unwrap();
/// assert_eq!(dt.year(), 2025);
/// ```
pub fn parse_to_utc(timestamp: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("Invalid timestamp '{}': {}", timestamp, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_utc_format() {
        let timestamp = now_utc();

        // 应该以 Z 结尾（UTC）
        assert!(timestamp.ends_with('Z'));

        // 应该能够被解析
        let parsed = parse_to_utc(&timestamp);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_parse_to_utc_valid() {
        let timestamp = "2025-12-17T02:30:00Z";
        let result = parse_to_utc(timestamp);

        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 17);
    }

    #[test]
    fn test_parse_to_utc_with_timezone() {
        // 带时区的时间戳应该被转换为 UTC
        let timestamp = "2025-12-17T10:30:00+08:00"; // 北京时间
        let result = parse_to_utc(timestamp);

        assert!(result.is_ok());
        let dt = result.unwrap();
        // 转换为 UTC 后应该是 02:30:00
        assert_eq!(dt.hour(), 2);
    }

    #[test]
    fn test_parse_to_utc_invalid() {
        let result = parse_to_utc("invalid-timestamp");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid timestamp"));
    }

    #[test]
    fn test_roundtrip() {
        let original = now_utc();
        let parsed = parse_to_utc(&original).unwrap();
        let regenerated = parsed.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        assert_eq!(original, regenerated);
    }
}
```

**修改**：`src-tauri/src/lib.rs`

```rust
// 在文件顶部添加
mod utils;
```

**测试命令**：
```bash
cd src-tauri
cargo test utils::time::tests --lib
```

**验收标准**：
- ✅ 所有测试通过
- ✅ `now_utc()` 返回以 Z 结尾的 UTC 时间戳
- ✅ `parse_to_utc()` 正确解析各种格式
- ✅ 带时区的时间戳正确转换为 UTC

---

#### 任务 1.2：创建 BlockMetadata 模型（1h）

**文件**：`src-tauri/src/models/metadata.rs`（新建）

**实现内容**：

```rust
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

/// Block 元数据结构（推荐格式）
///
/// 存储在 Block.metadata 字段中（JSON 格式）。
/// 该结构定义了推荐的 metadata 格式，但不强制所有代码使用。
///
/// # 字段说明
/// * `description` - Block 的详细描述
/// * `created_at` - 创建时间（ISO 8601 UTC 格式，例如："2025-12-17T02:30:00Z"）
/// * `updated_at` - 最后更新时间（ISO 8601 UTC 格式）
/// * `custom` - 自定义扩展字段（使用 #[serde(flatten)] 合并到根对象）
///
/// # 未来扩展（预留）
/// * `status` - 状态字段（draft, in_review, published 等）
///   本质上是权限模板的应用，MVP 阶段不实现
///
/// # 示例
/// ```
/// use elfiee_lib::models::BlockMetadata;
///
/// let metadata = BlockMetadata {
///     description: Some("项目需求文档".to_string()),
///     created_at: Some("2025-12-17T02:30:00Z".to_string()),
///     updated_at: Some("2025-12-17T10:15:00Z".to_string()),
///     // status: Some("draft".to_string()), // 未来扩展
///     custom: Default::default(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct BlockMetadata {
    /// Block 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 创建时间（ISO 8601 UTC）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// 最后更新时间（ISO 8601 UTC）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    // 未来扩展字段（预留）
    // /// Block 状态（draft, in_review, published 等）
    // /// MVP 阶段不实现，预留给未来的权限模板系统
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub status: Option<String>,

    /// 自定义扩展字段
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for BlockMetadata {
    fn default() -> Self {
        Self {
            description: None,
            created_at: None,
            updated_at: None,
            custom: HashMap::new(),
        }
    }
}

impl BlockMetadata {
    /// 创建新的 BlockMetadata，自动设置当前时间
    pub fn new() -> Self {
        let now = crate::utils::time::now_utc();
        Self {
            description: None,
            created_at: Some(now.clone()),
            updated_at: Some(now),
            custom: HashMap::new(),
        }
    }

    /// 从 JSON Value 解析 BlockMetadata
    pub fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse BlockMetadata: {}", e))
    }

    /// 转换为 JSON Value
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }

    /// 更新 updated_at 为当前时间
    pub fn touch(&mut self) {
        self.updated_at = Some(crate::utils::time::now_utc());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let metadata = BlockMetadata::default();
        assert!(metadata.description.is_none());
        assert!(metadata.created_at.is_none());
        assert!(metadata.updated_at.is_none());
        assert!(metadata.custom.is_empty());
    }

    #[test]
    fn test_new() {
        let metadata = BlockMetadata::new();
        assert!(metadata.created_at.is_some());
        assert!(metadata.updated_at.is_some());

        let created = metadata.created_at.unwrap();
        let updated = metadata.updated_at.unwrap();

        // 应该是有效的 UTC 时间戳
        assert!(created.ends_with('Z'));
        assert!(updated.ends_with('Z'));
    }

    #[test]
    fn test_to_json_and_from_json() {
        let metadata = BlockMetadata {
            description: Some("测试描述".to_string()),
            created_at: Some("2025-12-17T02:30:00Z".to_string()),
            updated_at: Some("2025-12-17T10:15:00Z".to_string()),
            custom: {
                let mut map = HashMap::new();
                map.insert("priority".to_string(), serde_json::json!("high"));
                map
            },
        };

        // 转换为 JSON
        let json = metadata.to_json();
        assert_eq!(json["description"], "测试描述");
        assert_eq!(json["created_at"], "2025-12-17T02:30:00Z");
        assert_eq!(json["priority"], "high");

        // 从 JSON 恢复
        let restored = BlockMetadata::from_json(&json).unwrap();
        assert_eq!(restored, metadata);
    }

    #[test]
    fn test_touch() {
        let mut metadata = BlockMetadata {
            description: Some("测试".to_string()),
            created_at: Some("2025-12-17T02:30:00Z".to_string()),
            updated_at: Some("2025-12-17T02:30:00Z".to_string()),
            custom: HashMap::new(),
        };

        let original_updated = metadata.updated_at.clone().unwrap();

        // 等待一小段时间
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 更新时间戳
        metadata.touch();

        let new_updated = metadata.updated_at.clone().unwrap();

        // updated_at 应该变化，created_at 不变
        assert_eq!(metadata.created_at.unwrap(), "2025-12-17T02:30:00Z");
        assert_ne!(original_updated, new_updated);
    }

    #[test]
    fn test_serialization_omits_none() {
        let metadata = BlockMetadata {
            description: Some("测试".to_string()),
            created_at: None,
            updated_at: None,
            custom: HashMap::new(),
        };

        let json = serde_json::to_value(&metadata).unwrap();

        // None 字段不应该出现在 JSON 中
        assert!(json["description"].is_string());
        assert!(json.get("created_at").is_none() || json["created_at"].is_null());
        assert!(json.get("updated_at").is_none() || json["updated_at"].is_null());
    }

    #[test]
    fn test_custom_fields() {
        let json = serde_json::json!({
            "description": "测试",
            "created_at": "2025-12-17T02:30:00Z",
            "custom_field_1": "value1",
            "custom_field_2": 42
        });

        let metadata = BlockMetadata::from_json(&json).unwrap();

        assert_eq!(metadata.description, Some("测试".to_string()));
        assert_eq!(metadata.custom.get("custom_field_1").unwrap(), "value1");
        assert_eq!(metadata.custom.get("custom_field_2").unwrap(), &serde_json::json!(42));
    }
}
```

**修改**：`src-tauri/src/models/mod.rs`

```rust
mod block;
mod capability;
mod command;
mod editor;
mod event;
mod grant;
pub mod metadata;  // ← 新增
pub mod payloads;

pub use block::Block;
pub use capability::Capability;
pub use command::Command;
pub use editor::Editor;
pub use event::Event;
pub use grant::Grant;
pub use metadata::BlockMetadata;  // ← 新增
pub use payloads::*;
```

**测试命令**：
```bash
cd src-tauri
cargo test models::metadata::tests --lib
```

**验收标准**：
- ✅ 所有测试通过
- ✅ `BlockMetadata::new()` 自动设置时间戳
- ✅ `to_json()` 和 `from_json()` 正确转换
- ✅ `touch()` 更新 `updated_at`
- ✅ None 字段在序列化时被省略
- ✅ 自定义字段通过 flatten 正确合并

---

#### 任务 1.3：修改 Block 模型（1.5h）

**文件**：`src-tauri/src/models/block.rs`

**修改内容**：

```rust
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Block {
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: serde_json::Value,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,

    /// 元数据（灵活的 JSON 对象）
    ///
    /// 推荐使用 BlockMetadata 结构，但不强制。
    /// 默认为空对象 {}
    pub metadata: serde_json::Value,  // ← 新增
}

impl Block {
    pub fn new(name: String, block_type: String, owner: String) -> Self {
        Self {
            block_id: uuid::Uuid::new_v4().to_string(),
            name,
            block_type,
            contents: serde_json::json!({}),
            children: HashMap::new(),
            owner,
            metadata: serde_json::json!({}),  // ← 新增
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_block_has_empty_metadata() {
        let block = Block::new(
            "Test Block".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        assert_eq!(block.name, "Test Block");
        assert_eq!(block.block_type, "markdown");
        assert_eq!(block.owner, "alice");
        assert_eq!(block.metadata, serde_json::json!({}));
    }

    #[test]
    fn test_block_with_metadata() {
        let mut block = Block::new(
            "Test".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        block.metadata = serde_json::json!({
            "description": "测试描述",
            "created_at": "2025-12-17T02:30:00Z"
        });

        assert_eq!(block.metadata["description"], "测试描述");
        assert_eq!(block.metadata["created_at"], "2025-12-17T02:30:00Z");
    }
}
```

**测试命令**：
```bash
cd src-tauri
cargo test models::block::tests --lib
```

**验收标准**：
- ✅ 编译通过
- ✅ 所有测试通过
- ✅ `Block::new()` 默认 metadata 为 `{}`
- ✅ metadata 可以存储任意 JSON

---

### 阶段 2：Capability 修改（3h）

#### 任务 2.1：扩展 CreateBlockPayload（0.5h）

**文件**：`src-tauri/src/models/payloads.rs`

**修改内容**：

```rust
/// Payload for core.create capability
///
/// This payload is used to create a new block with a name and type.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateBlockPayload {
    /// The display name for the new block
    pub name: String,
    /// The block type (e.g., "markdown", "code", "diagram")
    pub block_type: String,
    /// Optional metadata (description, custom fields, etc.)
    ///
    /// If provided, will be merged with auto-generated timestamps.
    /// Example: { "description": "项目需求文档" }
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,  // ← 新增
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_block_payload() {
        let json = serde_json::json!({
            "name": "My Block",
            "block_type": "markdown"
        });
        let payload: CreateBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.name, "My Block");
        assert_eq!(payload.block_type, "markdown");
        assert!(payload.metadata.is_none());  // ← 新增测试
    }

    #[test]
    fn test_create_block_payload_with_metadata() {  // ← 新增测试
        let json = serde_json::json!({
            "name": "My Block",
            "block_type": "markdown",
            "metadata": {
                "description": "测试描述"
            }
        });
        let payload: CreateBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.name, "My Block");
        assert!(payload.metadata.is_some());

        let metadata = payload.metadata.unwrap();
        assert_eq!(metadata["description"], "测试描述");
    }

    // ... 其他测试保持不变 ...
}
```

**测试命令**：
```bash
cd src-tauri
cargo test payloads::test_create_block_payload --lib
```

**验收标准**：
- ✅ 编译通过
- ✅ 测试通过
- ✅ metadata 字段可选（默认 None）
- ✅ 可以反序列化带 metadata 的 payload

---

#### 任务 2.2：修改 core.create Handler（1h）

**文件**：`src-tauri/src/capabilities/builtins/create.rs`

**修改内容**：

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, CreateBlockPayload, Event};
use crate::utils::time::now_utc;  // ← 新增
use capability_macros::capability;

/// Handler for core.create capability.
///
/// Creates a new block with name, type, owner, and optional metadata.
/// Automatically generates created_at and updated_at timestamps.
///
/// Note: The block parameter is None for create since the block doesn't exist yet.
#[capability(id = "core.create", target = "core/*")]
fn handle_create(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>> {
    // Strongly-typed deserialization
    let payload: CreateBlockPayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for core.create: {}", e))?;

    // Generate new block ID
    let block_id = uuid::Uuid::new_v4().to_string();

    // Prepare metadata with timestamps
    let now = now_utc();  // ← 新增
    let mut metadata = payload.metadata.unwrap_or_else(|| serde_json::json!({}));  // ← 新增

    // 合并用户提供的 metadata 和自动生成的时间戳  // ← 新增
    if let Some(obj) = metadata.as_object_mut() {  // ← 新增
        obj.insert("created_at".to_string(), serde_json::json!(now.clone()));  // ← 新增
        obj.insert("updated_at".to_string(), serde_json::json!(now));  // ← 新增
    }  // ← 新增

    // Create a single event with full initial state
    // Per README.md Part 2: create events contain the full initial state
    let event = create_event(
        block_id.clone(),
        "core.create", // cap_id
        serde_json::json!({
            "name": payload.name,
            "type": payload.block_type,
            "owner": cmd.editor_id,
            "contents": {},
            "children": {},
            "metadata": metadata  // ← 新增
        }),
        &cmd.editor_id,
        1, // Placeholder - engine actor updates with correct count (actor.rs:227)
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Command;

    #[test]
    fn test_create_generates_metadata_with_timestamps() {
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "markdown"
            }),
        );

        let result = handle_create(&cmd, None);
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        let metadata = &event.value["metadata"];

        // 应该自动生成时间戳
        assert!(metadata["created_at"].is_string());
        assert!(metadata["updated_at"].is_string());

        // 时间戳应该是 UTC 格式（以 Z 结尾）
        let created = metadata["created_at"].as_str().unwrap();
        let updated = metadata["updated_at"].as_str().unwrap();
        assert!(created.ends_with('Z'));
        assert!(updated.ends_with('Z'));
    }

    #[test]
    fn test_create_merges_user_metadata() {
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "markdown",
                "metadata": {
                    "description": "测试描述"
                }
            }),
        );

        let result = handle_create(&cmd, None);
        assert!(result.is_ok());

        let events = result.unwrap();
        let event = &events[0];
        let metadata = &event.value["metadata"];

        // 用户提供的字段应该保留
        assert_eq!(metadata["description"], "测试描述");

        // 自动生成的时间戳也应该存在
        assert!(metadata["created_at"].is_string());
        assert!(metadata["updated_at"].is_string());
    }

    #[test]
    fn test_create_without_metadata() {
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "markdown"
            }),
        );

        let result = handle_create(&cmd, None);
        assert!(result.is_ok());

        let events = result.unwrap();
        let event = &events[0];
        let metadata = &event.value["metadata"];

        // 即使用户没提供 metadata，也应该有时间戳
        assert!(metadata.is_object());
        assert!(metadata["created_at"].is_string());
        assert!(metadata["updated_at"].is_string());
    }
}
```

**测试命令**：
```bash
cd src-tauri
cargo test capabilities::builtins::create::tests --lib
```

**验收标准**：
- ✅ 编译通过
- ✅ 所有测试通过
- ✅ 自动生成 created_at 和 updated_at
- ✅ 合并用户提供的 metadata
- ✅ 时间戳格式正确（UTC，以 Z 结尾）

---

#### 任务 2.3：修改 markdown.write Handler（1.5h）

**文件**：`src-tauri/src/extensions/markdown/markdown_write.rs`

**修改内容**：

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use crate::utils::time::now_utc;  // ← 新增
use capability_macros::capability;

use super::MarkdownWritePayload;

/// Handler for markdown.write capability.
///
/// Writes markdown content to a markdown block's contents field.
/// The content is stored under the "markdown" key in the contents HashMap.
/// Automatically updates the block's metadata.updated_at timestamp.
///
/// # Payload
/// Uses `MarkdownWritePayload` with a single `content` field containing the markdown string.
#[capability(id = "markdown.write", target = "markdown")]
fn handle_markdown_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for markdown.write")?;

    // Deserialize strongly-typed payload
    let payload: MarkdownWritePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for markdown.write: {}", e))?;

    // Update contents JSON object with markdown content
    let mut new_contents = if let Some(obj) = block.contents.as_object() {
        obj.clone()
    } else {
        serde_json::Map::new()
    };
    new_contents.insert("markdown".to_string(), serde_json::json!(payload.content));

    // Update metadata.updated_at  // ← 新增
    let mut new_metadata = block.metadata.clone();  // ← 新增
    if let Some(obj) = new_metadata.as_object_mut() {  // ← 新增
        obj.insert("updated_at".to_string(), serde_json::json!(now_utc()));  // ← 新增
    }  // ← 新增

    // Create event
    let event = create_event(
        block.block_id.clone(),
        "markdown.write", // cap_id
        serde_json::json!({
            "contents": new_contents,
            "metadata": new_metadata  // ← 新增
        }),
        &cmd.editor_id,
        1, // Placeholder - engine actor updates with correct count (actor.rs:227)
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Block, Command};

    fn create_test_block() -> Block {
        Block {
            block_id: "block-123".to_string(),
            name: "Test Block".to_string(),
            block_type: "markdown".to_string(),
            owner: "alice".to_string(),
            contents: serde_json::json!({}),
            children: Default::default(),
            metadata: serde_json::json!({
                "created_at": "2025-12-17T02:30:00Z",
                "updated_at": "2025-12-17T02:30:00Z"
            }),
        }
    }

    #[test]
    fn test_markdown_write_updates_timestamp() {
        let block = create_test_block();
        let original_updated = block.metadata["updated_at"].as_str().unwrap();

        // 等待一小段时间确保时间戳不同
        std::thread::sleep(std::time::Duration::from_millis(10));

        let cmd = Command::new(
            "alice".to_string(),
            "markdown.write".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "content": "# Hello World"
            }),
        );

        let result = handle_markdown_write(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        let new_metadata = &event.value["metadata"];

        // updated_at 应该被更新
        let new_updated = new_metadata["updated_at"].as_str().unwrap();
        assert_ne!(original_updated, new_updated);

        // created_at 应该保持不变
        assert_eq!(
            new_metadata["created_at"].as_str().unwrap(),
            "2025-12-17T02:30:00Z"
        );

        // 内容应该被更新
        let new_contents = &event.value["contents"];
        assert_eq!(new_contents["markdown"], "# Hello World");
    }

    #[test]
    fn test_markdown_write_preserves_other_metadata() {
        let mut block = create_test_block();
        block.metadata = serde_json::json!({
            "description": "测试描述",
            "created_at": "2025-12-17T02:30:00Z",
            "updated_at": "2025-12-17T02:30:00Z",
            "custom_field": "custom_value"
        });

        let cmd = Command::new(
            "alice".to_string(),
            "markdown.write".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "content": "New content"
            }),
        );

        let result = handle_markdown_write(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        let new_metadata = &events[0].value["metadata"];

        // 其他字段应该保留
        assert_eq!(new_metadata["description"], "测试描述");
        assert_eq!(new_metadata["custom_field"], "custom_value");
        assert_eq!(new_metadata["created_at"], "2025-12-17T02:30:00Z");

        // updated_at 应该被更新
        assert_ne!(new_metadata["updated_at"], "2025-12-17T02:30:00Z");
    }

    #[test]
    fn test_markdown_write_handles_missing_metadata() {
        let mut block = create_test_block();
        block.metadata = serde_json::json!({});  // 空 metadata

        let cmd = Command::new(
            "alice".to_string(),
            "markdown.write".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "content": "Content"
            }),
        );

        let result = handle_markdown_write(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        let new_metadata = &events[0].value["metadata"];

        // 应该添加 updated_at
        assert!(new_metadata["updated_at"].is_string());
    }
}
```

**测试命令**：
```bash
cd src-tauri
cargo test extensions::markdown::markdown_write::tests --lib
```

**验收标准**：
- ✅ 编译通过
- ✅ 所有测试通过
- ✅ updated_at 被自动更新
- ✅ created_at 保持不变
- ✅ 其他 metadata 字段保留
- ✅ 处理空 metadata 的情况

---

### 阶段 3：StateProjector 修改（2.5h）

#### 任务 3.1：修改 StateProjector（2.5h）

**文件**：`src-tauri/src/engine/state.rs`

**修改内容**：

```rust
// 在 apply_event 方法中修改 "core.create" 分支
"core.create" => {
    // Create event should contain full block state
    if let Some(obj) = event.value.as_object() {
        let block = Block {
            block_id: event.entity.clone(),
            name: obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            block_type: obj
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            owner: obj
                .get("owner")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            contents: obj
                .get("contents")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            children: obj
                .get("children")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            metadata: obj  // ← 新增
                .get("metadata")  // ← 新增
                .cloned()  // ← 新增
                .unwrap_or_else(|| serde_json::json!({})),  // ← 新增
        };
        self.blocks.insert(block.block_id.clone(), block);
    }
}

// 修改 ".write" 和 ".link" 分支
_ if cap_id.ends_with(".write") || cap_id.ends_with(".link") => {
    if let Some(block) = self.blocks.get_mut(&event.entity) {
        // Update contents if present
        if let Some(contents) = event.value.get("contents") {
            if let Some(obj) = block.contents.as_object_mut() {
                if let Some(new_contents) = contents.as_object() {
                    for (k, v) in new_contents {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        // Update children if present
        if let Some(children) = event.value.get("children") {
            if let Ok(new_children) = serde_json::from_value(children.clone()) {
                block.children = new_children;
            }
        }
        // Update metadata if present  // ← 新增
        if let Some(new_metadata) = event.value.get("metadata") {  // ← 新增
            block.metadata = new_metadata.clone();  // ← 新增
        }  // ← 新增
    }
}
```

**新增测试**：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ... 保留原有测试 ...

    #[test]
    fn test_apply_create_event_with_metadata() {
        let mut state = StateProjector::new();

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "type": "markdown",
                "owner": "alice",
                "contents": {},
                "children": {},
                "metadata": {
                    "description": "测试描述",
                    "created_at": "2025-12-17T02:30:00Z",
                    "updated_at": "2025-12-17T02:30:00Z"
                }
            }),
            {
                let mut ts = std::collections::HashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );

        state.apply_event(&event);

        assert_eq!(state.blocks.len(), 1);
        let block = state.get_block("block1").unwrap();
        assert_eq!(block.name, "Test Block");

        // 验证 metadata
        assert_eq!(block.metadata["description"], "测试描述");
        assert_eq!(block.metadata["created_at"], "2025-12-17T02:30:00Z");
        assert_eq!(block.metadata["updated_at"], "2025-12-17T02:30:00Z");
    }

    #[test]
    fn test_apply_create_event_without_metadata() {
        let mut state = StateProjector::new();

        // 旧版本 Event，没有 metadata 字段
        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Old Block",
                "type": "markdown",
                "owner": "alice",
                "contents": {},
                "children": {}
                // 没有 metadata
            }),
            {
                let mut ts = std::collections::HashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );

        state.apply_event(&event);

        assert_eq!(state.blocks.len(), 1);
        let block = state.get_block("block1").unwrap();
        assert_eq!(block.name, "Old Block");

        // metadata 应该是空对象（向后兼容）
        assert_eq!(block.metadata, serde_json::json!({}));
    }

    #[test]
    fn test_apply_write_event_updates_metadata() {
        let mut state = StateProjector::new();

        // 先创建 Block
        let create_event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test",
                "type": "markdown",
                "owner": "alice",
                "contents": {},
                "children": {},
                "metadata": {
                    "created_at": "2025-12-17T02:30:00Z",
                    "updated_at": "2025-12-17T02:30:00Z"
                }
            }),
            {
                let mut ts = std::collections::HashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&create_event);

        // 写入内容
        let write_event = Event::new(
            "block1".to_string(),
            "alice/markdown.write".to_string(),
            serde_json::json!({
                "contents": {
                    "markdown": "# Hello"
                },
                "metadata": {
                    "created_at": "2025-12-17T02:30:00Z",
                    "updated_at": "2025-12-17T10:15:00Z"  // 时间戳更新
                }
            }),
            {
                let mut ts = std::collections::HashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
        );
        state.apply_event(&write_event);

        let block = state.get_block("block1").unwrap();

        // contents 应该被更新
        assert_eq!(block.contents["markdown"], "# Hello");

        // metadata 应该被更新
        assert_eq!(block.metadata["created_at"], "2025-12-17T02:30:00Z");
        assert_eq!(block.metadata["updated_at"], "2025-12-17T10:15:00Z");
    }

    #[test]
    fn test_replay_maintains_metadata() {
        let mut state = StateProjector::new();

        let events = vec![
            Event::new(
                "block1".to_string(),
                "alice/core.create".to_string(),
                serde_json::json!({
                    "name": "Block 1",
                    "type": "markdown",
                    "owner": "alice",
                    "contents": {},
                    "children": {},
                    "metadata": {
                        "description": "描述1",
                        "created_at": "2025-12-17T02:00:00Z",
                        "updated_at": "2025-12-17T02:00:00Z"
                    }
                }),
                {
                    let mut ts = std::collections::HashMap::new();
                    ts.insert("alice".to_string(), 1);
                    ts
                },
            ),
            Event::new(
                "block1".to_string(),
                "alice/markdown.write".to_string(),
                serde_json::json!({
                    "contents": { "markdown": "内容" },
                    "metadata": {
                        "description": "描述1",
                        "created_at": "2025-12-17T02:00:00Z",
                        "updated_at": "2025-12-17T03:00:00Z"
                    }
                }),
                {
                    let mut ts = std::collections::HashMap::new();
                    ts.insert("alice".to_string(), 2);
                    ts
                },
            ),
        ];

        state.replay(events);

        let block = state.get_block("block1").unwrap();
        assert_eq!(block.metadata["description"], "描述1");
        assert_eq!(block.metadata["created_at"], "2025-12-17T02:00:00Z");
        assert_eq!(block.metadata["updated_at"], "2025-12-17T03:00:00Z");
    }
}
```

**测试命令**：
```bash
cd src-tauri
cargo test engine::state::tests --lib
```

**验收标准**：
- ✅ 编译通过
- ✅ 所有原有测试仍然通过
- ✅ 新测试通过
- ✅ 向后兼容（旧 Event 没有 metadata）
- ✅ metadata 在重放时正确恢复

---

### 阶段 4：类型绑定（1h）

#### 任务 4.1：注册类型并生成绑定（1h）

**文件**：`src-tauri/src/lib.rs`

**修改内容**：

```rust
// 在类型注册部分添加
use elfiee_lib::models::BlockMetadata;  // ← 新增

// 在 export_types 函数中添加
builder = builder
    .ty::<Block>()?
    .ty::<BlockMetadata>()?  // ← 新增
    .ty::<Editor>()?
    // ... 其他类型 ...
```

**操作步骤**：

1. 修改 `lib.rs` 注册 `BlockMetadata` 类型
2. 运行 `cargo build` 或 `pnpm tauri dev`
3. 检查 `src/bindings.ts` 是否生成新类型

**验收标准**：

检查 `src/bindings.ts` 应该包含：

```typescript
export type Block = {
  block_id: string
  name: string
  block_type: string
  contents: any
  children: Record<string, string[]>
  owner: string
  metadata: any  // ← 新增
}

export type BlockMetadata = {
  description?: string | null
  created_at?: string | null
  updated_at?: string | null
  [key: string]: any
}

export type CreateBlockPayload = {
  name: string
  block_type: string
  metadata?: any | null  // ← 新增
}
```

**测试**：

```bash
cd src-tauri
cargo build --release
cd ..
# 检查 bindings.ts
grep "metadata" src/bindings.ts
```

---

### 阶段 5：集成测试（1.5h）

#### 任务 5.1：端到端测试（1.5h）

**文件**：`src-tauri/src/engine/actor.rs`（在现有测试基础上扩展）

**新增测试**：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ... 保留原有测试 ...

    #[tokio::test]
    async fn test_create_block_with_metadata() {
        let pool = create_test_pool().await;
        let actor = EngineActor::new(pool).await.unwrap();

        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "测试文档",
                "block_type": "markdown",
                "metadata": {
                    "description": "这是一个测试文档"
                }
            }),
        );

        let result = actor.process_command(cmd).await;
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let block_id = events[0].entity.clone();
        let block = actor.state.read().await.get_block(&block_id).cloned();
        assert!(block.is_some());

        let block = block.unwrap();
        assert_eq!(block.name, "测试文档");
        assert_eq!(block.metadata["description"], "这是一个测试文档");
        assert!(block.metadata["created_at"].is_string());
        assert!(block.metadata["updated_at"].is_string());
    }

    #[tokio::test]
    async fn test_write_updates_timestamp() {
        let pool = create_test_pool().await;
        let actor = EngineActor::new(pool).await.unwrap();

        // 创建 Block
        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test",
                "block_type": "markdown"
            }),
        );

        let events = actor.process_command(create_cmd).await.unwrap();
        let block_id = events[0].entity.clone();

        // 获取初始时间戳
        let block = actor.state.read().await.get_block(&block_id).cloned().unwrap();
        let original_updated = block.metadata["updated_at"].as_str().unwrap().to_string();

        // 等待一小段时间
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 写入内容
        let write_cmd = Command::new(
            "alice".to_string(),
            "markdown.write".to_string(),
            block_id.clone(),
            serde_json::json!({
                "content": "# Hello World"
            }),
        );

        let result = actor.process_command(write_cmd).await;
        assert!(result.is_ok());

        // 检查时间戳是否更新
        let block = actor.state.read().await.get_block(&block_id).cloned().unwrap();
        let new_updated = block.metadata["updated_at"].as_str().unwrap();

        assert_ne!(original_updated, new_updated);
        assert_eq!(
            block.metadata["created_at"].as_str().unwrap(),
            block.metadata["created_at"].as_str().unwrap()
        ); // created_at 不变
    }

    #[tokio::test]
    async fn test_metadata_persists_after_replay() {
        let pool = create_test_pool().await;

        // 创建第一个 actor，执行操作
        {
            let actor = EngineActor::new(pool.clone()).await.unwrap();

            let cmd = Command::new(
                "alice".to_string(),
                "core.create".to_string(),
                "".to_string(),
                serde_json::json!({
                    "name": "持久化测试",
                    "block_type": "markdown",
                    "metadata": {
                        "description": "测试持久化"
                    }
                }),
            );

            actor.process_command(cmd).await.unwrap();
        }

        // 创建第二个 actor，重放事件
        {
            let actor = EngineActor::new(pool.clone()).await.unwrap();

            // 应该能够从事件重放恢复 metadata
            let state = actor.state.read().await;
            assert_eq!(state.blocks.len(), 1);

            let block = state.blocks.values().next().unwrap();
            assert_eq!(block.name, "持久化测试");
            assert_eq!(block.metadata["description"], "测试持久化");
            assert!(block.metadata["created_at"].is_string());
            assert!(block.metadata["updated_at"].is_string());
        }
    }
}
```

**测试命令**：
```bash
cd src-tauri
cargo test engine::actor::tests --lib -- --test-threads=1
```

**验收标准**：
- ✅ 所有集成测试通过
- ✅ metadata 正确创建和更新
- ✅ metadata 持久化到 Event DB
- ✅ metadata 在重放后正确恢复

---

## 📊 工时汇总

| 阶段 | 任务 | 预计工时 | 累计工时 |
|------|------|---------|---------|
| **阶段 1** | 基础设施 | | |
| 1.1 | 创建时间戳工具 | 1.0h | 1.0h |
| 1.2 | 创建 BlockMetadata 模型 | 1.0h | 2.0h |
| 1.3 | 修改 Block 模型 | 1.5h | 3.5h |
| **阶段 2** | Capability 修改 | | |
| 2.1 | 扩展 CreateBlockPayload | 0.5h | 4.0h |
| 2.2 | 修改 core.create Handler | 1.0h | 5.0h |
| 2.3 | 修改 markdown.write Handler | 1.5h | 6.5h |
| **阶段 3** | StateProjector 修改 | | |
| 3.1 | 修改 StateProjector | 2.5h | 9.0h |
| **阶段 4** | 类型绑定 | | |
| 4.1 | 注册类型并生成绑定 | 1.0h | 10.0h |
| **阶段 5** | 集成测试 | | |
| 5.1 | 端到端测试 | 1.5h | 11.5h |
| **总计** | | **11.5h** | |

---

## ✅ 验收标准

### 功能验收

1. **Block 结构**
   - ✅ Block 有 metadata 字段（JSON）
   - ✅ 默认值为 `{}`

2. **创建 Block**
   - ✅ 自动生成 created_at 和 updated_at
   - ✅ 用户提供的 metadata 被合并
   - ✅ 时间戳格式正确（UTC，ISO 8601）

3. **更新 Block**
   - ✅ markdown.write 自动更新 updated_at
   - ✅ created_at 保持不变
   - ✅ 其他 metadata 字段保留

4. **持久化**
   - ✅ metadata 存储在 Event DB
   - ✅ 重放 Event 后 metadata 正确恢复
   - ✅ 向后兼容（旧 Event 没有 metadata）

5. **前端绑定**
   - ✅ TypeScript 类型正确生成
   - ✅ Block.metadata 可访问
   - ✅ BlockMetadata 类型可用

### 测试验收

1. **单元测试**
   - ✅ utils::time::tests - 5 个测试通过
   - ✅ models::metadata::tests - 7 个测试通过
   - ✅ models::block::tests - 2 个测试通过
   - ✅ payloads::tests - 2 个测试通过
   - ✅ capabilities::builtins::create::tests - 3 个测试通过
   - ✅ extensions::markdown::markdown_write::tests - 3 个测试通过
   - ✅ engine::state::tests - 4 个新测试通过

2. **集成测试**
   - ✅ engine::actor::tests - 3 个新测试通过

3. **测试覆盖率**
   - ✅ 新增代码测试覆盖率 > 90%

### 代码质量

1. **代码风格**
   - ✅ 通过 `cargo fmt` 检查
   - ✅ 通过 `cargo clippy` 检查
   - ✅ 无编译警告

2. **文档**
   - ✅ 所有公开函数有文档注释
   - ✅ 复杂逻辑有行内注释
   - ✅ 示例代码可运行

---

## 🎯 里程碑检查点

### 检查点 1：基础设施完成（3.5h）

**验证命令**：
```bash
cd src-tauri
cargo test utils::time --lib
cargo test models::metadata --lib
cargo test models::block --lib
```

**预期结果**：
- ✅ 14 个测试全部通过
- ✅ utils/time.rs 和 models/metadata.rs 文件存在
- ✅ Block 结构包含 metadata 字段

---

### 检查点 2：Capability 修改完成（6.5h）

**验证命令**：
```bash
cd src-tauri
cargo test capabilities::builtins::create --lib
cargo test extensions::markdown::markdown_write --lib
```

**预期结果**：
- ✅ 6 个测试全部通过
- ✅ core.create 自动生成时间戳
- ✅ markdown.write 自动更新 updated_at

---

### 检查点 3：StateProjector 修改完成（9.0h）

**验证命令**：
```bash
cd src-tauri
cargo test engine::state --lib
```

**预期结果**：
- ✅ 所有测试通过（包括新增的 4 个测试）
- ✅ metadata 在重放时正确恢复
- ✅ 向后兼容旧 Event

---

### 检查点 4：集成完成（11.5h）

**验证命令**：
```bash
cd src-tauri
cargo test --lib
cargo build
cd ..
grep "metadata" src/bindings.ts
```

**预期结果**：
- ✅ 所有测试通过
- ✅ 编译无错误无警告
- ✅ bindings.ts 包含 metadata 相关类型

---

## 🚀 开发顺序建议

### Day 1（4h）：基础设施
- 上午：任务 1.1 + 1.2（时间工具 + BlockMetadata）
- 下午：任务 1.3（修改 Block 模型）

### Day 2（4h）：Capability 修改
- 上午：任务 2.1 + 2.2（Payload + core.create）
- 下午：任务 2.3（markdown.write）

### Day 3（3.5h）：StateProjector 和集成
- 上午：任务 3.1（StateProjector）
- 下午：任务 4.1 + 5.1（类型绑定 + 集成测试）

---

## 📌 注意事项

### TDD 开发流程

1. **红灯阶段**：先写测试，测试失败（红灯）
2. **绿灯阶段**：实现功能，测试通过（绿灯）
3. **重构阶段**：优化代码，保持测试通过

**示例**：
```bash
# 1. 写测试（红灯）
# 在 tests 模块中添加测试函数
cargo test utils::time::test_now_utc_format --lib
# 预期：测试失败（函数未实现）

# 2. 实现功能（绿灯）
# 实现 now_utc() 函数
cargo test utils::time::test_now_utc_format --lib
# 预期：测试通过

# 3. 重构（保持绿灯）
# 优化代码结构，确保测试仍然通过
cargo test utils::time --lib
```

### 时区处理原则

1. **存储**：统一使用 UTC
2. **传输**：使用 ISO 8601 格式（以 Z 结尾）
3. **显示**：前端按需转换为本地时区

### 向后兼容原则

1. 所有新字段使用 `Option` 或默认值
2. 解析旧 Event 时，缺失字段使用默认值
3. StateProjector 兼容旧格式

### Git 提交建议

```bash
# 每个任务完成后提交
git add .
git commit -m "feat: 添加时间戳工具 utils/time.rs"
git commit -m "feat: 添加 BlockMetadata 模型"
git commit -m "feat: Block 添加 metadata 字段"
# ... 依次提交
```

---

**最后更新**: 2025-12-17

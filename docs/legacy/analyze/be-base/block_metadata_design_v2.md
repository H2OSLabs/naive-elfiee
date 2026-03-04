# Block 元数据扩展方案 v2（最终版）

## 文档信息

- **文档版本**: 2.0（基于用户反馈重新设计）
- **创建日期**: 2025-12-17
- **目标**: P0 任务 - Block 元数据字段统一设计
- **核心原则**: 统一流程、灵活扩展、时区安全

---

## 📋 用户反馈要点

### 关键决策

1. ✅ **name = title**，不需要区分两个字段
2. ✅ **需要添加的字段**：
   - `created_time` - 创建时间
   - `updated_time` - 最后更新时间
   - `description` - 描述
   - `status` - 状态（未来扩展，MVP 先预留）
3. ⚠️ **status 字段未确定**：
   - 本质是权限模板的应用（draft/in review/published）
   - 涉及"谁在什么阶段可以做什么"
   - MVP 阶段先预留，不强制
4. ⚠️ **时区问题**：
   - 时区变化时如何处理？
   - 是否需要标记时区或统一转换？
5. ✅ **灵活性原则**：
   - 除已有字段外，其他统一放在 `metadata` 字段中
   - 保持扩展灵活性

### 统一流程原则 ⭐

**不存在"有的在 Event 中，有的在内存中"的混乱处理**

所有数据修改必须：
```
前端 Command → Tauri IPC → Engine Actor → Event 生成 → Event DB 持久化 → StateProjector 重放 → 内存状态更新
```

---

## 🎯 最终设计方案

### 方案 A：最小化修改（推荐 MVP）✅

**核心思想**：只添加一个 `metadata` 字段，保持核心结构不变

#### 1. Block 数据结构

```rust
// src-tauri/src/models/block.rs
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Block {
    // ========== 核心字段（不变） ==========
    pub block_id: String,                       // UUID
    pub name: String,                           // 显示名称（同时作为 title）
    pub block_type: String,                     // markdown, code, diagram
    pub contents: serde_json::Value,            // 内容（markdown 文本、代码等）
    pub children: HashMap<String, Vec<String>>, // 关系图
    pub owner: String,                          // 所有者 editor_id

    // ========== 新增字段 ==========
    pub metadata: serde_json::Value,            // 灵活的元数据（JSON）
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
            metadata: serde_json::json!({}),  // 默认空对象
        }
    }
}
```

#### 2. Metadata 结构定义（推荐格式）

```rust
// src-tauri/src/models/metadata.rs (新建辅助文件，非强制)
use serde::{Deserialize, Serialize};
use specta::Type;

/// Block 元数据的推荐结构（非强制，仅作为参考）
///
/// 实际存储在 Block.metadata 字段中（JSON）
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BlockMetadata {
    /// 描述
    pub description: Option<String>,

    /// 状态（draft, in_review, published 等）
    /// MVP 阶段可选，未来扩展
    pub status: Option<String>,

    /// 创建时间（ISO 8601 with timezone）
    /// 例如："2025-12-17T10:30:00+08:00" 或 "2025-12-17T02:30:00Z"
    pub created_at: Option<String>,

    /// 最后更新时间（ISO 8601 with timezone）
    pub updated_at: Option<String>,

    /// 自定义字段（扩展用）
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for BlockMetadata {
    fn default() -> Self {
        Self {
            description: None,
            status: None,
            created_at: None,
            updated_at: None,
            custom: HashMap::new(),
        }
    }
}
```

**说明**：
- `BlockMetadata` 是**推荐格式**，不强制所有代码使用
- Block 存储时仍然是 `metadata: serde_json::Value`
- 前端可以直接使用 TypeScript 类型（自动生成）
- 后端代码可以选择性地反序列化为 `BlockMetadata`

#### 3. 时间戳格式规范 ⭐

**时区安全方案**：使用 ISO 8601 with timezone (RFC 3339)

```rust
// src-tauri/src/utils/time.rs (新建文件)
use chrono::{DateTime, Utc, Local, FixedOffset};

/// 生成当前时间戳（UTC）
///
/// 格式："2025-12-17T02:30:00Z"
pub fn now_utc() -> String {
    Utc::now().to_rfc3339()
}

/// 生成当前时间戳（本地时区）
///
/// 格式："2025-12-17T10:30:00+08:00"
pub fn now_local() -> String {
    Local::now().to_rfc3339()
}

/// 解析时间戳并转换为 UTC
pub fn parse_to_utc(timestamp: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("Invalid timestamp: {}", e))
}

/// 解析时间戳并转换为本地时区
pub fn parse_to_local(timestamp: &str) -> Result<DateTime<Local>, String> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Local))
        .map_err(|e| format!("Invalid timestamp: {}", e))
}
```

**时区策略**：

| 策略 | 存储格式 | 优点 | 缺点 | 推荐 |
|------|---------|------|------|------|
| **UTC 统一** | `2025-12-17T02:30:00Z` | 简单、无歧义 | 前端需转换显示 | ✅ 推荐 |
| **本地时区** | `2025-12-17T10:30:00+08:00` | 保留用户时区信息 | 复杂、容易混淆 | ⚠️ 可选 |

**MVP 建议**：统一使用 UTC (`now_utc()`)

#### 4. CreateBlockPayload 修改

```rust
// src-tauri/src/models/payloads.rs
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateBlockPayload {
    pub name: String,
    pub block_type: String,

    // 新增：可选的元数据
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}
```

**示例用法**：

```typescript
// 前端调用
await commands.executeCommand({
  editor_id: "alice",
  cap_id: "core.create",
  block_id: "",
  payload: {
    name: "需求文档",
    block_type: "markdown",
    metadata: {
      description: "这是一个需求文档",
      status: "draft"
    }
  }
})
```

---

### 方案 B：显式字段（备选方案）

如果未来确定 `created_at`, `updated_at`, `description` 是核心字段，可以提升为 Block 的直接字段：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Block {
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: serde_json::Value,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,

    // 核心元数据（显式字段）
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,

    // 扩展元数据（灵活字段）
    pub metadata: serde_json::Value,  // status 等未确定的字段放这里
}
```

**权衡**：
- ✅ 类型安全性更强
- ❌ 灵活性降低
- ❌ 未来修改需要数据库迁移

**建议**：MVP 阶段先用方案 A，确定核心字段后再升级为方案 B

---

## 🔄 统一的数据流转流程

### 创建 Block 时的完整流程

```
┌─────────────────────────────────────────────────────────────┐
│  1. 前端发起 Command                                          │
├─────────────────────────────────────────────────────────────┤
│  await commands.executeCommand({                            │
│    editor_id: "alice",                                      │
│    cap_id: "core.create",                                   │
│    payload: {                                               │
│      name: "需求文档",                                       │
│      block_type: "markdown",                                │
│      metadata: {                                            │
│        description: "项目需求文档",                          │
│        status: "draft"                                      │
│      }                                                       │
│    }                                                         │
│  })                                                          │
└─────────────────────────────────────────────────────────────┘
                         ↓ Tauri IPC
┌─────────────────────────────────────────────────────────────┐
│  2. Tauri Command 接收                                        │
├─────────────────────────────────────────────────────────────┤
│  #[tauri::command]                                          │
│  pub async fn execute_command(cmd: Command) {               │
│    engine_handle.process_command(cmd).await                 │
│  }                                                           │
└─────────────────────────────────────────────────────────────┘
                         ↓ 转发给 Engine Actor
┌─────────────────────────────────────────────────────────────┐
│  3. Engine Actor 处理                                         │
├─────────────────────────────────────────────────────────────┤
│  // 3.1 查找 Capability Handler                             │
│  let handler = registry.get("core.create")?;                │
│                                                              │
│  // 3.2 授权检查（Owner 自动通过）                           │
│  handler.certificator(cmd, None)?;                          │
│                                                              │
│  // 3.3 执行 Handler，生成 Event                            │
│  let events = handler.handler(cmd, None)?;                  │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  4. core.create Handler 生成 Event                           │
├─────────────────────────────────────────────────────────────┤
│  let block_id = uuid::Uuid::new_v4().to_string();           │
│  let now = now_utc();  // "2025-12-17T02:30:00Z"            │
│                                                              │
│  // 合并用户提供的 metadata 和自动生成的时间戳               │
│  let mut metadata = payload.metadata                        │
│    .unwrap_or_else(|| json!({}));                           │
│                                                              │
│  metadata["created_at"] = json!(now.clone());               │
│  metadata["updated_at"] = json!(now);                       │
│                                                              │
│  Event::new(                                                 │
│    block_id.clone(),                                        │
│    format!("{}/core.create", cmd.editor_id),                │
│    json!({                                                   │
│      "name": payload.name,                                  │
│      "type": payload.block_type,                            │
│      "owner": cmd.editor_id,                                │
│      "contents": {},                                        │
│      "children": {},                                        │
│      "metadata": metadata  // ← 包含完整元数据               │
│    }),                                                       │
│    cmd.editor_id                                            │
│  )                                                           │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  5. 向量时钟冲突检测                                          │
├─────────────────────────────────────────────────────────────┤
│  if !is_newer_than_state(event.timestamp, state) {         │
│    return Err("Conflict detected");                         │
│  }                                                           │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  6. 持久化到 Event DB（SQLite）                               │
├─────────────────────────────────────────────────────────────┤
│  INSERT INTO events (                                        │
│    event_id, entity, attribute, value, timestamp            │
│  ) VALUES (                                                  │
│    "evt-123",                                               │
│    "block-456",                                             │
│    "alice/core.create",                                     │
│    '{"name":"需求文档","metadata":{"description":"...",     │
│      "created_at":"2025-12-17T02:30:00Z"}}',               │
│    '{"alice":1}'                                            │
│  )                                                           │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  7. StateProjector 重放 Event → 更新内存状态                  │
├─────────────────────────────────────────────────────────────┤
│  impl StateProjector {                                      │
│    fn apply_event(&mut self, event: &Event) {              │
│      match cap_id {                                         │
│        "core.create" => {                                   │
│          let block = Block {                                │
│            block_id: event.entity.clone(),                  │
│            name: value["name"].as_str()?.to_string(),       │
│            block_type: value["type"].as_str()?.to_string(), │
│            owner: value["owner"].as_str()?.to_string(),     │
│            contents: value["contents"].clone(),             │
│            children: parse_children(value),                 │
│            metadata: value["metadata"].clone(),  // ← 直接存储 JSON │
│          };                                                  │
│          self.blocks.insert(block.block_id.clone(), block); │
│        }                                                     │
│      }                                                       │
│    }                                                         │
│  }                                                           │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  8. 返回给前端                                                │
├─────────────────────────────────────────────────────────────┤
│  events = [Event { ... }]                                   │
│  → 前端收到成功响应                                           │
│  → 可选：监听 "state_changed" 事件刷新 UI                     │
└─────────────────────────────────────────────────────────────┘
```

### 关键要点

1. **时间戳在 Handler 中生成** ✅
   - `created_at` 在 `core.create` 时生成
   - `updated_at` 在 `markdown.write` 等修改操作时更新

2. **所有数据走 Event** ✅
   - ❌ 不在内存中直接修改 Block
   - ✅ 所有修改通过 Event，保证可追溯

3. **StateProjector 是唯一的状态更新入口** ✅
   - 所有内存状态都由重放 Event 产生
   - 不存在"绕过 Event"的修改

---

## 📝 涉及的文件清单

### 后端修改（7 个文件）

| 文件路径 | 修改内容 | 工时 |
|---------|---------|------|
| `src-tauri/src/models/block.rs` | 添加 `metadata` 字段 | 0.5h |
| `src-tauri/src/models/metadata.rs` | 新建：定义 `BlockMetadata` 辅助结构 | 0.5h |
| `src-tauri/src/models/payloads.rs` | 扩展 `CreateBlockPayload` | 0.5h |
| `src-tauri/src/utils/time.rs` | 新建：时间戳工具函数 | 0.5h |
| `src-tauri/src/capabilities/builtins/create.rs` | 生成 metadata（含时间戳） | 1h |
| `src-tauri/src/engine/state.rs` | StateProjector 处理 metadata | 2h |
| `src-tauri/src/lib.rs` | 注册新类型（Specta） | 0.1h |

**后端总工时**：5.1h

### 前端开发（4 个文件）

| 文件路径 | 修改内容 | 工时 |
|---------|---------|------|
| `src/bindings.ts` | 自动生成检查 | 0.1h |
| `src/lib/tauri-client.ts` | 封装带 metadata 的 API | 1h |
| `src/lib/app-store.ts` | 状态管理扩展 | 1.5h |
| `src/components/BlockInfoPanel.tsx` | 显示/编辑 metadata | 3h |

**前端总工时**：5.6h

**总计**：10.7h

---

## 🔧 核心代码实现

### 1. Block 模型修改

```rust
// src-tauri/src/models/block.rs
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Block {
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: serde_json::Value,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,
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
            metadata: serde_json::json!({}),  // ← 默认空对象
        }
    }
}
```

### 2. 时间戳工具

```rust
// src-tauri/src/utils/time.rs
use chrono::Utc;

/// 生成 UTC 时间戳（ISO 8601）
///
/// 格式："2025-12-17T02:30:00Z"
pub fn now_utc() -> String {
    Utc::now().to_rfc3339()
}
```

### 3. core.create Handler 修改

```rust
// src-tauri/src/capabilities/builtins/create.rs
use crate::utils::time::now_utc;

impl CapabilityHandler for CoreCreateCapability {
    fn handler(&self, cmd: &Command, _block: Option<&Block>) -> Result<Vec<Event>, String> {
        let payload: CreateBlockPayload = serde_json::from_value(cmd.payload.clone())
            .map_err(|e| format!("Invalid payload: {}", e))?;

        let block_id = uuid::Uuid::new_v4().to_string();
        let now = now_utc();  // "2025-12-17T02:30:00Z"

        // 合并用户 metadata 和自动时间戳
        let mut metadata = payload.metadata.unwrap_or_else(|| json!({}));
        metadata["created_at"] = json!(now.clone());
        metadata["updated_at"] = json!(now);

        let event = Event::create(
            block_id.clone(),
            "core.create",
            json!({
                "name": payload.name,
                "type": payload.block_type,
                "owner": cmd.editor_id,
                "contents": {},
                "children": {},
                "metadata": metadata  // ← 包含完整元数据
            }),
            &cmd.editor_id,
            1,
        );

        Ok(vec![event])
    }
}
```

### 4. StateProjector 修改

```rust
// src-tauri/src/engine/state.rs
impl StateProjector {
    pub fn apply_event(&mut self, event: &Event) {
        let parts: Vec<&str> = event.attribute.split('/').collect();
        let cap_id = parts.get(1).unwrap_or(&"");

        match *cap_id {
            "core.create" => {
                let value = &event.value;
                let block = Block {
                    block_id: event.entity.clone(),
                    name: value["name"].as_str().unwrap_or("").to_string(),
                    block_type: value["type"].as_str().unwrap_or("").to_string(),
                    owner: value["owner"].as_str().unwrap_or("").to_string(),
                    contents: value["contents"].clone(),
                    children: self.parse_children(value),
                    metadata: value["metadata"].clone(),  // ← 直接存储 JSON
                };
                self.blocks.insert(block.block_id.clone(), block);
            }

            "markdown.write" => {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    // 更新 contents
                    if let Some(contents) = event.value.get("contents") {
                        block.contents = contents.clone();
                    }

                    // 自动更新 updated_at
                    if let Some(obj) = block.metadata.as_object_mut() {
                        obj.insert("updated_at".to_string(), json!(now_utc()));
                    }
                }
            }

            _ => {}
        }
    }
}
```

### 5. 前端 TypeScript 类型（自动生成）

```typescript
// src/bindings.ts（自动生成）
export type Block = {
  block_id: string
  name: string
  block_type: string
  contents: any
  children: Record<string, string[]>
  owner: string
  metadata: any  // ← 新增（JSON）
}

export type BlockMetadata = {
  description?: string | null
  status?: string | null
  created_at?: string | null
  updated_at?: string | null
  [key: string]: any  // 自定义字段
}
```

### 6. 前端使用示例

```typescript
// src/components/BlockInfoPanel.tsx
import { Block, BlockMetadata } from '@/bindings'

function BlockInfoPanel({ block }: { block: Block }) {
  // 解析 metadata
  const metadata = block.metadata as BlockMetadata

  return (
    <div>
      <h3>{block.name}</h3>
      <p>{metadata.description || '无描述'}</p>
      <span>状态: {metadata.status || 'draft'}</span>
      <time>创建于: {metadata.created_at}</time>
      <time>更新于: {metadata.updated_at}</time>
    </div>
  )
}

// 创建 Block
await commands.executeCommand({
  editor_id: activeEditorId,
  cap_id: 'core.create',
  block_id: '',
  payload: {
    name: '需求文档',
    block_type: 'markdown',
    metadata: {
      description: '这是项目需求文档',
      status: 'draft'
    }
  }
})
```

---

## ✅ 方案优势

### 1. 灵活性 ⭐⭐⭐⭐⭐
- ✅ 任何新字段直接加到 `metadata` 中，无需修改 Block 结构
- ✅ status 未确定？没关系，放 metadata 里
- ✅ 未来要加 tags, priority？直接在 metadata 里加

### 2. 统一性 ⭐⭐⭐⭐⭐
- ✅ 所有数据修改走 Event
- ✅ 没有"内存修改 vs Event 修改"的混乱
- ✅ StateProjector 是唯一状态来源

### 3. 时区安全 ⭐⭐⭐⭐⭐
- ✅ ISO 8601 with timezone
- ✅ 存储 UTC，前端按需转换本地时区
- ✅ 跨时区协作无歧义

### 4. 向后兼容 ⭐⭐⭐⭐⭐
- ✅ 旧 Block 没有 metadata → 默认 `{}`
- ✅ StateProjector 重放旧 Event → metadata 为空对象
- ✅ 不需要数据迁移

### 5. 类型安全 ⭐⭐⭐⭐
- ✅ Tauri Specta 自动生成 TypeScript 类型
- ✅ 前端有 BlockMetadata 类型提示
- ⚠️ metadata 是 JSON，需要运行时校验

---

## 🚧 实现步骤

### Phase 1: 后端基础（3h）

1. **添加 metadata 字段**（0.5h）
   - 修改 `models/block.rs`
   - 修改 `Block::new()`

2. **时间戳工具**（0.5h）
   - 创建 `utils/time.rs`
   - 实现 `now_utc()`

3. **扩展 Payload**（0.5h）
   - 修改 `CreateBlockPayload`

4. **修改 core.create Handler**（1h）
   - 生成 metadata（含时间戳）
   - 测试时间戳格式

5. **注册类型**（0.1h）
   - 在 `lib.rs` 中注册 `BlockMetadata`

### Phase 2: StateProjector（2h）

6. **修改 StateProjector**
   - 处理 `metadata` 字段
   - 测试旧 Event 兼容性
   - 测试 `updated_at` 自动更新

### Phase 3: 生成绑定（0.1h）

7. **运行 `cargo run`**
   - 检查 `bindings.ts` 更新
   - 验证类型正确

### Phase 4: 前端集成（5.6h）

8. **TauriClient 封装**（1h）
   - 封装带 metadata 的 API

9. **AppStore 扩展**（1.5h）
   - 状态管理

10. **BlockInfoPanel 组件**（3h）
    - 显示 metadata
    - 编辑 description
    - 显示时间戳

11. **测试**（0.5h）
    - 端到端测试

---

## 📊 对比旧方案

| 维度 | 旧方案（多字段） | 新方案（metadata） |
|------|-----------------|-------------------|
| 灵活性 | ⭐⭐ 固定字段 | ⭐⭐⭐⭐⭐ 灵活 JSON |
| 向后兼容 | ⭐⭐⭐ 需要 Option | ⭐⭐⭐⭐⭐ 自然兼容 |
| 扩展性 | ⭐⭐ 需修改结构 | ⭐⭐⭐⭐⭐ 直接扩展 |
| 类型安全 | ⭐⭐⭐⭐⭐ 编译期 | ⭐⭐⭐ 运行时 |
| 代码量 | ⭐⭐ 多 | ⭐⭐⭐⭐ 少 |
| MVP 适用性 | ⭐⭐⭐ 适用 | ⭐⭐⭐⭐⭐ 完美 |

**结论**：新方案更适合 MVP 阶段的不确定性

---

## 🎯 总结

### 核心决策

1. ✅ **name = title**（不区分）
2. ✅ **添加 metadata 字段**（JSON，灵活）
3. ✅ **时间戳使用 UTC + ISO 8601**
4. ✅ **统一流程**：所有修改走 Event
5. ✅ **status 预留**：放在 metadata 中

### 下一步行动

1. **立即开始**：Phase 1（后端基础，3h）
2. **第二天**：Phase 2（StateProjector，2h）
3. **第三天**：Phase 3-4（前端集成，5.7h）

**总工时**：10.7h

---

**最后更新**: 2025-12-17

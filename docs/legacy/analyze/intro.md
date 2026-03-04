# Elfiee 项目介绍

> Event-sourcing Literate programming Format Integrated Editing Environment

## 目录

- [项目概述](#项目概述)
- [核心架构理念](#核心架构理念)
- [数据格式与存储方案](#数据格式与存储方案)
- [前后端架构](#前后端架构)
- [快速开发指南](#快速开发指南)
- [扩展系统](#扩展系统)

---

## 项目概述

### 什么是 Elfiee?

Elfiee 是一个**基于事件溯源的块编辑器**，使用 `.elf` 文件格式来存储结构化内容。它结合了现代软件工程的最佳实践：

- **事件溯源 (Event Sourcing)** - 所有变更记录为不可变事件流
- **块编辑 (Block-based Editing)** - 内容组织为独立的、可重用的区块
- **能力驱动 (Capability-based)** - 功能通过可扩展的能力系统实现
- **跨平台 (Cross-platform)** - 基于 Tauri 2 构建的原生桌面应用

### 适用场景

✅ **最佳场景**：
- 结构化笔记管理和知识库
- 文学编程 (Literate Programming)
- 项目文档和技术写作
- 需要完整历史追溯的协作场景

❌ **不适合**：
- 实时文本协作编辑 (如 Google Docs)
- 大型二进制文件管理
- 低延迟实时同步

### 技术栈

```
┌─────────────────────────────────────┐
│       Tauri 2 Desktop App           │
├─────────────────────────────────────┤
│  Frontend: React + TypeScript       │
│  - Zustand (状态管理)                │
│  - Tailwind CSS 4 + Shadcn UI       │
│  - Vite (构建工具)                   │
├─────────────────────────────────────┤
│  Backend: Rust                      │
│  - tokio (异步运行时)                │
│  - sqlx (SQLite 数据库)             │
│  - serde (序列化)                    │
│  - tauri-specta (类型生成)          │
└─────────────────────────────────────┘
```

---

## 核心架构理念

### 1. 区块化编辑 (Block-based Editing)

**核心概念**：所有内容组织为独立的区块 (Block)，每个区块拥有：

```rust
pub struct Block {
    pub block_id: String,              // UUID 唯一标识
    pub name: String,                  // 用户可见名称
    pub block_type: String,            // 类型标识 (如 "markdown")
    pub contents: serde_json::Value,   // JSON 内容
    pub children: HashMap<String, Vec<String>>, // 关系图
    pub owner: String,                 // 所有者 editor_id
}
```

**特点**：
- **类型化**：每个区块有明确的类型（Markdown、代码、图表等）
- **可链接**：通过 `children` 字段构建有向图关系
- **独立性**：区块可以被移动、复制、重用
- **所有权**：每个区块有明确的所有者

**示例：Markdown 区块**

```json
{
  "block_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "项目介绍",
  "block_type": "markdown",
  "contents": {
    "markdown": "# Elfiee\n\n这是一个块编辑器..."
  },
  "children": {
    "references": ["另一个区块ID"]
  },
  "owner": "alice"
}
```

### 2. 事件溯源 (Event Sourcing)

**核心概念**：状态不是直接存储的，而是通过重放事件流重建。

#### EAVT 模式

事件使用 **Entity-Attribute-Value-Timestamp** 模式：

```rust
pub struct Event {
    pub event_id: String,              // UUID
    pub entity: String,                // 实体ID (block_id 或 editor_id)
    pub attribute: String,             // "{editor_id}/{cap_id}"
    pub value: serde_json::Value,      // 事件数据
    pub timestamp: HashMap<String, i64>, // 向量时钟
}
```

**字段说明**：

| 字段 | 说明 | 示例 |
|------|------|------|
| `entity` | 被修改的实体 | `"block-123"` |
| `attribute` | 动作描述 | `"alice/markdown.write"` |
| `value` | 变更内容 | `{"contents": {"markdown": "..."}}` |
| `timestamp` | 向量时钟 | `{"alice": 5, "bob": 3}` |

#### 事件示例

**创建区块事件**：
```json
{
  "event_id": "evt-001",
  "entity": "block-123",
  "attribute": "alice/core.create",
  "value": {
    "block_id": "block-123",
    "name": "新区块",
    "block_type": "markdown",
    "contents": {},
    "children": {},
    "owner": "alice"
  },
  "timestamp": {"alice": 1}
}
```

**修改内容事件**：
```json
{
  "event_id": "evt-002",
  "entity": "block-123",
  "attribute": "alice/markdown.write",
  "value": {
    "contents": {
      "markdown": "# 标题\n内容..."
    }
  },
  "timestamp": {"alice": 2}
}
```

#### 优势

- ✅ **完整历史**：所有变更永久记录
- ✅ **审计追踪**：谁在何时做了什么
- ✅ **时间旅行**：可以回溯到任意历史状态
- ✅ **调试利器**：重放事件流定位问题
- ✅ **协作友好**：向量时钟检测并发冲突

### 3. 能力驱动架构 (Capability-based Architecture)

**核心概念**：功能定义为可注册的能力 (Capability)，而不是硬编码的方法。

#### 能力定义

```rust
pub trait CapabilityTrait: Send + Sync {
    fn id(&self) -> &str;           // 能力ID，如 "markdown.write"
    fn target(&self) -> &str;       // 目标类型，如 "markdown"
    fn execute(
        &self,
        cmd: &Command,
        block: Option<&Block>,
        state: &StateProjector,
    ) -> CapResult<Vec<Event>>;
}
```

#### 内置能力

| 能力 ID | 目标 | 功能 |
|---------|------|------|
| `core.create` | `*` | 创建新区块 |
| `core.link` | `*` | 添加区块关系 |
| `core.unlink` | `*` | 移除区块关系 |
| `core.delete` | `*` | 软删除区块 |
| `core.grant` | `*` | 授予权限 |
| `core.revoke` | `*` | 撤销权限 |
| `editor.create` | `editor` | 创建编辑者 |

#### 扩展能力 (Markdown 示例)

| 能力 ID | 目标 | 功能 |
|---------|------|------|
| `markdown.write` | `markdown` | 写入 Markdown 内容 |
| `markdown.read` | `markdown` | 读取 Markdown 内容 |

#### CBAC 授权模型

**基于能力的访问控制 (Capability-Based Access Control)**：

1. **所有者总是授权** - `Block.owner == Command.editor_id`
2. **显式授权** - `GrantsTable` 中存在 `(editor_id, cap_id, block_id)`
3. **通配符授权** - `block_id = "*"` 授予所有区块权限

```rust
// 授权检查逻辑
if block.owner == cmd.editor_id {
    return true;  // 所有者总是授权
}

if grants_table.has_grant(&cmd.editor_id, &cmd.cap_id, &cmd.block_id) {
    return true;  // 显式授权
}

return false;  // 拒绝访问
```

---

## 数据格式与存储方案

### .elf 文件格式

**.elf 文件 = ZIP 归档**

```
example.elf (ZIP 文件)
├── events.db              # SQLite 数据库 - 唯一真相源
├── block-{uuid}/          # 每个区块的资源目录
│   ├── body.md            # Markdown 内容
│   └── assets/            # 图片、附件等
└── _snapshot (未来)        # 缓存的预览文件
```

#### events.db Schema

**SQLite 表结构**：

```sql
CREATE TABLE events (
    event_id TEXT PRIMARY KEY,
    entity TEXT NOT NULL,      -- block_id 或 editor_id
    attribute TEXT NOT NULL,   -- {editor_id}/{cap_id}
    value TEXT NOT NULL,       -- JSON 序列化
    timestamp TEXT NOT NULL    -- 向量时钟 JSON
);

CREATE INDEX idx_entity ON events(entity);
CREATE INDEX idx_attribute ON events(attribute);
```

**数据示例**：

| event_id | entity | attribute | value | timestamp |
|----------|--------|-----------|-------|-----------|
| evt-001 | block-123 | alice/core.create | `{"block_id":"block-123",...}` | `{"alice":1}` |
| evt-002 | block-123 | alice/markdown.write | `{"contents":{"markdown":"..."}}` | `{"alice":2}` |
| evt-003 | alice | system/core.grant | `{"cap_id":"markdown.write","block_id":"*"}` | `{"system":1}` |

#### ZIP 归档生命周期

```rust
// 1. 创建新文件
let archive = ElfArchive::new().await?;

// 2. 追加事件
let pool = archive.event_pool();
append_events(pool, events).await?;

// 3. 保存为 .elf
archive.save("example.elf")?;

// 4. 打开已有文件
let archive = ElfArchive::open("example.elf")?;

// 5. 读取事件
let events = get_all_events(archive.event_pool()).await?;
```

### 状态投影 (State Projection)

**内存中的当前状态**通过重放所有事件构建：

```rust
pub struct StateProjector {
    pub blocks: HashMap<String, Block>,      // 所有区块
    pub editors: HashMap<String, Editor>,    // 所有编辑者
    pub grants: GrantsTable,                 // 权限表
    pub editor_counts: HashMap<String, u64>, // 向量时钟
}

impl StateProjector {
    pub fn replay(&mut self, events: Vec<Event>) {
        for event in events {
            match event.attribute.split('/').collect::<Vec<_>>()[1] {
                "core.create" => self.apply_create(&event),
                "markdown.write" => self.apply_write(&event),
                "core.grant" => self.apply_grant(&event),
                // ...
            }
        }
    }
}
```

**冷启动流程**：
1. 打开 `.elf` 文件
2. 读取 `events.db` 的所有事件
3. 按 `timestamp` 排序
4. 依次应用到 `StateProjector`
5. 得到当前完整状态

---

## 前后端架构

### 整体架构

```
┌─────────────────────────────────────────────────────┐
│                   React Frontend                    │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ UI 组件      │  │ Zustand Store│  │ TauriClient│ │
│  └──────┬──────┘  └──────┬───────┘  └──────┬─────┘ │
└─────────┼─────────────────┼──────────────────┼───────┘
          │                 │                  │
          └─────────────────┴──────────────────┘
                            │
                    ┌───────▼────────┐
                    │  Auto-generated │
                    │   TypeScript    │
                    │    Bindings     │
                    │ (src/bindings.ts)│
                    └───────┬─────────┘
                            │ IPC (Tauri)
┌───────────────────────────▼──────────────────────────┐
│                   Rust Backend                       │
│  ┌─────────────┐  ┌───────────────┐  ┌────────────┐│
│  │ AppState    │  │ EngineManager │  │  Commands  ││
│  └──────┬──────┘  └───────┬───────┘  └──────┬─────┘│
│         │                 │                  │      │
│         └─────────────────┴──────────────────┘      │
│                           │                         │
│         ┌─────────────────▼─────────────────┐       │
│         │     ElfileEngineActor (Actor 模型) │       │
│         │  ┌────────────┐  ┌──────────────┐ │       │
│         │  │StateProject│  │CapabilityReg │ │       │
│         │  └────────────┘  └──────────────┘ │       │
│         └─────────────────┬─────────────────┘       │
│                           │                         │
│         ┌─────────────────▼─────────────────┐       │
│         │         EventStore (SQLite)        │       │
│         └─────────────────┬─────────────────┘       │
│                           │                         │
│         ┌─────────────────▼─────────────────┐       │
│         │      ElfArchive (ZIP 文件)         │       │
│         └─────────────────────────────────────┘     │
└─────────────────────────────────────────────────────┘
```

### Actor 模型并发设计

#### 为什么选择 Actor 模型？

**问题**：多个编辑者同时修改同一个 `.elf` 文件，如何避免竞态条件？

**解决方案**：每个 `.elf` 文件对应一个独立的 `ElfileEngineActor`

```rust
pub struct ElfileEngineActor {
    file_id: String,
    event_pool: SqlitePool,           // 异步数据库连接
    state: StateProjector,             // 内存状态
    registry: CapabilityRegistry,      // 能力注册表
    mailbox: mpsc::UnboundedReceiver<EngineMessage>,  // 消息邮箱
}
```

#### 并发保证

- **同一文件内串行** - Actor 单线程处理邮箱中的命令
- **不同文件并发** - 多个 Actor 可以并行运行
- **无锁设计** - 没有 `Mutex` 或 `RwLock` 在热路径上
- **确定性顺序** - 事件顺序由邮箱保证

#### EngineManager 多文件管理

```rust
pub struct EngineManager {
    engines: Arc<DashMap<String, EngineHandle>>,
}

pub struct EngineHandle {
    file_id: String,
    sender: mpsc::UnboundedSender<EngineMessage>,  // 发送消息到 Actor
}

// 使用示例
let manager = EngineManager::new();
let handle = manager.create_engine("file-1").await?;

// 发送命令到 Actor
handle.process_command(cmd).await?;
```

### 命令处理流程

```mermaid
sequenceDiagram
    participant UI as React UI
    participant Cmd as Tauri Command
    participant Mgr as EngineManager
    participant Actor as ElfileEngineActor
    participant DB as SQLite

    UI->>Cmd: executeCommand(fileId, cmd)
    Cmd->>Mgr: get_engine(fileId)
    Mgr-->>Cmd: EngineHandle
    Cmd->>Actor: send(EngineMessage::Command)

    Note over Actor: 1. 查找 Capability
    Note over Actor: 2. 授权检查
    Note over Actor: 3. 执行 Handler
    Note over Actor: 4. 更新向量时钟
    Note over Actor: 5. 冲突检测

    Actor->>DB: append_events(events)
    DB-->>Actor: 持久化成功

    Note over Actor: 6. 应用到 StateProjector

    Actor-->>Cmd: Result<Vec<Event>, String>
    Cmd-->>UI: JSON Response
```

### 类型安全的 IPC

#### Tauri Specta 工作流

**问题**：手动维护前后端类型容易不一致

**解决方案**：Rust 类型自动生成 TypeScript 类型

```rust
// 1. Rust 定义 (src-tauri/src/models/payloads.rs)
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateBlockPayload {
    pub name: String,
    pub block_type: String,
}

// 2. Tauri 命令 (src-tauri/src/commands/block.rs)
#[tauri::command]
#[specta::specta]
pub async fn execute_command(
    state: State<'_, AppState>,
    file_id: String,
    cmd: Command,
) -> Result<CommandResult<Vec<Event>>, String> {
    // 实现...
}
```

运行 `pnpm tauri dev` 后自动生成：

```typescript
// src/bindings.ts (自动生成，不要手动编辑)
export type CreateBlockPayload = {
    name: string;
    blockType: string;
}

export const commands = {
    executeCommand: (fileId: string, cmd: Command) =>
        invoke<CommandResult<Event[]>>("execute_command", { fileId, cmd })
}
```

#### 前端使用

```typescript
import { commands, type CreateBlockPayload } from '@/bindings'

// TypeScript 编译器自动检查类型
const payload: CreateBlockPayload = {
    name: "新区块",
    blockType: "markdown"
}

const result = await commands.executeCommand(fileId, {
    cmdId: uuidv4(),
    editorId: currentEditor,
    capId: "core.create",
    blockId: "",
    payload: payload,
    timestamp: new Date()
})
```

---

## 快速开发指南

### 后端开发 (Rust)

#### 项目结构

```
src-tauri/src/
├── models/              # 数据模型
│   ├── block.rs         # Block 定义
│   ├── event.rs         # Event 定义
│   ├── command.rs       # Command 定义
│   └── payloads.rs      # 强类型 Payload
├── engine/              # 核心引擎
│   ├── actor.rs         # ElfileEngineActor
│   ├── manager.rs       # EngineManager
│   ├── state.rs         # StateProjector
│   └── event_store.rs   # EventStore
├── capabilities/        # 能力系统
│   ├── registry.rs      # CapabilityRegistry
│   ├── grants.rs        # GrantsTable
│   └── builtins/        # 内置能力
├── extensions/          # 扩展
│   └── markdown/        # Markdown 扩展
├── elf/                 # 文件格式
│   └── archive.rs       # ZIP 处理
├── commands/            # Tauri 命令
│   ├── file.rs          # 文件操作
│   ├── block.rs         # 区块操作
│   └── editor.rs        # 编辑者操作
└── lib.rs               # Crate 根
```

#### 开发工作流

```bash
# 1. 编写代码
vim src-tauri/src/capabilities/builtins/create.rs

# 2. 运行测试 (TDD)
cd src-tauri
cargo test

# 3. 代码检查
cargo clippy

# 4. 格式化
cargo fmt

# 5. 构建
cargo build --release
```

#### 测试示例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_block() {
        let mut state = StateProjector::new();
        let capability = CoreCreate;

        let cmd = create_command(
            "test-editor",
            "core.create",
            "",
            json!({
                "name": "Test Block",
                "block_type": "markdown"
            }),
        );

        let events = capability.execute(&cmd, None, &state).unwrap();
        assert_eq!(events.len(), 1);

        state.replay(events);
        assert_eq!(state.blocks.len(), 1);
    }
}
```

### 前端开发 (React)

#### 项目结构

```
src/
├── components/          # UI 组件
│   ├── Toolbar.tsx      # 工具栏
│   ├── BlockList.tsx    # 区块列表
│   ├── BlockEditor.tsx  # 区块编辑器
│   └── ui/              # Shadcn UI 组件
├── lib/                 # 工具库
│   ├── tauri-client.ts  # Tauri 客户端包装
│   ├── app-store.ts     # Zustand Store
│   └── utils.ts         # 工具函数
├── App.tsx              # 主应用
├── main.tsx             # 入口点
└── bindings.ts          # 自动生成 (不要手动编辑)
```

#### 开发工作流

```bash
# 1. 启动开发服务器
pnpm tauri dev

# 2. 编辑组件
vim src/components/BlockList.tsx

# 3. 热重载自动生效

# 4. 类型检查
pnpm run type-check

# 5. 构建
pnpm run build
```

#### Zustand Store 示例

```typescript
// src/lib/app-store.ts
import { create } from 'zustand'

interface AppState {
  currentFileId: string | null
  currentEditorId: string
  blocks: Block[]

  setCurrentFile: (fileId: string) => void
  addBlock: (block: Block) => void
}

export const useAppStore = create<AppState>((set) => ({
  currentFileId: null,
  currentEditorId: 'default-editor',
  blocks: [],

  setCurrentFile: (fileId) => set({ currentFileId: fileId }),
  addBlock: (block) => set((state) => ({
    blocks: [...state.blocks, block]
  })),
}))
```

#### 组件示例

```typescript
// src/components/BlockList.tsx
import { useAppStore } from '@/lib/app-store'
import { BlockOperations } from '@/lib/tauri-client'

export function BlockList() {
  const { blocks, currentFileId } = useAppStore()

  const handleCreateBlock = async () => {
    if (!currentFileId) return

    await BlockOperations.createBlock(
      currentFileId,
      "New Block",
      "markdown"
    )
  }

  return (
    <div>
      <button onClick={handleCreateBlock}>Create Block</button>
      {blocks.map(block => (
        <BlockItem key={block.blockId} block={block} />
      ))}
    </div>
  )
}
```

---

## 扩展系统

### 创建扩展的完整流程

#### 1. 创建扩展目录

```bash
mkdir -p src-tauri/src/extensions/my_extension
cd src-tauri/src/extensions/my_extension
```

#### 2. 定义 Payload

```rust
// mod.rs
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MyExtensionPayload {
    pub data: String,
}
```

#### 3. 实现能力

```rust
// my_capability.rs
use crate::capabilities::core::*;
use crate::models::*;
use super::MyExtensionPayload;

pub struct MyExtensionWrite;

impl CapabilityTrait for MyExtensionWrite {
    fn id(&self) -> &str {
        "my_extension.write"
    }

    fn target(&self) -> &str {
        "my_extension"
    }

    fn execute(
        &self,
        cmd: &Command,
        block: Option<&Block>,
        _state: &StateProjector,
    ) -> CapResult<Vec<Event>> {
        // 1. 解析 Payload
        let payload: MyExtensionPayload =
            serde_json::from_value(cmd.payload.clone())
                .map_err(|e| format!("Invalid payload: {}", e))?;

        // 2. 验证区块存在
        let block = block.ok_or("Block not found")?;

        // 3. 验证类型
        if block.block_type != self.target() {
            return Err("Block type mismatch".into());
        }

        // 4. 生成事件
        let event = create_event(
            block.block_id.clone(),
            self.id(),
            json!({
                "contents": {
                    "data": payload.data
                }
            }),
            &cmd.editor_id,
            1,
        );

        Ok(vec![event])
    }
}
```

#### 4. 添加测试

```rust
// mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_extension_write() {
        let mut state = StateProjector::new();

        // 创建测试区块
        let block_id = "test-block";
        state.blocks.insert(
            block_id.to_string(),
            Block {
                block_id: block_id.to_string(),
                name: "Test".to_string(),
                block_type: "my_extension".to_string(),
                contents: json!({}),
                children: HashMap::new(),
                owner: "alice".to_string(),
            },
        );

        // 执行能力
        let capability = MyExtensionWrite;
        let cmd = create_command(
            "alice",
            "my_extension.write",
            block_id,
            json!({ "data": "Hello World" }),
        );

        let events = capability.execute(&cmd, state.blocks.get(block_id), &state).unwrap();
        assert_eq!(events.len(), 1);

        // 应用事件
        state.replay(events);
        let block = state.blocks.get(block_id).unwrap();
        assert_eq!(
            block.contents["data"].as_str().unwrap(),
            "Hello World"
        );
    }
}
```

#### 5. 注册到 CapabilityRegistry

```rust
// src-tauri/src/capabilities/registry.rs
impl CapabilityRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            capabilities: HashMap::new(),
        };

        // 注册内置能力
        registry.register_builtins();

        // 注册扩展
        registry.register_extensions();

        registry
    }

    fn register_extensions(&mut self) {
        use crate::extensions::markdown::*;
        use crate::extensions::my_extension::*;  // 添加这行

        self.register(Arc::new(MarkdownWrite));
        self.register(Arc::new(MarkdownRead));
        self.register(Arc::new(MyExtensionWrite));  // 添加这行
    }
}
```

#### 6. 运行并测试

```bash
cd src-tauri
cargo test my_extension
cargo build

cd ..
pnpm tauri dev  # 自动生成 TypeScript 类型
```

#### 7. 前端使用

```typescript
import { type MyExtensionPayload } from '@/bindings'

const payload: MyExtensionPayload = {
    data: "Hello from frontend"
}

await commands.executeCommand(fileId, {
    cmdId: uuidv4(),
    editorId: currentEditor,
    capId: "my_extension.write",
    blockId: blockId,
    payload: payload,
    timestamp: new Date()
})
```

### 扩展最佳实践

1. **强类型 Payload** - 总是定义 `#[derive(Type)]` 的 Payload 结构体
2. **完整测试** - 至少包括：基本功能、输入验证、授权场景
3. **错误处理** - 使用 `CapResult<T>` 返回详细错误信息
4. **文档注释** - 每个 public 函数都应有文档注释
5. **遵循命名** - 能力 ID 使用 `extension.action` 格式

---

## 性能特征

### 基准测试结果

| 指标 | 值 | 说明 |
|------|-----|------|
| **命令吞吐量** | ~3,700 cmd/s | 单文件并发处理 |
| **命令延迟** | ~270μs | 包括数据库写入 |
| **内存占用** | 1-2MB/文件 | 1000 个区块 |
| **文件大小** | 50-100KB | 100 个区块 + 事件 |
| **启动时间** | <1s | 冷启动加载 1000 事件 |

### 扩展性

- **文件数量** - 无理论上限（受 OS 限制）
- **区块数量** - 单文件建议 <10,000 区块
- **事件数量** - 单文件建议 <100,000 事件
- **并发编辑者** - 支持任意数量（Actor 模型串行化）

---

## 总结

Elfiee 是一个设计精良、架构清晰的块编辑器项目，它结合了事件溯源、Actor 模型和能力驱动架构的最佳实践。通过 Tauri 2 框架，它实现了真正的跨平台原生桌面应用，并通过 Tauri Specta 确保了端到端的类型安全。

**关键特性**：
- ✅ 完整的事件溯源实现
- ✅ 类型安全的 IPC
- ✅ 可扩展的能力系统
- ✅ 60 个测试 100% 通过
- ✅ 详细的文档（1000+ 行）

**下一步**：
- 阅读 [`quickstart.md`](./quickstart.md) 快速上手
- 查看 [`detail.md`](./detail.md) 了解文件详情
- 参考 [`plan.md`](./plan.md) 了解未来规划

---

**上次更新**：2025-10-28
**项目状态**：MVP 100% 完成

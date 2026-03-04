# Block 数据结构设计与扩展方案

## 文档信息

- **文档版本**: 1.0
- **创建日期**: 2025-12-16
- **目标**: P0 任务 - Block 数据结构修改及后端引擎适配
- **预计工时**: 17 人时（后端 10h + 前端 7h）

---

## 目录

1. [当前 Block 数据结构](#1-当前-block-数据结构)
2. [引擎流转方式](#2-引擎流转方式)
3. [扩展方案设计](#3-扩展方案设计)
4. [涉及的文件清单](#4-涉及的文件清单)
5. [前端对接方案](#5-前端对接方案)
6. [实现步骤](#6-实现步骤)
7. [测试验证](#7-测试验证)

---

## 1. 当前 Block 数据结构

### 1.1 Rust 数据模型

**文件位置**: `src-tauri/src/models/block.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Block {
    pub block_id: String,                       // UUID 唯一标识
    pub name: String,                           // 显示名称
    pub block_type: String,                     // 类型：markdown, code, diagram
    pub contents: serde_json::Value,            // JSON 动态内容
    pub children: HashMap<String, Vec<String>>, // 关系图：{ "parent_of": ["id1"], "links_to": ["id2"] }
    pub owner: String,                          // 所有者 editor_id
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
        }
    }
}
```

### 1.2 TypeScript 类型（自动生成）

**文件位置**: `src/bindings.ts`（自动生成，不可手动编辑）

```typescript
export type Block = {
  block_id: string
  name: string
  block_type: string
  contents: JsonValue
  children: Record<string, string[]>
  owner: string
}
```

### 1.3 当前的限制

| 限制 | 说明 |
|------|------|
| ❌ 无标题字段 | `name` 是内部名称，不是用户可见的标题 |
| ❌ 无描述字段 | 无法记录 Block 的用途说明 |
| ❌ 无时间戳 | 无法追踪创建和修改时间 |
| ⚠️ owner 存在 | 但前端需要通过 `getEditor()` 查询名字 |

---

## 2. 引擎流转方式

### 2.1 当前架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    前端 (React)                             │
├─────────────────────────────────────────────────────────────┤
│  1. 调用 TauriClient.executeCommand()                       │
│     → invoke('execute_command', { cmd })                   │
└─────────────────────────────────────────────────────────────┘
                         ↓ Tauri IPC
┌─────────────────────────────────────────────────────────────┐
│              Tauri Commands (commands/block.rs)             │
├─────────────────────────────────────────────────────────────┤
│  2. 接收 Command → 转发给 Engine Actor                      │
│     engine_handle.send_command(cmd).await                  │
└─────────────────────────────────────────────────────────────┘
                         ↓ 邮箱 (mpsc channel)
┌─────────────────────────────────────────────────────────────┐
│           ElfileEngineActor (engine/actor.rs)               │
├─────────────────────────────────────────────────────────────┤
│  3. 串行处理命令：                                            │
│     ├─ 加载 Capability Handler                              │
│     ├─ 获取 Block (从 StateProjector)                       │
│     ├─ 授权检查 (CBAC)                                       │
│     ├─ 执行 handler() → 生成 Event[]                        │
│     ├─ 更新 Vector Clock                                    │
│     ├─ 冲突检测 (Vector Clock)                              │
│     ├─ 原子提交到 SQLite                                     │
│     └─ 应用事件到内存状态 (StateProjector)                   │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│          EventStore (SQLite - _eventstore.db)               │
├─────────────────────────────────────────────────────────────┤
│  4. 持久化 Event (EAVT 模式)：                               │
│     ├─ event_id: UUID                                       │
│     ├─ entity: block_id 或 editor_id                        │
│     ├─ attribute: "{editor_id}/{cap_id}"                    │
│     ├─ value: JSON payload                                  │
│     └─ timestamp: Vector Clock                              │
└─────────────────────────────────────────────────────────────┘
                         ↓ 重放 (replay)
┌─────────────────────────────────────────────────────────────┐
│         StateProjector (engine/state.rs)                    │
├─────────────────────────────────────────────────────────────┤
│  5. 内存状态投影：                                            │
│     ├─ blocks: HashMap<block_id, Block>                     │
│     ├─ editors: HashMap<editor_id, Editor>                  │
│     ├─ grants: GrantsTable                                  │
│     └─ editor_counts: HashMap<editor_id, i64>               │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Block 生命周期

#### 阶段 1: 创建 Block

```rust
// 1. 前端发送命令
Command {
    cmd_id: "uuid-1",
    editor_id: "alice",
    cap_id: "core.create",
    block_id: "new-block-123",  // 前端生成
    payload: {
        "name": "我的文档",
        "block_type": "markdown"
    },
    timestamp: { "alice": 1 }
}

// 2. Handler 生成 Event
Event {
    event_id: "event-uuid-1",
    entity: "new-block-123",
    attribute: "alice/core.create",
    value: {
        "name": "我的文档",
        "type": "markdown",
        "owner": "alice",
        "contents": {},
        "children": {}
    },
    timestamp: { "alice": 1 }
}

// 3. StateProjector 应用事件（state.rs:64-95）
match cap_id {
    "core.create" => {
        let block = Block {
            block_id: event.entity.clone(),
            name: obj.get("name").unwrap().as_str().to_string(),
            block_type: obj.get("type").unwrap().as_str().to_string(),
            owner: obj.get("owner").unwrap().as_str().to_string(),
            contents: obj.get("contents").cloned().unwrap_or(json!({})),
            children: obj.get("children")...unwrap_or_default(),
        };
        self.blocks.insert(block.block_id.clone(), block);
    }
}
```

#### 阶段 2: 更新 Block 内容

```rust
// 示例：markdown.write
Event {
    event_id: "event-uuid-2",
    entity: "new-block-123",
    attribute: "alice/markdown.write",
    value: {
        "contents": {
            "content": "# Hello World"
        }
    },
    timestamp: { "alice": 2 }
}

// StateProjector 应用（state.rs:98-117）
_ if cap_id.ends_with(".write") => {
    if let Some(block) = self.blocks.get_mut(&event.entity) {
        // 更新 contents
        if let Some(contents) = event.value.get("contents") {
            if let Some(obj) = block.contents.as_object_mut() {
                for (k, v) in contents.as_object() {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
    }
}
```

### 2.3 关键流程总结

| 步骤 | 组件 | 操作 | 数据流向 |
|------|------|------|---------|
| 1 | 前端 | 构造 Command | → Tauri |
| 2 | Tauri Command | 转发 Command | → Engine Actor |
| 3 | Engine Actor | 授权 + 执行 Handler | → Event[] |
| 4 | Event Store | 持久化 Event | → SQLite |
| 5 | StateProjector | 重放 Event | → 内存 Block |
| 6 | Tauri Command | 返回 Block | → 前端 |

**关键点**:
- ✅ **单一数据源**: Event Store 是唯一真相来源
- ✅ **内存投影**: StateProjector 维护当前状态
- ✅ **原子操作**: Event 提交是原子的（SQLite 事务）
- ✅ **串行处理**: Actor 邮箱保证命令顺序执行

---

## 3. 扩展方案设计

### 3.1 新增字段规划

根据 MVP 需求（kick-off.md 和 04-block-data-structure.md），需要添加：

| 字段 | 类型 | 说明 | 必填 | 默认值 |
|------|------|------|------|--------|
| `title` | `Option<String>` | 用户可见标题 | ❌ | `None` |
| `description` | `Option<String>` | 块描述 | ❌ | `None` |
| `created_at` | `Option<String>` | 创建时间（ISO 8601） | ❌ | 首次 `core.create` 时自动生成 |
| `last_modified` | `Option<String>` | 最后修改时间（ISO 8601） | ❌ | 每次 write 时自动更新 |

**设计理由**:

1. **使用 `Option<T>` 保证向后兼容**
   - 旧的 Event 不包含这些字段时，自动为 `None`
   - 不需要数据迁移脚本

2. **时间戳自动生成**
   - `created_at` 在 `core.create` 事件时由引擎自动设置
   - `last_modified` 在任何 write 事件时自动更新
   - 存储在 Event 的 `value` 中，随 Event 持久化

3. **title vs name 的区别**
   - `name`: 内部名称，系统使用（已存在）
   - `title`: 用户可见标题，UI 显示（新增）
   - 示例: `name = "doc-001"`, `title = "项目需求文档"`

### 3.2 扩展后的 Block 结构

```rust
// src-tauri/src/models/block.rs
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Block {
    // === 原有字段 ===
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: serde_json::Value,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,

    // === 新增字段 ===
    pub title: Option<String>,        // 用户标题
    pub description: Option<String>,  // 描述
    pub created_at: Option<String>,   // 创建时间
    pub last_modified: Option<String>, // 最后修改时间
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
            // 新字段初始为 None，由引擎设置
            title: None,
            description: None,
            created_at: None,
            last_modified: None,
        }
    }
}
```

### 3.3 Event 结构变化

#### 方案 A: 在 `core.create` Event 中包含新字段（推荐）✅

```rust
// core.create Event 的 value
{
    "name": "我的文档",
    "type": "markdown",
    "owner": "alice",
    "contents": {},
    "children": {},
    // 新增字段
    "title": "项目需求文档",        // 可选，前端传入
    "description": "描述项目需求",  // 可选，前端传入
    "created_at": "2025-12-16T10:00:00Z",  // 自动生成
    "last_modified": "2025-12-16T10:00:00Z" // 自动生成
}
```

**优点**:
- ✅ 所有 Block 初始状态在一个 Event 中
- ✅ 符合现有 "create events contain full initial state" 原则
- ✅ 重放简单，一次性构建完整 Block

#### 方案 B: 使用单独的 `core.update_metadata` Event

```rust
// 需要两个 Event
Event 1: core.create (创建基础 Block)
Event 2: core.update_metadata (设置 title, description)
```

**缺点**:
- ❌ 增加复杂度，需要两次命令
- ❌ 不符合现有设计原则

**结论**: 采用**方案 A**

### 3.4 时间戳管理策略

#### 策略 1: 在 Handler 中生成（推荐）✅

```rust
// src-tauri/src/capabilities/builtins/create.rs
fn handle_create(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: CreateBlockPayload = serde_json::from_value(cmd.payload.clone())?;

    // 生成时间戳
    let now = chrono::Utc::now().to_rfc3339();

    let event = create_event(
        block_id.clone(),
        "core.create",
        serde_json::json!({
            "name": payload.name,
            "type": payload.block_type,
            "owner": cmd.editor_id,
            "contents": {},
            "children": {},
            // 新增：自动设置时间戳
            "title": payload.title,  // 从 Payload 获取
            "description": payload.description,  // 从 Payload 获取
            "created_at": now,       // 自动生成
            "last_modified": now     // 自动生成
        }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

#### 策略 2: 在 StateProjector 中设置

```rust
// ❌ 不推荐：时间戳应该持久化到 Event，不应该只在内存中
```

**结论**: 采用**策略 1** - 时间戳必须存储在 Event 中，以便重放时恢复

### 3.5 更新 last_modified 的时机

需要更新 `last_modified` 的操作：

| Capability | 是否更新 last_modified | 理由 |
|-----------|----------------------|------|
| `core.create` | ✅ 是 | 初始设置 |
| `markdown.write` | ✅ 是 | 内容变更 |
| `core.link` | ❌ 否 | 关系变更不算修改 Block 本身 |
| `core.unlink` | ❌ 否 | 关系变更 |
| `core.delete` | ❌ 否 | Block 已删除 |
| `core.update_metadata` | ✅ 是 | 如需实现此能力 |

**实现方式**:

```rust
// 在 StateProjector.apply_event() 中（state.rs）
match cap_id {
    "core.create" => {
        // 从 Event.value 读取时间戳
        let block = Block {
            // ...
            created_at: obj.get("created_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            last_modified: obj.get("last_modified")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };
    }

    _ if cap_id.ends_with(".write") => {
        if let Some(block) = self.blocks.get_mut(&event.entity) {
            // 更新 contents
            // ...

            // 自动更新 last_modified
            if let Some(last_mod) = event.value.get("last_modified") {
                if let Some(time_str) = last_mod.as_str() {
                    block.last_modified = Some(time_str.to_string());
                }
            }
        }
    }
}
```

---

## 4. 涉及的文件清单

### 4.1 必须修改的文件（P0）

| 文件路径 | 修改内容 | 工时 |
|---------|---------|------|
| `src-tauri/src/models/block.rs` | 添加 4 个新字段 | 0.5h |
| `src-tauri/src/models/payloads.rs` | 扩展 `CreateBlockPayload` | 0.5h |
| `src-tauri/src/capabilities/builtins/create.rs` | 在 Event 中包含新字段 | 1h |
| `src-tauri/src/engine/state.rs` | 更新 `apply_event()` 逻辑 | 2h |
| `src-tauri/src/lib.rs` | 确保类型已注册（Specta） | 0.5h |
| **测试文件** | 添加单元测试 | 2h |
| **运行** `cargo run` | 生成 `src/bindings.ts` | 0.1h |
| **小计** | | **6.6h** |

### 4.2 可选修改的文件（P1）

| 文件路径 | 修改内容 | 工时 |
|---------|---------|------|
| `src-tauri/src/capabilities/builtins/update_metadata.rs` | 新增 `core.update_metadata` 能力 | 2h |
| `src-tauri/src/extensions/markdown/markdown_write.rs` | 在 Event 中自动设置 `last_modified` | 1h |
| **测试文件** | 添加单元测试 | 1.5h |
| **小计** | | **4.5h** |

### 4.3 前端文件（P0）

| 文件路径 | 修改内容 | 工时 |
|---------|---------|------|
| `src/bindings.ts` | 自动生成，检查类型 | 0.1h |
| `src/lib/tauri-client.ts` | 封装 Block 查询接口 | 1h |
| `src/lib/app-store.ts` | 添加 Block 元数据管理 | 1.5h |
| `src/components/info/BlockInfoPanel.tsx` | 实现 Info 面板 | 3h |
| **测试** | 组件测试 | 1.5h |
| **小计** | | **7.1h** |

**总工时**: 6.6h（后端必须） + 4.5h（后端可选） + 7.1h（前端） = **18.2h**

---

## 5. 前端对接方案

### 5.1 TauriClient 封装

**文件位置**: `src/lib/tauri-client.ts`

```typescript
import { commands } from '@/bindings'
import type { Block, Command, Event } from '@/bindings'

export class TauriClient {
  // ========== Block 查询 ==========

  /**
   * 获取单个 Block（包含新字段）
   */
  static async getBlock(fileId: string, blockId: string): Promise<Block> {
    const result = await commands.getBlock(fileId, blockId)
    if (result.status === 'ok') {
      return result.data
    }
    throw new Error(result.error)
  }

  /**
   * 获取所有 Blocks
   */
  static async getAllBlocks(fileId: string): Promise<Block[]> {
    const result = await commands.getAllBlocks(fileId)
    if (result.status === 'ok') {
      return result.data
    }
    throw new Error(result.error)
  }

  // ========== Block 创建（含新字段）==========

  /**
   * 创建 Block（支持 title 和 description）
   */
  static async createBlock(
    fileId: string,
    params: {
      name: string
      blockType: string
      title?: string
      description?: string
    },
    editorId: string
  ): Promise<Event[]> {
    const blockId = crypto.randomUUID()
    const cmd: Command = {
      cmd_id: crypto.randomUUID(),
      editor_id: editorId,
      cap_id: 'core.create',
      block_id: blockId,
      payload: {
        name: params.name,
        block_type: params.blockType,
        title: params.title,           // 新增
        description: params.description // 新增
      },
      timestamp: await this.getCurrentTimestamp(fileId, editorId)
    }

    const result = await commands.executeCommand(fileId, cmd)
    if (result.status === 'ok') {
      return result.data
    }
    throw new Error(result.error)
  }

  // ========== Block 元数据更新（可选）==========

  /**
   * 更新 Block 元数据（如果实现了 core.update_metadata）
   */
  static async updateMetadata(
    fileId: string,
    blockId: string,
    metadata: {
      title?: string
      description?: string
    },
    editorId: string
  ): Promise<Event[]> {
    const cmd: Command = {
      cmd_id: crypto.randomUUID(),
      editor_id: editorId,
      cap_id: 'core.update_metadata',
      block_id: blockId,
      payload: metadata,
      timestamp: await this.getCurrentTimestamp(fileId, editorId)
    }

    const result = await commands.executeCommand(fileId, cmd)
    if (result.status === 'ok') {
      return result.data
    }
    throw new Error(result.error)
  }

  // ========== 辅助方法 ==========

  /**
   * 获取当前 Vector Clock
   */
  private static async getCurrentTimestamp(
    fileId: string,
    editorId: string
  ): Promise<Record<string, number>> {
    // 从 AppStore 获取当前 timestamp
    // 实现略
    return { [editorId]: 1 }
  }
}
```

### 5.2 AppStore 状态管理

**文件位置**: `src/lib/app-store.ts`

```typescript
import { create } from 'zustand'
import { TauriClient } from './tauri-client'
import type { Block, Editor } from '@/bindings'

interface AppStore {
  // ========== 状态 ==========
  blocks: Map<string, Block[]>  // fileId → Block[]
  editors: Map<string, Map<string, Editor>>  // fileId → (editorId → Editor)

  // ========== Block 操作 ==========

  /**
   * 加载文件的所有 Blocks
   */
  loadBlocks: (fileId: string) => Promise<void>

  /**
   * 获取单个 Block 的元数据信息
   */
  getBlockInfo: (fileId: string, blockId: string) => {
    title: string
    description: string
    ownerName: string
    createdAt: string | null
    lastModified: string | null
  } | null

  /**
   * 更新 Block 元数据
   */
  updateBlockMetadata: (
    fileId: string,
    blockId: string,
    metadata: {
      title?: string
      description?: string
    }
  ) => Promise<void>
}

export const useAppStore = create<AppStore>((set, get) => ({
  blocks: new Map(),
  editors: new Map(),

  // ========== 实现 ==========

  loadBlocks: async (fileId: string) => {
    const blocks = await TauriClient.getAllBlocks(fileId)
    set((state) => {
      state.blocks.set(fileId, blocks)
      return { blocks: new Map(state.blocks) }
    })
  },

  getBlockInfo: (fileId: string, blockId: string) => {
    const blocks = get().blocks.get(fileId)
    const block = blocks?.find(b => b.block_id === blockId)

    if (!block) return null

    // 查询 owner 名字
    const editors = get().editors.get(fileId)
    const owner = editors?.get(block.owner)

    return {
      title: block.title || block.name,  // fallback to name
      description: block.description || '暂无描述',
      ownerName: owner?.name || block.owner,
      createdAt: block.created_at || null,
      lastModified: block.last_modified || null,
    }
  },

  updateBlockMetadata: async (
    fileId: string,
    blockId: string,
    metadata: { title?: string; description?: string }
  ) => {
    const activeEditor = get().getActiveEditor(fileId)
    if (!activeEditor) {
      throw new Error('No active editor')
    }

    await TauriClient.updateMetadata(
      fileId,
      blockId,
      metadata,
      activeEditor.editor_id
    )

    // 刷新 blocks
    await get().loadBlocks(fileId)
  },
}))
```

### 5.3 BlockInfoPanel 组件

**文件位置**: `src/components/info/BlockInfoPanel.tsx`

```typescript
import { useState } from 'react'
import { useAppStore } from '@/lib/app-store'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Calendar, User, Edit2, Check, X } from 'lucide-react'

interface BlockInfoPanelProps {
  fileId: string
  blockId: string
}

export const BlockInfoPanel = ({ fileId, blockId }: BlockInfoPanelProps) => {
  const { getBlockInfo, updateBlockMetadata } = useAppStore()
  const info = getBlockInfo(fileId, blockId)

  const [isEditingTitle, setIsEditingTitle] = useState(false)
  const [isEditingDescription, setIsEditingDescription] = useState(false)
  const [title, setTitle] = useState(info?.title || '')
  const [description, setDescription] = useState(info?.description || '')

  if (!info) {
    return <div>Block not found</div>
  }

  const handleSaveTitle = async () => {
    await updateBlockMetadata(fileId, blockId, { title })
    setIsEditingTitle(false)
  }

  const handleSaveDescription = async () => {
    await updateBlockMetadata(fileId, blockId, { description })
    setIsEditingDescription(false)
  }

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Edit2 className="w-5 h-5" />
          Block 信息
        </CardTitle>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* 标题 */}
        <div className="space-y-2">
          <label className="text-sm font-medium">标题</label>
          {isEditingTitle ? (
            <div className="flex gap-2">
              <Input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleSaveTitle()
                  if (e.key === 'Escape') setIsEditingTitle(false)
                }}
                autoFocus
              />
              <Button size="sm" onClick={handleSaveTitle}>
                <Check className="w-4 h-4" />
              </Button>
              <Button
                size="sm"
                variant="outline"
                onClick={() => setIsEditingTitle(false)}
              >
                <X className="w-4 h-4" />
              </Button>
            </div>
          ) : (
            <div
              className="p-2 border rounded cursor-pointer hover:bg-accent"
              onClick={() => {
                setTitle(info.title)
                setIsEditingTitle(true)
              }}
            >
              {info.title}
            </div>
          )}
        </div>

        {/* 描述 */}
        <div className="space-y-2">
          <label className="text-sm font-medium">描述</label>
          {isEditingDescription ? (
            <div className="space-y-2">
              <Textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                rows={3}
                autoFocus
              />
              <div className="flex gap-2">
                <Button size="sm" onClick={handleSaveDescription}>
                  <Check className="w-4 h-4" /> 保存
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => setIsEditingDescription(false)}
                >
                  <X className="w-4 h-4" /> 取消
                </Button>
              </div>
            </div>
          ) : (
            <div
              className="p-2 border rounded cursor-pointer hover:bg-accent min-h-[60px]"
              onClick={() => {
                setDescription(info.description)
                setIsEditingDescription(true)
              }}
            >
              {info.description}
            </div>
          )}
        </div>

        {/* 所有者（只读）*/}
        <div className="space-y-2">
          <label className="text-sm font-medium flex items-center gap-2">
            <User className="w-4 h-4" />
            所有者
          </label>
          <div className="p-2 bg-muted rounded">
            {info.ownerName}
          </div>
        </div>

        {/* 时间信息（只读）*/}
        <div className="space-y-2">
          <label className="text-sm font-medium flex items-center gap-2">
            <Calendar className="w-4 h-4" />
            时间信息
          </label>
          <div className="space-y-1 text-sm">
            <div className="flex justify-between">
              <span className="text-muted-foreground">创建时间:</span>
              <span>
                {info.createdAt
                  ? new Date(info.createdAt).toLocaleString('zh-CN')
                  : '未知'}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">最后修改:</span>
              <span>
                {info.lastModified
                  ? new Date(info.lastModified).toLocaleString('zh-CN')
                  : '未知'}
              </span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
```

---

## 6. 实现步骤

### 6.1 后端开发（P0 任务）

#### 步骤 1: 修改 Block 模型（0.5h）

```bash
# 编辑文件
vim src-tauri/src/models/block.rs
```

```rust
// 添加新字段
pub struct Block {
    // ... 原有字段 ...
    pub title: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub last_modified: Option<String>,
}

// 更新 impl Block::new()
impl Block {
    pub fn new(name: String, block_type: String, owner: String) -> Self {
        Self {
            // ... 原有初始化 ...
            title: None,
            description: None,
            created_at: None,
            last_modified: None,
        }
    }
}
```

#### 步骤 2: 扩展 CreateBlockPayload（0.5h）

```bash
vim src-tauri/src/models/payloads.rs
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateBlockPayload {
    pub name: String,
    pub block_type: String,
    // 新增字段
    pub title: Option<String>,
    pub description: Option<String>,
}
```

#### 步骤 3: 更新 core.create Handler（1h）

```bash
vim src-tauri/src/capabilities/builtins/create.rs
```

```rust
fn handle_create(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: CreateBlockPayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for core.create: {}", e))?;

    let block_id = uuid::Uuid::new_v4().to_string();

    // 生成时间戳
    let now = chrono::Utc::now().to_rfc3339();

    let event = create_event(
        block_id.clone(),
        "core.create",
        serde_json::json!({
            "name": payload.name,
            "type": payload.block_type,
            "owner": cmd.editor_id,
            "contents": {},
            "children": {},
            // 新增字段
            "title": payload.title,
            "description": payload.description,
            "created_at": now,
            "last_modified": now,
        }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

**注意**: 需要在 `Cargo.toml` 添加 `chrono` 依赖（如果尚未添加）

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
```

#### 步骤 4: 更新 StateProjector（2h）

```bash
vim src-tauri/src/engine/state.rs
```

```rust
impl StateProjector {
    pub fn apply_event(&mut self, event: &Event) {
        // ... 现有逻辑 ...

        match cap_id {
            "core.create" => {
                if let Some(obj) = event.value.as_object() {
                    let block = Block {
                        block_id: event.entity.clone(),
                        name: obj.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        block_type: obj.get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        owner: obj.get("owner")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        contents: obj.get("contents")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!({})),
                        children: obj.get("children")
                            .and_then(|v| serde_json::from_value(v.clone()).ok())
                            .unwrap_or_default(),

                        // 新增字段
                        title: obj.get("title")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        description: obj.get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        created_at: obj.get("created_at")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        last_modified: obj.get("last_modified")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    };
                    self.blocks.insert(block.block_id.clone(), block);
                }
            }

            // 对于 write 操作，更新 last_modified
            _ if cap_id.ends_with(".write") => {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    // 更新 contents（已有逻辑）
                    if let Some(contents) = event.value.get("contents") {
                        // ... 现有逻辑 ...
                    }

                    // 新增：更新 last_modified
                    if let Some(last_mod) = event.value.get("last_modified") {
                        if let Some(time_str) = last_mod.as_str() {
                            block.last_modified = Some(time_str.to_string());
                        }
                    }
                }
            }

            // ... 其他逻辑 ...
        }
    }
}
```

#### 步骤 5: 确保类型注册（0.5h）

```bash
vim src-tauri/src/lib.rs
```

```rust
#[cfg(debug_assertions)]
fn export_types() -> Result<String, String> {
    // ... 现有代码 ...

    .typ::<models::Block>()  // 已有，会自动包含新字段
    .typ::<models::CreateBlockPayload>()  // 确保已注册

    // ... 生成 bindings ...
}
```

#### 步骤 6: 运行测试和生成绑定（0.1h）

```bash
cd src-tauri

# 运行测试
cargo test

# 运行应用生成 bindings.ts
cargo run
```

检查 `src/bindings.ts` 是否包含新字段：

```typescript
export type Block = {
  // ...
  title?: string
  description?: string
  created_at?: string
  last_modified?: string
}

export type CreateBlockPayload = {
  name: string
  block_type: string
  title?: string
  description?: string
}
```

#### 步骤 7: 编写单元测试（2h）

```bash
vim src-tauri/src/models/tests.rs
```

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_with_new_fields() {
        let block = Block {
            block_id: "test-123".to_string(),
            name: "test".to_string(),
            block_type: "markdown".to_string(),
            contents: json!({}),
            children: HashMap::new(),
            owner: "alice".to_string(),
            title: Some("Test Title".to_string()),
            description: Some("Test Description".to_string()),
            created_at: Some("2025-12-16T10:00:00Z".to_string()),
            last_modified: Some("2025-12-16T10:00:00Z".to_string()),
        };

        assert_eq!(block.title.unwrap(), "Test Title");
        assert_eq!(block.description.unwrap(), "Test Description");
        assert!(block.created_at.is_some());
        assert!(block.last_modified.is_some());
    }

    #[test]
    fn test_create_block_payload_with_optional_fields() {
        let json = json!({
            "name": "My Block",
            "block_type": "markdown",
            "title": "My Title",
            "description": "My Description"
        });

        let payload: CreateBlockPayload = serde_json::from_value(json).unwrap();
        assert_eq!(payload.title.unwrap(), "My Title");
        assert_eq!(payload.description.unwrap(), "My Description");
    }

    #[test]
    fn test_create_block_payload_without_optional_fields() {
        let json = json!({
            "name": "My Block",
            "block_type": "markdown"
        });

        let payload: CreateBlockPayload = serde_json::from_value(json).unwrap();
        assert!(payload.title.is_none());
        assert!(payload.description.is_none());
    }
}
```

```bash
vim src-tauri/src/capabilities/builtins/tests.rs
```

```rust
#[test]
fn test_create_with_title_and_description() {
    let cmd = Command {
        cmd_id: "cmd-1".to_string(),
        editor_id: "alice".to_string(),
        cap_id: "core.create".to_string(),
        block_id: "block-1".to_string(),
        payload: json!({
            "name": "test",
            "block_type": "markdown",
            "title": "Test Title",
            "description": "Test Description"
        }),
        timestamp: HashMap::from([("alice".to_string(), 1)]),
    };

    let events = handle_create(&cmd, None).unwrap();
    assert_eq!(events.len(), 1);

    let event = &events[0];
    assert_eq!(event.value["title"], "Test Title");
    assert_eq!(event.value["description"], "Test Description");
    assert!(event.value["created_at"].is_string());
    assert!(event.value["last_modified"].is_string());
}
```

```bash
vim src-tauri/src/engine/tests.rs
```

```rust
#[test]
fn test_state_projector_applies_create_with_metadata() {
    let mut projector = StateProjector::new();

    let event = Event {
        event_id: "e1".to_string(),
        entity: "block-1".to_string(),
        attribute: "alice/core.create".to_string(),
        value: json!({
            "name": "test",
            "type": "markdown",
            "owner": "alice",
            "contents": {},
            "children": {},
            "title": "Test Title",
            "description": "Test Desc",
            "created_at": "2025-12-16T10:00:00Z",
            "last_modified": "2025-12-16T10:00:00Z"
        }),
        timestamp: HashMap::from([("alice".to_string(), 1)]),
    };

    projector.apply_event(&event);

    let block = projector.get_block("block-1").unwrap();
    assert_eq!(block.title.as_ref().unwrap(), "Test Title");
    assert_eq!(block.description.as_ref().unwrap(), "Test Desc");
    assert_eq!(block.created_at.as_ref().unwrap(), "2025-12-16T10:00:00Z");
}
```

### 6.2 前端开发（P0 任务）

#### 步骤 1: 检查 bindings.ts（0.1h）

```bash
# 后端 cargo run 后自动生成
cat src/bindings.ts | grep -A 10 "export type Block"
```

#### 步骤 2: 封装 TauriClient（1h）

参考 [5.1 TauriClient 封装](#51-tauriclient-封装)

#### 步骤 3: 更新 AppStore（1.5h）

参考 [5.2 AppStore 状态管理](#52-appstore-状态管理)

#### 步骤 4: 实现 BlockInfoPanel（3h）

参考 [5.3 BlockInfoPanel 组件](#53-blockinfoPanel-组件)

#### 步骤 5: 集成到主页面（0.5h）

```typescript
// src/pages/EditorPage.tsx
import { BlockInfoPanel } from '@/components/info/BlockInfoPanel'

const EditorPage = () => {
  const [selectedBlockId, setSelectedBlockId] = useState<string | null>(null)

  return (
    <div className="flex">
      {/* 左侧：Block 列表 */}
      <div className="w-1/3">
        {/* ... Block 列表 ... */}
      </div>

      {/* 右侧：Info 面板 */}
      <div className="w-1/3">
        {selectedBlockId && (
          <BlockInfoPanel
            fileId={currentFileId}
            blockId={selectedBlockId}
          />
        )}
      </div>
    </div>
  )
}
```

#### 步骤 6: 测试（1.5h）

```typescript
// src/components/info/__tests__/BlockInfoPanel.test.tsx
import { render, screen, fireEvent } from '@testing-library/react'
import { BlockInfoPanel } from '../BlockInfoPanel'

describe('BlockInfoPanel', () => {
  it('displays block metadata', () => {
    render(<BlockInfoPanel fileId="file-1" blockId="block-1" />)

    expect(screen.getByText('Test Title')).toBeInTheDocument()
    expect(screen.getByText('Test Description')).toBeInTheDocument()
    expect(screen.getByText('Alice')).toBeInTheDocument()
  })

  it('allows editing title', async () => {
    render(<BlockInfoPanel fileId="file-1" blockId="block-1" />)

    const titleDiv = screen.getByText('Test Title')
    fireEvent.click(titleDiv)

    const input = screen.getByRole('textbox')
    fireEvent.change(input, { target: { value: 'New Title' } })
    fireEvent.keyDown(input, { key: 'Enter' })

    // 验证 API 调用
    // expect(mockUpdateMetadata).toHaveBeenCalledWith(...)
  })
})
```

---

## 7. 测试验证

### 7.1 后端测试清单

| 测试项 | 测试方法 | 预期结果 |
|-------|---------|---------|
| Block 创建（无 title） | `cargo test test_create_without_title` | ✅ title 为 None |
| Block 创建（有 title） | `cargo test test_create_with_title` | ✅ title 正确设置 |
| created_at 自动生成 | `cargo test test_created_at_auto` | ✅ 时间戳格式正确 |
| last_modified 自动生成 | `cargo test test_last_modified_auto` | ✅ 时间戳正确 |
| StateProjector 重放 | `cargo test test_replay_with_metadata` | ✅ Block 包含所有字段 |
| 向后兼容 | `cargo test test_backward_compat` | ✅ 旧 Event 可解析 |

### 7.2 前端测试清单

| 测试项 | 测试方法 | 预期结果 |
|-------|---------|---------|
| 显示 Block 信息 | 渲染 BlockInfoPanel | ✅ 显示 title, description, owner |
| 编辑标题 | 点击标题 → 编辑 → Enter | ✅ 标题更新 |
| 编辑描述 | 点击描述 → 编辑 → 保存 | ✅ 描述更新 |
| 时间格式化 | 查看创建时间 | ✅ 本地化时间显示 |
| 所有者名字 | 查看所有者 | ✅ 显示名字而非 UUID |

### 7.3 集成测试

```bash
# 1. 启动 Tauri 开发环境
pnpm tauri dev

# 2. 测试流程
# - 创建新文件
# - 创建 Block（带 title 和 description）
# - 查看 Info 面板
# - 编辑 title 和 description
# - 关闭文件并重新打开
# - 验证数据持久化
```

---

## 附录 A: 常见问题

### Q1: 为什么不直接修改 Event Store 的表结构？

**A**: Event Sourcing 的核心原则是 **Event 不可变**。修改表结构会破坏现有 Event 的兼容性。使用 `Option<T>` 可以：
- ✅ 向后兼容：旧 Event 自动解析为 `None`
- ✅ 无需迁移：不需要修改现有数据
- ✅ 符合原则：新信息通过新 Event 添加

### Q2: created_at 和 last_modified 应该存储在哪里？

**A**: 存储在 **Event 的 value 中**，理由：
- ✅ Event Store 是唯一真相来源
- ✅ 重放 Event 可恢复完整状态
- ✅ 时间戳随 Block 状态持久化

❌ **不应该**只在 StateProjector 内存中生成，因为：
- ❌ 重启应用后时间戳会变化
- ❌ 不符合 Event Sourcing 原则

### Q3: 如何处理旧的 .elf 文件？

**A**: **自动兼容**，无需手动迁移：
- 旧 Event 解析时，新字段自动为 `None`
- UI 显示 `None` 时可以 fallback 到其他值
- 用户编辑时会生成包含新字段的 Event

```rust
// 旧 Event（没有 title）
{
  "name": "doc",
  "type": "markdown",
  "owner": "alice",
  ...
}

// StateProjector 解析后
Block {
  title: None,  // ← 自动为 None
  description: None,
  created_at: None,
  last_modified: None,
}

// UI 显示
title: block.title || block.name  // fallback
```

### Q4: 需要实现 core.update_metadata 吗？

**A**: **P0 阶段可选，P1 阶段推荐**

- P0 方案：创建 Block 时设置 title/description，后续通过 `markdown.write` 等修改内容
- P1 方案：实现 `core.update_metadata` 专门用于更新元数据

P1 方案的优点：
- ✅ 语义清晰：专门用于元数据更新
- ✅ 事件分离：metadata 变更和 content 变更分开
- ✅ 权限控制：可单独授权 metadata 编辑权限

---

## 附录 B: 参考链接

- [Block 数据结构迁移方案](../migration/04-block-data-structure.md)
- [Event Sourcing 概念](../../elfiee/docs/concepts/ARCHITECTURE_OVERVIEW.md)
- [StateProjector 实现](../../elfiee/src-tauri/src/engine/state.rs)
- [Tauri Specta 文档](https://github.com/specta-rs/tauri-specta)
- [MVP 用户故事](../../elfiee-mvp-ui/docs/demo/kick-off.md)

---

**文档维护**: 本文档应与代码实现同步更新。如有疑问，请参考源代码注释或联系后端开发团队。

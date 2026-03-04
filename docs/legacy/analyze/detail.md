# Elfiee 详细文件说明

> 每个代码文件的详细内容和作用说明

## 目录

- [项目根目录](#项目根目录)
- [文档目录 (docs/)](#文档目录-docs)
- [后端代码 (src-tauri/src/)](#后端代码-src-taurisrc)
- [前端代码 (src/)](#前端代码-src)
- [配置文件](#配置文件)

---

## 项目根目录

### README.md
**作用**：项目总览和快速入口

**内容**：
- 项目简介和核心特性
- 技术栈说明
- 三大核心理念
- 安装和使用指南
- 文档导航

**位置**：`/README.md`

### CLAUDE.md
**作用**：AI 助手工作指南

**内容**：
- 项目架构概述
- 开发规范和最佳实践
- 重要提醒（如不手动编辑 bindings.ts）
- 文件组织结构
- 常用命令

**位置**：`/CLAUDE.md`

### package.json
**作用**：Node.js 项目配置

**关键依赖**：
```json
{
  "dependencies": {
    "@tauri-apps/api": "^2.3.1",      // Tauri IPC 绑定
    "react": "^18.3.1",               // React 框架
    "zustand": "^5.0.3"               // 状态管理
  },
  "scripts": {
    "dev": "vite",
    "tauri": "tauri"
  }
}
```

**位置**：`/package.json`

---

## 文档目录 (docs/)

### docs/README.md
**作用**：文档索引和导航

**内容**：
- 文档结构说明
- 推荐阅读顺序
- 文档分类（概念、指南、计划）

**位置**：`docs/README.md`

### docs/concepts/ - 核心概念

#### ARCHITECTURE_OVERVIEW.md
**作用**：架构高层设计

**关键内容**：
- 三大核心理念详解
- EAVT 事件模式
- Block/Event/Command 数据模型
- .elf 文件格式说明
- 能力系统概述

**位置**：`docs/concepts/ARCHITECTURE_OVERVIEW.md`
**行数**：约 300 行

#### ENGINE_CONCEPTS.md
**作用**：引擎设计理念

**关键内容**：
- 为什么选择 Actor 模型
- ElfileEngineActor 设计
- StateProjector 状态投影
- 并发和冲突处理
- 性能考虑

**位置**：`docs/concepts/ENGINE_CONCEPTS.md`
**行数**：约 400 行

### docs/guides/ - 开发指南

#### EXTENSION_DEVELOPMENT.md
**作用**：扩展开发完整指南

**关键内容**：
- 创建扩展的完整流程
- **Payload 类型定义**（关键）
- 授权模式和 CBAC 实现
- Markdown 扩展示例
- 测试驱动开发
- 常见错误和解决方案

**位置**：`docs/guides/EXTENSION_DEVELOPMENT.md`
**行数**：约 500 行
**重要性**：⭐⭐⭐⭐⭐（创建能力前必读）

#### FRONTEND_DEVELOPMENT.md
**作用**：前端开发指南

**关键内容**：
- Tauri Specta v2 工作流
- **能力 Payload 类型**（关键）
- 类型映射 (Rust ↔ TypeScript)
- 常见陷阱和最佳实践
- 不要手动编辑 bindings.ts 的警告

**位置**：`docs/guides/FRONTEND_DEVELOPMENT.md`
**行数**：约 350 行
**重要性**：⭐⭐⭐⭐⭐（前端开发必读）

### docs/plans/ - 实现计划

#### STATUS.md
**作用**：当前实现状态

**关键内容**：
- MVP 完成情况（100%）
- 测试统计（60 个测试）
- 每个部分的提交哈希
- 已知问题和未来增强

**位置**：`docs/plans/STATUS.md`
**更新频率**：每次重大功能完成后更新

#### IMPLEMENTATION_PLAN.md
**作用**：六部分实现计划

**关键内容**：
- Part 1-6 的详细任务分解
- 每个部分的验收标准
- 依赖关系图
- 时间估算

**位置**：`docs/plans/IMPLEMENTATION_PLAN.md`

#### engine-architecture.md
**作用**：引擎架构详细设计

**关键内容**：
- Actor 模型深入分析
- 消息流和生命周期
- 错误处理策略
- 性能优化技巧

**位置**：`docs/plans/engine-architecture.md`
**行数**：约 600 行

#### part1-6.md 系列
**作用**：每个实现部分的详细指南

| 文件 | 内容 |
|------|------|
| `part1-core-models.md` | 数据模型设计和实现 |
| `part2-event-structure.md` | 事件系统和 EAVT 模式 |
| `part3-elf-file-format.md` | ZIP 归档和文件格式 |
| `part4-extension-interface.md` | 能力系统和 CBAC |
| `part5-elfile-engine.md` | Actor 实现和状态管理 |
| `part6-tauri-app.md` | Tauri 应用和 UI |

---

## 后端代码 (src-tauri/src/)

### 核心入口

#### lib.rs
**作用**：Crate 根和 Tauri Specta 配置

**关键内容**：
```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            crate::commands::file::new_file,
            crate::commands::file::save_file,
            // ...
        ]);

    tauri::Builder::default()
        .setup(move |app| {
            builder.export(
                specta_typescript::Typescript::default(),
                "../src/bindings.ts"
            )?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**位置**：`src-tauri/src/lib.rs`
**行数**：约 100 行

#### main.rs
**作用**：应用入口点

**内容**：
```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    naive_elfiee_lib::run();
}
```

**位置**：`src-tauri/src/main.rs`
**行数**：约 10 行

#### state.rs
**作用**：Tauri 应用全局状态

**关键内容**：
```rust
pub struct AppState {
    pub engine_manager: Arc<EngineManager>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            engine_manager: Arc::new(EngineManager::new()),
        }
    }
}
```

**位置**：`src-tauri/src/state.rs`
**行数**：约 30 行

### models/ - 数据模型

#### block.rs
**作用**：Block 核心数据结构

**关键定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Block {
    #[serde(rename = "blockId")]
    pub block_id: String,              // UUID
    pub name: String,                  // 用户可见名称
    #[serde(rename = "blockType")]
    pub block_type: String,            // 类型标识
    pub contents: serde_json::Value,   // JSON 内容
    pub children: HashMap<String, Vec<String>>, // 关系图
    pub owner: String,                 // 所有者 editor_id
}
```

**位置**：`src-tauri/src/models/block.rs`
**行数**：约 40 行
**测试**：在使用处测试

#### event.rs
**作用**：Event 事件结构

**关键定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Event {
    #[serde(rename = "eventId")]
    pub event_id: String,              // UUID
    pub entity: String,                // block_id 或 editor_id
    pub attribute: String,             // "{editor_id}/{cap_id}"
    pub value: serde_json::Value,      // 事件数据
    pub timestamp: HashMap<String, i64>, // 向量时钟
}
```

**工具函数**：
- `create_event()` - 创建新事件
- `merge_timestamps()` - 合并向量时钟

**位置**：`src-tauri/src/models/event.rs`
**行数**：约 80 行

#### command.rs
**作用**：Command 命令结构

**关键定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Command {
    #[serde(rename = "cmdId")]
    pub cmd_id: String,                // UUID（幂等性）
    #[serde(rename = "editorId")]
    pub editor_id: String,             // 发起者
    #[serde(rename = "capId")]
    pub cap_id: String,                // 能力ID
    #[serde(rename = "blockId")]
    pub block_id: String,              // 目标区块
    pub payload: serde_json::Value,    // 命令参数
    pub timestamp: DateTime<Utc>,      // 时间戳
}
```

**工具函数**：
- `create_command()` - 创建测试命令（仅测试）

**位置**：`src-tauri/src/models/command.rs`
**行数**：约 100 行

#### editor.rs
**作用**：Editor 编辑者结构

**关键定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Editor {
    #[serde(rename = "editorId")]
    pub editor_id: String,             // UUID
    pub name: String,                  // 编辑者名称
}
```

**位置**：`src-tauri/src/models/editor.rs`
**行数**：约 30 行

#### capability.rs
**作用**：Capability trait 定义

**关键内容**：
```rust
pub type CapResult<T> = Result<T, String>;

pub trait CapabilityTrait: Send + Sync {
    fn id(&self) -> &str;           // 能力ID
    fn target(&self) -> &str;       // 目标类型
    fn execute(
        &self,
        cmd: &Command,
        block: Option<&Block>,
        state: &StateProjector,
    ) -> CapResult<Vec<Event>>;
}
```

**位置**：`src-tauri/src/models/capability.rs`
**行数**：约 50 行
**重要性**：⭐⭐⭐⭐⭐（所有能力的基础）

#### grant.rs
**作用**：Grant 授权结构

**关键定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Grant {
    #[serde(rename = "editorId")]
    pub editor_id: String,             // 被授权者
    #[serde(rename = "capId")]
    pub cap_id: String,                // 能力ID
    #[serde(rename = "blockId")]
    pub block_id: String,              // 目标区块（或 "*"）
}
```

**位置**：`src-tauri/src/models/grant.rs`
**行数**：约 30 行

#### payloads.rs
**作用**：所有核心能力的 Payload 类型

**关键定义**：
```rust
// core.create
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateBlockPayload {
    pub name: String,
    #[serde(rename = "blockType")]
    pub block_type: String,
}

// core.link
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LinkPayload {
    #[serde(rename = "targetBlockId")]
    pub target_block_id: String,
    #[serde(rename = "relationType")]
    pub relation_type: String,
}

// core.grant
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GrantPayload {
    #[serde(rename = "targetEditorId")]
    pub target_editor_id: String,
    #[serde(rename = "capId")]
    pub cap_id: String,
    #[serde(rename = "targetBlockId")]
    pub target_block_id: String,
}

// ... 更多
```

**位置**：`src-tauri/src/models/payloads.rs`
**行数**：约 150 行
**重要性**：⭐⭐⭐⭐⭐（类型安全的基础）

### engine/ - 核心引擎

#### actor.rs
**作用**：ElfileEngineActor 实现

**关键结构**：
```rust
pub struct ElfileEngineActor {
    file_id: String,
    event_pool: SqlitePool,           // 异步数据库
    state: StateProjector,             // 内存状态
    registry: CapabilityRegistry,      // 能力注册表
    mailbox: mpsc::UnboundedReceiver<EngineMessage>,
}

pub enum EngineMessage {
    Command {
        cmd: Command,
        reply: oneshot::Sender<Result<Vec<Event>, String>>,
    },
    GetBlock {
        block_id: String,
        reply: oneshot::Sender<Option<Block>>,
    },
    GetAllBlocks {
        reply: oneshot::Sender<Vec<Block>>,
    },
    // ... 更多消息类型
}
```

**主要方法**：
- `spawn()` - 启动 Actor 任务
- `run()` - 消息循环
- `handle_command()` - 处理命令

**处理流程**：
1. 从邮箱接收消息
2. 查找能力 Handler
3. 授权检查（Owner 或 Grant）
4. 执行 Handler
5. 更新向量时钟
6. 冲突检测
7. 异步持久化
8. 应用到 StateProjector
9. 通过 oneshot 返回结果

**位置**：`src-tauri/src/engine/actor.rs`
**行数**：约 400 行
**测试**：7 个测试
**重要性**：⭐⭐⭐⭐⭐（引擎核心）

#### manager.rs
**作用**：EngineManager 多文件管理

**关键结构**：
```rust
pub struct EngineManager {
    engines: Arc<DashMap<String, EngineHandle>>,
}

pub struct EngineHandle {
    file_id: String,
    sender: mpsc::UnboundedSender<EngineMessage>,
}
```

**主要方法**：
- `create_engine()` - 创建新 Actor
- `get_engine()` - 获取已有 Actor
- `close_engine()` - 关闭 Actor

**位置**：`src-tauri/src/engine/manager.rs`
**行数**：约 200 行
**测试**：7 个测试

#### state.rs
**作用**：StateProjector 状态投影

**关键结构**：
```rust
pub struct StateProjector {
    pub blocks: HashMap<String, Block>,
    pub editors: HashMap<String, Editor>,
    pub grants: GrantsTable,
    pub editor_counts: HashMap<String, u64>, // 向量时钟
}
```

**主要方法**：
- `replay()` - 重放事件列表
- `apply_event()` - 应用单个事件
- `apply_create()` - 应用创建事件
- `apply_write()` - 应用写入事件
- `apply_grant()` - 应用授权事件
- `apply_link()` - 应用链接事件

**事件路由**：
```rust
fn apply_event(&mut self, event: &Event) {
    let parts: Vec<&str> = event.attribute.split('/').collect();
    if parts.len() != 2 { return; }

    let cap_id = parts[1];
    match cap_id {
        "core.create" => self.apply_create(event),
        "markdown.write" => self.apply_write(event),
        "core.grant" => self.apply_grant(event),
        // ...
    }
}
```

**位置**：`src-tauri/src/engine/state.rs`
**行数**：约 300 行
**测试**：4 个测试

#### event_store.rs
**作用**：SQLite 事件存储

**关键函数**：
```rust
// 初始化数据库
pub async fn init_event_store(pool: &SqlitePool) -> Result<()>

// 追加事件（单个或批量）
pub async fn append_event(pool: &SqlitePool, event: &Event) -> Result<()>
pub async fn append_events(pool: &SqlitePool, events: &[Event]) -> Result<()>

// 查询事件
pub async fn get_all_events(pool: &SqlitePool) -> Result<Vec<Event>>
pub async fn get_events_by_entity(pool: &SqlitePool, entity: &str) -> Result<Vec<Event>>
```

**SQL Schema**：
```sql
CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY,
    entity TEXT NOT NULL,
    attribute TEXT NOT NULL,
    value TEXT NOT NULL,
    timestamp TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_entity ON events(entity);
CREATE INDEX IF NOT EXISTS idx_attribute ON events(attribute);
```

**位置**：`src-tauri/src/engine/event_store.rs`
**行数**：约 150 行
**测试**：2 个测试

### capabilities/ - 能力系统

#### core.rs
**作用**：能力系统核心工具

**关键函数**：
```rust
// 授权检查
pub fn is_authorized(
    cmd: &Command,
    block: Option<&Block>,
    state: &StateProjector,
) -> bool {
    // 1. Owner 检查
    if let Some(block) = block {
        if block.owner == cmd.editor_id {
            return true;
        }
    }

    // 2. Grant 检查
    state.grants.has_grant(&cmd.editor_id, &cmd.cap_id, &cmd.block_id)
}
```

**位置**：`src-tauri/src/capabilities/core.rs`
**行数**：约 50 行

#### registry.rs
**作用**：CapabilityRegistry 能力注册表

**关键结构**：
```rust
pub struct CapabilityRegistry {
    capabilities: HashMap<String, Arc<dyn CapabilityTrait>>,
}
```

**主要方法**：
- `new()` - 创建并注册所有能力
- `register()` - 注册单个能力
- `get()` - 查找能力
- `register_builtins()` - 注册内置能力
- `register_extensions()` - 注册扩展能力

**位置**：`src-tauri/src/capabilities/registry.rs`
**行数**：约 150 行
**测试**：19 个测试（包含所有能力）

#### grants.rs
**作用**：GrantsTable CBAC 实现

**关键结构**：
```rust
pub struct GrantsTable {
    // Map<editor_id, Vec<(cap_id, block_id)>>
    grants: HashMap<String, Vec<(String, String)>>,
}
```

**主要方法**：
- `add_grant()` - 添加授权
- `remove_grant()` - 移除授权
- `has_grant()` - 检查授权（支持通配符）
- `get_grants_for_editor()` - 查询编辑者所有授权

**通配符支持**：
```rust
pub fn has_grant(&self, editor_id: &str, cap_id: &str, block_id: &str) -> bool {
    if let Some(grants) = self.grants.get(editor_id) {
        for (granted_cap, granted_block) in grants {
            if granted_cap == cap_id {
                if granted_block == "*" || granted_block == block_id {
                    return true;
                }
            }
        }
    }
    false
}
```

**位置**：`src-tauri/src/capabilities/grants.rs`
**行数**：约 100 行
**测试**：6 个测试

#### builtins/ - 内置能力

##### create.rs
**能力**：`core.create`
**功能**：创建新区块

**Payload**：`CreateBlockPayload { name, block_type }`

**逻辑**：
1. 解析 Payload
2. 生成新的 block_id (UUID)
3. 创建完整 Block 结构
4. 生成创建事件
5. 自动授予所有者所有权限（通配符授权）

**位置**：`src-tauri/src/capabilities/builtins/create.rs`
**测试**：3 个测试

##### link.rs / unlink.rs
**能力**：`core.link` / `core.unlink`
**功能**：管理区块关系

**Payload**：`LinkPayload { target_block_id, relation_type }`

**逻辑**：
- 验证源区块和目标区块存在
- 添加/移除 `children[relation_type]` 中的目标 ID
- 生成链接/取消链接事件

**位置**：`src-tauri/src/capabilities/builtins/link.rs`, `unlink.rs`

##### delete.rs
**能力**：`core.delete`
**功能**：软删除区块

**Payload**：空

**逻辑**：
- 验证区块存在
- 生成删除事件（标记 `deleted: true`）
- 不物理删除数据（事件溯源原则）

**位置**：`src-tauri/src/capabilities/builtins/delete.rs`

##### grant.rs / revoke.rs
**能力**：`core.grant` / `core.revoke`
**功能**：权限管理

**Payload**：`GrantPayload { target_editor_id, cap_id, target_block_id }`

**逻辑**：
- 仅所有者可以授予权限
- 支持通配符 block_id = "*"
- 修改 GrantsTable
- 生成授权/撤销事件

**位置**：`src-tauri/src/capabilities/builtins/grant.rs`, `revoke.rs`

##### editor_create.rs
**能力**：`editor.create`
**功能**：创建新编辑者

**Payload**：`EditorCreatePayload { name }`

**位置**：`src-tauri/src/capabilities/builtins/editor_create.rs`

### extensions/ - 扩展系统

#### markdown/mod.rs
**作用**：Markdown 扩展模块定义

**Payload 定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MarkdownWritePayload {
    pub content: String,  // 直接字符串，非嵌套
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MarkdownReadPayload {}  // 空 Payload
```

**位置**：`src-tauri/src/extensions/markdown/mod.rs`

#### markdown/markdown_write.rs
**能力**：`markdown.write`
**功能**：写入 Markdown 内容

**逻辑**：
1. 解析 `MarkdownWritePayload`
2. 验证区块类型为 "markdown"
3. 生成写入事件（更新 `contents.markdown`）

**位置**：`src-tauri/src/extensions/markdown/markdown_write.rs`
**测试**：6 个测试

#### markdown/markdown_read.rs
**能力**：`markdown.read`
**功能**：读取 Markdown 内容

**逻辑**：
1. 验证区块类型
2. 读取 `contents.markdown` 字段
3. 不生成事件（只读操作）

**位置**：`src-tauri/src/extensions/markdown/markdown_read.rs`
**测试**：3 个测试

### elf/ - 文件格式

#### archive.rs
**作用**：.elf ZIP 归档处理

**关键结构**：
```rust
pub struct ElfArchive {
    temp_dir: TempDir,           // 临时目录（自动清理）
    event_pool: SqlitePool,      // 事件数据库连接
}
```

**主要方法**：
- `new()` - 创建新归档（内存）
- `open()` - 打开已有 .elf 文件
- `save()` - 保存到 .elf 文件
- `event_pool()` - 获取数据库连接

**生命周期**：
```rust
// 1. 创建
let archive = ElfArchive::new().await?;

// 2. 操作数据库
let pool = archive.event_pool();
append_events(pool, events).await?;

// 3. 保存
archive.save("example.elf")?;
// TempDir 自动清理

// 4. 打开
let archive = ElfArchive::open("example.elf")?;
let events = get_all_events(archive.event_pool()).await?;
```

**位置**：`src-tauri/src/elf/archive.rs`
**行数**：约 200 行
**测试**：3 个测试

### commands/ - Tauri 命令

#### file.rs
**作用**：文件操作命令

**暴露命令**：
```rust
#[tauri::command]
#[specta::specta]
pub async fn new_file(state: State<'_, AppState>) -> Result<CommandResult<String>, String>

#[tauri::command]
#[specta::specta]
pub async fn open_file(state: State<'_, AppState>, path: String) -> Result<CommandResult<String>, String>

#[tauri::command]
#[specta::specta]
pub async fn save_file(state: State<'_, AppState>, file_id: String, path: String) -> Result<CommandResult<()>, String>
```

**位置**：`src-tauri/src/commands/file.rs`
**行数**：约 150 行

#### block.rs
**作用**：区块操作命令

**暴露命令**：
```rust
#[tauri::command]
#[specta::specta]
pub async fn execute_command(
    state: State<'_, AppState>,
    file_id: String,
    cmd: Command,
) -> Result<CommandResult<Vec<Event>>, String>

#[tauri::command]
#[specta::specta]
pub async fn get_block(
    state: State<'_, AppState>,
    file_id: String,
    block_id: String,
) -> Result<CommandResult<Block>, String>

#[tauri::command]
#[specta::specta]
pub async fn get_all_blocks(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<CommandResult<Vec<Block>>, String>
```

**位置**：`src-tauri/src/commands/block.rs`
**行数**：约 100 行

#### editor.rs
**作用**：编辑者操作命令

**暴露命令**：
```rust
#[tauri::command]
#[specta::specta]
pub async fn get_all_editors(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<CommandResult<Vec<Editor>>, String>
```

**位置**：`src-tauri/src/commands/editor.rs`
**行数**：约 50 行

---

## 前端代码 (src/)

### 入口文件

#### main.tsx
**作用**：React 应用入口

**内容**：
```typescript
import React from "react"
import ReactDOM from "react-dom/client"
import App from "./App"
import "./index.css"

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
```

**位置**：`src/main.tsx`

#### App.tsx
**作用**：主应用组件

**结构**：
```typescript
function App() {
  const [activeTab, setActiveTab] = useState<"blocks" | "events" | "permissions" | "links">("blocks")

  return (
    <div className="app">
      <Toolbar />
      <Tabs>
        <TabsList>
          <TabsTrigger value="blocks">Blocks</TabsTrigger>
          <TabsTrigger value="events">Events</TabsTrigger>
          <TabsTrigger value="permissions">Permissions</TabsTrigger>
          <TabsTrigger value="links">Links</TabsTrigger>
        </TabsList>

        <TabsContent value="blocks"><BlockList /></TabsContent>
        <TabsContent value="events"><EventViewer /></TabsContent>
        <TabsContent value="permissions"><PermissionManager /></TabsContent>
        <TabsContent value="links"><LinkManager /></TabsContent>
      </Tabs>
    </div>
  )
}
```

**位置**：`src/App.tsx`
**行数**：约 150 行

#### bindings.ts
**作用**：自动生成的 TypeScript 类型绑定

**内容示例**：
```typescript
// 自动生成 - 不要手动编辑
export type Block = {
  blockId: string;
  name: string;
  blockType: string;
  contents: Record<string, any>;
  children: Record<string, string[]>;
  owner: string;
}

export type CreateBlockPayload = {
  name: string;
  blockType: string;
}

export const commands = {
  newFile: () => invoke<CommandResult<string>>("new_file"),
  executeCommand: (fileId: string, cmd: Command) =>
    invoke<CommandResult<Event[]>>("execute_command", { fileId, cmd }),
  // ...
}
```

**位置**：`src/bindings.ts`
**生成时机**：运行 `pnpm tauri dev` 时
**重要警告**：⚠️ 不要手动编辑此文件

### lib/ - 工具库

#### tauri-client.ts
**作用**：Tauri 命令包装层

**主要模块**：

**FileOperations**：
```typescript
export const FileOperations = {
  async newFile(): Promise<string> {
    const result = await commands.newFile()
    if (result.status === "ok") {
      return result.data
    }
    throw new Error(result.error)
  },

  async openFile(path: string): Promise<string> {
    // ...
  },

  async saveFile(fileId: string, path: string): Promise<void> {
    // ...
  },
}
```

**BlockOperations**：
```typescript
export const BlockOperations = {
  async createBlock(fileId: string, name: string, blockType: string): Promise<Event[]> {
    const cmd: Command = {
      cmdId: uuidv4(),
      editorId: useAppStore.getState().currentEditorId,
      capId: "core.create",
      blockId: "",
      payload: { name, blockType },
      timestamp: new Date(),
    }
    return executeCommand(fileId, cmd)
  },

  async writeMarkdown(fileId: string, blockId: string, content: string): Promise<Event[]> {
    // ...
  },

  async linkBlocks(/* ... */): Promise<Event[]> {
    // ...
  },

  // ...
}
```

**位置**：`src/lib/tauri-client.ts`
**行数**：约 300 行
**测试**：4 个测试

#### app-store.ts
**作用**：Zustand 全局状态管理

**状态结构**：
```typescript
interface AppState {
  // 文件状态
  currentFileId: string | null
  currentEditorId: string

  // 数据
  blocks: Block[]
  editors: Editor[]
  events: Event[]

  // 操作
  setCurrentFile: (fileId: string) => void
  setCurrentEditor: (editorId: string) => void
  addBlock: (block: Block) => void
  updateBlock: (blockId: string, updates: Partial<Block>) => void
  addEvent: (event: Event) => void
  addEditor: (editor: Editor) => void
  reset: () => void
}

export const useAppStore = create<AppState>((set) => ({
  currentFileId: null,
  currentEditorId: "default-editor",
  blocks: [],
  editors: [],
  events: [],

  setCurrentFile: (fileId) => set({ currentFileId: fileId }),
  // ...
}))
```

**位置**：`src/lib/app-store.ts`
**行数**：约 150 行
**测试**：3 个测试

#### utils.ts
**作用**：通用工具函数

**内容**：
```typescript
// Tailwind CSS class 合并
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
```

**位置**：`src/lib/utils.ts`

#### constants.ts
**作用**：常量定义

**内容**：
```typescript
export const CAPABILITY_IDS = {
  CORE_CREATE: "core.create",
  CORE_LINK: "core.link",
  CORE_DELETE: "core.delete",
  MARKDOWN_WRITE: "markdown.write",
  // ...
} as const

export const BLOCK_TYPES = {
  MARKDOWN: "markdown",
  CODE: "code",
  // ...
} as const
```

**位置**：`src/lib/constants.ts`

### components/ - UI 组件

#### Toolbar.tsx
**作用**：顶部工具栏

**功能**：
- 新建文件按钮
- 打开文件按钮
- 保存文件按钮
- 编辑者选择器

**位置**：`src/components/Toolbar.tsx`
**行数**：约 100 行
**测试**：2 个测试

#### BlockList.tsx
**作用**：区块列表和 CRUD

**功能**：
- 显示所有区块
- 创建新区块表单
- 编辑区块按钮
- 删除区块按钮

**位置**：`src/components/BlockList.tsx`
**行数**：约 200 行
**测试**：2 个测试

#### BlockEditor.tsx
**作用**：区块内容编辑器

**功能**：
- Markdown 内容编辑
- 保存按钮
- 实时预览（未来）

**位置**：`src/components/BlockEditor.tsx`
**行数**：约 150 行

#### EventViewer.tsx
**作用**：事件历史查看器

**功能**：
- 显示所有事件列表
- 事件详情展示
- 时间戳显示

**位置**：`src/components/EventViewer.tsx`
**行数**：约 100 行

#### PermissionManager.tsx
**作用**：权限管理界面

**功能**：
- 授予权限表单
- 撤销权限按钮
- 权限列表显示

**位置**：`src/components/PermissionManager.tsx`
**行数**：约 150 行

#### LinkManager.tsx
**作用**：区块关系管理

**功能**：
- 创建链接表单
- 移除链接按钮
- 关系图可视化（未来）

**位置**：`src/components/LinkManager.tsx`
**行数**：约 150 行

#### EditorSelector.tsx
**作用**：编辑者选择器

**功能**：
- 编辑者下拉列表
- 创建新编辑者
- 切换当前编辑者

**位置**：`src/components/EditorSelector.tsx`
**行数**：约 100 行
**测试**：2 个测试

### components/ui/ - Shadcn UI 组件

预构建的 UI 组件库，包括：

- `button.tsx` - 按钮组件
- `input.tsx` - 输入框
- `select.tsx` - 下拉选择
- `table.tsx` - 表格
- `sonner.tsx`, `toaster.tsx` - Toast 通知

**位置**：`src/components/ui/`
**来源**：Shadcn UI (https://ui.shadcn.com)

### test/ - 测试工具

#### setup.ts
**作用**：Vitest 测试设置

**内容**：
- 全局 beforeEach/afterEach
- Mock 配置

**位置**：`src/test/setup.ts`

#### tauri-mocks.ts
**作用**：Tauri IPC Mock

**内容**：
```typescript
export const mockTauriCommands = {
  newFile: vi.fn().mockResolvedValue({ status: "ok", data: "file-123" }),
  executeCommand: vi.fn().mockResolvedValue({ status: "ok", data: [] }),
  // ...
}
```

**位置**：`src/test/tauri-mocks.ts`

---

## 配置文件

### Rust 配置

#### Cargo.toml
**作用**：Rust 项目配置

**关键依赖**：
```toml
[dependencies]
tauri = { version = "2", features = ["windows7-compat"] }
tauri-specta = "2.0.0-rc.20"
specta = "2.0.0-rc.20"
specta-typescript = "0.0.7"
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
zip = "0.6"
tempfile = "3"
dashmap = "6"
```

**位置**：`src-tauri/Cargo.toml`

#### tauri.conf.json
**作用**：Tauri 应用配置

**关键配置**：
```json
{
  "productName": "Elfiee",
  "version": "0.1.0",
  "identifier": "com.elfiee.app",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420"
  }
}
```

**位置**：`src-tauri/tauri.conf.json`

### 前端配置

#### vite.config.ts
**作用**：Vite 构建配置

**内容**：
```typescript
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
})
```

**位置**：`vite.config.ts`

#### tsconfig.json
**作用**：TypeScript 配置

**关键配置**：
```json
{
  "compilerOptions": {
    "strict": true,
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler"
  }
}
```

**位置**：`tsconfig.json`

#### tailwind.config.js
**作用**：Tailwind CSS 配置

**位置**：`tailwind.config.js`

---

## 文件统计

### 后端代码统计

| 模块 | 文件数 | 总行数 | 测试数 |
|------|--------|--------|--------|
| models/ | 7 | ~600 | - |
| engine/ | 4 | ~1,050 | 13 |
| capabilities/ | 10 | ~1,200 | 25 |
| extensions/ | 3 | ~400 | 9 |
| elf/ | 1 | ~200 | 3 |
| commands/ | 3 | ~300 | - |
| **总计** | **28** | **~3,750** | **60** |

### 前端代码统计

| 模块 | 文件数 | 总行数 | 测试数 |
|------|--------|--------|--------|
| lib/ | 4 | ~600 | 7 |
| components/ | 7 | ~1,000 | 4 |
| components/ui/ | 5 | ~500 | 1 |
| **总计** | **16** | **~2,100** | **12** |

### 文档统计

| 目录 | 文件数 | 总行数 |
|------|--------|--------|
| docs/concepts/ | 2 | ~700 |
| docs/guides/ | 2 | ~850 |
| docs/plans/ | 9 | ~2,000 |
| **总计** | **13** | **~3,550** |

---

## 重要提醒

### 不要手动编辑的文件

⚠️ **src/bindings.ts** - 自动生成，运行 `pnpm tauri dev` 时更新

### 必读文件（开发前）

⭐ **docs/guides/EXTENSION_DEVELOPMENT.md** - 创建能力前必读
⭐ **docs/guides/FRONTEND_DEVELOPMENT.md** - 前端开发必读
⭐ **src-tauri/src/models/payloads.rs** - 强类型 Payload 定义
⭐ **src-tauri/src/capabilities/core.rs** - 授权逻辑

### 关键代码路径

**命令处理流程**：
```
src/lib/tauri-client.ts (前端)
  → src/bindings.ts (自动生成)
  → src-tauri/src/commands/*.rs (Tauri 命令)
  → src-tauri/src/engine/manager.rs (EngineManager)
  → src-tauri/src/engine/actor.rs (ElfileEngineActor)
  → src-tauri/src/capabilities/registry.rs (CapabilityRegistry)
  → src-tauri/src/capabilities/builtins/*.rs (能力 Handler)
  → src-tauri/src/engine/event_store.rs (持久化)
  → src-tauri/src/engine/state.rs (状态投影)
```

---

**上次更新**：2025-10-28
**版本**：MVP 1.0
**总文件数**：约 60 个代码文件 + 13 个文档文件

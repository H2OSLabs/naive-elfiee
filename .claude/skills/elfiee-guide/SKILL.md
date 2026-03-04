---
name: elfiee-guide
description: Elfiee 开发任务分类判断指南。当用户提出开发任务时使用此 skill 来判断任务类别：(1) Engine 核心开发 - 修改事件溯源、Actor 模型、StateProjector 等核心组件；(2) Extension/Capability 开发 - 添加新的区块类型或能力；(3) 前端开发 - React/TypeScript UI 组件和状态管理；(4) Tauri Command 开发 - 新增前后端接口。触发示例："我要添加一个新功能"、"这个任务应该改哪里"、"判断这是什么类型的开发"。
---

# Elfiee 开发任务分类指南

## 概览

Elfiee 是一个基于事件溯源和 Actor 模型的区块编辑器，采用 Tauri 2（Rust 后端 + React 前端）架构。本指南帮助判断开发任务的类别，以便选择正确的开发方式和规范。

## 核心概念速查

- **Block（区块）**：内容的基本单元，具有 `block_id`、`block_type`、`contents`、`children`
- **Event（事件）**：不可变的状态变更记录，是系统的"单一事实来源"
- **Command（命令）**：用户意图的封装，包含 `cap_id`、`block_id`、`payload`
- **Capability（能力）**：定义操作逻辑，包含 `certificator`（授权检查）和 `handler`（执行逻辑）
- **Engine Actor**：每个 .elf 文件对应一个 Actor，串行处理所有 Command

## 任务分类决策树

```
用户提出开发任务
    │
    ├─ 涉及事件处理流程？ ──────────────────────────────> Engine 核心开发
    │   - EventStore 读写
    │   - StateProjector 状态投影
    │   - Actor 消息处理
    │   - 向量时钟/冲突检测
    │
    ├─ 添加新的区块类型或操作？ ────────────────────────> Extension 开发
    │   - 新 block_type（如 image、diagram）
    │   - 新 Capability（如 image.write）
    │   - 新 Payload 类型定义
    │
    ├─ 修改现有能力的逻辑？ ────────────────────────────> Extension 开发
    │   - 修改 capability handler
    │   - 修改 certificator 授权逻辑
    │
    ├─ 添加新的后端接口？ ──────────────────────────────> Tauri Command 开发
    │   - 新增 #[tauri::command] 函数
    │   - 不涉及 Capability 系统
    │   - 直接的后端功能（如文件操作、配置读取）
    │
    ├─ 修改 UI 界面？ ──────────────────────────────────> 前端开发
    │   - React 组件
    │   - Zustand 状态管理
    │   - 样式和布局
    │
    └─ 修改前后端通信？ ────────────────────────────────> 需要前后端协作
        - 后端添加 Command + 类型
        - 前端添加 Action + 调用
```

## 1. Engine 核心开发

**代码位置**：`src-tauri/src/engine/`

**适用场景**：
- 修改事件存储逻辑（`event_store.rs`）
- 修改状态投影逻辑（`state.rs`）
- 修改 Actor 消息处理（`actor.rs`）
- 修改 Engine 管理器（`manager.rs`）
- 涉及向量时钟、冲突检测

**关键文件**：
```
src-tauri/src/engine/
├── mod.rs           # 模块导出
├── actor.rs         # ElfileEngineActor 实现
├── manager.rs       # EngineManager 管理多个 Actor
├── event_store.rs   # SQLite 事件存储
└── state.rs         # StateProjector 状态投影
```

**注意事项**：
- 这是最核心的代码，修改需谨慎
- 所有状态变更必须通过 Event 记录
- 保证 Actor 模型的串行处理特性
- 充分测试并发场景

## 2. Extension/Capability 开发

**代码位置**：`src-tauri/src/extensions/` 和 `src-tauri/src/capabilities/`

**适用场景**：
- 添加新的 `block_type`（如 `image`、`diagram`、`table`）
- 添加新的 `Capability`（如 `image.write`、`diagram.render`）
- 修改现有能力的业务逻辑

**关键文件**：
```
src-tauri/src/extensions/
├── mod.rs                    # 扩展模块导出
├── markdown/                 # Markdown 扩展
│   ├── mod.rs               # Payload 类型定义
│   ├── markdown_write.rs    # markdown.write 能力
│   └── markdown_read.rs     # markdown.read 能力
├── code/                     # Code 扩展
├── directory/                # Directory 扩展
└── terminal/                 # Terminal 扩展

src-tauri/src/capabilities/
├── mod.rs                    # 能力系统模块
├── core.rs                   # 核心接口和工具函数
├── registry.rs               # CapabilityRegistry
├── grants.rs                 # GrantsTable CBAC
└── builtins/                 # 内置能力
    ├── create.rs            # core.create
    ├── delete.rs            # core.delete
    ├── link.rs              # core.link
    ├── grant.rs             # core.grant
    └── ...
```

**开发步骤**：
1. 在 `extensions/{ext_name}/mod.rs` 定义 Payload 类型（添加 `#[derive(Type)]`）
2. 创建 `{ext_name}_{cap}.rs` 实现 capability handler
3. 在 `capabilities/registry.rs` 注册能力
4. 在 `lib.rs` 注册 Payload 类型（debug 和 release 都要）
5. 运行 `pnpm tauri dev` 生成前端类型

**Write vs Read 能力的 Entity 区别**：
- **Write 能力**：`entity = block.block_id`（被修改的块）
- **Read 能力**：`entity = cmd.editor_id`（读取者）

## 3. 前端开发

**代码位置**：`src/`

**适用场景**：
- 创建/修改 React 组件
- 修改 Zustand 状态管理
- 调整 UI 样式和布局
- 使用 Shadcn UI 组件

**关键文件**：
```
src/
├── bindings.ts              # 【自动生成】禁止手动编辑
├── lib/
│   ├── app-store.ts         # Zustand Store（所有 Actions 在这里）
│   ├── tauri-client.ts      # TauriClient（只在 Actions 中使用）
│   └── utils.ts             # 工具函数
├── components/
│   ├── ui/                  # Shadcn UI 组件
│   └── editor/              # 编辑器组件
└── hooks/                   # 自定义 Hooks
```

**三大硬性规则**：
1. **只能通过 Zustand Actions 操作数据** - 组件禁止直接调用 TauriClient
2. **禁止手动编辑 bindings.ts** - 由 tauri-specta 自动生成
3. **不要直接修改状态对象** - 必须通过 Actions

**数据流**：
```
组件 → Zustand Action → TauriClient → Tauri IPC → 后端
  ↑                                                  │
  └────────── State 变化触发重新渲染 ←────────────────┘
```

## 4. Tauri Command 开发

**代码位置**：`src-tauri/src/commands/`

**适用场景**：
- 添加新的后端接口（不涉及 Capability 系统）
- 文件系统操作
- 配置读取/写入
- 直接的后端功能

**关键文件**：
```
src-tauri/src/commands/
├── mod.rs           # 命令模块导出
├── file.rs          # 文件操作命令
├── block.rs         # 块操作命令
├── editor.rs        # 编辑器操作命令
├── event.rs         # 事件操作命令
└── checkout.rs      # 工作区导出命令
```

**开发步骤**：
1. 在 `commands/{module}.rs` 定义函数
2. 添加 `#[tauri::command]` 和 `#[specta]` 宏
3. 在 `lib.rs` 注册（debug 和 release 都要）
4. 运行 `pnpm tauri dev` 生成前端绑定
5. 在 `app-store.ts` 添加对应的 Action

## 判断示例

| 任务描述 | 类别 | 原因 |
|---------|------|------|
| "添加图片块支持" | Extension | 新 block_type + capability |
| "修改事件存储格式" | Engine 核心 | EventStore 修改 |
| "添加文件导出功能" | Tauri Command | 直接后端功能 |
| "修改块编辑器 UI" | 前端 | React 组件修改 |
| "添加用户权限检查" | Extension | Capability certificator |
| "修复状态同步问题" | Engine 核心 | StateProjector 相关 |
| "添加拖拽排序" | 前端 | UI 交互功能 |
| "修改 markdown 渲染" | Extension | markdown 能力修改 |

## 需要协作的场景

当任务涉及**新的数据类型或接口**时，通常需要前后端协作：

1. **后端先行**：
   - 定义数据类型（`#[derive(Type)]`）
   - 添加 Tauri Command 或 Capability
   - 在 lib.rs 注册

2. **生成绑定**：
   - 运行 `pnpm tauri dev`
   - 检查 `bindings.ts` 更新

3. **前端跟进**：
   - 在 `app-store.ts` 添加 Action
   - 组件调用 Action

## 相关规范文档

- **后端开发**：`docs/mvp/guidelines/后端开发规范.md`
- **前端开发**：`docs/mvp/guidelines/前端开发规范.md`
- **完整流程**：`docs/mvp/guidelines/开发流程.md`
- **架构概览**：`docs/concepts/ARCHITECTURE_OVERVIEW.md`
- **引擎设计**：`docs/concepts/ENGINE_CONCEPTS.md`

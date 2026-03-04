---
name: elfiee-be-dev
description: Elfiee 后端开发专家 skill。适用于 Rust/Tauri 后端开发任务，包括：(1) Extension/Capability 开发 - 添加新区块类型和能力；(2) Tauri Command 开发 - 添加前后端接口；(3) Engine 核心修改 - 事件溯源和 Actor 模型。遵循事件溯源架构、CBAC 权限模型和 tauri-specta 类型生成规范。触发示例："添加新的块类型"、"实现一个 capability"、"添加后端接口"。
---

# Elfiee 后端开发指南

## 概览

Elfiee 后端使用 Rust + Tauri 2 构建，核心架构基于事件溯源（Event Sourcing）和 Actor 模型。本指南涵盖 Extension 开发、Tauri Command 开发和 Engine 修改的完整流程。

## 五大硬性规则

1. **所有前端接口必须是 Tauri Command** - 添加 `#[tauri::command]` + `#[specta]` 宏
2. **每个能力必须定义类型化 Payload** - 添加 `#[derive(Serialize, Deserialize, Type)]`
3. **Read vs Write 能力的 Entity 不同** - Write: `block_id`，Read: `editor_id`
4. **通过 Event 记录状态变更** - 使用 `create_event()`，不直接修改状态
5. **CBAC 授权检查** - Owner 总是授权，非 Owner 需要 Grant

## 目录结构

```
src-tauri/src/
├── main.rs                 # Tauri 入口
├── lib.rs                  # 【关键】命令注册 + Specta 配置
├── state.rs                # 全局应用状态
├── models/                 # 数据模型
│   ├── block.rs           # Block 定义
│   ├── command.rs         # Command 定义
│   ├── event.rs           # Event 定义
│   ├── editor.rs          # Editor 定义
│   └── payloads.rs        # Core Payload 类型
├── commands/               # Tauri Commands
│   ├── file.rs            # 文件操作
│   ├── block.rs           # 块操作
│   └── editor.rs          # 编辑器操作
├── capabilities/           # 能力系统
│   ├── core.rs            # 核心接口
│   ├── registry.rs        # CapabilityRegistry
│   ├── grants.rs          # GrantsTable CBAC
│   └── builtins/          # 内置能力
├── extensions/             # 扩展系统
│   ├── markdown/          # Markdown 扩展
│   ├── code/              # Code 扩展
│   ├── directory/         # Directory 扩展
│   └── terminal/          # Terminal 扩展
└── engine/                 # 事件溯源引擎
    ├── actor.rs           # ElfileEngineActor
    ├── manager.rs         # EngineManager
    ├── event_store.rs     # SQLite 事件存储
    └── state.rs           # StateProjector
```

## 1. Extension/Capability 开发

### 1.1 完整开发流程

#### 步骤 1：定义 Payload 类型

```rust
// src/extensions/my_extension/mod.rs
use serde::{Deserialize, Serialize};
use specta::Type;

/// Payload for my_extension.write capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MyExtWritePayload {
    pub content: String,
    pub format: Option<String>,
}

pub mod my_write;
pub use my_write::handle_my_write;
```

#### 步骤 2：实现 Capability Handler

```rust
// src/extensions/my_extension/my_write.rs
use capability_macros::capability;
use crate::capabilities::core::{CapResult, create_event};
use crate::models::{Block, Command, Event};
use super::MyExtWritePayload;
use serde_json::json;

/// Handler for my_extension.write capability
#[capability(id = "my_extension.write", target = "my_block_type")]
fn handle_my_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required")?;

    // 1. 解析 Payload（类型安全）
    let payload: MyExtWritePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload: {}", e))?;

    // 2. 业务逻辑验证
    if payload.content.is_empty() {
        return Err("Content cannot be empty".into());
    }

    // 3. 构建新状态
    let mut new_contents = serde_json::Map::new();
    new_contents.insert("content".to_string(), json!(payload.content));
    if let Some(fmt) = payload.format {
        new_contents.insert("format".to_string(), json!(fmt));
    }

    // 4. 创建事件（Write 能力：entity = block_id）
    let event = create_event(
        block.block_id.clone(),      // Entity: 被修改的块
        cmd.cap_id.as_str(),         // cap_id
        json!(new_contents),         // Value: 新状态
        &cmd.editor_id,              // editor_id
        1,                           // vector clock count
    )?;

    Ok(vec![event])
}
```

#### 步骤 3：注册能力

```rust
// src/capabilities/registry.rs
use crate::extensions::my_extension::handle_my_write;

impl CapabilityRegistry {
    pub fn register_extensions(&mut self) {
        // ... 已有能力 ...
        self.register_capability(handle_my_write());
    }
}
```

#### 步骤 4：在 lib.rs 注册类型

```rust
// src-tauri/src/lib.rs

// Debug 模式
#[cfg(debug_assertions)]
let specta_builder = tauri_specta::Builder::<tauri::Wry>::new()
    .commands(tauri_specta::collect_commands![
        // ... 命令列表 ...
    ])
    .typ::<extensions::my_extension::MyExtWritePayload>()  // 注册类型
    // ...

// Release 模式（必须与 debug 一致）
#[cfg(not(debug_assertions))]
let builder = builder.invoke_handler(tauri::generate_handler![
    // ... 命令列表（必须与 debug 一致）...
]);
```

#### 步骤 5：生成前端绑定

```bash
pnpm tauri dev
# bindings.ts 会自动更新，包含 MyExtWritePayload 类型
```

### 1.2 Read vs Write 能力的区别

**Write 能力（修改状态）**：
```rust
// Entity = block.block_id（被修改的块）
let event = create_event(
    block.block_id.clone(),  // ← Entity
    cmd.cap_id.as_str(),
    json!(new_contents),
    &cmd.editor_id,
    1,
)?;
```

**Read 能力（观察状态）**：
```rust
// Entity = cmd.editor_id（读取者）
let event = create_event(
    cmd.editor_id.clone(),   // ← Entity
    cmd.cap_id.as_str(),
    json!({
        "block_id": block.block_id,
        "data": read_data,
    }),
    &cmd.editor_id,
    1,
)?;
```

### 1.3 CBAC 授权规则

- **Owner 总是授权**：`block.owner == cmd.editor_id` → 直接通过
- **非 Owner 需要 Grant**：检查 `GrantsTable.has_grant(editor_id, cap_id, block_id)`
- **通配符 Grant**：`block_id = "*"` 适用于所有块

## 2. Tauri Command 开发

### 2.1 定义命令

```rust
// src/commands/my_module.rs
use tauri::State;
use crate::state::AppState;

/// 获取某个数据
#[tauri::command]
#[specta]
pub async fn get_my_data(
    param1: String,
    param2: i32,
    state: State<'_, AppState>,
) -> Result<MyResult, String> {
    // 实现逻辑
    Ok(MyResult { data: "result".to_string() })
}

/// 返回类型也需要 Type derive
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MyResult {
    pub data: String,
}
```

### 2.2 注册命令（debug + release）

```rust
// src-tauri/src/lib.rs

// Debug 模式
#[cfg(debug_assertions)]
let specta_builder = tauri_specta::Builder::<tauri::Wry>::new()
    .commands(tauri_specta::collect_commands![
        commands::my_module::get_my_data,  // 添加命令
        // ...
    ])
    .typ::<commands::my_module::MyResult>()  // 添加类型
    // ...

// Release 模式
#[cfg(not(debug_assertions))]
let builder = builder.invoke_handler(tauri::generate_handler![
    commands::my_module::get_my_data,  // 必须与 debug 一致
    // ...
]);
```

## 3. 事件溯源系统

### 3.1 Event 结构

```rust
pub struct Event {
    pub event_id: String,                        // UUID
    pub entity: String,                          // block_id 或 editor_id
    pub attribute: String,                       // "{editor_id}/{cap_id}"
    pub value: JsonValue,                        // 状态变更数据
    pub timestamp: HashMap<String, u64>,         // 向量时钟
    pub created_at: String,                      // ISO 8601
}
```

### 3.2 向量时钟

用于并发控制和冲突检测：
```rust
timestamp: {
    "alice": 5,  // Alice 的第 5 个操作
    "bob": 3,    // Bob 的第 3 个操作
}
```

`create_event()` 会自动处理向量时钟，开发者无需手动管理。

### 3.3 状态投影

`StateProjector` 通过重放事件构建内存状态：
```rust
for event in events {
    projector.apply_event(&event)?;
}
let blocks = projector.get_blocks();
```

## 4. 测试规范

### 4.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_capability() {
        let block = Block {
            block_id: "block-1".to_string(),
            owner: "alice".to_string(),
            // ...
        };

        let cmd = Command {
            editor_id: "alice".to_string(),
            cap_id: "my_extension.write".to_string(),
            payload: json!({ "content": "test" }),
            // ...
        };

        let result = handle_my_write(&cmd, Some(&block));
        assert!(result.is_ok());
    }
}
```

### 4.2 授权测试

```rust
#[test]
fn test_authorization_owner() {
    // Owner 总是被授权
}

#[test]
fn test_authorization_denied() {
    // 非 Owner 无 Grant 时被拒绝
}

#[test]
fn test_authorization_granted() {
    // 非 Owner 有 Grant 时可执行
}
```

## 5. 常见错误与陷阱

| 错误 | 后果 | 解决 |
|------|------|------|
| 忘记在 lib.rs 注册 | 前端报错 | debug 和 release 都要注册 |
| Read/Write Entity 错误 | 状态投影错误 | Write=block_id, Read=editor_id |
| 手动解析 Payload | 类型不匹配 | 使用 `serde_json::from_value` |
| 直接修改状态 | 违反事件溯源 | 使用 `create_event()` |
| 忘记测试授权 | 权限漏洞 | 测试 Owner/非Owner/Grant 场景 |

## 6. 开发检查清单

- [ ] Payload 类型添加了 `#[derive(Type)]`
- [ ] 在 lib.rs 的 **debug 和 release** 都注册了命令和类型
- [ ] Write 能力使用 `block_id` 作为 Entity
- [ ] Read 能力使用 `editor_id` 作为 Entity
- [ ] 使用 `create_event()` 而不是直接修改状态
- [ ] 在 `CapabilityRegistry` 中注册了能力
- [ ] 编写了单元测试和授权测试
- [ ] 运行 `cargo test` 通过
- [ ] 运行 `pnpm tauri dev` 生成了正确的 bindings.ts

## 相关文档

- **完整规范**：`docs/mvp/guidelines/后端开发规范.md`
- **开发流程**：`docs/mvp/guidelines/开发流程.md`
- **扩展开发**：`docs/guides/EXTENSION_DEVELOPMENT.md`
- **架构概览**：`docs/concepts/ARCHITECTURE_OVERVIEW.md`
- **引擎设计**：`docs/concepts/ENGINE_CONCEPTS.md`

# 前后端对接迁移标准方案

## 文档信息

- **文档版本**: 1.0
- **最后更新**: 2025-12-12
- **适用范围**: elfiee Tauri 项目前后端对接
- **目标读者**: 前端开发者、后端开发者

---

## 目录

1. [项目结构](#项目结构)
2. [后端新增功能规范](#后端新增功能规范)
3. [前端新增组件规范](#前端新增组件规范)
4. [前后端对接流程](#前后端对接流程)
5. [关键文件位置](#关键文件位置)
6. [文件编写责任说明](#文件编写责任说明)
7. [最佳实践](#最佳实践)
8. [完整流程示例](#完整流程示例)

---

## 项目结构

**Tauri 项目架构**：
- `elfiee/` 是 Tauri 项目，前后端在同一项目中
- 前端代码：`elfiee/src/`（React + TypeScript）
- 后端代码：`elfiee/src-tauri/src/`（Rust）

**目录结构**：
```
elfiee/
├── src-tauri/                 # 后端（Rust）
│   ├── src/
│   │   ├── commands/          # Tauri 命令定义
│   │   ├── models/            # 数据模型
│   │   ├── extensions/        # 扩展能力
│   │   ├── capabilities/      # 能力处理器
│   │   └── lib.rs             # 注册命令和类型
│   └── Cargo.toml
└── src/                       # 前端（React + TypeScript）
    ├── bindings.ts            # ← 自动生成的 TypeScript 类型（只读）
    ├── lib/
    │   ├── tauri-client.ts    # ← Tauri API 客户端（前端手动编写）
    │   └── app-store.ts       # ← Zustand store（前端手动编写）
    └── components/
        └── editor/
            ├── EditorCanvas.tsx    # ← Markdown 编辑器组件
            └── ContextPanel.tsx    # ← 上下文面板（Timeline 等）
```

---

## 后端新增功能规范

### 1. 新增 Tauri 命令（Commands）

**位置**：在 `src-tauri/src/commands/` 目录下创建新文件或修改现有文件

**示例**：`commands/event.rs`（新增 Timeline 回溯命令）

**注册步骤**：

1. **在 `src-tauri/src/lib.rs` 中注册命令**：
   ```rust
   // Debug 模式（自动生成 TypeScript 类型）
   .commands(tauri_specta::collect_commands![
       commands::event::get_block_content_at_event,  // 新增命令
   ])
   
   // Release 模式
   tauri::generate_handler![
       commands::event::get_block_content_at_event,  // 新增命令
   ]
   ```

2. **类型导出**（如果命令使用了自定义 Payload 类型）：
   ```rust
   .typ::<models::YourPayloadType>()
   ```

3. **自动生成**：运行 `cargo run` 后，类型会自动生成到 `src/bindings.ts`

### 2. 新增能力（Capabilities）

**位置**：在 `src-tauri/src/extensions/` 或 `src-tauri/src/capabilities/` 目录下

**示例**：`extensions/markdown/markdown_write.rs`（Markdown 写入能力）

**注册步骤**：

1. **在 `src-tauri/src/capabilities/registry.rs` 中注册**：
   ```rust
   registry.register(Arc::new(YourCapability));
   ```

2. **调用方式**：前端通过 `execute_command` 调用，传递 `cap_id` 和 `payload`

---

## 前端新增组件规范

### 1. 查找可用接口

**关键文件位置**：

- **类型定义**：查看 `elfiee/src/bindings.ts`（**自动生成**，包含所有命令的类型）
- **API 客户端**：查看 `elfiee/src/lib/tauri-client.ts`（**前端手动编写**，封装了所有后端 API 调用）
- **状态管理**：查看 `elfiee/src/lib/app-store.ts`（**前端手动编写**，Zustand store，管理全局状态）

**⚠️ 重要说明**：
- `bindings.ts` 是自动生成的，前端只读
- `tauri-client.ts` 是前端手动编写的封装层，可以随时修改
- 如果后端新增了命令但 `tauri-client.ts` 中还没有封装，前端开发者需要手动添加

### 2. 使用现有接口

#### 方式 1：通过 TauriClient（推荐）

```typescript
import TauriClient from '@/lib/tauri-client'

// 文件操作
const fileId = await TauriClient.file.openFile()

// Block 操作
const block = await TauriClient.block.getBlock(fileId, blockId)
await TauriClient.block.writeBlock(fileId, blockId, content, editorId)

// Editor 操作
const editors = await TauriClient.editor.listEditors(fileId)

// 终端操作
await TauriClient.terminal.initTerminal(fileId, blockId, editorId, rows, cols)
```

#### 方式 2：通过 AppStore（适合需要状态管理的场景）

```typescript
import { useAppStore } from '@/lib/app-store'

const { loadBlocks, createBlock, writeBlockContent } = useAppStore()
await loadBlocks(fileId)
await createBlock(fileId, name, blockType)
await writeBlockContent(fileId, blockId, content)
```

#### 方式 3：直接调用 bindings（不推荐，除非 TauriClient 未封装）

```typescript
import { commands } from '@/bindings'
const result = await commands.getBlock(fileId, blockId)
```

### 3. 新增前端组件

**位置**：在 `elfiee/src/components/` 目录下创建新组件

**示例**：`components/editor/EditorCanvas.tsx`（Markdown 编辑器组件）

**对接方式**：
- 使用 `TauriClient` 或 `useAppStore` 调用后端 API
- 从 `bindings.ts` 导入类型定义（`Block`, `Event`, `Command` 等）
- 遵循现有的错误处理和加载状态模式

---

## 前后端对接流程

### 后端开发流程（后端开发者）

```
1. 在 src-tauri/src/commands/ 中定义新命令（Rust）
2. 在 src-tauri/src/lib.rs 中注册命令
3. 运行 cargo run，自动生成 src/bindings.ts
```

### 前端开发流程（前端开发者）

```
1. **查看 `src/bindings.ts`** 了解新命令的类型定义（**自动生成，只读**）
   - 这是前端修改 `tauri-client.ts` 的**唯一依据**
   - 查看 `commands.xxx()` 的接口签名、参数类型、返回类型
   - 查看注释了解命令的功能和用法

2. **在 `src/lib/tauri-client.ts` 中封装新命令**（**手动编写，推荐**）
   - **依据**：`bindings.ts` 中的 `commands.xxx()` 接口
   - **步骤**：
     a. 从 `bindings.ts` 导入命令和类型：`import { commands, type Xxx } from '@/bindings'`
     b. 调用 `commands.xxx()` 获取 `Result<T, E>` 类型的结果
     c. 处理 `Result` 类型：`if (result.status === 'ok') return result.data else throw Error`
     d. 提供更友好的 API：直接返回 `T` 类型，而不是 `Result<T, E>`

3. **在 `src/lib/app-store.ts` 中添加状态管理方法**（**手动编写，可选**）
   - 使用 `TauriClient` 调用后端
   - 管理全局状态

4. **在组件中使用 `TauriClient` 或 `AppStore` 调用后端**
```

---

## 完整流程示例

### 场景：后端新增了 `get_block_content_at_event` 命令

#### 步骤 1（后端）：定义命令

```rust
// src-tauri/src/commands/event.rs
pub async fn get_block_content_at_event(...) -> Result<String, String>
```

#### 步骤 2（后端）：注册命令

```rust
// src-tauri/src/lib.rs
.commands(tauri_specta::collect_commands![
    commands::event::get_block_content_at_event,
])
```

#### 步骤 3（后端）：运行 cargo run

自动生成 `src/bindings.ts`：

```typescript
// src/bindings.ts（自动生成）
export const commands = {
  async getBlockContentAtEvent(
    fileId: string,
    blockId: string,
    eventId: string
  ): Promise<Result<string, string>> {
    // ... 自动生成的代码
  }
}
```

#### 步骤 4（前端）：查看 bindings.ts 了解接口（修改依据）

```typescript
// src/bindings.ts（自动生成，只读）
// 前端开发者查看这个文件，了解：
// 1. 命令名称：getBlockContentAtEvent
// 2. 参数类型：fileId: string, blockId: string, eventId: string
// 3. 返回类型：Promise<Result<string, string>>
// 4. 注释说明：命令的功能和用法
```

#### 步骤 5（前端）：根据 bindings.ts 封装到 TauriClient（手动编写）

```typescript
// src/lib/tauri-client.ts（前端手动编写）
import { commands } from '@/bindings'  // ← 从 bindings.ts 导入

export class FileOperations {
  /**
   * 获取指定事件时刻的 Block 内容（Timeline 回溯）
   * 
   * @param fileId - 文件 ID
   * @param blockId - Block ID
   * @param eventId - 事件 ID
   * @returns Markdown 内容字符串
   * 
   * 依据：bindings.ts 中的 commands.getBlockContentAtEvent()
   */
  static async getContentAtEvent(
    fileId: string,
    blockId: string,
    eventId: string
  ): Promise<string> {
    // ← 调用 bindings.ts 中的命令
    const result = await commands.getBlockContentAtEvent(fileId, blockId, eventId)
    
    // ← 封装 Result<T, E> 为直接返回 T 或抛出错误
    if (result.status === 'ok') {
      return result.data
    } else {
      throw new Error(result.error)
    }
  }
}
```

#### 步骤 6（前端）：在组件中使用

```typescript
// src/components/editor/ContextPanel.tsx
import TauriClient from '@/lib/tauri-client'

const content = await TauriClient.file.getContentAtEvent(fileId, blockId, eventId)
```

---

## 前端修改 `tauri-client.ts` 的详细依据

### 1. 查看 `bindings.ts` 中的命令接口

```typescript
// bindings.ts（自动生成）
export const commands = {
  async getBlock(
    fileId: string,
    blockId: string
  ): Promise<Result<Block, string>> {
    // 返回 Result<Block, string> 类型
  }
}
```

### 2. 理解接口特点

- 命令名称：`getBlock`（驼峰命名）
- 参数：`fileId: string, blockId: string`
- 返回类型：`Promise<Result<Block, string>>`
- `Result<T, E>` 结构：`{ status: 'ok', data: T } | { status: 'error', error: E }`

### 3. 在 `tauri-client.ts` 中封装

```typescript
// tauri-client.ts（前端手动编写）
import { commands, type Block } from '@/bindings'  // ← 依据：从 bindings.ts 导入

export class BlockOperations {
  static async getBlock(
    fileId: string,
    blockId: string
  ): Promise<Block> {  // ← 依据：bindings.ts 中返回 Result<Block, string>
    const result = await commands.getBlock(fileId, blockId)  // ← 依据：调用 bindings.ts 中的命令
    
    if (result.status === 'ok') {
      return result.data  // ← 依据：Result 的 data 字段类型是 Block
    } else {
      throw new Error(result.error)  // ← 依据：Result 的 error 字段类型是 string
    }
  }
}
```

### 4. 封装的好处

- ✅ 统一错误处理：将 `Result<T, E>` 转换为直接返回 `T` 或抛出错误
- ✅ 更友好的 API：组件中不需要处理 `Result` 类型
- ✅ 类型安全：TypeScript 自动推断类型，无需手动指定
- ✅ 代码复用：多个组件可以共享同一个封装方法

---

## 关键文件位置

| 文件 | 路径 | 编写者 | 说明 |
|------|------|--------|------|
| 类型定义（自动生成） | `elfiee/src/bindings.ts` | **后端自动生成** | 所有 Tauri 命令的 TypeScript 类型，运行 `cargo run` 自动生成 |
| API 客户端 | `elfiee/src/lib/tauri-client.ts` | **前端手动编写** | 封装所有后端 API 调用，提供高级接口 |
| 状态管理 | `elfiee/src/lib/app-store.ts` | **前端手动编写** | Zustand store，管理全局状态 |
| 后端命令 | `elfiee/src-tauri/src/commands/` | **后端手动编写** | Tauri 命令定义（Rust） |
| 后端能力 | `elfiee/src-tauri/src/extensions/` | **后端手动编写** | 扩展能力定义（Rust） |
| 命令注册 | `elfiee/src-tauri/src/lib.rs` | **后端手动编写** | 注册所有 Tauri 命令 |
| 能力注册 | `elfiee/src-tauri/src/capabilities/registry.rs` | **后端手动编写** | 注册所有能力处理器 |

---

## 文件编写责任说明

### 1. `bindings.ts`（自动生成）

- **编写者**：后端自动生成（tauri-specta）
- **触发时机**：运行 `cargo run` 时自动生成
- **修改权限**：❌ 前端不能修改（会被覆盖）
- **用途**：提供类型定义和底层命令接口
- **作用**：前端修改 `tauri-client.ts` 的**唯一依据**

### 2. `tauri-client.ts`（手动编写）

- **编写者**：**前端开发者**
- **编写时机**：后端新增命令后，前端需要封装时
- **修改权限**：✅ 前端可以随时修改
- **修改依据**：**`bindings.ts` 中自动生成的命令接口**
- **修改流程**：
  1. 后端新增命令后，运行 `cargo run` 更新 `bindings.ts`
  2. 前端查看 `bindings.ts` 中的新命令接口（如 `commands.getBlock()`）
  3. 根据接口签名、参数类型、返回类型在 `tauri-client.ts` 中封装
  4. 封装 `Result<T, E>` 为直接返回 `T` 或抛出错误
- **用途**：封装 `bindings.ts` 中的底层命令，提供更友好的 API
- **示例**：`TauriClient.block.getBlock()` 封装了 `commands.getBlock()`，并处理错误

### 3. `app-store.ts`（手动编写）

- **编写者**：**前端开发者**
- **编写时机**：需要全局状态管理时
- **修改权限**：✅ 前端可以随时修改
- **修改依据**：使用 `TauriClient` 调用后端（不直接使用 `bindings.ts`）
- **用途**：使用 Zustand 管理应用状态，调用 `TauriClient` 获取数据

---

## 最佳实践

### 1. 后端开发

- 新增命令时，同时更新 Debug 和 Release 模式的注册
- 自定义 Payload 类型需要在 `lib.rs` 中显式注册
- 使用强类型 Payload（如 `MarkdownWritePayload`），避免使用 `serde_json::Value`

### 2. 前端开发

#### `bindings.ts` 使用规范

- **作用**：前端修改 `tauri-client.ts` 的**唯一依据**
- **查看方式**：查看 `commands.xxx()` 的接口定义和类型
- **使用方式**：导入类型和命令，但不直接调用（通过 `tauri-client.ts` 封装）

#### `tauri-client.ts` 编写规范

- **修改依据**：根据 `bindings.ts` 中的 `commands.xxx()` 接口进行封装
- **修改流程**：
  1. 后端新增命令后，运行 `cargo run` 更新 `bindings.ts`
  2. 查看 `bindings.ts` 中的新命令接口（如 `commands.getBlockContentAtEvent()`）
  3. 在 `tauri-client.ts` 中添加封装方法
  4. 提供统一的错误处理和类型转换
- **封装原则**：
  - 封装 `commands.xxx()` 的 `Result<T, E>` 返回值为直接返回 `T` 或抛出错误
  - 提供更友好的 API 接口（如 `writeBlock()` 封装 `executeCommand()`）
  - 保持与现有代码风格一致

#### `app-store.ts` 使用规范

- 使用 `TauriClient` 调用后端
- 管理文件、Block、Editor 等状态

#### 通用规范

- 优先使用 `TauriClient` 而不是直接调用 `bindings.ts`
- 需要状态管理时，使用 `useAppStore`
- 所有类型从 `bindings.ts` 导入，不要手动定义
- 遵循现有的错误处理模式（try-catch + toast 通知）

### 3. 类型安全

- 后端类型变更后，运行 `cargo run` 自动更新 `bindings.ts`
- 前端 TypeScript 会立即检测到类型不匹配
- 不要手动修改 `bindings.ts`（会被自动覆盖）
- `tauri-client.ts` 中的类型从 `bindings.ts` 导入，保持类型一致

---

## 总结

### 核心原则

1. **单一数据源**：`bindings.ts` 是前端修改 `tauri-client.ts` 的唯一依据
2. **分层封装**：`bindings.ts` → `tauri-client.ts` → `app-store.ts` → 组件
3. **类型安全**：所有类型从 `bindings.ts` 导入，保持类型一致
4. **错误处理**：统一在 `tauri-client.ts` 中处理 `Result<T, E>` 类型

### 工作流程

```
后端新增命令
    ↓
运行 cargo run 自动生成 bindings.ts
    ↓
前端查看 bindings.ts 了解接口
    ↓
前端在 tauri-client.ts 中封装
    ↓
前端在组件中使用 TauriClient
```

### 关键要点

- ✅ `bindings.ts` 是自动生成的，前端只读
- ✅ `tauri-client.ts` 是前端手动编写的，依据 `bindings.ts` 进行修改
- ✅ 前端应该优先使用 `TauriClient`，而不是直接调用 `bindings.ts`
- ✅ 所有类型从 `bindings.ts` 导入，保持类型安全

---

**文档维护**：本文档应与项目代码同步更新，当项目架构或开发流程发生变化时，请及时更新本文档。


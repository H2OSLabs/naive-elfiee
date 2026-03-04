# Phase 2 — Agent 模块需求文档 (3.1)

> **版本**: v1.0
> **对应计划**: `docs/mvp/phase2/task-and-cost_v3.md` §3.1
> **预估人时**: 15
> **任务编号**: F1-01, F1-02, F3-01, F3-02, F3-03

---

## 目录

1. [概述](#1-概述)
2. [术语与约定](#2-术语与约定)
3. [现状分析](#3-现状分析)
4. [需求变更说明](#4-需求变更说明)
5. [数据结构设计](#5-数据结构设计)
6. [Capability 定义](#6-capability-定义)
7. [详细需求：agent.create](#7-详细需求agentcreate)
8. [详细需求：agent.enable](#8-详细需求agentenable)
9. [详细需求：agent.disable](#9-详细需求agentdisable)
10. [详细需求：MCP 配置合并器](#10-详细需求mcp-配置合并器)
11. [幂等性设计](#11-幂等性设计)
12. [错误处理](#12-错误处理)
13. [文件结构](#13-文件结构)
14. [依赖关系](#14-依赖关系)
15. [测试计划](#15-测试计划)
16. [验收标准](#16-验收标准)

---

## 1. 概述

### 1.1 模块定位

Agent 模块将 Elfiee 的 Block 系统与外部 AI 编码工具（Phase 2 仅支持 Claude Code）集成。每个 Agent Block 代表一个"已激活的项目"——即 Elfiee 已接管该项目的 AI 工作流配置。

### 1.2 核心价值

- **一键启用**：`agent.create(target_project_id)` 完成从创建 Block 到注入 MCP 配置的全部操作
- **可逆操作**：`agent.disable` 清理所有注入的配置，项目恢复原始状态
- **幂等安全**：多次 enable 不会产生副作用，只会更新到最新配置

### 1.3 设计约束

| 约束 | 说明 |
| :--- | :--- |
| **前提条件** | 目标项目必须已有 `.claude/` 目录（已初始化 Claude Code） |
| **Phase 2 限定** | 仅支持 Claude Code，通过标准 MCP 协议集成 |
| **静态资源** | `.elf/Agents/elfiee-client/` 是所有 Agent Block 共享的内置工具目录 |
| **Editor 级唯一** | 每个 (外部项目, bot editor) 对最多一个 Agent Block |

---

## 2. 术语与约定

| 术语 | 含义 |
| :--- | :--- |
| **Agent Block** | `block_type: "agent"` 的 Block，存储项目级 AI 集成配置 |
| **目标项目** | Agent Block 关联的外部项目，通过 `target_project_id`（Dir Block ID）标识 |
| **external_path** | 目标项目的物理磁盘路径，从 Dir Block 的 `metadata.external_root_path` 获取 |
| **软连接注入** | 在目标项目 `.claude/skills/elfiee-client/` 创建指向 `.elf/Agents/elfiee-client/` 的 symlink |
| **MCP 配置注入** | 将 MCP Server 配置合并到目标项目 `.claude/mcp.json` |
| **`{elf_path}`** | 占位符，指当前 `.elf` 文件的物理路径，在 enable 时动态替换 |

---

## 3. 现状分析

### 3.1 已实现的 Agent 功能

当前 `src-tauri/src/extensions/agent/` 已实现的功能面向 **Phase 1 的 LLM 直接调用**场景：

| 文件 | 功能 | Phase 2 是否沿用 |
| :--- | :--- | :--- |
| `mod.rs` | `AgentConfig`（provider/model/api_key_env/system_prompt）、`Proposal`、`ProposedCommand` 等类型 | **需重构** — `AgentConfig` 需重新定义为项目级配置 |
| `agent_create.rs` | 创建 Agent Block + 生成 `agent-{uuid}` editor_id | **需重构** — 新的 create 逻辑完全不同 |
| `agent_configure.rs` | 更新 Agent 配置（provider/model 等） | **可保留** — 但需适配新数据结构 |
| `context/` | Context 收集和截断逻辑 | **不涉及** — Phase 2 不使用 |
| `llm/` | Anthropic API 调用和解析 | **不涉及** — Phase 2 不使用 |
| `tests.rs` | 类型序列化测试 + CBAC 授权测试 | **需重写** — 覆盖新功能 |

### 3.2 需复用的基础设施

| 组件 | 位置 | 用途 |
| :--- | :--- | :--- |
| `Block` 模型 | `models/block.rs` | Agent Block 的基础结构 |
| `#[capability]` 宏 | `capability-macros/` | 定义 `agent.create`, `agent.enable`, `agent.disable` |
| `CapabilityRegistry` | `capabilities/registry.rs` | 注册新的 Capability Handler |
| `StateProjector` | `engine/state.rs` | 管理 Agent Block 状态投影 |
| `.elf/` 初始化 | `extensions/directory/elf_meta.rs` | `.elf/Agents/elfiee-client/` 骨架已创建 |
| `directory.export` | `extensions/directory/directory_export.rs` | 分离式 I/O 架构参考（authorization + audit event + 外部 I/O） |
| `inject_block_dir` | `engine/actor.rs` | 运行时注入 `_block_dir` 路径 |

---

## 4. 需求变更说明

Phase 2 的 Agent 模块与 Phase 1 有**根本性差异**：

| 维度 | Phase 1（已实现） | Phase 2（本次需求） |
| :--- | :--- | :--- |
| **Agent 定位** | 内置 LLM Agent，直接调用 API | 外部 AI 工具的集成配置 |
| **contents 存储** | `AgentConfig`（provider, model, api_key） | `AgentContents`（name, target_project_id, status） |
| **核心操作** | invoke → approve（LLM 提议-审批流） | create → enable/disable（配置注入-清理流） |
| **副作用** | 无外部 I/O | **有外部 I/O**：创建 symlink、写入 `.claude/mcp.json` |
| **关联实体** | `editor_id`（Agent 作为 Editor） | `target_project_id`（Dir Block） |

**处理方式**：
- 保留 Phase 1 的 `AgentConfig`、`Proposal` 等类型（不删除），后续可能用于 Phase 3
- 新增 `AgentContents` 等 Phase 2 类型
- `agent.create` Handler 完全重写
- `agent.configure` 暂不修改（Phase 2 不使用 LLM 直接调用功能）
- 新增 `agent.enable`、`agent.disable` Capability

---

## 5. 数据结构设计

### 5.1 AgentContents（Block.contents）

```rust
// 文件位置: extensions/agent/mod.rs

/// Phase 2 Agent Block 内容，存储项目级 AI 集成配置。
///
/// 与 Phase 1 的 AgentConfig（LLM 直接调用配置）并存。
/// AgentContents 存储在 Block.contents 中。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentContents {
    /// Agent 显示名称（默认 "elfiee"）
    pub name: String,

    /// 关联的外部项目 Dir Block ID
    ///
    /// 通过此 ID 在 StateProjector.blocks 中查找 Dir Block，
    /// 再从 Dir Block 的 metadata.custom["external_root_path"] 获取物理路径。
    pub target_project_id: String,

    /// Agent 当前状态
    pub status: AgentStatus,

    /// Bot editor_id associated with this agent.
    /// Used by MCP server to attribute operations to the correct identity.
    /// None for legacy agents (falls back to GUI active editor).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_id: Option<String>,
}
```

### 5.2 AgentStatus

```rust
/// Agent 启用状态
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// 已启用：软连接存在，MCP 配置已注入
    Enabled,

    /// 已禁用：软连接已清理，MCP 配置已移除
    Disabled,
}
```

### 5.3 Payload 类型

```rust
/// agent.create 的 Payload
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentCreateV2Payload {
    /// Agent 显示名称（可选，默认 "elfiee"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// 关联的外部项目 Dir Block ID（必须）
    pub target_project_id: String,

    /// Bot editor_id to associate with this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_id: Option<String>,
}

/// agent.enable 的 Payload
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentEnablePayload {
    /// Agent Block ID（必须）
    pub agent_block_id: String,
}

/// agent.disable 的 Payload
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentDisablePayload {
    /// Agent Block ID（必须）
    pub agent_block_id: String,
}
```

### 5.4 Event Value 结构

**agent.create Event**：
```json
{
  "name": "elfiee",
  "type": "agent",
  "owner": "alice",
  "contents": {
    "name": "elfiee",
    "target_project_id": "uuid-of-dir-block",
    "status": "enabled",
    "editor_id": "bot-editor-uuid",
    "source": "outline"
  },
  "children": {},
  "metadata": {
    "description": "Agent for project: my-project",
    "created_at": "2026-01-30T...",
    "updated_at": "2026-01-30T..."
  }
}
```

**agent.enable Event**：
```json
{
  "contents": {
    "name": "elfiee",
    "target_project_id": "uuid-of-dir-block",
    "status": "enabled"
  },
  "metadata": {
    "updated_at": "2026-01-30T...",
    "custom": {
      "last_enabled_at": "2026-01-30T...",
      "symlink_path": "/path/to/project/.claude/skills/elfiee-client",
      "mcp_config_path": "/path/to/project/.claude/mcp.json"
    }
  }
}
```

**agent.disable Event**：
```json
{
  "contents": {
    "name": "elfiee",
    "target_project_id": "uuid-of-dir-block",
    "status": "disabled"
  },
  "metadata": {
    "updated_at": "2026-01-30T...",
    "custom": {
      "last_disabled_at": "2026-01-30T..."
    }
  }
}
```

---

## 6. Capability 定义

### 6.1 总览

| Capability ID | Target | 操作类型 | 外部 I/O |
| :--- | :--- | :--- | :--- |
| `agent.create` | `core/*` | 创建 Block + 自动 enable | 创建 symlink + 写入 MCP 配置 |
| `agent.enable` | `agent` | 更新已有 Block | 创建 symlink + 写入 MCP 配置 |
| `agent.disable` | `agent` | 更新已有 Block | 删除 symlink + 移除 MCP 配置 |

### 6.2 架构决策：I/O 操作位置

参照 `directory.export` 的架构模式（Capability Handler 做授权 + 审计事件，Tauri Command 做实际 I/O），Agent 模块的外部 I/O（symlink、MCP 配置文件操作）**在 Tauri Command 层**执行，而非在 Capability Handler 内部。

**原因**：
1. Event 日志保持纯粹（无 I/O 副作用）
2. Event 重放时不会重复执行 I/O
3. 符合现有 `directory.export` 模式的一致性

**工作流**：
```
前端 → Tauri Command (agent_create / agent_enable / agent_disable)
       ├── Step 1: 调用 Engine.process_command() → Capability Handler
       │            → 授权检查 + 生成 Event（含状态变更）
       ├── Step 2: 执行外部 I/O（symlink + MCP 配置）
       │            → 基于 Event 中的路径信息
       └── Step 3: 返回结果（含是否需要重启 Claude 提示）
```

---

## 7. 详细需求：agent.create

### 7.1 任务编号

**F1-02**: `agent.create` Capability

### 7.2 触发方式

```typescript
// 前端调用
await commands.agentCreate(fileId, {
  target_project_id: "uuid-of-dir-block",
  name: "elfiee"  // 可选
});
```

### 7.3 处理流程

```
agent.create(target_project_id, name?)
  │
  ├── 1. 验证 Payload
  │     ├── target_project_id 非空
  │     └── name 非空（默认 "elfiee"）
  │
  ├── 2. 验证 目标项目有效性
  │     ├── target_project_id 对应的 Block 存在于 StateProjector
  │     ├── 该 Block 的 block_type == "directory"
  │     └── 该 Block 的 metadata.custom["external_root_path"] 存在
  │
  ├── 3. 验证 唯一性约束
  │     └── StateProjector 中不存在 block_type == "agent"
  │         且 contents.target_project_id == target_project_id
  │         且 contents.editor_id == payload.editor_id 的 Block
  │
  ├── 4. 验证 .claude/ 目录存在
  │     └── {external_path}/.claude/ 目录存在（已初始化 Claude Code）
  │
  ├── 5. 创建 Agent Block
  │     ├── block_id: uuid::Uuid::new_v4()
  │     ├── name: payload.name 或 "elfiee"
  │     ├── block_type: "agent"
  │     ├── contents: AgentContents { name, target_project_id, status: Enabled }
  │     └── metadata: { description, created_at, updated_at }
  │
  ├── 6. 生成 core.create Event
  │     └── Event value 包含完整 Block 初始状态
  │
  └── 7. 返回 Event + enable 所需的路径信息
```

### 7.4 Capability Handler（Event 生成）

```rust
#[capability(id = "agent.create", target = "core/*")]
fn handle_agent_create_v2(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>>
```

**输入**：
- `cmd.payload`: `AgentCreateV2Payload` JSON
- `cmd.editor_id`: 操作者

**输出**：
- `Vec<Event>`: 包含一个 `agent.create` 事件

**验证规则**：
1. `target_project_id` 不能为空字符串
2. name（如提供）不能为空字符串

> **注意**: 因为 Capability Handler 无法访问 StateProjector（handler 的函数签名为 `(cmd, block) -> Vec<Event>`），唯一性检查和目标项目有效性检查应在 **Tauri Command 层** 执行。

### 7.5 Tauri Command（I/O 执行）

```rust
#[tauri::command]
#[specta::specta]
pub async fn agent_create(
    file_id: String,
    payload: AgentCreateV2Payload,
    state: tauri::State<'_, AppState>,
) -> Result<AgentCreateResult, String>
```

**执行步骤**：

1. **获取 Engine Handle**
2. **验证目标项目**：
   - 从 StateProjector 查找 `target_project_id` 对应的 Dir Block
   - 确认 `block_type == "directory"`
   - 从 `metadata.custom["external_root_path"]` 获取 `external_path`
3. **唯一性检查**：
   - 遍历 StateProjector 所有 blocks，检查无重复 `target_project_id`
4. **检查 `.claude/` 存在**：
   - `Path::new(&external_path).join(".claude").exists()`
5. **调用 Engine**：
   - `process_command(create_cmd)` → 获得 Events
6. **执行 enable I/O**：
   - 创建 symlink：`.claude/skills/elfiee-client/` → `.elf/Agents/elfiee-client/`（Block 快照路径）
   - 合并 MCP 配置到 `.claude/mcp.json`
7. **返回结果**：
   ```rust
   pub struct AgentCreateResult {
       pub agent_block_id: String,
       pub status: AgentStatus,
       pub needs_restart: bool,  // 提示用户重启 Claude Code
       pub message: String,
   }
   ```

### 7.6 软连接路径计算

```
源路径（symlink target）:
  {elf_temp_dir}/block-{elf_dir_block_id}/Agents/elfiee-client/

  说明：.elf/ Dir Block 经 directory.export 或快照后，
  elfiee-client/ 的物理文件位于 block 目录下。

  具体路径获取：
  - elf_temp_dir: 从 Engine actor 的 temp_dir 获取
  - elf_dir_block_id: 在 StateProjector 中查找 name == ".elf" 且 block_type == "directory" 的 Block

目标路径（symlink location）:
  {external_path}/.claude/skills/elfiee-client/

  说明：Claude Code 自动读取此目录下的 SKILL.md
```

### 7.7 MCP 配置注入

调用 `mcp_config::merge_server()` 将以下配置合并到 `{external_path}/.claude/mcp.json`：

```json
{
  "mcpServers": {
    "elfiee": {
      "command": "elfiee",
      "args": ["mcp-server", "--elf", "{elf_path}"]
    }
  }
}
```

其中 `{elf_path}` 替换为当前 `.elf` 文件的物理路径。

---

## 8. 详细需求：agent.enable

### 8.1 任务编号

**F3-01**: `agent.enable` Capability

### 8.2 触发方式

```typescript
// 前端调用
await commands.agentEnable(fileId, { agent_block_id: "uuid" });
```

### 8.3 处理流程

```
agent.enable(agent_block_id)
  │
  ├── 1. Capability Handler
  │     ├── 验证 Block 存在且 block_type == "agent"
  │     ├── 获取当前 AgentContents
  │     ├── 生成 Event: status → Enabled + metadata 更新
  │     └── 返回 Event
  │
  └── 2. Tauri Command (I/O)
        ├── 获取 target_project_id → external_path
        ├── 获取 .elf/ Block 的快照目录路径
        ├── 创建 symlink
        │     ├── 目标: {external_path}/.claude/skills/elfiee-client/
        │     ├── 源: {elf_block_dir}/Agents/elfiee-client/
        │     └── 如已存在则先删除再创建（幂等）
        ├── 合并 MCP 配置
        │     ├── 读取 {external_path}/.claude/mcp.json
        │     ├── 合并 "elfiee" server 配置
        │     └── 写回（保留已有配置）
        └── 返回结果（含 needs_restart 提示）
```

### 8.4 Capability Handler

```rust
#[capability(id = "agent.enable", target = "agent")]
fn handle_agent_enable(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>>
```

**输入**：
- `cmd.block_id`: Agent Block ID
- `cmd.payload`: `AgentEnablePayload`（可为空 JSON `{}`，block_id 已在 cmd 中）
- `block`: 必须存在，且 `block_type == "agent"`

**输出**：
- `Vec<Event>`: 包含一个 `agent.enable` 事件，value 包含更新后的 contents 和 metadata

**处理逻辑**：
1. 验证 block 存在且类型为 agent
2. 从 block.contents 反序列化 `AgentContents`
3. 更新 status 为 `Enabled`
4. 更新 metadata.updated_at
5. 记录 `last_enabled_at` 到 metadata.custom
6. 生成 Event

### 8.5 Tauri Command

```rust
#[tauri::command]
#[specta::specta]
pub async fn agent_enable(
    file_id: String,
    agent_block_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<AgentEnableResult, String>
```

**执行步骤**：

1. **获取 Agent Block**：从 StateProjector 获取
2. **获取目标项目路径**：
   - 从 Agent Block contents 获取 `target_project_id`
   - 从 StateProjector 查找对应 Dir Block
   - 获取 `metadata.custom["external_root_path"]`
3. **获取 .elf/ Block 快照路径**：
   - 查找 `.elf/` Dir Block（name == ".elf"）
   - 获取 `_block_dir` 或通过 temp_dir + block_id 计算
4. **调用 Engine**：`process_command(enable_cmd)` → Events
5. **创建 symlink**：
   - `ensure_parent_dir("{external_path}/.claude/skills/")`
   - 如 `elfiee-client/` symlink 已存在：删除后重建
   - `std::os::unix::fs::symlink(source, target)` (Unix)
   - 或 `junction::create(source, target)` (Windows)
6. **合并 MCP 配置**：`mcp_config::merge_server(...)`
7. **返回结果**

### 8.6 跨平台 Symlink 处理

| 平台 | 实现方式 | 说明 |
| :--- | :--- | :--- |
| **macOS/Linux** | `std::os::unix::fs::symlink` | 标准 symlink |
| **Windows** | `junction::create` 或 `std::os::windows::fs::symlink_dir` | 目录使用 junction（无需管理员权限） |

需使用条件编译：
```rust
#[cfg(unix)]
fn create_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn create_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    // Windows junction 不需要管理员权限
    junction::create(src, dst)
}
```

---

## 9. 详细需求：agent.disable

### 9.1 任务编号

**F3-02**: `agent.disable` Capability

### 9.2 触发方式

```typescript
// 前端调用
await commands.agentDisable(fileId, { agent_block_id: "uuid" });
```

### 9.3 处理流程

```
agent.disable(agent_block_id)
  │
  ├── 1. Capability Handler
  │     ├── 验证 Block 存在且 block_type == "agent"
  │     ├── 获取当前 AgentContents
  │     ├── 生成 Event: status → Disabled + metadata 更新
  │     └── 返回 Event
  │
  └── 2. Tauri Command (I/O)
        ├── 获取 target_project_id → external_path
        ├── 清理 symlink
        │     ├── 检查 {external_path}/.claude/skills/elfiee-client/ 是否存在
        │     └── 如存在则删除（包括 broken symlink）
        ├── 移除 MCP 配置
        │     ├── 读取 {external_path}/.claude/mcp.json
        │     ├── 移除 "elfiee" server 条目
        │     └── 写回（如 mcpServers 为空则保留空对象）
        └── 返回结果
```

### 9.4 Capability Handler

```rust
#[capability(id = "agent.disable", target = "agent")]
fn handle_agent_disable(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>>
```

**处理逻辑**：
1. 验证 block 存在且类型为 agent
2. 从 block.contents 反序列化 `AgentContents`
3. **如已经是 Disabled 状态**：仍然生成 Event（静默成功，不报错）
4. 更新 status 为 `Disabled`
5. 更新 metadata
6. 生成 Event

### 9.5 Tauri Command

```rust
#[tauri::command]
#[specta::specta]
pub async fn agent_disable(
    file_id: String,
    agent_block_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<AgentDisableResult, String>
```

**执行步骤**：

1. **获取 Agent Block** + 目标项目路径
2. **调用 Engine**：`process_command(disable_cmd)` → Events
3. **清理 symlink**：
   - 检查 `{external_path}/.claude/skills/elfiee-client/` 是否存在
   - 如存在：`std::fs::remove_dir_all()` 或 `std::fs::remove_file()`（symlink 本身）
   - 如不存在：静默忽略
4. **移除 MCP 配置**：`mcp_config::remove_server(...)`
5. **返回结果**

---

## 10. 详细需求：MCP 配置合并器

### 10.1 任务编号

**F3-03**: MCP 配置合并器

### 10.2 文件位置

`src-tauri/src/utils/mcp_config.rs`（新建）

### 10.3 接口定义

```rust
/// 合并 MCP Server 配置到 .claude/mcp.json
///
/// 幂等操作：如果 server_name 已存在则覆盖。
/// 如果文件不存在则创建。
pub fn merge_server(
    config_path: &Path,     // .claude/mcp.json 的路径
    server_name: &str,      // "elfiee"
    server_config: Value,   // { "command": "elfiee", "args": [...] }
) -> Result<(), String>

/// 从 .claude/mcp.json 移除指定 Server 配置
///
/// 幂等操作：如果 server_name 不存在则静默成功。
/// 如果文件不存在则静默成功。
/// 移除后如 mcpServers 为空，保留 {"mcpServers": {}}。
pub fn remove_server(
    config_path: &Path,     // .claude/mcp.json 的路径
    server_name: &str,      // "elfiee"
) -> Result<(), String>

/// 将 MCP 配置模板中的占位符替换为实际值
///
/// 支持的占位符：
/// - {elf_path}: .elf 文件的物理路径
pub fn resolve_template(
    template: &Value,       // 模板 JSON
    elf_path: &str,         // .elf 文件路径
) -> Value
```

### 10.4 mcp.json 格式

**Claude Code 格式**（目标）：
```json
{
  "mcpServers": {
    "elfiee": {
      "command": "elfiee",
      "args": ["mcp-server", "--elf", "/path/to/project.elf"]
    },
    "other-tool": {
      "command": "other-tool",
      "args": ["--flag"]
    }
  }
}
```

### 10.5 merge_server 行为规范

| 场景 | 行为 |
| :--- | :--- |
| 文件不存在 | 创建文件，写入 `{"mcpServers": {"elfiee": {...}}}` |
| 文件存在，无 `mcpServers` 字段 | 添加 `mcpServers` 字段 |
| `mcpServers` 中无 `elfiee` 条目 | 添加 `elfiee` 条目 |
| `mcpServers` 中已有 `elfiee` 条目 | **覆盖**为新配置 |
| 文件中有其他 Server 配置 | **保留不动** |
| JSON 格式错误 | 返回 Error（不覆盖损坏的文件） |

### 10.6 remove_server 行为规范

| 场景 | 行为 |
| :--- | :--- |
| 文件不存在 | 静默成功（`Ok(())`） |
| `mcpServers` 中无目标条目 | 静默成功 |
| `mcpServers` 中有目标条目 | 移除该条目 |
| 移除后 `mcpServers` 为空 | 保留 `{"mcpServers": {}}` |
| 文件中有其他 Server 配置 | **保留不动** |
| JSON 格式错误 | 返回 Error |

### 10.7 占位符替换

模板文件（`templates/elfiee-client/mcp.json`）内容：
```json
{
  "mcpServers": {
    "elfiee": {
      "command": "elfiee",
      "args": ["mcp-server", "--elf", "{elf_path}"]
    }
  }
}
```

`resolve_template()` 将 `{elf_path}` 替换为实际路径：
```json
{
  "mcpServers": {
    "elfiee": {
      "command": "elfiee",
      "args": ["mcp-server", "--elf", "/home/user/project.elf"]
    }
  }
}
```

---

## 11. 幂等性设计

### 11.1 enable 幂等

| 当前状态 | 操作 | 结果 |
| :--- | :--- | :--- |
| Enabled | agent.enable | 更新配置（删除旧 symlink → 创建新 symlink，覆盖 MCP 配置） |
| Disabled | agent.enable | 创建 symlink + 注入 MCP 配置，状态变为 Enabled |

### 11.2 disable 幂等

| 当前状态 | 操作 | 结果 |
| :--- | :--- | :--- |
| Enabled | agent.disable | 清理 symlink + 移除 MCP 配置，状态变为 Disabled |
| Disabled | agent.disable | 静默成功（生成 Event 但无 I/O 变化） |

### 11.3 create 唯一性

| 场景 | 结果 |
| :--- | :--- |
| (target_project_id, editor_id) 尚无 Agent | 创建成功 + 自动 enable |
| (target_project_id, editor_id) 已有 Agent | 返回错误 `"Agent already exists for this editor on project"` |
| 同一项目不同 editor_id | 各自独立创建成功 |

---

## 12. 错误处理

### 12.1 Capability Handler 错误

| 错误条件 | 错误消息 | HTTP-like 语义 |
| :--- | :--- | :--- |
| Payload 反序列化失败 | `"Invalid payload for agent.{action}: {detail}"` | 400 |
| target_project_id 为空 | `"target_project_id cannot be empty"` | 400 |
| Block 不存在（enable/disable） | `"Block required for agent.{action}"` | 404 |
| Block 类型不是 agent | `"Expected block_type 'agent', got '{type}'"` | 400 |

### 12.2 Tauri Command 错误

| 错误条件 | 错误消息 |
| :--- | :--- |
| Engine 未找到 | `"File '{file_id}' is not open"` |
| 目标 Dir Block 不存在 | `"Target project block not found: {id}"` |
| Dir Block 无 external_root_path | `"Target project has no external path"` |
| `.claude/` 目录不存在 | `"Claude not initialized in target project: {path}. Run 'claude' first."` |
| 重复创建 Agent | `"Agent already exists for this editor on project: {name} (block_id: {id})"` |
| Symlink 创建失败 | `"Failed to create symlink: {io_error}"` |
| MCP 配置写入失败 | `"Failed to write MCP config: {io_error}"` |
| MCP 配置 JSON 损坏 | `"Invalid JSON in {path}: {parse_error}"` |

### 12.3 部分失败处理

如果 enable 的 I/O 步骤部分失败（如 symlink 成功但 MCP 配置失败）：

**策略**：**不回滚 Event**，但返回警告信息。

原因：
- Event 已提交到 EventStore，是不可变的
- 状态已标记为 Enabled
- 用户可以再次调用 enable（幂等）来修复

**返回格式**：
```rust
pub struct AgentEnableResult {
    pub agent_block_id: String,
    pub status: AgentStatus,
    pub needs_restart: bool,
    pub message: String,
    pub warnings: Vec<String>,  // 部分失败的警告
}
```

---

## 13. 文件结构

### 13.1 新建文件

```
src-tauri/src/extensions/agent/
├── mod.rs                    # 修改：添加 AgentContents, AgentStatus, V2 Payload 类型
├── agent_create.rs           # 重写：Phase 2 的 agent.create handler
├── agent_enable.rs           # 新建：agent.enable handler
├── agent_disable.rs          # 新建：agent.disable handler

src-tauri/src/utils/
├── mcp_config.rs             # 新建：MCP 配置合并器

src-tauri/src/commands/
├── agent_commands.rs          # 新建：Tauri commands (agent_create, agent_enable, agent_disable)
```

### 13.2 修改文件

```
src-tauri/src/extensions/agent/mod.rs        # 添加新类型、pub mod 声明
src-tauri/src/capabilities/registry.rs       # 注册 AgentEnableCapability, AgentDisableCapability
src-tauri/src/utils/mod.rs                   # 添加 pub mod mcp_config
src-tauri/src/commands/mod.rs                # 添加 pub mod agent_commands
src-tauri/src/lib.rs                         # 注册新 Tauri commands + Specta 类型
```

### 13.3 模板文件（由 F7 Skills 模块提供）

```
src-tauri/templates/elfiee-client/
├── mcp.json                  # MCP 配置模板（F3-03 使用）
```

> **注意**：模板文件是 F7 Skills 模块（§3.3）的产出。Agent 模块假设这些模板在 agent.enable 执行时已可用。如果模板尚未就绪，可使用硬编码的 JSON 作为临时方案。

---

## 14. 依赖关系

### 14.1 上游依赖

| 依赖 | 模块 | 说明 |
| :--- | :--- | :--- |
| `.elf/` 初始化 | I10 (elf_meta.rs) | `.elf/Agents/elfiee-client/` 目录骨架必须已创建 |
| Block 快照 | I1 | `elfiee-client/SKILL.md` 等文件需要物理快照，symlink 才有内容可指向 |
| 目标项目 import | directory.import | 目标项目必须先作为 Dir Block 导入，才有 `target_project_id` |

### 14.2 下游依赖

| 被依赖方 | 模块 | 说明 |
| :--- | :--- | :--- |
| MCP Server | F4-F5 | agent.enable 注入 MCP 配置后，MCP Server 可运行才有意义 |
| Skills 模板 | F7 | SKILL.md 和 mcp.json 模板需在 enable 之前就绪 |
| Session 同步 | F10-F13 | Session 同步依赖 Agent 已启用（知道要监听哪个项目的 session） |

### 14.3 开发顺序建议

```
1. F1-01: AgentContents / AgentStatus / Payload 数据结构  (2h)
2. F3-03: MCP 配置合并器 (utils/mcp_config.rs)          (3h)
3. F1-02: agent.create Capability Handler + Tauri Command (4h)
4. F3-01: agent.enable Capability Handler + Tauri Command (3h)
5. F3-02: agent.disable Capability Handler + Tauri Command(3h)
```

---

## 15. 测试计划

### 15.1 单元测试

#### F1-01: 数据结构测试

| 测试 | 内容 |
| :--- | :--- |
| `test_agent_contents_serialization` | AgentContents ↔ JSON 往返 |
| `test_agent_status_serialization` | AgentStatus 枚举 rename_all = "lowercase" |
| `test_agent_create_v2_payload_default_name` | name 为 None 时行为 |
| `test_agent_enable_payload` | AgentEnablePayload 序列化 |
| `test_agent_disable_payload` | AgentDisablePayload 序列化 |

#### F1-02: agent.create Handler 测试

| 测试 | 内容 |
| :--- | :--- |
| `test_create_success` | 正常创建，验证 Event 结构 |
| `test_create_empty_target_project_id` | 空 target_project_id 失败 |
| `test_create_default_name` | 不提供 name 时默认为 "elfiee" |
| `test_create_custom_name` | 自定义 name |
| `test_create_event_structure` | 验证 Event value 包含完整 Block 初始状态 |

#### F3-01: agent.enable Handler 测试

| 测试 | 内容 |
| :--- | :--- |
| `test_enable_success` | Block 存在，状态从 Disabled 变为 Enabled |
| `test_enable_already_enabled` | 已启用时仍成功（幂等） |
| `test_enable_block_not_found` | Block 不存在时失败 |
| `test_enable_wrong_block_type` | block_type 不是 agent 时失败 |
| `test_enable_metadata_update` | 验证 metadata.updated_at 和 custom.last_enabled_at |

#### F3-02: agent.disable Handler 测试

| 测试 | 内容 |
| :--- | :--- |
| `test_disable_success` | 状态从 Enabled 变为 Disabled |
| `test_disable_already_disabled` | 已禁用时静默成功 |
| `test_disable_metadata_update` | 验证 metadata |

#### F3-03: MCP 配置合并器测试

| 测试 | 内容 |
| :--- | :--- |
| `test_merge_new_file` | 文件不存在时创建 |
| `test_merge_empty_file` | 空文件时创建结构 |
| `test_merge_existing_no_server` | 有 mcpServers 但无 elfiee |
| `test_merge_existing_with_server` | 已有 elfiee 时覆盖 |
| `test_merge_preserves_other_servers` | 不影响其他 Server 配置 |
| `test_merge_invalid_json` | JSON 损坏时报错 |
| `test_remove_existing` | 移除已有 server |
| `test_remove_nonexistent` | 移除不存在的 server 静默成功 |
| `test_remove_file_not_found` | 文件不存在静默成功 |
| `test_remove_preserves_other_servers` | 不影响其他 Server 配置 |
| `test_remove_leaves_empty_object` | 移除最后一个 server 后保留 `{}` |
| `test_resolve_template_elf_path` | `{elf_path}` 占位符替换 |
| `test_resolve_template_nested` | 嵌套 JSON 中的占位符替换 |

### 15.2 集成测试

| 测试 | 内容 |
| :--- | :--- |
| `test_create_enable_disable_lifecycle` | 完整的创建 → 启用 → 禁用生命周期 |
| `test_enable_creates_symlink` | 验证 symlink 实际创建 |
| `test_enable_writes_mcp_config` | 验证 mcp.json 正确更新 |
| `test_disable_cleans_symlink` | 验证 symlink 删除 |
| `test_disable_removes_mcp_config` | 验证 mcp.json 中 elfiee 被移除 |
| `test_duplicate_create_fails` | 同一项目重复创建失败 |
| `test_enable_idempotent` | 多次 enable 结果一致 |

### 15.3 授权测试

| 测试 | 内容 |
| :--- | :--- |
| `test_create_owner_authorized` | Block owner 可以创建 |
| `test_enable_owner_authorized` | Agent Block owner 可以 enable |
| `test_enable_non_owner_without_grant` | 非 owner 无权限被拒绝 |
| `test_enable_non_owner_with_grant` | 非 owner 有 grant 时可操作 |

---

## 16. 验收标准

### 16.1 功能验收

- [ ] `agent.create(target_project_id)` 创建 Agent Block 且自动执行 enable 逻辑
- [ ] `agent.enable(agent_block_id)` 创建 symlink 到 `.claude/skills/elfiee-client/`
- [ ] `agent.enable` 将 MCP 配置合并到 `.claude/mcp.json`
- [ ] `agent.disable(agent_block_id)` 清理 symlink 和 MCP 配置
- [ ] 重复 enable 已启用的 Agent → 更新配置（幂等）
- [ ] 重复 disable 已禁用的 Agent → 静默成功（幂等）
- [ ] 同一 (项目, editor) 对不可创建两个 Agent Block（唯一性约束）
- [ ] 目标项目无 `.claude/` 目录时报错并给出清晰提示

### 16.2 技术验收

- [ ] Agent 数据结构通过 Specta 自动生成 TypeScript 类型
- [ ] 所有 Capability Handler 遵循 `#[capability]` 宏模式
- [ ] Event 值遵循既有格式（entity/attribute/value/timestamp）
- [ ] MCP 配置合并器幂等且不破坏已有配置
- [ ] 跨平台 symlink（macOS/Linux: symlink, Windows: junction）
- [ ] 单元测试覆盖所有正向和异常路径
- [ ] Tauri Command 注册到 `lib.rs` 并在 `bindings.ts` 中可见

### 16.3 里程碑对照

对应 **M2: Agent 模块**（Week 2 Day 1-2）：
- ✓ 软连接创建成功
- ✓ MCP 配置幂等合并
- ✓ 重启后 Claude 可连接（通过 MCP Server，属 M1 验证）

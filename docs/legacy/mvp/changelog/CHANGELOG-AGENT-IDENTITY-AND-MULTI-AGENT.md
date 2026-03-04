# Changelog: Agent Identity Attribution & Multi-Agent 支持

> **分支**: `feat/agent-block`
> **日期**: 2026-02-02
> **变更规模**: 14 个文件修改（8 后端 + 6 前端）

---

## 概述

本次变更解决三个核心问题：

1. **MCP Agent 身份归因**：MCP 操作被错误归因到 GUI owner，而非实际执行操作的 bot editor
2. **多 Agent 支持**：同一项目只能创建一个 Agent Block 的限制被放宽为每个 bot editor 可独立创建
3. **Agent Toggle 简化**：Bot 编辑者无论是否已有 Agent Block，均显示 Agent 开关

---

## 1. MCP Agent 身份归因修复

### 问题

`ElfieeMcpServer.get_editor_id()`（`mcp/server.rs:301-305`）始终返回 GUI 的 `active_editor`（即人类 owner）。所有 MCP 工具调用经过 `execute_capability()` 时均使用此方法，导致所有操作被归因到 owner 身份。

### 解决方案

#### 1.1 `AgentContents` 新增 `editor_id` 字段

**文件**: `src-tauri/src/extensions/agent/mod.rs`

```rust
pub struct AgentContents {
    pub name: String,
    pub target_project_id: String,
    pub status: AgentStatus,
    /// Bot editor_id associated with this agent.
    /// Used by MCP server to attribute operations to the correct identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_id: Option<String>,
}
```

**向后兼容**：`Option<String>` + `skip_serializing_if` 确保不含此字段的旧 JSON 可正常反序列化为 `editor_id: None`。

同样更新了 `AgentCreateV2Payload`：

```rust
pub struct AgentCreateV2Payload {
    pub name: Option<String>,
    pub target_project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_id: Option<String>,
}
```

#### 1.2 MCP Server 新增 `resolve_agent_editor_id()`

**文件**: `src-tauri/src/mcp/server.rs`

新增方法扫描已启用的 Agent Block，提取 `editor_id`：

```rust
async fn resolve_agent_editor_id(&self, file_id: &str) -> Result<String, McpError> {
    // 扫描所有 block，查找 enabled 且有 editor_id 的 agent block
    // 如果找到 → 返回 bot editor_id
    // 否则 → 回退到 GUI active editor（兼容旧 agent）
}
```

`execute_capability()` 从 `self.get_editor_id()` 改为 `self.resolve_agent_editor_id().await`。

#### 1.3 创建流程贯通

| 层级 | 文件 | 变更 |
|------|------|------|
| Rust Handler | `agent_create.rs` | 将 `payload.editor_id` 写入 `AgentContents` |
| Frontend Store | `app-store.ts` | `createAgent()` 接受 `editorId?` 参数 |
| Frontend UI | `AddCollaboratorDialog.tsx` | 传递 `newEditor.editor_id` 到 `createAgent()` |

---

## 2. 多 Agent 支持

### 问题

原唯一性约束为 "一个项目一个 Agent"（仅比较 `target_project_id`），导致多个 bot editor 无法各自拥有独立的 Agent Block。

### 解决方案

**文件**: `src-tauri/src/commands/agent.rs`

唯一性检查从：
```rust
// 旧：一个项目一个 Agent
if contents.target_project_id == payload.target_project_id
```

改为：
```rust
// 新：一个 (项目, editor) 对一个 Agent
if contents.target_project_id == payload.target_project_id
    && contents.editor_id == payload.editor_id
```

| 场景 | 旧行为 | 新行为 |
|------|--------|--------|
| Bot A 创建 Agent for Project X | 成功 | 成功 |
| Bot B 创建 Agent for Project X | **失败** | **成功** |
| Bot A 再次创建 Agent for Project X | 失败 | 失败 |
| 无 `editor_id` 的旧 Agent（`None == None`） | N/A | 保持一个项目一个（`None` 自身相等） |

---

## 3. Agent Toggle 简化 & UI 改进

### 问题

原代码中 Agent 开关仅在 `isBot && agentBlock && agentStatus` 条件下显示，用户反馈为何需要先有 Agent Block 才能控制状态。

### 解决方案

#### 3.1 渲染条件简化

渲染条件从 `isBot && agentBlock && agentStatus` 简化为 `isBot`。

#### 3.2 自动创建 Agent

新增 `onCreateAgent` 回调，当 Bot 编辑者无 Agent Block 时，切换开关自动触发 Agent 创建：

```tsx
const handleToggleAgent = async () => {
  if (agentBlock && agentStatus && onToggleAgentStatus) {
    // Agent 已存在 → 切换 enable/disable
    await onToggleAgentStatus(agentBlock.block_id, agentStatus)
  } else if (!agentBlock && onCreateAgent) {
    // Agent 不存在 → 自动创建（默认 enabled）
    await onCreateAgent(editor.editor_id)
  }
}
```

#### 3.3 UI 视觉改进

| 元素 | 旧 | 新 |
|------|----|----|
| 图标 | 绿色圆点 | `Sparkles` 图标 |
| 标签 | "Agent" | "Agent Capabilities" |
| 状态文字 | "Enabled" / "Disabled" | "Active" / "Inactive" |
| 加载态 | 无 | `Loader2` 旋转动画 + "Updating..." |
| 容器 | 无边框 inline | 圆角边框卡片，启用时绿色主题 |
| 暗色模式 | 无 | `dark:border-green-900/50 dark:bg-green-900/20` |

---

## 4. 变更文件清单

### 后端（Rust）

| 文件 | 变更 |
|------|------|
| `src-tauri/src/extensions/agent/mod.rs` | `AgentContents` 和 `AgentCreateV2Payload` 新增 `editor_id` 字段 |
| `src-tauri/src/extensions/agent/agent_create.rs` | Handler 写入 `editor_id`，新增 2 个测试 |
| `src-tauri/src/mcp/server.rs` | 新增 `resolve_agent_editor_id()`，更新 `execute_capability()` |
| `src-tauri/src/commands/agent.rs` | 唯一性约束改为 `(project, editor)` 对 |
| `src-tauri/src/extensions/agent/agent_enable.rs` | 测试 fixture 包含 `editor_id` |
| `src-tauri/src/extensions/agent/agent_disable.rs` | 测试 fixture 包含 `editor_id` |
| `src-tauri/src/extensions/agent/tests.rs` | 序列化测试覆盖 `editor_id` 和向后兼容 |

### 前端（TypeScript/React）

| 文件 | 变更 |
|------|------|
| `src/lib/app-store.ts` | `createAgent()` 新增 `editorId?` 参数 |
| `src/components/permission/AddCollaboratorDialog.tsx` | 传递 `editor_id` 到 `createAgent()` |
| `src/components/permission/CollaboratorItem.tsx` | Agent Toggle 简化、UI 改进、`onCreateAgent` 回调 |
| `src/components/permission/CollaboratorList.tsx` | 新增 `handleCreateAgent` 回调 |
| `src/components/permission/CollaboratorItem.test.tsx` | 更新状态文字断言、新增 `onCreateAgent` 测试 |
| `src/components/permission/CollaboratorList.test.tsx` | 更新状态文字断言、更新 toggle 可见性测试 |

### 自动生成

| 文件 | 说明 |
|------|------|
| `src/bindings.ts` | 由 `pnpm tauri dev` 重新生成，包含 `editor_id` 字段 |

---

## 5. 测试

### Rust 测试

- 84 个 agent 测试全部通过
- 新增测试：
  - `test_agent_create_v2_with_editor_id`
  - `test_agent_create_v2_without_editor_id`
  - `test_agent_contents_without_editor_id`（向后兼容）
  - `test_agent_contents_backward_compat_no_editor_id`

### 前端测试

- 39 个 permission 组件测试全部通过（7 AddCollaboratorDialog + 8 CollaboratorList + 24 CollaboratorItem）

---

## 6. 边界情况

| 场景 | 处理 |
|------|------|
| 同一文件多个 enabled agent | `resolve_agent_editor_id` 返回第一个找到的。实际使用中每个外部项目对应一个 SSE 连接 |
| 旧 agent 无 `editor_id` | 回退到 GUI active editor（保持旧行为） |
| Bot editor 无足够权限 | CBAC 拒绝命令，返回 "not authorized" |
| `editor_id: None` 的重复检查 | `None == None` 为 true，保持旧的一个项目一个 agent 语义 |

---

**最后更新**: 2026-02-02

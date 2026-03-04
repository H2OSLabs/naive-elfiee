# 目标 API 接口定义

> 重构后 Elfiee 后端暴露的完整 API 清单。
> 所有 API 通过 Message Router 统一分发，可通过 Tauri IPC 或 WebSocket 调用。

---

## 一、API 设计原则

1. **User-story 驱动**：每个 API 的存在是为了满足至少一个 user-story testcase
2. **可组合**：复杂操作通过组合基础 API 实现，而非创建大粒度 API
3. **可独立测试**：每个 API 有独立的 unit test，无需启动完整应用
4. **统一消息格式**：JSON-RPC 2.0 风格，请求/响应/通知三种类型

---

## 二、消息格式

### 请求

```json
{
  "jsonrpc": "2.0",
  "id": "<request_id>",
  "method": "<api_method>",
  "params": { ... }
}
```

### 响应

```json
{
  "jsonrpc": "2.0",
  "id": "<request_id>",
  "result": { ... }
}
```

### 通知（Engine → Client）

```json
{
  "jsonrpc": "2.0",
  "method": "<notification_type>",
  "params": { ... }
}
```

---

## 三、核心 API（block.command 路径）

所有 Capability 操作统一走 `block.command` 方法。Engine 内部根据 `cap_id` 路由到对应 Handler。

### 请求格式

```json
{
  "method": "block.command",
  "params": {
    "elf_id": "project-a",
    "cmd_id": "<uuid>",
    "editor_id": "<editor_uuid>",
    "cap_id": "<capability_id>",
    "block_id": "<target_block_id>",
    "payload": { ... }
  }
}
```

### 3.1 Core Capabilities（保持不变）

| cap_id | target | payload | 返回 Event |
|--------|--------|---------|-----------|
| `core.create` | `core/*` | `{ block_type, name, contents?, children? }` | BlockCreated |
| `core.delete` | `core/*` | `{}` | BlockDeleted |
| `core.link` | `core/*` | `{ child_block_id, relation_type }` | BlockLinked |
| `core.unlink` | `core/*` | `{ child_block_id, relation_type }` | BlockUnlinked |
| `core.grant` | `core/*` | `{ target_editor_id, target_cap_id, target_block_id }` | GrantCreated |
| `core.revoke` | `core/*` | `{ target_editor_id, target_cap_id, target_block_id }` | GrantRevoked |
| `core.rename` | `core/*` | `{ new_name }` | BlockRenamed |
| `core.change_type` | `core/*` | `{ new_type, new_name? }` | BlockTypeChanged |
| `core.update_metadata` | `core/*` | `{ metadata }` | MetadataUpdated |
| `core.read` | `core/*` | `{}` | 无 Event（纯权限检查） |

### 3.2 Editor Capabilities（保持不变）

| cap_id | target | payload | 返回 Event |
|--------|--------|---------|-----------|
| `editor.create` | `system` | `{ name, editor_type }` | EditorCreated |
| `editor.delete` | `system` | `{ editor_id }` | EditorDeleted |

### 3.3 Document Extension（新增，合并 markdown + code）

| cap_id | target | payload | 返回 Event |
|--------|--------|---------|-----------|
| `document.write` | `document` | `{ content?, delta?, hash?, path?, size?, mime? }` | DocumentWritten (mode: full/delta/ref) |
| `document.read` | `document` | `{}` | DocumentRead（审计事件） |

**说明：**
- 替代原有的 `markdown.write/read` + `code.write/read`（4 个 → 2 个）
- 创建时通过 `core.create` 指定 `format` 字段（md / rs / py / toml 等）
- `delta` 模式的 payload 包含 diff 操作而非完整内容

### 3.4 Task Extension（精简）

| cap_id | target | payload | 返回 Event |
|--------|--------|---------|-----------|
| `task.write` | `task` | `{ description?, status?, assigned_to?, template? }` | TaskWritten |
| `task.read` | `task` | `{}` | TaskRead（审计事件） |

**移除：** `task.commit`（Git 操作委托给 AgentContext）

### 3.5 Agent Extension（精简）

| cap_id | target | payload | 返回 Event |
|--------|--------|---------|-----------|
| `agent.create` | `core/*` | `{ name, prompt, provider, model?, editor_id }` | AgentCreated |
| `agent.enable` | `agent` | `{}` | AgentEnabled |
| `agent.disable` | `agent` | `{}` | AgentDisabled |

**移除：** MCP 配置注入副作用。Handler 只生成 Event，不再启停 MCP 服务器。

### 3.6 Session Extension（新增）

| cap_id | target | payload | 返回 Event |
|--------|--------|---------|-----------|
| `session.append` | `session` | `{ entry_type, ...entry_fields }` | SessionAppended (mode: append) |

entry_type 三种：

| entry_type | 必填字段 |
|-----------|---------|
| `command` | `command`, `output`, `exit_code` |
| `message` | `role` (human/agent/system), `content` |
| `decision` | `action`, `related_blocks?` |

### 3.7 Checkpoint（新增）

| cap_id | target | payload | 返回 Event |
|--------|--------|---------|-----------|
| `core.checkpoint` | `core/*` | `{ block_ids? }` | CheckpointCreated |

---

## 四、查询 API

查询操作不产生 Event，通过 `block.query` 方法路由。

| method | params | 返回 |
|--------|--------|------|
| `block.query` | `{ elf_id, block_id }` | Block 完整状态 |
| `block.list` | `{ elf_id, block_type?, filter? }` | Block 列表 |
| `editor.list` | `{ elf_id }` | Editor 列表 |
| `editor.get` | `{ elf_id, editor_id }` | Editor 详情 |
| `grant.list` | `{ elf_id, editor_id?, block_id? }` | Grant 列表 |
| `event.list` | `{ elf_id, entity?, attribute?, limit? }` | Event 列表 |
| `event.state_at` | `{ elf_id, event_id }` | 指定事件时刻的状态快照 |

---

## 五、文件管理 API

.elf 文件级操作，不走 `block.command`，直接由 EngineManager 处理。

| method | params | 说明 |
|--------|--------|------|
| `elf.open` | `{ path }` | 打开 .elf/ 目录，启动 Actor |
| `elf.create` | `{ path, name? }` | 创建新的 .elf/ 目录 |
| `elf.close` | `{ elf_id }` | 关闭 Actor，释放资源 |
| `elf.list` | `{}` | 列出已打开的 .elf 文件 |
| `elf.info` | `{ elf_id }` | 文件信息（路径、block 数量等） |

---

## 六、通知类型

Engine → Client 推送，所有连接的客户端收到。

| method | params | 触发条件 |
|--------|--------|---------|
| `state.changed` | `{ elf_id, events }` | 任何 Command 处理成功后 |
| `command.rejected` | `{ cmd_id, reason }` | CBAC 授权失败 |
| `command.failed` | `{ cmd_id, error }` | Handler 执行失败 |

---

## 七、认证 API

仅 WebSocket Adapter 使用。

| method | params | 说明 |
|--------|--------|------|
| `auth.login` | `{ editor_id, credentials }` | 连接级认证 |
| `auth.result` | `{ success, editor_id }` | 认证结果 |

---

## 八、User-Story → API 映射示例

### 示例 1：用户打开 task-1 查看修改历史

```
1. elf.open({ path: "project.elf" })
2. block.query({ elf_id: "project-a", block_id: "task-1" })
3. event.list({ elf_id: "project-a", entity: "task-1" })
   → 返回所有关于 task-1 的 Event，包含 attribute（谁做了什么）
```

### 示例 2：Agent 执行任务并记录过程

```
1. auth.login({ editor_id: "coder-bot", credentials: ... })
2. block.command({ cap_id: "task.write", block_id: "task-1",
     payload: { status: "in_progress", assigned_to: "coder-bot" } })
3. -- Agent 在 AgentContext 中执行 I/O --
4. block.command({ cap_id: "session.append", block_id: "session-1",
     payload: { entry_type: "command", command: "cargo test", output: "...", exit_code: 0 } })
5. block.command({ cap_id: "document.write", block_id: "auth-rs",
     payload: { content: "fn main() { ... }" } })
6. block.command({ cap_id: "task.write", block_id: "task-1",
     payload: { status: "completed" } })
```

### 示例 3：PM 审查代码并记录决策

```
1. block.query({ block_id: "auth-rs" })
2. event.list({ entity: "auth-rs" })
3. block.command({ cap_id: "session.append", block_id: "review-session",
     payload: { entry_type: "decision", action: "approved",
       related_blocks: ["auth-rs", "task-1"] } })
```

---

## 九、Capability 数量对比

| 类别 | Phase 1 | 重构后 | 变化 |
|------|---------|--------|------|
| Core | 10 | 11 (+checkpoint) | +1 |
| Editor | 2 | 2 | 不变 |
| Document | — | 2 | 新增（合并 markdown 4 + code 2 = 6 → 2） |
| Task | 3 | 2 | -1 (移除 commit) |
| Agent | 3 | 3 | 不变（但移除副作用） |
| Session | — | 1 | 新增 |
| Directory | 7 | 0 | 全部移除 |
| Terminal | 4 | 0 | 全部移除 |
| **总计** | **33** | **21** | **-12** |

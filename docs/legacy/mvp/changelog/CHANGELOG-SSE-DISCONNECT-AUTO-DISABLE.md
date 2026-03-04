# Changelog: SSE 断连自动禁用 Agent

> **分支**: `feat/agent-block`
> **日期**: 2026-02-01
> **变更规模**: 3 个文件修改

---

## 概述

当所有 Claude Code SSE 客户端断开连接后，自动将已启用的 Agent Block 状态切换为 `Disabled`，并清理相关 I/O（symlink、MCP 配置）。

**解决的问题**: 此前 Claude Code 退出后，Agent Block 仍保持 `Enabled` 状态，symlink 和 `.mcp.json` 配置残留在目标项目中，导致状态与实际连接不一致。

---

## 实现方案

### 核心思路

rmcp 的 `SseServer::with_service()` 是一个 fire-and-forget 的连接循环，无法 hook 连接关闭事件。改为手动调用 `SseServer::next_transport()`（public API）构建自定义连接循环，在 `server.waiting().await` 返回（连接断开）后执行清理逻辑。

通过原子计数器跟踪活跃连接数，仅当所有连接归零时触发 agent 自动禁用，避免在多个 Claude Code 实例并存时误禁用。

### 连接生命周期

```
Client connects  ->  counter++  ->  serve MCP connection
                                         |
Client disconnects  <-  counter--  <-  server.waiting() returns
                                         |
                                    counter == 0 ?
                                    -> disable_all_agents()
```

---

## 变更文件

### 1. `src-tauri/src/state.rs`

新增 `sse_connection_count` 字段：

```rust
pub struct AppState {
    // ... existing fields ...

    /// Active MCP SSE connection count.
    /// Used to detect when all clients disconnect so we can auto-disable agent blocks.
    pub sse_connection_count: Arc<AtomicUsize>,
}
```

### 2. `src-tauri/src/commands/agent.rs`

将两个 helper 函数提升为 `pub(crate)` 可见性，供 `mcp::transport` 模块复用：

| 函数 | 变更 | 用途 |
|------|------|------|
| `get_external_path()` | `fn` -> `pub(crate) fn` | 从 directory block metadata 提取外部路径 |
| `perform_disable_io()` | `fn` -> `pub(crate) fn` | 移除 symlink + 清理 MCP 配置 |

### 3. `src-tauri/src/mcp/transport.rs`

**主要改动**：

1. **替换 `with_service` 为自定义连接循环**：手动消费 `next_transport()`，包裹连接生命周期跟踪
2. **连接计数**：每个 SSE 连接建立时 `fetch_add(1)`，断开时 `fetch_sub(1)`
3. **新增 `disable_all_agents()` 函数**：
   - 遍历所有已打开的 `.elf` 文件
   - 筛选 `block_type == "agent"` 且 `status == Enabled` 的 block
   - 发送 `agent.disable` 命令到 engine（更新 block 状态）
   - 执行 I/O 清理（移除 symlink、删除 `.mcp.json` 和 `.claude/mcp.json` 中的 elfiee 条目）

---

## 行为说明

| 场景 | 行为 |
|------|------|
| 单个 Claude Code 连接后断开 | Agent 自动禁用 |
| 多个 Claude Code 连接，其中一个断开 | Agent 保持启用（计数器 > 0） |
| 多个 Claude Code 连接，全部断开 | Agent 自动禁用 |
| Agent 已处于 Disabled 状态 | 跳过，不重复操作（幂等） |
| 无 active editor 的文件 | 跳过该文件 |

---

## 测试

- 编译通过：`cargo build` 成功
- 全部 409 个单元测试通过，0 回归

---

**最后更新**: 2026-02-01

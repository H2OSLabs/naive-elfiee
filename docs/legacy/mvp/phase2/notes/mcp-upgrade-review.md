# MCP 升级方案可行性评审

> 日期: 2026-01-30
> 对比文档: `plans/mcp-upgrade-plan.md` vs `task-and-cost_v3.md` Section 3.2

## 1. 任务覆盖度

| v3 原始任务 | v3 预估 | mcp-upgrade-plan 对应 | 状态 |
|---|---|---|---|
| F4-01 MCP Server 入口 | 2h | §5.1 standalone.rs CLI 入口 | 待实现 |
| F4-02 MCP 协议实现 | 5h | §3.3 SSE ✅ + §5.2 stdio 待实现 | 部分完成 |
| F4-03 execute_command tool | 3h | §2.2 的 26 个独立 Tools + elfiee_exec | 超额完成 |
| F5-01 Engine 独立模式 | 4h | §5.3 standalone.rs + WAL | 待实现 |
| F5-02 GUI EventStore 重载 | 1h | §5.4 reload_events() | 待实现 |

## 2. 结论：方案可行，范围已超出原计划

### 优势

1. **嵌入模式已完成** — v3 没有规划嵌入模式，这是额外收益（开发调试方便）
2. **26 个独立 Tools > 单一 execute_command** — AI agent 不需要记住 capability 名，工具发现性更好
3. **7 个 Resources** — v3 没有规划 Resources，这是 MCP 协议的正确用法（读写分离）
4. **rmcp 官方 SDK** — v3 原计划手写 JSON-RPC 协议（F4-02 `protocol.rs`），官方 SDK 更稳定

### 待实现部分（独立模式）

- standalone.rs + stdio_transport.rs + WAL 模式 + reload_events
- 这是 Phase 2 Agent 模块 (F1-F3) 的前置依赖，需要优先完成

### 潜在风险

1. **SQLite WAL 并发** — GUI Engine + MCP Engine 各自持有 StateProjector，内存状态不同步。reload_events 的触发时机是关键：计划中写的是"手动触发或定期轮询"，高频写入场景下可能不够及时
2. **双 Engine 冲突** — 两个 Engine 各自维护 vector clock。由于 editor_id 不同（GUI 用户 vs "mcp-agent"），vector clock 应独立，不会冲突

## 3. MCP 通信方向分析

### 已覆盖

| 方向 | 说明 |
|---|---|
| Claude → Elfiee（调用 Tools） | 26 个 MCP Tools + 7 Resources |
| 错误返回 | `CallToolResult::error` + MCP 错误码（project_not_open, block_not_found 等） |

### 未覆盖

| 方向 | 说明 |
|---|---|
| Elfiee → Claude（主动推送） | MCP Notifications 已标注"推迟" |
| Elfiee → Claude（发起请求） | 需要 Elfiee 作为 MCP Client，当前不在范围内 |

### 错误返回机制

MCP 方案中 `execute_capability` 有明确的错误返回路径：

```rust
match handle.process_command(cmd).await {
    Ok(events) => CallToolResult::success(...),
    Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
}
```

Claude 调用 Tool 失败后能看到错误原因，可据此调整策略重试。

## 4. 身份确认（Authentication）缺口

当前设计无认证：
- 传输层绑定 `127.0.0.1`（仅本机访问），无认证
- 应用层用 `get_active_editor()` 获取 editor_id，服务端自行决定身份
- 独立模式自动创建 `"mcp-agent"` editor 并授予所有权限
- 所有 MCP 操作以同一 editor 身份执行，无法区分不同 AI agent

桌面单用户场景可接受，多 agent 协作需要引入身份机制（Phase 3+）。

## 5. 从命令行操作 Elfiee

MCP 协议是给 AI 用的，不是给人用的。cli/ 已删除后，人工命令行操作有两个选项：

| 方案 | 说明 |
|---|---|
| 轻量 CLI wrapper | 封装 MCP SSE 的 JSON-RPC 调用 |
| 通过 Claude Code 操作 | Claude 自动发现 MCP Tools，用自然语言下达指令（Phase 2 核心场景） |

# Changelog: L3 Engine 清理 + L4 冗余 Extension 删除

> 日期：2026-03-02
> 分支：feat/refactor-plan
> 前置：L0 data-model, L1 event-system, L1 cbac, L2 elf-format

## 概述

Engine 层回归纯 Event Sourcing 执行核心（无 I/O 副作用），删除 Phase 1 遗留的 I/O Extension，精简 AppState 和 MCP Server。Elfiee 定位为纯被动 EventWeaver，I/O 操作委托给 AgentContext。

## 删除的 Extension 目录（3 个，~284KB，28 文件）

| 目录 | 大小 | 文件数 | 原因 |
|------|------|--------|------|
| `src/extensions/directory/` | 132KB | 12 | 文件系统操作 → AgentContext |
| `src/extensions/terminal/` | 72KB | 9 | PTY 终端 → AgentContext |
| `src/extensions/agent/` | 80KB | 7 | Per-agent MCP → WebSocket 架构 |

## Engine 清理

### 删除的 Engine 机制

| 机制 | 文件 | 说明 |
|------|------|------|
| `_block_dir` 注入 | `actor.rs` | 运行时注入临时目录路径到 block contents，持久化前 strip |
| `write_snapshots()` | `actor.rs` | 命令执行后写物理 block-{uuid}/body.{ext} 文件 |
| `write_block_snapshot()` | `utils/snapshot.rs`（删除） | write_snapshots 的底层实现，250 行 |

### 简化后的 `process_command()` 流程

```
1. 查找 block → 2. certificator 鉴权 → 3. 环检测(link)
→ 4. handler 执行 → 5. vector clock → 6. 冲突检测
→ 7. 持久化 → 8. apply state → 返回
```

移除的步骤：_block_dir 注入(2.5)、core.create _block_dir 注入(5.5)、strip _block_dir(7)、write_snapshots(10)

## AppState 精简

### 删除的字段

| 字段 | 类型 | 用途 |
|------|------|------|
| `agent_servers` | `Arc<DashMap<String, AgentServerHandle>>` | Per-agent MCP 服务器 |
| `next_agent_port` | `Arc<AtomicU16>` | Agent 端口分配 |
| `terminal_sessions` | `Arc<Mutex<HashMap<String, TerminalSession>>>` | PTY 会话 |
| `terminal_output_buffers` | `Arc<DashMap<String, Arc<Mutex<Vec<u8>>>>>` | 终端输出缓冲 |
| `sse_connection_count` | `Arc<AtomicUsize>` | SSE 连接计数 |

### 精简后的 AppState

```rust
pub struct AppState {
    pub engine_manager: EngineManager,
    pub files: Arc<DashMap<String, FileInfo>>,
    pub active_editors: Arc<DashMap<String, String>>,
    pub state_changed_tx: broadcast::Sender<String>,
}
```

## Task Extension 清理

| 删除文件 | 行数 | 内容 |
|----------|------|------|
| `extensions/task/git.rs` | 310 | git_exec, git_commit_flow |
| `extensions/task/git_hooks.rs` | 348 | inject/remove git hooks |

保留：task.write、task.read、task.commit（纯审计事件，无 git I/O）

## 删除的命令和文件

| 文件 | 行数 | 内容 |
|------|------|------|
| `commands/agent.rs` | 725 | agent_create/enable/disable Tauri 命令 |
| `commands/checkout.rs` | 297 | directory.export + 文件系统写出 |
| `commands/task.rs` | 380→2 | 4 个 git 命令清空 |

## Capability Registry 变更

### 删除的 Capability（16 个）

- Terminal: `terminal.init`, `terminal.execute`, `terminal.save`, `terminal.close`
- Directory: `directory.create`, `directory.delete`, `directory.rename`, `directory.write`, `directory.import`, `directory.export`, `directory.rename_with_type_change`
- Agent: `agent.create`, `agent.enable`, `agent.disable`

### 保留的 Capability（此时为 16 个，后续在 L4-extension 中重组为 15 个）

- Core（7）: `core.create`, `core.delete`, `core.link`, `core.unlink`, `core.write`, `core.grant`, `core.revoke`
- Editor（2）: `editor.create`, `editor.delete`
- Markdown（2）: `markdown.write`, `markdown.read` *(L4 中合并为 document.write/read)*
- Code（2）: `code.write`, `code.read` *(L4 中合并为 document.write/read)*
- Task（3）: `task.write`, `task.read`, `task.commit`

## MCP Server 变更

### 删除的 MCP Tool（~16 个）

- Directory: `elfiee_directory_create/delete/rename/write/import/export`
- Terminal: `elfiee_terminal_init/execute/save/close`
- capture_terminal_output 辅助方法

### 删除的 MCP 基础设施

- Per-agent MCP server（`transport.rs` 中 `start_agent_mcp_server`, `stop_agent_mcp_server`, `allocate_agent_port`）
- AgentContents 解析（`resolve_agent_editor_id` 简化为纯 active editor 查询）
- SSE 连接计数跟踪

### 保留的 MCP Tool（此时 ~19 个，后续在 L4-extension 中重组）

`elfiee_file_list`, `elfiee_block_list/get/create/delete/rename/link/unlink`, `elfiee_block_change_type` *(L4 中删除)*, `elfiee_markdown_read/write` *(L4 中改为 document)*, `elfiee_code_read/write` *(L4 中改为 document)*, `elfiee_grant/revoke`, `elfiee_editor_create/delete`, `elfiee_task_create/write/commit/link`, `elfiee_exec`

## 删除的集成测试

| 文件 | 内容 |
|------|------|
| `tests/elf_meta_integration.rs` | .elf/ Dir Block 初始化 |
| `tests/template_integration.rs` | 模板系统物理文件 |
| `tests/engine_block_dir_integration.rs` | _block_dir 注入验证 |
| `tests/snapshot_integration.rs` | Block 快照写入 |
| `tests/terminal_integration.rs` | Terminal 生命周期 |

保留：`commands_block_permissions.rs`（权限检查）、`relation_integration.rs`（关系系统）

## 删除的依赖

| Crate | 用途 |
|-------|------|
| `portable-pty` | Terminal PTY 会话（已在 L2 删除） |

## 验证结果

- `cargo check`: 零错误、零 warning
- `cargo test`: 241 单元测试 + 17 集成测试全部通过
- `grep -r "_block_dir" src/`: 无残留
- `grep -r "write_snapshot" src/`: 无残留
- `grep -r "agent_servers" src/`: 无残留
- `grep -r "terminal_sessions" src/`: 无残留

## 文件变更汇总

| 操作 | 文件数 | 说明 |
|------|--------|------|
| 物理删除 | 33 | 3 extension 目录(28) + task git(2) + agent cmd(1) + checkout(1) + snapshot(1) |
| 集成测试删除 | 5 | 依赖已删除代码的测试 |
| 修改 | 14 | mod.rs, registry, actor, state, lib, mcp/*, commands/* |
| 新建 | 1 | 本 changelog |

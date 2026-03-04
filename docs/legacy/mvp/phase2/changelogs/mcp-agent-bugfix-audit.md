# MCP Agent Bug 修复 + 全面审计

## 概述

对 MCP Server、SSE transport、权限系统、模板系统和 SKILL.md 进行全面审计，修复 6 个 bug（含 11 处 payload 字段不匹配）。核心问题：MCP tool handler 发送的 JSON payload 字段名与 capability handler 期望的 typed struct 不一致，导致所有通过 MCP 的 block 创建、链接、授权操作静默失败。

**分支**: `feat/agent-block-new`
**基准**: 上一轮 agent-block-revision-round3
**日期**: 2026-02-03

---

## Bug 总览

| # | 问题 | 根因 | 严重度 |
|---|------|------|--------|
| 1 | MCP tools payload 字段名与 capability handlers 不匹配 | MCP server 构造的 JSON key 与 Payload struct 字段不一致 | 高 |
| 2 | SSE 断连后 agent 立即被 disable，永远无法重连 | auto-disconnect 无 grace period，disable 做完整 cleanup | 高 |
| 3 | `.claude/skills/elfiee-client` 无法进入 | Bug 2 的连锁效应：rapid disable/enable 破坏 symlink | 中 |
| 4 | agent 创建的 block，GUI 用户看不到 | bot 是 block owner，人类 editor 无 grants | 中 |
| 5 | `mcp.json` 模板冗余 | 实际 MCP 配置由 `perform_enable_io` 动态生成 `.mcp.json` | 低 |
| 6 | MCP 断连后 Claude 直接修改文件 | SKILL.md 缺少 MCP 断连时的行为规则 | 中 |

---

## Fix 1: MCP Server Payload 全面修复 — **已完成** ✅

### 背景

MCP server 中每个 tool handler 构造的 `json!({...})` payload 字段名与 Rust typed payload struct 的字段名不一致。`serde_json::from_value()` 反序列化失败，导致所有关键操作（创建、链接、授权、终端等）通过 MCP 调用时全部报 "Invalid payload" 错误。

### 审计结果

| MCP Tool | MCP 发送字段 | Handler 期望字段 | 状态 |
|----------|-------------|-----------------|------|
| `elfiee_block_create` | `"type"` | `"block_type"` (CreateBlockPayload) | **BUG → 已修复** |
| `elfiee_task_create` | `"type"` | `"block_type"` (CreateBlockPayload) | **BUG → 已修复** |
| `elfiee_block_link` | `"child_id"` | `"target_id"` (LinkBlockPayload) | **BUG → 已修复** |
| `elfiee_block_unlink` | `"child_id"` | `"target_id"` (UnlinkBlockPayload) | **BUG → 已修复** |
| `elfiee_task_link` | `"child_id"` | `"target_id"` (LinkBlockPayload) | **BUG → 已修复** |
| `elfiee_grant` | `"editor_id"`, `"cap_id"` | `"target_editor"`, `"capability"`, `"target_block"` (GrantPayload) | **BUG → 已修复** |
| `elfiee_revoke` | `"editor_id"`, `"cap_id"` | `"target_editor"`, `"capability"`, `"target_block"` (GrantPayload) | **BUG → 已修复** |
| `elfiee_block_change_type` | `"new_type"` | `"block_type"` (ChangeTypePayload) | **BUG → 已修复** |
| `elfiee_terminal_init` | `{}` / `{"shell":...}` | `cols, rows, block_id, editor_id, file_id` (TerminalInitPayload) | **BUG → 已修复** |
| `elfiee_terminal_save` | `"content"` | `"saved_content"`, `"saved_at"` (TerminalSavePayload) | **BUG → 已修复** |
| `elfiee_editor_create` | `name` optional | `name` required (EditorCreatePayload) | **BUG → 已修复** |
| `elfiee_block_delete` | `{}` | `{}` | OK |
| `elfiee_block_rename` | `"name"` | `"name"` (RenamePayload) | OK |
| `elfiee_block_update_metadata` | `"metadata"` | `"metadata"` | OK |
| `elfiee_markdown_write` | `"content"` | `"content"` (MarkdownWritePayload) | OK |
| `elfiee_code_write` | `"content"` | `"content"` (CodeWritePayload) | OK |
| `elfiee_directory_*` | 各字段 | DirectoryPayloads | OK |
| `elfiee_task_write` | `"content"` | `"content"` (TaskWritePayload) | OK |
| `elfiee_terminal_execute` | `"command"` | `"command"` (TerminalExecutePayload) | OK |
| `elfiee_terminal_close` | `{}` | (empty) | OK |

### 修改

**文件**: `src-tauri/src/mcp/server.rs`

11 处 payload 字段名统一修正，使 MCP tool handler 发送的 JSON 与 capability handler 的 typed struct 完全匹配。

关键修改说明：
- `elfiee_terminal_init`: 重构为先 resolve `file_id` 和 `editor_id`，注入到 payload 中。`shell` 参数标记 TODO（TerminalInitPayload 尚无此字段）。
- `elfiee_terminal_save`: 自动生成 `saved_at` 时间戳（`crate::utils::time::now_utc()`）。
- `elfiee_editor_create`: `name` 从 optional 改为 fallback 到 `editor_id`，确保必填。
- `elfiee_grant` / `elfiee_revoke`: 新增 `target_block` 字段，使用 `input.block_id.clone()`。

---

## Fix 2: 移除 per-agent auto-disconnect — **已完成** ✅

### 背景

SSE 客户端断连时，MCP transport 层自动调用 `disable_single_agent` / `disable_all_agents`，触发完整的 agent disable 流程（停止 MCP server、删除 `.mcp.json`、删除 symlink）。Claude Code 的 SSE 连接可能因网络抖动、进程重启等原因短暂断开，此时 agent 被永久 disable，需要用户手动重新 enable。

### 设计原则

Agent 在 enabled 状态时，MCP server **永远运行**，不因 SSE 断连而停止。停止条件仅限：
- GUI 手动 disable
- 文件关闭 (`shutdown_agent_servers`)
- App 退出

### 修改

**文件**: `src-tauri/src/mcp/transport.rs`

1. **Management server**: 移除 `remaining == 0` 时的 `disable_all_agents` 调用
2. **Per-agent server**: 移除 `remaining == 0` 时的 `disable_single_agent` 调用
3. `disable_single_agent` 和 `disable_all_agents` 函数保留，标记 `#[allow(dead_code)]`
4. 更新模块级文档说明新行为

---

## Fix 3: Symlink 问题 — **随 Fix 2 自动解决** ✅

Fix 2 移除 auto-disconnect 后：
- Server 不停 → 端口不变 → `.mcp.json` 不被删
- Agent 不 disable → symlink 不被删
- Client 重连到同一端口 → 正常工作

无需额外修改。

---

## Fix 4: System Owner 全局授权 — **已完成** ✅

### 背景

Agent（Bot editor）创建的 block，owner 是 bot 的 `editor_id`。GUI 的人类用户（system editor）不是 owner，也没有显式 grant，因此无法查看或操作这些 block。

### 设计

`is_authorized()` 增加规则：`system_editor_id`（来自 `~/.elf/config.json`）**始终有所有权限**。

### 修改

**文件**: `src-tauri/src/engine/state.rs`

1. `StateProjector` 新增字段 `system_editor_id: Option<String>`
2. `new()` 初始化为 `None`
3. `is_authorized()` 最高优先级检查：

```rust
// 0. System owner always authorized
if let Some(ref sys_id) = self.system_editor_id {
    if editor_id == sys_id {
        return true;
    }
}
// 1. Block owner always authorized
// 2. Check explicit grants
```

**文件**: `src-tauri/src/engine/actor.rs`

在 `ElfileEngineActor::new()` 中注入：
```rust
state.system_editor_id = crate::config::get_system_editor_id().ok();
```

---

## Fix 5: 移除 `mcp.json` 模板 — **已完成** ✅

### 分析

- 模板 `mcp.json`（无 dot）在 bootstrap 时写入 `_block_dir/agents/elfiee-client/mcp.json`
- 实际 MCP 配置是 `{project_root}/.mcp.json`（有 dot），由 `perform_enable_io` 动态生成
- 模板通过 symlink 可见，但 Claude Code 不读取它
- 功能上完全冗余

### 修改

**文件**: `src-tauri/src/extensions/directory/elf_meta.rs`
- 从 `TEMPLATE_FILES` 中移除 `mcp.json` 条目
- `test_template_files_count`: 4 → 3
- 删除 `test_mcp_json_content_valid` 测试

**文件**: `templates/elf-meta/agents/elfiee-client/mcp.json`
- 删除此文件

**文件**: `tests/template_integration.rs`
- 删除 `test_mcp_json_exists_and_valid` 测试
- 删除 `test_mcp_json_has_elfiee_server_config` 测试

---

## Fix 6: SKILL.md 增加 MCP 断连规则 — **已完成** ✅

### 问题

MCP 连接失败时，Claude 会尝试直接在 `.claude/` 项目目录中修改文件，绕过 event sourcing。

### 修改

**文件**: `src-tauri/templates/elf-meta/agents/elfiee-client/SKILL.md`

在 "Prohibited Actions" 和 "The only exception" 之间，新增 `## MCP Connection Failure Protocol` 章节：

1. **STOP** 所有 Elfiee 操作
2. **DO NOT** 回退到文件系统工具
3. **DO NOT** 修改 `.claude/`、`.elf/` 或 block 目录中的文件
4. **REPORT** 连接失败给用户
5. **WAIT** 等待用户确认

附带说明：GUI 持有 event store 锁，直接文件修改会被覆盖。

---

## 修改文件清单

| 文件 | Fix | 修改内容 |
|------|-----|---------|
| `src-tauri/src/mcp/server.rs` | Fix 1 | 11 处 payload 字段名修复 |
| `src-tauri/src/mcp/transport.rs` | Fix 2 | 移除 per-agent 和 management auto-disable (2 处)，保留函数标记 dead_code |
| `src-tauri/src/engine/state.rs` | Fix 4 | `StateProjector` 增加 `system_editor_id` 字段，`is_authorized()` bypass |
| `src-tauri/src/engine/actor.rs` | Fix 4 | 注入 `system_editor_id` |
| `src-tauri/src/extensions/directory/elf_meta.rs` | Fix 5 | 移除 mcp.json 模板条目 + 更新测试 |
| `templates/elf-meta/agents/elfiee-client/mcp.json` | Fix 5 | 删除文件 |
| `templates/elf-meta/agents/elfiee-client/SKILL.md` | Fix 6 | 增加 MCP 断连规则 |
| `tests/template_integration.rs` | Fix 5 | 删除 2 个 mcp.json 测试 |

---

## 验证结果

- `cargo test` — **391 unit + 54 integration = 全部通过**
- `cargo clippy` — **无新增警告**（16 个 pre-existing 未改变）
- 手动验证场景：
  - `elfiee_task_create` / `elfiee_block_create` 正常创建 block
  - `elfiee_block_link` / `elfiee_grant` 等工具正常工作
  - SSE 断连后 agent server 继续运行，client 可重连
  - `.claude/skills/elfiee-client` 正常显示为可进入的文件夹
  - GUI 用户（system owner）能看到 agent 创建的所有 block
  - MCP 断连时 Claude 不再尝试直接修改文件

---

**最后更新**: 2026-02-03

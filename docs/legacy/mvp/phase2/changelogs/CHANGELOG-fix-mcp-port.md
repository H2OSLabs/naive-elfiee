# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/).

---

## [Unreleased]

### Fixed

- **MCP 端口持久化 — reserved port 泄漏** (`src-tauri/src/commands/agent.rs`)
  Phase 2 恢复中，若 preferred port bind 成功或失败，该端口仍留在 `reserved_ports` 中，
  导致后续 agent 的 `allocate_agent_port` 无法使用已释放的端口。
  修复：每个 agent 处理完毕后立即 `reserved_ports.remove()`。

- **删除 editor 后仍可通过 owner 授权** (`src-tauri/src/engine/state.rs`)
  `is_authorized()` 的 block owner 检查不验证 editor 是否已被删除。
  修复：新增 `deleted_editors: HashSet<String>`，`editor.delete` 事件写入后，
  该 editor 在 `is_authorized` 中被拒绝（即使仍为 block owner）。

### Added

- **MCP 端口持久化 — 两阶段恢复** (`src-tauri/src/commands/agent.rs`, `src-tauri/src/mcp/transport.rs`, `src-tauri/src/extensions/agent/mcp_config.rs`)
  Elfiee 重启后 agent MCP server 会分配新端口，导致 Claude Code 连接断开。
  新增 `read_existing_port()` 从 `.mcp.json` 读取上次端口号；
  `start_agent_mcp_server` 新增 `preferred_port` / `reserved_ports` 参数；
  `recover_agent_servers` 改为两阶段（Phase 1 收集保留端口，Phase 2 优先绑定原端口）。

- **端口分配单元测试** (`src-tauri/src/mcp/transport.rs`)
  新增 4 个 `allocate_agent_port` 测试：基本分配、跳过 reserved、跳过 in-use、
  同时跳过 reserved + in-use。

- **URL 解析边界测试** (`src-tauri/src/extensions/agent/mcp_config.rs`)
  新增 2 个 `read_existing_port` 测试：`localhost` URL 和 IPv6 `[::1]` URL
  均正确返回 `None`。

- **MCP 副作用 TODO 标注** (`src-tauri/src/commands/agent.rs`, `src-tauri/src/commands/task.rs`, `src-tauri/src/mcp/server.rs`)
  在 `do_agent_create`、`do_agent_enable`、`do_agent_disable`、`do_commit_task`、
  `elfiee_exec` 处标注 `TODO(mcp-side-effects)`，提示这些 I/O 副作用无法通过
  `elfiee_exec` 触达，需要专用 MCP tool。

### Improved

- **Phase 1 恢复可观测性** (`src-tauri/src/commands/agent.rs`)
  `recover_agent_servers` Phase 1 中 `serde_json::from_value` 失败时从静默 `continue`
  改为 `log::warn!`，输出跳过的 agent block_id 和错误原因。

- **URL 解析可观测性** (`src-tauri/src/extensions/agent/mcp_config.rs`)
  `read_existing_port` 解析 URL 失败时输出 `log::debug!`，包含原始 URL 和文件路径。
  扩展 doc comment 说明 URL 格式与 `build_elfiee_server_config` 一致。

- **设计决策文档化**
  - `deleted_editors` 添加 append-only 安全设计说明 (`state.rs`)
  - `allocate_agent_port` 添加竞态条件安全性分析注释 (`transport.rs`)
  - `start_agent_mcp_server` 的 `reserved_ports` 参数说明非 recovery 传空集合即可 (`transport.rs`)

- **elfiee-client skill 文档** (`templates/elf-meta/agents/elfiee-client/`)
  - 修复 "Two connection modes" → "Three connection modes"
  - 重写 MCP Connection Failure 异常处理协议
  - `elfiee_directory_export` 从绝对禁止改为推荐 `elfiee_task_commit`
  - 新增 `elfiee_exec` 限制表（✅/❌ dedicated tool 对比）
  - `capabilities.md` 移除冗余警告，保持纯能力参考文档

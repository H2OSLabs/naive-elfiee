# Changelog: L4 Communication Layer 重构

> 日期：2026-03-02
> 分支：feat/refactor-plan
> 测试：264 pass（247 unit + 4 block-perm + 12 relation + 1 doc-test）

## 设计决策

- **Elfiee Core 独立于前端**：engine + MCP server 可以无 GUI 运行
- **双部署模式**：`elf serve`（headless）和 Tauri 桌面（内嵌 Core + GUI）
- **Per-connection identity**：每个 MCP SSE 连接有独立的 ElfieeMcpServer 实例
- **Tauri IPC 是效率优化**：保留 specta 类型绑定，不影响 Core 独立性
- **broadcast channel 统一通知**：engine 变更 → broadcast → MCP notification + Tauri Event

## 代码变更

### 新建文件
| 文件 | 说明 |
|------|------|
| `src/bin/serve.rs` | `elf-serve` headless 二进制入口 |
| `src/commands/project.rs` | 共享项目管理逻辑（open/close/seed） |

### 修改文件
| 文件 | 变更 |
|------|------|
| `Cargo.toml` | 添加 `clap` 依赖 + `[[bin]] elf-serve` |
| `src/mcp/server.rs` | per-connection `connection_editor_id`，新增 `elfiee_auth`/`elfiee_open`/`elfiee_close` tools |
| `src/mcp/transport.rs` | 通知扇出：broadcast → `peer.notify_resource_list_changed()` |
| `src/mcp/mod.rs` | 架构文档更新，新增 `not_authenticated()` 错误辅助，删除 `no_active_editor()` |
| `src/commands/file.rs` | 委托给 `project.rs` 共享逻辑，清理未使用 imports |
| `src/commands/mod.rs` | 添加 `pub mod project` |
| `docs/mvp/frame/concepts/communication.md` | 全面更新（§一-§十） |

### 删除/清理
- `resolve_agent_editor_id()` — 被 `get_connection_editor_id()` 替代
- `get_editor_id()` — 被 `get_connection_editor_id()` 替代
- `no_active_editor()` — MCP 不再使用 GUI active_editor
- 所有 "open in GUI" / "GUI must be running" 错误消息

## MCP Tools 变更

### 新增
- `elfiee_auth` — 认证连接（绑定 editor_id）
- `elfiee_open` — 打开/创建 .elf 项目
- `elfiee_close` — 关闭 .elf 项目

### 修改
- 所有写操作 tool 改用 `get_connection_editor_id()`（不再依赖 GUI active_editor）
- `elfiee_file_list` 显示 `connection_editor` 而非 `active_editor`
- ServerInfo instructions 更新为 auth → open → operate 三步流程

## 验证
- `cargo check` — 零错误（lib + elf-serve + elfiee-app）
- `cargo test` — 264 tests 全部通过
- `elf-serve --help` — 正常输出
- `elf-serve` — 正常启动监听 :47200

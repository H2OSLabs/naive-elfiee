# Elfiee MCP 升级方案

> 版本: 3.0
> 创建日期: 2026-01-28
> 更新日期: 2026-01-30
> 状态: In Progress

## 1. 概述

### 1.1 背景

Elfiee 原先通过 HTTP API (Port 47100) + `cli/` 模块与 AI Agent 通信。该架构已被废弃并替换为 MCP 协议。

**旧架构问题（已解决）：**
1. ~~**多余的中间层**：`cli/` 模块仅作为 HTTP → EngineManager 的转发层~~ → ✅ 已删除 cli/ 和 ipc/
2. ~~**非标准协议**：AI Agent 需要阅读 SKILL.md 学习 curl 调用方式~~ → ✅ MCP 自动发现 Tools
3. ~~**与前端通信冗余**：前端通过 Tauri Commands 直接调用后端，HTTP API 从未被前端使用~~ → ✅ HTTP API 已删除

**当前状态：** MCP 嵌入模式已实现（SSE 传输，与 GUI 同进程）。
**待实现：** MCP 独立模式（stdio 传输，独立 Engine，Phase 2 Agent 集成所需）。

MCP (Model Context Protocol) 是 Anthropic 设计的标准协议，用于 AI 模型与外部工具/资源的交互。升级到 MCP 带来的优势：

1. **标准化**: Claude Code 等工具原生支持 MCP，无需自定义 SKILL.md ✅
2. **类型安全**: MCP 提供 JSON Schema 验证 ✅
3. **更好的发现性**: AI 可以自动发现可用的 tools 和 resources ✅
4. **双向通信**: 支持服务端主动推送通知（Notifications 推迟）

### 1.2 架构对比

```
旧架构 (已删除):
┌─────────────────────────────────────────────────┐
│                Elfiee GUI 进程                    │
│                                                   │
│  React ──Tauri Commands──► Rust Backend           │
│                               │                   │
│  Claude ──curl──► HTTP :47100 ──► cli/ ──► EngineManager
│  Code              ipc/server.rs   handler.rs     │
└─────────────────────────────────────────────────┘
  ❌ cli/ 是多余的中间层
  ❌ ipc/ HTTP API 前端从不使用
  ❌ Agent 需要学习 SKILL.md 中的 curl 格式

当前架构 (✅ 已实现 — 嵌入模式):
┌─────────────────────────────────────────────────┐
│                Elfiee GUI 进程                    │
│                                                   │
│  React ──Tauri Commands──► Rust Backend           │
│                               │                   │
│  Claude ──MCP SSE──► MCP Server ──► EngineManager │
│  Code     (独立端口)    mcp/server.rs  (直接调用) │
└─────────────────────────────────────────────────┘
  ✅ MCP Tools 直接调用 EngineManager，无中间层
  ✅ 前端通信方式不变（Tauri Commands）
  ✅ AI 原生支持 MCP，自动发现 Tools
  ✅ 删除 cli/ 和 ipc/ 模块
  ❌ 依赖 GUI 运行，Agent 无法独立使用

目标架构 (🔲 待实现 — 双模式):
┌─────────────────────────────────────────────────┐
│  模式 A: 嵌入 GUI 进程（现有）                     │
│  Claude ──MCP SSE:47200──► mcp/server.rs         │
│                              ↓                    │
│                         Arc<AppState>             │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│  模式 B: 独立进程（Phase 2 新增）                  │
│  Claude ──MCP stdio──► elfiee mcp-server         │
│                          ↓                        │
│                    独立 Engine (WAL mode)          │
│                    直接打开 .elf 文件               │
└─────────────────────────────────────────────────┘
  ✅ 无需 GUI 运行
  ✅ agent.enable 注入到 .claude/mcp.json
  ✅ SQLite WAL 模式支持 GUI 和 MCP 并发写入
  ✅ 标准 stdio 传输，兼容所有 MCP 客户端
```

### 1.3 核心设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| **传输协议 (嵌入模式)** | SSE | MCP Server 与 GUI 同进程，共享 AppState |
| **传输协议 (独立模式)** | stdio | 标准 MCP 传输，agent.enable 注入时使用 |
| **进程模型** | 双模式 | 嵌入模式用于开发调试，独立模式用于 Agent 集成 |
| **前端通信** | Tauri Commands（不变） | 前端不需要 HTTP API |
| **MCP → Engine** | 直接调用 | 不经过 cli/ 或 ipc/，无中间层 |
| **并发写入** | SQLite WAL | GUI 和独立 MCP 可同时写入同一个 .elf |

### 1.4 重构范围

| 模块 | 操作 | 状态 |
|------|------|------|
| `src-tauri/src/ipc/` | **整个删除** — 前端用 Tauri Commands，MCP 独立运行 | ✅ 已完成 |
| `src-tauri/src/cli/` | **整个删除** — MCP Tools 直接调 EngineManager | ✅ 已完成 |
| `src-tauri/src/mcp/` | **重写** — 独立 SSE Server，直接调引擎，自带错误码 | ✅ 已完成 |
| `docs/skills/elfiee-dev/SKILL.md` | **简化** — 只需 MCP 配置说明 | ✅ 已完成 |
| `src-tauri/src/mcp/standalone.rs` | **新增** — 独立 Engine 模式 (无需 GUI) | 🔲 待实现 |
| `src-tauri/src/mcp/stdio_transport.rs` | **新增** — stdio 传输层 | 🔲 待实现 |
| `src-tauri/src/engine/event_store.rs` | **修改** — 启用 WAL 模式 | 🔲 待实现 |
| `src-tauri/src/commands/reload.rs` | **新增** — GUI EventStore 重载 | 🔲 待实现 |

## 2. MCP 概念映射

### 2.1 核心概念对应

| MCP 概念 | Elfiee 对应 | 说明 |
|----------|-------------|------|
| **Tool** | Capability | 可执行操作 (markdown.write, directory.create) |
| **Resource** | Block/File | 可读取数据 (elfiee://project/block/{id}) |
| **Prompt** | - | 暂不使用 |
| **Root** | Project (.elf file) | MCP 的文件系统根 |

### 2.2 Tool 映射（完整列表）

将现有 Capabilities 映射为 MCP Tools：

#### 2.2.1 文件操作

| Tool | 描述 | 参数 |
|------|------|------|
| `elfiee_file_list` | 列出已打开的 .elf 文件 | - |

#### 2.2.2 Block 操作

| Tool | 描述 | 参数 |
|------|------|------|
| `elfiee_block_list` | 列出项目中所有 blocks | `project` |
| `elfiee_block_get` | 获取 block 详情 | `project`, `block_id` |
| `elfiee_block_create` | 创建新 block | `project`, `name`, `block_type`, `parent_id?` |
| `elfiee_block_delete` | 删除 block | `project`, `block_id` |
| `elfiee_block_rename` | 重命名 block | `project`, `block_id`, `name` |
| `elfiee_block_link` | 添加 block 关系 | `project`, `parent_id`, `child_id`, `relation` |
| `elfiee_block_unlink` | 移除 block 关系 | `project`, `parent_id`, `child_id`, `relation` |
| `elfiee_block_change_type` | 改变 block 类型 | `project`, `block_id`, `new_type` |
| `elfiee_block_update_metadata` | 更新 block 元数据 | `project`, `block_id`, `metadata` |

#### 2.2.3 Markdown 操作

| Tool | 描述 | 参数 |
|------|------|------|
| `elfiee_markdown_read` | 读取 markdown 内容 | `project`, `block_id` |
| `elfiee_markdown_write` | 写入 markdown 内容 | `project`, `block_id`, `content` |

#### 2.2.4 Code 操作

| Tool | 描述 | 参数 |
|------|------|------|
| `elfiee_code_read` | 读取代码内容 | `project`, `block_id` |
| `elfiee_code_write` | 写入代码内容 | `project`, `block_id`, `content` |

#### 2.2.5 Directory 操作

| Tool | 描述 | 参数 |
|------|------|------|
| `elfiee_directory_create` | 创建文件/目录 | `project`, `block_id`, `path`, `type`, `source`, `content?`, `block_type?` |
| `elfiee_directory_delete` | 删除文件/目录 | `project`, `block_id`, `path` |
| `elfiee_directory_write` | 更新目录索引 | `project`, `block_id`, `entries`, `source?` |
| `elfiee_directory_rename` | 重命名文件/目录 | `project`, `block_id`, `old_path`, `new_path` |
| `elfiee_directory_import` | 从文件系统导入 | `project`, `block_id`, `source_path`, `target_path?` |
| `elfiee_directory_export` | 导出到文件系统 | `project`, `block_id`, `target_path`, `source_path?` |

#### 2.2.6 Terminal 操作

| Tool | 描述 | 参数 |
|------|------|------|
| `elfiee_terminal_init` | 初始化终端 | `project`, `block_id`, `shell?` |
| `elfiee_terminal_execute` | 执行终端命令 | `project`, `block_id`, `command` |
| `elfiee_terminal_save` | 保存终端会话 | `project`, `block_id`, `content` |
| `elfiee_terminal_close` | 关闭终端 | `project`, `block_id` |

#### 2.2.7 权限操作

| Tool | 描述 | 参数 |
|------|------|------|
| `elfiee_grant` | 授予权限 | `project`, `block_id`, `editor_id`, `cap_id` |
| `elfiee_revoke` | 撤销权限 | `project`, `block_id`, `editor_id`, `cap_id` |

#### 2.2.8 Editor 操作

| Tool | 描述 | 参数 |
|------|------|------|
| `elfiee_editor_create` | 创建编辑者 | `project`, `editor_id`, `name?` |
| `elfiee_editor_delete` | 删除编辑者 | `project`, `editor_id` |

#### 2.2.9 通用操作

| Tool | 描述 | 参数 |
|------|------|------|
| `elfiee_exec` | 执行任意 capability | `project`, `capability`, `block_id?`, `payload?` |

**总计: 26 个 MCP Tools**

### 2.3 Resource 映射（完整列表）

MCP Resources 用于读取数据（只读），Tools 用于执行操作。

#### 2.3.1 静态 Resources

| URI | 名称 | 描述 |
|-----|------|------|
| `elfiee://files` | Open Files | 已打开的 .elf 文件列表 |
| `elfiee://editors` | All Editors | 所有编辑者列表 |

#### 2.3.2 动态 Resources (Templates)

| URI Pattern | 名称 | 描述 |
|-------------|------|------|
| `elfiee://{project}/blocks` | Project Blocks | 项目中所有 blocks |
| `elfiee://{project}/block/{block_id}` | Block Content | 单个 block 详情 |
| `elfiee://{project}/block/{block_id}/content` | Block Raw Content | block 内容（markdown/code text） |
| `elfiee://{project}/grants` | Project Grants | 项目权限表 |
| `elfiee://{project}/events` | Event Log | 项目事件日志 |

**总计: 7 个 MCP Resources**

## 3. 技术方案

### 3.1 依赖

使用官方 Rust MCP SDK: [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk)

```toml
# Cargo.toml
[dependencies]
rmcp = { version = "0.5", features = ["server", "transport-sse-server"] }
schemars = "1"
async-trait = "0.1"
tokio-util = "0.7"
axum = "0.8"
```

### 3.2 模块结构

```
src-tauri/src/
├── mcp/                           # MCP 模块
│   ├── mod.rs                    # ✅ 模块入口 + 错误码定义
│   ├── server.rs                 # ✅ MCP Server 实现（26 Tools + Resources）
│   ├── transport.rs              # ✅ SSE 传输层（独立 HTTP Server, port 47200）
│   ├── standalone.rs             # 🔲 独立模式入口（F4-01: elfiee mcp-server --elf）
│   └── stdio_transport.rs        # 🔲 stdio 传输层（F4-02: JSON-RPC over stdin/stdout）
├── engine/
│   ├── ...                       # ✅ 现有 Engine 代码不变
│   ├── standalone.rs             # 🔲 独立 Engine（F5-01: 无 GUI, WAL 模式）
│   └── event_store.rs            # 🔲 修改：启用 WAL 模式（F5-01）
├── commands/
│   ├── ...                       # ✅ 现有 Tauri Commands 不变
│   └── reload.rs                 # 🔲 EventStore 重载（F5-02: reload_events()）
├── models/                       # 不变
├── extensions/                   # 不变
├── state.rs                      # ✅ 已修改（Clone 支持）
├── lib.rs                        # ✅ 已修改（移除 cli/ipc，启动 MCP）
└── main.rs                       # 🔲 修改：CLI 参数解析（F4-01）
```

**已删除的目录：**
- `src-tauri/src/ipc/` — ✅ 已删除
- `src-tauri/src/cli/` — ✅ 已删除

### 3.3 MCP Server 实现

MCP Tools **直接调用 EngineManager**，不经过任何中间层：

```rust
// src/mcp/server.rs
use crate::models::Command;
use crate::state::AppState;
use rmcp::{tool, tool_handler, tool_router};
use std::sync::Arc;

#[derive(Clone)]
pub struct ElfieeMcpServer {
    app_state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl ElfieeMcpServer {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state, tool_router: Self::tool_router() }
    }

    // ── Helper: project path → file_id ──

    fn get_file_id(&self, project: &str) -> Result<String, McpError> {
        let files = self.app_state.list_open_files();
        for (file_id, path) in &files {
            if path == project {
                return Ok(file_id.clone());
            }
        }
        Err(McpError::invalid_request(
            format!("Project not open: {}. Open it in Elfiee GUI first.", project),
            None,
        ))
    }

    // ── Helper: execute capability directly on EngineManager ──

    async fn execute_capability(
        &self,
        project: &str,
        capability: &str,
        block_id: Option<String>,
        payload: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(project)?;
        let editor_id = self.app_state
            .get_active_editor(&file_id)
            .ok_or_else(|| McpError::invalid_request("No active editor", None))?;
        let handle = self.app_state.engine_manager
            .get_engine(&file_id)
            .ok_or_else(|| McpError::invalid_request("Engine not found", None))?;

        let cmd = Command::new(
            editor_id,
            capability.to_string(),
            block_id.unwrap_or_default(),
            payload,
        );

        match handle.process_command(cmd).await {
            Ok(events) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "success": true,
                    "events": events.len(),
                })).unwrap(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Error: {}", e)
            )])),
        }
    }

    // ── Tools（全部直接调 EngineManager）──

    #[tool(description = "List all currently open .elf files")]
    async fn elfiee_file_list(&self) -> Result<CallToolResult, McpError> {
        let files = self.app_state.list_open_files();
        // ... 直接返回
    }

    #[tool(description = "List all blocks in a project")]
    async fn elfiee_block_list(&self, Parameters(input): Parameters<ProjectInput>) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let handle = self.app_state.engine_manager.get_engine(&file_id)
            .ok_or_else(|| McpError::invalid_request("Engine not found", None))?;
        let blocks = handle.get_all_blocks().await;
        // ... 直接返回
    }

    #[tool(description = "Write markdown content")]
    async fn elfiee_markdown_write(&self, Parameters(input): Parameters<ContentWriteInput>) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project, "markdown.write",
            Some(input.block_id), json!({ "content": input.content }),
        ).await
    }

    // ... 其他 Tools 同理
}

#[tool_handler]
impl ServerHandler for ElfieeMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "elfiee".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "Elfiee MCP Server for .elf file operations. \
                 Use elfiee_file_list to see open files, then use other tools."
                    .to_string(),
            ),
            ..Default::default()
        }
    }
}
```

### 3.4 MCP 错误码

在 `mcp/mod.rs` 中定义，不依赖 `ipc/protocol.rs`：

```rust
// src/mcp/mod.rs
pub mod server;
pub mod transport;

pub use server::ElfieeMcpServer;

use rmcp::ErrorData as McpError;

/// MCP 错误构造辅助函数
pub fn project_not_open(project: &str) -> McpError {
    McpError::invalid_request(
        format!("Project not open: {}. Open it in Elfiee GUI first.", project),
        None,
    )
}

pub fn block_not_found(block_id: &str) -> McpError {
    McpError::invalid_request(
        format!("Block not found: {}", block_id),
        None,
    )
}

pub fn engine_not_found(file_id: &str) -> McpError {
    McpError::invalid_request(
        format!("Engine not found for file: {}", file_id),
        None,
    )
}

pub fn no_active_editor(file_id: &str) -> McpError {
    McpError::invalid_request(
        format!("No active editor for file: {}", file_id),
        None,
    )
}

pub fn invalid_payload(err: impl std::fmt::Display) -> McpError {
    McpError::invalid_params(format!("Invalid payload: {}", err), None)
}
```

### 3.5 传输层：独立 SSE Server

MCP Server 独立运行自己的 HTTP Server，不挂在任何已有的 Router 上：

```rust
// src/mcp/transport.rs
use super::ElfieeMcpServer;
use crate::state::AppState;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// MCP SSE Server 默认端口
pub const MCP_PORT: u16 = 47200;

/// 启动独立的 MCP SSE Server
///
/// 在 Tauri setup 中调用，作为后台任务运行。
/// MCP Server 与 GUI 同进程，共享 AppState。
pub async fn start_mcp_server(app_state: Arc<AppState>, port: u16) -> Result<(), String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let config = SseServerConfig {
        bind: addr,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(Duration::from_secs(30)),
    };

    let (sse_server, router) = SseServer::new(config);

    // 注册 MCP 服务：每个连接创建一个新的 ElfieeMcpServer 实例（共享 AppState）
    let _ct = sse_server.with_service(move || ElfieeMcpServer::new(app_state.clone()));

    println!("MCP Server starting on http://{}", addr);
    println!("  GET  /sse      - SSE connection");
    println!("  POST /message  - MCP messages");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("MCP: Failed to bind to port {}: {}", port, e))?;

    axum::serve(listener, router)
        .await
        .map_err(|e| format!("MCP Server error: {}", e))?;

    Ok(())
}
```

### 3.6 Tauri 集成

在 `lib.rs` 的 setup 中启动 MCP Server：

```rust
// src/lib.rs
pub mod capabilities;
pub mod commands;
pub mod config;
pub mod elf;
pub mod engine;
pub mod extensions;
pub mod mcp;          // MCP 模块
pub mod models;
pub mod state;
pub mod utils;
// 删除: pub mod cli;
// 删除: pub mod ipc;

use state::AppState;

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .manage(extensions::terminal::TerminalState::new())
        .setup(|app| {
            // 启动 MCP Server（独立端口，后台运行）
            let app_state: tauri::State<AppState> = app.state();
            let mcp_state = Arc::new((*app_state).clone());

            tauri::async_runtime::spawn(async move {
                let port = mcp::transport::MCP_PORT;
                if let Err(e) = mcp::transport::start_mcp_server(mcp_state, port).await {
                    eprintln!("MCP Server error: {}", e);
                    // MCP 启动失败不影响 GUI 正常使用
                }
            });

            Ok(())
        });

    // ... Tauri Commands 注册（不变）
}
```

## 4. 状态共享机制

### 4.1 模式 A: 嵌入模式（✅ 已实现）

MCP Server 与 Tauri GUI 运行在**同一个进程**中，共享同一个 `AppState`：

```
elfiee.exe 进程内:
┌──────────────────────────────────────────────┐
│                                               │
│  React  ──Tauri Cmd──►  commands/*.rs         │
│                              │                │
│  Claude ──MCP SSE──►  mcp/server.rs           │
│  Code                        │                │
│                              ▼                │
│              Arc<AppState> (同一实例)          │
│              ├── files: { a.elf, b.elf }      │
│              ├── active_editors: { ... }       │
│              └── engine_manager               │
│                  ├── engine_a                  │
│                  └── engine_b                  │
└──────────────────────────────────────────────┘
```

**效果：**
- 用户在 GUI 中打开 a.elf 和 b.elf
- Claude Code 调用 `elfiee_file_list` → 返回 `[a.elf, b.elf]`
- Claude Code 调用 `elfiee_block_list(project="a.elf")` → 返回 a.elf 的 blocks
- GUI 创建新 block → MCP 立即可见（同一个 EngineManager）
- MCP 写入内容 → GUI 通过 Tauri Events 收到通知并刷新

**前提：** 必须先启动 Elfiee GUI 才能使用 MCP。

### 4.2 模式 B: 独立模式（🔲 待实现 — Phase 2 F4/F5）

> 对应 Phase 2 开发计划: F4-01, F4-02, F5-01, F5-02

MCP Server 作为**独立进程**运行，嵌入自己的 Engine 实例：

```
elfiee mcp-server --elf /path/to/project.elf
┌──────────────────────────────────────────────┐
│  独立进程 (无 GUI)                             │
│                                               │
│  Claude ──MCP stdio──►  mcp/server.rs         │
│  Code                        │                │
│                              ▼                │
│              独立 Engine 实例                   │
│              ├── 直接打开 .elf (解压到临时目录) │
│              ├── EventStore (WAL 模式)         │
│              ├── StateProjector               │
│              └── 自动创建 editor               │
└──────────────────────────────────────────────┘
```

**与嵌入模式的并发共存：**
```
同一个 .elf 文件:

┌─────────────────┐    ┌──────────────────────┐
│  Elfiee GUI     │    │  MCP 独立进程          │
│  (如果运行)      │    │  (agent.enable 启动)  │
│                 │    │                       │
│  Engine A       │    │  Engine B              │
│  ↓              │    │  ↓                    │
│  EventStore     │    │  EventStore            │
│  (WAL mode)     │    │  (WAL mode)            │
│       │         │    │       │                │
└───────│─────────┘    └───────│────────────────┘
        │                      │
        └──────────┬───────────┘
                   ▼
           _eventstore.db
           (SQLite WAL 支持并发写入)
```

**设计要点：**
- MCP 独立进程嵌入完整的 Engine 实例，无需 GUI
- EventStore 启用 WAL 模式，支持 GUI 和 MCP 同时写入
- GUI 提供 `reload_events()` 命令，检测外部 MCP 的修改并刷新内存状态
- 通过 stdio 传输（标准 MCP 协议），agent.enable 写入 `.claude/mcp.json`

**使用场景：**
- `agent.enable` 注入 MCP 配置后，Claude Code 启动独立 MCP Server
- 不需要用户先打开 GUI
- 适合 CI/CD、自动化场景

## 5. 独立模式技术方案（🔲 待实现）

> 对应 Phase 2 开发计划: F4-01 ~ F4-03, F5-01, F5-02

### 5.1 MCP Server CLI 入口（F4-01）

**需求：** 支持 `elfiee mcp-server --elf {path}` 命令，启动独立 MCP Server。

```rust
// src/mcp/standalone.rs (新建)
use crate::engine::{Engine, EventStore};
use crate::mcp::server::ElfieeMcpServer;
use std::path::PathBuf;

/// 独立模式 MCP Server
///
/// 不依赖 GUI，直接打开 .elf 文件并创建 Engine 实例。
/// 通过 stdin/stdout 与 MCP 客户端通信。
pub async fn run_standalone(elf_path: PathBuf) -> Result<(), String> {
    // 1. 打开 .elf 文件（解压到临时目录）
    // 2. 创建 EventStore（启用 WAL 模式）
    // 3. 构建 StateProjector
    // 4. 自动创建默认 editor（"mcp-agent"）
    // 5. 构建 AppState（单文件模式）
    // 6. 创建 ElfieeMcpServer
    // 7. 启动 stdio 传输
    todo!()
}
```

**命令行接口：**
```bash
# 独立模式启动
elfiee mcp-server --elf /path/to/project.elf

# agent.enable 注入到 .claude/mcp.json 的配置
{
  "mcpServers": {
    "elfiee": {
      "command": "elfiee",
      "args": ["mcp-server", "--elf", "/path/to/project.elf"]
    }
  }
}
```

### 5.2 stdio 传输层（F4-02）

**需求：** 使用标准 MCP stdio 传输协议（JSON-RPC over stdin/stdout）。

```rust
// src/mcp/stdio_transport.rs (新建)
use crate::mcp::server::ElfieeMcpServer;
use rmcp::transport::stdio;

/// 启动 stdio 传输的 MCP Server
///
/// 通过 stdin 接收 JSON-RPC 请求，stdout 返回响应。
/// 这是 agent.enable 注入到 .claude/mcp.json 时使用的传输方式。
pub async fn start_stdio_server(server: ElfieeMcpServer) -> Result<(), String> {
    // rmcp 提供 stdio 传输支持
    // 需要 rmcp features: ["server", "transport-stdio"]
    todo!()
}
```

**Cargo.toml 变更：**
```toml
# 需要额外添加 stdio 传输 feature
rmcp = { version = "0.5", features = ["server", "transport-sse-server", "transport-io"] }
```

### 5.3 Engine 独立模式（F5-01）

**需求：** MCP Server 嵌入独立 Engine 实例，无需 GUI。

```rust
// src/engine/standalone.rs (新建)
use crate::engine::{EventStore, StateProjector, Engine};
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;

/// 为 MCP Server 创建独立 Engine
///
/// 1. 打开 .elf 文件（解压到临时目录）
/// 2. 修改 EventStore 启用 WAL 模式
/// 3. 构建 StateProjector，无需 GUI
/// 4. 返回可供 MCP Server 使用的 AppState
pub async fn create_standalone_engine(elf_path: PathBuf) -> Result<Arc<AppState>, String> {
    // 核心变更：EventStore 启用 WAL 模式
    // .journal_mode(SqliteJournalMode::Wal)
    // 支持多进程并发写入（GUI + MCP 独立进程）
    todo!()
}
```

**关键技术点：**
- EventStore 必须启用 SQLite WAL 模式：`.journal_mode(SqliteJournalMode::Wal)`
- WAL 模式允许 GUI 进程和 MCP 独立进程同时读写同一个 `_eventstore.db`
- 独立 Engine 自动创建默认 editor（"mcp-agent"），并授予所有权限
- .elf 文件解压到临时目录后，Engine 直接操作该目录

### 5.4 GUI EventStore 重载（F5-02）

**需求：** GUI 可以检测并加载外部 MCP 进程写入的新 Events。

```rust
// src/commands/reload.rs (新建)
use crate::state::AppState;

/// Tauri 命令：重新加载 Events
///
/// 当外部 MCP 进程修改了 .elf 文件后，GUI 可以调用此命令
/// 重新从 EventStore 加载所有 Events，重建 StateProjector。
#[tauri::command]
pub async fn reload_events(
    file_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // 1. 获取 Engine Handle
    // 2. 重新从 EventStore 加载所有 Events
    // 3. 重建 StateProjector（内存状态）
    // 4. 广播 state_changed 事件到前端
    todo!()
}
```

**触发时机：**
- 用户手动触发（UI 按钮或 Tauri 命令）
- 后续可改为定期轮询（如每 5 秒检查一次 `_eventstore.db` 的 modification time）

## 6. 配置

### 6.1 嵌入模式 MCP 配置（✅ 已实现）

项目级配置 `.mcp.json`（项目根目录），适用于开发调试：

```json
{
  "mcpServers": {
    "elfiee": {
      "type": "sse",
      "url": "http://127.0.0.1:47200/sse"
    }
  }
}
```

> 注：Claude Code 的项目级 MCP 配置放在 `.mcp.json`（项目根目录），不是 `.claude/mcp.json`。

### 6.2 独立模式 MCP 配置（🔲 待实现）

由 `agent.enable` 注入到外部项目 `.claude/mcp.json`：

```json
{
  "mcpServers": {
    "elfiee": {
      "command": "elfiee",
      "args": ["mcp-server", "--elf", "/path/to/project.elf"]
    }
  }
}
```

> 注：独立模式使用 stdio 传输（`command` + `args`），不需要固定端口。Claude Code 自动管理进程生命周期。

### 6.3 端口说明

| 用途 | 端口 | 模式 | 说明 |
|------|------|------|------|
| MCP SSE Server | 47200 | 嵌入模式 | GUI 同进程，AI Agent 通过 SSE 连接 |
| MCP stdio | N/A | 独立模式 | 无端口，Claude Code 通过 stdin/stdout 通信 |

> 注：原 47100 端口（IPC HTTP API）已删除。前端通过 Tauri Commands 通信，不需要 HTTP 端口。

## 7. 文件变更清单

### 7.1 已完成的变更

| 文件/目录 | 操作 | 状态 |
|-----------|------|------|
| `src-tauri/src/ipc/` (整个目录) | 删除 | ✅ 已完成 |
| `src-tauri/src/cli/` (整个目录) | 删除 | ✅ 已完成 |
| `src-tauri/src/mcp/mod.rs` | 重写 | ✅ 已完成 |
| `src-tauri/src/mcp/server.rs` | 重写 | ✅ 已完成 (26 Tools + Resources) |
| `src-tauri/src/mcp/transport.rs` | 重写 | ✅ 已完成 (独立 SSE Server) |
| `src-tauri/src/lib.rs` | 修改 | ✅ 已完成 |
| `src-tauri/src/state.rs` | 修改 | ✅ 已完成 (Clone) |
| `src-tauri/src/engine/manager.rs` | 修改 | ✅ 已完成 (Clone) |
| `src-tauri/Cargo.toml` | 修改 | ✅ 已完成 |
| `.mcp.json` | 修改 | ✅ 已完成 |
| `docs/skills/elfiee-dev/SKILL.md` | 重写 | ✅ 已完成 |

### 7.2 待实现的变更（独立模式）

| 文件/目录 | 操作 | 对应任务 | 说明 |
|-----------|------|----------|------|
| `src-tauri/src/mcp/standalone.rs` | **新建** | F4-01 | 独立 MCP Server 入口，解析 CLI 参数 |
| `src-tauri/src/mcp/stdio_transport.rs` | **新建** | F4-02 | stdio 传输层 (JSON-RPC over stdin/stdout) |
| `src-tauri/src/engine/standalone.rs` | **新建** | F5-01 | 独立 Engine：打开 .elf、WAL 模式、无 GUI |
| `src-tauri/src/engine/event_store.rs` | **修改** | F5-01 | 启用 WAL 模式 (`.journal_mode(SqliteJournalMode::Wal)`) |
| `src-tauri/src/commands/reload.rs` | **新建** | F5-02 | GUI EventStore 重载命令 `reload_events()` |
| `src-tauri/src/mcp/mod.rs` | **修改** | F4-01 | 添加 `pub mod standalone` 和 `pub mod stdio_transport` |
| `src-tauri/Cargo.toml` | **修改** | F4-02 | 添加 `transport-io` feature 到 rmcp |
| `src-tauri/src/main.rs` | **修改** | F4-01 | 添加 CLI 参数解析，支持 `mcp-server` 子命令 |

### 7.3 不变

| 文件/目录 | 说明 |
|-----------|------|
| `src-tauri/src/commands/` | Tauri Commands，前端用，不受影响 |
| `src-tauri/src/models/` | 数据模型 |
| `src-tauri/src/extensions/` | 扩展系统 |
| `src/` (前端) | 不受影响 |

## 8. 实现优先级

### 8.1 P0 - 核心框架 ✅ 已完成

- [x] 删除 `ipc/` 模块
- [x] 删除 `cli/` 模块
- [x] 更新 `lib.rs`：移除 cli/ipc 引用
- [x] 重写 `mcp/mod.rs`：错误码定义
- [x] 重写 `mcp/transport.rs`：独立 SSE Server
- [x] 重写 `mcp/server.rs`：Tools 直接调 EngineManager
- [x] `lib.rs` setup 中启动 MCP Server
- [x] 验证编译通过

### 8.2 P1 - 核心 Tools（AI 必需）✅ 已完成

**Block 操作:**
- [x] `elfiee_file_list`
- [x] `elfiee_block_list`
- [x] `elfiee_block_get`
- [x] `elfiee_block_create`
- [x] `elfiee_block_delete`

**内容操作:**
- [x] `elfiee_markdown_read`
- [x] `elfiee_markdown_write`
- [x] `elfiee_code_read`
- [x] `elfiee_code_write`

**Directory 操作:**
- [x] `elfiee_directory_create`
- [x] `elfiee_directory_delete`
- [x] `elfiee_directory_rename`

### 8.3 P2 - 完整 Tools ✅ 已完成

**Block 高级操作:**
- [x] `elfiee_block_rename`
- [x] `elfiee_block_link`
- [x] `elfiee_block_unlink`
- [x] `elfiee_block_change_type`
- [x] `elfiee_block_update_metadata`

**Directory 高级操作:**
- [x] `elfiee_directory_write`
- [x] `elfiee_directory_import`
- [x] `elfiee_directory_export`

**Terminal 操作:**
- [x] `elfiee_terminal_init`
- [x] `elfiee_terminal_execute`
- [x] `elfiee_terminal_save`
- [x] `elfiee_terminal_close`

**权限操作:**
- [x] `elfiee_grant`
- [x] `elfiee_revoke`
- [x] `elfiee_editor_create`
- [x] `elfiee_editor_delete`

**通用:**
- [x] `elfiee_exec`

### 8.4 P3 - Resources & 增强 ✅ 部分完成

- [x] Resources: `elfiee://files`, `elfiee://{project}/blocks`
- [x] Resources: `elfiee://{project}/block/{id}`
- [x] Resources: `elfiee://{project}/grants`, `elfiee://{project}/events`
- [ ] Notifications（状态变更推送）— 推迟到后续版本
- [x] SKILL.md 更新

### 8.5 P4 - 独立模式（🔲 待实现 — Phase 2 核心需求）

> **重要**：独立模式是 Phase 2 Agent 模块 (F1-F3) 的前置依赖。
> `agent.enable` 注入到 `.claude/mcp.json` 的配置需要 `elfiee mcp-server --elf {path}` 命令。

**独立 MCP Server CLI (F4-01):**
- [ ] 解析 CLI 参数：`elfiee mcp-server --elf {path}`
- [ ] `main.rs` 添加子命令分发（GUI 模式 vs MCP Server 模式）

**stdio 传输 (F4-02):**
- [ ] 实现 `mcp/stdio_transport.rs`
- [ ] Cargo.toml 添加 `transport-io` feature
- [ ] JSON-RPC over stdin/stdout 通信

**execute_command tool (F4-03):**
- [x] 已超额实现：26 个独立 Tools（比原计划的单一 `execute_command` 更优）
- [x] 保留 `elfiee_exec` 作为通用入口（等效于 `execute_command`）

**Engine 独立模式 (F5-01):**
- [ ] 实现 `engine/standalone.rs`：为 MCP Server 创建独立 Engine
- [ ] 打开 .elf 文件（解压到临时目录）
- [ ] EventStore 启用 WAL 模式（`SqliteJournalMode::Wal`）
- [ ] 构建 StateProjector，无需 GUI
- [ ] 自动创建默认 editor 并授予权限

**GUI EventStore 重载 (F5-02):**
- [ ] 实现 `commands/reload.rs`：Tauri 命令 `reload_events()`
- [ ] 从 EventStore 重新加载所有 Events
- [ ] 重建 StateProjector
- [ ] 广播 `state_changed` 事件到前端

### 8.6 P5 - 测试与验证

- [x] MCP 端点可用性验证
- [x] `cargo check` 编译通过
- [ ] 独立模式端到端测试
- [ ] GUI + 独立 MCP 并发写入测试（WAL 模式验证）
- [ ] 完整功能测试（GUI 手动验证）

## 9. SKILL.md 更新（✅ 已完成）

升级后大幅简化：

```markdown
---
name: elfiee-system
description: "[System-level] How AI agents interact with .elf files via MCP."
---

# Elfiee System Interface

**CRITICAL**: When working with `.elf` files, use Elfiee MCP Server. NEVER use filesystem commands.

## Prerequisites

1. Elfiee GUI must be running (嵌入模式) 或 MCP Server 已配置 (独立模式)
2. MCP 配置（二选一）：

### 嵌入模式（开发调试用）
`.mcp.json`（项目根目录）：
```json
{
  "mcpServers": {
    "elfiee": {
      "type": "sse",
      "url": "http://127.0.0.1:47200/sse"
    }
  }
}
```

### 独立模式（agent.enable 注入）
`.claude/mcp.json`（外部项目目录）：
```json
{
  "mcpServers": {
    "elfiee": {
      "command": "elfiee",
      "args": ["mcp-server", "--elf", "/path/to/project.elf"]
    }
  }
}
```

## Forbidden Operations

| Instead of | Use |
|------------|-----|
| `cat`, `ls`, `find` on .elf | `elfiee_block_list`, `elfiee_block_get` |
| `echo >`, `touch`, `mkdir` | `elfiee_directory_create` |
| `rm`, `rmdir` | `elfiee_directory_delete` |
| `git add/commit` on .elf internals | Never - .elf manages its own history |
```

> 注：具体 Tool 列表不再需要写在 SKILL.md 中，Claude Code 通过 MCP 协议自动发现。

## 10. 与 Phase 2 开发计划的对应关系

本文档对应 Phase 2 开发计划 (`docs/mvp/phase2/task-and-cost_v3.md`) 中的 **3.2 MCP Server 模块（15 人时）** 和相关模块。

### 10.1 任务对照表

| Phase 2 任务 | 本文档章节 | 实现状态 | 说明 |
|-------------|-----------|---------|------|
| **F4-01** MCP Server 入口 | 5.1 | 🔲 待实现 | `elfiee mcp-server --elf {path}` CLI 命令 |
| **F4-02** MCP 协议实现 | 3.3 + 5.2 | ✅ 部分完成 | SSE 已实现, stdio 待实现 |
| **F4-03** execute_command tool | 2.2 | ✅ 已超额完成 | 26 个独立 Tools + `elfiee_exec` 通用入口 |
| **F5-01** Engine 独立模式 | 5.3 | 🔲 待实现 | standalone.rs + WAL 模式 |
| **F5-02** GUI EventStore 重载 | 5.4 | 🔲 待实现 | reload_events() Tauri 命令 |

### 10.2 架构差异说明

Phase 2 计划原定 MCP Server 为**独立进程**（stdio 传输 + 独立 Engine）。当前实现为**嵌入模式**（SSE 传输 + 共享 AppState）。两种模式各有优势：

| 维度 | 嵌入模式 (已实现) | 独立模式 (待实现) |
|------|-----------------|-----------------|
| **依赖** | 需要 GUI 运行 | 不需要 GUI |
| **状态一致性** | 实时同步（同一进程） | 需要 WAL + reload |
| **使用场景** | 开发调试 | Agent 集成、CI/CD |
| **传输** | SSE (HTTP) | stdio (标准 MCP) |
| **Phase 2 Agent 集成** | ❌ 不满足 | ✅ agent.enable 所需 |

**结论：** 嵌入模式适合当前开发阶段。独立模式是 Phase 2 Agent 模块（F1-F3）的前置依赖，需要优先实现。

### 10.3 跨模型 MCP 兼容性

| 平台 | MCP 支持 | 配置格式 | 传输方式 | 备注 |
|------|---------|---------|---------|------|
| **Claude Code** | ✅ 原生 | JSON (`mcpServers`) | SSE / stdio | Phase 2 主要目标 |
| **OpenAI Codex** | ✅ 原生 | TOML (`mcp_servers`) | stdio | 格式略有不同 |
| **Qwen Agent** | ✅ 框架支持 | Python/JSON | stdio | 通过 Qwen-Agent 框架 |
| **DeepSeek** | ✅ MCP Server | JSON | stdio | 标准 MCP 协议 |

**Phase 2 策略**：仅支持 Claude Code，使用标准 MCP 协议实现（SSE + stdio 双模式）。
**Phase 3+ 扩展**：添加配置适配器，支持 TOML/Python 等格式输出，实现跨模型兼容。

## 11. 风险与缓解

| 风险 | 影响 | 缓解措施 | 状态 |
|------|------|----------|------|
| rmcp crate 不成熟 | API 变动、功能缺失 | 锁定版本 0.5，备选方案：手动实现 MCP 协议 | ✅ 已验证可用 |
| MCP SSE 端口冲突 | Server 启动失败 | 启动失败不阻止 GUI，日志提示 | ✅ 已实现 |
| 删除 cli/ipc 后遗漏引用 | 编译失败 | 编译验证 + 全局搜索确认无残留引用 | ✅ 已验证 |
| SQLite WAL 并发写入 | 数据竞争 | WAL 模式 + reload 机制 | 🔲 待验证 |
| stdio 传输稳定性 | MCP 连接中断 | rmcp 内置重连机制 + 错误日志 | 🔲 待验证 |
| 独立模式 .elf 文件锁 | 多进程访问冲突 | SQLite WAL + 文件级别锁检测 | 🔲 待验证 |

## 12. 参考资料

- [MCP 规范](https://spec.modelcontextprotocol.io/)
- [官方 Rust MCP SDK (rmcp)](https://github.com/modelcontextprotocol/rust-sdk)
- [rmcp 使用指南](https://hackmd.io/@Hamze/S1tlKZP0kx)
- [SSE MCP Server with OAuth in Rust](https://www.shuttle.dev/blog/2025/08/13/sse-mcp-server-with-oauth-in-rust)
- [Phase 2 开发计划](../mvp/phase2/task-and-cost_v3.md) — Section 3.2 MCP Server 模块

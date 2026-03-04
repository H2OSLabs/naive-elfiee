# Elfiee MCP 升级方案

> 版本: 2.0
> 创建日期: 2026-01-28
> 更新日期: 2026-01-29
> 状态: Draft

## 1. 概述

### 1.1 背景

当前 Elfiee 通过 HTTP API (Port 47100) + `cli/` 模块与 AI Agent 通信。这套架构存在以下问题：

1. **多余的中间层**：`cli/` 模块仅作为 HTTP → EngineManager 的转发层
2. **非标准协议**：AI Agent 需要阅读 SKILL.md 学习 curl 调用方式
3. **与前端通信冗余**：前端通过 Tauri Commands 直接调用后端，HTTP API 从未被前端使用

MCP (Model Context Protocol) 是 Anthropic 设计的标准协议，用于 AI 模型与外部工具/资源的交互。升级到 MCP 可以：

1. **标准化**: Claude Code 等工具原生支持 MCP，无需自定义 SKILL.md
2. **类型安全**: MCP 提供 JSON Schema 验证
3. **更好的发现性**: AI 可以自动发现可用的 tools 和 resources
4. **双向通信**: 支持服务端主动推送通知

### 1.2 架构对比

```
当前架构:
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

新架构:
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
```

### 1.3 核心设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| **传输协议** | SSE | MCP Server 与 GUI 同进程，共享 AppState |
| **进程模型** | 嵌入 GUI 进程 | GUI 打开的文件，MCP 直接可见 |
| **前端通信** | Tauri Commands（不变） | 前端不需要 HTTP API |
| **MCP → Engine** | 直接调用 | 不经过 cli/ 或 ipc/，无中间层 |

### 1.4 重构范围

| 模块 | 操作 |
|------|------|
| `src-tauri/src/ipc/` | **整个删除** — 前端用 Tauri Commands，MCP 独立运行 |
| `src-tauri/src/cli/` | **整个删除** — MCP Tools 直接调 EngineManager |
| `src-tauri/src/mcp/` | **重写** — 独立 SSE Server，直接调引擎，自带错误码 |
| `docs/skills/elfiee-dev/SKILL.md` | **简化** — 只需 MCP 配置说明 |

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
├── mcp/                      # MCP 模块（重写）
│   ├── mod.rs               # 模块入口 + 错误码定义
│   ├── server.rs            # MCP Server 实现（Tools 直接调 EngineManager）
│   └── transport.rs         # SSE 传输层（独立 HTTP Server）
├── engine/                   # 不变
├── models/                   # 不变
├── extensions/               # 不变
├── commands/                 # 不变（Tauri Commands，前端用）
├── state.rs                  # 不变
├── lib.rs                    # 修改：删除 cli/ipc 引用，setup 中启动 MCP
└── main.rs                   # 不变
```

**删除的目录：**
- `src-tauri/src/ipc/` — 整个删除（server.rs, protocol.rs, registry.rs, mod.rs）
- `src-tauri/src/cli/` — 整个删除（handler.rs, parser.rs, output.rs, mod.rs）

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

### 4.1 同进程共享

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

### 4.2 效果

- 用户在 GUI 中打开 a.elf 和 b.elf
- Claude Code 调用 `elfiee_file_list` → 返回 `[a.elf, b.elf]`
- Claude Code 调用 `elfiee_block_list(project="a.elf")` → 返回 a.elf 的 blocks
- GUI 创建新 block → MCP 立即可见（同一个 EngineManager）
- MCP 写入内容 → GUI 通过 Tauri Events 收到通知并刷新

### 4.3 前提

必须先启动 Elfiee GUI 才能使用 MCP。这符合实际使用场景：用户在 GUI 中打开项目，然后 Claude Code 通过 MCP 协作操作。

## 5. 配置

### 5.1 Claude Code MCP 配置

项目级配置 `.mcp.json`（项目根目录）：

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

### 5.2 端口说明

| 用途 | 端口 | 说明 |
|------|------|------|
| MCP SSE Server | 47200 | AI Agent（Claude Code）连接 |

> 注：原 47100 端口（IPC HTTP API）已删除。前端通过 Tauri Commands 通信，不需要 HTTP 端口。

## 6. 文件变更清单

### 6.1 删除

| 文件/目录 | 理由 |
|-----------|------|
| `src-tauri/src/ipc/` (整个目录) | 前端用 Tauri Commands，MCP 独立运行 |
| `src-tauri/src/cli/` (整个目录) | MCP Tools 直接调 EngineManager |

### 6.2 重写

| 文件 | 变更 |
|------|------|
| `src-tauri/src/mcp/mod.rs` | 新增错误码定义，更新 exports |
| `src-tauri/src/mcp/server.rs` | 移除 cli/ipc 依赖，Tools 直接调 EngineManager |
| `src-tauri/src/mcp/transport.rs` | 改为独立 SSE Server（不挂在 ipc/server 上） |

### 6.3 修改

| 文件 | 变更 |
|------|------|
| `src-tauri/src/lib.rs` | 删除 `pub mod cli`/`pub mod ipc`，setup 中启动 MCP |
| `src-tauri/Cargo.toml` | 清理不再需要的依赖 |

### 6.4 不变

| 文件/目录 | 说明 |
|-----------|------|
| `src-tauri/src/commands/` | Tauri Commands，前端用，不受影响 |
| `src-tauri/src/engine/` | 核心引擎，MCP 直接调用 |
| `src-tauri/src/models/` | 数据模型 |
| `src-tauri/src/extensions/` | 扩展系统 |
| `src-tauri/src/state.rs` | AppState，MCP 和 GUI 共享 |
| `src/` (前端) | 不受影响 |

## 7. 实现优先级

### 7.1 P0 - 核心框架

- [ ] 删除 `ipc/` 模块
- [ ] 删除 `cli/` 模块
- [ ] 更新 `lib.rs`：移除 cli/ipc 引用
- [ ] 重写 `mcp/mod.rs`：错误码定义
- [ ] 重写 `mcp/transport.rs`：独立 SSE Server
- [ ] 重写 `mcp/server.rs`：Tools 直接调 EngineManager
- [ ] `lib.rs` setup 中启动 MCP Server
- [ ] 验证编译通过

### 7.2 P1 - 核心 Tools（AI 必需）

**Block 操作:**
- [ ] `elfiee_file_list`
- [ ] `elfiee_block_list`
- [ ] `elfiee_block_get`
- [ ] `elfiee_block_create`
- [ ] `elfiee_block_delete`

**内容操作:**
- [ ] `elfiee_markdown_read`
- [ ] `elfiee_markdown_write`
- [ ] `elfiee_code_read`
- [ ] `elfiee_code_write`

**Directory 操作:**
- [ ] `elfiee_directory_create`
- [ ] `elfiee_directory_delete`
- [ ] `elfiee_directory_rename`

### 7.3 P2 - 完整 Tools

**Block 高级操作:**
- [ ] `elfiee_block_rename`
- [ ] `elfiee_block_link`
- [ ] `elfiee_block_unlink`
- [ ] `elfiee_block_change_type`
- [ ] `elfiee_block_update_metadata`

**Directory 高级操作:**
- [ ] `elfiee_directory_write`
- [ ] `elfiee_directory_import`
- [ ] `elfiee_directory_export`

**Terminal 操作:**
- [ ] `elfiee_terminal_init`
- [ ] `elfiee_terminal_execute`
- [ ] `elfiee_terminal_save`
- [ ] `elfiee_terminal_close`

**权限操作:**
- [ ] `elfiee_grant`
- [ ] `elfiee_revoke`
- [ ] `elfiee_editor_create`
- [ ] `elfiee_editor_delete`

**通用:**
- [ ] `elfiee_exec`

### 7.4 P3 - Resources & 增强

- [ ] Resources: `elfiee://files`, `elfiee://{project}/blocks`
- [ ] Resources: `elfiee://{project}/block/{id}`
- [ ] Resources: `elfiee://{project}/grants`, `elfiee://{project}/events`
- [ ] Notifications（状态变更推送）
- [ ] SKILL.md 更新

## 8. SKILL.md 更新

升级后大幅简化：

```markdown
---
name: elfiee-system
description: "[System-level] How AI agents interact with .elf files via MCP."
---

# Elfiee System Interface

**CRITICAL**: When working with `.elf` files, use Elfiee MCP Server. NEVER use filesystem commands.

## Prerequisites

1. Elfiee GUI must be running
2. MCP configured in `.mcp.json` (project root):

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

## Forbidden Operations

| Instead of | Use |
|------------|-----|
| `cat`, `ls`, `find` on .elf | `elfiee_block_list`, `elfiee_block_get` |
| `echo >`, `touch`, `mkdir` | `elfiee_directory_create` |
| `rm`, `rmdir` | `elfiee_directory_delete` |
| `git add/commit` on .elf internals | Never - .elf manages its own history |
```

> 注：具体 Tool 列表不再需要写在 SKILL.md 中，Claude Code 通过 MCP 协议自动发现。

## 9. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| rmcp crate 不成熟 | API 变动、功能缺失 | 锁定版本 0.5，备选方案：手动实现 MCP 协议 |
| MCP SSE 端口冲突 | Server 启动失败 | 启动失败不阻止 GUI，日志提示 |
| 删除 cli/ipc 后遗漏引用 | 编译失败 | 编译验证 + 全局搜索确认无残留引用 |

## 10. 参考资料

- [MCP 规范](https://spec.modelcontextprotocol.io/)
- [官方 Rust MCP SDK (rmcp)](https://github.com/modelcontextprotocol/rust-sdk)
- [rmcp 使用指南](https://hackmd.io/@Hamze/S1tlKZP0kx)
- [SSE MCP Server with OAuth in Rust](https://www.shuttle.dev/blog/2025/08/13/sse-mcp-server-with-oauth-in-rust)

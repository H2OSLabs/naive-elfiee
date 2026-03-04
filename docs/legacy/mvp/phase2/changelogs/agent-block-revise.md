# Agent 系统重设计方案

## 概述

Agent 系统从"绑定 Dir Block"模型迁移到"绑定 .claude/ 目录"模型。同时引入 per-agent 独立 MCP Server 端口、全局协作者机制、Task 专用 MCP 工具和 SKILL 模板更新。

**分支**: `feat/agent-block`
**基准**: `dev`
**日期**: 2026-02-03
**状态**: 方案设计（未实施）

---

## 一、核心设计原则

1. **一个 .claude/ = 一个 Agent** — Agent 绑定 `.claude/` 目录路径，不绑定 Dir Block
2. **保留 Symlink** — 避免污染原项目，symlink 目标指向 .elf/ block_dir（持久）
3. **Per-Agent MCP Server** — 每个 agent 运行独立 MCP 端口，身份 100% 对应正确
4. **不做向后兼容** — 直接替换旧数据模型
5. **全局协作者** — 通过 wildcard grant 实现，前端添加入口
6. **Task 工作流完整化** — 新增专用 MCP 工具，SKILL 模板指导自动链接

---

## 二、新数据模型

### AgentContents（替换旧版）

```rust
pub struct AgentContents {
    pub name: String,           // 显示名称 (default: "elfiee")
    pub claude_dir: String,     // 绝对路径: "/home/user/repo-a/.claude"
    pub status: AgentStatus,    // Enabled/Disabled
    pub editor_id: String,      // 必填 bot editor_id（不再是 Option）
}
```

**关键变化**：
- `target_project_id: String` → `claude_dir: String`：不再依赖 Dir Block
- `editor_id: Option<String>` → `editor_id: String`：必填

### AgentCreatePayload

```rust
pub struct AgentCreatePayload {
    pub name: Option<String>,
    pub claude_dir: String,         // 绝对路径到 .claude/
    pub editor_id: Option<String>,  // 不提供则自动创建 bot editor
}
```

### 路径推导

```
claude_dir = "/home/user/repo-a/.claude"

symlink src: {elf_block_dir}/agents/elfiee-client/    （.elf/ block 内部，不变）
symlink dst: {claude_dir}/skills/elfiee-client/
MCP config:  {claude_dir.parent()}/.mcp.json + {claude_dir}/mcp.json
```

---

## 三、Per-Agent 独立 MCP Server

### 问题

当多个 agent 同时 enabled 时，`resolve_agent_editor_id()` 取"第一个 enabled agent"会导致：
- Agent B 的操作被记在 Agent A 的 editor_id 上
- 无法 per-block 控制每个 agent 的不同权限
- Audit trail 错误

### 方案：每个 Agent 运行独立端口的 MCP Server

```
端口分配：
  47200 — 管理端口（保留，用于无 agent 的 legacy 模式）
  47201 — Agent A 的 MCP Server
  47202 — Agent B 的 MCP Server
  ...

repo-a/.mcp.json → { "elfiee": { "url": "http://localhost:47201/sse" } }
repo-b/.mcp.json → { "elfiee": { "url": "http://localhost:47202/sse" } }
```

### ElfieeMcpServer 改造

```rust
pub struct ElfieeMcpServer {
    app_state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
    agent_block_id: Option<String>,  // NEW: 绑定具体 agent
}

// resolve_agent_editor_id 确定性返回
async fn resolve_agent_editor_id(&self, file_id: &str) -> Result<String, McpError> {
    if let Some(agent_id) = &self.agent_block_id {
        let handle = self.get_engine(file_id)?;
        let block = handle.get_block(agent_id.clone()).await
            .ok_or_else(|| mcp::block_not_found(agent_id))?;
        let contents: AgentContents = serde_json::from_value(block.contents.clone())
            .map_err(|e| mcp::invalid_payload(format!("Invalid agent: {}", e)))?;
        return Ok(contents.editor_id);  // 确定性返回
    }
    self.get_editor_id(file_id) // 管理端口 fallback
}
```

### AppState 新增

```rust
pub struct AgentServerHandle {
    pub port: u16,
    pub agent_block_id: String,
    pub cancel_token: CancellationToken,
    pub sse_count: Arc<AtomicUsize>,
}

// AppState 新增字段
pub agent_servers: DashMap<String, AgentServerHandle>,  // agent_block_id → handle
pub next_agent_port: AtomicU16,                         // 初始值 47201
```

### Auto-Disable 精细化

```
旧行为: 所有 SSE 断开 → disable_all_agents()
新行为: Agent A 的 SSE 断开 → 只 disable Agent A
        Agent B 不受影响
```

---

## 四、端口管理方案

### 端口分配策略

```rust
fn allocate_agent_port(app_state: &AppState) -> Result<u16, String> {
    // 尝试从 next_agent_port 开始分配
    loop {
        let port = app_state.next_agent_port.fetch_add(1, Ordering::SeqCst);
        if port > 47299 {
            // 端口范围耗尽，回收已释放的端口
            return try_reclaim_port(app_state);
        }
        // 检查端口未被占用
        if !app_state.agent_servers.iter().any(|e| e.value().port == port) {
            return Ok(port);
        }
    }
}
```

**端口范围**：47201–47299（最多 99 个并发 agent，实际使用远小于此）

**端口回收**：agent disable 时释放端口，记入可回收池。新 agent 优先从回收池分配。

### 端口冲突处理

```rust
async fn start_agent_mcp_server(
    app_state: Arc<AppState>,
    agent_block_id: String,
    port: u16,
) -> Result<AgentServerHandle, String> {
    // 使用 rmcp 的 serve_with_config（内部创建 TcpListener + axum::serve + graceful shutdown）
    let ct = CancellationToken::new();
    let config = SseServerConfig {
        bind: SocketAddr::from(([127, 0, 0, 1], port)),
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: ct.clone(),
        sse_keep_alive: Some(Duration::from_secs(30)),
    };

    // serve_with_config 内部: TcpListener::bind + axum::serve + with_graceful_shutdown
    // 如果端口被占用，返回 io::Error
    let mut sse_server = SseServer::serve_with_config(config)
        .await
        .map_err(|e| format!("Failed to bind agent MCP on port {}: {}", port, e))?;

    // 启动 transport 处理循环
    let sse_count = Arc::new(AtomicUsize::new(0));
    let sse_count_clone = sse_count.clone();
    let ct_clone = ct.clone();
    let app_state_clone = app_state.clone();
    let agent_id_clone = agent_block_id.clone();

    tokio::spawn(async move {
        while let Some(transport) = sse_server.next_transport().await {
            let app_state = app_state_clone.clone();
            let agent_id = agent_id_clone.clone();
            let sse_count = sse_count_clone.clone();
            let child_ct = ct_clone.child_token();

            // Track connection
            let count = sse_count.fetch_add(1, Ordering::SeqCst) + 1;
            println!("MCP Agent {}: Client connected (active: {})", agent_id, count);

            tokio::spawn(async move {
                let service = ElfieeMcpServer::new(app_state.clone(), Some(agent_id.clone()));
                let result = async {
                    let server = service.serve_with_ct(transport, child_ct).await
                        .map_err(std::io::Error::other)?;
                    server.waiting().await?;
                    tokio::io::Result::Ok(())
                }.await;

                if let Err(e) = result {
                    eprintln!("MCP Agent {}: Connection error: {}", agent_id, e);
                }

                // Disconnect
                let remaining = sse_count.fetch_sub(1, Ordering::SeqCst) - 1;
                println!("MCP Agent {}: Client disconnected (active: {})", agent_id, remaining);

                if remaining == 0 {
                    // 只 disable 这一个 agent
                    disable_single_agent(&app_state, &agent_id).await;
                }
            });
        }
    });

    Ok(AgentServerHandle {
        port,
        agent_block_id,
        cancel_token: ct,
        sse_count,
    })
}
```

### 端口冲突 fallback

```rust
async fn start_agent_mcp_with_fallback(
    app_state: Arc<AppState>,
    agent_block_id: String,
) -> Result<AgentServerHandle, String> {
    // 尝试分配端口，失败则重试下一个
    for _ in 0..5 {
        let port = allocate_agent_port(&app_state)?;
        match start_agent_mcp_server(app_state.clone(), agent_block_id.clone(), port).await {
            Ok(handle) => return Ok(handle),
            Err(e) if e.contains("bind") || e.contains("address") => {
                // 端口被占用，继续尝试
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err("Failed to allocate port for agent MCP server after 5 attempts".to_string())
}
```

---

## 五、SseServer 干净关闭方案

### rmcp SseServer API 分析

rmcp 0.5.0 提供两种创建模式：

| 方法 | 行为 |
|------|------|
| `SseServer::new(config)` | 返回 `(SseServer, Router)`，手动 bind + axum::serve |
| `SseServer::serve_with_config(config)` | 内部创建 TcpListener + 启动 axum::serve + **自动 with_graceful_shutdown(ct.cancelled())** |

### 关键发现

`serve_with_config` 源码（rmcp 0.5.0 sse_server.rs）：

```rust
pub async fn serve_with_config(config: SseServerConfig) -> io::Result<Self> {
    let (sse_server, service) = Self::new(config);
    let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
    let ct = sse_server.config.ct.child_token();
    let server = axum::serve(listener, service).with_graceful_shutdown(async move {
        ct.cancelled().await;
        tracing::info!("sse server cancelled");
    });
    tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!(error = %e, "sse server shutdown with error");
        }
    });
    Ok(sse_server)
}
```

**要点**：
- 自动调用 `with_graceful_shutdown(ct.cancelled())`
- 使用 child_token，所以 cancel 根 token 即可关闭
- axum::serve 在 spawn 中运行，不阻塞调用方

### 关闭流程

```
Agent Disable 触发：
  1. app_state.agent_servers.get(agent_block_id)
  2. handle.cancel_token.cancel()
     ├─ child token (axum::serve) 收到取消 → 停止接受新连接
     ├─ child token (per-connection) 收到取消 → 中断活跃 MCP 会话
     └─ with_graceful_shutdown 等待现有请求完成
  3. 等待所有 SSE 连接关闭（handle.sse_count == 0 或超时）
  4. 从 agent_servers 移除
  5. 端口回收
```

### 超时保护

```rust
async fn stop_agent_mcp_server(
    app_state: &AppState,
    agent_block_id: &str,
) -> Result<(), String> {
    let handle = app_state.agent_servers.remove(agent_block_id)
        .ok_or_else(|| format!("Agent server not found: {}", agent_block_id))?;

    let (_, handle) = handle;

    // 1. 触发取消
    handle.cancel_token.cancel();

    // 2. 等待连接清理（最多 5 秒）
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while handle.sse_count.load(Ordering::SeqCst) > 0 {
        if tokio::time::Instant::now() > deadline {
            println!("MCP Agent {}: Force shutdown (connections remaining: {})",
                agent_block_id, handle.sse_count.load(Ordering::SeqCst));
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!("MCP Agent {}: Server stopped on port {}", agent_block_id, handle.port);
    Ok(())
}
```

### 当前 transport.rs 的改造

当前代码的问题：
- 使用 `SseServer::new()` + 手动 `axum::serve()` 但**没有** `with_graceful_shutdown`
- 从不调用 `token.cancel()`
- `start_mcp_server` 永远不返回（阻塞在 `axum::serve().await`）

改造方案：

```rust
// 管理端口 47200 也改用 serve_with_config
pub async fn start_management_mcp_server(
    app_state: Arc<AppState>,
    port: u16,
) -> Result<CancellationToken, String> {
    let ct = CancellationToken::new();
    let config = SseServerConfig {
        bind: SocketAddr::from(([127, 0, 0, 1], port)),
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: ct.clone(),
        sse_keep_alive: Some(Duration::from_secs(30)),
    };

    let mut sse_server = SseServer::serve_with_config(config)
        .await
        .map_err(|e| format!("MCP: Failed to bind on port {}: {}", port, e))?;

    println!("MCP Management Server listening on http://127.0.0.1:{}", port);

    // 管理端口仍用通用循环（agent_block_id = None）
    tokio::spawn(async move {
        while let Some(transport) = sse_server.next_transport().await {
            let app_state = app_state.clone();
            tokio::spawn(async move {
                let service = ElfieeMcpServer::new(app_state, None);
                // ... serve_with_ct + waiting
            });
        }
    });

    Ok(ct)  // 返回 token，Tauri 关闭时调用 ct.cancel()
}
```

### Tauri Lifecycle 集成

```rust
// lib.rs setup 中
let mcp_ct = start_management_mcp_server(app_state.clone(), 47200).await?;

// Tauri 关闭时
app.on_window_event(move |_window, event| {
    if let tauri::WindowEvent::Destroyed = event {
        // 关闭管理端口
        mcp_ct.cancel();
        // 关闭所有 agent 端口
        for entry in app_state.agent_servers.iter() {
            entry.value().cancel_token.cancel();
        }
    }
});
```

---

## 六、Elfiee 重启后端口回收

### 问题

Elfiee 重启后，之前的 agent_servers 记录丢失（内存态），但 .mcp.json 中仍记录着旧端口号。

### 方案

**Agent Enable 恢复流程**（Elfiee 启动时）：

```
Elfiee 启动
  → 遍历所有打开的 .elf 文件
  → 查找 status=Enabled 的 agent blocks
  → 对每个 enabled agent:
    1. 分配新端口（不沿用旧端口）
    2. 启动 agent MCP server
    3. 更新 .mcp.json 中的端口号（覆盖旧值）
    4. 更新 symlink（幂等）
```

这确保每次 Elfiee 启动时 agent 状态与实际 MCP server 一致。

### .mcp.json 的幂等更新

`mcp_config::merge_server` 已经是覆盖模式——如果 "elfiee" key 存在，直接替换新 URL。所以端口变更自然生效。

---

## 七、全局协作者

### 后端

Agent 创建时自动执行 wildcard grants：

```rust
const AGENT_DEFAULT_CAPS: &[&str] = &[
    "core.read", "core.create", "core.link", "core.unlink",
    "core.delete", "core.rename", "core.change_type", "core.update_metadata",
    "markdown.read", "markdown.write",
    "code.read", "code.write",
    "directory.read", "directory.write", "directory.create",
    "directory.delete", "directory.rename",
    "terminal.init", "terminal.execute", "terminal.save", "terminal.close",
    "task.read", "task.write", "task.commit",
];

// do_agent_create 成功后
for cap in AGENT_DEFAULT_CAPS {
    let grant_cmd = Command::new(
        editor_id.to_string(),
        "core.grant".to_string(),
        "*".to_string(),  // wildcard block_id
        json!({
            "target_editor": agent_editor_id,
            "capability": cap,
            "target_block": "*"
        }),
    );
    handle.process_command(grant_cmd).await?;
}
```

**已验证**：
- `core.grant` handler 不验证 block_id 存在性，接受 `"*"`
- `GrantPayload.target_block` 默认为 `"*"`
- `GrantsTable.has_grant()` 支持 `block_id == "*"` 通配匹配
- 前端 `grantCapability()` 已支持 `targetBlock='*'`

**Wildcard 是默认值，用户仍可 per-block 精细控制**：
- 全局 grant 后，可以对特定 block revoke 特定 agent 的权限
- 也可以不用全局 grant，只给特定 block 添加协作者

### 前端

#### Sidebar.tsx 改造

Editor 列表中添加全局协作者标识和入口：

```tsx
// 判断全局协作者
const isGlobal = grants.some(g =>
  g.editor_id === editor.editor_id && g.block_id === '*'
);

// Editor 列表项
<DropdownMenuItem>
  {editor.name}
  {isGlobal && <Badge variant="secondary">Global</Badge>}
</DropdownMenuItem>

// 新增按钮
<DropdownMenuSeparator />
<DropdownMenuItem onClick={() => setShowGlobalDialog(true)}>
  + Add Global Collaborator
</DropdownMenuItem>
```

#### GlobalCollaboratorDialog.tsx（新组件）

- 选择已有 editor 或创建新 editor（Human/Bot）
- 选择后调用 `addGlobalCollaborator(fileId, editorId)`
- Bot 类型可选自动创建 agent

#### app-store.ts 新增

```typescript
// Action
async addGlobalCollaborator(fileId: string, editorId: string) {
  const caps = [
    'core.read', 'markdown.read', 'markdown.write',
    'code.read', 'code.write', 'directory.read', 'directory.write',
    'task.read', 'task.write', 'task.commit',
    // ...
  ];
  for (const cap of caps) {
    await TauriClient.grantCapability(fileId, editorId, cap, '*');
  }
  await this.loadGrants(fileId);
}

// Selector
isGlobalCollaborator(fileId: string, editorId: string): boolean {
  const grants = this.files.get(fileId)?.grants || [];
  return grants.some(g => g.editor_id === editorId && g.block_id === '*');
}
```

#### CollaboratorItem.tsx

per-block 协作者列表中，全局协作者显示 "Global" badge，权限标记为全局（可在此级别 revoke）。

---

## 八、Task 专用 MCP 工具

### 当前状态

Task 功能已完整实现（task.write, task.read, task.commit capabilities + do_commit_task I/O），但无专用 MCP 工具。需要通过 `elfiee_block_create` + `elfiee_exec` 组合使用，体验不佳。

### 新增 4 个 MCP 工具

#### elfiee_task_create

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskCreateInput {
    pub project: String,
    pub name: String,
    pub description: Option<String>,
}

#[tool(description = "Create a new task block. Returns the task block ID for use in task_link and task_commit.")]
async fn elfiee_task_create(&self, Parameters(input): Parameters<TaskCreateInput>)
    -> Result<CallToolResult, McpError>
{
    // 1. core.create(type="task", name=name)
    // 2. if description → core.update_metadata
    // 3. 返回 task_block_id
}
```

#### elfiee_task_write

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskWriteInput {
    pub project: String,
    pub block_id: String,
    pub content: String,
}

#[tool(description = "Write markdown content to a task block.")]
async fn elfiee_task_write(&self, Parameters(input): Parameters<TaskWriteInput>)
    -> Result<CallToolResult, McpError>
{
    // execute_capability("task.write", block_id, { "content": content })
}
```

#### elfiee_task_commit

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskCommitInput {
    pub project: String,
    pub block_id: String,
}

#[tool(description = "Commit a task: export linked code blocks to git repositories, create branch, and commit. Returns commit hash, branch name, and exported files.")]
async fn elfiee_task_commit(&self, Parameters(input): Parameters<TaskCommitInput>)
    -> Result<CallToolResult, McpError>
{
    // 调用 do_commit_task（包含 capability 验证 + 导出 + git 操作）
    let file_id = self.get_file_id(&input.project)?;
    let editor_id = self.resolve_agent_editor_id(&file_id).await?;
    match do_commit_task(&self.app_state, &file_id, &editor_id, &input.block_id).await {
        Ok(result) => Ok(CallToolResult::success(vec![Content::text(
            json!({
                "ok": true,
                "commit_hash": result.commit_hash,
                "branch_name": result.branch_name,
                "exported_files": result.exported_files,
            }).to_string()
        )])),
        Err(e) => Ok(CallToolResult::error(vec![Content::text(
            json!({ "ok": false, "error": e }).to_string()
        )])),
    }
}
```

#### elfiee_task_link

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskLinkInput {
    pub project: String,
    pub task_id: String,
    pub block_id: String,
}

#[tool(description = "Link a code or markdown block to a task as its implementation. Uses 'implement' relation. Idempotent.")]
async fn elfiee_task_link(&self, Parameters(input): Parameters<TaskLinkInput>)
    -> Result<CallToolResult, McpError>
{
    // core.link(parent=task_id, child=block_id, relation="implement")
    self.execute_capability(
        &input.project, "core.link", Some(input.task_id),
        json!({ "child_id": input.block_id, "relation": "implement" }),
    ).await
}
```

---

## 九、SKILL 模板更新

### 新增内容

#### Block Types 更新

```markdown
## Block Types

`markdown` | `code` | `directory` | `terminal` | `task`
```

#### Task Operations 工具表

```markdown
### Task Operations

| Tool | Purpose | Key Params |
|------|---------|------------|
| `elfiee_task_create` | Create a new task | `project`, `name`, `description?` |
| `elfiee_task_write` | Write task content | `project`, `block_id`, `content` |
| `elfiee_task_commit` | Commit task to git | `project`, `block_id` |
| `elfiee_task_link` | Link task→implementation | `project`, `task_id`, `block_id` |
```

#### /new-task 工作流

```markdown
## /new-task Workflow

When the user says `/new-task` or asks you to create a task:

### Step 1: Create task
```
elfiee_task_create(project, name="Task name", description="What needs to be done")
→ Store returned task_block_id as ACTIVE_TASK
```

### Step 2: Work on implementation (Task Context)
For EVERY code/markdown block you create or modify while working on this task:
```
elfiee_code_write(project, block_id, content)       # or markdown_write
elfiee_task_link(project, task_id=ACTIVE_TASK, block_id=block_id)  # auto-link
```
The link is idempotent — calling it multiple times for the same pair is safe.

### Step 3: Commit
```
elfiee_task_commit(project, block_id=ACTIVE_TASK)
→ Exports implement-linked blocks to their git repos
→ Creates branch: feat/{task_name}
→ Git commit with task description
→ Returns: { commit_hash, branch_name, exported_files }
```

### Step 4: Test (optional)
```
elfiee_terminal_execute(project, terminal_block_id, command="cd /repo && cargo test")
→ If tests fail: fix code → commit again → test again
→ If tests pass: task complete
```
```

#### Capability IDs 更新

添加 `task.write`, `task.read`, `task.commit` 到列表。

---

## 十、Create 完整流程

```
agent.create(claude_dir="/home/user/repo-a/.claude")
  1. 验证 claude_dir 目录存在
  2. 唯一性检查：无已有 agent 绑定同一 claude_dir
  3. 若未提供 editor_id → 自动创建 bot editor (editor.create)
  4. agent.create capability handler → 创建 Agent Block (pure state)
  5. 自动执行 wildcard grants (AGENT_DEFAULT_CAPS × block_id="*")
  6. 分配端口 → 启动 per-agent MCP server
  7. perform_enable_io:
     a. symlink {elf_block_dir}/agents/elfiee-client/ → {claude_dir}/skills/elfiee-client/
     b. merge "elfiee" → {claude_dir.parent()}/.mcp.json  (URL = localhost:{agent_port}/sse)
     c. merge "elfiee" → {claude_dir}/mcp.json
```

---

## 十一、多 Repo 场景分析

**场景**：A 创建了 agent，A 和 B 都导入到 Elfiee，想在 B 中也用 agent。

**结论**：MCP 给 Claude Code 提供 Elfiee 内部 block 的完整访问（与本地文件系统无关），所以 block 操作不受 Claude Code 启动位置影响。但 Claude Code 需要 `.claude/skills/` 和 `.mcp.json` 才能发现 Elfiee MCP server。

**所以**：每个需要启动 Claude Code 的项目都需要自己的 agent：
- repo-a 有 agent-A → Claude Code 从 repo-a 启动，通过 MCP 操作所有 block
- repo-b 需要 agent-B → Claude Code 从 repo-b 启动，通过 MCP 操作所有 block
- 两个 agent 共享同一个 .elf + 同一套 blocks
- 两个 agent 有独立端口、独立 editor_id、独立 audit trail

---

## 十二、修改文件清单

### Phase A: Agent 数据模型

| 文件 | 修改 |
|------|------|
| `extensions/agent/mod.rs` | AgentContents: `target_project_id → claude_dir`, `editor_id: String` |
| `extensions/agent/agent_create.rs` | 使用 `claude_dir` |
| `extensions/agent/agent_enable.rs` | 使用 `claude_dir` |
| `extensions/agent/agent_disable.rs` | 使用 `claude_dir` |
| `extensions/agent/tests.rs` | 更新测试 |
| `state.rs` | 新增 `agent_servers`, `next_agent_port`, `AgentServerHandle` |

### Phase B: Per-Agent MCP Server

| 文件 | 修改 |
|------|------|
| `mcp/server.rs` | `ElfieeMcpServer::new(state, agent_block_id)`; `resolve_agent_editor_id` 确定性返回 |
| `mcp/transport.rs` | 改用 `serve_with_config`; 新增 `start_agent_mcp_server()`; per-agent SSE tracking; `stop_agent_mcp_server()`; 删除 `disable_all_agents()` |
| `utils/mcp_config.rs` | `build_elfiee_server_config(elf_file_path, port)` |

### Phase C: Agent Command 重写

| 文件 | 修改 |
|------|------|
| `commands/agent.rs` | 去除 Dir Block 依赖; auto bot editor; auto wildcard grants; 启停 agent MCP server; 删除 `get_external_path()` |

### Phase D: Task MCP 工具

| 文件 | 修改 |
|------|------|
| `mcp/server.rs` | 新增 `elfiee_task_create`, `elfiee_task_write`, `elfiee_task_commit`, `elfiee_task_link` |

### Phase E: SKILL 模板

| 文件 | 修改 |
|------|------|
| `templates/elf-meta/agents/elfiee-client/SKILL.md` | 新增 task block type; Task Operations; /new-task 工作流; Task Context 自动链接; Capability IDs 更新 |

### Phase F: 前端全局 Collaborator

| 文件 | 修改 |
|------|------|
| `Sidebar.tsx` | Global badge + "Add Global Collaborator" 按钮 |
| `GlobalCollaboratorDialog.tsx` (新) | 选择/创建 editor + wildcard grants |
| `CollaboratorItem.tsx` | Global badge 显示 |
| `app-store.ts` | `addGlobalCollaborator`, `isGlobalCollaborator` |
| `tauri-client.ts` | `agent_create` 参数更新 |
| `bindings.ts` | 自动重新生成 |

### Phase G: 启动恢复

| 文件 | 修改 |
|------|------|
| `lib.rs` (Tauri setup) | 启动时恢复 enabled agents 的 MCP server; Tauri 关闭时 cancel 所有 tokens |

### 不改

| 模块 | 原因 |
|------|------|
| GrantsTable / CBAC | 已支持 wildcard |
| core.grant handler | 已接受 `"*"` |
| Template 系统 | 刚统一完 |
| Engine / EventStore | 无需修改 |
| task capabilities | 已存在 (task.write/read/commit) |
| commands/task.rs | 已存在 (do_commit_task) |
| 右侧 per-block Collaborators tab | 仅加 Global badge |

---

## 十三、风险点

| 风险 | 缓解方案 |
|------|---------|
| 端口冲突 | fallback 重试机制 (最多 5 次) |
| rmcp CancellationToken 关闭不干净 | 5 秒超时 + force shutdown |
| Elfiee 重启后端口变化 | 启动时重新分配端口 + 覆盖 .mcp.json |
| bindings.ts 类型变化 | AgentContents 改变后需更新所有前端引用 |
| 多 agent 共享 elf_block_dir | symlink 源相同（.elf/ 内部），各自目标不同（各自 .claude/） |

---

## 十四、实现优先级

```
Phase A (数据模型)
  ↓
Phase B (Per-Agent MCP)  ←→  Phase D (Task MCP 工具)  ←→  Phase E (SKILL)
  ↓                           ↓
Phase C (Agent Command)      (可并行)
  ↓
Phase F (前端)
  ↓
Phase G (启动恢复 + Tauri lifecycle)
```

Phase D, E 与 A/B/C 无依赖，可并行开发。

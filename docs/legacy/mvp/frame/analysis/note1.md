# 讨论笔记 #1：架构分析与设计决策

> 基于 pi-mono 和 entireio/cli 两个项目的架构分析，结合 Elfiee 重构概念文档的逐篇审查，整理出的设计决策和借鉴方案。

---

## 一、概念文档审查进度

| # | 文档 | 层级 | 状态 | 关键发现 |
|---|------|------|------|---------|
| 1 | data-model.md | L0 | ✅ 已审查（前 session） | 4 种 block_type, EAVT 模型 |
| 2 | event-system.md | L1 | ✅ 已审查（前 session） | 4 种 mode (full/delta/ref/append) |
| 3 | cbac.md | L1 | ✅ 已审查 + 已实现 | 470 tests pass, certificator 唯一鉴权 |
| 4 | elf-format.md | L2 | ✅ 已审查（前 session） | ZIP → .elf/ 目录 |
| 5 | engine.md | L3 | ✅ 已审查（前 session） | 6 个差距已识别 |
| 6 | extension-system.md | L4 | ✅ 已审查 | 6 个差距（见下文§二） |
| 7 | communication.md | L4 | ✅ 已审查 | WebSocket 单端口统一，JSON-RPC 2.0，两层认证 |
| 8 | literate-programming.md | L5 | ✅ 已审查 | Block DAG + MyST directive 编织叙事 |
| 9 | agent-building.md | L5 | ✅ 已审查 | Agent/Task/Session 协作，工作模板，Skill 演化 |
| 10 | dogfooding-support.md | L6 | ✅ 已审查 | 5 项度量指标全从 Event 计算 |
| 11 | migration.md | L6 | ✅ 已审查 | 删 5600 行 + 加 3000 行，7 步迁移 |

---

## 二、Extension System (L4) 差距分析

### 现有实现 vs 概念目标

| Extension | 当前 Capability 数 | 概念目标 | 差距 |
|-----------|-------------------|---------|------|
| markdown | 2 (write, read) | 统一到 `document` | 需合并 |
| code | 2 (write, read) | 统一到 `document` | 需合并 |
| directory | 7 (import, export, write, create, delete, rename, rename_with_type_change) | **删除** | 全部移除 |
| terminal | 4 (init, execute, save, close) | 演进为 `session` | PTY 剥离 |
| task | 3 (write, read, commit) | 保留，去 Git 集成 | Git 操作委托 AgentContext |
| agent | 3 (create, enable, disable) | 保留，去 I/O | I/O 移到 AgentContext |

### 6 个关键差距

1. **markdown + code → Document 统一**（⭐⭐⭐）：document.write/read 替代 4 个 capability，contents schema 改为 `{format, content?, path?, hash?, size?, mime?}`
2. **Directory Extension 整体删除**（⭐⭐⭐）：7 个 capability 全部不符合纯函数要求
3. **Terminal → Session 演进**（⭐⭐⭐）：PTY 完全不属于 Elfiee 职责，只留 session.append
4. **Handler 纯度违反**（⭐⭐）：agent.enable/disable 有 I/O，terminal 有 PTY，directory 有 fs
5. **概念文档过时引用**（⭐）：Mermaid 图中仍列 core.rename/change_type（已合并为 core.write）、core.checkpoint（未实现）
6. **MyST Directive 未实现**（⭐）：前端工作，依赖 literate-programming.md

### 重构优先级

P0: 更新概念文档过时引用 → P1: 删除 Directory + Terminal → P2: 统一 Document + Agent I/O 外推 → P3: MyST Directive

---

## 三、Agent 动作记录设计

### 核心流程（data-model.md §2.6 定义）

```
1. Agent 在 AgentContext 中执行 I/O（写文件、跑命令）
2. Agent 向 Elfiee 发送 Command 记录决策事实
3. Elfiee Engine: Certificator → Handler(纯函数) → Event → persist
4. 返回确认
```

### 前三层的借鉴改进

**data-model（L0）— Session entry_type 扩展**

现有的 3 种 entry_type（command, message, decision）太粗。参考 pi-mono 的丰富消息类型：

```
建议: tool_call | tool_result | user_message | assistant_message |
      command_execution | decision | checkpoint | custom
```

参考 entireio/cli 的 NativeData 设计，Session entry 应保留 agent 原生格式：

```json
{
  "entry_type": "tool_call",
  "agent_type": "claude_code",
  "native": { /* Claude Code 原生格式，不做转换 */ },
  "normalized": { "tool_name": "edit", "path": "src/main.rs" }
}
```

不强制统一格式，但提供 `normalized` 字段做跨 agent 查询。

**event-system（L1）— Session compaction 机制**

Pi-mono 有完整的 compaction：LLM 总结旧消息，保留最近 ~20000 token。Elfiee 的 Session Block 会无限增长，需要类似机制：

- 新增 `session.compact` capability
- compact Event 的 mode = `full`（摘要是完整内容）
- value 包含 `firstKeptEntryId`，回溯时从此处开始

**cbac（L1）— Task 粒度的权限授予**

Agent 的权限应按 Task 范围授予，而非 wildcard：

```
Task 分配给 Agent 时：
  core.grant(agent_editor_id, "document.write", task_downstream_block_ids)
  core.grant(agent_editor_id, "session.append", session_block_id)

Task 完成后：
  core.revoke(agent_editor_id, ...)
```

比 wildcard grant 更安全，比 pi-mono（无权限系统）更精细。

---

## 四、.elf 自注册与初始化

### 类比

| 工具 | 初始化 | 创建什么 | 对接方式 |
|------|--------|---------|---------|
| Git | `git init` | `.git/` | 工具检测 `.git/` 存在 |
| VS Code | 打开文件夹 | `.vscode/` | workspace 配置 |
| Entire | `entire setup` | `.entire/` + hooks | settings.json + git hooks |
| **Elfiee** | `elf init` + `elf register` | `.elf/` + MCP config | MCP server 配置注入 |

### 两步流程

**Step 1: `elf init`（创建 .elf/ 目录）**

```
elf init
├── 创建 .elf/ 目录
├── 创建 eventstore.db（events + snapshots 表）
├── 写入 bootstrap events：
│   ├── editor.create（system editor）
│   └── core.grant × N（system editor wildcard grants）
├── 创建 config.toml：
│   ├── project_name = 目录名
│   ├── default_editor = system editor id
│   └── extensions = ["document", "task", "agent", "session"]
└── 创建 templates/（空目录）
```

**Step 2: `elf register <agent-type> <config-dir>`（注册 Agent）**

```
elf register claude_code /path/to/.claude
├── 创建 bot editor（Event: editor.create）
├── 授予基础权限（Events: core.grant × N）
├── 注入 MCP 配置到 agent 的 settings
├── 安装 skill/hooks（可选）
└── 更新 config.toml [[agents]] 段
```

### elf-format 与 communication 的职责划分

```
elf-format (L2):                     communication (L4):
├── .elf/ 目录结构定义                ├── MCP Server 实现
├── eventstore.db schema             ├── WebSocket Adapter
├── config.toml 格式                 ├── Tauri IPC Adapter
├── 初始化流程 (elf init)            ├── Message Router
├── Agent 注册流程 (elf register)     └── 连接级身份认证
├── 项目发现机制
└── config.toml [[agents]] 段
```

elf-format 解决"发现"（agent 怎么找到 .elf/），communication 解决"连接"（消息怎么传递）。

---

## 五、Elfiee 的三重定位

```
角色 1: 记录器（类 entireio/cli）
  Agent 做事 → Agent 告诉 Elfiee → Elfiee 记录 Event
  被动。MCP tools: session.append, document.write

角色 2: 工具提供者（类 pi-mono 的 tool 系统）
  Agent 通过 MCP 查询 Elfiee 的结构化数据
  主动辅助。MCP tools: block.query, task.list

角色 3: Agent 工厂（Elfiee 独有）
  定义 Agent Block + 工作模板 + Task 分配 + Skill 提炼
  编排者。MCP tools: agent.create, task.assign
```

| 维度 | 记录器 | 工具提供者 | Agent 工厂 |
|------|--------|-----------|-----------|
| 谁发起 | Agent → Elfiee | Agent → Elfiee | Elfiee → Agent |
| 数据流 | Agent 写入 | Agent 读取 | 双向 |
| 类比 | entireio/cli | pi-mono read/grep | 无直接类比 |

---

## 六、防绕过与防 Double-Token

### 6.1 防绕过

| 策略 | 怎么做 | 参考 |
|------|--------|------|
| MCP tool 比裸操作更好用 | 一次 tool call = Event 记录 + I/O 执行 | pi-mono: tool IS action |
| Git hook 兜底 | git commit 时扫描未记录的文件变更，生成 ref Event | entireio/cli |
| Skill/Prompt 引导 | system prompt 要求使用 Elfiee MCP tool | 软约束 |
| 不强求 100% 覆盖 | Elfiee 记录的是决策，不是文件 diff | 务实 |

**关键认知：Elfiee 记录的是决策事实，不是文件变更历史。文件 diff 可以从 Git 获取。**

### 6.2 防 Double-Token

**问题**：Agent 描述意图给 MCP tool + 自己做实际工作 = token 花两次。

**核心原则：MCP tool call = Event 记录 + I/O 执行，一次完成。**

```
❌ 错误（double-token）：
Agent → MCP: document.write("auth.rs", content)    // Token 1
Agent → Bash: echo content > src/auth.rs            // Token 2

✅ 正确（single-token）：
Agent → MCP: document.write("auth.rs", content)
  Elfiee 内部：Event persist + 委托写入 src/auth.rs   // 一次完成
```

MCP tool 返回值尽量小：`{ "ok": true, "event_id": "..." }`

### 6.3 不同 Agent 的 Token 成本差异

| Agent | 接入方式 | Double-Token |
|-------|---------|-------------|
| Claude Code | MCP tool call | 有（MCP call 是额外 LLM 调用） |
| Pi / OpenClaw | Operations 拦截 | **可消除**（透明拦截，LLM 不知情） |
| Claude Code + Hook | hook 被动记录 | 仅 systemMessage（极小） |

Pi 的 Pluggable Operations 是最干净的集成——完全透明，零额外 token：

```typescript
const elfieeWriteOps: WriteOperations = {
  writeFile: async (path, content) => {
    await fs.writeFile(path, content);         // 实际写
    await mcpClient.callTool("document.write", { path, content }); // 记录
  },
};
```

---

## 七、可借鉴的设计模式

### 7.1 纯函数状态机（来源：entireio/cli）

**原模式**：`Transition(Phase, Event, Context) → (NewPhase, []Action)`

**Elfiee 落地**：拆分 actor.rs 的 process_command()

```rust
// 纯函数：给定状态和命令，返回结果 + 副作用声明
fn process(state: &StateProjector, registry: &CapabilityRegistry, cmd: &Command)
    -> Result<CommandResult, CommandError>
{
    // 1. 鉴权（纯逻辑）
    // 2. 执行 handler（纯函数）
    // 3. 冲突检测（纯逻辑）
    // 4. 返回 events + 声明式副作用
    Ok(CommandResult {
        events,
        side_effects: vec![
            SideEffect::PersistEvents,
            SideEffect::ApplyToState,
            SideEffect::NotifyClients { elf_id },
            SideEffect::WriteSnapshot { block_ids },
        ],
    })
}

// 副作用执行器（薄层）
async fn execute_side_effects(&mut self, result: CommandResult) {
    for effect in result.side_effects {
        match effect {
            SideEffect::PersistEvents => { /* ... */ }
            SideEffect::ApplyToState => { /* ... */ }
            SideEffect::NotifyClients { elf_id } => { /* ... */ }
            SideEffect::WriteSnapshot { block_ids } => { /* ... */ }
        }
    }
}
```

**好处**：process() 100% 可单元测试，不需要 mock EventStore/channel。

### 7.2 Interface Segregation（来源：entireio/cli）

**原模式**：1 核心接口 + N 可选接口，调用者类型断言检测。

**Elfiee 落地**：在 CapabilityHandler trait 之外增加可选 trait：

```rust
// 可选：MyST 渲染（extension-system.md 要求）
trait RenderableCapability: CapabilityHandler {
    fn render_directive(&self, block: &Block, params: &DirectiveParams) -> String;
}

// 可选：contents schema 校验
trait ValidatableCapability: CapabilityHandler {
    fn validate_contents(&self, contents: &Value) -> Result<(), ValidationError>;
}

// 可选：Event 模式声明（StateProjector 用）
trait ModeAwareCapability: CapabilityHandler {
    fn event_mode(&self) -> EventMode; // Full | Delta | Append
}

// 可选：数据迁移
trait MigratableCapability: CapabilityHandler {
    fn migrate_event(&self, old_event: &Event, from_version: u32) -> Event;
}
```

Rust 实现：用 `downcast-rs` crate 或注册时分别存储可选能力。

### 7.3 自注册工厂（来源：entireio/cli + pi-mono）

**Phase 2 推荐方案**：Cargo features 编译时开关（简单可控）

```toml
[features]
default = ["ext-document", "ext-task", "ext-agent", "ext-session"]
ext-document = []
ext-task = []
```

```rust
fn register_extensions(&mut self) {
    #[cfg(feature = "ext-document")]
    self.register(Arc::new(DocumentWriteCapability));
    #[cfg(feature = "ext-task")]
    self.register(Arc::new(TaskWriteCapability));
}
```

**未来方案**：`inventory` crate 实现声明式自注册。

### 7.4 Context Struct（来源：entireio/cli）

**Elfiee 落地**：

```rust
struct HandlerContext<'a> {
    command: &'a Command,
    block: Option<&'a Block>,
    state: &'a StateProjector,
    grants: &'a GrantsTable,
    elf_path: &'a Path,
}

struct McpContext<'a> {
    editor_id: &'a str,
    elf_id: &'a str,
    engine: &'a EngineHandle,
}

struct InitContext {
    project_path: PathBuf,
    project_name: String,
    default_editor_name: String,
}

struct RegisterContext {
    elf_path: PathBuf,
    agent_type: AgentType,
    config_dir: PathBuf,
    engine: EngineHandle,
}
```

### 7.5 Pluggable Operations（来源：pi-mono）

**Elfiee 落地**：AgentContext 接口抽象

```rust
#[async_trait]
trait FileOps: Send + Sync {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    async fn write_file(&self, path: &Path, content: &[u8]) -> Result<()>;
    async fn exists(&self, path: &Path) -> bool;
}

#[async_trait]
trait BashOps: Send + Sync {
    async fn exec(&self, command: &str, cwd: &Path) -> Result<ExecResult>;
}

#[async_trait]
trait GitOps: Send + Sync {
    async fn status(&self, repo_path: &Path) -> Result<GitStatus>;
    async fn commit(&self, repo_path: &Path, message: &str) -> Result<String>;
}
```

MCP tool 通过 Operations 执行 I/O：
```rust
async fn handle_document_write(ctx: &McpContext, ops: &dyn FileOps, params: DocWriteParams) {
    let events = ctx.engine.process_command(cmd).await?;  // 记录
    ops.write_file(&path, content.as_bytes()).await?;       // 执行
}
```

---

## 八、MCP 生态对接

### 支持矩阵

| Agent | 接入方式 | 复杂度 | 优先级 |
|-------|---------|--------|--------|
| Claude Code | 原生 MCP config | 低 | P0 |
| Cursor / Windsurf | MCP config | 低 | P1 |
| Pi / OpenClaw | Extension + MCP Client 或 Operations 拦截 | 中 | P2 |
| AgentChannel | WebSocket | 中 | P3 |
| Aider | 自定义 adapter | 高 | P4 |

### Pi / OpenClaw 两种接入路径

**路径 A：Pi Extension API**

写一个 elfiee-extension for pi，注册额外 tool 和拦截 tool_result：

```typescript
pi.on("tool_result", async (event) => {
    if (event.toolName === "write" && !event.isError) {
        await mcpClient.callTool("document.write", {
            path: event.input.path,
            content: event.input.content,
        });
    }
});
```

**路径 B：Pluggable Operations 拦截（更深层）**

Pi 的每次文件操作通过自定义 Operations 自动记录到 Elfiee：

```typescript
const elfieeWriteOps: WriteOperations = {
    writeFile: async (path, content) => {
        await fs.writeFile(path, content);                           // 实际写
        await mcpClient.callTool("document.write", { path, content }); // 记录
    },
};
```

零 double-token 开销，LLM 完全不知情。

---

## 九、Communication 深入分析（L4）

### 架构要点

| 组件 | 职责 | 不做什么 |
|------|------|---------|
| 传输适配器 | 协议转换（IPC/WS → 统一格式）、连接生命周期 | 不做授权、不解析业务语义 |
| Message Router | 按 elf_id 路由到 Actor、连接注册表、连接级认证 | 不做 CBAC、不修改消息内容 |
| Engine Actor | 命令处理、CBAC、事件持久化 | 不关心消息来源 |

### 消息类型

| method | 方向 | 含义 |
|--------|------|------|
| `block.command` | Client → Engine | 执行 Command |
| `block.query` | Client → Engine | 查询 Block 状态 |
| `state.changed` | Engine → Client | 状态变更通知 |
| `command.rejected` | Engine → Client | 命令被拒绝 |
| `auth.login` | Client → Router | 连接级认证 |
| `auth.result` | Router → Client | 认证结果 |

### 两层认证分离

```
连接认证（Message Router）: "这个连接是谁" — 验证 editor_id 合法性
操作授权（Engine CBAC）:    "这个人能不能做" — 检查 Grant/Owner
```

Tauri IPC 不走 WebSocket 认证，直接从 `.elf/config.toml` 读取 default_editor。

### 关键问题：MCP 协议与 WebSocket 的关系

communication.md 定义了 WebSocket 取代 MCP SSE，但**未明确 MCP 协议语义是否保留**。这是关键设计缺口：

| 选项 | 方案 | 优点 | 缺点 |
|------|------|------|------|
| A | WebSocket 传输 + MCP 语义子集（tools/resources） | Claude Code 等 MCP 原生 agent 无缝切换 | 需要实现 MCP 协议的 tool/resource 子集 |
| B | WebSocket 传输 + 自定义 JSON-RPC | 完全自主控制 | 每个 Agent 需要写适配器 |
| C | 保留 MCP stdio/SSE + WebSocket 并存 | 零适配成本 | 两套协议维护 |

**建议：选项 A**。MCP 本身就是 JSON-RPC 2.0，消息格式天然兼容。在 WebSocket 上实现 MCP 的 `tools/list`、`tools/call`、`resources/list`、`resources/read` 四个方法即可。这样：
- Claude Code 只需改 transport（SSE → WebSocket），不改业务逻辑
- Cursor/Windsurf 等 MCP 原生客户端直接适配
- Elfiee 自定义消息（state.changed, auth.login）作为 MCP notification 扩展

### 其他待补充点

1. **backpressure / 消息排队**：多 Agent 高并发时 state.changed 广播可能 chatty，需要 per-connection 过滤（只推送该连接关心的 elf_id 变更）
2. **重连机制**：Agent 断线后重连，如何恢复到最新状态？需要 replay from last_seen_event_id
3. **auth.login 凭据格式**：文档未定义。建议最简方案：editor_id + shared secret（从 config.toml 生成），不引入 OAuth/JWT 复杂度

---

## 十、Literate Programming 分析（L5）

### 核心设计

两层分离：代码/数据独立存储为 Block，叙事文档通过 MyST directive 按需编织。

```
数据层（Block DAG）:       叙事层（Document Block）:
├── task: 实现登录           ├── # 登录功能开发记录
├── document: auth.rs        ├── ```{task} task-uuid
├── document: auth_test.rs   ├── ```{document} doc-uuid :lines: 15-45
└── session: 执行记录        └── ```{session} sess-uuid :filter: decision
```

### Directive 体系

| Extension | Directive | 渲染效果 |
|-----------|----------|---------|
| Document | `{document} block-id` | 嵌入代码/文档内容 |
| Task | `{task} block-id` | 任务卡片（标题、状态、分配） |
| Session | `{session} block-id` | 执行记录（命令、对话、决策） |
| Agent | `{agent} block-id` | Agent 信息卡片 |

参数：`:lines:`（行范围）、`:filter:`（类型过滤）、`:collapse:`（折叠）、`:caption:`（标题）

### 渲染管线（前端）

```
叙事文档内容 → MyST Parser → directive 列表 → Block Resolver(查询 Engine) → 渲染器 → UI
```

实时更新：Engine 广播 `state.changed` → 检查引用的 block_id 是否变更 → 局部重渲染。

### 关键问题：Directive 引用的 CBAC 权限穿透

literate-programming.md 未讨论：**读者有叙事文档的访问权限，但没有被引用 block 的权限时怎么办？**

建议方案：
```
渲染时对每个 directive 引用的 block_id 做 CBAC 检查：
  ├── 有权限 → 正常渲染 block 内容
  ├── 无权限 → 显示占位符 "[无权访问: block-uuid]"
  └── block 不存在 → 显示警告 "[block 已删除]"
```

这符合 CBAC 的最小权限原则，且不泄露未授权内容。

### 自动叙事生成

Agent 遍历 Task DAG → 收集关联 Block → 按 timestamp 排序 → 生成 MyST Markdown → 人类 Review。

**产品价值**：这是 Dogfooding 报告的自动生成路径——Agent 开发完功能后，自动生成一篇结构化的开发记录。

---

## 十一、Agent Building 分析（L5）

### 三个核心 Block 的协作

```
Agent Block (定义: prompt/能力/provider)
    ↓ assigned_to
Task Block (枢纽: description/status/assigned_to)
    ↓ implement
├── Document Block (产出物: 代码/文档)
├── Document Block (产出物: 测试)
└── Session Block (过程: 命令/对话/决策)
```

| Block | 角色 | 生命周期 |
|-------|------|---------|
| Agent | "它是谁、能做什么" | 长期存在，可启用/禁用 |
| Task | "要做什么、谁来做" | 随工作创建，状态流转后归档 |
| Session | "怎么做的、说了什么" | 随 Task 创建，只追加不修改 |

### Task 状态机

`[*] → pending → in_progress → completed/failed`

每次状态变更 = Event。completed 触发 Checkpoint 快照。

### Session entry_type 对齐

agent-building.md 定义了 3 种：

| entry_type | 何时产生 |
|------------|---------|
| command | Agent 在 AgentContext 中执行命令 |
| message | Agent 与用户/其他 Agent 对话 |
| decision | Agent 做出关键决策 |

**与 note1.md §三的扩展提案对比**：§三建议 8 种 entry_type（tool_call, tool_result, user_message, assistant_message, command_execution, decision, checkpoint, custom）。

**对齐建议**：保留概念文档的 3 种作为**语义分类**，增加 `subtype` 字段做细分：

```json
{
  "entry_type": "command",
  "subtype": "tool_call",
  "native": { /* agent 原生格式 */ },
  "normalized": { "tool_name": "edit", "path": "src/main.rs" }
}
```

这样不破坏概念文档的简洁性，又保留了扩展能力。

### 工作模板系统

模板声明 4 件事：参与者、工作流、权限矩阵、演化策略。

**模板实例化 = Event 生成**：创建 Block + 创建 Editor + Grant 权限。纯 Event Sourcing。

### Skill 演化路径（Phase 3）

```
Session → 模式识别 → Skill → 模板更新 → 下次 Session...
```

三阶段：Local Skill (个人) → Shared Skill (社区) → Organized Skill (团队)。

**实施建议**：Phase 2 只实现 Session 记录 + 基本模板实例化。Skill 提炼（需要 ML/NLP）留到 Phase 3。

### Agent 与 AgentContext 的分工（再确认）

| Elfiee 负责 | AgentContext 负责 |
|-------------|-----------------|
| Agent 定义（prompt, capabilities, provider） | 文件操作（FileSystem） |
| 权限控制（CBAC Grant/Revoke） | 命令执行（Bash Session） |
| 工作分配（Task assigned_to） | Git 操作（commit/branch） |
| 执行记录（Session append） | 凭据管理（API Key 注入） |

---

## 十二、Dogfooding Support 分析（L6）

### 5 项度量指标的架构映射

| 指标 | 目标 | 数据来源 | 计算方式 |
|------|------|---------|---------|
| FPY | > 60% | Session `command` entry 的 exit_code | 首次 exit_code=0 的 Task / 全部 completed Task |
| 回溯时间 | < 30s | Block DAG + Snapshot | DAG 反向遍历跳数 |
| 修复闭环率 | > 90% | Session 修复记录 | 最终成功修复 / 全部测试失败 |
| Summary 采纳率 | > 80% | Document delta Event | 编辑距离 < 20% 原文长度 |
| Memo 频次 | > 3条/功能 | human Editor 创建的 Document Block | 每个 Task 关联的 human doc 数量 |

**核心设计：所有度量从 Event 链直接计算，不需要额外的埋点系统。**

### Missing Tools 覆盖

| 缺失工具 | 覆盖方式 |
|---------|---------|
| test_result / FPY | Session `command` entry 自动记录 exit_code |
| task.commit | Task completed + Checkpoint；Git 委托 AgentContext，hash 记入 Session |
| Task Type | Task Block 的 `template` 字段 |
| implement 细分 | 当前保持单一 implement 关系，预留扩展 |
| Clarification | Session `message` entry + 事后标注 |
| PM 验收 | human Editor 的 `task.write(status=completed)` |
| Archive | completed + Checkpoint 快照 |
| Summary | Agent 遍历 DAG 自动生成叙事文档 |

### 度量实施优先级

| 优先级 | 指标 | 依赖 |
|--------|------|------|
| P0 | FPY、Memo 频次 | Session Block + Event 统计（纯后端） |
| P1 | 回溯时间、Summary 采纳率 | DAG 可视化 + delta 比较（需前端） |
| P2 | 修复闭环率 | AgentContext Bash Session 集成 |

### 命名修正

文档 §2.3 称"Terminal 修复闭环率"，但 Terminal Extension 已计划删除。应改为"**修复闭环率**"（Repair Loop Rate），数据来源改为 Session Block 而非 Terminal Block。

---

## 十三、Migration 分析（L6）

### 删除/保留/新增总览

| 类别 | 文件数 | 代码行 | 详情 |
|------|--------|--------|------|
| 删除 | ~32 | ~5600 | directory(2000) + terminal(1500) + MCP SSE(800) + agent MCP(400) + task Git(600) + commands(300) |
| 改造 | ~10 | ~1500 | engine, models, retained extensions |
| 新增 | ~15 | ~3000 | Document/Session Extension, WebSocket, Message Router, Checkpoint, MyST, Template |
| **净减** | — | **~2600** | 代码量减少 ~25% |

### 7 步迁移顺序

```
Step 1: 数据模型改造 (L0) — block_type 收束、Event mode 字段
Step 2: Event 系统扩展 (L1) — mode 处理、snapshots 表
Step 3: .elf/ 格式迁移 (L2) — ZIP → 目录、config.toml
Step 4: Engine 适配 (L3) — StateProjector 适配、Manager 收束
Step 5: Extension 重组 (L4) — 删 directory/terminal，合并 document，新增 session
Step 6: 通讯层替换 (L4) — 删 MCP SSE，新增 WebSocket + Message Router
Step 7: 应用层实现 (L5) — MyST 渲染、模板系统
```

### ⚠️ 与 CBAC Changelog 的冲突

migration.md 基于 Phase 1 代码编写，**未反映 CBAC 重构已完成的工作**：

| migration.md 描述 | 实际状态 | 影响 |
|-------------------|---------|------|
| §3.2 `metadata.rs` "保持" | **已删除**（cbac.md Step 1） | 需更新 |
| §3.4 `core.rename` "保持" | **已合并到 core.write**（cbac.md Step 2） | 需更新 |
| §3.4 `core.change_type` "保持" | **已合并到 core.write**（cbac.md Step 2） | 需更新 |
| §3.4 `core.read` "保持" | **已删除**（cbac.md Step 2） | 需更新 |
| Step 1 数据模型改造 | **部分完成**（metadata → description） | 需标记 |

**建议**：每次 changelog 完成后同步更新 migration.md 的进度标记。

### 风险评估补充

文档列出的 5 项风险之外，补充：

| 风险 | 影响 | 缓解 |
|------|------|------|
| **概念文档与代码不同步** | 后续重构参照过时文档做错决策 | 每步 changelog 同步更新相关概念文档 |
| **前端 binding 重建** | Extension 重组后 bindings.ts 大量变更 | 前端重构与后端重构分开 PR |
| **Session Block 无限增长** | 长时间运行的 Task 产生巨量 entry | 实现 session.compact（note1.md §三） |

---

## 十四、跨文档一致性问题

审查全部 11 份概念文档后发现的一致性问题：

| # | 问题 | 涉及文档 | 建议 |
|---|------|---------|------|
| 1 | core.rename/change_type 已合并为 core.write，多处文档仍引用旧名称 | extension-system, migration, architecture-overview | 全局替换 |
| 2 | metadata.rs 已删除，migration.md 仍标记"保持" | migration | 更新为"已删除" |
| 3 | Session entry_type 定义不一致（agent-building 3 种 vs note1 建议 8 种） | agent-building, data-model | 统一为 3 分类 + subtype |
| 4 | "Terminal 修复闭环率" 命名与 Terminal 删除矛盾 | dogfooding-support | 改为"修复闭环率" |
| 5 | communication.md 未定义 MCP 协议兼容性 | communication | 补充 MCP-over-WebSocket 方案 |
| 6 | core.checkpoint 在多处提及但无独立定义 | event-system, engine, dogfooding | 补充 checkpoint capability 规格 |

---

## 十五、更新后的待讨论事项

1. ~~communication.md 深入分析~~ → ✅ 已完成（§九）
2. ~~概念文档 #8-11 审查~~ → ✅ 已完成（§十~§十三）
3. **MCP-over-WebSocket 方案细化**：选项 A（WebSocket 传输 + MCP 语义子集）的具体实现，需要 Rust MCP SDK 调研
4. **重构实施顺序**：CBAC changelog 已完成 Step 1 部分工作，下一步应做什么？建议 Step 3（.elf/ 格式迁移）或 Step 5（Extension 重组）
5. **Session compaction 触发策略**：按 entry 数量（>1000）？按 token 估算（>20000）？按时间间隔？
6. **Directive CBAC 穿透策略**：渲染时检查权限 + 占位符方案是否可接受？
7. **概念文档同步更新**：是否现在就修正 §十四 列出的 6 个一致性问题？
8. **Skill 提炼时间线**：确认 Phase 2 不做 Skill ML/NLP，只做 Session 记录 + 模板基础

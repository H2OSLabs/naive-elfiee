# Agent Block 第一轮修改建议

## 概述

基于 `feat/agent-block` 分支的退出审查（详见 `agent-block-exit-review.md`）和用户 10 点反馈，整理的修改建议。本文档**仅记录建议方案**，不包含实际代码修改。

**分支**: `feat/agent-block`
**基准**: `dev`
**日期**: 2026-02-02

---

## 一、理想工作流对齐（反馈 #1, #2）

### 原始设计流程（task-and-cost_v3.md）

```
create .elf → create task → import project → create agent → claude 通过 MCP 执行 → task.commit
```

### 当前实现覆盖范围

```
create .elf ✅ → create task ✅ → import project ✅ → create agent ✅ → claude MCP ✅ → task.commit ✅
                                                        ↑ 本分支核心
```

### 建议

- **不需要改当前代码**。Agent create → enable → symlink + MCP 注入链路正确
- 后续 Session 同步 (3.4) 完成后流程才能完整串起来
- 当务之急是清理 Phase 1 死代码、修复架构层面的设计偏差

---

## 二、enable/disable 调用链澄清（反馈 #3）

### 结论：无需修改逻辑

经代码追踪确认，enable/disable **确实调用了 mcp_config.rs**。完整调用链：

```
Frontend (app-store.ts)
  → TauriClient.agent.enable(fileId, agentBlockId)
    → Tauri Command: commands/agent.rs::agent_enable()
      → handle.process_command(agent.enable)          # Capability: 仅更新 status
      → perform_enable_io(external_path, elf_block_dir, elf_file_path)
        → create_symlink_dir(src, dst)                # 创建 symlink
        → mcp_config::merge_server(.mcp.json, ...)    # ✅ 写入项目根 .mcp.json
        → mcp_config::merge_server(.claude/mcp.json)  # ✅ 写入 .claude/mcp.json
```

### 建议：补充架构文档

在 `agent_enable.rs` 和 `agent_disable.rs` 模块头添加分层说明：

```rust
//! ## 架构说明
//!
//! Agent enable/disable 采用分层设计：
//! - **Capability Handler（本文件）**: 仅更新 Block.contents.status 字段，纯状态变更
//! - **Tauri Command（commands/agent.rs）**: 执行 I/O 操作（symlink、MCP 配置合并/移除）
//!
//! 这样保证 Capability 层可独立测试，I/O 副作用集中在 Command 层管理。
```

---

## 三、Symlink 导致 dir.import 循环引用（反馈 #4） — **已修复** ✅

### 问题

`agent.enable` 在 `{project}/.claude/skills/elfiee-client/` 创建 symlink 指向 `.elf/ block_dir/agents/elfiee-client/`。后续 `directory.import` 导入该项目时，`fs_scanner::scan_directory` 会跟随 symlink 进入 `.elf/` 内部文件系统，造成循环引用。

### 实际修复方案

采用 `WalkDir::filter_entry()` 阻止隐藏目录下降（含 `.claude/`），比 symlink 检测更简单有效。

**文件**: `src-tauri/src/utils/fs_scanner.rs`

```rust
// filter_entry prevents descent into filtered directories.
// Depth 0 is the root entry itself — always allow it through since the
// caller explicitly chose to scan that path (it may be a hidden dir).
for entry in walker.into_iter().filter_entry(move |e| {
    if e.depth() == 0 {
        return true;
    }
    let name = e.file_name().to_string_lossy();
    if ignore_hidden && name.starts_with('.') {
        return false;  // ← 阻止 .claude/ 等隐藏目录的下降
    }
    if ignore_patterns.iter().any(|p| name == *p) {
        return false;
    }
    true
}) {
```

关键点：`filter_entry()` 与 `filter()` 不同 — 前者**阻止 WalkDir 进入该目录**，后者仅跳过结果但仍会递归进入子目录。Depth 0 始终放行，因为调用方明确选择了扫描该路径。

### 验证

- `cargo test fs_scanner` — 5/5 通过（含 `test_ignore_hidden_with_hidden_root` 边界用例）
- `.claude/` 目录不再被递归扫描，symlink 不会被跟随

---

## 四、Claude 交互方式改进（反馈 #5） — **部分完成** ✅

### 问题

当前 Claude 只能通过底层 MCP tools 操作（`elfiee_block_read`, `elfiee_code_write` 等），缺少高层工作流抽象。Claude 需要多步调用才能完成一个 task.commit。

### 实际实现

#### 4.1 Task MCP Tools（Phase D）

MCP Server 新增 Task 专属工具，让 Claude 可直接操作 Task 工作流：

| MCP Tool | 功能 |
|----------|------|
| `elfiee_task_read` | 读取 task block 内容 |
| `elfiee_task_write` | 写入/更新 task 内容 |
| `elfiee_task_commit` | 调用 `do_commit_task()` 执行完整的 task commit（含 git hooks 注入） |

`elfiee_task_commit` 调用的是 `commands/task.rs::do_commit_task()`（Split Pattern 抽取的业务函数），确保 MCP 和 Tauri Command 走完全相同的 I/O 流程。

#### 4.2 SKILL.md 模板更新（Phase E）

**文件**: `src-tauri/templates/elf-meta/agents/elfiee-client/SKILL.md`

SKILL.md 已更新，包含 Task 工作流说明、MCP tool 列表和使用示例。

#### 4.3 未来考虑：高层 MCP tool

高层封装 tool（如 `execute_task_workflow`）暂未实现，当前通过 SKILL.md 引导 Claude 组合使用现有 MCP tools。

---

## 五、统一模板系统（反馈 #6） — **已完成** ✅

> 实现详见：第十二节（Hook 模板提取）+ 第十三节（Skills block 化 + 模板重组）

### 问题

Hooks 和 Skills 都是 "模板注入" 模式，但机制完全不同：

| 维度 | Skills | Hooks |
|------|--------|-------|
| 存储 | 物理文件（block_dir 内） | Event Store（code block） |
| 模板 | `template_copy.rs` + `include_str!()` | `git_hooks.rs` 常量 |
| 注入 | bootstrap Step 5 写文件 | bootstrap Step 2-4 创建 block + 写内容 |
| 使用 | symlink 到 `.claude/skills/` | `commit_task` 从 block 读取快照 |

### 建议方案

**推荐方案 A：Hooks 也改为 template 模式**

```
templates/
├── elfiee-client/           # Skills 模板（已有）
│   ├── SKILL.md
│   ├── mcp.json
│   └── references/
│       └── capabilities.md
└── git-hooks/               # Hooks 模板（新增）
    └── pre-commit
```

修改 `elf_meta.rs` bootstrap：

```rust
// 删除 Step 2-4（创建 code block 存 hook 内容）
// 改为：
// Step 2: 初始化 skills 模板
template_copy::init_elfiee_client(block_dir, "")?;

// Step 3: 初始化 hooks 模板
template_copy::init_git_hooks(block_dir)?;
// 写入 block_dir/git/hooks/pre-commit 物理文件
```

`commit_task` 中注入 hooks 时从 `block_dir/git/hooks/pre-commit` 读取物理文件（而非通过 event store 查 code block）。

**优点**：
- 简化 bootstrap（少 3 步 command，少一个 code block）
- Hooks 和 Skills 统一管理
- 模板更新只需修改 `templates/` 目录

**备选方案 B：统一接口 trait**

```rust
trait TemplateProvider {
    fn name(&self) -> &str;
    fn files(&self) -> Vec<(&str, &str)>;  // (relative_path, content)
    fn write_to(&self, target_dir: &Path) -> Result<(), String>;
}

// Skills 和 Hooks 各自实现
struct ElfieeClientTemplate;
struct GitHooksTemplate;

impl TemplateProvider for ElfieeClientTemplate { ... }
impl TemplateProvider for GitHooksTemplate { ... }
```

Bootstrap 简化为：

```rust
ElfieeClientTemplate.write_to(block_dir)?;
GitHooksTemplate.write_to(block_dir)?;
```

---

## 六、身份认证 & editor_id（反馈 #7） — **已修复** ✅

### 原始问题

1. `AgentContents.editor_id: Option<String>` — 可选，可能缺失
2. `resolve_agent_editor_id()` 遍历所有 block，取第一个 enabled + 有 editor_id 的 agent — 多 agent 时归因错误
3. 无真正认证机制

### 实际修复方案：Per-Agent MCP Server（Phase B）

**彻底消除 `resolve_agent_editor_id()` 的非确定性问题**：每个 enabled agent 获得独立的 MCP Server 实例，绑定专属 SSE 端口。

#### 架构变化

```
Before: 1 个全局 MCP Server (port 47200) → resolve_agent_editor_id() 猜测身份
After:  每个 agent 1 个 MCP Server (port 47201-47299) → agent_editor_id 直接注入
```

#### 实现要点

**文件**: `src-tauri/src/mcp/server.rs`

- `McpServer` 新增 `agent_editor_id: Option<String>` 字段
- 当 `agent_editor_id` 有值时，所有 MCP tool 调用直接使用该 editor_id，无需查找
- 每个 per-agent server 有独立的 `CancellationToken`，支持单独启停

**文件**: `src-tauri/src/commands/agent.rs`

- `do_agent_enable()`: 启动 per-agent MCP server，端口 47201+
- `do_agent_disable()`: 通过 CancellationToken 停止对应 server
- `recover_agent_servers()`: 文件打开时扫描 enabled agents，恢复 MCP servers
- `shutdown_agent_servers()`: 文件关闭时停止所有 agent MCP servers

**文件**: `src-tauri/src/mcp/transport.rs`

- `AgentServerHandle { block_id, cancel_token, port }` 存储每个 agent server 的句柄
- `agent_servers: HashMap<String, AgentServerHandle>` 管理所有 per-agent servers

#### `editor_id` 变为 Required

`AgentContents.editor_id` 从 `Option<String>` 改为 `String`（必填），在 `agent.create` 时即确定。

#### 中期 Token 认证

仍为未来计划，当前的 per-agent 端口隔离已基本解决身份归因问题。

---

## 七、Agent 重定义：".claude/ folder mount"（反馈 #8） — **已修复** ✅

### 核心洞察

Agent 的本质不是 "绑定到某个项目 block"，而是 "管理了某个外部项目的 `.claude/` 目录"。

### 实际实现（Phase A）

采纳了核心洞察，但采用比建议更简洁的方案：**`claude_dir` 直接替代 `target_project_id`**，而非在其基础上新增字段。

#### 新 AgentContents

```rust
pub struct AgentContents {
    pub name: String,
    pub claude_dir: String,        // 替代 target_project_id，直接存储 .claude/ 路径
    pub status: AgentStatus,
    pub editor_id: String,         // 从 Option<String> 改为 String（必填）
}
```

#### 关键设计决策

| 建议方案 | 实际采用 | 原因 |
|----------|----------|------|
| `target_project_id` 保留 + `claude_dir` 新增 | 仅 `claude_dir`，删除 `target_project_id` | Agent 不再绑定 Dir Block，直接绑定 `.claude/` 路径更直接 |
| `editor_id: Option<String>` | `editor_id: String` | Bot editor 在 create 时即确定，不应为空 |
| `session_dir` 字段 | 暂不实现 | Session 同步（3.4）是独立 Phase，不在本次范围 |
| 方案 B（运行时注入） | `claude_dir` 持久化到 event store | `claude_dir` 是用户选择的配置，不是计算缓存，应当持久化 |

#### 修改文件

| 文件 | 修改 |
|------|------|
| `extensions/agent/mod.rs` | `AgentContents` 字段变更 |
| `extensions/agent/agent_create.rs` | `target_project_id` → `claude_dir` |
| `commands/agent.rs` | `do_agent_create/enable/disable` 使用 `claude_dir` 构造路径 |
| `mcp/server.rs` | MCP tool 中移除 `target_project_id` 相关逻辑 |
| 前端 `bindings.ts` | `AgentContents.claude_dir`, `AgentCreatePayload.claude_dir` |
| 前端 `app-store.ts` | `createAgent(fileId, claudeDir, ...)` |

#### enable/disable I/O 路径变化

```
Before: agent.enable → 从 target_project_id 查 Dir Block → get_external_path → 构造 .claude/ 路径
After:  agent.enable → 直接从 claude_dir 获取 .claude/ 路径（无需中间查找）
```

---

## 八、项目级协作者模型（反馈 #9） — **已修复** ✅

### 问题

当前 AddCollaboratorDialog 在单个 block 上添加协作者。用户需要逐 block 手动授权。理想模型是 "在 .elf 项目级别添加协作者，自动获得所有 block 的 read+write"。

### 实际实现方案：Global Collaborator（通配符 Grants）（Phase F）

采用比建议更简洁的方案：**通配符 grant（`block_id = "*"`）实现全局协作者**，无需遍历所有 blocks 或在 `core.create` 中自动继承。

#### 核心机制

```
addGlobalCollaborator(editorId)
  → 对 24 个默认 capability 各发一个 core.grant(editor_id, cap_id, block_id="*")
  → CBAC 通配符匹配：has_grant(editor_id, cap_id, ANY_block_id) → true
```

24 个默认 capabilities 覆盖所有操作：
```
core: read, create, link, unlink, delete, rename, change_type, update_metadata
markdown: read, write
code: read, write
directory: read, write, create, delete, rename
terminal: init, execute, save, close
task: read, write, commit
```

#### 后端：Agent 创建时自动授予

**文件**: `src-tauri/src/commands/agent.rs`

`do_agent_create()` 在创建 Agent Block 后，自动为 bot editor 授予 24 个通配符 grants。无需额外的 Tauri Command。

#### 前端：Global Collaborator UI

**文件**: `src/components/dashboard/Sidebar.tsx`

- Editor 下拉列表中，Global 协作者显示蓝色 `Globe` + `Global` 徽章
- 新增 "Add Global Collaborator" 菜单项，打开 `GlobalCollaboratorDialog`

**文件**: `src/components/permission/GlobalCollaboratorDialog.tsx`（新建）

- "Select Existing" 标签页：从已有 editor 中选择，授予全局权限
- "Create New" 标签页：创建新 editor (Human/Bot) 并授予全局权限
- 调用 `app-store.ts::addGlobalCollaborator()` 完成通配符 grant

**文件**: `src/components/permission/CollaboratorItem.tsx`

- 新增 `isGlobal` prop，显示蓝色 "Global" 徽章

**文件**: `src/components/permission/CollaboratorList.tsx`

- 通过 `isGlobalCollaborator()` 检测并传递 `isGlobal` prop

**文件**: `src/lib/app-store.ts`

- `addGlobalCollaborator(fileId, editorId)`: 授予 24 个 `block_id="*"` grants
- `isGlobalCollaborator(fileId, editorId)`: 检查是否存在 `block_id="*"` grant

#### 与建议方案的差异

| 建议 | 实际 | 原因 |
|------|------|------|
| 新增 `add_project_collaborator` Tauri Command | 前端直接组合现有 `grantCapability` | 通配符 grant 是标准 CBAC 操作，无需新 command |
| `core.create` 自动继承权限 | 不需要 | 通配符 grant 自动覆盖所有现有和未来的 blocks |
| 遍历所有 blocks 逐一授权 | 不需要 | 通配符 `block_id="*"` 一次匹配所有 |

#### 验证

- 98 前端测试通过（含 Agent toggle、Global badge、Dialog 测试）
- 449 后端测试通过

---

## 九、Session 内容解析路径（反馈 #10）

### 前提

基于 Point 8 的 Agent 重定义，每个 Agent 知道自己的 `session_dir`（`~/.claude/projects/{path-hash}/`）。

### 实现路径（参照 task-and-cost_v3.md 3.4）

#### 9.1 Session 目录计算器（F10-01，2h）

**文件**: `src-tauri/src/sync/session_path.rs`（新建）

```rust
/// 根据 external_path 计算 Claude session 目录
/// /home/yaosh/projects/elfiee → ~/.claude/projects/-home-yaosh-projects-elfiee/
pub fn compute_session_dir(external_path: &str) -> PathBuf {
    let home = dirs::home_dir().expect("No home directory");
    let encoded = external_path.replace('/', "-");
    home.join(".claude").join("projects").join(encoded)
}
```

#### 9.2 JSONL 文件监听器（F11-01，4h）

**文件**: `src-tauri/src/sync/watcher.rs`（新建）

- 使用 `notify` crate 监听 session_dir 下所有 `.jsonl` 文件
- 支持多项目同时监听（每个 enabled agent 一个 watcher）
- `agent.enable` → 启动 watcher，`agent.disable` → 停止 watcher

#### 9.3 增量解析器（F12-01，4h）

**文件**: `src-tauri/src/sync/parser.rs`（新建）

- 解析 Claude Code JSONL 格式：
  - `type: "human"` — 用户消息
  - `type: "assistant"` — Claude 回复
  - `type: "tool_use"` — 工具调用（含文件路径，可追踪修改）
- 记录文件偏移量，只解析新增行

#### 9.4 Session Block 写入器（F13-01，4h）

**文件**: `src-tauri/src/sync/writer.rs`（新建）

- 每解析一行 JSONL，用 `code.write` 追加到 `.elf/Agents/session/{project-name}/` Code Block
- Block 不存在则先 `core.create`

---

## 十、Phase 1 死代码清理 — **已完成** ✅

### 已删除的文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `extensions/agent/agent_configure.rs` | **已删除** | 与 Phase 2 不兼容 |
| `extensions/agent/context/collector.rs` | **已删除** | Phase 1 残留，无调用方 |
| `extensions/agent/context/truncator.rs` | **已删除** | Phase 1 残留，无调用方 |
| `extensions/agent/context/mod.rs` | **已删除** | 空的模块入口 |
| `extensions/agent/llm/anthropic.rs` | **已删除** | Phase 1 残留，无调用方 |
| `extensions/agent/llm/parser.rs` | **已删除** | Phase 1 残留，无调用方 |
| `extensions/agent/llm/error.rs` | **已删除** | Phase 1 残留，无调用方 |
| `extensions/agent/llm/mod.rs` | **已删除** | 空的模块入口 |

### 已修改的文件

| 文件 | 修改 |
|------|------|
| `extensions/agent/mod.rs` | 删除 Phase 1 模块声明（`agent_configure`, `context`, `llm`）、re-export、全部 Phase 1 类型定义（`AgentConfig`, `ProposedCommand`, `ProposalStatus`, `Proposal`, `AgentCreatePayload`, `AgentConfigurePayload`, `AgentInvokePayload`, `AgentApprovePayload`）；更新模块文档为 Phase 2 only |
| `extensions/agent/tests.rs` | 删除 Phase 1 测试（`test_agent_config_serialization`, `test_proposed_command_*`, `test_proposal_*`） |
| `capabilities/registry.rs` | 删除 `AgentConfigureCapability` 注册 |
| `Cargo.toml` | 删除 `reqwest`（仅 `llm/anthropic.rs` 使用）和 `regex`（仅 Phase 1 使用） |

### 验证

- `cargo check` — 编译通过，无错误无警告
- `cargo test agent` — 41 tests passed, 0 failed
- `cargo test registry` — 17 tests passed, 0 failed

---

## 十一、Split Pattern 重构：抽取业务函数统一 Tauri/MCP I/O — **已完成** ✅

### 问题

项目使用 Split Pattern：Capability Handler 只做授权 + 审计事件（纯函数），I/O 操作在 Tauri Command 层执行。MCP Server 的 `execute_capability()` 仅调用 Capability Handler，导致所有 I/O 密集型操作（`directory.export`、`agent.enable/disable`、`task.commit`）通过 MCP 调用时**缺少实际 I/O**。

`transport.rs::disable_all_agents` 手动复制了 agent.disable 的完整逻辑（process_command + perform_disable_io），造成代码重复。

### 解决方案：Option D — 抽取业务函数

从 Tauri Command 中提取独立的 `do_*` 业务函数，接受 `&AppState` 参数。Tauri Command、MCP Server、transport.rs 均复用同一业务函数。

### 已修改的文件

| 文件 | 修改 |
|------|------|
| `commands/agent.rs` | 提取 `do_agent_create()`, `do_agent_enable()`, `do_agent_disable()` 为 `pub async fn`；Tauri Command 变为薄 wrapper，仅解析 editor_id 后委托 |
| `commands/checkout.rs` | 提取 `do_checkout_workspace()` 为 `pub async fn`；Tauri Command 变为薄 wrapper |
| `commands/task.rs` | 提取 `do_commit_task()` 为 `pub async fn`；将 `get_elf_hooks_dir` 改为接受 `&AppState`；Tauri Command 变为薄 wrapper |
| `mcp/server.rs` | `elfiee_directory_export` 改为调用 `do_checkout_workspace()`（而非仅 `execute_capability`），实际执行文件 I/O |
| `mcp/transport.rs` | `disable_all_agents()` 改为调用 `do_agent_disable()`，删除复制的 process_command + perform_disable_io 逻辑；移除 `use crate::commands::agent::{get_external_path, perform_disable_io}` 导入 |

### 业务函数签名

```rust
// commands/agent.rs
pub async fn do_agent_create(app_state: &AppState, file_id: &str, editor_id: &str, payload: AgentCreateV2Payload) -> Result<AgentCreateResult, String>
pub async fn do_agent_enable(app_state: &AppState, file_id: &str, editor_id: &str, agent_block_id: &str) -> Result<AgentEnableResult, String>
pub async fn do_agent_disable(app_state: &AppState, file_id: &str, editor_id: &str, agent_block_id: &str) -> Result<AgentDisableResult, String>

// commands/checkout.rs
pub async fn do_checkout_workspace(app_state: &AppState, file_id: &str, editor_id: &str, block_id: &str, payload: &DirectoryExportPayload) -> Result<(), String>

// commands/task.rs
pub async fn do_commit_task(app_state: &AppState, file_id: &str, editor_id: &str, task_block_id: &str) -> Result<TaskCommitResult, String>
```

### 设计要点

- **`editor_id` 由调用方提供**：Tauri Command 用 `state.get_active_editor()`（GUI 用户），MCP Server 用 `resolve_agent_editor_id()`（Agent bot），transport.rs 用 `get_active_editor()`（auto-disconnect）
- **零行为变更**：业务逻辑完全不变，仅从 Tauri Command 函数体移动到 `do_*` 函数
- **向后兼容**：Tauri Command 签名和前端 bindings 完全不变

### 验证

- `cargo check` — 编译通过
- `cargo test` — 388 passed, 1 failed（pre-existing `test_init_writes_skill_md`，与本次修改无关）
- 零新增测试失败

---

## 十二、统一模板系统：模板 → Block → 注入 数据流 — **已完成** ✅

### 背景

`git_hooks.rs` 中的 `PRE_COMMIT_HOOK_CONTENT` 为硬编码的 `pub const` 字符串。
`inject_git_hooks()` 直接写 const 到外部 repo，跳过了 event store，导致：

1. hook 内容的变更没有 event 审计记录
2. 无法在 Elfiee 中编辑 hook 并验证效果（dogfooding 断裂）

### 正确数据流

```
模板文件 (compile-time 默认值)
    ↓ include_str!()
PRE_COMMIT_HOOK_CONTENT const
    ↓ bootstrap
elf_meta: core.create(code block) → code.write(template content) → events 记录
    ↓ inject 时
read_hook_content(): 从 .elf/ block 读取 hook 内容 → inject_git_hooks() 写入外部 repo
```

Dogfooding 流程：
1. 在 Elfiee 中编辑 hook code block → events 记录
2. 下次 inject 读到修改后的 block 内容 → 验证效果
3. 确认无误后更新 `templates/git-hooks/pre-commit` 源模板
4. 重新编译 → 新文件的 bootstrap 使用新默认值

### 修改内容

#### 新增文件
- `templates/git-hooks/pre-commit` — hook 脚本模板（编译时默认值）

#### 修改文件

| 文件 | 修改 |
|------|------|
| `src/utils/git_hooks.rs` | `PRE_COMMIT_HOOK_CONTENT` 改用 `include_str!()`；`inject_git_hooks()` 新增 `hook_content: &str` 参数（不再读 const） |
| `src/commands/task.rs` | 新增 `read_hook_content()` helper：从 .elf/ block 读取 hook 内容（fallback 到 const）；`do_commit_task` 和 `inject_hooks_for_repo` 调用前读 block |
| `src/extensions/directory/elf_meta.rs` | 模块文档更新，说明 模板→Block→注入 数据流；bootstrap 保留 4 步（code block 创建 + code.write） |
| `tests/elf_meta_integration.rs` | 恢复完整的 hook block 测试（`test_hook_block_has_template_content`） |

#### elf_meta.rs Bootstrap 流程（保持不变）

```
Step 1: core.create(.elf/ Dir Block)
Step 2: core.create(pre-commit code block) → events
Step 3: code.write(hook content from include_str) → events
Step 4: directory.write(entries + hook file ref) → events
Step 5: template_copy::init_elfiee_client() → block_dir (TODO: 也应改为 block 模式)
```

#### inject_git_hooks() 数据来源变化

```
Before: inject_git_hooks(repo_path, elf_hooks_dir) → 直接写 PRE_COMMIT_HOOK_CONTENT const
After:  inject_git_hooks(repo_path, elf_hooks_dir, hook_content) → 写调用方传入的 block 内容
```

调用方通过 `read_hook_content()` 从 .elf/ block 读取，找不到时 fallback 到 const。

### TODO

- ~~**Skills 统一**：当前 Skills 仍通过 `template_copy` 直接写 block_dir~~ → **已完成**，见第十三节

### 验证

- `cargo check` — 编译通过
- `cargo test` — 388 passed, 1 failed（pre-existing `test_init_writes_skill_md`，与本次修改无关）
- 集成测试 6/6 通过（含 `test_hook_block_has_template_content`）
- Git hooks 测试 7/7 通过

---

## 十三、统一模板系统：Template → Block + Physical Files — **已完成** ✅

### 背景

第十二节完成了 Hook 模板的 event-sourced 化，但 Skills（SKILL.md, mcp.json, capabilities.md）仍通过 `template_copy.rs` 直接写物理文件到 `block_dir/Agents/`，完全绕过 event store。此外存在以下问题：

| 问题 | 详情 |
|------|------|
| Skills 无 block/events | `template_copy` 直接写文件，无 block 无审计 |
| `ELF_DIR_PATHS` 硬编码 | 目录骨架写死在代码中，难以扩展 |
| 大小写不一致 | entries 用 `"agents/"` (小写)，`template_copy` 和 `agent.rs` 用 `"Agents"` (大写) |
| 模板目录不镜像 | `templates/elfiee-client/` 不对应 `.elf/` 内部结构 |

### 解决方案：TemplateFile 注册表 + 统一 bootstrap

所有模板统一为同一数据流模型：

```
模板文件 (include_str!)
    │
    ├──→ 创建 Block + 写入内容 (event sourced, 审计)
    │
    └──→ 直接写入 _block_dir 物理文件 (供 symlink 使用)
```

不需要 assemble 函数。物理文件来源于模板（bootstrap 时写入），Block 用于 events 审计和 Elfiee 内编辑验证。

### 实现分 5 个 Phase

#### Phase 1: 重组模板目录

将模板目录重组为镜像 `.elf/` 结构：

```
templates/elf-meta/              # 新目录，镜像 .elf/ 结构
├── agents/
│   └── elfiee-client/
│       ├── SKILL.md
│       ├── mcp.json
│       └── references/
│           └── capabilities.md
└── git/
    └── hooks/
        └── pre-commit
```

删除旧目录 `templates/elfiee-client/` 和 `templates/git-hooks/`。

#### Phase 2: TemplateFile 注册表

替换 `ELF_DIR_PATHS` 硬编码为模板驱动的注册表：

```rust
pub struct TemplateFile {
    pub path: &'static str,         // .elf/ 内 entry 路径
    pub content: &'static str,      // include_str! 编译时嵌入
    pub block_type: &'static str,   // "markdown" 或 "code"
    pub name: &'static str,         // block 名称
    pub description: &'static str,  // block 描述
    pub write_cap: &'static str,    // "markdown.write" 或 "code.write"
}

pub const TEMPLATE_FILES: &[TemplateFile] = &[ /* 4 entries */ ];

const EXTRA_DIRS: &[&str] = &[
    "session/",
    "agents/elfiee-client/scripts/",
    "agents/elfiee-client/assets/",
];
```

目录路径从 `TEMPLATE_FILES` 文件路径自动推导（`derive_dir_paths()`），不再硬编码。

#### Phase 3: 重写 bootstrap_elf_meta

新的 bootstrap 流程（统一处理所有 4 个模板文件）：

```
Step 1: core.create — .elf/ Dir Block
Step 2: for each TemplateFile:
          core.create — 创建 block (markdown/code)
          {type}.write — 写入模板内容 → events 记录
Step 3: directory.write — entries = 自动推导目录 + 文件 block 引用
Step 4: 写入物理文件 — 从模板内容直接写入 _block_dir + 创建 EXTRA_DIRS
```

#### Phase 4: 大小写统一

- `agent.rs` `perform_enable_io`: `join("Agents")` → `join("agents")`
- `git_hooks.rs`: `include_str!` 路径更新到 `templates/elf-meta/git/hooks/pre-commit`

#### Phase 5: 删除 template_copy.rs

bootstrap Step 4 完全取代其功能。

### 修改文件

| 文件 | 操作 | 修改 |
|------|------|------|
| `templates/elf-meta/` | **新建** | 重组模板目录结构（镜像 .elf/） |
| `templates/elfiee-client/` | **删除** | 移动到 elf-meta/agents/elfiee-client/ |
| `templates/git-hooks/` | **删除** | 移动到 elf-meta/git/hooks/ |
| `src/extensions/directory/elf_meta.rs` | **重写** | TemplateFile 注册表 + derive_dir_paths() + 新 bootstrap |
| `src/utils/git_hooks.rs` | 修改 | include_str! 路径更新到新模板位置 |
| `src/commands/agent.rs` | 修改 | 大小写修正：`Agents` → `agents` |
| `src/utils/template_copy.rs` | **删除** | 整个删除 |
| `src/utils/mod.rs` | 修改 | 移除 `pub mod template_copy` |
| `tests/elf_meta_integration.rs` | **重写** | 使用新 API：`build_elf_entries_with_files`, `derive_dir_paths`, `TEMPLATE_FILES`；验证 4 个模板文件成为 block |
| `tests/template_integration.rs` | **重写** | 使用统一模板系统流程，路径使用小写 `agents/` |

### API 变更

| Before | After |
|--------|-------|
| `ELF_DIR_PATHS: &[&str]` | `derive_dir_paths() -> Vec<String>` |
| `build_elf_entries_with_hooks()` | `build_elf_entries_with_files()` |
| `template_copy::init_elfiee_client()` | 内联到 bootstrap Step 4 |

### 验证

- `cargo test` — **390 单元测试 + 56 集成测试** 全部通过
- `.elf/` entries 包含 4 个 file block + 自动推导的目录
- 物理文件写入正确的小写路径 `agents/elfiee-client/`
- `read_hook_content` 仍从 block 读取 hook 内容（未改动）
- `PRE_COMMIT_HOOK_CONTENT` 常量保留（git_hooks.rs 测试需要），路径更新

---

## 修改优先级总结

| 优先级 | 修改项 | 对应反馈 | 工作量 | 状态 |
|--------|--------|----------|--------|------|
| **P0** | 删除 Phase 1 死代码 | 审查发现 | 小 | **已完成** ✅ (§十) |
| **P0** | 从 registry 移除 agent.configure | 审查发现 | 极小 | **已完成** ✅ (§十) |
| **P0** | 删除 Phase 1 专用依赖 (reqwest, regex) | 审查发现 | 极小 | **已完成** ✅ (§十) |
| **P0** | 补充 enable/disable 架构文档 | #3 | 极小 | **已完成** ✅ (§二) |
| **P0** | Split Pattern 重构：抽取业务函数 | MCP I/O 缺失 | 中 | **已完成** ✅ (§十一) |
| **P1** | fs_scanner 隐藏目录过滤 | #4 | 中 | **已完成** ✅ (§三, filter_entry) |
| **P1** | 项目级协作者 UI | #9 | 大 | **已完成** ✅ (§八, Global Collaborator 通配符 grants) |
| **P1** | 解决 agent editor_id 非确定性匹配 | #7 | 中 | **已完成** ✅ (§六, Per-Agent MCP Server) |
| **P2** | Task MCP Tools + SKILL.md 工作流 | #5 | 小 | **已完成** ✅ (§四, Phase D+E) |
| **P2** | Agent 重定义为 claude_dir | #8 | 中 | **已完成** ✅ (§七, Phase A) |
| **P2** | 统一模板系统（Hook 模板提取） | #6 | 中 | **已完成** ✅ (§十二) |
| **P2** | 统一模板系统（Skills block 化 + 模板重组） | #6 | 中 | **已完成** ✅ (§十三) |
| **P2** | Agent 启停恢复（文件打开/关闭生命周期） | Phase G | 小 | **已完成** ✅ (recover/shutdown_agent_servers) |
| **P3** | MCP Token 认证 | #7 | 大 | 未开始（per-agent 端口隔离已基本解决） |
| **P3** | Session 同步模块 | #10 | 大 | 未开始 |

---

## 待确认问题

1. ~~**Phase 1 代码清理**：在本分支直接删除，还是单独开 cleanup PR？~~ → **已在本分支直接删除** ✅
2. ~~**项目级协作者 UI**（#9）：倾向 "文件级 UI 面板"（新入口）还是 "复用 .elf/ block 的 CollaboratorList"？~~ → **采用 Sidebar + GlobalCollaboratorDialog 方案**：在侧边栏 Editor 下拉菜单中添加 "Add Global Collaborator" 入口，通过通配符 grants 实现全局协作者 ✅
3. ~~**Agent 路径字段**（#8）：采用方案 B（运行时注入，不持久化到 event store）还是方案 A（持久化）？~~ → **采用持久化方案**：`claude_dir` 是用户选择的配置（非计算缓存），持久化到 event store。同时 `target_project_id` 被完全替代，不再保留 ✅
4. ~~**Symlink 处理**（#4）：选择 "记录 symlink 到 entries"（信息保留）还是 "完全跳过 symlink"（更简单）？~~ → **采用 `filter_entry()` 跳过隐藏目录方案**：阻止 WalkDir 进入 `.claude/` 等隐藏目录，比 symlink 检测更简洁有效 ✅

**所有待确认问题已解决。**

---

## 实现总结

### 七个 Phase 全部完成

| Phase | 内容 | 对应章节 |
|-------|------|----------|
| Phase A | 数据模型重构（`claude_dir` 替代 `target_project_id`） | §七 |
| Phase B | Per-Agent MCP Server（独立端口 + 确定性身份路由） | §六 |
| Phase C | Agent Command 重构（`do_*` 业务函数 + 24 默认 caps） | §十一 |
| Phase D | Task MCP Tools（`elfiee_task_read/write/commit`） | §四 |
| Phase E | SKILL.md 模板更新 | §四 |
| Phase F | 前端 Global Collaborator（通配符 grants + UI） | §八 |
| Phase G | Agent 启停恢复（file open/close 生命周期） | 优先级表 |

### 测试覆盖

- **后端**: 451 tests passed (`cargo test`)（含 Round 2 通配符 grant/revoke 测试）
- **前端**: 98 tests passed (`pnpm test`)
- **总计**: 549 tests, 0 failures

---

**最后更新**: 2026-02-03 (Phase A-G 全部完成；Round 2 修复见 `agent-block-revision-round2.md`)

# Agent Block 退出审查记录

## 概述

对 `feat/agent-block` 分支的完整 code review，对比 `dev` 分支，覆盖后端 Agent 扩展、MCP Server、模板系统、前端协作者 UI 等模块。审查包含架构合理性评估和用户反馈的 10 个关键问题。

**分支**: `feat/agent-block`
**基准**: `dev`
**审查日期**: 2026-02-02

---

## 一、代码质量问题

### 1.1 Phase 1 死代码残留 (~1,100 行)

**严重度**: HIGH

**问题**: `extensions/agent/` 中保留了 Phase 1 (LLM Direct) 的全部代码，与 Phase 2 (External AI Tool) 不兼容。

| 文件 | 行数 | 说明 |
|------|------|------|
| `agent/mod.rs` L51-156 | ~105 | `AgentConfig`, `ProposedCommand`, `ProposalStatus`, `Proposal`, 4 个 Payload 类型 |
| `agent/agent_configure.rs` | ~330 | 整个文件。期望 `AgentConfig` 格式的 contents，Phase 2 Block 用 `AgentContents`，调用必崩 |
| `agent/context/collector.rs` | ~345 | Phase 1 上下文收集器，无调用方 |
| `agent/context/truncator.rs` | ~191 | Phase 1 上下文截断器，无调用方 |
| `agent/llm/anthropic.rs` | ~232 | Anthropic API 直接调用，Phase 2 不使用 |
| `agent/llm/parser.rs` | ~279 | LLM 响应解析器，无调用方 |
| `agent/llm/error.rs` | ~58 | LLM 错误类型，无调用方 |

**影响**:
- Phase 1 类型仍有 `#[derive(Type)]`，但未在 `lib.rs` 注册 specta 导出（不一致）
- `agent_configure` 在 `registry.rs` 中已注册为能力，但对 Phase 2 Block 无法工作
- 增加编译时间和维护混淆

### 1.2 agent.configure 与 Phase 2 不兼容

**严重度**: CRITICAL

**文件**: `src-tauri/src/extensions/agent/agent_configure.rs`

`agent_configure` handler 尝试从 `block.contents` 反序列化为 `AgentConfig`：
```rust
let config: AgentConfig = serde_json::from_value(block.contents.clone())
    .map_err(|e| format!("Invalid AgentConfig in block: {}", e))?;
```

但 Phase 2 agent block 的 contents 是 `AgentContents { name, target_project_id, status, editor_id }`，反序列化必然失败。该能力已在 `registry.rs` 注册但实际不可用。

### 1.3 resolve_agent_editor_id 取第一个匹配

**严重度**: MEDIUM

**文件**: `src-tauri/src/mcp/server.rs:320-340`

多 agent 场景下，`resolve_agent_editor_id()` 遍历所有 block 返回第一个 `enabled + 有 editor_id` 的 agent。如果同一 `.elf` 文件关联了多个外部项目（各有自己的 agent），MCP 请求会被错误归因到第一个而非当前连接的 agent。

### 1.4 SSE 连接计数潜在竞态

**严重度**: LOW

**文件**: `src-tauri/src/mcp/transport.rs:65-98`

`fetch_sub(1, SeqCst) - 1` 计算 `remaining` 时，另一个连接可能同时进来导致 remaining 误判为 0。实际上因为 `disable_all_agents` 内部会检查 `status == Enabled`，不会造成数据损坏，但会产生不必要的扫描。

---

## 二、架构设计问题

### 2.1 Agent 与项目耦合方式偏离设计

**原始设计 (task-and-cost_v3.md 3.1)**:
- Agent Block 仅存 `{name, target_project_id, status}`
- `.elf/Agents/elfiee-client/` 是**静态共享资源**，所有 Agent 共用
- `agent.create` 自动调用 enable（symlink + MCP 注入）

**实际实现**:
- `AgentContents` 多了 `editor_id: Option<String>`（合理扩展）
- Agent Block 的 enable/disable I/O 在 Tauri Command 层而非 Capability 层（合理的架构分离，但文档未体现）
- symlink 指向 `.elf/ block_dir/Agents/elfiee-client/` 而非文档中的 `.elf/Agents/elfiee-client/`（实现正确，文档表述不准确）

### 2.2 协作者权限模型问题

**用户反馈点 #9**: 当前实现要求在每个 Block 上添加协作者，但实际需求是"在 .elf 项目级别添加协作者，默认 read+write"。

**当前流程**:
1. 在特定 block 的 CollaboratorList 点击 "Add Collaborator"
2. 授权 `core.read` + type-specific read + type-specific write（仅对该 block）
3. 如果是 Bot 且 block 是 directory，额外创建 Agent Block

**问题**:
- Bot 协作者只在 directory block 上才创建 Agent，不合理
- 新协作者需要逐 block 手动添加，无法一次授权所有已有 block
- 与 `.elf/` 的 wildcard write 设计冲突（dev 分支已移除 wildcard grant）

### 2.3 Symlink 导致 dir.import 问题

**用户反馈点 #4**: `agent.enable` 在 `{external_path}/.claude/skills/elfiee-client/` 创建 symlink。后续 `directory.import` 如果导入该项目，会跟随 symlink 导入 `.elf/` 内部文件，造成循环引用。

**文件**: `src-tauri/src/extensions/directory/directory_import.rs`

代码中**无任何 symlink 检测逻辑**。`fs_scanner::scan_directory` 也未检查 symlink。

### 2.4 Claude 交互方式过于单一

**用户反馈点 #5**: 当前唯一的交互方式是通过 `/elfiee` skill + MCP tools。但 Claude 需要更丰富的上下文感知：
- 当前 SKILL.md 是静态模板，不知道 task 上下文
- 缺少 `/task` 类的高层命令让 Claude 直接操作 task 工作流
- MCP tools 过于底层，Claude 需要多步调用才能完成一个 task.commit

### 2.5 Agent 定义的本质思考

**用户反馈点 #8**: Agent 的本质是 "a `.claude/` folder mount"，而非绑定到特定项目。

**当前**: `AgentContents.target_project_id` 将 Agent 绑定到一个 Dir Block（外部项目）。

**理想**: Agent 代表 "我在这个外部项目中部署了 Elfiee 的 `.claude/` 配置"。Agent 的核心是：
1. 管理 `.claude/skills/` 目录下的 skill 注入
2. 管理 `.claude/mcp.json` 中的 MCP 配置
3. 跟踪 `~/.claude/projects/{path-hash}/` 下的 session

这个抽象更贴近实际：Agent 不关心 "哪个项目"，它关心 "我管理了哪个 `.claude/` 目录"。

### 2.6 Session 追踪缺失

**用户反馈点 #10**: 基于 `.claude/` 路径可以计算出 session 目录 (`~/.claude/projects/{path-hash}/`)，但当前实现完全没有 session 监听/同步逻辑。task-and-cost_v3.md 3.4 节定义了完整的 Session 同步模块（F10-F13，14 人时），尚未实现。

---

## 三、实现细节问题

### 3.1 enable/disable I/O 架构正确但文档不足

**用户反馈点 #3 (澄清)**: 经代码追踪确认，enable/disable **确实调用了 mcp_config.rs**。调用链：

```
Frontend → Store → TauriClient → Tauri Command (commands/agent.rs)
    → perform_enable_io()
        → create_symlink_dir()           # symlink
        → mcp_config::merge_server()     # 写 .mcp.json
        → mcp_config::merge_server()     # 写 .claude/mcp.json
    → Capability Handler (agent_enable.rs)
        → 仅更新 block.contents.status   # 状态
```

**问题**: 这个分层（Capability 只管状态，Command 管 I/O）是合理的，但代码注释和文档都没有明确说明这一点。Capability handler 文件头有一行注释 "I/O is done by Tauri command layer" 但过于简略。

### 3.2 MCP 配置写两份

**文件**: `src-tauri/src/commands/agent.rs:145-158`

`perform_enable_io()` 同时写 `.mcp.json` 和 `.claude/mcp.json`。Claude Code 实际只从 `.mcp.json`（项目根目录）读取。`.claude/mcp.json` 是兼容性 fallback。这导致 disable 时也需要清理两个文件。

### 3.3 Hooks 模板系统

**用户反馈点 #6**: hooks（如 pre-commit）和 skills（如 SKILL.md）都是"模板注入"模式，但使用了完全不同的机制：
- Skills: `template_copy.rs` + `include_str!()` 编译时嵌入
- Hooks: `git_hooks.rs` + `PRE_COMMIT_HOOK_CONTENT` 常量 + 在 `elf_meta.rs` bootstrap 中创建 code block

两者可以统一为一个模板系统。

---

## 四、与原始设计的偏差

| 设计项 (task-and-cost_v3.md) | 预期 | 实际 | 评估 |
|-----|------|------|------|
| F1-01 AgentContents | `{name, target_project_id, status}` | 多了 `editor_id: Option<String>` | 合理扩展 |
| F1-02 agent.create | Capability 内自动 enable | Command 层做 I/O，Capability 只创建 block | 合理分层 |
| F3-01 agent.enable | Capability 内做 symlink + MCP | Command 层做 I/O，Capability 只更新 status | 合理分层 |
| F3-03 mcp_config.rs | merge/remove 两个方法 | 实现完整，+440 行，20+ 测试 | 超预期 |
| Phase 1 代码 | 应清理 | 保留 ~1,100 行死代码 | 需清理 |
| agent.configure | Phase 2 不需要 | 仍注册在 registry，与 Phase 2 不兼容 | 需移除 |
| Session 同步 (3.4) | F10-F13，14 人时 | 未实现 | 按计划后续做 |
| MCP Server 独立模式 (3.2 F5-01) | 独立 Engine 实例 | 共享 AppState（同进程） | 实现方式不同但可行 |

---

## 五、修改文件清单

### 后端 (Rust)

| 文件 | 变更类型 | 行数 |
|------|----------|------|
| `src-tauri/src/extensions/agent/mod.rs` | 新增 | ~265 (Phase 1+2 类型) |
| `src-tauri/src/extensions/agent/agent_create.rs` | 新增 | Phase 2 create handler |
| `src-tauri/src/extensions/agent/agent_enable.rs` | 新增 | Phase 2 enable handler |
| `src-tauri/src/extensions/agent/agent_disable.rs` | 新增 | Phase 2 disable handler |
| `src-tauri/src/extensions/agent/agent_configure.rs` | 新增 | Phase 1 残留，不兼容 |
| `src-tauri/src/extensions/agent/context/` | 新增 | Phase 1 残留，~536 行 |
| `src-tauri/src/extensions/agent/llm/` | 新增 | Phase 1 残留，~569 行 |
| `src-tauri/src/commands/agent.rs` | 新增 | ~487 行，I/O 层 |
| `src-tauri/src/mcp/server.rs` | 修改 | +29 行 (resolve_agent_editor_id) |
| `src-tauri/src/mcp/transport.rs` | 修改 | +125 行 (SSE tracking + auto-disable) |
| `src-tauri/src/utils/mcp_config.rs` | 新增 | ~440 行，MCP 配置合并器 |
| `src-tauri/src/utils/template_copy.rs` | 新增 | ~330 行，模板复制工具 |
| `src-tauri/src/extensions/directory/elf_meta.rs` | 修改 | +模板初始化 Step 5 |
| `src-tauri/src/engine/state.rs` | 修改 | +53 行 (agent event 处理) |
| `src-tauri/src/engine/actor.rs` | 修改 | +3 行 (agent.create 白名单) |
| `src-tauri/src/capabilities/registry.rs` | 修改 | +7 行 (4 个 agent capability) |
| `src-tauri/src/lib.rs` | 修改 | +17 行 (commands + specta types) |
| `src-tauri/src/state.rs` | 修改 | +6 行 (sse_connection_count) |

### 前端 (TypeScript/React)

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/lib/tauri-client.ts` | 修改 | 新增 AgentOperations 类 |
| `src/lib/app-store.ts` | 修改 | 4 个 agent actions |
| `src/components/permission/CollaboratorItem.tsx` | 修改 | Agent toggle Switch |
| `src/components/permission/CollaboratorList.tsx` | 修改 | findAgentBlockForEditor 匹配 |
| `src/components/permission/AddCollaboratorDialog.tsx` | 修改 | Bot + directory → createAgent |

---

**最后更新**: 2026-02-02

# Changelog: L5 CLI + Skill + Workflow — MVP 完整能力

> 日期：2026-03-03
> 分支：feat/refactor-plan
> 对应概念文档：`elf-format.md`（§4 CLI）、`agent-building.md`（§3 模板系统、§4 Skill 演化）
> 测试：287 pass（261 unit + 4 block-perm + 9 project + 12 relation + 1 doc-test）

## 概要

实现 Elfiee MVP 的用户侧入口：统一 CLI 工具（`elf`）、Agent Skill 注入系统、工作模板解析运行。此前 L0-L4 完成了引擎核心（Event Sourcing / CBAC / Block DAG / Extensions / MCP Communication），但缺少"如何使用"的入口。本次变更补全了从 `elf init` 到 `elf run` 的完整用户旅程。

---

## 设计决策

- **统一 CLI 二进制**：`elf-serve` 升级为 `elf`（clap subcommands），`serve` 成为子命令之一
- **Skill 编译内嵌**：`DEFAULT_SKILL` 通过 `include_str!()` 编译进二进制，保证无 `.elf/` 时仍可提供 Skill
- **Skill 三级 fallback**：`role.md` → `default.md` → `DEFAULT_SKILL`（编译内嵌）
- **MCP auth/open 返回 Skill**：Agent 连接时自动获得操作指南，无需额外步骤
- **模板格式选 TOML**：结构化、可解析，存储在 `.elf/templates/workflows/`
- **内置模板编译内嵌**：`code-review.toml` 通过 `include_str!()` 编译进二进制，作为 fallback
- **`elf register` 双向操作**：向内（eventstore.db 写 Editor + Grants）+ 向外（注入 MCP 配置 + Skill 到 Agent 配置目录）
- **`elf run` = CLI 便利脚本**：类 `docker-compose up`，不是 Elfiee 功能本身

---

## 新建文件

| 文件 | 行数 | 说明 |
|------|------|------|
| `src/bin/elf.rs` | 184 | 统一 CLI 入口（clap subcommands: init, register, serve, run, status） |
| `src/cli/mod.rs` | 8 | CLI 模块声明 |
| `src/cli/init.rs` | 42 | `elf init`：创建 .elf/ + seed bootstrap events |
| `src/cli/register.rs` | 304 | `elf register`：创建 Editor + Grants + 注入 MCP 配置 + Skill（含 3 测试） |
| `src/cli/run.rs` | 246 | `elf run`：TOML 模板解析 + 参与者注册 + Task/Session 创建 + serve（含 3 测试） |
| `src/cli/status.rs` | 51 | `elf status`：项目统计输出 |
| `templates/skills/default.md` | 103 | 默认 Agent Skill（连接协议 + 工具参考 + DAG 协议 + MCP 资源） |
| `templates/workflows/code-review.toml` | 40 | 内置 Code Review 模板（2 参与者：coder + reviewer） |
| `docs/mvp/frame/README.md` | 206 | MVP 完整使用指南（安装 → init → register → serve → run → status） |

---

## 修改文件

| 文件 | 变更 |
|------|------|
| `Cargo.toml` | `[[bin]] elf-serve` → `[[bin]] elf` |
| `src/lib.rs` | 添加 `pub mod cli;` |
| `src/elf_project/mod.rs` | 添加 `DEFAULT_SKILL` 常量、`skills_dir()`、`read_skill(role)`；`init()` 创建 `templates/skills/default.md`（新增 3 测试） |
| `src/mcp/server.rs` | `AuthInput` 新增 `project` + `role` 字段；`elfiee_auth` 返回 `skill`；`elfiee_open` 返回 `skill`；新增 3 个 MCP Resources（`editors`、`my-tasks`、`my-grants`） |

---

## 删除文件

| 文件 | 说明 |
|------|------|
| `src/bin/serve.rs` | 被 `src/bin/elf.rs` 的 `Serve` 子命令替代 |
| `templates/elf-meta/` 目录 | Phase 1 遗留（elf-meta block type 已在 L3 删除） |

---

## CLI 命令参考

### `elf init [project]`

```
1. 创建项目目录（如不存在）
2. ElfProject::init() → .elf/ 目录结构 + eventstore.db + config.toml + templates/skills/default.md
3. seed_bootstrap_events() → system editor + wildcard grants
```

### `elf register <agent-type> [--name N] [--config-dir D] [--project P] [--port PORT]`

```
1. 打开项目（ElfProject::open + Engine 启动）
2. 生成 editor_id："{agent_type}-{uuid8}"
3. 创建 Editor（engine command: editor.create）
4. 授予默认权限（11 个 cap × wildcard block_id）
5. 推断 Agent 配置目录（openclaw/claude → .claude/）
6. 注入 MCP 配置（settings.local.json + mcpServers.elfiee）
7. 注入 Skill（skills/elfiee/SKILL.md）
```

**默认 Agent 权限（非 Owner）：**
```
document.read, document.write, task.read, task.write, task.commit,
session.append, session.read, core.create, core.link, core.unlink, core.delete
```

**不授权（Owner 专属）：** `core.grant`, `core.revoke`, `editor.create`, `editor.delete`

### `elf serve [--port PORT] [--project P]`

与之前的 `elf-serve` 功能一致，现在是 `elf` 的子命令。

### `elf run <template> [--project P] [--port PORT]`

```
1. 确保项目已初始化（未初始化则自动 elf init）
2. 加载模板：.elf/templates/workflows/{name}.toml → 内置 fallback
3. 为每个参与者执行 register（带角色特定权限）
4. 创建 Task Block + Session Block
5. 启动 elf serve（同进程）
6. 输出各 Agent 启动指令
```

### `elf status [project]`

```
读取 eventstore.db，输出事件数、Editor 数、Block 数、Grant 数统计。
```

---

## Skill 系统

### 默认 Skill 内容（`templates/skills/default.md`）

覆盖当前 L4 工具面：

| 章节 | 内容 |
|------|------|
| Connection Protocol | auth → open → operate 三步连接 |
| Critical Rules | 禁止直接文件操作、Event Sourcing 原则、CBAC 权限 |
| Block Types | document / task / session |
| Tool Reference | 23 个 MCP tools（连接 4 + 区块 7 + 内容 7 + 权限 4 + 通用 1） |
| Causal Chain Protocol | `implement` 关系 DAG |
| MCP Resources | 8 个 URI（files, blocks, block/{id}, grants, events, editors, my-tasks, my-grants） |
| Best Practices | 检查优先、任务驱动、会话记录、权限尊重 |

### Skill 获取路径

| 路径 | 触发条件 |
|------|---------|
| `elfiee_auth` 响应 | Agent 认证时携带 `project` 参数 |
| `elfiee_open` 响应 | Agent 打开项目时 |
| `elf register` 注入 | 注册时写入 `.claude/skills/elfiee/SKILL.md` |

### Skill 查找链

```
ElfProject::read_skill(role) →
  1. .elf/templates/skills/{role}.md（角色特定）
  2. .elf/templates/skills/default.md（项目级默认）
  3. DEFAULT_SKILL（编译内嵌 fallback）
```

---

## MCP Resources 新增

| URI | 说明 | 数据来源 |
|-----|------|---------|
| `elfiee://{project}/editors` | 所有 Editor 列表 | `handle.get_all_editors()` |
| `elfiee://{project}/my-tasks` | 当前 Editor 的任务 | 过滤 `assigned_to` 或 `owner` 匹配 `connection_editor_id` |
| `elfiee://{project}/my-grants` | 当前 Editor 的权限 | 从 `get_all_grants()` 按 editor_id 查找 |

---

## 模板系统

### 模板格式（TOML）

```toml
[template]
name = "Code Review"
description = "标准代码审查工作流"

[[participants]]
role = "coder"
agent_type = "openclaw"
capabilities = ["document.write", "document.read", "session.append", ...]

[[participants]]
role = "reviewer"
agent_type = "openclaw"
capabilities = ["document.read", "session.append", "task.read"]

[task]
name = "Code Review Task"
description = "按模板执行代码审查"
```

### 模板加载优先级

1. `.elf/templates/workflows/{name}.toml`（项目级）
2. 内置模板（`BUILTIN_CODE_REVIEW`，编译内嵌）

---

## 与概念文档的一致性

| 概念文档条目 | 实现状态 | 备注 |
|---|---|---|
| `elf init` 创建 .elf/ + bootstrap events（elf-format.md §4.2） | ✅ | |
| `elf register` 双向操作（elf-format.md §4.3） | ✅ | 目前仅支持 openclaw/claude，cursor/windsurf 待扩展 |
| `elf serve` 单端口 SSE（elf-format.md §4.4） | ✅ | 从 L4 communication 继承 |
| `elf run` 便利脚本（elf-format.md §4.5） | ✅ | |
| `elf status` 项目状态（elf-format.md §4.1） | ✅ | |
| config.toml 有 `[editor]` section（elf-format.md §2.2） | ❌ | 实现中沿用 L2 的 Git 模式（无 [editor]），system_editor_id 从 GlobalConfig 读取 |
| 模板格式 TOML（agent-building.md §3） | ✅ | 概念文档写的是 "Markdown/TOML"，实现选 TOML |
| Skill 三级 fallback（agent-building.md §4） | ✅ | |
| MCP auth/open 返回 Skill | ✅ | |
| 8 个 MCP Resources | ✅ | L4 有 5 个，本次新增 3 个 |

---

## 验证

- `cargo check` — 零错误（lib + elf binary + elfiee-app）
- `cargo test` — 287 tests 全部通过（+23 新增）
- `cargo clippy` — 零警告
- 新增测试覆盖：ElfProject skills（3）、register MCP 注入（3）、run 模板解析（3）

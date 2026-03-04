# Elfiee MVP — 可测试用户旅程

Elfiee 是一个 **EventWeaver**（事件织机）：纯被动的事件溯源 + 能力控制 + 区块管理系统。

> 本文档定义了每个用户旅程的操作步骤和预期结果，作为手动测试基准。
> 每个 Journey 对应一个或多个端到端自动化测试（见文末[测试覆盖矩阵](#端到端测试覆盖)）。
>
> **当前测试：329 pass**（295 unit + 33 integration + 1 doc-test）

---

## 前置：构建 CLI

**操作**：
```bash
cd src-tauri && cargo build --release --bin elf
export PATH="$PWD/target/release:$PATH"
```

**预期**：`elf --version` 输出版本号（如 `elf 0.1.0`）。

---

## Journey 1: 创建 .elf 项目

**场景**：将已有代码仓库纳入 Elfiee 管理。

### 操作

```bash
cd /your-project
elf init
```

### 预期结果

**1.1 目录结构**：

```
.elf/
├── eventstore.db              # SQLite 事件日志
├── config.toml                # 项目配置
└── templates/
    └── skills/
        └── default.md         # 默认 Agent Skill
```

验证：`ls .elf/eventstore.db .elf/config.toml .elf/templates/skills/default.md` 三个文件均存在。

**1.2 config.toml 内容**：

```toml
[project]
name = "<项目目录名>"

[extensions]
enabled = ["document", "task", "session"]
```

验证：`cat .elf/config.toml`，`name` = 当前目录名。

**1.3 CLI 输出**：

```
Initialized .elf/ in /canonical/project/path
  eventstore.db: created
  config.toml: created
  templates/skills/default.md: created
  <N> files scanned, <M> blocks created
```

其中 N = 扫描文件数，M = 创建的 block 数（排除 `.gitignore` + 二进制文件）。

**1.4 Block 列表**：

```bash
elf block list
```

输出格式：
```
NAME                                               TYPE       ID                                   OWNER
──────────────────────────────────────────────────────────────────────────────────────────────────────────
src/main.rs                                        document   <uuid>                               system
README.md                                          document   <uuid>                               system
...
Total: <M> blocks
```

验证：每个非忽略/非二进制文件对应一个 `document` block，`OWNER` = `system`。

**1.5 项目状态**：

```bash
elf status
```

输出格式：
```
Elfiee Project: /canonical/project/path
  Config: <project_name>

  Events:  <count>
  Editors: 1
  Blocks:  <M>
  Grants:  <count>
```

验证：`Editors: 1`（只有 system），`Blocks` = block list 的 Total 数。

---

## Journey 2: 注册单个 Agent

**场景**：让一个 AI Agent 能连接和操作 Elfiee 项目。

### 操作

```bash
# 基本注册（配置注入到项目目录/.claude）
elf register openclaw --name "coder"

# 跨路径注册
elf register openclaw --name "coder" \
    --config-dir /path/to/agent/.claude \
    --project /path/to/project
```

### 预期结果

**2.1 CLI 输出**：

```
Registered openclaw as editor 'openclaw-<8字符>'
  MCP config -> /path/to/config/dir
  Skill -> /path/to/config/dir/skills/elfiee/
```

验证：`editor_id` 格式为 `openclaw-<8字符hex>`。

**2.2 eventstore.db 变更**（通过 `elf status` 验证）：

```bash
elf status
```

验证：`Editors: 2`（system + 新注册的 agent）。

**2.3 注入的 MCP 配置**：

文件 1：`{project_root}/.mcp.json`（MCP server 连接）
```json
{
  "mcpServers": {
    "elfiee": {
      "type": "sse",
      "url": "http://localhost:47200/sse"
    }
  }
}
```

文件 2：`{config_dir}/settings.local.json`（env + permissions）
```json
{
  "env": {
    "ELFIEE_EDITOR_ID": "openclaw-<8字符>",
    "ELFIEE_PROJECT": "/canonical/project/path"
  },
  "permissions": {
    "allow": [
      "mcp__elfiee__elfiee_auth",
      "mcp__elfiee__elfiee_exec",
      "..."
    ]
  }
}
```

验证：`.mcp.json` 包含 SSE URL；`settings.local.json` 包含 editor_id + project path + 18 个 MCP 工具权限。

**2.4 注入的 Skill**：

文件：`{config_dir}/skills/elfiee/SKILL.md`

验证：文件存在且内容非空。

**2.5 权限验证**：

通过 MCP 工具或事件列表验证注册时授予的默认权限（wildcard）：

| 权限 | 授予？ |
|------|--------|
| `document.read`, `document.write` | 是 |
| `task.read`, `task.write`, `task.commit` | 是 |
| `session.read`, `session.append` | 是 |
| `core.create`, `core.link`, `core.unlink`, `core.delete` | 是 |
| `core.grant`, `core.revoke`, `editor.create`, `editor.delete` | **否**（Owner 专属） |

---

## Journey 3: 手动调整权限

**场景**：授予/撤回 Agent 的特定权限。

### 操作

```bash
# 授予 wildcard 权限
elf grant <editor_id> document.write

# 授予对特定 block 的权限（支持 name 或 id）
elf grant <editor_id> document.write src/main.rs

# 撤回权限
elf revoke <editor_id> document.write
elf revoke <editor_id> document.write src/main.rs
```

### 预期结果

**3.1 Grant 输出**：

```
Granted document.write to editor '<editor_id>' on block '<block>'
```

其中 `<block>` = `*`（wildcard）或 block name/id。

**3.2 Revoke 输出**：

```
Revoked document.write from editor '<editor_id>' on block '<block>'
```

**3.3 事件验证**：

```bash
elf event list
```

验证：出现 `system/core.grant` 和 `system/core.revoke` 事件。

---

## Journey 4: 启动 MCP Server + Agent 连接

**场景**：启动 headless MCP Server，Agent 通过 SSE 连接后开始操作。

### Step 1: 启动 Server

**操作**：

```bash
elf serve --port 47200 --project /your-project
```

**预期输出**：

```
elf v<version>
Elfiee MCP server starting...
Opened project: /your-project (file_id: file-<uuid>)

Ready. Clients can connect via MCP SSE at http://127.0.0.1:47200
  1. Call elfiee_auth to authenticate
  2. Call elfiee_open to open a project
  3. Use block/document/task tools to operate

Press Ctrl+C to stop.
```

验证：进程保持运行，端口 47200 可访问。

### Step 2: Agent 连接三步曲

Agent 通过 MCP SSE 连接后，按以下顺序调用：

| 步骤 | MCP Tool | 参数 | 预期返回 |
|------|----------|------|----------|
| 1 | `elfiee_auth` | `editor_id` | 认证成功，绑定连接身份 |
| 2 | `elfiee_open` | `project_path` | block 数量 + Skill 内容 |
| 3 | 开始操作 | 任意 MCP tool | 正常执行（受 CBAC 限制） |

### Step 3: Agent 典型操作

以下 MCP 工具调用顺序展示完整工作流（18 个 tool，extension 操作统一走 `elfiee_exec`）：

| 操作 | MCP Tool | CBAC |
|------|----------|------|
| 读取 block 内容 | `elfiee_block_get(project, block_id)` | `{type}.read` |
| 创建新 block | `elfiee_block_create(project, name, type)` | `core.create` |
| 写入文档 | `elfiee_exec(capability="document.write", block_id, payload)` | `document.write` |
| 建立关系 | `elfiee_block_link(project, parent, child, relation)` | `core.link` |
| 创建任务 | `elfiee_block_create(project, name, type="task")` | `core.create` |
| 更新任务 | `elfiee_exec(capability="task.write", block_id, payload)` | `task.write` |
| 记录会话 | `elfiee_exec(capability="session.append", block_id, payload)` | `session.append` |
| 提交任务 | `elfiee_exec(capability="task.commit", block_id, payload)` | `task.commit` |

**CBAC 规则**：
- Owner（创建者）始终有权限
- 非 Owner 需要 Grant（wildcard `*` 或精确 block 匹配）
- 无权限时返回错误

---

## Journey 5: 查看 Event 历史

**场景**：查看谁在什么时候对哪个 block 做了什么。

### 5a: 列出所有事件

**操作**：

```bash
elf event list
```

**预期输出**：

```
BLOCK                    CAPABILITY           EDITOR       CREATED_AT           EVENT_ID
──────────────────────────────────────────────────────────────────────────────────────────
my-document              core.create          system       2025-01-01 12:00:00  a1b2c3d4
my-task                  task.write           alice        2025-01-01 12:01:00  e5f6g7h8
...

Total: <N> events
```

验证：
- BLOCK 列显示 block name（非 UUID），找不到 name 时显示短 ID（前 8 位）
- CAPABILITY 和 EDITOR 从 attribute `{editor_id}/{cap_id}` 解析
- EVENT_ID 显示前 8 位
- CBAC 过滤：只显示有 read 权限的 block 事件 + 所有 editor 事件
- 空结果显示 `No events found.`

### 5b: 查看指定 Block 的历史

**操作**：

```bash
# 按名称查询
elf event history src/main.rs

# 按 UUID 查询
elf event history <block_id>
```

**预期输出**：

```
Block: src/main.rs (a1b2c3d4)

CAPABILITY           EDITOR       CREATED_AT           EVENT_ID
──────────────────────────────────────────────────────────────────────────
core.create          system       2025-01-01 12:00:00  a1b2c3d4
document.write       alice        2025-01-01 12:05:00  e5f6g7h8
...

Total: <N> events
```

验证：
- Block ID 显示短 ID（前 8 位）
- CAPABILITY 和 EDITOR 分列显示
- 只显示该 block 相关的事件
- 空结果显示 `No events found for block '<name>'.`

### 5c: 对应 MCP 工具

| CLI 命令 | MCP 等价工具 |
|----------|-------------|
| `elf event list` | 读取 Resource `elfiee://{project}/events` |
| `elf event history <block>` | `elfiee_block_history(project, block_id)` |

---

## Journey 6: 时间回溯

**场景**：查看某个 block 在历史某个时间点的完整状态。

### Step 1: 获取事件列表

**操作**：

```bash
elf event history src/main.rs
```

记下目标 event 的 EVENT_ID。

### Step 2: 回溯到目标时间点

**操作**：

```bash
elf event at src/main.rs <event_id>
```

**预期输出**：

```
Block state at event <event_id>:

  ID:    <block_id>
  Name:  src/main.rs
  Type:  document
  Owner: system
  Desc:  <description>            # 仅当 description 非空时显示

Contents:
{
  "content": "...",
  ...
}

Grants at this point (N):          # 仅当 grants 非空时显示
  <editor_id> — <cap_id> on <block_id>
  ...
```

验证：
- 显示的是该 event 发生时刻的 block 状态（非当前状态）
- Contents 为 pretty-printed JSON
- Grants 列表反映该时间点的权限快照

### 对应 MCP 工具

```
elfiee_state_at_event(project, block_id, event_id)
→ 返回 { block: Block, grants: Vec<Grant> }
```

---

## Journey 7: 增量扫描

**场景**：手动创建了新文件后，需要同步到 Elfiee。

### 操作

```bash
# 手动创建新文件
touch src/new_file.rs

# 增量扫描
elf scan
```

### 预期结果

**7.1 CLI 输出**：

```
Scanned <N> files, created <M> new blocks (<N-M> already existed)
```

验证：M >= 1（至少包含新创建的文件）。

**7.2 Block 验证**：

```bash
elf block list
```

验证：`src/new_file.rs` 出现在 block 列表中，TYPE = `document`。

---

## Journey 8: 多 Agent 模板（Socialware）

**场景**：定义多个 Agent 的角色和权限，一键注册并启动 MCP Server。

### Step 1: 编写模板

在 `.elf/templates/workflows/` 创建 TOML 文件（或使用内置 `code-review`）：

```toml
# .elf/templates/workflows/code-review.toml

[socialware]
name = "Code Review"
namespace = "code-review"
description = "Two-agent code review workflow: coder writes, reviewer reviews"

[[roles]]
id = "coder"
agent_type = "openclaw"
capabilities = [
    "document.write", "document.read",
    "session.append", "session.read",
    "task.write", "task.read", "task.commit",
    "core.create", "core.link", "core.unlink", "core.delete",
]

[[roles]]
id = "reviewer"
agent_type = "openclaw"
capabilities = [
    "document.read",
    "session.append", "session.read",
    "task.read", "task.write",
    "core.create", "core.link",
]
```

**模板结构**：

| 段 | 用途 | Elfiee 解析？ |
|----|------|--------------|
| `[socialware]` | 名称、命名空间、描述 | 是 |
| `[[roles]]` | 角色 ID、Agent 类型、权限列表 | 是 |
| `[[flows]]` | 状态机定义（给 Coordinator 用） | **否**（passthrough） |
| `[[commitments]]` | 承诺/SLA 约定（给 Coordinator 用） | **否**（passthrough） |

**细粒度权限**（可选）：

```toml
[[roles]]
id = "coder"
agent_type = "openclaw"
capabilities = ["session.append", "session.read", "core.create"]
grants = [
    { capability = "document.write", block = "src/main.rs" },
    { capability = "document.read", block = "*" },
]
```

### Step 2: 执行模板

**操作**：

```bash
elf run code-review
```

**预期输出**：

```
Starting socialware: Code Review — Two-agent code review workflow: coder writes, reviewer reviews

Registered openclaw as editor 'openclaw-<8字符>'
  MCP config -> ...
  Skill -> ...
Registered openclaw as editor 'openclaw-<8字符>'
  MCP config -> ...
  Skill -> ...

Socialware 'Code Review' ready.

  Start coder: ELFIEE_EDITOR_ID=openclaw-<8字符> claude
  Start reviewer: ELFIEE_EDITOR_ID=openclaw-<8字符> claude

Starting MCP Server on port 47200...
Ready. MCP Server: http://127.0.0.1:47200
Press Ctrl+C to stop.
```

验证：
- 每个 role 创建独立的 Editor（不同 editor_id）
- 输出包含每个 role 的启动指令
- MCP Server 正常启动
- **不创建** Task Block 或 Session Block（编排由 Coordinator 完成）

**`elf run` 只做两件事**：
1. 按模板逐 role 注册 Editor + 授权 + 注入 MCP 配置
2. 启动 MCP Server

### Step 3: 启动各 Agent

在不同终端中按输出的指令启动：

```bash
# 终端 1
ELFIEE_EDITOR_ID=openclaw-<coder_id> claude

# 终端 2
ELFIEE_EDITOR_ID=openclaw-<reviewer_id> claude
```

Agent 自动读取注入的 SKILL.md，通过 MCP 连接 Elfiee。

**模板查找顺序**：
1. `.elf/templates/workflows/<name>.toml`（项目级）
2. 内置模板（目前仅 `code-review`）
3. 未找到则报错

---

## Journey 9: 取消注册 Agent

**场景**：移除一个 Agent 的所有身份和配置（register 的逆操作）。

### 操作

```bash
elf unregister <editor_id>

# 指定配置目录
elf unregister <editor_id> --config-dir /path/to/.claude --project /path/to/project
```

### 预期结果

**9.1 CLI 输出**：

```
Unregistered editor '<editor_id>'
  Cleaned config <- /path/to/config/dir
```

**9.2 eventstore.db 变更**：

```bash
elf status
```

验证：Editor 数量减少 1。Grant 数自动级联减少（StateProjector 级联删除）。

**9.3 清理的配置文件**：

| 文件 | 清理内容 |
|------|---------|
| `.mcp.json` | 删除 `mcpServers.elfiee`；如果 mcpServers 为空则删除整个 key |
| `settings.local.json` | 删除 `env.ELFIEE_EDITOR_ID` 和 `env.ELFIEE_PROJECT`；删除所有 `mcp__elfiee__*` permissions |
| `skills/elfiee/` | 整个目录删除（SKILL.md + scripts/） |

验证：其他 MCP server（如有）和非 Elfiee 权限保留不变。

**9.4 往返验证**：

```bash
elf register openclaw --name "agent"
# 记录 editor_id
elf status          # Editors: 2
elf unregister <editor_id>
elf status          # Editors: 1（恢复到注册前）
```

---

## CLI 命令速查

| 命令 | 描述 |
|------|------|
| `elf init [project]` | 初始化 .elf/ 项目（自动扫描文件创建 blocks） |
| `elf register <type> [--name <n>] [--config-dir <d>] [--project <p>] [--port 47200]` | 注册 Agent |
| `elf unregister <editor_id> [--config-dir <d>] [--project .]` | 取消注册 Agent（register 的逆操作） |
| `elf serve [--port 47200] [--project <p>]` | 启动 MCP Server |
| `elf run <template> [--project <p>] [--port 47200]` | 按 Socialware 模板注册角色 + 启动 MCP Server |
| `elf status [project]` | 查看项目状态 |
| `elf scan [--project .]` | 增量扫描文件创建 blocks |
| `elf block list [--project .]` | 列出所有 blocks |
| `elf event list [--project .]` | 列出所有事件 |
| `elf event history <block> [--project .]` | 查看 block 事件历史 |
| `elf event at <block> <event_id> [--project .]` | 时间回溯：查看 block 在指定 event 时的状态 |
| `elf grant <editor_id> <cap> [block] [--project .]` | 授予权限 |
| `elf revoke <editor_id> <cap> [block] [--project .]` | 撤回权限 |

---

## MCP 工具速查（18 个）

### 连接管理（4）

| 工具 | 描述 |
|------|------|
| `elfiee_auth` | 认证连接（绑定 editor_id） |
| `elfiee_open` | 打开项目（返回 Skill） |
| `elfiee_close` | 关闭项目 |
| `elfiee_file_list` | 列出已打开的项目 |

### 区块 CRUD（5）

| 工具 | CBAC |
|------|------|
| `elfiee_block_list` | `{type}.read` 过滤 |
| `elfiee_block_get` | `{type}.read`（覆盖所有类型的读取） |
| `elfiee_block_create` | `core.create` |
| `elfiee_block_delete` | `core.delete` |
| `elfiee_block_rename` | 需 write 权限 |

### DAG 关系（2）

| 工具 | CBAC |
|------|------|
| `elfiee_block_link` | `core.link` |
| `elfiee_block_unlink` | `core.unlink` |

### 时间回溯（2）

| 工具 | CBAC |
|------|------|
| `elfiee_block_history` | `{type}.read`（区块事件历史） |
| `elfiee_state_at_event` | `{type}.read`（时间旅行快照） |

### 权限管理（4）

| 工具 | 说明 |
|------|------|
| `elfiee_grant` / `elfiee_revoke` | Owner 专属 |
| `elfiee_editor_create` / `elfiee_editor_delete` | Owner 专属 |

### 通用执行（1）

| 工具 | 说明 |
|------|------|
| `elfiee_exec` | 执行任意 capability — 替代原有 9 个 extension tool |

**`elfiee_exec` 覆盖的操作**：

| Capability | block_type | Payload |
|---|---|---|
| `document.write` | document | `{"content": "..."}` |
| `task.write` | task | `{"description":..., "status":..., "assigned_to":...}` |
| `task.commit` | task | `{}` |
| `session.append` | session | `{"entry_type":"...", "data":{...}}` |

> 读取操作：`elfiee_block_get` 返回任意类型区块的完整内容（CBAC: `{type}.read`）。
> 任务创建：`elfiee_block_create` + `block_type="task"`，再用 `elfiee_exec(task.write)` 设置描述。
> 任务关联：`elfiee_block_link` + `relation="implement"`。

### MCP Resources（只读）

| URI | 描述 |
|-----|------|
| `elfiee://files` | 已打开项目列表 |
| `elfiee://{project}/blocks` | 所有区块 |
| `elfiee://{project}/block/{id}` | 单个区块详情 |
| `elfiee://{project}/grants` | 权限表 |
| `elfiee://{project}/events` | 事件日志 |
| `elfiee://{project}/editors` | 编辑者列表 |
| `elfiee://{project}/my-tasks` | 当前 Editor 的任务 |
| `elfiee://{project}/my-grants` | 当前 Editor 的权限 |

---

## 端到端测试覆盖

`tests/cli_e2e_integration.rs` 中 7 个 E2E 测试覆盖 Journey 1-8 的完整链路（走 services 层）：

| 测试函数 | 覆盖 Journey | 验证内容 |
|----------|-------------|---------|
| `test_init_scans_files` | J1 初始化 | `elf init` → 自动扫描文件 → 创建 document blocks |
| `test_init_register_flow` | J2 注册 Agent | `elf init` → `elf register` → Editor + Grants + MCP 配置注入 |
| `test_scan_incremental` | J3 增量扫描 | 新增文件 → `elf scan` → 只创建新 block，不重复 |
| `test_agent_create_write_block` | J4 Agent 操作 | 创建 block → `exec(document.write)` → 读取验证 |
| `test_grant_revoke_permission_flow` | J6 权限管理 | grant → 验证授权 → revoke → 验证拒绝 |
| `test_fine_grained_block_permission` | J7 细粒度权限 | 通配符 grant → 单 block grant → revoke 单 block |
| `test_resolve_block_by_name_and_id` | J8 Block 解析 | name 解析 → id 解析 → 短 id 解析 → 不存在报错 |

**其他集成测试**（`tests/` 目录）：

| 测试文件 | 测试数 | 覆盖范围 |
|----------|--------|---------|
| `commands_block_permissions.rs` | CBAC 权限矩阵 | 所有 capability × 授权/未授权 × owner/non-owner |
| `relation_integration.rs` | Block DAG | link/unlink/循环检测/级联删除 |
| `project_integration.rs` | 项目生命周期 | open/close/reopen + ElfProject 持久化 |

**总测试数**：329 pass（295 unit + 33 integration + 1 doc-test）

**架构一致性验证**：所有 transport（Tauri Commands / CLI / MCP）均通过 services 层调用 engine，无直接 `handle.process_command()` 旁路。

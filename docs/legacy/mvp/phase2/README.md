# Elfiee 使用指南

Elfiee 是一个 AI 原生编辑器，作为 AI 开发工具（Claude Code、Cursor 等）的**决策记忆层**。它不替代你的 AI 工具，而是在后台记录你的意图、实现和验证过程，建立完整的因果链条，并驱动 Git 提交。

本文档以一次完整的开发流程为主线，介绍如何使用 Elfiee 进行日常开发。

---

## 核心概念

开始之前，了解 Elfiee 的几个核心概念：

| 概念 | 说明 |
|------|------|
| **.elf 文件** | Elfiee 的项目文件，存储所有 Block、事件记录和权限配置。本质是一个 ZIP 包 |
| **Block（块）** | 内容的基本单元。每个 Block 有类型、内容和关系图 |
| **Event（事件）** | 所有操作都会产生不可变的事件记录，是系统的唯一数据来源 |
| **Agent（代理）** | 连接外部 AI 工具（如 Claude Code）的桥梁。每个 Agent 管理一个外部项目的 `.claude/` 目录 |
| **implement 关系** | Block 之间的因果链接。`A → B` 表示"A 的变更导致了 B 的变更" |

### Block 类型

| 类型 | 用途 | 典型内容 |
|------|------|----------|
| **markdown** | 文档、笔记、需求描述 | MyST 格式的 Markdown 文本 |
| **code** | 源代码 | 任意编程语言的源文件 |
| **directory** | 文件树管理 | 外部项目的目录结构映射 |
| **task** | 任务跟踪 | 需求描述 + 关联代码的因果链 |
| **agent** | AI 代理配置 | 绑定的 config 目录、Provider、启停状态 |
| **terminal** | 命令执行 | 交互式终端会话 |

---

## 完整开发流程

以下是使用 Elfiee + Claude Code 完成一次功能开发的完整流程：

```
创建 .elf 项目 → 导入代码 → 创建 Agent → 创建 Task → AI 开发 → 提交到 Git → 禁用 Agent
```

### 第 1 步：创建 .elf 项目

在 Elfiee GUI 中新建项目，选择保存位置。

**Elfiee 自动完成**：
- 创建 `.elf` 文件（ZIP 格式）
- 启动事件引擎
- 初始化系统目录结构（`.elf/` 内的 SKILL 模板、Git Hooks 模板等）
- 创建默认编辑者身份

### 第 2 步：导入外部项目代码

在 Elfiee 中创建一个 **Directory Block**，然后导入你的外部项目路径（如 `~/projects/my-app/`）。

**Elfiee 自动完成**：
- 扫描外部目录，为每个文件创建对应的 Block（Markdown 或 Code）
- 在 Directory Block 的 entries 中记录 `相对路径 → Block ID` 的映射
- 在 metadata 中记录 `external_root_path`（外部项目的绝对路径）
- 跳过 `.git/`、`node_modules/` 等隐藏目录和忽略模式

导入后，外部项目的所有文件都成为 Elfiee 中可编辑、可追踪的 Block。

### 第 3 步：创建 Agent（连接 Claude Code）

在侧边栏添加 Global Collaborator，选择 Bot 类型，填写：
- **名称**：如 "Claude Code"
- **Config Directory**：AI 工具的配置目录路径，如 `/home/user/projects/my-app/.claude`
- **Provider**：选择 Claude Code / Cursor / Windsurf

也可以在 CollaboratorList 中为已有的 Bot 编辑者开启 Agent 开关，系统会提示选择 config 目录。

**Elfiee 自动完成**：
1. 创建 Bot 编辑者身份
2. 为 Bot 授予全局权限（24 个 capability 的通配符 grant）
3. 创建 Agent Block，记录 config_dir、provider、status
4. 启动**独立的 MCP Server**（分配端口 47201-47299）
5. 创建符号链接：`{config_dir}/skills/elfiee-client/` → `.elf/` 内的 Skill 目录
6. 注入 MCP 配置到 `{project_root}/.mcp.json` 和 `{config_dir}/mcp.json`

完成后，Elfiee 会提示"请重启 Claude Code 以激活 MCP 连接"。

**重启 Claude Code 后**，Claude 会自动发现 Elfiee 的 MCP Server，获得以下能力：

| MCP 工具 | 作用 |
|----------|------|
| `elfiee_file_list` | 列出已打开的 .elf 项目 |
| `elfiee_block_list` / `elfiee_block_get` | 浏览项目结构 |
| `elfiee_markdown_read` / `elfiee_markdown_write` | 读写文档 |
| `elfiee_code_read` / `elfiee_code_write` | 读写代码 |
| `elfiee_directory_import` / `elfiee_directory_export` | 导入/导出文件 |
| `elfiee_task_read` / `elfiee_task_write` | 读写任务 |
| `elfiee_task_commit` | 提交任务到 Git |
| `elfiee_block_link` / `elfiee_block_unlink` | 建立/解除因果关系 |
| `elfiee_terminal_execute` | 执行终端命令 |

> **注意**：Agent 不具备 `core.grant` / `core.revoke` 权限。权限管理只能由 Block Owner（通常是人类用户）在 Elfiee GUI 中操作。这是有意的安全设计 — AI 工具不应自行扩展或修改权限。

#### Skill 自动发现机制

启用 Agent 后，Elfiee 会在 `{config_dir}/skills/elfiee-client/` 创建符号链接，其中包含 `SKILL.md` 文件。Claude Code 通过以下机制自动使用这个 Skill：

1. **自动发现**：Claude Code 启动时扫描 `{config_dir}/skills/` 目录，读取每个 Skill 的 `SKILL.md` 中的 `description` 字段
2. **语义匹配**：当用户的 prompt 内容与 Skill 描述语义相关时，Claude Code **自动加载**该 Skill 的完整内容作为上下文
3. **无需手动触发**：你不需要在每个 prompt 前加 `/elfiee-client`。只要你的请求涉及 block、task、代码读写等 Elfiee 相关操作，Claude 会自动激活 elfiee-client skill

**什么时候 Claude 会自动使用 Elfiee Skill**：
- 提到 "block"、"task"、"commit"、".elf" 等关键词时
- 要求读写 Elfiee 项目中的代码或文档时
- 要求建立因果关系或导航关系图时

**如果 Claude 没有自动使用**：可以在 prompt 中明确提及 "使用 elfiee MCP 工具" 或 "通过 elfiee 操作"，帮助 Claude 识别上下文。

### 第 4 步：创建 Task

创建一个 **Task Block**，填写任务标题和描述。这是本次开发的需求定义。

可以在 Elfiee GUI 中直接创建，也可以让 Claude Code 通过 MCP 创建：

> "创建一个新任务：添加用户认证功能"

Claude 会调用 `elfiee_block_create` + `elfiee_task_write` 完成创建。

### 第 5 步：AI 开发

这是核心开发阶段。你在 Claude Code 中正常编写代码，Claude 通过 MCP 工具操作 Elfiee：

1. **读取上下文**：`elfiee_code_read` 读取已有代码
2. **修改代码**：`elfiee_code_write` 写入新代码或修改
3. **建立因果链接**：`elfiee_block_link(task_block, code_block, "implement")` — 标记"这段代码是为了实现这个任务"

每一步操作都会产生事件记录，形成完整的审计链：谁、在什么时候、对哪个 Block、做了什么操作。

**因果链示例**：

```
Task: "添加用户认证"
  ├─ implement → Code: "src/auth.rs"
  ├─ implement → Code: "src/middleware.rs"
  └─ implement → Code: "tests/auth_test.rs"
```

### 第 6 步：提交到 Git（Task Commit）

开发完成后，执行 Task Commit。可以在 GUI 中点击任务的"提交"按钮，也可以让 Claude 通过 MCP 调用 `elfiee_task_commit`。

**Elfiee 自动完成**：

1. **发现关联项目**：
   - 沿着 Task 的 `implement` 关系找到所有下游 Code Block
   - 从 Directory Block 的 entries 反查每个 Code Block 的外部路径
   - 按外部项目分组

2. **注入 Git Hooks**（首次提交时）：
   - 从 `.elf/` 中读取 pre-commit hook 脚本
   - 设置 `git config core.hooksPath` 指向 Elfiee 管理的 hook 目录
   - Hook 会**链式调用**原项目的 hook（husky 等照常运行）

3. **导出代码**：
   - 将 Code Block 的内容写回外部文件系统的原始路径
   - 例如：Block 对应 `src/auth.rs` → 写入 `~/projects/my-app/src/auth.rs`

4. **Git 操作**：
   ```
   git checkout -b feat/{task-name}      # 创建 feature 分支
   git add src/auth.rs src/middleware.rs  # 仅添加导出的文件
   ELFIEE_TASK_COMMIT=1 git commit -m "添加用户认证: ..."  # 提交
   ```

5. **返回结果**：commit hash、分支名、导出文件列表

#### Git Hook 保护机制

Task Commit 后，Elfiee 注入的 pre-commit hook 会保护外部项目：

| 场景 | 结果 |
|------|------|
| 通过 Elfiee 的 task.commit 提交 | 通过（设置了 `ELFIEE_TASK_COMMIT=1` 环境变量） |
| 在外部直接 `git commit` | 拦截，提示使用 Elfiee 的 task.commit |
| 使用 `git commit --no-verify` | 强制绕过（紧急情况） |
| 原项目的 husky 等 hook | 照常运行（链式调用） |

这确保了所有代码变更都有对应的 Elfiee 事件记录。

### 第 7 步：禁用 Agent（可选）

任务完成后，可以在 CollaboratorItem 中关闭 Agent 开关，或在 GUI 中禁用 Agent。

**Elfiee 自动完成**：
- 停止该 Agent 的 MCP Server
- 删除符号链接（`{config_dir}/skills/elfiee-client/`）
- 从 `.mcp.json` 和 `{config_dir}/mcp.json` 中移除 elfiee 配置
- 更新 Agent Block 状态为 Disabled

外部项目恢复到启用 Agent 之前的状态。下次需要时重新 Enable 即可。

---

## 权限管理

Elfiee 使用 **CBAC（Capability-Based Access Control）** 模型管理权限。

### 基本规则

1. **Block Owner** 拥有该 Block 的全部权限，无需额外授权
2. **其他编辑者** 需要通过 `core.grant` 获得特定能力
3. **通配符 Grant**：`block_id = "*"` 表示对所有 Block 都有该权限

### 全局协作者

通过侧边栏的 "Add Global Collaborator" 添加全局协作者，系统会自动授予 24 个通配符权限，覆盖所有读写操作。全局协作者可以在任意 Block 上执行读写操作。

**Agent（Bot）的权限范围**：Agent 获得的 24 个权限覆盖所有读写、链接、任务操作，但**不包括 `core.grant` 和 `core.revoke`**。权限管理始终由人类 Owner 在 Elfiee GUI 中操作，AI 工具无法自行扩展或修改权限。

### 权限层级

| 权限类别 | 包含的 Capability |
|----------|-------------------|
| 核心操作 | core.read, core.create, core.delete, core.link, core.unlink, core.rename, core.change_type, core.update_metadata |
| Markdown | markdown.read, markdown.write |
| Code | code.read, code.write |
| Directory | directory.read, directory.write, directory.create, directory.delete, directory.rename |
| Terminal | terminal.init, terminal.execute, terminal.save, terminal.close |
| Task | task.read, task.write, task.commit |

---

## Block 关系（因果链）

Elfiee 使用 `implement` 关系建立 Block 之间的因果链条。这是唯一的关系类型。

### 语义

`A → B`（A 的 children 中包含 B）表示："A 的变更导致了 B 的变更"。

### 何时建立关系

| 场景 | 关系 |
|------|------|
| Task 描述需求，编写 Code 实现 | Task → Code |
| PRD 定义任务，从中拆分 Task | PRD → Task |
| 为 Code 编写 Test | Code → Test |
| Bug 报告导致代码修复 | Bug → Code |

### 为什么关系重要

1. **task.commit 依赖关系图**：提交时，Elfiee 沿着 Task 的 implement 关系找到所有关联代码，自动导出并提交
2. **上下文导航**：AI 可以沿着关系图向上（理解需求）、向下（查看实现）、横向（查看相关模块）获取上下文
3. **审计追溯**：每个代码变更都可以追溯到驱动它的需求

### DAG 约束

关系图是严格的有向无环图（DAG）。创建 link 时 Elfiee 会自动检测环路并拒绝。

#### 关系方向规则

链接方向遵循**因果方向**：从"驱动变更的原因"指向"被驱动的结果"。

```
原因 → 结果
Task → Code        # 任务驱动代码编写
Code → Test        # 代码驱动测试编写
Bug  → Code        # Bug 报告驱动代码修复
```

**先建立链接，再修改内容**。在修改 Block B 之前，先创建 `A → B` 链接，声明修改的因果来源。

#### TDD 场景的最佳实践

TDD 流程中，测试和代码可能交替修改。正确的关系方向：

```
Task → Code → Test
```

**不要创建反向链接** `Test → Code`，否则会形成环路（`Code → Test → Code`）并被 Elfiee 拒绝。

如果修改测试后发现需要回去修改代码，这不是 `Test → Code` 的因果关系 — 实际驱动力仍然是原始 Task。正确做法：

1. `Task → Code`（Task 驱动代码编写）
2. `Code → Test`（代码驱动测试编写）
3. 修改测试发现代码需要调整 → 仍然是 Task 驱动的，不需要新链接

**环路拒绝处理**：如果 `elfiee_block_link` 返回环路错误，说明链接方向有误。重新审视因果关系，确保箭头从上游（原因）指向下游（结果）。

### 上下文导航（图优先）

Elfiee 的 SKILL.md 指导 AI 工具在需要上下文时，优先沿关系图导航，而非搜索无关 Block：

1. 向上遍历 parents — 理解"为什么要做这个修改"
2. 向下遍历 children — 查看"这个需求产生了哪些实现"
3. 横向查看 siblings — 了解"相关模块有哪些"
4. 仅在关系图信息不足时，搜索无关 Block

> **当前状态**：图优先导航是 SKILL.md 中的提示词指导，Claude 会尽量遵循但不是代码层面强制的。效果取决于 Claude 对 SKILL.md 的理解程度。

---

## 多 Agent 协作

Elfiee 支持同时启用多个 Agent，每个 Agent 绑定不同的外部项目或同一项目的不同 AI 工具。

### 工作方式

- 每个 Agent 拥有**独立的 MCP Server**（端口 47201-47299）
- 每个 Agent 拥有**独立的 editor_id**，操作可追溯到具体的 Agent
- 每个 Agent 的权限独立管理（各自的 24 个通配符 grant）

### 当前限制

- **无自动合并**：多个 Agent 同时修改同一个 Block 时，后写入的会覆盖先写入的。Elfiee 目前不提供自动合并或冲突检测。
- **无锁机制**：Block 没有悲观锁，两个 Agent 可以同时写入同一 Block。

### 最佳实践

1. **按模块分工**：不同 Agent 负责不同的目录或模块，避免同时修改同一文件
2. **按 Task 分工**：每个 Agent 绑定不同的 Task，各自管理自己的因果链
3. **同一项目不同工具**：例如一个 Claude Code Agent 负责后端、一个 Cursor Agent 负责前端
4. **串行提交**：多个 Agent 的 task.commit 应串行执行，避免 Git 冲突

---

## 自动恢复

Elfiee 在打开 `.elf` 文件时会自动恢复所有已启用的 Agent：

1. 重放事件日志，重建状态
2. 扫描所有 `status = enabled` 的 Agent Block
3. 为每个 Agent 分配新端口，启动 MCP Server
4. 更新外部项目的 `.mcp.json`（端口可能与上次不同）
5. 刷新符号链接

关闭 `.elf` 文件时，自动停止所有 Agent 的 MCP Server。

**注意**：端口不跨会话持久化。每次打开文件都会分配新端口，但 Elfiee 会自动更新配置文件，所以重启 Claude Code 后就能重新连接。

---

## 常见问题

### Claude Code 连接不上 Elfiee

1. 确认 Agent 状态为 Enabled（在 CollaboratorItem 中查看）
2. 确认已重启 Claude Code（MCP 配置在启动时读取）
3. 检查 `{project_root}/.mcp.json` 中是否有 `elfiee` 配置条目

### task.commit 提示 "no linked projects found"

Task 没有通过 `implement` 关系链接到任何 Code Block，或链接的 Code Block 不属于任何 Directory Block。确保：
1. Task → Code Block 有 implement 关系
2. Code Block 是某个 Directory Block 的 entry（通过导入创建的）

### 外部项目的 git commit 被拦截

Elfiee 的 pre-commit hook 正在保护该项目。有三种处理方式：
1. 使用 Elfiee 的 task.commit 提交（推荐）
2. 在 Elfiee GUI 中关闭 Commit Protect 开关
3. 使用 `git commit --no-verify` 绕过（紧急情况）

### Agent 端口变了，Claude 连不上

每次打开 `.elf` 文件，Agent 会分配新端口。Elfiee 会自动更新 `.mcp.json`，但 Claude Code 需要重启才能读取新配置。

---

## 术语表

| 术语 | 说明 |
|------|------|
| .elf | Elfiee 项目文件格式（ZIP 包，内含事件数据库和 Block 快照） |
| Block | 内容的基本单元，有 ID、类型、内容和关系图 |
| Event | 不可变的操作记录，包含实体、属性、值、向量时钟 |
| Capability | 定义一种操作（如 `markdown.write`），包含授权检查和执行逻辑 |
| Grant | 权限授予记录：`(editor_id, cap_id, block_id)` |
| Agent | 连接外部 AI 工具的配置。管理符号链接、MCP Server 和权限 |
| MCP | Model Context Protocol，AI 工具与外部服务的标准通信协议 |
| implement | Block 间唯一的关系类型，表示因果依赖 |
| config_dir | AI 工具的配置目录（如 `.claude/`、`.cursor/`） |
| task.commit | 将 Task 关联的代码导出到外部 Git 并提交 |

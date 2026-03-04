# Changelog: System Skills & MCP 集成

> **PR**: [#66 feat: elfiee skill md](https://github.com/H2OSLabs/elfiee/pull/66)
> **分支**: `feat/system-skills` -> `dev`
> **作者**: zyli-developer
> **日期**: 2026-01-22 ~ 2026-01-30
> **变更规模**: +2,716 / -15，涉及 14 个文件

---

## 概述

本 PR 经历了多轮迭代，从最初的 Skill 文档编写，逐步演进为完整的 AI Agent 集成方案。核心目标是让 AI（如 Claude Code）能够通过标准化协议与 Elfiee 交互，操作 `.elf` 文件。

**演进路线**:
1. Skill 文档编写 -> 2. CLI 工具 -> 3. HTTP IPC Server -> 4. **MCP (Model Context Protocol) 集成**（最终方案）

最终方案采用 MCP 协议，替代了中间尝试的 HTTP API 方案，提供更标准化的 AI-应用交互能力。

---

## 1. Skill 文档体系建立

### 1.1 CLAUDE.md 更新

**文件**: `CLAUDE.md`

新增 "CRITICAL: Required Skills" 章节，强制开发者在参与项目前必须阅读两份核心 Skill 文档：

- **系统级** `docs/skills/elfiee-dev/SKILL.md` — AI 与 `.elf` 文件交互规则（禁止 `cat`/`ls`/`rm` 等文件系统命令，必须使用 elf API）
- **项目级** `docs/skills/elfiee-workflow/SKILL.md` — 前后端开发规则（禁止编辑 `bindings.ts`、禁止使用 `invoke()` 等）

### 1.2 Skill 文档迭代

经过 4 次提交迭代，Skill 文档从初版演进为结构化的开发指南：

| 提交 | 内容 |
|------|------|
| `190acc0` | 初始 skill 文档 |
| `68a0bbf` | 文档内容更新 |
| `92ad904` | 添加 capability 注册和 payload 定义规范 |
| `f9c7663` | 拆分为 `elfiee-dev`（系统级）和 `elfiee-workflow`（项目级）两份文档 |

> **注**: 最终提交 `d6f6c53` 移除了 `docs/skills/` 下的旧文档，说明 Skill 文档已迁移到 `.claude/skills/` 目录下（Claude Code 标准路径）。

---

## 2. Elfiee MCP 集成（核心变更）

### 2.1 架构演进

本 PR 中 AI 集成方案经历了三次架构迭代：

| 阶段 | 提交 | 方案 | 状态 |
|------|------|------|------|
| Phase 1 | `ff58889` | Elfiee CLI（命令行工具） | 被替代 |
| Phase 2 | `d8a3a78` / `d870b97` | HTTP IPC Server | 被替代 |
| Phase 3 | `14bbf98` | **MCP (Model Context Protocol)** | **最终方案** |

### 2.2 MCP Server 实现

**涉及文件**:
- `src-tauri/src/mcp/mod.rs` — MCP 模块结构
- `src-tauri/src/mcp/server.rs` — MCP Server 核心实现
- `src-tauri/src/mcp/transport.rs` — MCP 传输层

**连接模式**:
- **内嵌模式**: Tauri 应用内部启动 MCP Server，AI Agent 通过 stdio 连接
- **独立模式**: 独立运行 MCP Server，支持外部工具连接

### 2.3 MCP 工具集

新增 `.claude/skills/elfiee-mcp/SKILL.md`（+189 行），定义了完整的 MCP 工具参考：

**文件操作**:
- `elfiee_file_list` — 列出所有 .elf 文件

**Block 操作**:
- `elfiee_block_list` — 列出文件中的所有 block
- `elfiee_block_create` — 创建新 block
- `elfiee_block_read` — 读取 block 内容
- `elfiee_block_delete` — 删除 block

**内容操作**:
- `elfiee_markdown_read` / `elfiee_markdown_write` — Markdown 读写
- `elfiee_code_read` / `elfiee_code_write` — 代码读写

**目录操作**:
- `elfiee_directory_list` — 列出目录内容
- `elfiee_directory_create` — 创建目录/文件

**终端操作**:
- `elfiee_terminal_create` — 创建终端会话
- `elfiee_terminal_write` — 发送命令
- `elfiee_terminal_read` — 读取输出

**权限管理**:
- `elfiee_grant` / `elfiee_revoke` — 授予/撤销权限

**编辑器管理**:
- `elfiee_editor_list` / `elfiee_editor_create` — 编辑器列表/创建

**执行**:
- `elfiee_exec` — 在 block 工作区执行命令

### 2.4 MCP Resources

支持通过 URI 访问 Elfiee 资源：

| URI | 说明 |
|-----|------|
| `elfiee://files` | 所有 .elf 文件列表 |
| `elfiee://{project}/blocks` | 项目中所有 block |
| `elfiee://{project}/block/{id}` | 指定 block 详情 |
| `elfiee://{project}/grants` | 权限授予列表 |
| `elfiee://{project}/events` | 事件日志 |

---

## 3. 后端变更

### 3.1 依赖更新

**文件**: `src-tauri/Cargo.toml`

添加 MCP 相关依赖（具体 crate 根据实现需要）。

### 3.2 状态管理更新

**文件**: `src-tauri/src/state.rs`

扩展应用状态管理以支持 MCP Server 的生命周期管理。

### 3.3 Engine Manager 更新

**文件**: `src-tauri/src/engine/manager.rs`

修改 Engine Manager 以支持 MCP 通道的命令接入，使 MCP 工具调用能够路由到正确的 Engine Actor。

### 3.4 应用初始化更新

**文件**: `src-tauri/src/lib.rs`

更新 Tauri 应用初始化流程，注册 MCP Server 组件。

---

## 4. 前端变更

### 4.1 ContextPanel 更新

**文件**: `src/components/editor/ContextPanel.tsx`

UI 组件调整（具体变更与 MCP 集成配合）。

---

## 5. 配置与工程化

### 5.1 MCP 配置迁移

- **删除**: `.mcp.json`（项目根目录旧配置）
- **更新**: `.claude/mcp.json`（迁移到 Claude Code 标准路径，API key 改用环境变量 `${CONTEXT7_API_KEY}`）

### 5.2 .gitignore 更新

新增忽略规则：
- `**/temp/` — 临时目录
- `.claude/settings.local.json` — Claude Code 本地设置

---

## 6. 已回退变更

### 6.1 MCP Notifications 机制

提交 `9f9b328` 实现了 MCP 通知机制（实时状态广播），但随后被 `03f6d8b` 回退。

**回退原因**: 该功能包含 peer registry、dispatcher 等模块，引入了较大复杂度，需要进一步设计后重新实现。

**回退内容**:
- 通知模块（notifications）
- Peer 注册表（peer registry）
- 事件分发器（dispatcher）
- EventStore 事件重载命令

---

## 7. 已清理文件

| 文件 | 操作 | 原因 |
|------|------|------|
| `.mcp.json` | 删除 | 迁移到 `.claude/mcp.json` |
| `docs/skills/elfiee-dev/SKILL.md` | 删除 | 迁移到 `.claude/skills/` |
| `docs/skills/elfiee-workflow/SKILL.md` | 删除 | 迁移到 `.claude/skills/` |

---

## 8. 影响分析

### 对开发者的影响

1. **AI 开发工作流**: 开发者可通过 MCP 协议使用 Claude Code 直接操作 `.elf` 文件
2. **Skill 文档**: 新贡献者需阅读 `.claude/skills/` 下的 Skill 文档了解开发规范
3. **配置变更**: MCP 配置从 `.mcp.json` 迁移到 `.claude/mcp.json`

### 对用户的影响

- 无直接用户可见变更（本 PR 为基础设施/工具链改进）

### 已知限制

1. MCP Notifications（实时广播）已回退，后续需重新设计
2. MCP Server 当前为内嵌模式，独立模式需进一步测试

---

## 9. 提交时间线

| 日期 | 提交 | 描述 |
|------|------|------|
| 01-22 | `190acc0` | 初始 skill 文档 |
| 01-22 | `68a0bbf` | 更新 skill 文档 |
| 01-23 | `92ad904` | 添加 capability 注册和 payload 定义规范 |
| 01-23 | `f9c7663` | 拆分为 elfiee-dev 和 elfiee-workflow 两份文档 |
| 01-25 | `ff58889` | 添加 Elfiee CLI（后被 MCP 替代） |
| 01-26 | `d8a3a78` | 实现 IPC Server（后被 MCP 替代） |
| 01-26 | `d870b97` | 实现 HTTP IPC Server（后被 MCP 替代） |
| 01-29 | `ed802bb` | 合并 dev 分支 |
| 01-29 | `14bbf98` | **实现 MCP 集成（最终方案）** |
| 01-29 | `a325e52` | 合并远程分支 |
| 01-30 | `4ada999` | 增强 MCP 文档 |
| 01-30 | `9f9b328` | 实现 MCP Notifications（后回退） |
| 01-30 | `03f6d8b` | 回退 MCP Notifications |
| 01-30 | `d6f6c53` | 清理旧 Skill 文档 |
| 01-30 | `9ea0595` | 清理旧 .mcp.json 配置 |

---

**最后更新**: 2026-01-30

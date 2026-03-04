# Phase 2 — Skills 模块需求文档 (3.3)

> **版本**: v1.1
> **对应计划**: `docs/mvp/phase2/task-and-cost_v3.md` §3.3
> **预估人时**: 5
> **任务编号**: F7-01, F7-02, F7-03

---

## 目录

1. [概述](#1-概述)
2. [术语与约定](#2-术语与约定)
3. [现状分析](#3-现状分析)
4. [总体设计](#4-总体设计)
5. [详细需求：F7-01 elfiee-client SKILL 模板](#5-详细需求f7-01-elfiee-client-skill-模板)
6. [详细需求：F7-02 elfiee-client MCP 配置模板](#6-详细需求f7-02-elfiee-client-mcp-配置模板)
7. [详细需求：F7-03 模板复制工具](#7-详细需求f7-03-模板复制工具)
8. [模板占位符规范](#8-模板占位符规范)
9. [与其他模块的交互](#9-与其他模块的交互)
10. [错误处理](#10-错误处理)
11. [文件结构](#11-文件结构)
12. [依赖关系](#12-依赖关系)
13. [测试计划](#13-测试计划)
14. [验收标准](#14-验收标准)

---

## 1. 概述

### 1.1 模块定位

Skills 模块负责维护 Elfiee 的**系统级 Skill 模板**——即内置的 `elfiee-client` 工具包。这个工具包通过 Claude Code 的 Skills 机制（`SKILL.md`）和 MCP 协议（`mcp.json`）让外部 AI 编码工具学会如何操作 `.elf` 文件。

在 Phase 2 中，Skills 模块是**静态模板**驱动的：模板文件在编译时嵌入二进制，运行时复制到 `.elf/Agents/elfiee-client/` 目录。后续版本可扩展为动态 Skill 导入。

### 1.2 核心价值

- **教 AI 使用 Elfiee**：`SKILL.md` 定义了 Claude Code 应如何通过 MCP 工具与 Elfiee 交互的完整指南
- **零配置集成**：SSE 模式下模板为固定配置，无需路径占位符替换
- **单一职责**：Phase 2 仅维护一个系统级 Skill（`elfiee-client`），所有 Agent Block 共享

### 1.3 设计约束

| 约束 | 说明 |
| :--- | :--- |
| **唯一系统级 Skill** | Phase 2 只有 `elfiee-client`，不支持用户自定义 Skill |
| **静态模板** | 模板手动维护，编译时嵌入，不支持运行时修改 |
| **Claude Code 专属** | `SKILL.md` 格式遵循 Claude Code Skills 规范（YAML Frontmatter + Markdown Body） |
| **SSE 模式** | 当前使用 SSE 模式（port 47200），mcp.json 为固定配置无需占位符。`_elf_path` 参数预留给未来 Standalone CLI 模式 |

### 1.4 与原计划 (task-and-cost_v3.md §3.3) 的设计演进

原计划与实际实现存在三处设计演进，均为合理的架构调整：

| 方面 | 原计划 (§3.3) | 实际实现 | 原因 |
| :--- | :--- | :--- | :--- |
| **MCP 工具模型** | 单一 `execute_command` 工具 | 29 个专用工具 (`elfiee_file_list` 等) | MCP Server (PR #66) 实现时选择了更细粒度的工具定义，提升 AI 的调用准确率 |
| **连接模式** | CLI/stdio (`elfiee mcp-server --elf {path}`) | SSE (port 47200，嵌入 GUI 进程) | Phase 2 MCP Server 嵌入 Tauri 进程，共享 `AppState`，无需独立进程 |
| **`{elf_path}` 占位符** | mcp.json 中需替换为实际路径 | SSE 模式无需路径，固定配置 | SSE 服务端已知所有打开的文件，客户端不需要指定路径 |

> **YAML Frontmatter 说明**：原计划 F7-01 中的"YAML Frontmatter"指的是 Claude Code Skills 标准格式——SKILL.md 文件顶部的 `---\nname: ...\ndescription: ...\n---` 元数据区。这不是一个独立的 YAML 配置文件来生成 SKILL.md，而是 SKILL.md 文件本身的一部分。当前实现已包含完整的 YAML Frontmatter。

---

## 2. 术语与约定

| 术语 | 含义 |
| :--- | :--- |
| **SKILL.md** | Claude Code 的 Skills 定义文件。放在 `.claude/skills/{skill-name}/SKILL.md`，Claude 自动读取其中的指令 |
| **elfiee-client** | Elfiee 的内置 Skill 名称，教 Claude 如何通过 MCP 工具操作 `.elf` 文件 |
| **模板文件** | 存放在 `src-tauri/templates/elfiee-client/` 的源文件，编译时嵌入二进制 |
| **Block 目录** | 运行时 `.elf/Agents/elfiee-client/` Block 对应的物理目录（`block-{uuid}/`） |
| **软连接注入** | `agent.enable` 将 Block 目录软连接到目标项目 `.claude/skills/elfiee-client/` |
| **`{elf_path}`** | 模板占位符，运行时替换为 `.elf` 文件的实际物理路径 |

---

## 3. 现状分析

### 3.1 已存在的 Skills 基础设施

当前项目中已有完善的 Skills 体系，用于开发期间的 AI 辅助：

| 位置 | 内容 | 与 Phase 2 Skills 模块关系 |
| :--- | :--- | :--- |
| `.claude/skills/elfiee-mcp/SKILL.md` | 教 Claude 使用 Elfiee MCP 工具的指南（26 个工具 + 7 个 Resource） | **参考模板** — Phase 2 的 `SKILL.md` 应基于此内容精简和标准化 |
| `.claude/skills/elfiee-be-dev/SKILL.md` | 后端开发 Skill | 不涉及（开发期工具） |
| `.claude/skills/elfiee-fe-dev/SKILL.md` | 前端开发 Skill | 不涉及（开发期工具） |
| `.claude/skills/mcp-builder/SKILL.md` | MCP Server 构建指南 | 不涉及（开发期工具） |

### 3.2 已实现的相关组件

| 组件 | 位置 | 用途 |
| :--- | :--- | :--- |
| MCP Server（SSE） | `src-tauri/src/mcp/server.rs` | 26 个 MCP 工具的完整实现，Skills 中需要引用这些工具名称 |
| MCP 配置合并器 | `src-tauri/src/utils/mcp_config.rs` | `merge_server()` / `remove_server()` / `resolve_template()` 已实现 |
| Agent enable I/O | `src-tauri/src/commands/agent.rs` | 软连接创建和 MCP 配置注入逻辑已实现 |
| `.elf/` 初始化 | `extensions/directory/elf_meta.rs` | `.elf/Agents/elfiee-client/` 目录骨架在创建 `.elf` 文件时自动生成 |
| `inject_block_dir` | `engine/actor.rs` | 运行时为 Block 注入物理目录路径 |

### 3.3 尚未实现的部分

| 组件 | 说明 |
| :--- | :--- |
| `src-tauri/templates/` 目录 | **不存在** — 需要新建模板目录和文件 |
| `SKILL.md` 标准化模板 | 需要从现有 `.claude/skills/elfiee-mcp/SKILL.md` 中提炼面向终端用户的版本 |
| `mcp.json` 配置模板 | 需要创建标准化模板文件 |
| 模板复制工具函数 | 需要将模板嵌入二进制并在运行时复制到 Block 目录 |

---

## 4. 总体设计

### 4.1 工作流全景

```
编译时:
  src-tauri/templates/elfiee-client/
  ├── SKILL.md          ──── include_str!() ────→  嵌入到 Rust 二进制
  ├── mcp.json          ──── include_str!() ────→  嵌入到 Rust 二进制
  └── references/
      └── capabilities.md  ── include_str!() ──→  嵌入到 Rust 二进制

运行时（创建 .elf 文件时 / I10-01）:
  template_copy::init_elfiee_client(block_dir, _elf_path)
    │
    ├── 1. 将嵌入的 SKILL.md 写入 {block_dir}/Agents/elfiee-client/SKILL.md
    ├── 2. 将嵌入的 mcp.json 写入 Block 目录（SSE 模式，固定配置无需替换）
    ├── 3. 写入 references/capabilities.md
    └── 4. 创建空的 scripts/ 和 assets/ 目录
    注: _elf_path 参数预留给未来 Standalone CLI 模式占位符替换

启用 Agent 时（agent.enable / F3-01）:
  commands/agent.rs:
    │
    ├── 创建 symlink:
    │     {external_path}/.claude/skills/elfiee-client/
    │     → {elf_block_dir}/Agents/elfiee-client/
    │
    └── 合并 MCP 配置:
          mcp_config::merge_server(
            "{external_path}/.claude/mcp.json",
            "elfiee",
            build_elfiee_server_config(elf_path)
          )
```

### 4.2 模板 vs 运行时配置

Skills 模块涉及两种配置注入方式：

| 文件 | 注入方式 | 说明 |
| :--- | :--- | :--- |
| `SKILL.md` | **软连接** | 通过 `agent.enable` 软连接到 `.claude/skills/`，Claude 自动读取 |
| `mcp.json` | **合并写入** | 不通过软连接。`agent.enable` 读取模板内容，替换占位符，合并到目标 `.claude/mcp.json` |
| `references/capabilities.md` | **软连接**（间接） | 随 `elfiee-client/` 目录一起被软连接，SKILL.md 内可引用 |

### 4.3 SKILL.md 内容结构设计

`SKILL.md` 遵循 Claude Code Skills 规范：

```markdown
---
name: elfiee-client
description: "Guide for using Elfiee..."
---

# Elfiee Client Skill

## 1. Connection
## 2. Quick Start
## 3. Tool Reference
## 4. Workflow Examples
## 5. Error Handling
```

**YAML Frontmatter** 是 Claude Code 识别 Skill 的关键：
- `name`: Skill 标识符，用于 Claude 日志和识别
- `description`: Skill 用途描述，Claude 据此判断何时激活

---

## 5. 详细需求：F7-01 elfiee-client SKILL 模板

### 5.1 任务概述

| 属性 | 值 |
| :--- | :--- |
| **编号** | F7-01 |
| **任务名称** | elfiee-client SKILL 模板 |
| **文件位置** | `src-tauri/templates/elfiee-client/SKILL.md` |
| **预估人时** | 2 |

### 5.2 SKILL.md 内容规范

`SKILL.md` 的目标读者是 **Claude Code AI Agent**（非人类开发者），因此内容需满足：

1. **结构化**：清晰的章节标题，便于 AI 快速定位
2. **完整性**：覆盖所有可用 MCP 工具和典型工作流
3. **约束性**：明确禁止的操作（如不可直接用 shell 命令操作 .elf 内容）

### 5.3 必须包含的章节

#### 5.3.1 YAML Frontmatter

```yaml
---
name: elfiee-client
description: "Guide for using Elfiee MCP tools to interact with .elf files. Use when Claude needs to read, write, or manage blocks inside .elf projects via MCP tools (elfiee_file_list, elfiee_block_*, elfiee_markdown_*, elfiee_code_*, elfiee_directory_*, elfiee_terminal_*, elfiee_grant/revoke, elfiee_editor_*, elfiee_exec) or MCP resources (elfiee://files, elfiee://{project}/blocks, elfiee://{project}/block/{id}, elfiee://{project}/grants, elfiee://{project}/events). Triggers: working with .elf files, managing blocks, reading/writing markdown or code in blocks, directory operations inside .elf, terminal sessions, permission management."
---
```

> **description 设计原则**：列举所有工具名和资源 URI 前缀，确保 Claude Code 在用户提到相关操作时能准确匹配到此 Skill。

#### 5.3.2 关键约束声明（Critical Rules）

必须在文件顶部声明：
```markdown
**CRITICAL**: NEVER use filesystem commands (`cat`, `ls`, `rm`, etc.) on `.elf` contents.
Always use MCP tools.
```

#### 5.3.3 连接方式

说明两种连接模式及其配置：

| 模式 | 传输方式 | 适用场景 |
| :--- | :--- | :--- |
| **GUI mode** | SSE on port 47200 | Elfiee GUI 已打开并加载文件 |
| **Standalone mode** | stdio (JSON-RPC) | 无需 GUI，Claude Code 启动 `elfiee mcp-server --elf <path>` |

#### 5.3.4 工具参考表

以表格形式列出所有 MCP 工具，按分类组织：

- **文件发现**: `elfiee_file_list`
- **Block CRUD**: `elfiee_block_list`, `elfiee_block_get`, `elfiee_block_create`, `elfiee_block_delete`, `elfiee_block_rename`, `elfiee_block_change_type`, `elfiee_block_update_metadata`
- **Block 关系**: `elfiee_block_link`, `elfiee_block_unlink`
- **内容读写**: `elfiee_markdown_read/write`, `elfiee_code_read/write`
- **目录操作**: `elfiee_directory_create/delete/rename/write/import/export`
- **终端操作**: `elfiee_terminal_init/execute/save/close`
- **权限管理**: `elfiee_grant`, `elfiee_revoke`
- **编辑器管理**: `elfiee_editor_create/delete`
- **通用执行**: `elfiee_exec`

每个工具需列出：工具名、用途、关键参数。

#### 5.3.5 MCP Resources

列出只读资源 URI：

| URI Pattern | 描述 |
| :--- | :--- |
| `elfiee://files` | 当前打开的 .elf 项目列表 |
| `elfiee://{project}/blocks` | 项目中所有 Block 概要 |
| `elfiee://{project}/block/{block_id}` | 特定 Block 完整内容 |
| `elfiee://{project}/grants` | 权限授予表 |
| `elfiee://{project}/events` | 事件溯源日志 |

#### 5.3.6 工作流示例

至少包含以下工作流：

1. **读取所有 Markdown Block**：`file_list` → `block_list`（筛选 markdown）→ `markdown_read`
2. **在目录 Block 中创建代码文件**：`file_list` → `block_list`（找目录 Block）→ `directory_create`
3. **执行终端命令**：`file_list` → `block_list`（找终端 Block）→ `terminal_init` → `terminal_execute` → `terminal_close`

#### 5.3.7 错误处理指南

常见错误和解决方案：

| 错误 | 原因 | 修复方式 |
| :--- | :--- | :--- |
| `Project not open` | .elf 文件未在 GUI 中打开 | 先在 Elfiee GUI 中打开文件 |
| `Block not found` | 无效的 block_id | 使用 `elfiee_block_list` 获取有效 ID |
| `No active editor` | 无编辑器会话 | GUI 需有活跃编辑器会话 |
| `Engine not found` | Engine 未启动 | 重新打开文件 |

### 5.4 与现有 elfiee-mcp SKILL 的关系

现有的 `.claude/skills/elfiee-mcp/SKILL.md` 是**开发期使用**的 Skill，直接存在于项目的 `.claude/skills/` 中。Phase 2 的 `SKILL.md` 模板：

- **内容来源**：基于现有 `elfiee-mcp/SKILL.md` 整理和标准化
- **差异点**：
  - 移除开发期特有的说明
  - 添加 Standalone 模式的完整配置说明
  - 确保工具列表与 `mcp/server.rs` 中实际注册的工具完全一致
  - 使用更简洁的格式，减少 AI 的 token 消耗

### 5.5 SKILL.md 质量标准

| 标准 | 说明 |
| :--- | :--- |
| **工具名准确性** | 每个工具名必须与 `mcp/server.rs` 中注册的名称完全一致 |
| **参数完整性** | 必填参数必须标注，可选参数必须说明默认值 |
| **示例可执行** | 工作流示例中的每一步都是可直接执行的 MCP 调用 |
| **无占位符** | `SKILL.md` 中不包含需要运行时替换的占位符（与 `mcp.json` 不同） |

---

## 6. 详细需求：F7-02 elfiee-client MCP 配置模板

### 6.1 任务概述

| 属性 | 值 |
| :--- | :--- |
| **编号** | F7-02 |
| **任务名称** | elfiee-client MCP 配置模板 |
| **文件位置** | `src-tauri/templates/elfiee-client/mcp.json` |
| **预估人时** | 1 |

### 6.2 模板内容

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

### 6.3 设计决策

#### 6.3.1 SSE 模式（当前实现）vs CLI 模式（原计划）

**原计划**（task-and-cost_v3.md 中描述）：
```json
{
  "mcpServers": {
    "elfiee": {
      "command": "elfiee",
      "args": ["mcp-server", "--elf", "{elf_path}"]
    }
  }
}
```

**当前实现**（`utils/mcp_config.rs` 中的 `build_elfiee_server_config`）：
```json
{
  "type": "sse",
  "url": "http://127.0.0.1:47200/sse"
}
```

当前实现使用 **SSE 模式**，MCP Server 嵌入在 Elfiee GUI 进程中（端口 47200），共享 `AppState`。这意味着：

- 模板中 **不需要** `{elf_path}` 占位符（SSE 模式下路径信息在服务端已知）
- `mcp.json` 模板实际上是一个**固定配置**，不需要运行时替换
- 但保留 `{elf_path}` 占位符的 `resolve_template()` 能力，供未来 Standalone 模式使用

#### 6.3.2 模板用途

`mcp.json` 模板文件在 Phase 2 中的实际用途：

1. **文档参考**：作为标准 MCP 配置的参考格式
2. **未来 CLI 模式**：当 Standalone MCP Server（`elfiee mcp-server --elf <path>`）就绪后，切换到 CLI 模式配置
3. **实际注入**：当前 `agent.enable` 使用 `build_elfiee_server_config()` 硬编码生成配置，不直接读取模板文件

> **注意**：当前的 `agent.enable` 实现（`commands/agent.rs`）并不读取模板文件，而是通过 `mcp_config::build_elfiee_server_config()` 硬编码生成 SSE 配置。模板文件主要作为标准化参考和未来扩展的基础。

### 6.4 模板格式规范

| 属性 | 要求 |
| :--- | :--- |
| **JSON 格式** | 标准 JSON，2 空格缩进 |
| **顶层结构** | `{"mcpServers": {...}}` |
| **Server 名称** | `"elfiee"` — 必须与 `mcp_config.rs` 中使用的名称一致 |
| **占位符语法** | `{elf_path}` — 大括号包围，无空格 |
| **编码** | UTF-8，无 BOM |

---

## 7. 详细需求：F7-03 模板复制工具

### 7.1 任务概述

| 属性 | 值 |
| :--- | :--- |
| **编号** | F7-03 |
| **任务名称** | 模板复制工具 |
| **文件位置** | `src-tauri/src/utils/template_copy.rs`（新建） |
| **预估人时** | 2 |

### 7.2 核心设计

模板文件在编译时通过 `include_str!()` 嵌入 Rust 二进制，运行时复制到 `.elf/` Block 目录中。

### 7.3 接口定义

```rust
/// 将 elfiee-client 模板文件初始化到指定的 Block 目录。
///
/// 在 .elf 文件创建时（I10-01 elf_meta.rs）调用此函数，
/// 将编译时嵌入的模板文件写入到 .elf/Agents/elfiee-client/ 目录。
///
/// # Arguments
/// * `block_dir` - .elf/ Dir Block 的物理目录路径（`block-{uuid}/`）
/// * `elf_path` - 当前 .elf 文件的物理路径（用于替换 mcp.json 中的占位符）
///
/// # Returns
/// * `Ok(())` - 所有文件写入成功
/// * `Err(String)` - 任何文件写入失败
///
/// # 创建的文件结构
/// ```text
/// {block_dir}/Agents/elfiee-client/
/// ├── SKILL.md
/// ├── mcp.json
/// ├── scripts/          (空目录)
/// ├── assets/           (空目录)
/// └── references/
///     └── capabilities.md
/// ```
pub fn init_elfiee_client(block_dir: &Path, elf_path: &str) -> Result<(), String>
```

### 7.4 实现规范

#### 7.4.1 模板嵌入

```rust
// 编译时嵌入模板内容
const SKILL_MD: &str = include_str!("../../templates/elfiee-client/SKILL.md");
const MCP_JSON: &str = include_str!("../../templates/elfiee-client/mcp.json");
const CAPABILITIES_MD: &str = include_str!("../../templates/elfiee-client/references/capabilities.md");
```

#### 7.4.2 目录创建

函数需创建以下目录结构：

```
{block_dir}/Agents/elfiee-client/
{block_dir}/Agents/elfiee-client/scripts/
{block_dir}/Agents/elfiee-client/assets/
{block_dir}/Agents/elfiee-client/references/
{block_dir}/Agents/session/
```

使用 `std::fs::create_dir_all()` 确保幂等性。

#### 7.4.3 文件写入

| 目标路径 | 源 | 占位符替换 |
| :--- | :--- | :--- |
| `Agents/elfiee-client/SKILL.md` | `SKILL_MD` | **无** |
| `Agents/elfiee-client/mcp.json` | `MCP_JSON` | **无**（SSE 模式为固定配置） |
| `Agents/elfiee-client/references/capabilities.md` | `CAPABILITIES_MD` | **无** |

#### 7.4.4 幂等性

- 如果目标文件已存在，**覆盖写入**（确保模板更新后能生效）
- 如果目标目录已存在，静默跳过目录创建

#### 7.4.5 错误处理

```rust
/// 写入单个模板文件
fn write_template_file(
    target_path: &Path,
    content: &str,
) -> Result<(), String> {
    // 确保父目录存在
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }

    std::fs::write(target_path, content)
        .map_err(|e| format!("Failed to write {}: {}", target_path.display(), e))
}
```

### 7.5 调用时机

`init_elfiee_client()` 在以下场景被调用：

| 调用方 | 时机 | 说明 |
| :--- | :--- | :--- |
| `elf_meta.rs`（I10-01） | 创建新 `.elf` 文件时 | `.elf/` Dir Block 初始化过程中，在创建目录骨架后调用 |

> **不在 `agent.enable` 中调用**：`agent.enable` 假设模板文件在 Block 目录中已存在（由 `.elf/` 初始化保证）。如果文件缺失，enable 的 symlink 会指向空目录，但不会 crash。

### 7.6 references/capabilities.md 内容

`capabilities.md` 是 Elfiee 所有 Capabilities 的参考文档，供 AI 在制定操作计划时查阅。

**必须包含的内容**：

```markdown
# Elfiee Capabilities Reference

## Core Capabilities
- `core.create` - Create new blocks
- `core.link` - Add relations between blocks
- `core.unlink` - Remove relations between blocks
- `core.delete` - Soft-delete blocks
- `core.grant` - Grant capabilities to editors
- `core.revoke` - Revoke capabilities from editors

## Content Capabilities
- `markdown.write` / `markdown.read`
- `code.write` / `code.read`

## Directory Capabilities
- `directory.create` / `directory.delete` / `directory.rename`
- `directory.write` / `directory.import` / `directory.export`

## Terminal Capabilities
- `terminal.init` / `terminal.execute` / `terminal.save` / `terminal.close`

## Agent Capabilities
- `agent.create` / `agent.enable` / `agent.disable`
```

每个 Capability 需说明：
- 用途
- 所需参数（Payload 结构）
- 目标 Block 类型
- 返回的 Event 结构

---

## 8. 模板占位符规范

### 8.1 当前状态（SSE 模式）

当前实现使用 **SSE 模式**，mcp.json 为固定配置（`"type": "sse", "url": "http://127.0.0.1:47200/sse"`），**不需要占位符替换**。

`init_elfiee_client()` 的 `_elf_path` 参数当前未使用，预留给未来 Standalone CLI 模式。

### 8.2 未来扩展（Standalone CLI 模式）

当 Standalone MCP Server 就绪后，mcp.json 模板将切换为 CLI 模式，届时需要占位符替换：

| 占位符 | 出现位置 | 替换值 | 替换时机 |
| :--- | :--- | :--- | :--- |
| `{elf_path}` | `mcp.json`（CLI 模式） | `.elf` 文件的物理绝对路径 | `init_elfiee_client()` 调用时 |

CLI 模式的 mcp.json 格式（未来实现）：
```json
{
  "mcpServers": {
    "elfiee": {
      "command": "elfiee",
      "args": ["mcp-server", "--elf", "{elf_path}"]
    }
  }
}
```

占位符替换逻辑可复用已实现的 `mcp_config::resolve_template()` 函数。

---

## 9. 与其他模块的交互

### 9.1 上游：被谁调用

```
I10-01: .elf/ Dir Block 初始化 (elf_meta.rs)
   │
   └── 调用 template_copy::init_elfiee_client(block_dir, _elf_path)
       ├── 写入 SKILL.md
       ├── 写入 mcp.json（SSE 固定配置）
       ├── 写入 references/capabilities.md
       └── 创建空目录 scripts/, assets/
```

### 9.2 下游：谁消费产出

```
F3-01: agent.enable (commands/agent.rs)
   │
   ├── 读取 {elf_block_dir}/Agents/elfiee-client/ 目录
   │   └── 创建 symlink → {external_path}/.claude/skills/elfiee-client/
   │
   └── 读取 MCP 配置（当前实际使用 build_elfiee_server_config() 硬编码）
       └── 合并到 {external_path}/.claude/mcp.json

Claude Code (外部):
   │
   ├── 读取 .claude/skills/elfiee-client/SKILL.md
   │   └── 学习如何使用 Elfiee MCP 工具
   │
   └── 读取 .claude/mcp.json
       └── 连接 Elfiee MCP Server
```

### 9.3 交互时序图

```
创建 .elf 文件:
  elf_meta.rs ─── init_elfiee_client() ───→ 写入模板到 Block 目录
                                              │
                                              ▼
                                        .elf/Agents/elfiee-client/
                                        ├── SKILL.md
                                        ├── mcp.json
                                        └── references/capabilities.md

启用 Agent:
  commands/agent.rs ─── agent.enable ───→ 创建 symlink
                                          │
                                          ▼
                              .claude/skills/elfiee-client/ → Block 目录
                              .claude/mcp.json              ← 合并配置

Claude Code 使用:
  Claude Code ─── 读取 SKILL.md ───→ 学习 Elfiee 交互规则
              ─── 连接 MCP Server ──→ 通过 MCP 工具操作 .elf 文件
```

---

## 10. 错误处理

### 10.1 init_elfiee_client 错误

| 错误条件 | 错误消息 | 影响 |
| :--- | :--- | :--- |
| block_dir 不存在 | `"Block directory does not exist: {path}"` | `.elf/` 初始化失败 |
| 目录创建失败 | `"Failed to create directory {path}: {io_error}"` | 模板目录结构不完整 |
| 文件写入失败 | `"Failed to write {path}: {io_error}"` | 模板文件缺失 |
| 占位符替换后 JSON 无效 | 不会发生（`resolve_template` 保持 JSON 结构） | N/A |

### 10.2 降级策略

如果 `init_elfiee_client()` 部分失败：

| 缺失文件 | 影响 | 降级方案 |
| :--- | :--- | :--- |
| `SKILL.md` 缺失 | Claude 无法学习 Elfiee 使用方式 | Agent 仍可创建和启用，但 Claude 不知道如何使用 MCP 工具 |
| `mcp.json` 缺失 | 无直接影响（当前使用硬编码配置） | 无影响 |
| `capabilities.md` 缺失 | Claude 缺少 Capability 参考 | 影响有限，Claude 仍可通过 SKILL.md 中的工具列表工作 |

---

## 11. 文件结构

### 11.1 新建文件

```
src-tauri/templates/                          # 新建目录
└── elfiee-client/                            # 新建目录
    ├── SKILL.md                              # F7-01: Skill 定义模板
    ├── mcp.json                              # F7-02: MCP 配置模板
    └── references/                           # 新建目录
        └── capabilities.md                   # Capabilities 参考文档

src-tauri/src/utils/
└── template_copy.rs                          # F7-03: 模板复制工具
```

### 11.2 修改文件

```
src-tauri/src/utils/mod.rs                    # 添加 pub mod template_copy
src-tauri/src/extensions/directory/elf_meta.rs # 在 .elf/ 初始化中调用 init_elfiee_client()
```

### 11.3 运行时生成的文件结构

`.elf/` 内部（Block 目录）：
```
block-{elf-dir-uuid}/
└── Agents/
    ├── elfiee-client/
    │   ├── SKILL.md                 # 从模板复制
    │   ├── mcp.json                 # 从模板复制（SSE 固定配置）
    │   ├── scripts/                 # 空目录（预留）
    │   ├── assets/                  # 空目录（预留）
    │   └── references/
    │       └── capabilities.md      # 从模板复制
    └── session/                     # 空目录（Session 同步模块使用）
```

外部项目（通过 `agent.enable` 注入）：
```
{external_path}/
├── .claude/
│   ├── skills/
│   │   └── elfiee-client/           # symlink → Block 目录
│   │       ├── SKILL.md
│   │       ├── mcp.json
│   │       └── references/
│   │           └── capabilities.md
│   └── mcp.json                     # 合并写入（非 symlink）
└── .mcp.json                        # 合并写入（项目根目录）
```

---

## 12. 依赖关系

### 12.1 上游依赖

| 依赖 | 模块 | 说明 |
| :--- | :--- | :--- |
| `.elf/` Dir Block 初始化 | I10 (`elf_meta.rs`) | `init_elfiee_client()` 由此处调用，需要 Block 目录已存在 |
| Block 目录注入 | Engine (`inject_block_dir`) | 运行时 `_block_dir` 路径可用 |
| MCP Server 实现 | F4-F5 (`mcp/server.rs`) | SKILL.md 中引用的工具名必须与实际注册的一致 |

### 12.2 下游依赖

| 被依赖方 | 模块 | 说明 |
| :--- | :--- | :--- |
| Agent enable | F3-01 (`commands/agent.rs`) | `agent.enable` 创建的 symlink 指向此模块生成的文件 |
| Claude Code | 外部 | 通过 SKILL.md 学习如何使用 Elfiee |

### 12.3 并行可行性

| 任务 | 是否可并行 | 说明 |
| :--- | :--- | :--- |
| F7-01 (SKILL.md) | ✅ 可并行 | 独立的模板编写，无代码依赖 |
| F7-02 (mcp.json) | ✅ 可并行 | 独立的模板编写 |
| F7-03 (template_copy.rs) | ⚠️ 需 F7-01/F7-02 就绪 | 需要模板文件存在才能通过 `include_str!()` 编译 |

### 12.4 开发顺序建议

```
1. F7-01 + F7-02: 编写模板文件（可并行）          (2h + 1h)
2. F7-03: 实现 template_copy.rs                   (2h)
   ├── 嵌入模板
   ├── 实现 init_elfiee_client()
   └── 集成到 elf_meta.rs
```

---

## 13. 测试计划

### 13.1 单元测试

#### F7-01: SKILL.md 内容验证

| 测试 | 内容 |
| :--- | :--- |
| `test_skill_md_has_frontmatter` | 验证包含 YAML Frontmatter（以 `---` 开头和结尾） |
| `test_skill_md_has_name` | 验证 Frontmatter 中 `name: elfiee-client` |
| `test_skill_md_has_description` | 验证 Frontmatter 中有 `description` 字段 |
| `test_skill_md_mentions_all_tools` | 验证提到所有 29 个 MCP 工具名 |
| `test_skill_md_mentions_critical_rule` | 验证包含"NEVER use filesystem commands"警告 |
| `test_skill_md_not_empty` | 验证内容非空且长度合理（>500 字符） |

#### F7-02: mcp.json 格式验证

| 测试 | 内容 |
| :--- | :--- |
| `test_mcp_json_valid` | 模板是合法 JSON |
| `test_mcp_json_has_mcp_servers` | 包含 `mcpServers` 字段 |
| `test_mcp_json_has_elfiee_server` | 包含 `elfiee` server 配置 |
| `test_mcp_json_server_config` | server 配置包含必要字段（`type` 或 `command`） |

#### F7-03: template_copy 功能测试

| 测试 | 内容 |
| :--- | :--- |
| `test_init_creates_directory_structure` | 验证创建完整的目录结构 |
| `test_init_writes_skill_md` | 验证 SKILL.md 写入成功且内容非空 |
| `test_init_writes_mcp_json` | 验证 mcp.json 写入成功且是合法 JSON |
| `test_init_writes_capabilities_md` | 验证 capabilities.md 写入成功 |
| `test_init_creates_empty_dirs` | 验证 scripts/ 和 assets/ 空目录创建 |
| `test_init_creates_session_dir` | 验证 session/ 目录创建 |
| `test_init_idempotent` | 调用两次不报错，文件被覆盖 |

### 13.2 集成测试

| 测试 | 内容 |
| :--- | :--- |
| `test_elf_creation_includes_templates` | 创建新 .elf 文件后，`.elf/Agents/elfiee-client/` 中有模板文件 |
| `test_agent_enable_symlink_has_content` | `agent.enable` 后 symlink 目标目录中文件可读 |

---

## 14. 验收标准

### 14.1 功能验收

- [x] `src-tauri/templates/elfiee-client/SKILL.md` 存在且内容完整
- [x] `src-tauri/templates/elfiee-client/mcp.json` 存在且是合法 JSON
- [x] `src-tauri/templates/elfiee-client/references/capabilities.md` 存在且内容完整
- [x] 创建新 `.elf` 文件后，`.elf/Agents/elfiee-client/` 目录中包含所有模板文件
- [x] `mcp.json` 为 SSE 固定配置（无占位符替换，当前实现正确）
- [ ] `agent.enable` 创建的 symlink 指向的目录中 `SKILL.md` 可读（需端到端验证）
- [ ] Claude Code 能通过 symlink 读取 SKILL.md 并识别为有效 Skill（需端到端验证）

### 14.2 技术验收

- [x] 模板文件通过 `include_str!()` 编译时嵌入（无运行时文件依赖）
- [x] `init_elfiee_client()` 函数幂等（多次调用不报错）
- [x] `template_copy.rs` 有完整的单元测试（11 个单元 + 12 个集成）
- [x] SKILL.md 中引用的工具名与 `mcp/server.rs` 中注册的一致（29 个工具）
- [x] 模板文件不包含硬编码的绝对路径

### 14.3 内容验收

- [x] SKILL.md 包含 YAML Frontmatter（`name` + `description`）
- [x] SKILL.md 覆盖所有 29 个 MCP 工具
- [x] SKILL.md 覆盖所有 MCP Resource URI
- [x] SKILL.md 包含至少 3 个工作流示例（实际包含 4 个）
- [x] SKILL.md 包含关键约束声明（禁止直接 shell 操作）
- [x] capabilities.md 覆盖所有已注册的 Capabilities

### 14.4 里程碑对照

对应 **M3: Skills + Session**（Week 2 Day 3-5）：
- ✓ SKILL.md 生效（通过 symlink 可被 Claude 读取）
- ✓ 模板文件随 .elf 文件创建自动生成

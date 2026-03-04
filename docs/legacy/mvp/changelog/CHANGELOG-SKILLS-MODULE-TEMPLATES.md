# Changelog: Skills 模块 — elfiee-client 模板系统

> **分支**: `feat/agent-block`
> **日期**: 2026-02-02
> **变更规模**: +900 行，涉及 7 个新文件、2 个修改文件
> **需求文档**: `docs/mvp/phase2/skills-module-requirements.md`
> **任务编号**: F7-01, F7-02, F7-03

---

## 概述

实现 Skills 模块的模板系统，在 `.elf` 文件创建时自动将 `elfiee-client` Skill 模板文件（SKILL.md、mcp.json、capabilities.md）写入 `.elf/` Dir Block 的物理目录中。

**核心机制**: 模板通过 `include_str!()` 在编译时嵌入二进制，运行时由 `template_copy::init_elfiee_client()` 写入 `block-{uuid}/Agents/elfiee-client/` 目录。当 `agent.enable` 创建 symlink 后，Claude Code 即可读取 SKILL.md 获得完整的 Elfiee 操作指南。

**数据流**:
```
create_file()
  -> bootstrap_elf_meta()
       -> core.create .elf/ block
       -> directory.write (目录骨架)
       -> core.grant (权限)
       -> init_elfiee_client()  [NEW: Step 4]
            -> 写入 SKILL.md, mcp.json, capabilities.md
```

---

## 0. 与原计划的设计演进

原计划 (task-and-cost_v3.md §3.3) 与实际实现存在三处设计演进：

| 方面 | 原计划 | 实际实现 | 原因 |
|------|--------|---------|------|
| MCP 工具模型 | 单一 `execute_command` 工具 | 29 个专用工具 | PR #66 MCP Server 选择了细粒度工具，提升 AI 调用准确率 |
| 连接模式 | CLI/stdio (`elfiee mcp-server --elf {path}`) | SSE (port 47200) | MCP Server 嵌入 Tauri 进程，共享 AppState |
| `{elf_path}` 占位符 | mcp.json 需要替换 | 固定配置，无需替换 | SSE 服务端已知所有打开的文件 |

**关于 YAML Frontmatter**：原计划 F7-01 描述的 "YAML Frontmatter" 是 Claude Code Skills 标准格式（`---\nname: ...\ndescription: ...\n---`），是 SKILL.md 文件本身的元数据头，不是独立的 YAML 配置文件。当前实现已包含完整的 YAML Frontmatter。

---

## 1. 模板文件

### 1.1 SKILL.md (F7-01)

**文件**: `src-tauri/templates/elfiee-client/SKILL.md`

Claude Code Skill 定义文件，包含：

- YAML frontmatter（`name: elfiee-client`）
- 29 个 MCP 工具的分类参考表（File、Block、Content、Directory、Terminal、Permission、Editor、Exec）
- 4 个工作流示例（创建 markdown block、目录操作、终端执行、权限管理）
- MCP Resources URI 参考（`elfiee://files`、`elfiee://{project}/blocks` 等）
- 关键约束：**NEVER use filesystem commands**（强制通过 MCP 工具操作）

### 1.2 mcp.json (F7-02)

**文件**: `src-tauri/templates/elfiee-client/mcp.json`

MCP Server 连接配置：

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

SSE 模式，连接 Tauri 内嵌 MCP Server（端口 47200）。

### 1.3 capabilities.md (F7-03)

**文件**: `src-tauri/templates/elfiee-client/references/capabilities.md`

完整的 Capability 参考文档，涵盖：

| 分类 | Capabilities |
|------|-------------|
| Core | create, read, link, unlink, delete, grant, revoke, update_metadata, rename, change_type |
| Content | markdown.write, markdown.read, code.write, code.read |
| Directory | directory.create, delete, rename, write, import, export |
| Terminal | terminal.init, execute, save, close |
| Agent | agent.create, enable, disable |
| Editor | editor.create, delete |

每个 Capability 包含 Purpose、Target、Params、Returns 说明。

---

## 2. template_copy 工具模块

**文件**: `src-tauri/src/utils/template_copy.rs`（新建）

### 核心 API

```rust
pub fn init_elfiee_client(block_dir: &Path, _elf_path: &str) -> Result<(), String>
```

### 实现要点

- 使用 `include_str!()` 编译时嵌入模板，零运行时文件依赖
- 创建完整目录结构：`scripts/`、`assets/`、`references/`、`session/`
- 写入 3 个模板文件（SKILL.md、mcp.json、capabilities.md）
- 幂等操作：`create_dir_all` + 覆盖写入，可安全重复调用
- `_elf_path` 参数预留给未来 standalone CLI 模式

### 生成的目录结构

```
block-{uuid}/Agents/
├── elfiee-client/
│   ├── SKILL.md
│   ├── mcp.json
│   ├── scripts/          (空目录)
│   ├── assets/           (空目录)
│   └── references/
│       └── capabilities.md
└── session/              (会话同步目录)
```

---

## 3. bootstrap_elf_meta 集成

**文件**: `src-tauri/src/extensions/directory/elf_meta.rs`（修改）

在 `bootstrap_elf_meta()` 函数末尾新增 **Step 4**：

```rust
// Step 4: Initialize elfiee-client skill templates into the block directory.
if let Some(elf_block) = handle.get_block(elf_block_id.clone()).await {
    if let Some(block_dir) = elf_block
        .contents
        .get("_block_dir")
        .and_then(|v| v.as_str())
    {
        if let Err(e) = template_copy::init_elfiee_client(Path::new(block_dir), "") {
            eprintln!("Warning: Failed to initialize elfiee-client templates: {}", e);
        }
    }
}
```

**设计决策**:
- **Non-blocking**: 模板写入失败仅打印警告，不影响 `.elf/` Block 创建
- **依赖 `_block_dir`**: 在 `inject_block_dir` 注入物理路径后执行，确保目录已存在

---

## 4. 变更文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `src-tauri/templates/elfiee-client/SKILL.md` | 新建 | Claude Code Skill 定义 |
| `src-tauri/templates/elfiee-client/mcp.json` | 新建 | MCP SSE 连接配置 |
| `src-tauri/templates/elfiee-client/references/capabilities.md` | 新建 | Capability 参考文档 |
| `src-tauri/src/utils/template_copy.rs` | 新建 | 模板复制工具 + 11 个单元测试 |
| `src-tauri/tests/template_integration.rs` | 新建 | 12 个集成测试 |
| `docs/mvp/phase2/skills-module-requirements.md` | 新建 | 需求文档 |
| `src-tauri/src/utils/mod.rs` | 修改 | 添加 `pub mod template_copy` |
| `src-tauri/src/extensions/directory/elf_meta.rs` | 修改 | 添加 Step 4 模板初始化调用 |

---

## 5. 测试

### 5.1 单元测试（11 个）

位于 `src-tauri/src/utils/template_copy.rs`:

| 测试 | 验证内容 |
|------|---------|
| `test_init_creates_directory_structure` | 目录骨架创建 |
| `test_init_writes_skill_md` | SKILL.md 内容正确 |
| `test_init_writes_mcp_json` | mcp.json 有效且包含 elfiee server |
| `test_init_writes_capabilities_md` | capabilities.md 包含核心 capability |
| `test_init_creates_empty_dirs` | scripts/ 和 assets/ 为空目录 |
| `test_init_creates_session_dir` | session/ 目录创建 |
| `test_init_idempotent` | 重复调用不报错 |
| `test_skill_md_has_frontmatter` | YAML frontmatter 格式 |
| `test_skill_md_mentions_all_tools` | 29 个 MCP 工具名全部出现 |
| `test_mcp_json_valid` | JSON 语法有效 |
| `test_mcp_json_has_elfiee_server` | elfiee server 配置存在 |

### 5.2 集成测试（12 个）

位于 `src-tauri/tests/template_integration.rs`:

| 测试 | 验证内容 |
|------|---------|
| `test_template_directory_structure` | 完整目录结构 |
| `test_template_empty_dirs` | 空目录正确 |
| `test_skill_md_exists` | SKILL.md 文件存在 |
| `test_skill_md_frontmatter` | frontmatter name 字段 |
| `test_skill_md_contains_constraints` | 关键约束规则 |
| `test_skill_md_tool_references` | 工具名引用完整 |
| `test_skill_md_resource_uris` | MCP Resource URI |
| `test_mcp_json_valid` | JSON 有效性 |
| `test_mcp_json_server_config` | SSE 配置正确 |
| `test_capabilities_md_exists` | 文件存在 |
| `test_capabilities_md_references` | capability 引用完整 |
| `test_template_idempotency` | 幂等性验证 |

### 5.3 测试结果

全部 420 个测试通过（376 lib + 44 integration），0 回归。

---

## 6. 与其他模块的关系

| 模块 | 关系 | 说明 |
|------|------|------|
| Agent 模块 (F3) | 上游消费者 | `agent.enable` 创建 symlink 指向 block 目录，Claude Code 通过 symlink 读取 SKILL.md。支持多 Agent（每个 bot editor 独立创建），MCP Server 通过 `AgentContents.editor_id` 归因操作到正确的 bot 身份。详见 `CHANGELOG-AGENT-IDENTITY-AND-MULTI-AGENT.md` |
| `.elf/` 初始化 (I10) | 调用方 | `bootstrap_elf_meta` Step 4 调用 `init_elfiee_client` |
| MCP Server (F4-F5) | 配置对齐 | mcp.json 中的 SSE URL 与 MCP Server 端口一致 |

---

**最后更新**: 2026-02-02

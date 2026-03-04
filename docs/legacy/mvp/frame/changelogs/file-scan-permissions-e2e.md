# Changelog: L5.1 文件扫描 + 权限管理 + E2E 测试

> 日期：2026-03-03
> 分支：feat/refactor-plan
> 前置：cli-skill-workflow.md（L5 CLI + Skill + Workflow，287 tests）
> 测试：304 pass（270 unit + 8 E2E + 4 block-perm + 9 project + 12 relation + 1 doc-test）

## 概要

补全 Elfiee MVP 的文件-Block 映射、权限管理 CLI、跨路径 Agent 支持和端到端测试。
此前 L5 完成了 CLI 入口和 Skill 注入，但：
1. Block 与文件的关联缺失（init 不扫描文件）
2. 无 CLI 权限管理手段（grant/revoke 只有 MCP）
3. 无端到端链路测试
4. register 不支持跨路径 Agent
5. 模板不支持细粒度权限

---

## 设计决策

- **Block name = 文件相对路径**：`src/main.rs` 而非用户自定义标签
- **init 自动扫描**：`elf init` 时扫描项目所有文件，为每个文件创建 document block
- **增量同步**：`elf scan` 只为新增文件创建 block，已有 block 不会重复创建
- **扫描规则复用**：`.elfignore`（编译内嵌）+ `.gitignore` + `.elftypes`（扩展名→类型映射）
- **Name/ID 双模解析**：grant/revoke 的 block 参数支持文件名或 UUID
- **CLI 身份**：system editor（`~/.elf/config.json` 中的 UUID），通过 bootstrap wildcard grants 拥有 owner 权限
- **project_path 注入**：`ELFIEE_PROJECT` 环境变量让跨路径 Agent 知道项目位置
- **模板双层权限**：`capabilities`（wildcard）+ `grants`（per-block）

---

## Bug 修复

| 文件 | 修复 |
|------|------|
| `src/cli/register.rs:108` | payload 字段 `editor`/`block` → `target_editor`/`target_block`（匹配 `GrantPayload` 定义） |

---

## 新建文件

| 文件 | 行数 | 说明 |
|------|------|------|
| `src/cli/scan.rs` | 302 | 文件扫描器 + `elf scan` 命令 + `create_blocks_for_files()`（含 5 测试） |
| `src/cli/resolve.rs` | 143 | `resolve_block_id()` name/id 双模解析器（含 4 测试） |
| `src/cli/block.rs` | 49 | `elf block list` 命令 |
| `src/cli/grant.rs` | 56 | `elf grant` 命令 |
| `src/cli/revoke.rs` | 56 | `elf revoke` 命令 |
| `tests/cli_e2e_integration.rs` | 320 | 8 个 E2E 集成测试 |
| `docs/mvp/frame/changelogs/file-scan-permissions-e2e.md` | — | 本文档 |

---

## 修改文件

| 文件 | 变更 |
|------|------|
| `src/cli/init.rs` | 集成文件扫描（scan_project → create_blocks_for_files） |
| `src/cli/mod.rs` | 新增 `block`/`grant`/`resolve`/`revoke`/`scan` 模块声明 |
| `src/cli/register.rs` | Bug 修复 + `inject_mcp_config` 新增 `project_path` 参数 + `run_with_grants()` 方法 + `GrantEntry` 结构 |
| `src/cli/run.rs` | `Participant.grants` 字段 + `TemplateGrantEntry` 结构 + 调用 `run_with_grants` |
| `src/bin/elf.rs` | 新增 `Scan`/`Block`/`Grant`/`Revoke` 子命令 |
| `docs/mvp/frame/README.md` | 新增命令文档 + 跨路径用法 + 细粒度权限 + scan/block list |

---

## CLI 新增命令

### `elf scan [--project .]`

```
1. 打开项目 + 启动 engine
2. 扫描文件（.elfignore + .gitignore + .elftypes）
3. 获取已有 blocks
4. 只为不存在的文件创建新 block
```

### `elf block list [--project .]`

```
按 name 排序输出所有 blocks：NAME, TYPE, ID, OWNER
```

### `elf grant <editor_id> <capability> [block] [--project .]`

```
1. 解析 block（name/id 双模，默认 "*"）
2. 通过 system editor 执行 core.grant
```

### `elf revoke <editor_id> <capability> [block] [--project .]`

```
1. 解析 block（name/id 双模，默认 "*"）
2. 通过 system editor 执行 core.revoke
```

---

## E2E 测试

| # | 测试 | 验证点 |
|---|------|--------|
| 1 | `test_init_scans_files` | init 后 blocks ≥ 4，block name = 相对路径 |
| 2 | `test_init_register_flow` | init → register → editor + grants + MCP 配置 + ELFIEE_PROJECT 注入 + Skill 注入 |
| 3 | `test_scan_incremental` | 外部新增文件 → elf scan → 只创建新 block |
| 4 | `test_agent_create_write_block` | Agent 创建 + 写入 block 成功 |
| 5 | `test_grant_revoke_permission_flow` | revoke → owner 仍可写自己的 block |
| 6 | `test_fine_grained_block_permission` | grant 只对 block_a → agent 写 a 成功，写 b 失败 |
| 7 | `test_resolve_block_by_name_and_id` | id 匹配、name 匹配、`*` 直通、不存在报错 |
| 8 | `test_template_with_fine_grained_grants` | 解析含 grants 字段的 TOML 模板 |

---

## 验证

- `cargo check` — 零错误
- `cargo test` — 304 tests 全部通过（+17 新增）
- `cargo clippy` — 零警告

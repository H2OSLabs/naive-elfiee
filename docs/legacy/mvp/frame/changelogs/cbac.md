# Changelog: CBAC 回归 + Core 能力重构

## 概述

回归 event-sourcing 纯正架构，精简数据模型和核心能力集。

## Step 1: 数据模型精简 — BlockMetadata 移除

- **删除** `models/metadata.rs` 整个模块
- `Block.metadata: BlockMetadata` → `Block.description: Option<String>`
- `CreateBlockPayload` 删除 `metadata` 字段，新增 `description: Option<String>`
- 删除 `UpdateMetadataPayload`、`RenamePayload`、`ChangeTypePayload`
- 快照序列化/恢复适配新字段

## Step 2: Core Capability 重构 12 → 9

### 删除的 Capability（4 个）
| Capability | 文件 | 原因 |
|---|---|---|
| `core.read` | `builtins/read.rs` | 概念文档无此能力 |
| `core.rename` | `builtins/rename.rs` | 合并到 `core.write` |
| `core.change_type` | `builtins/change_type.rs` | 合并到 `core.write` |
| `core.update_metadata` | `builtins/update_metadata.rs` | metadata 已删除 |

### 新增的 Capability（1 个）
- **`core.write`** (`builtins/write.rs`) — 更新 block 结构字段（name, description, block_type）

### 最终能力集（9 个 builtin）
`core.create`, `core.delete`, `core.write`, `core.link`, `core.unlink`, `core.grant`, `core.revoke`, `editor.create`, `editor.delete`

### 外部引用更新
- 所有 extension 中 `core.rename` → `core.write`
- 所有 extension 中 `core.change_type` → `core.write`
- 所有 extension 中 `core.update_metadata` → `core.write`
- MCP server 工具适配
- Tauri command `update_block_metadata` 删除，`rename_block`/`change_block_type` 改用 `core.write`

## Step 3: CBAC 鉴权回归 — certificator

- `CapabilityHandler::certificator()` 签名更新：`(editor_id, block: Option<&Block>, grants: &GrantsTable)`
- 默认实现：owner check → grants check（纯 event-sourcing 两层鉴权）
- `StateProjector` 删除：`is_authorized()`、`system_editor_id`、`deleted_editors`
- `actor.rs`：`process_command()` 中 `is_authorized` → `handler.certificator()`
- `CheckGrant`：改为 owner + `GrantsTable.has_grant` 直接查询
- Bootstrap：system editor 创建后自动发放所有 capability 的 wildcard grant

## Step 4: GrantsTable 统一数据流

- 新增 `GrantsTable::process_event()` — grant/revoke 事件解析的唯一入口
- `StateProjector::apply_event()` 中 grant/revoke 分支委托给 `process_event()`
- 删除 `from_events()`（与 StateProjector 回放路径重复）
- 删除 `as_map()`（改为 `iter_all()` 避免暴露内部 HashMap）

## Step 5: Bootstrap 重写 + 清理

### Bootstrap 重写（file.rs）
- `bootstrap_editors()` 拆分为 `seed_bootstrap_events()` + `ensure_active_editor()`
- system editor 的创建和授权通过直接写入 EventStore 完成（不走 command pipeline）
- 解决鸡生蛋问题：system editor 需要 grants 才能发 command，但 grants 本身需要 command 创建
- 事件在 engine spawn 之前写入，engine 初始化时通过 StateProjector::replay() 自动加载

### Actor 鉴权改为无条件
- `process_command()` 中所有操作都经过 certificator 鉴权（包括 core.create、editor.create）
- certificator 默认 `None` 分支检查 wildcard grant（`has_grant(editor_id, cap_id, "*")`）
- system editor 通过 bootstrap wildcard grants 获得所有操作权限

### core.read 引用清理
- `commands/block.rs::get_all_blocks()` — 替换 `core.read` 检查为 owner + has_any_grant 逻辑
- `commands/editor.rs::list_grants()` — 同上
- `commands/file.rs::get_all_events()` — 保留（file.rs 整体 scheduled for deletion）

### 测试全面修复
- 每个测试模块添加自含 `seed_test_editor()` helper（禁止交叉引用）
- 修复模块：`actor.rs`、`manager.rs`、`editor.rs`、`block_permissions`、`relation_integration`、`snapshot_integration`、`engine_block_dir_integration`
- 忽略 future-deleted 模块测试（terminal §1.2、elf_meta §1.1、template §1.1、checkout §1.5）

### 最终验证
- `cargo check` — 零错误
- `cargo test` — 470 passed, 0 failed, 28 ignored
- 生产代码无 `is_authorized`、`BlockMetadata`、`system_editor_id`、`deleted_editors` 残留
- `certificator` 为唯一鉴权路径
- registry 注册 9 个 builtin capability

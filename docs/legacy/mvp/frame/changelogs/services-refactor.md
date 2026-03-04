# Changelog: 服务层抽取 + CBAC 严格化 + 死代码清理

> 日期：2026-03-03
> 分支：feat/refactor-plan
> 前置：file-scan-permissions-e2e.md（304 tests）
> 测试：320 pass（286 unit + 8 E2E + 4 block-perm + 9 project + 12 relation + 1 doc-test）

## 概要

代码审阅后的 6 部分重构，核心目标是统一数据总线：

1. **服务层抽取**：新建 `src/services/` 模块（8 个文件），封装 CBAC 过滤 + 业务逻辑
2. **CBAC 严格化**：MCP 读操作（block_list、block_get、document_read、资源）全部通过服务层获得 CBAC 过滤
3. **core.write 禁止修改 block_type**：block_type 在 init/scan 时确定，之后不可更改
4. **EngineHandle 扩展**：暴露 EventStore 的事件查询方法（get_events_by_entity、get_events_after_event_id、get_latest_event_id）
5. **传输层瘦化**：CLI / MCP / Tauri Commands 统一调用服务层，无例外
6. **MCP 新增事件查询工具**：elfiee_block_history、elfiee_state_at_event、elfiee_task_read、elfiee_session_read（全部带 CBAC）
7. **死代码清理**：删除 save_file、duplicate_file、change_block_type、空 commands/task.rs

---

## 设计决策

- **统一数据总线**：所有三个传输层（CLI / MCP / Tauri Commands）统一通过 `services` 层访问 Engine，无例外
- **CLI 也走服务层**：CLI 虽然使用 system editor，也必须通过服务层，保持架构一致性
- **CBAC 约定式 read**：读操作统一使用 `{block_type}.read` 能力检查（document.read、task.read、session.read）
- **事件查询也过 CBAC**：`list_events` 按 `{block_type}.read` 过滤 block 事件，editor 事件不过滤（项目级信息）
- **时间旅行也过 CBAC**：`get_state_at_event` 在回放后检查目标 block 的 `{block_type}.read` 权限
- **block_type 不可变**：在 WriteBlockPayload 中删除 block_type 字段，StateProjector 不再处理类型变更

---

## 新增文件

| 文件 | 说明 |
|------|------|
| `src/services/mod.rs` | 模块声明 |
| `src/services/block.rs` | Block CRUD + CBAC 过滤（3 tests） |
| `src/services/document.rs` | Document 读写 + CBAC（3 tests） |
| `src/services/editor.rs` | Editor 管理（薄包装） |
| `src/services/grant.rs` | Grant 管理 + CBAC 过滤（2 tests） |
| `src/services/event.rs` | 事件查询 + 时间旅行 + CBAC（3 tests） |
| `src/services/task.rs` | Task 读写提交 + CBAC（3 tests） |
| `src/services/session.rs` | Session 读取追加 + CBAC（2 tests） |

## 修改文件

| 文件 | 说明 |
|------|------|
| `src/engine/actor.rs` | +3 EngineMessage（事件查询）+ 对应 EngineHandle 方法 |
| `src/models/payloads.rs` | WriteBlockPayload 删除 block_type 字段 |
| `src/capabilities/builtins/write.rs` | 删除 block_type 处理逻辑 |
| `src/engine/state.rs` | 删除 core.write 的 block_type 应用逻辑 |
| `src/commands/block.rs` | 调用 services::block::* |
| `src/commands/file.rs` | 调用 services::event::list_events，删除 save_file、duplicate_file |
| `src/commands/editor.rs` | 调用 services::editor::* 和 services::grant::* |
| `src/commands/event.rs` | 调用 services::event::get_state_at_event |
| `src/commands/mod.rs` | 移除 task 模块、save_file 等死 re-exports |
| `src/mcp/server.rs` | 读操作调用 services，CBAC 已启用 |
| `src/cli/block.rs` | 调用 services::block::list_blocks |
| `src/cli/grant.rs` | 调用 services::grant::grant_permission |
| `src/cli/revoke.rs` | 调用 services::grant::revoke_permission |
| `src/cli/scan.rs` | 调用 services::block::list_blocks + execute_command |
| `src/lib.rs` | +services 模块，删除 save_file/duplicate_file/change_block_type 注册 |

## 删除文件

| 文件 | 原因 |
|------|------|
| `src/commands/task.rs` | 空文件（Git 操作已委托 AgentContext） |

---

## 架构图

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Tauri Cmds  │  │  MCP Server  │  │     CLI      │
│ (commands/)  │  │ (mcp/)       │  │  (cli/)      │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                 │
       └─────────────────┼─────────────────┘
                         │
                  ┌──────▼──────┐
                  │  services/  │  ← CBAC + 业务逻辑
                  └──────┬──────┘
                         │
                  ┌──────▼──────┐
                  │ EngineHandle │  ← Actor 消息传递
                  └──────┬──────┘
                         │
                  ┌──────▼──────┐
                  │ Engine Actor │  ← 串行命令处理
                  └─────────────┘
```

## 测试新增

+16 服务层测试（block: 3, document: 3, event: 3, grant: 2, task: 3, session: 2）

## 验证

- `cargo check` — 零错误
- `cargo test` — 320 pass
- `cargo clippy` — 零警告
- CBAC 验证：MCP 读操作返回过滤后的结果
- core.write 验证：block_type 字段已从 payload 中移除
- 事件查询验证：EngineHandle 新增 3 个事件查询方法，通过服务层暴露

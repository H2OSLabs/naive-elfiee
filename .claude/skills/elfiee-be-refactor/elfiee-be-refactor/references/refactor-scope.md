# 重构范围清单

> 基于 `docs/mvp/frame/concepts/migration.md` + 代码扫描生成。
> 本文档是 elfiee-be-refactor skill 的精确参考，列出每个文件的处置方式。

---

## 一、删除清单（DELETE）

以下文件/目录将被完整删除。删除前需确认无其他模块依赖。

### 1.1 Directory Extension（整个目录，~2432 行）

**路径：** `src-tauri/src/extensions/directory/`

| 文件 | 行数 | 包含的 Capability |
|------|------|-------------------|
| `directory_create.rs` | 155 | `directory.create` |
| `directory_delete.rs` | 91 | `directory.delete` |
| `directory_export.rs` | 83 | `directory.export` |
| `directory_import.rs` | 194 | `directory.import` |
| `directory_rename.rs` | 154 | `directory.rename` |
| `directory_rename_with_type_change.rs` | 149 | `directory.rename_with_type_change` |
| `directory_write.rs` | 64 | `directory.write` |
| `fs_scanner.rs` | 345 | —（工具函数） |
| `elf_meta.rs` | 572 | —（.elf 元数据管理） |
| `tests.rs` | 1432 | —（测试） |
| `mod.rs` | 109 | —（模块定义 + Payload） |

**替代方案：** 文件系统操作委托给 AgentContext。

### 1.2 Terminal Extension（整个目录，~2795 行）

**路径：** `src-tauri/src/extensions/terminal/`

| 文件 | 行数 | 包含的功能 |
|------|------|-----------|
| `terminal_init.rs` | 48 | `terminal.init` capability |
| `terminal_execute.rs` | 56 | `terminal.execute` capability |
| `terminal_save.rs` | 54 | `terminal.save` capability |
| `terminal_close.rs` | 48 | `terminal.close` capability |
| `pty.rs` | 244 | PTY 工具函数 |
| `commands.rs` | 583 | Tauri PTY 命令 |
| `state.rs` | 47 | TerminalState 全局状态 |
| `tests.rs` | 701 | 测试 |
| `mod.rs` | 114 | 模块定义 + Payload |

**替代方案：** Session Block 的 `session.append` 记录命令结果，PTY 由 AgentContext 托管。

### 1.3 Agent Extension 中的 MCP 配置代码（~819 行）

| 文件 | 行数 | 删除内容 |
|------|------|---------|
| `extensions/agent/mcp_config.rs` | 559 | 整个文件（`.mcp.json` 读写、端口管理） |
| `extensions/agent/settings_config.rs` | 260 | 整个文件（Agent 配置注入到外部工具） |

### 1.4 Task Extension 中的 Git 集成（~716 行）

| 文件 | 行数 | 删除内容 |
|------|------|---------|
| `extensions/task/task_commit.rs` | 58 | 整个文件（`task.commit` capability） |
| `extensions/task/git.rs` | 310 | 整个文件（Git 操作工具函数） |
| `extensions/task/git_hooks.rs` | 348 | 整个文件（Git hook 注入） |

### 1.5 Commands 层中的 I/O 操作

| 文件 | 行数 | 删除/改造 |
|------|------|---------|
| `commands/file.rs` | 693 | 整个删除（文件操作委托给 AgentContext） |
| `commands/checkout.rs` | 299 | 整个删除（checkout 需基于 .elf/ 目录重写） |

### 1.6 MCP SSE 传输层

| 文件 | 行数 | 删除内容 |
|------|------|---------|
| `mcp/transport.rs` | 520 | 整个文件（per-agent SSE 端口管理） |

### 1.7 集成测试

| 文件 | 处置 |
|------|------|
| `tests/elf_meta_integration.rs` | 删除（依赖 directory extension） |
| `tests/terminal_integration.rs` | 删除（依赖 terminal extension） |

**删除总计：约 8290+ 行**

---

## 二、保留清单（KEEP）

以下代码保持不变或仅做微调。

### 2.1 Engine 核心（~3882 行，保持）

| 文件 | 行数 | 状态 |
|------|------|------|
| `engine/actor.rs` | 1274 | 保持（Actor 模型不变） |
| `engine/state.rs` | 1545 | 改造（适配新 Event mode + editor 存在性检查） |
| `engine/event_store.rs` | 307 | 保持（Event 存储逻辑不变） |
| `engine/manager.rs` | 321 | 改造（移除 Agent MCP 管理职责） |
| `engine/mod.rs` | 9 | 保持 |

### 2.2 Capability 系统（~2076 行，保持）

| 文件 | 行数 | 状态 |
|------|------|------|
| `capabilities/registry.rs` | 744 | 保持 |
| `capabilities/grants.rs` | 395 | 保持 |
| `capabilities/core.rs` | 72 | 保持 |
| `capabilities/builtins/*.rs` | ~840 | 保持（12 个内置 capability） |
| `capabilities/mod.rs` | 8 | 保持 |

### 2.3 数据模型（~665 行，保持 + 微调）

| 文件 | 行数 | 状态 |
|------|------|------|
| `models/block.rs` | 87 | 改造（block_type 6→4，添加 format 字段） |
| `models/event.rs` | 31 | 改造（添加 mode 字段） |
| `models/editor.rs` | 129 | 保持 |
| `models/command.rs` | 32 | 保持 |
| `models/grant.rs` | 34 | 保持 |
| `models/payloads.rs` | 262 | 改造（移除 directory/terminal payload） |
| `models/capability.rs` | 11 | 保持 |
| `models/metadata.rs` | 184 | 保持 |

### 2.4 工具函数

| 文件 | 状态 |
|------|------|
| `utils/time.rs` | 保持 |
| `utils/path_validator.rs` | 保持 |
| `utils/block_type_inference.rs` | 改造（适配 4 种 block_type） |
| `utils/snapshot.rs` | 改造或删除（新 snapshot 机制不同） |

### 2.5 集成测试（保留）

| 文件 | 状态 |
|------|------|
| `tests/commands_block_permissions.rs` | 保持 |
| `tests/relation_integration.rs` | 保持 |
| `tests/snapshot_integration.rs` | 改造（适配新 snapshot） |
| `tests/engine_block_dir_integration.rs` | 改造或删除 |
| `tests/template_integration.rs` | 保持 |

---

## 三、改造清单（MODIFY）

### 3.1 Extension 重组

| 操作 | 源 | 目标 |
|------|-----|------|
| 合并 | `extensions/markdown/` + `extensions/code/` | `extensions/document/`（新） |
| 改造 | `extensions/agent/` | 移除 mcp_config.rs、settings_config.rs；保留 agent_create/enable/disable 的纯事件逻辑 |
| 改造 | `extensions/task/` | 移除 git.rs、git_hooks.rs、task_commit.rs；保留 task_read/task_write |
| 新增 | — | `extensions/session/`（session.append capability） |

### 3.2 Commands 层改造

| 文件 | 改造内容 |
|------|---------|
| `commands/agent.rs` (724 行) | 移除 MCP 服务器启停、.mcp.json 管理、symlink 管理 |
| `commands/task.rs` (381 行) | 移除 Git 集成部分（commit_task、inject_hooks 等） |
| `commands/block.rs` (434 行) | 保持，可能简化 |
| `commands/editor.rs` (659 行) | 保持 |

### 3.3 MCP 服务器改造

| 文件 | 改造内容 |
|------|---------|
| `mcp/server.rs` (1963 行) | 大幅改造：移除 directory/terminal 工具，更新为 document/session 工具 |
| `mcp/mod.rs` (114 行) | 适配新的传输层 |

### 3.4 入口文件

| 文件 | 改造内容 |
|------|---------|
| `lib.rs` | 更新注册的 Tauri 命令、移除 terminal 状态管理 |
| `state.rs` | 移除 TerminalState 引用 |
| `config.rs` | 可能需要适配 .elf/ 目录配置 |

---

## 四、新增清单（ADD）

| 模块 | 路径 | 说明 |
|------|------|------|
| Document Extension | `extensions/document/` | 合并 markdown+code，添加 format 字段，delta 模式 |
| Session Extension | `extensions/session/` | session.append capability，append-only 语义 |
| WebSocket Adapter | `communication/` 或 `ws/` | 单端口 WebSocket 服务器 |
| Message Router | `communication/router.rs` | 统一消息分发 |
| Checkpoint | `capabilities/builtins/checkpoint.rs` | core.checkpoint capability |

---

## 五、依赖清理

### 5.1 可能移除的 crate

| crate | 用途 | 移除条件 |
|-------|------|---------|
| `portable-pty` | PTY 管理 | terminal extension 删除后移除 |
| `base64` | terminal 输出编码 | terminal extension 删除后检查是否有其他使用者 |
| `zip` | .elf ZIP 归档 | 如果迁移到 .elf/ 目录格式 |
| `walkdir` | 文件系统遍历 | directory extension 删除后检查 |
| `ignore` | .gitignore 匹配 | directory extension 删除后检查 |

### 5.2 可能新增的 crate

| crate | 用途 |
|-------|------|
| `tokio-tungstenite` 或类似 | WebSocket 服务器 |
| `similar` 或 `diff-match-patch` | delta 模式的文本差异计算 |

---

## 六、迁移顺序

按 Layer 依赖顺序，每步可独立验证：

```
Step 1: 数据模型改造 (L0)
  → block_type 收束、Event mode 字段
  → 验证：cargo test（models 模块）

Step 2: Event 系统扩展 (L1)
  → mode 处理、snapshots 表
  → 验证：event_store 测试

Step 3: .elf/ 格式迁移 (L2)
  → ZIP → 目录
  → 验证：.elf/ 初始化测试

Step 4: Engine 适配 (L3)
  → StateProjector 适配、Manager 收束
  → 验证：完整 command 流程测试

Step 5: Extension 重组 (L4)
  → 删除 directory/terminal
  → 合并 markdown+code → document
  → 新增 session
  → 验证：extension 单元测试

Step 6: 通讯层替换 (L4)
  → 删除 MCP SSE
  → 新增 WebSocket + Message Router
  → 验证：连接 + 消息路由测试

Step 7: Commands 层清理
  → 移除 file/checkout 命令
  → 简化 agent/task 命令
  → 验证：集成测试
```

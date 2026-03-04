# Changelog: L4 Extension System 重构

## 概述

合并 markdown + code 为统一的 document extension，新建 session extension，更新 task extension 的 contents schema。对齐 `data-model.md §五` 和 `extension-system.md` 的定义。

## Step 1: Document Extension — 合并 markdown + code

### 新建文件
| 文件 | 内容 |
|------|------|
| `extensions/document/mod.rs` | DocumentWritePayload (`content: String`)、DocumentReadPayload、模块导出 |
| `extensions/document/document_write.rs` | `#[capability(id = "document.write", target = "document")]` handler |
| `extensions/document/document_read.rs` | `#[capability(id = "document.read", target = "document")]` handler |
| `extensions/document/tests.rs` | 10 个单元测试（write/read/payload/wrong_type） |

### Handler 逻辑
- `document.write`：从 `block.contents` 读取现有对象，将 `content` 存入 `contents.content`，保留其他字段（format, path, hash 等），使用 `EventMode::Full`
- `document.read`：权限门控，返回空事件列表（读操作无副作用）
- 两个 handler 都包含显式 `block_type != "document"` 检查（defense-in-depth，与 certificator 的 target 检查互补）

### 删除文件
- `extensions/markdown/` 整个目录（mod.rs, markdown_write.rs, tests.rs）
- `extensions/code/` 整个目录（mod.rs, code_write.rs, tests.rs）

### Contents schema 对齐
| 概念文档 (§5.1) | 实现 |
|---|---|
| `format` | `core.create` 时注入到 initial contents ✓ |
| `content?` | `document.write` handler 写入 `contents.content` ✓ |
| `path?`, `hash?`, `size?`, `mime?` | contents 中可选字段，handler 保留不覆盖 ✓ |

## Step 2: Session Extension — 新建

### 新建文件
| 文件 | 内容 |
|------|------|
| `extensions/session/mod.rs` | SessionAppendPayload (`entry_type: String`, `data: Value`)、模块导出 |
| `extensions/session/session_append.rs` | `#[capability(id = "session.append", target = "session")]` handler |
| `extensions/session/tests.rs` | 6 个单元测试（message/command/decision/payload） |

### Handler 逻辑
- 构造 entry 对象：`{ "entry_type": ..., "data": ..., "timestamp": now() }`
- 使用 `Event::new_with_mode(EventMode::Append)` — StateProjector 的 Append 模式会累加到 `contents.entries[]`

### Contents schema 对齐
| 概念文档 (§5.3) | 实现 |
|---|---|
| `entries: [...]` | Append 模式自动累积 ✓ |
| entry_type: command / message / decision | handler 不限制类型值，由调用方决定 ✓ |

## Step 3: Task Extension — contents schema 更新

### 修改 `TaskWritePayload`
```
旧: { content: String }
新: { description?: String, status?: String, assigned_to?: String, template?: String }
```

### Handler 逻辑改为
- 逐字段 merge：只更新 payload 中非 None 的字段
- 校验：至少一个字段非空

### 测试
- 35 个单元测试覆盖 payload 反序列化、write/read/commit 功能、authorization、集成流程

### Contents schema 对齐
| 概念文档 (§5.2) | 实现 |
|---|---|
| `description` | TaskWritePayload.description ✓ |
| `status` | TaskWritePayload.status ✓ |
| `assigned_to` | TaskWritePayload.assigned_to ✓ |
| `template?` | TaskWritePayload.template ✓ |

## Step 4: Capability Registry 更新

- 注册 DocumentWrite, DocumentRead, TaskWrite, TaskRead, TaskCommit, SessionAppend
- 删除 MarkdownWrite, MarkdownRead, CodeWrite, CodeRead 注册
- 最终能力集：9 builtin + 6 extension = 15 个 capability
  - builtin: core.create/delete/write/link/unlink/grant/revoke + editor.create/delete
  - extension: document.write/read + task.write/read/commit + session.append

## Step 5: core.create — format 注入

- 当 `block_type == "document"` 且 payload 提供 `format` 时，注入到 initial contents
- 支持 `contents` 字段的 merge（caller 可在创建时提供初始内容）

## Step 6: MCP Server 更新

### 删除
- `elfiee_markdown_read` / `elfiee_markdown_write` tools
- `elfiee_code_read` / `elfiee_code_write` tools
- `elfiee_block_change_type` tool + `BlockChangeTypeInput`

### 新增
- `elfiee_document_read` — 读 document block 的 content
- `elfiee_document_write` — 写 document block 的 content
- `elfiee_session_append` — 追加 session entry
- `SessionAppendInput` struct

### 修改
- `TaskWriteInput` 字段从 `content` 改为 `description/status/assigned_to/template`
- `format_block_summary()` 更新为 document/task/session 分支
- Resources MIME 映射更新
- Tool 描述更新

## Step 7: lib.rs Specta 类型注册

- 删除 `CodeWritePayload`, `CodeReadPayload`, `MarkdownWritePayload`
- 新增 `DocumentWritePayload`, `DocumentReadPayload`, `SessionAppendPayload`

## Step 8: 全局引用清理

批量替换所有旧 block type 和 capability 引用：

| 范围 | 替换 |
|------|------|
| 测试中 `block_type` | `"markdown"` → `"document"` |
| 测试中 contents 字段 | `"markdown": "..."` → `"content": "..."` |
| 测试中 capability 引用 | `"markdown.write/read"` → `"document.write/read"` |
| 测试中 capability 引用 | `"code.write"` → `"document.write"` |
| 测试中 capability 引用 | `"directory.write"` → `"task.write"` |
| 注释和文档 | 所有旧 capability 名称更新 |
| MCP resource handler | `"markdown"/"code"` match → `"document"` match |
| 事件属性 | `"system/markdown.write"` → `"system/document.write"` |

涉及文件：actor.rs, state.rs, manager.rs, cache_store.rs, create.rs, event.rs, grants.rs, payloads.rs, grant.rs, core.rs, server.rs

## Delta Mode 说明（本轮不实现）

Delta mode 需与未来 CRDT 方案对齐：delta 格式应为 **operation-based**（如 Automerge/Yjs 的 operation log），而非 text unified diff。当前 StateProjector 中 Delta placeholder 保留不动（debug log），document.write 始终使用 `EventMode::Full`。

## 测试结果

- 244 unit tests + 16 integration tests + 1 doc test = **261 全部通过**
- `grep -r '"markdown"' src/` — 仅 `block_type_inference.rs`（映射 "markdown" → "document"，正确）
- `grep -r '"code\.(write|read)"' src/` — 仅 `registry.rs` negative assertion（验证旧 cap 不存在，正确）
- `grep -r '"directory"' src/` — 零残留

## 文件变更汇总

| 操作 | 文件数 | 说明 |
|------|--------|------|
| **新建** | 7 | document/(3) + session/(3) + changelog |
| **删除** | 6 | markdown/(3) + code/(3) |
| **修改** | 17 | task/(3) + registry + create + server + lib + payloads + state + actor + manager + cache_store + event + grants + grant + core + block commands |

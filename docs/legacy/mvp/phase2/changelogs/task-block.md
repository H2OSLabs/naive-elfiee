# 3.5 Task Block 模块 — 开发变更记录

## 概述

实现 Task Block 完整功能：独立 `block_type: "task"` 的 4 个 Capabilities（write / read / commit / archive），Git 工具链（命令执行 + Hook 注入），前端 Task 区域（FilePanel + EditorCanvas），以及 Tauri Command 层的 Split Pattern I/O。

## 设计决策

| 决策 | 结论 | 理由 |
|------|------|------|
| Task Block 类型 | 独立 `block_type: "task"` | 与 markdown/code 平级，有专属 capabilities |
| **Task 内容 = Markdown** | `contents.markdown` 与 markdown block 相同格式 | title/description 是 block 字段（name, metadata.description），不是 contents 字段。task 内容就是 markdown，用 MyST 渲染 |
| **无显式状态字段** | 无状态 UI，event history 推导保留在后端 | Phase 2 不做状态流转前端展示 |
| task.commit 架构 | Split Pattern（capability handler + Tauri command） | capability handler 只生成纯审计事件（保持 event log 纯净），git I/O 在 Tauri command 层执行。同 `directory.export` 模式 |
| Git 分支策略 | `feat/{sanitize(title)}` | 固定命名前缀，简单可预测 |
| Git hooks 注入 | `core.hooksPath` + 链式调用 | 不污染原项目 hooks，保留原有 lint/test 规则。`ELFIEE_TASK_COMMIT=1` 环境变量放行 |
| task.archive 实现 | 向 task 自身 contents 写入 `archived_at` 时间戳 | Phase 2 简化，不创建独立 Archive Block |
| 前端 Task 区域 | Outline 和 Linked Repos 之间的独立子区域 | 与大纲同层，不混入仓库列表 |
| Hook 拦截方式 | 建议性（`--no-verify` 可绕过） | Phase 2 不做强制拦截，保留用户灵活性 |
| 开发工具 | elfiee-ext-gen + TDD | 生成骨架 → guide/validate 流程保证质量 |
| 前端渲染 | MyST（与 markdown block 一致） | task 内容就是 markdown 文件，用 MySTDocument 组件渲染，额外提供 Commit/Archive 工具栏 |

## 与 task-and-cost_v3.md 的差异

| v3 原始设计 | 实际实现 | 差异说明 |
|------------|---------|---------|
| `TaskContents { title, description, status }` | `contents.markdown`（与 markdown block 一致） | title/description 是 block 字段（name, metadata.description），task 内容就是 markdown |
| `TaskStatus` 枚举 (Pending/InProgress/Committed/Archived) | 无状态字段，无状态 UI | Phase 2 不做状态流转前端展示 |
| 前端 TaskBlockEditor（表单式 title+description 输入） | MySTDocument + TaskToolbar（commit/archive） | task 用 MyST 渲染 markdown，额外提供 commit/archive 按钮 |
| task.archive 创建 `.elf/Archives/{date}-{title}.md` | 向自身 contents 写入 `archived_at` | Phase 2 简化，不创建独立 Archive Block |
| `不管理 Hooks`（v3 Section 3.5） | 实现了 git hooks 管理 | task-block.md 计划增加了 hook 管理 |
| `task_rw.rs` 单文件 | `task_write.rs` + `task_read.rs` 独立文件 | 遵循 elfiee-ext-gen 的每 capability 独立文件规范 |

## 变更文件清单

### 后端：Task Extension

#### F16-01: Task 数据结构（Payload 类型）

**文件**: `src-tauri/src/extensions/task/mod.rs`（新建）
- `TaskWritePayload { content: String }` — task.write 的输入（markdown 内容，与 MarkdownWritePayload 同构）
- `TaskReadPayload {}` — task.read 的输入（权限门控，无需数据）
- `TaskCommitPayload { target_path: String }` — task.commit 的输入（外部 git repo 路径）
- `TaskArchivePayload {}` — task.archive 的输入（无需数据）
- 所有 payload 均 `#[derive(Debug, Clone, Serialize, Deserialize, Type)]`

**文件**: `src-tauri/src/extensions/mod.rs`
- 新增 `pub mod task;` 模块导出（由 ext-gen 自动注册）

#### F16-02a: task.write Capability

**文件**: `src-tauri/src/extensions/task/task_write.rs`（新建）
- `#[capability(id = "task.write", target = "task")]`
- 验证 `block.block_type == "task"`
- 反序列化 `TaskWritePayload`，将 `content` 写入 `contents.markdown`（与 markdown.write 相同格式）
- 保留 contents 中其他字段（如 archived_at）
- 调用 `touch()` 更新 `metadata.updated_at`
- 生成包含完整 contents 的 event

#### F16-02b: task.read Capability

**文件**: `src-tauri/src/extensions/task/task_read.rs`（新建）
- `#[capability(id = "task.read", target = "task")]`
- 验证 `block.block_type == "task"`
- 权限门控模式：通过授权检查即返回 `Ok(vec![])`，不产生 event
- 同 `code.read` 模式

#### F16-03: task.commit Capability（Split Pattern: Handler 侧）

**文件**: `src-tauri/src/extensions/task/task_commit.rs`（新建）
- `#[capability(id = "task.commit", target = "task")]`
- 验证 `block.block_type == "task"`
- 反序列化 `TaskCommitPayload`
- 从 `block.children[RELATION_IMPLEMENT]` 获取下游 block IDs
- 检查下游列表非空（无 implement 关系则报错 "No downstream blocks"）
- 生成审计事件，value 包含：
  - `target_path`：外部项目路径
  - `downstream_block_ids`：Vec<String> 下游 block 列表
- **不执行任何 I/O**（纯事件生成，遵循 Split Pattern）

#### F16-04: task.archive Capability

**文件**: `src-tauri/src/extensions/task/task_archive.rs`（新建）
- `#[capability(id = "task.archive", target = "task")]`
- 验证 `block.block_type == "task"`
- 向 contents 写入 `archived_at: chrono::Utc::now().to_rfc3339()`
- 保留 markdown 等现有字段
- 调用 `touch()` 更新 `metadata.updated_at`

#### F16-05: Task Extension 测试

**文件**: `src-tauri/src/extensions/task/tests.rs`（新建）

39 个测试覆盖 4 层：

| 类别 | 测试数 | 覆盖内容 |
|------|--------|---------|
| Payload 反序列化 | 5 | 各 Payload 类型的 JSON 解析正确性 |
| 功能测试 | 12 | write 更新 contents、read 权限门控、commit 下游检测、archive 时间戳 |
| 授权测试 | 12 | owner 通过、non-owner 无权拒绝、grant 后 non-owner 通过（每 capability 3 个） |
| block_type 验证 | 8 | 各 capability 在非 task 类型 block 上被拒绝 |
| 集成工作流 | 2 | 完整 create→write→commit 流程、完整 create→write→archive 流程 |

辅助函数：
- `create_task_block()` — 创建基础 task block
- `create_task_block_with_children()` — 创建带 implement 关系的 task block

### 后端：Git 工具

#### Git 命令执行

**文件**: `src-tauri/src/utils/git.rs`（新建）
- `git_exec(repo_path, args, env)` — 异步执行 git 命令，支持环境变量注入
- `is_git_repo(repo_path)` — 检查 .git 目录是否存在
- `git_commit_flow(repo_path, branch_name, message, files)` — 完整的分支创建/切换 → add → 变更检查 → commit → 返回 hash 的流程
  - 自动检查分支是否存在（存在则 checkout，不存在则 checkout -b）
  - 设置 `ELFIEE_TASK_COMMIT=1` 环境变量让 hook 放行
  - 无变更时返回 `Err("No changes to commit")`
- `sanitize_branch_name(name)` — 清洗分支名
  - 非字母数字字符（除 `-`、`_`、`/`）替换为 `-`
  - 合并连续 `-`
  - 去除尾部 `-`
  - 转小写
  - 中文字符通过 Unicode `is_alphanumeric()` 保留

**测试**（8 个）：

| 测试名 | 验证内容 |
|--------|---------|
| `test_sanitize_branch_name_basic` | 空格替换为 `-` |
| `test_sanitize_branch_name_chinese` | 中文字符保留（Unicode alphanumeric） |
| `test_sanitize_branch_name_special_chars` | 特殊字符清洗 |
| `test_sanitize_branch_name_already_clean` | 已合法名称不变 |
| `test_sanitize_branch_name_with_slash` | `/` 保留 |
| `test_sanitize_branch_name_trailing_dash` | 尾部 `-` 去除 |
| `test_sanitize_branch_name_consecutive_dashes` | 连续 `-` 合并 |
| `test_sanitize_branch_name_empty` | 空字符串处理 |

**异步集成测试**（4 个，使用 `tempfile::TempDir`）：

| 测试名 | 验证内容 |
|--------|---------|
| `test_is_git_repo_nonexistent` | 不存在的路径返回 false |
| `test_git_commit_flow_with_temp_repo` | 完整 commit 流程：init → 初始 commit → 新文件 → git_commit_flow → 验证 hash、分支、commit message |
| `test_git_commit_flow_existing_branch` | 分支已存在时切换并 commit |
| `test_git_commit_flow_no_changes` | 无变更报错 "No changes to commit" |

#### Git Hooks 管理

**文件**: `src-tauri/src/utils/git_hooks.rs`（新建）
- `PRE_COMMIT_HOOK_CONTENT` — 常量，pre-commit hook 脚本内容
  - 链式调用：先执行原项目的 hook（从 `.elf/git/original_hooks_path` 读取路径，或回退到 `.git/hooks/pre-commit`）
  - 检查 `ELFIEE_TASK_COMMIT=1` 环境变量：有则放行，无则拒绝并提示
- `inject_git_hooks(repo_path, elf_hooks_dir)` — 注入 hooks
  1. 保存当前 `core.hooksPath` 到 `{elf_hooks_dir}/original_hooks_path` 文件
  2. 写入 pre-commit hook 脚本（设置可执行权限）
  3. 设置 `git config --local core.hooksPath {elf_hooks_dir}`
- `remove_git_hooks(repo_path, elf_hooks_dir)` — 撤销 hooks
  1. 读取 `original_hooks_path` 文件恢复原始 hooksPath
  2. 若无原始值则 `--unset core.hooksPath`
  3. 清理 hook 文件和 original_hooks_path 文件
- `is_hooks_injected(repo_path, elf_hooks_dir)` — 检查当前 hooksPath 是否指向 elf_hooks_dir

**测试**（5 个，使用 `tempfile::TempDir`）：

| 测试名 | 验证内容 |
|--------|---------|
| `test_inject_and_check_hooks` | 注入后 `core.hooksPath` 指向正确、pre-commit 文件存在 |
| `test_remove_hooks` | 撤销后 `core.hooksPath` 被 unset、文件清理 |
| `test_preserve_original_hooks_path` | 有原始 hooksPath 时注入/撤销保持原始值 |
| `test_block_direct_commit` | hook 拦截直接 commit（无 env var） |
| `test_allow_elfiee_commit` | `ELFIEE_TASK_COMMIT=1` 放行 commit |

#### 模块注册

**文件**: `src-tauri/src/utils/mod.rs`
- 新增 `pub mod git;`
- 新增 `pub mod git_hooks;`

### 后端：Tauri Command（Split Pattern I/O 侧）

#### commit_task 命令

**文件**: `src-tauri/src/commands/task.rs`（新建）
- `TaskCommitResult { commit_hash, branch_name, exported_files }` — 返回类型
- `commit_task(file_id, task_block_id, target_path, editor_id)` — Tauri command
  1. 从 EngineManager 获取 engine handle
  2. 确定 editor_id（传入值或 active editor）
  3. 调用 `task.commit` capability（步骤 1：授权 + 审计事件）
  4. 从 commit event 中提取 `downstream_block_ids`
  5. 获取 task block 的 title（`block.name`）和 description（`metadata.description`）
  6. 遍历下游 blocks，调用 `write_block_snapshot()` 导出快照（步骤 4）
  7. 调用 `git_commit_flow()` 执行 git 操作（步骤 5）
  8. 返回 `TaskCommitResult`

**文件**: `src-tauri/src/commands/mod.rs`
- 新增 `pub mod task;`
- 新增 `pub use task::commit_task;`

### 后端：快照支持

**文件**: `src-tauri/src/utils/snapshot.rs`
- `snapshot_filename()` 添加 `"task"` 分支 → 返回 `"body.md"`
- `extract_content()` 添加 `"task"` 分支 → 读取 `contents.markdown`（与 markdown block 完全一致）

**文件**: `src-tauri/src/engine/actor.rs`
- `write_snapshots()` 的 match 添加 `"task.write" | "task.archive"`
- 快照在 task.write 和 task.archive 时同步更新

#### 快照集成测试

**文件**: `src-tauri/tests/snapshot_integration.rs`
- 新增 `test_task_write_creates_snapshot` — task.write 后 `block-{uuid}/body.md` 存在且内容正确
- 新增 `test_task_write_updates_snapshot` — 二次 task.write 更新快照内容

### 后端：注册点

**文件**: `src-tauri/src/capabilities/registry.rs`
- Task Extension section：注册 4 个 Capability（TaskWriteCapability、TaskReadCapability、TaskCommitCapability、TaskArchiveCapability）
- 修复 ext-gen 自动注册产生的格式问题

**文件**: `src-tauri/src/lib.rs`
- Debug 模式 specta_builder：
  - `collect_commands!` 添加 `commands::task::commit_task`
  - `.typ::<>()` 添加 TaskArchivePayload、TaskCommitPayload、TaskReadPayload、TaskWritePayload、TaskCommitResult
- Release 模式 `generate_handler!` 添加 `commands::task::commit_task`

### 前端：TauriClient

**文件**: `src/lib/tauri-client.ts`
- 新增 import：`TaskCommitResult`
- `BlockOperations.writeBlock()` 扩展：`blockType === 'task'` → `capId = 'task.write'`（task 走 `task.write` capability，payload 为 `{ content }` 与 markdown 一致）
- 新增 `TaskOperations` 类：
  - `archiveTask(fileId, blockId)` — 通过 `execute_command` 调用 task.archive capability
  - `commitTask(fileId, taskBlockId, targetPath)` — 调用 `commands.commitTask` Tauri 命令（Split Pattern I/O）
- `TauriClient` 导出添加 `task: TaskOperations`
- **删除 `writeTask` 方法**：task.write 现在走 `writeBlock` 统一路径

### 前端：App Store

**文件**: `src/lib/app-store.ts`

接口新增：

| 方法 | 签名 | 说明 |
|------|------|------|
| `getTaskBlocks` | `(fileId) => Block[]` | 筛选 `block_type === 'task'` |
| `createTaskBlock` | `(fileId, name) => Promise<void>` | 创建 task block（core.create） |
| `commitTask` | `(fileId, taskBlockId, targetPath) => Promise<TaskCommitResult>` | 提交 task 到 git |
| `archiveTask` | `(fileId, blockId) => Promise<void>` | 归档 task |

- **删除 `writeTask`**：task 内容通过 `updateBlock(fileId, blockId, content, 'task')` 写入，走统一路径
- `commitTask` 成功后 toast 显示分支名和 commit hash 前 7 位

### 前端：FilePanel

**文件**: `src/components/editor/FilePanel.tsx`

**Tasks 子区域**（位于 Outline 和 Linked Repos 之间）：
- 使用 `getTaskBlocks()` 获取 task blocks 列表
- 显示 `block.name`（不再显示 `contents.title`）
- **无状态图标**：移除 `getEvents()`、`getTaskStatus()`、Circle/GitCommitHorizontal/Circle 状态图标
- "+" 按钮创建新 task block（默认名 "New Task"）
- Dropdown 菜单：Archive、Delete

### 前端：VfsTree

**文件**: `src/components/editor/VfsTree.tsx`
- 新增 `CheckSquare` import（lucide-react）
- `InlineEditInput` 组件图标选择：`blockType === 'task'` → `CheckSquare`
- `TreeNode` 组件图标选择：`node.blockType === 'task'` → `CheckSquare`

### 前端：EditorCanvas

**文件**: `src/components/editor/EditorCanvas.tsx`

**Task 渲染改为 MyST + TaskToolbar**（不再使用独立 TaskBlockEditor）：
- Task block 内容通过 `MySTDocument` 渲染（与 markdown 完全一致）
- 新增 `TaskToolbar` 组件：提供 Commit 和 Archive 按钮
- Task block 加入 `lastDocBlockId` 追踪（与 markdown/code 一致）
- `handleSave` 中 `task` 类型使用 `task.write` capability
- 内容加载从 `contents.markdown` 读取（与 markdown 一致）
- **删除 `TaskBlockEditor` 组件**：移除 title/description 表单、状态 Badge、状态推导逻辑
- 清理未使用 import：`Circle`、`CheckSquare` 等

### 前端：测试修复

**文件**: `src/components/editor/FilePanel.test.tsx`
- mockStore 新增缺失的函数 mock：
  - `createTaskBlock`、`archiveTask`
  - `getTaskBlocks`（返回 `[]`）
  - `createEntry`、`renameEntry`、`renameEntryWithTypeChange`、`deleteEntry`
  - `importDirectory`、`checkoutWorkspace`

## 错误修复记录

### 1. `await` not allowed in closure（git.rs 测试）

**问题**: 测试中使用 `.or_else(|_| async { git_exec(...).await }.await)` 编译报错
**原因**: Rust 不允许在闭包中使用 `.await`
**修复**: 改为 `if/else` 模式：`let result = ...; if result.is_err() { ... }`

### 2. 中文分支名测试断言错误

**问题**: `sanitize_branch_name("实现登录功能")` 预期返回 `"--------"` 但实际返回 `"实现登录功能"`
**原因**: 中文字符通过 `char::is_alphanumeric()` 返回 `true`（Unicode 标准），不会被替换
**修复**: 更正断言为 `assert_eq!(sanitize_branch_name("实现登录功能"), "实现登录功能")`

### 3. Task 快照未触发

**问题**: `test_task_write_creates_snapshot` 和 `test_task_write_updates_snapshot` 失败
**原因**: `engine/actor.rs` 的 `write_snapshots()` match 只包含 `"markdown.write" | "code.write" | "directory.write" | "core.create"`，缺少 task 分支
**修复**: 添加 `"task.write" | "task.archive"` 到 match pattern

### 4. FilePanel 前端测试失败

**问题**: 5 个 FilePanel 测试报错 `getTaskBlocks is not a function`
**原因**: 测试的 mockStore 缺少新增的 `getTaskBlocks`、`getEvents` 等函数
**修复**: 向 mockStore 补充所有 FilePanel 使用的新函数

## 测试结果

### Rust 后端

全部 **371 个测试通过**：

| 测试套件 | 数量 | 说明 |
|---------|------|------|
| 单元测试 | 325 | 含 39 个新增 task 测试 + 13 个 git 测试 |
| snapshot_integration | 13 | 含 2 个新增 task 快照测试 |
| relation_integration | 12 | 既有 |
| elf_meta_integration | 7 | 既有 |
| cbac_integration | 5 | 既有 |
| permission_filter_integration | 2 | 既有 |
| terminal_integration | 4 | 既有 |
| doc tests | 3 | 既有 |

### 前端

全部 **89 个测试通过**：

| 测试文件 | 数量 |
|---------|------|
| FilePanel.test.tsx | 6 |
| EditorCanvas.test.tsx | 3 |
| 其他组件测试 | 80 |

### TypeScript 类型检查

`npx tsc --noEmit` 通过，0 errors。

### elfiee-ext-gen validate

```
Validation passed for task
Passed checks:
  - mod.rs exists
  - 5 capability files found
  - TaskWritePayload has correct derives
  - TaskReadPayload has correct derives
  - TaskCommitPayload has correct derives
  - TaskArchivePayload has correct derives
  - Test module found
  - extensions/mod.rs exports module `task`
  - capabilities/registry.rs imports crate::extensions::task::*
  - capabilities/registry.rs registers TaskCommitCapability
  - capabilities/registry.rs registers TaskArchiveCapability
  - capabilities/registry.rs registers TaskWriteCapability
  - capabilities/registry.rs registers TaskReadCapability
  - lib.rs registers Specta types for extensions::task
```

14/14 检查通过。

## 文件变更汇总

### 新建文件

| 文件路径 | 行数 | 说明 |
|---------|------|------|
| `src-tauri/src/extensions/task/mod.rs` | ~30 | Payload 类型定义 |
| `src-tauri/src/extensions/task/task_write.rs` | ~50 | task.write handler |
| `src-tauri/src/extensions/task/task_read.rs` | ~25 | task.read handler |
| `src-tauri/src/extensions/task/task_commit.rs` | ~50 | task.commit handler |
| `src-tauri/src/extensions/task/task_archive.rs` | ~45 | task.archive handler |
| `src-tauri/src/extensions/task/tests.rs` | ~700 | 39 个测试 |
| `src-tauri/src/utils/git.rs` | ~320 | Git 命令执行 + 测试 |
| `src-tauri/src/utils/git_hooks.rs` | ~250 | Git hooks 注入/撤销 + 测试 |
| `src-tauri/src/commands/task.rs` | ~135 | commit_task Tauri command |

### 修改文件

| 文件路径 | 改动说明 |
|---------|---------|
| `src-tauri/src/extensions/mod.rs` | `pub mod task;` |
| `src-tauri/src/capabilities/registry.rs` | 4 个 Capability 注册 |
| `src-tauri/src/utils/mod.rs` | `pub mod git;` + `pub mod git_hooks;` |
| `src-tauri/src/utils/snapshot.rs` | "task" 分支（filename + content） |
| `src-tauri/src/engine/actor.rs` | `write_snapshots()` 添加 "task.write"/"task.archive" |
| `src-tauri/src/commands/mod.rs` | `pub mod task;` + re-export |
| `src-tauri/src/lib.rs` | commit_task 注册 + 5 个 Specta type |
| `src-tauri/tests/snapshot_integration.rs` | 2 个 task 快照测试 |
| `src/lib/tauri-client.ts` | TaskOperations 类 + TauriClient 注册 |
| `src/lib/app-store.ts` | 5 个 task 操作方法 |
| `src/components/editor/FilePanel.tsx` | Tasks 子区域 UI |
| `src/components/editor/VfsTree.tsx` | CheckSquare 图标 |
| `src/components/editor/EditorCanvas.tsx` | TaskBlockEditor 组件 |
| `src/components/editor/FilePanel.test.tsx` | mockStore 补充 |

## 不做的事

| 排除项 | 原因 |
|--------|------|
| TaskStatus 显式状态字段 | Event history 隐式推导，避免回退复杂度 |
| git push | Phase 2 不自动推送，用户手动 |
| 强制 git hook 拦截 | 建议性提示即可 |
| 独立 Archive Block | 归档信息在 task 自身 contents |
| Block 删除业务流程 | 先不考虑 |
| agent.enable/disable 联动 | 属于 3.1 范围 |
| hooks 注入/撤销时机 wiring | `inject_git_hooks` / `remove_git_hooks` 已实现为工具函数，但未 wired 到 directory.import 和 app close handler。Phase 2 最小化，task.commit 通过 `ELFIEE_TASK_COMMIT=1` env var 已可正常工作 |

## 修正记录（2026-01-30）

### 问题 1：Task 内容格式错误

**原始实现**：`TaskWritePayload { title: String, description: String }`，contents 存储为 `{ title, description }`
**修正后**：`TaskWritePayload { content: String }`，contents 存储为 `{ markdown: "..." }`（与 markdown block 一致）

**原因**：用户指出 title 和 description 是 block 字段（`block.name` 和 `block.metadata.description`），不是 contents 字段。task 内容就是 markdown，应与 markdown block 完全一致。

**影响范围**：

| 文件 | 变更 |
|------|------|
| `src-tauri/src/extensions/task/mod.rs` | `TaskWritePayload { content: String }` |
| `src-tauri/src/extensions/task/task_write.rs` | 写入 `contents.markdown` |
| `src-tauri/src/utils/snapshot.rs` | task 快照读取 `contents.markdown` |
| `src-tauri/src/commands/task.rs` | title 从 `block.name`，description 从 `metadata.description` |
| `src-tauri/src/extensions/task/tests.rs` | 全部 39 个测试更新 payload 和断言 |
| `src-tauri/tests/snapshot_integration.rs` | 2 个 task 快照测试更新 |

### 问题 2：前端不应有状态 UI 和表单式编辑器

**原始实现**：`TaskBlockEditor` 组件 — title input + description textarea + 状态 Badge + 状态图标
**修正后**：用 `MySTDocument` 渲染（与 markdown 一致） + `TaskToolbar`（Commit/Archive 按钮）

**原因**：task 内容就是 markdown，应用 MyST 渲染。Phase 2 不做状态流转前端。

**影响范围**：

| 文件 | 变更 |
|------|------|
| `src/components/editor/EditorCanvas.tsx` | 删除 TaskBlockEditor，改用 MySTDocument + TaskToolbar |
| `src/components/editor/FilePanel.tsx` | 移除 getTaskStatus/getEvents/状态图标，显示 block.name |
| `src/lib/tauri-client.ts` | writeBlock 支持 task 类型，删除 writeTask |
| `src/lib/app-store.ts` | 删除 writeTask 方法 |

### 问题 3：.elf/ 目录结构 session 位置错误

**原始实现**：`Agents/session/`（session 在 Agents 下）
**修正后**：`session/`（session 为一级目录）

**原因**：agents/、git/、session/ 均为一级目录。agents/ 下才有 elfiee-client/，再下一级才有 assets/references/ 等 skill 相关目录。

**影响范围**：

| 文件 | 变更 |
|------|------|
| `src-tauri/src/extensions/directory/elf_meta.rs` | `ELF_DIR_PATHS` 修正 + 小写 `agents/` |
| `src-tauri/tests/elf_meta_integration.rs` | 路径断言更新 |

### 修正后测试结果

- **Rust 后端**：370 个测试全部通过
- **前端**：89 个测试全部通过（12 个文件）
- **TypeScript**：`tsc --noEmit` 0 errors

## 修正记录（2026-01-30 第二轮）

基于用户 6 点反馈，对 task-block 模块进行了以下修正：

### 修正 4：移除 archive 功能

**原始实现**：`task.archive` capability + 前后端完整支持
**修正后**：完全移除 archive，Phase 2 不实现

**影响范围**：

| 文件 | 变更 |
|------|------|
| `src-tauri/src/extensions/task/mod.rs` | 删除 `pub mod task_archive;` 导出和 `TaskArchivePayload` |
| `src-tauri/src/capabilities/registry.rs` | 删除 `TaskArchiveCapability` 注册 |
| `src-tauri/src/lib.rs` | 删除 `TaskArchivePayload` specta 类型注册 |
| `src-tauri/src/extensions/task/tests.rs` | 删除 10 个 archive 相关测试 |
| `src/components/editor/FilePanel.tsx` | 删除 Archive 菜单项和 handleArchiveTask |
| `src/components/editor/EditorCanvas.tsx` | 删除 Archive 按钮和 handleArchive |
| `src/lib/app-store.ts` | 删除 `archiveTask` 方法 |
| `src/lib/tauri-client.ts` | 删除 `archiveTask` 方法 |

### 修正 5：Outline 改名为 .elf，移除 + 按钮

**原始实现**：导航栏第一分区为 "Outline"，带 "+" 按钮添加工作目录
**修正后**：改名为 ".elf"，移除 "+" 按钮

**影响范围**：

| 文件 | 变更 |
|------|------|
| `src/components/editor/FilePanel.tsx` | 标题 "Outline" → ".elf"，删除 Plus Button |
| `src/components/editor/FilePanel.test.tsx` | 断言 `.elf` 替代 `Outline` |

### 修正 6：Task block 获得 dir-block 操作能力

**原始实现**：Task dropdown 只有 Archive 和 Delete
**修正后**：Task dropdown 改为 Rename、Export、Delete

**影响范围**：

| 文件 | 变更 |
|------|------|
| `src/components/editor/FilePanel.tsx` | 新增 `handleRenameTask`、`handleExportTask` handler；Dropdown: Rename/Export/Delete |

### 修正 7：task.commit 自动发现项目（移除 target_path）

**原始实现**：`TaskCommitPayload { target_path: String }` 需要用户选择路径
**修正后**：`TaskCommitPayload {}` 空 payload，自动从 implement 下游 block 的父目录发现 linked repo

**核心变更**：
- `task_commit.rs`：payload 为空，无需反序列化，只生成审计事件
- `commands/task.rs`：新增 `find_block_repo_path()` 函数，遍历所有 directory blocks（source="linked"），从 entries 中查找目标 block，返回 `metadata.custom["external_root_path"]`
- 按项目分组 downstream blocks，逐项目验证 `.git` 存在、导出快照、git commit

**影响范围**：

| 文件 | 变更 |
|------|------|
| `src-tauri/src/extensions/task/mod.rs` | `TaskCommitPayload {}` 空 struct |
| `src-tauri/src/extensions/task/task_commit.rs` | 移除 payload 反序列化 |
| `src-tauri/src/commands/task.rs` | 完全重写：自动发现 + 多项目支持 |
| `src/lib/tauri-client.ts` | `commitTask` 不再需要 `targetPath` |
| `src/lib/app-store.ts` | `commitTask` 签名简化 |
| `src/components/editor/EditorCanvas.tsx` | Commit 按钮不再弹出文件夹选择器 |
| `src-tauri/src/extensions/task/tests.rs` | 所有 commit 测试使用空 payload |

### 修正 8：Git hooks 注入 Tauri 命令 + 前端 wiring

**原始实现**：`inject_git_hooks` / `remove_git_hooks` 为工具函数，未暴露给前端
**修正后**：新增两个 Tauri 命令 + 前端 importDirectory 后自动注入

**影响范围**：

| 文件 | 变更 |
|------|------|
| `src-tauri/src/commands/task.rs` | 新增 `inject_hooks_for_repo` 和 `remove_hooks_for_repo` Tauri 命令 |
| `src-tauri/src/lib.rs` | 注册两个新命令（debug + release） |
| `src/lib/tauri-client.ts` | TaskOperations 新增 `injectHooksForRepo` 和 `removeHooksForRepo` |
| `src/lib/app-store.ts` | `importDirectory` 成功后自动调用 `injectHooksForRepo` |

### 修正 9：新增 Links 管理标签页

**新功能**：ContextPanel 右侧面板新增 "Links" 标签页，位于 Collaborators 和 Timeline 之间

**功能**：
- 显示当前 block 的 `implement` 关系列表
- 点击 "+" 从可用 blocks 中选择目标建立 link
- 点击 Unlink 图标解除关系
- 排除 directory blocks（不可作为 link 目标）

**影响范围**：

| 文件 | 变更 |
|------|------|
| `src/components/editor/ContextPanel.tsx` | 新增 `LinksTab` 组件 + "Links" tab trigger/content |
| `src/lib/app-store.ts` | 新增 `linkBlock` 和 `unlinkBlock` actions |

### 第二轮修正后测试结果

- **Rust 后端**：360 个测试全部通过（减少 10 个 archive 测试）
- **前端**：89 个测试全部通过（12 个文件）
- **TypeScript**：`tsc --noEmit` 0 errors

### 更新 "不做的事"

| 排除项 | 状态 |
|--------|------|
| task.archive | ❌ 已从 Phase 2 移除 |
| hooks 注入/撤销时机 wiring | ✅ 已实现：import 时自动注入 |
| Link 管理 UI | ✅ 已实现：ContextPanel Links 标签页 |

## 修正记录（2026-01-30 第三轮）

第 #42-48 号任务从原始 plan 文档重新实现 task 模块，导致第一、二轮修正（修正 1-9）部分被回退。
本轮重新应用了修正 1、2、4、7 并修复了集成测试。修正 3、5、6、8、9 未被回退，经验证仍然正确。

### 重新应用的修正

| 修正 | 内容 | 涉及文件 |
|------|------|----------|
| 修正 1 | `TaskWritePayload { content }` → `contents.markdown` | `mod.rs`, `task_write.rs`, `snapshot.rs` |
| 修正 2 | 前端 task 用 MySTDocument 渲染，无 TaskBlockEditor | `EditorCanvas.tsx`, `tauri-client.ts`, `app-store.ts` |
| 修正 4 | 移除 task.archive（mod.rs、registry、lib.rs、前端） | `mod.rs`, `registry.rs`, `lib.rs`, 前端 4 文件 |
| 修正 7 | `TaskCommitPayload {}` 空 payload + 自动发现 repo | `mod.rs`, `task_commit.rs`, `commands/task.rs`, 前端 3 文件 |

### 额外修复

| 文件 | 变更 |
|------|------|
| `tests/snapshot_integration.rs` | 2 个 task 快照集成测试更新为 `{content: "..."}` payload 格式 |
| `extensions/task/tests.rs` | 全部测试重写：write 用 `{content}`、commit 用 `{}`、删除 archive 测试 |

### 验证已保持正确的修正

| 修正 | 验证方式 |
|------|----------|
| 修正 3 (.elf/ session 目录结构) | `elf_meta.rs` 未被重写，路径正确 |
| 修正 5 (Outline → .elf，移除 +) | `FilePanel.tsx` grep 确认 ".elf" 标题 |
| 修正 6 (Task dropdown: Rename/Export/Delete) | `FilePanel.tsx` grep 确认 handler 存在 |
| 修正 8 (Git hooks Tauri 命令 + 前端 wiring) | `lib.rs` 已注册、`app-store.ts` importDirectory 后调用 |
| 修正 9 (Links 标签页) | `ContextPanel.tsx` LinksTab 组件存在 |

### 第三轮修正后测试结果

- **Rust 后端**：314 个测试全部通过（单元 + 集成 + doc-tests）
- **前端**：89 个测试全部通过（12 个文件）
- **TypeScript**：`tsc --noEmit` 0 errors

### 与 plan 文档的差异说明

原始 plan (`docs/mvp/phase2/plans/task-block.md`) 使用旧设计：

| plan 文档内容 | 实际实现（修正后） | 修正编号 |
|---------------|-------------------|----------|
| `TaskWritePayload { title, description }` | `TaskWritePayload { content }` | 修正 1 |
| `contents: { title, description }` | `contents: { markdown }` | 修正 1 |
| `TaskCommitPayload { target_path }` | `TaskCommitPayload {}` (自动发现) | 修正 7 |
| `task.archive` capability | 已移除 | 修正 4 |
| Outline 标题 + "+" 按钮 | ".elf" 标题，无 "+" 按钮 | 修正 5 |

plan 文档为初始设计记录，不再更新。以 changelog 修正记录为准。

## 代码审查记录（2026-02-01）

### 设计原则合规性审查

基于 `docs/concepts/ARCHITECTURE_OVERVIEW.md` 和 `ENGINE_CONCEPTS.md` 审查全部改动：

| 原则 | 结果 | 说明 |
|------|------|------|
| Block type capability 隔离 | ✅ 通过 | task.write/read/commit 全部校验 `block_type == "task"`，不操作其他类型。hook 使用 `code.write`（code block 用 code capability） |
| 无循环嵌套 | ✅ 通过 | handler 单向：Command → Events。task.commit 只生成审计事件不调用其他 capability |
| 数据流单向性 | ✅ 通过 | UI → Zustand → TauriClient → Tauri Command → Engine → Capability → Events → State → UI |
| Event Sourcing 纯净性 | ✅ 通过 | Split Pattern: I/O 在 Tauri Command 层，Capability handler 只生成纯事件 |
| CBAC 权限模型 | ✅ 通过 | 所有 capability 使用标准 certificator（owner 通过 + grant 通过） |

### 发现的遗留问题

| # | 类型 | 描述 | 影响 |
|---|------|------|------|
| 1 | 已清理 | `task_archive.rs` 文件已删除，`actor.rs` 中 `"task.archive"` 死 match arm 已移除 | — |
| 2 | 类型绕过 | `app-store.ts` linkBlock 使用 `as unknown as JsonValue` 类型断言 | 绕过 TypeScript 类型安全，core.link payload 结构在 bindings 中是 JsonValue，可接受 |
| 3 | 重复解构 | `FilePanel.tsx` 解构 `checkoutWorkspace` 和 `checkoutWorkspace: checkoutWorkspaceAction` | 可能引起混淆，建议统一 |

以上均为代码质量问题，不构成设计原则违规。

### changelog 文档自身的勘误

以下为本 changelog 早期章节中因多轮修正导致的过时描述，以此处为准：

| 章节位置 | 过时内容 | 正确内容 |
|---------|---------|---------|
| F16-01 "TaskCommitPayload" | `{ target_path: String }` | `{}` 空 struct（修正 7 已改） |
| "注册点" 章节 | 声称注册 4 个 Capability（含 TaskArchiveCapability） | 实际注册 3 个（write, read, commit），archive 已移除（修正 4） |
| "lib.rs" 章节 | 列出 TaskArchivePayload specta 注册 | 已移除（修正 4） |

---

## 修正记录（2026-02-01 — Fix 1/2/3: 导出路径 + Commit Protect）

### 背景

task.commit 存在三个运行时问题：
1. 导出文件使用 `block-{uuid}/body.{ext}` 内部格式，未覆盖回原始路径
2. Hooks 文件创建在外部 repo 内（`{repo}/.elf/git/hooks/`），污染外部项目
3. 关闭/崩溃后 `core.hooksPath` 仍生效，无法主动清理

### Fix 1: task.commit 导出到原始文件路径

**文件**: `src-tauri/src/commands/task.rs`

| 变更 | 说明 |
|------|------|
| `find_block_repo_path` 返回值 | `Option<String>` → `Option<(String, String)>`，同时返回 entry_key（原始相对路径） |
| `repo_blocks` 类型 | `HashMap<String, Vec<String>>` → `HashMap<String, Vec<(String, String)>>`，存 `(block_id, entry_key)` |
| 导出循环 | 删掉 `write_block_snapshot()` 调用，内联 checkout 的内容提取模式：`text` → `markdown` → 写到 `{repo_path}/{entry_key}` |
| `git_commit_flow` 调用 | 传入导出的文件路径列表，不再 `git add -A` |
| 删除 import | `use crate::utils::snapshot::write_block_snapshot` |

**设计依据**: 复用 `checkout_workspace`（`commands/checkout.rs`）的内容提取模式。两者都是系统级 Tauri command（非 capability），共享相同的 I/O 模式：读 block.contents → 提取 text/markdown → 写到外部文件系统。

### Fix 2: hooks 文件放到 .elf temp dir

**核心变更**: `elf_hooks_dir` 从 `{外部repo}/.elf/git/hooks` 改为 `{elf_temp_dir}/.elf/git/hooks`。

| 文件 | 变更 |
|------|------|
| `commands/task.rs` | 新增 `get_elf_hooks_dir()` helper，通过 `state.files[file_id].archive.temp_path()` 计算 |
| `commands/task.rs` | `inject_hooks_for_repo` 签名 `(repo_path, elf_hooks_dir)` → `(file_id, repo_path, state)` |
| `commands/task.rs` | `remove_hooks_for_repo` 签名同上 |
| `commands/task.rs` | `commit_task` 中 hooks 路径改用 `get_elf_hooks_dir()` |
| `lib/tauri-client.ts` | `injectHooksForRepo` / `removeHooksForRepo` 参数改为 `(fileId, repoPath)` |
| `lib/app-store.ts` | `importDirectory` 中 hooks 调用改用 `fileId` |

**效果**:
- 外部 repo 不再被 `.elf/` 目录污染
- 关闭 Elfiee / 崩溃 → temp dir 消失 → `core.hooksPath` 指向空路径 → git 跳过 hooks → 外部可自由 commit

### Fix 3: commit protect 开关

**新增 Tauri command**: `is_hooks_active(file_id, repo_path, state) -> bool`

**注册**: `lib.rs` debug + release handler 均注册 `is_hooks_active`

**前端 UI**: `EditorCanvas.tsx` TaskToolbar

| 变更 | 说明 |
|------|------|
| 新增 `ShieldCheck` / `ShieldOff` 图标 | 来自 lucide-react |
| Toggle 按钮 | Commit 按钮旁边，仅在有 linked repo 时显示 |
| 初始状态 | mount 时调用 `isHooksActive` 查询当前状态 |
| Toggle ON | 调用 `injectHooksForRepo` 注入所有 linked repo |
| Toggle OFF | 调用 `removeHooksForRepo` 移除所有 linked repo |

### Fix 0d: core.delete 清理父 block 的 children 引用

**文件**: `src-tauri/src/engine/state.rs`

`core.delete` 原本只清理被删 block 的 children 的 `parents` 反向索引，不清理其他 block 的 `children` map 中对被删 block 的引用（悬空指针）。

新增步骤：通过 `self.parents` 反向索引找到所有引用被删 block 的父 block，从其 `children[RELATION_IMPLEMENT]` 中移除被删 block ID。

新增测试：`test_delete_child_cleans_parent_children_map`

### 测试结果

- 后端：360 tests passed（315 unit + 42 integration + 3 doc）
- 前端：89 tests passed（12 test files）

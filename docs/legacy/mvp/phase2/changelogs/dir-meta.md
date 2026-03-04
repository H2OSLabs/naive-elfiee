# 2.4 .elf/ 元数据管理 — 开发变更记录

## 概述

在 `create_file` 时自动创建 `.elf/` Dir Block，提供系统级目录骨架。新增 `editor_id = "*"` 通配符支持"所有人"授权。

## 变更文件清单

### I10-01a: GrantsTable editor_id 通配符

**文件**: `src-tauri/src/capabilities/grants.rs`
- 重写 `has_grant()` 方法：先精确匹配 `editor_id`，再查 `"*"` 通配符条目
- 新增 `editor_id != "*"` 守卫，避免 `"*"` 查自身的无限递归
- 新增 4 个单元测试：
  - `test_wildcard_editor_grant`：`"*"` grant 匹配任意 editor
  - `test_wildcard_editor_revoke`：revoke `"*"` 后所有 editor 失去权限
  - `test_wildcard_editor_with_exact_grant`：精确 grant 和 wildcard grant 共存
  - `test_wildcard_editor_combined_with_wildcard_block`：双通配符（editor + block）

### I10-01b: .elf/ Dir Block 初始化逻辑

**文件**: `src-tauri/src/extensions/directory/elf_meta.rs`（新建）
- `ELF_DIR_PATHS` 常量：定义 7 个目录路径
- `build_elf_entries()` 纯函数：构造 entries JSON（全部 `type: "directory"`, `source: "outline"`）
- `bootstrap_elf_meta()` 异步函数：通过 3 个 `process_command` 创建 .elf/ Dir Block
  1. `core.create` — 创建 `.elf/` Dir Block
  2. `directory.write` — 写入目录骨架 entries
  3. `core.grant("*", "directory.write", elf_block_id)` — 所有人可写
- 新增 3 个单元测试：
  - `test_build_elf_entries_structure`：验证所有路径和字段
  - `test_build_elf_entries_count`：验证条目数量
  - `test_build_elf_entries_unique_ids`：验证 id 唯一性

**文件**: `src-tauri/src/extensions/directory/mod.rs`
- 新增 `pub mod elf_meta;` 模块导出

### I10-01c: 修改 create_file 调用链

**文件**: `src-tauri/src/commands/file.rs`
- 在 `create_file()` 中 `bootstrap_editors()` 之后新增 `bootstrap_elf_meta()` 调用
- 仅 `create_file` 触发，`open_file` 不触发

### 集成测试

**文件**: `src-tauri/tests/elf_meta_integration.rs`（新建）

7 个集成测试用例：

| 测试名 | 验证内容 |
|--------|---------|
| `test_elf_block_created` | .elf/ block 存在且 name/type/owner 正确 |
| `test_elf_block_entries_structure` | entries 包含全部 7 个目录路径 |
| `test_elf_block_source_outline` | source 为 "outline"（非 linked） |
| `test_elf_block_wildcard_write_permission` | 非 owner 通过 wildcard grant 获得授权 |
| `test_elf_block_wildcard_write_execution` | 非 owner 实际执行 directory.write 成功 |
| `test_elf_block_no_write_without_grant` | 无 wildcard grant 时非 owner 被拒绝 |
| `test_elf_block_metadata` | description 字段正确 |

## 目录骨架结构

```
.elf/
├── agents/
│   └── elfiee-client/
│       ├── scripts/
│       ├── assets/
│       └── references/
├── session/
└── git/
```

**修正**（2026-01-30）：`session/` 从 `Agents/session/` 提升为一级目录。`agents/`、`git/`、`session/` 均为一级目录。`agents/` 下仅有 `elfiee-client/`，其下才有 `scripts/`、`assets/`、`references/` 等 skill 相关目录。这样才能正确支持软链接。

所有 entries 为虚拟目录（`type: "directory"`），不包含 content Block。

## 设计决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 权限方案 | editor_id `"*"` 通配符 | 对称设计，改动最小 |
| entries 内容 | 仅虚拟目录 | .elf/ 是骨架模板，内容由后续模块填充 |
| 触发时机 | 仅 create_file | open_file 已有 events 无需重复 |
| git/ 目录 | 预留 pre-commit/pre-push hook 模板 | 防止绕过 Elfiee 直接提交 |
| 实现方式 | 通过 process_command | 保证 capability 检查、vector clock、快照一致性 |
| source 值 | "outline" | 系统内部创建，非外部导入 |

## 测试结果

全部 **308 个后端测试通过**（267 单元 + 34 集成 + 5 文档 + 2 ignored）。
全部 **89 个前端测试通过**。

## 与 task-and-cost_v3.md 的对照

| v3 编号 | v3 任务 | 对应实现 | 状态 |
|---------|---------|---------|------|
| I10-01 | .elf/ Dir Block 初始化 | I10-01a (grant 通配符) + I10-01b (初始化逻辑) + I10-01c (调用链) | 完成 |

---

## 修正记录（2026-01-31 — Task Block 集成后的 bootstrap 重构）

### 背景

3.5 Task Block 模块需要在 `.elf/git/hooks/` 下存放 pre-commit hook 模板。原 bootstrap 只创建虚拟目录骨架，不含任何 content Block。本次重构将 hook 模板作为 code block 内嵌到 `.elf/` 结构中，同时移除了 wildcard grant（改由 AddCollaboratorDialog 显式授予 write 权限）。

### 变更：bootstrap_elf_meta 从 3 步变为 4 步

**原流程（3 步）**：
1. `core.create` — 创建 .elf/ Dir Block
2. `directory.write` — 写入 7 个虚拟目录 entries
3. `core.grant("*", "directory.write", elf_block_id)` — 所有人可写

**新流程（4 步）**：
1. `core.create` — 创建 .elf/ Dir Block（不变）
2. `core.create` — 创建 pre-commit hook **code block**（`name: "pre-commit"`, `block_type: "code"`）
3. `code.write` — 向 hook code block 写入 `PRE_COMMIT_HOOK_CONTENT` 模板
4. `directory.write` — 写入目录 entries，其中 `git/hooks/pre-commit` 为 file 类型引用 hook block

**移除的步骤**：
- `core.grant("*", "directory.write", elf_block_id)` — wildcard grant 不再使用

### 变更：build_elf_entries 重构

- 原 `build_elf_entries()` 返回固定的 7 个虚拟目录
- 新增 `build_elf_entries_with_hooks(hook_block_id)` 函数：
  - 7 个虚拟目录（不变）
  - 新增 `git/hooks/pre-commit` 条目，entry type 为 `"file"`（directory entries 内的分类标记，非 block_type），`id` 指向 hook code block
  - 该 hook 条目的实际 Block 类型为 `block_type: "code"`，与 directory entries 的 `type: "file"` 是两层概念

### 变更：elf_meta 集成测试

**文件**: `src-tauri/tests/elf_meta_integration.rs`

| 变更 | 说明 |
|------|------|
| `bootstrap_elf_meta()` 辅助函数 | 返回 `(elf_block_id, hook_block_id)` 元组，执行完整 4 步流程 |
| 移除 `test_elf_block_wildcard_write_permission` | wildcard grant 已移除 |
| 移除 `test_elf_block_wildcard_write_execution` | wildcard grant 已移除 |
| 新增 `test_hook_block_has_template_content` | 验证 hook code block 包含 `ELFIEE_TASK_COMMIT` 关键字 |
| 路径常量 | `ELF_DIR_PATHS` 使用小写 + `git/hooks/` 前缀 |

### 变更：AddCollaboratorDialog 默认写权限

**文件**: `src/components/permission/AddCollaboratorDialog.tsx`

添加协作者时不再依赖 wildcard grant，改为在对话框中显式授予 read + write 权限：

- 新增 `getDefaultWritePermission(blockType)` 函数
  - `code` → `code.write`
  - `directory` → `directory.write`
  - `task` → `task.write`
  - 其他 → `markdown.write`
- `handleAddExisting` 和 `handleCreateNew` 中新增 write permission grant 调用
- 同时为 `getDefaultReadPermission` 添加 `task` → `task.read` 映射

### 变更：vfs-tree.ts 尾随斜杠修复

**文件**: `src/utils/vfs-tree.ts`

**Bug**: `buildTreeFromEntries()` 中 entry key 带尾随斜杠（如 `"agents/"`），nodeMap 以原始 key 存储。计算父路径时 `segments.slice(0, -1).join('/')` 得到 `"agents"`（无斜杠），导致 `nodeMap.get("agents")` 查找失败，所有嵌套目录都变成根节点，.elf/ 结构被扁平化。

**修复**: nodeMap key 做 normalize — 去掉尾随斜杠：
```typescript
const normalizedPath = path.endsWith('/') ? path.slice(0, -1) : path
nodeMap.set(normalizedPath, node)
```

### 设计决策变更

| 原决策 | 新决策 | 理由 |
|--------|--------|------|
| wildcard grant 所有人可写 .elf/ | 通过 AddCollaboratorDialog 显式授予 write | 更精细的权限控制，避免默认开放 |
| .elf/ 仅虚拟目录，不含 content Block | git/hooks/pre-commit 为 code block | Task Block 需要 hook 模板，作为 code block 可利用 event sourcing 版本控制 |

### 集成测试结果

修正后 elf_meta 集成测试：5 个通过（原 7 个减去 2 个 wildcard 测试，加 1 个 hook 测试）。

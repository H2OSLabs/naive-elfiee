# 2.3 Relation 系统增强 — 开发变更记录

## 概述

构建纯粹的"逻辑因果图"（Logical Causal Graph），将 `Block.children` 限制为唯一的 `implement` 关系，添加 DAG 环检测和反向索引。

## 变更文件清单

### I2-01: 定义 RELATION_IMPLEMENT 常量

**文件**: `src-tauri/src/models/block.rs`
- 新增 `pub const RELATION_IMPLEMENT: &str = "implement";`
- 为 `Block` struct 添加文档注释，说明 `children` 字段的因果关系语义

**文件**: `src-tauri/src/models/mod.rs`
- 从 `block` 模块导出 `RELATION_IMPLEMENT` 常量

### I2-02: core.link 限制 relation type

**文件**: `src-tauri/src/capabilities/builtins/link.rs`
- 添加 `RELATION_IMPLEMENT` 导入
- 在 handler 开头校验 `payload.relation == RELATION_IMPLEMENT`，非 `implement` 关系返回错误
- 添加重复链接检测，同一对 source→target 不允许重复添加

### I2-03: core.unlink 限制 relation type

**文件**: `src-tauri/src/capabilities/builtins/unlink.rs`
- 添加 `RELATION_IMPLEMENT` 导入
- 在 handler 开头校验 `payload.relation == RELATION_IMPLEMENT`

### I2-04: StateProjector 反向索引

**文件**: `src-tauri/src/engine/state.rs`
- 新增字段 `pub parents: HashMap<String, Vec<String>>`（child_block_id → parent_block_ids）
- `new()` 初始化 `parents` 为空
- `apply_event` 更新：
  - `core.create`: 若初始 children 含 implement targets，构建反向索引
  - `core.link` / `.write` / `.save`: diff 新旧 children，维护 parents 增减
  - `core.unlink`: diff 新旧 children，移除解除链接的 parent 引用
  - `core.delete`: 清理被删除 block 作为 parent 的所有引用，并移除自身的 parents 条目
- 新增方法 `get_parents(&self, block_id: &str) -> Vec<String>`
- 新增方法 `get_children(&self, block_id: &str) -> Vec<String>`
- 新增 5 个单元测试：
  - `test_parents_after_link`
  - `test_parents_after_unlink`
  - `test_parents_multiple_parents`
  - `test_parents_cleanup_on_delete`
  - `test_get_children_convenience`

### I2-05: DAG 环检测

**文件**: `src-tauri/src/engine/actor.rs`
- 新增导入 `HashSet`, `LinkBlockPayload`, `RELATION_IMPLEMENT`
- 新增方法 `check_link_cycle(&self, source_id, target_id) -> Result<(), String>`
  - 自环检测（source == target）
  - DFS 从 target 出发，沿 implement children 遍历，检测是否回到 source
- 在 `process_command` 步骤 3（授权）之后、步骤 4（handler 执行）之前插入环检测

### I2-06: 替换现有测试中的 relation type

**文件**:
- `src-tauri/src/models/payloads.rs`: 测试中 `"references"` / `"depends_on"` → `RELATION_IMPLEMENT`；文档注释更新
- `src-tauri/src/capabilities/registry.rs`: 所有测试中 `"references"` → `"implement"`，unlink 测试中 children 键和断言也替换
- `src-tauri/src/engine/actor.rs`: 所有测试中 `"references"` → `"implement"`

### I2-07: 新增集成测试

**文件**: `src-tauri/tests/relation_integration.rs`（新建）

12 个集成测试用例：

| 测试名 | 验证内容 |
|--------|---------|
| `test_link_implement_success` | 正常 implement 链接成功 |
| `test_link_non_implement_rejected` | 非 implement 关系被拒绝 |
| `test_link_duplicate_prevented` | 同一对 source→target 不重复 |
| `test_link_cycle_direct` | A→B→A 直接环被拒绝 |
| `test_link_cycle_indirect` | A→B→C→A 间接环被拒绝 |
| `test_link_dag_valid` | 菱形 DAG（A→B, A→C, B→D, C→D）合法 |
| `test_link_self_cycle` | A→A 自环被拒绝 |
| `test_unlink_removes_parent` | unlink 后可重新链接 |
| `test_unlink_non_implement_rejected` | unlink 非 implement 被拒绝 |
| `test_parents_query` | 多个上游 block 查询 |
| `test_children_query` | 多个下游 block 查询 |
| `test_delete_cleans_parents` | 删除中间 block 后环检测正确 |

### Dir Block 纯化验证（v3 I2-02）

**文件**: `src-tauri/src/extensions/directory/directory_import.rs`
- 已确认 `directory.import` 仅更新 `contents.entries` (JSON) 索引
- 新建 block 时设置 `children: {}`，不向 `Block.children` 添加任何关系
- 物理结构不污染逻辑因果图，无需代码改动

## 与 task-and-cost_v3.md 的对照

| v3 编号 | v3 任务 | 对应实现 | 状态 |
|---------|---------|---------|------|
| I2-01 | Relation 逻辑收敛（定义常量） | plan I2-01 | 完成 |
| I2-02 | Dir Block 纯化（确认不污染） | 验证通过，无需改动 | 完成 |
| I2-03 | 严格 DAG 环检测 | plan I2-05 (actor.rs) + plan I2-02/I2-03 (handler 限制) | 完成 |
| I2-04 | 反向索引构建 | plan I2-04 | 完成 |
| T-02 | Relation DAG 测试 | plan I2-06 + I2-07 | 完成 |

## 测试结果

全部 **297 个测试通过**：
- 260 个单元测试（含 5 个新增 state 反向索引测试）
- 34 个集成测试（含 12 个新增 relation 测试）
- 3 个文档测试

## 设计决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 关系类型数量 | 仅 `implement` 一种 | 现有多种关系无实际语义区分 |
| 语义方向 | A.children(B) = "A 的改动导致 B 的改动" | 上游定义/决定下游（因→果） |
| 环检测位置 | Actor 层前置检查（process_command 步骤 3.5） | handler 无法访问全局图 |
| 反向索引粒度 | `parents: HashMap<block_id, Vec<parent_id>>` | 只有一种关系，无需区分 type |
| 数据结构不变 | `Block.children` 仍为 `HashMap<String, Vec<String>>` | 运行时限制 key，保持兼容 |

## 构建修复

### 回退 tsconfig.build.json 方案

初始方案：创建 `tsconfig.build.json`（`noUnusedLocals: false`）绕过 `bindings.ts` 中 `TAURI_CHANNEL` 和 `__makeEvents__` 的 unused 警告。

**问题**：全局关闭 `noUnusedLocals` 会掩盖我们自己代码中的 unused variable。

**根因分析**：
- `TAURI_CHANNEL`：tauri-specta 始终 import `Channel` 类型，但当前无 command 使用 Channel 参数
- `__makeEvents__`：tauri-specta 始终生成事件工厂函数，但 specta builder 未注册 `.events()`
- 这两个是 Part 6（实时事件广播）的基础设施，当前 MVP 使用 poll 模式（写操作后主动 `loadBlocks()` 拉取），事件系统尚未实现

**正确方案**：在 specta export 配置中添加 `.header("// @ts-nocheck")`，让 tsc 跳过自动生成的 `bindings.ts`，而非全局降低检查标准。

**变更**：
- `src-tauri/src/lib.rs`：specta `Typescript::default()` 增加 `.header("// @ts-nocheck")`
- `package.json`：build script 回退为 `"tsc && vite build"`
- 删除 `tsconfig.build.json`

### TerminalPanel.tsx 类型修复

**文件**: `src/components/editor/TerminalPanel.tsx`
- `NodeJS.Timeout` → `ReturnType<typeof setInterval>`（Vite 环境无 `@types/node`）

## 不做的事

- 不改 `Block.children` 数据结构
- 不改 `LinkBlockPayload` / `UnlinkBlockPayload` 结构
- 不做变更传播逻辑（仅建图）
- 不做 Dir Block 改动（`directory_import` 已正确隔离）
- 不做 Agent 间 relation（通过 Block 因果链 + CBAC 间接体现）

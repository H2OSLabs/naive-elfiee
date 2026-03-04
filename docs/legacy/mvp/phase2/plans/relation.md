# 2.3 Relation 系统增强 — 开发计划

## 一、目标

构建纯粹的"逻辑因果图"（Logical Causal Graph），支撑以下场景：

- **文件因果链**：Task → Code、PRD → Task → Test
- **变更传播**：A 修改了 Task B，因为 B→C 的 implement 关系，C 需要联动修改
- **Agent 间接协作**：Agent A 负责 Task，Agent B 负责 Code，通过 Task→Code 链推导出协作关系

## 二、设计决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 关系类型数量 | **仅 `implement` 一种** | 现有 `"references"` 等无实际语义区分，统一简化 |
| 语义方向 | A.children(B) = "A 的改动导致 B 的改动" | 上游定义/决定下游（因→果） |
| 环检测位置 | **Actor 层前置检查** | handler 无法访问全局图，actor 拥有全量状态 |
| 反向索引粒度 | `parents: HashMap<block_id, Vec<parent_id>>` | 只有一种关系，不需要区分 relation type |
| Agent 关系表达 | **不进 relation graph** | Agent 间关系通过 Block 因果链 + CBAC 权限间接体现 |
| Dir Block 物理包含 | **不进 relation graph** | 存 `contents.entries`，已实现，无需改动 |

## 三、现状分析

| 组件 | 现状 | 需要改动 |
|------|------|---------|
| `Block.children` | `HashMap<String, Vec<String>>`，relation key 是任意字符串 | 限制为 `"implement"` |
| `core.link` handler | 接受任意 relation string，无校验，允许重复和环 | 限制 relation + 环检测 |
| `core.unlink` handler | 接受任意 relation string，幂等 | 限制 relation |
| StateProjector | 无反向索引，查询入边需 O(n) 扫描 | 新增 `parents` |
| `directory_import.rs` | 已不碰 `Block.children` | 无需改动 |
| 测试 | 所有测试用 `"references"`，无 DAG 测试 | 全部替换为 `"implement"` |

## 四、任务分解

### I2-01：定义 RELATION_IMPLEMENT 常量（1h）

**文件**：`src-tauri/src/models/block.rs`

**内容**：
- 定义 `pub const RELATION_IMPLEMENT: &str = "implement";`
- 在 `Block` 文档注释中说明 `children` 的语义

**不做**：不改 `children` 的数据结构（仍然是 `HashMap<String, Vec<String>>`）

### I2-02：core.link 限制 relation type（1h）

**文件**：`src-tauri/src/capabilities/builtins/link.rs`

**内容**：
- 在 handler 开头校验 `payload.relation == RELATION_IMPLEMENT`
- 非 `"implement"` 关系直接返回错误
- 同时防止重复链接（同一对 source→target 不重复添加）

### I2-03：core.unlink 限制 relation type（0.5h）

**文件**：`src-tauri/src/capabilities/builtins/unlink.rs`

**内容**：
- 同样校验 `payload.relation == RELATION_IMPLEMENT`

### I2-04：StateProjector 反向索引（2h）

**文件**：`src-tauri/src/engine/state.rs`

**内容**：
- 新增字段：`pub parents: HashMap<String, Vec<String>>`
  - key = 子 block_id，value = 父 block_id 列表
- `apply_event` 中：
  - `core.link` 事件：diff 新旧 children，新增的 target 添加到 `parents[target].push(source)`
  - `core.unlink` 事件：diff 新旧 children，移除的 target 从 `parents[target]` 删除
  - `core.delete` 事件：清理被删除 block 在 `parents` 中的所有引用
- 新增查询方法：
  - `pub fn get_parents(&self, block_id: &str) -> Vec<String>` — 返回所有上游 block
  - `pub fn get_children(&self, block_id: &str) -> Vec<String>` — 便捷方法，从 block.children 提取

### I2-05：DAG 环检测（3h）

**文件**：`src-tauri/src/engine/actor.rs`

**内容**：
- 在 `process_command` 中，当 `cmd.cap_id == "core.link"` 时，执行前置检查
- 检查位置：步骤 3（授权检查）之后、步骤 4（handler 执行）之前
- 算法：从 `target_id` 出发，沿 `implement` 关系做 DFS，检查是否能回到 `source_block_id`
- 如果发现环，返回错误：`"Cycle detected: linking {source} → {target} would create a cycle"`

**实现伪代码**：
```rust
fn check_link_cycle(&self, source_id: &str, target_id: &str) -> Result<(), String> {
    // 从 target 出发，DFS 遍历所有 implement 下游
    // 如果遇到 source，说明 source → target → ... → source 形成环
    let mut visited = HashSet::new();
    let mut stack = vec![target_id.to_string()];

    while let Some(current) = stack.pop() {
        if current == source_id {
            return Err(format!(
                "Cycle detected: linking {} → {} would create a cycle",
                source_id, target_id
            ));
        }
        if visited.insert(current.clone()) {
            if let Some(block) = self.state.get_block(&current) {
                if let Some(targets) = block.children.get("implement") {
                    stack.extend(targets.iter().cloned());
                }
            }
        }
    }
    Ok(())
}
```

### I2-06：替换现有测试中的 relation type（1h）

**文件**：多个测试文件

**内容**：
- `src-tauri/src/models/payloads.rs` — 测试中 `"references"` → `"implement"`
- `src-tauri/src/engine/actor.rs` — 测试中 `"references"` → `"implement"`
- `src-tauri/src/capabilities/registry.rs` — 测试中 `"references"` → `"implement"`
- 使用 `RELATION_IMPLEMENT` 常量替换硬编码字符串

### I2-07：新增测试（2h）

**文件**：`src-tauri/tests/relation_integration.rs`（新建）

**测试用例**：

| 测试名 | 验证内容 |
|--------|---------|
| `test_link_implement_success` | 正常 implement 链接成功 |
| `test_link_non_implement_rejected` | 非 implement 关系被拒绝 |
| `test_link_duplicate_prevented` | 同一对 source→target 不重复 |
| `test_link_cycle_direct` | A→B→A 直接环被拒绝 |
| `test_link_cycle_indirect` | A→B→C→A 间接环被拒绝 |
| `test_link_dag_valid` | A→B, A→C, B→D, C→D（菱形 DAG）合法 |
| `test_link_self_cycle` | A→A 自环被拒绝 |
| `test_unlink_removes_parent` | unlink 后反向索引正确更新 |
| `test_parents_query` | 查询 block 的所有上游 |
| `test_children_query` | 查询 block 的所有下游 |
| `test_delete_cleans_parents` | 删除 block 后清理反向索引 |

## 五、执行顺序

```
I2-01 定义常量
  ↓
I2-04 反向索引（state.rs）
  ↓
I2-02 link 限制 + I2-03 unlink 限制（并行）
  ↓
I2-05 DAG 环检测（依赖反向索引做验证）
  ↓
I2-06 替换现有测试
  ↓
I2-07 新增测试
```

## 六、不做的事

- **不改 `Block.children` 的数据结构** — 仍然是 `HashMap<String, Vec<String>>`，只是运行时限制 key 为 `"implement"`
- **不改 `LinkBlockPayload` 结构** — 保留 `relation` 字段，在 handler 中校验值
- **不做变更传播逻辑** — 2.3 只建图，传播是 Task 模块或未来功能
- **不做 Dir Block 改动** — `directory_import` 已经正确隔离
- **不做 Agent 间 relation** — Agent 协作通过 Block 因果链 + CBAC 间接体现

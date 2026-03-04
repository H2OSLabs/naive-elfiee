# 删除机制实现方案（正确版）

**日期**: 2025-12-29
**核心理念**: Event Sourcing 纯粹性 - Event 软删除 + StateProjector 硬删除

---

## 1. 架构理解修正

### 1.1 之前的误解

❌ **错误理解**: StateProjector 应该维护 `deleted_blocks` 字段来实现软删除
- 违反了 Event Sourcing 原则
- 在投影层混入历史状态
- 增加内存占用和复杂度

### 1.2 正确的理解

✅ **正确模式**: 分层职责

```
┌────────────────────────────────────────────┐
│ Event Store (唯一真相源 - 不可变日志)       │
│                                            │
│ [core.create] [core.write] [core.delete]  │
│      ↑            ↑             ↑          │
│      └────────────┴─────────────┘          │
│      完整历史永久保存（软删除在这里）        │
└────────────────────────────────────────────┘
                    ↓ replay events
┌────────────────────────────────────────────┐
│ StateProjector (当前活跃状态投影)           │
│                                            │
│ blocks: HashMap<String, Block>             │
│   - 只包含活跃 Block                        │
│   - delete 事件 → remove() (硬删除)        │
│   - 可完全重建                             │
└────────────────────────────────────────────┘
                    ↓ query
┌────────────────────────────────────────────┐
│ Application (查询活跃数据)                  │
└────────────────────────────────────────────┘

需要历史/恢复时：
Event Store → 临时投影 → 重建历史状态
```

---

## 2. 当前实现分析

### 2.1 StateProjector 实现 ✅

**文件**: `src-tauri/src/engine/state.rs:162-164`

```rust
"core.delete" => {
    self.blocks.remove(&event.entity);  // ✅ 正确！
}
```

**分析**:
- ✅ 从投影中移除 Block（硬删除）
- ✅ Event Store 仍保留完整历史
- ✅ 投影职责清晰：只反映当前活跃状态
- ✅ 符合 Event Sourcing 原则

**结论**: **无需修改**

### 2.2 delete.rs 实现 ⚠️

**文件**: `src-tauri/src/capabilities/builtins/delete.rs`

```rust
/// Soft-deletes a block by marking it as deleted.  // ❌ 误导性注释
#[capability(id = "core.delete", target = "core/*")]
fn handle_delete(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for core.delete")?;

    let event = create_event(
        block.block_id.clone(),
        "core.delete",
        serde_json::json!({ "deleted": true }),  // ❌ 冗余 payload
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

**问题识别**:
1. 注释说"Soft-deletes by marking"，实际 StateProjector 是硬删除
2. Payload `{"deleted": true}` 完全未被使用（StateProjector 直接 remove）
3. 代码与实际行为不一致，造成混淆

**结论**: **需要清理**

### 2.3 directory.delete 实现 ✅

**文件**: `src-tauri/src/extensions/directory/directory_delete.rs`

```rust
// ✅ 纯引用语义 - 只删除索引引用
let event = create_event(
    block.block_id.clone(),
    "directory.write",
    json!({ "contents": { "entries": new_entries } }),
    &cmd.editor_id,
    1,
);
Ok(vec![event])  // 不生成 core.delete 事件
```

**分析**:
- ✅ 实现正确
- ✅ 不生成 `core.delete` 事件
- ✅ Block 生命周期独立于 Directory

**结论**: **无需修改**

---

## 3. 修改方案

### 3.1 唯一需要修改的：delete.rs

**修改文件**: `src-tauri/src/capabilities/builtins/delete.rs`

**修改前**:
```rust
/// Handler for core.delete capability.
///
/// Soft-deletes a block by marking it as deleted.
#[capability(id = "core.delete", target = "core/*")]
fn handle_delete(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for core.delete")?;
    // Create event marking block as deleted
    let event = create_event(
        block.block_id.clone(),
        "core.delete", // cap_id
        serde_json::json!({ "deleted": true }),
        &cmd.editor_id,
        1, // Placeholder - engine actor updates with correct count (actor.rs:227)
    );

    Ok(vec![event])
}
```

**修改后**:
```rust
/// Handler for core.delete capability.
///
/// Deletes a block from the active state projection.
///
/// # Event Sourcing Semantics
/// - **Event Store**: Preserves complete deletion history (immutable log)
/// - **StateProjector**: Removes block from active state (hard delete from memory)
/// - **Recovery**: Can reconstruct deleted blocks by replaying events from Event Store
///
/// # Design
/// This follows pure Event Sourcing principles:
/// - Event = single source of truth (soft delete at Event layer)
/// - StateProjector = current active state only (hard delete at projection layer)
/// - Historical state reconstructed on-demand via event replay
#[capability(id = "core.delete", target = "core/*")]
fn handle_delete(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for core.delete")?;

    // Generate deletion event
    // Payload is empty - deletion is signaled by the event type itself
    let event = create_event(
        block.block_id.clone(),
        "core.delete",
        serde_json::json!({}),  // Empty payload
        &cmd.editor_id,
        1, // Placeholder - updated by engine actor (actor.rs:329)
    );

    Ok(vec![event])
}
```

**关键变更**:
1. ✅ 移除冗余的 `{"deleted": true}` payload
2. ✅ 更新注释准确描述实际行为
3. ✅ 说明 Event Sourcing 语义

---

## 4. 设计验证

### 4.1 Event Sourcing 纯粹性检查

| 层次 | 职责 | 删除语义 | 状态 |
|------|------|---------|------|
| **Event Store** | 唯一真相源 | 软删除（永久保存） | ✅ 正确 |
| **StateProjector** | 当前状态投影 | 硬删除（从内存移除） | ✅ 正确 |
| **Application** | 查询活跃数据 | 查询时看不到已删除 | ✅ 正确 |

### 4.2 功能验证

**场景 1: 正常删除**
```
1. Alice 创建 Block A
2. Alice 删除 Block A
   → Event Store: [create, delete] ✅
   → StateProjector: blocks.remove("A") ✅
3. 查询 Block A
   → 返回 None ✅
```

**场景 2: 恢复已删除 Block（未来功能）**
```
1. 从 Event Store 读取 Block A 的所有事件
2. 临时重放事件到删除前的状态
3. 返回重建的 Block
   → 无需在 StateProjector 中保留已删除 Block ✅
```

**场景 3: 审计历史**
```
1. 查询 Event Store 中的 delete 事件
2. 分析删除时间、操作者等信息
   → Event 完整保存历史 ✅
```

### 4.3 与其他模块的一致性

| 模块 | 实现 | 与删除语义的一致性 |
|------|------|------------------|
| **directory.delete** | 纯引用语义，不删除 Block | ✅ 一致 |
| **core.delete** | Event 软删除 + 投影硬删除 | ✅ 一致 |
| **Event Store** | 不可变日志 | ✅ 一致 |
| **StateProjector** | 可重建投影 | ✅ 一致 |

---

## 5. 未来扩展：恢复功能

### 5.1 实现思路

```rust
/// 从 Event Store 恢复已删除的 Block
pub async fn recover_deleted_block(
    event_pool: &SqlitePool,
    block_id: &str,
) -> Result<Option<Block>, String> {
    // 1. 从 Event Store 读取该 Block 的所有事件
    let events = EventStore::get_events_for_entity(event_pool, block_id)
        .await
        .map_err(|e| format!("Failed to get events: {}", e))?;

    // 2. 创建临时投影
    let mut temp_projector = StateProjector::new();

    // 3. 重放事件直到遇到 delete 事件
    let mut last_state = None;
    for event in events {
        if event.attribute.ends_with("/core.delete") {
            // 找到 delete 事件，停止重放
            break;
        }
        temp_projector.apply_event(&event);
        last_state = temp_projector.blocks.get(block_id).cloned();
    }

    // 4. 返回被删除前的最后状态
    Ok(last_state)
}
```

### 5.2 Undelete Capability（未来）

```rust
/// Capability: core.undelete
#[capability(id = "core.undelete", target = "core/*")]
fn handle_undelete(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>> {
    // 注意：此时 block 参数为 None（因为已删除）
    // 需要从 Event Store 恢复后再执行

    let event = create_event(
        cmd.block_id.clone(),
        "core.undelete",
        json!({}),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}

// StateProjector 处理
"core.undelete" => {
    // 从 Event Store 重建该 Block 的最后状态
    // 然后 insert 回 self.blocks
}
```

---

## 6. 测试影响分析

### 6.1 需要修改的测试

**无！** 当前所有测试已经按照正确的语义编写：

```rust
#[test]
fn test_state_projector_delete_block() {
    // ... 创建 Block
    // 应用 delete 事件
    state.apply_event(&delete_event);

    // 验证 Block 被移除
    assert!(state.get_block("block1").is_none());  // ✅ 已经在测试硬删除
}
```

### 6.2 测试验证清单

- ✅ `state.rs::test_state_projector_delete_block` - 验证硬删除
- ✅ `directory::tests::test_delete_basic` - 验证引用语义
- ✅ `actor::tests` - 所有测试无需修改
- ✅ 集成测试 - 无需修改

---

## 7. 实施步骤

### Step 1: 清理 delete.rs ✅
1. 移除 `{"deleted": true}` payload
2. 更新注释说明 Event Sourcing 语义
3. 运行测试验证

### Step 2: 更新文档 ✅
1. 更新 `CODE_REVIEW_DATASTRUCT.md`
2. 说明当前实现是**正确的**
3. 解释 Event 软删除 + 投影硬删除的设计

### Step 3: 验证 ✅
1. 运行所有后端测试
2. 确认 192 个测试通过

---

## 8. 设计总结

### 8.1 核心原则

✅ **Event Sourcing 纯粹性**
- Event Store = 唯一真相源（软删除）
- StateProjector = 可重建投影（硬删除）
- 历史恢复通过 Event 重放实现

### 8.2 优势

| 优势 | 说明 |
|------|------|
| **简单性** | StateProjector 保持简洁 |
| **内存效率** | 不保留已删除 Block |
| **职责清晰** | Event ≠ State |
| **可恢复性** | Event Store 完整历史 |
| **符合标准** | 标准 Event Sourcing 模式 |

### 8.3 与设计文档的一致性

| 设计要求 | 当前实现 | 状态 |
|----------|---------|------|
| Event 不可变 | ✅ Event Store 完整保存 | ✅ 符合 |
| State 可重建 | ✅ StateProjector 可重放 | ✅ 符合 |
| 引用语义 | ✅ directory.delete 纯引用 | ✅ 符合 |
| 扁平存储 | ✅ HashMap 扁平结构 | ✅ 符合 |

---

## 9. 常见问题

### Q1: 为什么不在 StateProjector 中保留已删除 Block？

**A**: 违反 Event Sourcing 原则。StateProjector 是投影，应该只反映**当前活跃状态**。历史状态应该通过 Event 重放获取。

### Q2: 如果需要查询已删除的 Block 怎么办？

**A**: 从 Event Store 临时重放事件重建。这是标准的 Event Sourcing 模式。

### Q3: 性能会不会有问题？

**A**:
- 正常查询：✅ O(1) HashMap 查询
- 恢复已删除：⚠️ O(n) Event 重放（但这是低频操作）

### Q4: directory.delete 为什么不删除 Block？

**A**: 纯引用语义。Directory.entries 是索引，不是容器。Block 生命周期独立。

---

## 10. 结论

**当前实现已经是正确的** ✅

唯一需要的修改：
- 清理 `delete.rs` 中的冗余代码和误导性注释

核心理念：
- **Event Store**: 软删除（永久保存历史）
- **StateProjector**: 硬删除（只保留活跃状态）
- **Recovery**: 通过 Event 重放实现

这是**标准的 Event Sourcing 模式**，符合设计哲学。

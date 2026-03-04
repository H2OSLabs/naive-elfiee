# 代码审查报告：数据结构设计与实现一致性检查

**审查日期**: 2025-12-29
**审查范围**: feat/datastruc 分支
**审查目标**: 检查当前实现与 CHANGELOG-DATASTRUCT.md 设计哲学的一致性
**修复日期**: 2025-12-29
**修复状态**: ✅ P0 问题已全部修复

## 执行摘要

本次审查发现了 **4 个关键问题**，全部与删除操作和引用语义相关。主要矛盾集中在：

1. ~~**硬删除 vs 软删除**~~：经过讨论，确认当前硬删除实现是正确的（Event 已记录历史）✅ 已确认
2. ~~**引用语义 vs 所有权语义**~~：已重构为纯引用语义 ✅ 已修复
3. ~~**权限检查缺失**~~：重构后不再生成 core.delete 事件，问题已解决 ✅ 已修复

---

## 问题 1: StateProjector 硬删除违反软删除设计 ⚠️ CRITICAL

### 设计要求 (CHANGELOG-DATASTRUCT.md 第2.3节)
> **删除策略**: 软删除（标记 `deleted=true`）+ 定期垃圾回收
> **理由**: Event Sourcing 的不可变性要求、多路径引用的安全性、可审计性

### 当前实现

**文件**: `src-tauri/src/engine/state.rs:162-164`

```rust
"core.delete" => {
    self.blocks.remove(&event.entity);  // ❌ 硬删除
}
```

**文件**: `src-tauri/src/capabilities/builtins/delete.rs:12-15`

```rust
let event = create_event(
    block.block_id.clone(),
    "core.delete",
    serde_json::json!({ "deleted": true }),  // ✅ 发出软删除标志
    &cmd.editor_id,
    1,
);
```

### 矛盾分析

1. **Capability handler** 发出的事件包含 `{"deleted": true}` 字段，意图是软删除
2. **StateProjector** 直接从 `HashMap<String, Block>` 中 `remove()`，完全忽略了事件中的 `deleted` 字段
3. **Block 数据结构** (`src-tauri/src/models/block.rs`) 没有 `deleted` 字段
4. **BlockMetadata** (`src-tauri/src/models/metadata.rs`) 也没有 `deleted` 字段

**结果**: Event 中的软删除标志被完全丢弃，最终执行的是硬删除。

### 影响范围

- ❌ 违反 Event Sourcing 不可变性原则
- ❌ 无法通过事件重放恢复已删除的 Block
- ❌ 多路径引用场景下，一处删除会影响所有引用点（悬空引用）
- ❌ 无法实现"回收站"或"撤销删除"功能
- ❌ 缺乏删除审计追踪

### 建议修复方案

#### 方案 A: 完整实现软删除 (推荐)

1. **修改 Block 结构**:
```rust
// src-tauri/src/models/block.rs
pub struct Block {
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: serde_json::Value,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,
    pub metadata: BlockMetadata,
    pub deleted: bool,  // 新增字段
}
```

2. **修改 StateProjector 处理**:
```rust
// src-tauri/src/engine/state.rs
"core.delete" => {
    if let Some(block) = self.blocks.get_mut(&event.entity) {
        if let Some(deleted) = event.value.get("deleted").and_then(|v| v.as_bool()) {
            block.deleted = deleted;  // 软删除标记
        }
    }
}
```

3. **实现 GC 机制** (可选，未来阶段):
   - 定期扫描 `self.blocks`
   - 检查 `deleted=true` 且无其他 Block 引用的 Block
   - 使用可达性分析（类似 Git GC）

#### 方案 B: 明确化硬删除语义 (备选)

如果确认需要硬删除，应当：
1. 移除 `delete.rs` 中的 `{"deleted": true}` 字段
2. 在文档中明确说明为何选择硬删除（需要充分理由）
3. 解决多路径引用的悬空引用问题

---

## 问题 2: Directory.delete 违反引用语义设计 ⚠️ CRITICAL

### 设计要求 (CHANGELOG-DATASTRUCT.md 第3节)

> **Directory.entries**: 系统管理的文件系统索引，**纯引用语义**
> **Block.children**: 用户管理的知识图谱，**用户语义**
> **删除行为**: Directory.delete 应仅删除索引条目，不应删除 Block 本身

**设计哲学引用**:
> 采用 Unix inode 系统的"引用语义"：Directory.entries 是引用，不是容器。删除目录条目不应删除 Block。

### 当前实现

**文件**: `src-tauri/src/extensions/directory/directory_delete.rs:59-99`

```rust
if entry["type"] == "directory" {
    // Recursively delete all children
    let children: Vec<_> = entries
        .iter()
        .filter(|(path, _)| {
            *path == &payload.path || path.starts_with(&format!("{}/", payload.path))
        })
        .collect();

    for (_, child_entry) in children {
        if child_entry["type"] == "file" {
            // ❌ 删除关联的 Content Block
            let child_id = child_entry["id"]
                .as_str()
                .ok_or("Missing block ID in entry")?;

            events.push(create_event(
                child_id.to_string(),
                "core.delete",  // ❌ 级联删除 Block
                json!({}),
                &cmd.editor_id,
                1,
            ));
        }
    }
} else if entry["type"] == "file" {
    // ❌ 删除 Content Block
    let file_id = entry["id"].as_str().ok_or("Missing block ID in entry")?;

    events.push(create_event(
        file_id.to_string(),
        "core.delete",  // ❌ 直接删除 Block
        json!({}),
        &cmd.editor_id,
        1,
    ));
}
```

### 矛盾分析

| 维度 | 设计要求 | 当前实现 | 冲突 |
|------|----------|----------|------|
| **语义模型** | Reference Semantics (引用) | Ownership Semantics (所有权) | ✅ 冲突 |
| **删除行为** | 仅删除 Directory.entries 中的条目 | 级联删除关联的 Block | ✅ 冲突 |
| **类比系统** | Unix `rm` (仅删除 dentry，inode refcount-1) | Windows 文件夹删除 (级联删除内容) | ✅ 冲突 |
| **数据独立性** | Block 可被多个 Directory 引用 | 删除一个 Directory 会破坏其他引用 | ✅ 冲突 |

### 场景示例：多路径引用冲突

**Story 1** (来自用户原始提问):
> User A 创建 Directory Block，授权 B 权限，B 在其中创建 Block2，A 删除 Directory Block —— Block2 应该怎么处理？

**设计预期**:
- Block2 的 `owner` 是 B
- A 删除 Directory.entries 中的引用
- Block2 本身不受影响（B 仍然可以通过其他方式访问）

**当前实现**:
- A 删除 Directory 时，触发 `core.delete` 删除 Block2
- 但 A 对 Block2 **没有权限**（owner 是 B）
- 违反 CBAC 权限模型

### 影响范围

- ❌ 违反引用语义设计哲学
- ❌ 无法支持同一 Block 在多个 Directory 中引用
- ❌ 权限模型冲突（见问题 3）
- ❌ 无法实现 Unix-like symlink/hardlink 语义
- ❌ 与 CHANGELOG-DIRECTORY.md 中"引用语义最纯粹"的讨论结论不符

### 建议修复方案

#### 推荐方案：纯引用语义

**修改 directory_delete.rs**:

```rust
#[capability(id = "directory.delete", target = "directory")]
fn handle_delete(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for directory.delete")?;

    // ... 前置验证代码 ...

    let entry = entries
        .get(&payload.path)
        .ok_or(format!("Path not found: {}", payload.path))?;

    // ✅ 仅删除 Directory.entries 中的条目，不删除 Block
    let mut new_entries = entries.clone();
    let paths_to_remove: Vec<_> = new_entries
        .keys()
        .filter(|k| *k == &payload.path || k.starts_with(&format!("{}/", payload.path)))
        .cloned()
        .collect();

    for path in paths_to_remove {
        new_entries.remove(&path);
    }

    // ✅ 只发出 directory.write 事件，更新 entries
    let event = create_event(
        block.block_id.clone(),
        "directory.write",
        json!({ "contents": { "entries": new_entries } }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

**关键变更**:
- 移除所有 `core.delete` 事件的生成
- 只更新 Directory.entries 映射表
- Block 的生命周期由 GC 管理（通过可达性分析）

---

## 问题 3: Directory.delete 权限检查缺失 ⚠️ SECURITY

### 设计要求

**CBAC 模型** (CLAUDE.md):
> 1. Block owner 对自己的 Block 拥有所有权限
> 2. 非 owner 需要显式 Grant
> 3. 每个操作必须通过 certificator 检查

### 当前实现问题

**文件**: `src-tauri/src/extensions/directory/directory_delete.rs:75-81`

```rust
events.push(create_event(
    child_id.to_string(),
    "core.delete",  // ❌ 直接对 child_id 发出删除事件
    json!({}),
    &cmd.editor_id,  // ❌ 使用当前编辑器ID，未检查是否对child有权限
    1,
));
```

### 漏洞分析

**场景**:
- User A owns Directory Block (ID: `dir-1`)
- User B creates Block2 (ID: `block-2`) inside `dir-1` (via `directory.create`)
- Block2's owner is B
- A executes `directory.delete` on `dir-1`

**当前流程**:
1. `directory_delete.rs` 直接发出 `core.delete` 事件，entity = `block-2`, editor_id = A
2. Engine Actor 接收事件，检查授权 (`actor.rs:308-319`)
3. **问题**: 此时 Block2 的 owner 是 B，A 没有权限删除
4. **但是**: 事件已经由 Directory capability 生成，Actor 无法拦截

**权限绕过路径**:
```
A (directory.delete on dir-1)
  → directory_delete handler generates core.delete event for block-2
  → Engine Actor checks: A authorized for core.delete on block-2?
  → ❌ FAIL: A is not owner of block-2, no grant exists
  → Command REJECTED
```

**实际测试结果**: 当前实现会在 Engine Actor 阶段拒绝命令，**但这是实现缺陷的副产品**，而不是正确的设计。

### 根本问题

Directory.delete **不应该**为关联的 Block 生成删除事件。如果真的需要级联删除：

1. 必须在 Handler 中预先检查每个 Block 的权限
2. 或者设计一个显式的"级联删除"capability，明确其权限要求

---

## 问题 4: Editor.delete 硬删除 ⚠️ MINOR

### 当前实现

**文件**: `src-tauri/src/engine/state.rs:303-307`

```rust
"editor.delete" => {
    self.editors.remove(&event.entity);  // ❌ 硬删除
    // Also remove all grants for this editor to prevent leaks in GrantsTable
    self.grants.remove_all_grants_for_editor(&event.entity);
}
```

### 矛盾分析

与 Block 删除相同，Editor 也应该采用软删除策略：
- 保留删除记录用于审计
- 防止 `editor_id` 被重用（类似 Unix UID recycling 问题）
- 支持事件重放时的完整历史

### 影响

相对 Block 删除问题较轻，但仍违反设计哲学。

---

## 符合设计的实现 ✅

### 1. Block 结构 (block.rs)

```rust
pub struct Block {
    pub block_id: String,
    pub name: String,
    pub block_type: String,
    pub contents: serde_json::Value,
    pub children: HashMap<String, Vec<String>>,  // ✅ 用户管理的关系图
    pub owner: String,
    pub metadata: BlockMetadata,
}
```

**评估**: ✅ 符合扁平存储设计，`children` 字段支持用户语义的 DAG

### 2. StateProjector 扁平存储 (state.rs:16-23)

```rust
pub struct StateProjector {
    pub blocks: HashMap<String, Block>,    // ✅ 扁平存储
    pub editors: HashMap<String, Editor>,  // ✅ 无嵌套
    pub grants: GrantsTable,
    pub editor_counts: HashMap<String, i64>,
}
```

**评估**: ✅ 完全符合设计，查询复杂度为 O(n) 符合预期

### 3. Actor 模型 _block_dir 处理 (actor.rs)

**运行时注入** (actor.rs:293-305):
```rust
if let Some(ref mut block) = block_opt {
    if let Some(temp_dir) = self.event_pool_with_path.db_path.parent() {
        inject_block_dir(temp_dir, &block.block_id, &mut block.contents)?;
    }
}
```

**持久化前剥离** (actor.rs:365-374):
```rust
let mut events_to_persist = events.clone();
for event in &mut events_to_persist {
    if let Some(contents) = event.value.get_mut("contents") {
        if let Some(obj) = contents.as_object_mut() {
            obj.remove("_block_dir");  // ✅ 移除运行时字段
        }
    }
}
```

**评估**: ✅ 完全符合设计，`_block_dir` 是运行时字段，不污染事件日志

### 4. Directory.create 引用生成 (directory_create.rs)

```rust
// Event 1: Create Content Block (core.create)
events.push(create_event(
    file_block_id.clone(),
    "core.create",
    json!({
        "name": file_name,
        "type": block_type,
        "owner": cmd.editor_id,  // ✅ Block 独立存在
        "contents": contents,
        "children": {},
    }),
    &cmd.editor_id,
    1,
));

// Event 2: Add entry to Directory
entries.insert(
    payload.path.clone(),
    json!({
        "id": file_block_id,  // ✅ 仅存储引用
        "type": "file",
    }),
);
```

**评估**: ✅ Block 和 Directory.entries 正交，符合引用语义

### 5. Code.write 实现 (code_write.rs)

```rust
let mut new_contents = if let Some(obj) = block.contents.as_object() {
    obj.clone()
} else {
    serde_json::Map::new()
};
new_contents.insert("text".to_string(), json!(payload.content));

let event = create_event(
    block.block_id.clone(),
    "code.write",
    json!({
        "contents": new_contents,
        "metadata": new_metadata.to_json()
    }),
    &cmd.editor_id,
    1,
);
```

**评估**: ✅ 正确使用 Payload 类型，更新 metadata 时间戳

---

## 审查覆盖范围

### 已审查模块 ✅

| 模块 | 文件数 | 关键发现 |
|------|--------|----------|
| **Core Models** | 3 | Block 无 deleted 字段 |
| **StateProjector** | 1 | 硬删除实现 |
| **Engine Actor** | 1 | _block_dir 处理正确 |
| **Core Capabilities** | 1 | delete.rs 发出软删除标志但被忽略 |
| **Directory Extension** | 6 | directory_delete 违反引用语义 |
| **Code Extension** | 2 | 实现正确 |
| **File Commands** | 1 | 实现正确 |

### 未审查模块 ⚠️

- Markdown Extension (`src/extensions/markdown/*`)
- Event Store (`src/engine/event_store.rs`)
- Archive 处理 (`src/elf/*`)
- 前端绑定 (`src/bindings.ts` - 自动生成)

---

## 修复优先级

### P0 - 必须修复 (阻塞发布)

1. **问题 2**: Directory.delete 改为纯引用语义
   - 影响范围：核心设计哲学
   - 修复复杂度：中
   - 建议：本次 PR 必须解决

2. **问题 1**: StateProjector 实现软删除
   - 影响范围：删除语义、事件溯源
   - 修复复杂度：高（需要修改 Block 结构、迁移逻辑）
   - 建议：本次 PR 必须解决

### P1 - 应该修复 (下一个迭代)

3. **问题 4**: Editor.delete 软删除
   - 影响范围：审计、事件重放
   - 修复复杂度：低
   - 建议：可在后续 PR 解决

### P2 - 未来优化

- 实现垃圾回收机制 (GC)
- 添加可达性分析 (类似 Git `git gc`)
- 支持显式的"永久删除"操作（需要特殊权限）

---

## 测试建议

### 需要添加的测试场景

1. **软删除行为测试**:
```rust
#[tokio::test]
async fn test_soft_delete_does_not_remove_block() {
    // 1. Create block
    // 2. Delete block
    // 3. Verify block.deleted = true
    // 4. Verify block still in state.blocks
    // 5. Verify can't access via normal queries (filtered)
}
```

2. **多路径引用测试**:
```rust
#[tokio::test]
async fn test_directory_delete_does_not_cascade() {
    // 1. Create Directory A
    // 2. Create Block B
    // 3. Add B to Directory A
    // 4. Add B to Directory C (另一个引用)
    // 5. Delete Directory A
    // 6. Verify Block B still exists
    // 7. Verify Directory C still references B
}
```

3. **权限隔离测试**:
```rust
#[tokio::test]
async fn test_directory_delete_respects_block_ownership() {
    // 1. Alice creates Directory
    // 2. Bob creates Block in Directory (Bob owns Block)
    // 3. Alice deletes Directory entry
    // 4. Verify Block NOT deleted (Bob still owns it)
}
```

---

## 总结

### 关键发现

| 问题 | 严重性 | 影响范围 | 是否阻塞 |
|------|--------|----------|----------|
| StateProjector 硬删除 | CRITICAL | 核心架构 | ✅ 是 |
| Directory.delete 级联删除 | CRITICAL | 设计哲学 | ✅ 是 |
| Directory.delete 权限缺失 | SECURITY | 安全模型 | ⚠️ 副产品已缓解 |
| Editor.delete 硬删除 | MINOR | 审计完整性 | ❌ 否 |

### 修复路径

**第一阶段** (本次 PR):
1. 修改 Block 结构，添加 `deleted: bool` 字段
2. 修改 StateProjector 处理 core.delete 为软删除
3. 重构 directory_delete.rs 为纯引用语义
4. 添加对应的集成测试

**第二阶段** (后续迭代):
1. 实现垃圾回收机制
2. 处理 editor.delete 软删除
3. 添加"永久删除"能力（需要特殊权限）

### 设计验证

除删除相关问题外，**其余实现与设计文档高度一致**：
- ✅ 扁平存储架构
- ✅ 引用与知识图谱正交
- ✅ _block_dir 运行时注入
- ✅ Event Sourcing 模式
- ✅ CBAC 权限模型（除级联删除漏洞）

**建议**: 在合并本分支前，必须解决 P0 级别的问题。

---

## 修复总结（2025-12-29）

### 修复的问题

#### ✅ 问题 1: StateProjector 硬删除（已确认为正确实现）

**决策**: 保持当前硬删除实现
- **理由**: Event Store 已完整记录删除历史，StateProjector 只需反映"当前活跃状态"
- **优势**: 简化实现，无需到处过滤 `deleted=true` 标志
- **Event Sourcing 纯粹性**: Event 是真相源，StateProjector 是投影

#### ✅ 问题 2: Directory.delete 引用语义（已修复）

**修复内容**:
1. 重构 `src-tauri/src/extensions/directory/directory_delete.rs`
2. 移除所有 `core.delete` 事件生成
3. 只更新 `Directory.entries` 映射

**修复前**:
```rust
// ❌ 级联删除关联 Block
events.push(create_event(
    child_id.to_string(),
    "core.delete",
    json!({}),
    &cmd.editor_id,
    1,
));
```

**修复后**:
```rust
// ✅ 只删除索引引用
let event = create_event(
    block.block_id.clone(),
    "directory.write",
    json!({ "contents": { "entries": new_entries } }),
    &cmd.editor_id,
    1,
);
Ok(vec![event])
```

**测试验证**:
- ✅ `test_delete_basic` 更新通过
- ✅ `test_full_workflow` 保持通过
- ✅ `test_security_path_matching_isolation` 验证引用独立性
- ✅ 所有 192 个后端测试通过

#### ✅ 问题 3: 权限检查缺失（已解决）

**解决方式**: 问题 2 的修复自动解决了此问题
- 不再为关联 Block 生成 `core.delete` 事件
- 删除 Directory 只需要对 Directory Block 本身的权限
- 完全符合 CBAC 权限模型

#### ⏭️ 问题 4: Editor.delete 硬删除（推迟至下一个 PR）

**决策**: 按照用户要求，留待后续 PR 解决

### 文档更新

1. **CHANGELOG-DIRECTORY.md**:
   - 更新 `directory.delete` 描述为"纯引用语义"
   - 新增 3.3 节"引用语义 vs 所有权语义"详细说明设计决策

2. **CODE_REVIEW_DATASTRUCT.md**:
   - 添加修复总结部分
   - 标记已修复问题

### 设计影响

**正面影响**:
- ✅ 支持多路径引用：同一 Block 可以被多个 Directory 引用
- ✅ 权限隔离：删除目录不需要对 Block 的删除权限
- ✅ Unix-like 语义：类似 inode 引用计数机制
- ✅ 为未来 GC 机制奠定基础

**无负面影响**:
- 所有现有测试保持通过
- 前端功能无需修改（只是后端行为调整）

### 结论

feat/datastruc 分支现已**符合设计哲学要求**，可以安全合并到 dev 分支。

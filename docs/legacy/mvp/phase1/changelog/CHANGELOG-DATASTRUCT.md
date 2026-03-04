# Changelog: 扁平化数据结构设计与查询优化

本文档记录了 Elfiee 核心数据结构的设计思想、设计原因、最佳实践参考以及在去中心化协作场景下的创新性。

## 目录

1. [数据结构设计](#1-数据结构设计)
2. [设计原因](#2-设计原因)
3. [最佳实践参考](#3-最佳实践参考)
4. [创新性与特性](#4-创新性与特性)
5. [实施案例](#5-实施案例)

---

## 1. 数据结构设计

### 1.1 数据层次抽象

Elfiee 的数据按照**所有权和归属关系**划分为四层抽象：

```
Layer 4: Organization / Workspace (未来扩展)
  ├─ Team A
  │  ├─ alice (Editor)
  │  └─ bob (Editor)
  └─ Team B
     └─ charlie (Editor)

Layer 3: Editor (编辑者 / 所有权层)
  ├─ alice
  │  └─ owns: [block-1, block-2, block-dir-1]
  ├─ bob
  │  └─ owns: [block-3, block-4]
  └─ gpt-agent (AI)
     └─ owns: [block-5]

Layer 2: Block (内容对象 / 扁平存储层)
  ├─ block-1 { owner: "alice", type: "markdown" }
  ├─ block-2 { owner: "alice", type: "code" }
  ├─ block-3 { owner: "bob", type: "markdown" }
  └─ block-dir-1 { owner: "alice", type: "directory",
                    contents.entries: {"file.md" -> block-3} }

Layer 1: Event (历史记录 / 事件溯源层)
  └─ _eventstore.db: [e1, e2, e3, ...]
```

**关键特征**：
- **Layer 1**：不可变事件日志，所有状态变更的唯一来源
- **Layer 2**：扁平存储，所有 Block 在 `HashMap<block_id, Block>` 中平等存在
- **Layer 3**：所有权关系，每个 Block 归属于一个 Editor（通过 `owner` 字段）
- **Layer 4**：组织层次（未来），Editor 可以属于 Team/Workspace

### 1.2 扁平存储的查询复杂度

#### 基本查询场景

**查询 1**：获取某个 Editor 拥有的所有 Blocks

```rust
// 朴素实现：O(n) 全表扫描
fn get_blocks_by_owner(owner: &str) -> Vec<&Block> {
    state.blocks.values()
        .filter(|b| b.owner == owner)
        .collect()
}
// n = blocks.len()，当 n = 100万时，扫描耗时显著
```

**查询 2**：通过 Directory 获取所有子 Blocks

```rust
// 两步查询
// Step 1: 读取 Directory.entries (O(1))
let dir_block = state.blocks.get(dir_id)?;
let entries = dir_block.contents["entries"].as_object()?;

// Step 2: 遍历 entries，查询每个 Block (O(m))
for (path, ref_obj) in entries {
    let block_id = ref_obj["block_id"].as_str()?;
    let block = state.blocks.get(block_id)?;  // O(1)
}
// 总复杂度：O(m)，m = entries.len()
```

**查询 3**：获取某个 Editor 可以访问的所有 Blocks（包括 owned + granted）

```rust
// 朴素实现：O(n + g)
fn get_accessible_blocks(editor_id: &str) -> Vec<&Block> {
    // 1. 扫描所有 blocks，找到 owned (O(n))
    let owned: Vec<_> = state.blocks.values()
        .filter(|b| b.owner == editor_id)
        .collect();

    // 2. 查询 grants 表 (O(g))
    let granted_block_ids = state.grants.get_granted_blocks(editor_id);

    // 3. 合并结果
    owned.extend(
        granted_block_ids.iter()
            .filter_map(|id| state.blocks.get(id))
    );
    owned
}
// n = blocks.len(), g = grants 数量
```

### 1.3 两种优化思路

#### 思路 A：合并 User（索引优化）

**核心思想**：建立反向索引，将 Blocks 按所有者（Editor）分组。

```rust
pub struct StateProjector {
    /// 主存储：扁平的 Block HashMap
    pub blocks: HashMap<String, Block>,

    /// 索引 1：按所有者分组
    /// editor_id -> [block_ids]
    pub blocks_by_owner: HashMap<String, Vec<String>>,

    /// 索引 2：按权限分组
    /// editor_id -> [granted_block_ids]
    pub blocks_by_grant: HashMap<String, HashSet<String>>,

    pub editors: HashMap<String, Editor>,
    pub grants: GrantsTable,
}
```

**优化后的查询**：

```rust
// 查询 1：O(n) → O(k)
fn get_blocks_by_owner(owner: &str) -> Vec<&Block> {
    state.blocks_by_owner.get(owner)
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|id| state.blocks.get(id))
        .collect()
}
// k = 该 owner 拥有的 blocks 数量（通常 k << n）

// 查询 3：O(n + g) → O(k1 + k2)
fn get_accessible_blocks(editor_id: &str) -> Vec<&Block> {
    let owned_ids = state.blocks_by_owner.get(editor_id).unwrap_or(&vec![]);
    let granted_ids = state.blocks_by_grant.get(editor_id).unwrap_or(&HashSet::new());

    owned_ids.iter().chain(granted_ids.iter())
        .filter_map(|id| state.blocks.get(id))
        .collect()
}
// k1 = owned blocks 数量, k2 = granted blocks 数量
```

**维护成本**：

```rust
// 在 apply_event 中维护索引
impl StateProjector {
    pub fn apply_event(&mut self, event: &Event) {
        match cap_id {
            "core.create" => {
                let block = parse_block_from_event(event);
                let owner = block.owner.clone();

                // 1. 更新主存储
                self.blocks.insert(block.block_id.clone(), block);

                // 2. 更新索引
                self.blocks_by_owner
                    .entry(owner)
                    .or_default()
                    .push(block.block_id.clone());
            }

            "core.delete" => {
                let block = self.blocks.get(&event.entity)?;
                let owner = block.owner.clone();

                // 1. 标记删除（不移除）
                self.blocks.get_mut(&event.entity)?.metadata.deleted = true;

                // 2. 更新索引
                if let Some(ids) = self.blocks_by_owner.get_mut(&owner) {
                    ids.retain(|id| id != &event.entity);
                }
            }

            "core.grant" => {
                let grant = parse_grant_from_event(event);

                // 更新 grants 表
                self.grants.add_grant(...);

                // 更新索引
                self.blocks_by_grant
                    .entry(grant.editor_id.clone())
                    .or_default()
                    .insert(grant.block_id.clone());
            }

            _ => {}
        }
    }
}
```

**优劣分析**：

✅ **优点**：
- 查询性能大幅提升（O(n) → O(k)）
- 适合"按用户浏览"的 UI 场景
- 索引维护逻辑简单（在 apply_event 中同步更新）

❌ **缺点**：
- 内存开销增加（索引占用额外空间）
- 事件重放时需要重建所有索引（启动时间增加）
- 索引维护代码与业务逻辑耦合

#### 思路 B：合并 Block（级联关系）

**核心思想**：在 Editor 或 Directory 层级直接嵌入 Blocks 数据，形成层级结构。

**方案 B1：Editor 嵌入 Blocks**

```rust
pub struct Editor {
    pub editor_id: String,
    pub name: String,

    /// 直接包含 owned blocks
    pub blocks: HashMap<String, Block>,
}

pub struct StateProjector {
    /// Editor 包含其 Blocks（层级存储）
    pub editors: HashMap<String, Editor>,

    /// 全局索引（用于快速查找）
    pub block_index: HashMap<String, String>,  // block_id -> editor_id
}
```

**查询优化**：

```rust
// 查询 1：O(n) → O(1)
fn get_blocks_by_owner(owner: &str) -> Vec<&Block> {
    state.editors.get(owner)
        .map(|editor| editor.blocks.values().collect())
        .unwrap_or_default()
}

// 查询 Block：O(1) → O(1)（通过索引）
fn get_block(block_id: &str) -> Option<&Block> {
    let editor_id = state.block_index.get(block_id)?;
    let editor = state.editors.get(editor_id)?;
    editor.blocks.get(block_id)
}
```

**问题**：

```rust
// 问题 1：Block 被多个 Directory 引用时如何处理？
Directory A: entries["file.md"] -> block-123
Directory B: entries["docs/file.md"] -> block-123

// 如果 block-123 嵌入在某个 Directory 中：
// - 需要在 Directory A 和 B 中都存储副本？（数据冗余）
// - 还是只在一个 Directory 中存储，其他引用？（回到扁平存储）

// 问题 2：跨 Editor 的引用如何处理？
alice.blocks["dir-1"].entries["file.md"] -> bob.blocks["markdown-1"]
// 跨 Editor 的引用需要全局索引，失去了层级存储的意义

// 问题 3：事件重放复杂度
// Event: bob/markdown.write -> block-123
// 需要先找到 block-123 属于哪个 Editor（查全局索引）
// 然后更新 alice.blocks["block-123"]
// 层级存储反而增加了复杂度
```

**方案 B2：Directory 嵌入子 Blocks**

```rust
pub struct DirectoryBlock {
    pub block_id: String,
    pub owner: String,

    /// 直接嵌入子 Blocks（而非引用）
    pub children: HashMap<String, Block>,
}
```

**致命问题**：

1. **循环引用**：
   ```
   Directory A 的 children 包含 Directory B
   Directory B 的 children 包含 Directory A
   → 无法序列化，内存泄漏
   ```

2. **多路径冲突**：
   ```
   Directory A 的 children 包含 Block X
   Directory B 也引用 Block X
   → 需要复制 Block X？更新如何同步？
   ```

3. **与事件溯源冲突**：
   ```
   Event: alice/markdown.write -> block-X
   如果 block-X 嵌入在 Directory A 中：
   - Event 应该记录 Directory A 的 ID 还是 Block X 的 ID？
   - 如果 Block X 被移动到 Directory B，历史事件如何解释？
   ```

**优劣分析**：

✅ **优点**（理论上）：
- 查询性能极佳（O(1) 直接访问）
- 符合某些用户的直觉（文件夹"包含"文件）

❌ **缺点**（实际上不可行）：
- 与扁平存储、事件溯源的设计理念根本冲突
- 无法处理多路径引用、循环引用
- 数据冗余、更新同步复杂
- 与 CBAC 权限模型冲突（权限是 Block 级别的）

### 1.4 Elfiee 的选择：扁平存储 + 按需索引

**核心决策**：

1. ✅ **主存储层**：扁平的 `HashMap<block_id, Block>`
2. ✅ **按需索引**：根据查询热点建立反向索引（思路 A）
3. ❌ **拒绝级联**：不采用层级嵌套存储（思路 B）

**当前实现**（MVP）：

```rust
pub struct StateProjector {
    /// 主存储：扁平 HashMap
    pub blocks: HashMap<String, Block>,

    /// 权限表
    pub grants: GrantsTable,

    pub editors: HashMap<String, Editor>,
    pub editor_counts: HashMap<String, i64>,

    // ⚠️ 当前未实现反向索引（接受 O(n) 查询）
}
```

**未来优化方向**：

```rust
pub struct StateProjector {
    pub blocks: HashMap<String, Block>,
    pub grants: GrantsTable,
    pub editors: HashMap<String, Editor>,
    pub editor_counts: HashMap<String, i64>,

    // 🔄 Phase 2: 添加索引（当用户规模增长时）
    pub blocks_by_owner: HashMap<String, Vec<String>>,
    pub blocks_by_grant: HashMap<String, HashSet<String>>,
    pub blocks_by_type: HashMap<String, Vec<String>>,  // 按 block_type 索引
}
```

**设计权衡**：

| 维度 | 扁平存储 | 扁平 + 索引 | 层级存储 |
|------|---------|------------|---------|
| 查询性能 | O(n) | O(k) | O(1) |
| 内存占用 | 低 | 中 | 高（数据冗余） |
| 实现复杂度 | 简单 | 中等 | 高（难以实现） |
| 事件溯源兼容性 | ✅ 完美 | ✅ 兼容 | ❌ 冲突 |
| 多路径引用 | ✅ 支持 | ✅ 支持 | ❌ 难以支持 |
| CBAC 权限模型 | ✅ 对象级权限 | ✅ 对象级权限 | ❌ 继承权限混乱 |

**为什么 MVP 不实现索引？**

1. **延迟优化原则**：在性能成为瓶颈之前，保持简单
2. **灵活性**：索引策略可以根据实际使用模式调整
3. **正确性优先**：先保证扁平存储的语义正确性
4. **渐进式优化**：从朴素实现 → 分析热点 → 针对性优化

#### Layer 3a: 文件系统索引

```json
{
  "block_type": "directory",
  "contents": {
    "entries": {
      "src/main.rs": {
        "block_id": "uuid-123"
      },
      "lib/": {
        "block_id": "uuid-456"
      }
    }
  },
  "children": {}
}
```

**语义**：
- `entries` 是**命名空间**（namespace），不是所有权容器
- 只存储 `block_id` 引用，不复制 block 数据
- 引用可能悬空（指向已删除的 Block）
- 一个 Block 可以被多个 Directory 引用（多路径）

#### Layer 3b: 知识图谱索引

```json
{
  "block_id": "note-A",
  "block_type": "markdown",
  "children": {
    "links": ["note-B", "note-C"],
    "embeds": ["diagram-1"],
    "annotates": ["code-block-1"]
  }
}
```

**语义**：
- 用户主动创建的语义关系（不是系统管理的）
- 通过 `core.link` / `core.unlink` capability 管理
- 形成有向图（DAG），支持任意关系类型
- 与 `Directory.entries` 完全正交

### 1.3 关系的正交性

```
文件系统视图                          知识图谱视图
(System-Managed)                     (User-Managed)
        ↓                                   ↓

    Root Dir                            Note A
    ├─ docs/                            ├─ links → [Note B]
    │  └─ README.md ────────┐           └─ embeds → [Diagram 1]
    └─ src/                 │                     ↓
       └─ main.rs           │            Note B
                            │            └─ links → [Note C]
                            │
                    同一个 Block (block-123)
                            │
                    扁平存储: blocks["block-123"]
                            │
                    事件日志: _eventstore.db
```

**关键洞察**：
- 同一个 Block 可以同时出现在两个视图中
- Directory 引用不影响 `children` 关系
- `children` 关系不影响文件系统路径
- 两者在 GC 时都需要考虑（**双维度可达性**）

---

## 2. 设计原因

### 2.1 为什么采用扁平存储而非层级存储？

#### 问题 1：层级存储与查询性能的矛盾

**层级存储看似高效**：

```rust
// 理想情况：Editor 直接包含 Blocks
struct Editor {
    blocks: HashMap<String, Block>  // O(1) 查询
}

get_blocks_by_owner("alice") → O(1)
```

**实际困境**：

```rust
// 问题 1：跨 Editor 引用
alice.blocks["dir-1"].entries["file.md"] -> bob.blocks["markdown-1"]
// 需要全局索引才能解析引用，层级存储失去意义

// 问题 2：多路径引用
Directory A: entries["file.md"] -> block-X
Directory B: entries["docs/file.md"] -> block-X
// block-X 应该嵌入在 A 还是 B？还是复制两份？

// 问题 3：事件重放
Event: bob/markdown.write -> block-123
// 需要先找到 block-123 在哪个 Editor 下（查全局索引）
// 然后更新对应的嵌套结构
// 层级存储反而增加了复杂度
```

**根本矛盾**：层级存储与**多路径引用**、**跨所有者引用**、**事件溯源**在语义上冲突。

#### 问题 2：层级存储与事件溯源的冲突

**事件溯源的核心**：Event 记录的是 Block 级别的变更

```rust
Event {
    entity: "block-123",  // Block 的 ID
    attribute: "alice/markdown.write",
    value: { contents: { markdown: "new content" } }
}
```

**如果采用层级存储**：

```rust
// 方案 A：Event 记录 Block ID（当前做法）
// 问题：需要反查 block-123 在哪个 Editor 下
//      如果 block-123 被移动到另一个 Editor，历史事件如何解释？

// 方案 B：Event 记录嵌套路径
Event {
    entity: "alice/blocks/block-123",  // 路径式 ID
    ...
}
// 问题：block-123 被移动后，entity 改变，无法追踪同一对象的历史

// 方案 C：Event 同时记录 Block ID 和路径
// 问题：数据冗余，且路径信息在重放时可能失效
```

**结论**：事件溯源要求对象具有**稳定的、全局唯一的标识符**（`block_id`），这与层级存储中的"路径式标识"冲突。

#### 解决方案：扁平存储 + 索引分离

Elfiee 采用**扁平对象存储 + 按需索引**：

```rust
// Layer 2: 扁平存储（稳定的对象标识）
blocks: HashMap<String, Block>  // block_id -> Block

// Layer 3: 索引（可变的组织方式）
Directory.entries: {path -> block_id}  // 路径索引
Block.children: {relation -> [block_ids]}  // 关系索引
blocks_by_owner: {editor_id -> [block_ids]}  // 所有权索引（可选）
```

**优势**：

1. ✅ **稳定的对象标识**
   ```rust
   Event: alice/markdown.write -> block-123
   // block-123 永远指向同一个对象，无论它在哪个索引中
   ```

2. ✅ **多路径引用**
   ```rust
   Directory A: entries["file.md"] -> block-123
   Directory B: entries["docs/file.md"] -> block-123
   // block-123 只存储一份，多个索引指向它
   ```

3. ✅ **权限对象化**
   ```rust
   grants: (bob, markdown.read, block-123)
   // 权限绑定到 block-123，与它在哪个 Directory 无关
   ```

4. ✅ **索引灵活性**
   ```rust
   // 可以按需添加索引，不影响主存储
   blocks_by_owner: alice -> [block-1, block-2]
   blocks_by_type: markdown -> [block-1, block-3]
   ```

### 2.2 为什么区分文件系统和知识图谱？

#### 问题：混淆组织和语义

很多系统（如 Notion、Obsidian）将**文件夹结构**和**知识关联**混为一谈：

```
# Notion 的困境
/Projects
  /Project A
    - Design Doc  ←─ 在文件夹中
    - Meeting Notes  ←─ 同时又被 [[linked]]
```

用户常困惑：
- 这个笔记应该放在哪个文件夹？
- 它被多个地方引用，应该复制吗？
- 移动文件夹会破坏引用关系吗？

#### 解决方案：正交的双维度索引

Elfiee 明确区分：

| 维度 | 用途 | 管理者 | 数据结构 |
|------|------|--------|---------|
| **文件系统** | 路径导航、项目组织 | 系统/约定 | `Directory.entries` |
| **知识图谱** | 语义连接、思维关联 | 用户 | `Block.children` |

**案例**：
```
文件系统视图：
  /research
    /papers
      - quantum-computing.md
    /notes
      - algorithm-ideas.md

知识图谱视图：
  quantum-computing.md
    └─ links → [algorithm-ideas.md, neural-networks.md]

  algorithm-ideas.md
    └─ links → [quantum-computing.md, optimization-theory.md]
```

**优势**：
- 用户可以按项目组织文件，同时按主题建立关联
- 移动文件不会破坏知识链接
- 两个维度可以独立演化

### 2.3 为什么采用软删除 + GC？

#### 问题：协作环境中的删除冲突

传统系统的立即删除（Hard Delete）在协作场景下会导致：

```
时间线：
T1: Alice 删除 Document A
T2: Bob 在 Document B 中引用 Document A
T3: Bob 保存时发现 Document A 不存在 → 数据丢失！
```

#### 解决方案：软删除 + 可达性 GC

```rust
// 软删除：只标记 deleted
core.delete(block_id) → metadata.deleted = true

// GC 清理：定期或手动触发
archive.gc() → 删除 (deleted=true && !reachable) 的 Blocks
```

**双维度可达性定义**：

```rust
fn is_reachable(block_id) -> bool {
    // 从文件系统根节点可达
    reachable_via_filesystem(block_id, roots)

    // 或从知识图谱根节点可达
    || reachable_via_children(block_id, roots)
}
```

**优势**：
1. ✅ **协作友好**：删除标记后，其他用户仍能看到引用
2. ✅ **可恢复性**：误删除可以在 GC 前恢复
3. ✅ **审计完整**：删除事件记录在 Event Log 中
4. ✅ **空间优化**：GC 定期清理真正不需要的数据

---

## 3. 最佳实践参考

### 3.1 Unix inode 系统（引用计数 + 扁平存储）

**设计**：
```c
// 扁平存储：inode table
struct inode {
    ino_t ino;        // 唯一标识
    uid_t owner;      // 所有者
    int nlink;        // 硬链接计数 ⭐
    char *data;       // 数据
};

// 索引：directory entries
struct dirent {
    ino_t ino;        // 指向 inode
    char name[256];   // 文件名
};
```

**关键机制**：
- 所有 inode 在扁平的 inode table 中
- Directory 只存储 `(name, ino)` 映射
- `nlink` 引用计数：`unlink()` 减少计数，0 时删除

**Elfiee 的对应**：
```rust
// 扁平存储
StateProjector.blocks: HashMap<block_id, Block>

// 索引
Directory.entries: {path -> block_id}

// 生命周期管理（类似但更灵活）
软删除 + 可达性 GC（支持 DAG，不仅仅是树）
```

**借鉴点**：
1. ✅ 扁平存储，索引分离
2. ✅ 引用不等于所有权
3. ✅ 生命周期由引用关系决定

### 3.2 Git 对象存储（内容寻址 + 可达性 GC）

**设计**：
```bash
# 扁平存储：所有对象通过 SHA-1 hash 索引
.git/objects/
  ab/cdef123...  (blob - 文件内容)
  12/3456abc...  (tree - 目录索引)
  78/90abcde...  (commit - 提交对象)

# tree 对象内容
tree 12345 {
    "main.rs" -> blob:abcdef123
    "lib/"    -> tree:234567
}
```

**关键机制**：
- 所有对象（blob/tree/commit）扁平存储在 `.git/objects/`
- tree 对象只存储 hash 引用，不复制内容
- `git gc` 从 refs/heads, refs/tags 开始可达性分析

**Elfiee 的对应**：
```rust
// 扁平存储
blocks: HashMap<block_id, Block>

// 索引（类似 tree）
Directory.entries: {path -> block_id}

// GC（类似但双维度）
从 outline_root + linked_repos + children 关系可达性分析
```

**借鉴点**：
1. ✅ 内容与索引分离（内容寻址思想）
2. ✅ 可达性 GC（从 roots 标记-清除）
3. ✅ 不可变对象 + 事件日志

### 3.3 Capability-based OS（KeyKOS/EROS）

**设计**：
```
// 扁平对象空间
ObjectSpace {
    objects: Map<OID, Object>
}

// Capability = 对象引用 + 权限
Capability {
    oid: ObjectID,
    rights: Rights  // READ, WRITE, DELETE
}

// Directory 也是对象
DirectoryObject {
    entries: Map<String, Capability>
}
```

**关键机制**：
- 所有对象在统一的对象空间，无层级
- 通过 Capability 引用对象（带权限）
- 权限是对象级别的，不通过容器继承

**Elfiee 的对应**：
```rust
// 扁平对象空间
StateProjector.blocks

// Capability-Based Access Control
GrantsTable: {(editor_id, cap_id, block_id) -> grant}

// 权限检查在 actor 层统一执行
is_authorized(editor_id, cap_id, block_id)
```

**借鉴点**：
1. ✅ 扁平对象模型
2. ✅ 能力驱动的权限模型（CBAC）
3. ✅ 权限独立于引用关系

### 3.4 Elfiee 的实际案例

#### 案例 1：Directory 删除不影响子 Blocks

**实现**（`directory_delete.rs`）：

```rust
#[capability(id = "directory.delete", target = "directory")]
fn handle_delete(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required")?;
    let payload: DirectoryDeletePayload = serde_json::from_value(cmd.payload.clone())?;

    let mut entries = get_entries(block)?;

    // ⚠️ 关键：只删除 entry，不删除 Block
    entries.remove(&payload.path);

    // 生成 directory.write 事件（更新索引）
    let event = create_event(
        block.block_id.clone(),
        "directory.write",
        json!({ "contents": { "entries": entries } }),
        &cmd.editor_id, 1
    );

    Ok(vec![event])
    // ✅ 不生成 core.delete 事件
}
```

**对比旧实现**（已修复的权限漏洞）：

```rust
// ❌ 旧实现：级联删除，绕过权限检查
if entry["type"] == "file" {
    events.push(create_event(
        child_id.to_string(),
        "core.delete",  // 未检查权限！
        json!({}),
        &cmd.editor_id, 1
    ));
}
```

**改进**：
- ✅ 纯引用语义：删除 entry 不删除 Block
- ✅ 权限安全：不能绕过 Block 的 delete 权限
- ✅ 协作友好：其他用户的引用不受影响

#### 案例 2：双维度可达性 GC

**实现**（`actor.rs`）：

```rust
impl EngineActor {
    fn find_orphans(&self) -> Vec<String> {
        let roots = self.get_root_set();

        // 维度 1：文件系统可达性
        let reachable_fs = self.reachable_via_filesystem(&roots);

        // 维度 2：知识图谱可达性
        let reachable_graph = self.reachable_via_children(&roots);

        // 并集：任一维度可达即保留
        let all_reachable: HashSet<_> = reachable_fs
            .union(&reachable_graph)
            .cloned()
            .collect();

        // 找出已删除且不可达的 Blocks
        self.state.blocks.iter()
            .filter(|(id, block)| {
                block.metadata.deleted
                && !all_reachable.contains(*id)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    fn reachable_via_filesystem(&self, roots: &RootSet) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from(roots.filesystem_roots());

        while let Some(block_id) = queue.pop_front() {
            if !visited.insert(block_id.clone()) { continue; }

            if let Some(block) = self.state.blocks.get(&block_id) {
                if block.block_type == "directory" {
                    // 遍历 entries
                    if let Some(entries) = block.contents.get("entries") {
                        for entry in entries.as_object().unwrap().values() {
                            if let Some(id) = entry["block_id"].as_str() {
                                queue.push_back(id.to_string());
                            }
                        }
                    }
                }
            }
        }
        visited
    }

    fn reachable_via_children(&self, roots: &RootSet) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from(roots.all());

        while let Some(block_id) = queue.pop_front() {
            if !visited.insert(block_id.clone()) { continue; }

            if let Some(block) = self.state.blocks.get(&block_id) {
                // 遍历 children 关系
                for child_ids in block.children.values() {
                    queue.extend(child_ids.clone());
                }
            }
        }
        visited
    }
}
```

**特点**：
- ✅ 支持 DAG 关系（不仅仅是树）
- ✅ 两个维度独立计算，取并集
- ✅ 保留用户的语义关联（即使文件系统中删除）

---

## 4. 创新性与特性

### 4.1 面向去中心化协作的设计

#### 传统集中式系统的问题

Google Docs、Notion 等系统采用**中心化服务器 + OT/CRDT**：

```
         Central Server
              ↓
    ┌─────────┴─────────┐
  Client A         Client B
    ↓                 ↓
  冲突解决依赖服务器的仲裁
```

**局限**：
1. ❌ 依赖网络：离线无法协作
2. ❌ 中心化：服务器故障导致数据不可用
3. ❌ 信任问题：必须信任服务器不篡改历史

#### Elfiee 的去中心化设计

```
Alice.elf ←─ P2P Sync ─→ Bob.elf
    ↓                        ↓
  本地完整                 本地完整
  事件日志                 事件日志
    ↓                        ↓
  向量时钟                 向量时钟
  冲突检测                 冲突检测
```

**核心机制**：

1. **本地优先（Local-First）**
   ```rust
   // 每个 .elf 文件包含完整的 Event Log
   _eventstore.db: 所有历史事件

   // 无需服务器即可工作
   EngineActor → StateProjector → 本地状态
   ```

2. **向量时钟冲突检测**
   ```rust
   Event {
       timestamp: {
           "alice": 5,  // Alice 的第 5 次操作
           "bob": 3     // Bob 的第 3 次操作
       }
   }

   // 检测冲突
   fn has_conflict(&self, editor_id: &str, expected_count: i64) -> bool {
       let current = self.editor_counts.get(editor_id);
       expected_count < current  // 基于过时状态
   }
   ```

3. **事件日志合并**
   ```
   Alice 的操作：e1(alice:1) → e2(alice:2) → e3(alice:3)
   Bob 的操作：  e4(bob:1) → e5(bob:2)

   合并后：
   _eventstore.db: [e1, e2, e3, e4, e5]  // 按 timestamp 排序

   StateProjector 重放 → 最终一致状态
   ```

**优势**：
- ✅ **离线可用**：本地完整数据，无需网络
- ✅ **去中心化**：P2P 同步，无单点故障
- ✅ **可验证性**：事件日志不可篡改，完整审计轨迹
- ✅ **最终一致性**：向量时钟保证冲突检测

### 4.2 面向多智能体协作的设计

#### AI Agent 作为一等公民

传统系统中，AI 只是外部工具。Elfiee 将 AI Agent 设计为**原生编辑者**：

```rust
pub enum EditorType {
    Human,   // 人类用户
    Bot,     // AI Agent
}

pub struct Editor {
    pub editor_id: String,
    pub name: String,
    pub editor_type: EditorType,  // ⭐ 类型标识
}
```

**CBAC 权限控制**：

```rust
// 人类可以给 AI 授予特定权限
core.grant(
    editor: "gpt-4-agent",
    capability: "markdown.write",
    block: "research-notes"
)

// AI 只能在授权范围内操作
// 所有操作记录在 Event Log 中
Event {
    attribute: "gpt-4-agent/markdown.write",
    value: { contents: { markdown: "AI 生成的总结" } }
}
```

**审计与回溯**：

```rust
// 查询所有 AI 的操作
fn get_ai_edits() -> Vec<Event> {
    events.iter()
        .filter(|e| {
            let editor_id = e.attribute.split('/').next()?;
            editors.get(editor_id)?.editor_type == EditorType::Bot
        })
        .collect()
}

// 可以精确回滚 AI 的修改
fn undo_ai_edit(event_id: &str) {
    // Event Sourcing 天然支持历史回溯
}
```

**多 Agent 协同工作流**：

```
场景：代码审查流水线

1. Alice (Human) 创建代码 Block
   Event: alice/code.write → code_block_1

2. CodeReviewer (Bot) 自动审查
   Event: code-reviewer-bot/code.annotate → comment_block_1
   Block.children: code_block_1 { "annotates": [comment_block_1] }

3. TestRunner (Bot) 执行测试
   Event: test-runner-bot/code.test → test_result_block_1
   Block.children: code_block_1 { "test_results": [test_result_block_1] }

4. Alice 查看审查结果
   通过 Block.children["annotates"] 导航到评论
```

**优势**：
- ✅ **可追溯**：每个 AI 操作都有完整审计日志
- ✅ **可控制**：细粒度权限，AI 只能访问授权资源
- ✅ **可协作**：人类和 AI 共享同一个 Event Log
- ✅ **可回滚**：AI 的修改可以精确撤销

### 4.3 带来的用户体验提升

#### 1. 快速加载与响应

**机制**：
```rust
// Actor 初始化时一次性重放所有事件
StateProjector::replay(events) → 内存中的完整状态

// 后续所有读操作直接查询内存
get_block(block_id) → O(1) HashMap 查询

// 写操作异步持久化
append_events(events) → 异步写入 SQLite
apply_event(event) → 同步更新内存
```

**对比传统数据库查询**：
```
传统系统：
  每次操作 → SQL 查询 → 磁盘 I/O → 延迟 50-100ms

Elfiee：
  读操作 → 内存查询 → 延迟 < 1ms
  写操作 → 内存更新（立即） + 异步持久化（后台）
```

**用户体验**：
- ✅ **即时响应**：编辑操作无感知延迟
- ✅ **流畅导航**：文件树、搜索结果瞬间加载
- ✅ **大文件友好**：内存投影支持百万级 Block

#### 2. 完整的历史与时间旅行

**机制**：
```rust
// 重放到任意时间点
fn replay_until(timestamp: DateTime) -> StateProjector {
    let events = event_store.get_events_before(timestamp);
    let mut state = StateProjector::new();
    state.replay(events);
    state
}

// 查看特定 Block 的演化历史
fn get_block_history(block_id: &str) -> Vec<Event> {
    events.iter()
        .filter(|e| e.entity == block_id)
        .collect()
}
```

**用户体验**：
- ✅ **无限撤销**：可以回到任意历史版本
- ✅ **审计友好**：查看"谁在何时修改了什么"
- ✅ **冲突解决**：并排查看冲突的两个版本

#### 3. 灵活的组织与发现

**双维度视图**：

```
UI 提供三种视图：

1. 文件树视图（文件系统维度）
   Root
   ├─ research/
   │  └─ quantum.md
   └─ notes/
      └─ ideas.md

2. 知识图谱视图（语义维度）
   quantum.md
   └─ links → [ideas.md, neural-networks.md]

3. 我的文件视图（权限维度）
   所有我 own 或有 grant 的 Blocks
   - quantum.md (owner: alice)
   - shared-doc.md (grant: read)
```

**用户体验**：
- ✅ **多路径访问**：同一内容可以通过不同路径发现
- ✅ **灵活重组**：移动文件不破坏语义关联
- ✅ **权限透明**：清晰知道自己能访问什么

#### 4. 安全的协作与权限

**场景：项目协作**

```
Alice (项目负责人) 的操作：

1. 创建项目 Directory
   Event: alice/core.create → project_dir_1

2. 导入代码文件
   Event: alice/directory.import → [code_block_1, code_block_2]

3. 授予 Bob 特定权限
   Event: alice/core.grant → (bob, markdown.read, code_block_1)
   Event: alice/core.grant → (bob, code.write, code_block_2)

Bob 的视图：
- ✅ 可以读取 code_block_1（有 read grant）
- ✅ 可以修改 code_block_2（有 write grant）
- ❌ 无法访问 project_dir_1（无 directory.read grant）
- ✅ 可以通过搜索发现 code_block_1（权限独立于路径）
```

**用户体验**：
- ✅ **细粒度控制**：Block 级别的权限
- ✅ **安全隔离**：无权限的内容完全不可见
- ✅ **灵活授权**：可以跨文件夹授权特定文件

#### 5. 存证与不可抵赖性

**机制**：
```rust
Event {
    event_id: "uuid-123",
    entity: "contract-block",
    attribute: "alice/markdown.write",  // ⭐ 明确记录操作者
    value: { contents: { markdown: "合同条款..." } },
    timestamp: {"alice": 15, "bob": 8}
}

// Event Log 不可篡改
// - SQLite Write-Ahead Log (WAL)
// - 可选：数字签名
// - 可选：区块链锚定
```

**应用场景**：
- ✅ **法律文档**：每次修改都有完整记录
- ✅ **学术诚信**：论文编辑历史可追溯
- ✅ **合规审计**：满足 GDPR、SOX 等要求

---

## 5. 实施案例

### 5.1 修复 Directory Delete 的权限漏洞

**问题**（旧实现）：
```rust
// ❌ directory.delete 递归删除子 Blocks，绕过权限检查
if entry["type"] == "file" {
    events.push(create_event(
        child_id.to_string(),
        "core.delete",  // 未检查当前用户是否有删除权限！
        json!({}),
        &cmd.editor_id, 1
    ));
}
```

**修复**（新实现）：
```rust
#[capability(id = "directory.delete", target = "directory")]
fn handle_delete(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required")?;
    let payload: DirectoryDeletePayload = serde_json::from_value(cmd.payload.clone())?;

    let mut entries = get_entries(block)?;
    entries.remove(&payload.path);

    // ✅ 只删除 entry，不删除 Block
    let event = create_event(
        block.block_id.clone(),
        "directory.write",
        json!({ "contents": { "entries": entries } }),
        &cmd.editor_id, 1
    );

    Ok(vec![event])
}
```

**影响**：
- ✅ 安全：不能绕过 Block 的权限
- ✅ 协作友好：其他用户的引用不受影响
- ✅ 符合设计哲学：Directory 是索引，不是容器

### 5.2 修改 StateProjector 的删除语义

**问题**（旧实现）：
```rust
// state.rs:162-164
"core.delete" => {
    // ❌ 立即从内存中删除
    self.blocks.remove(&event.entity);
}
```

**修复**（新实现）：
```rust
"core.delete" => {
    // ✅ 标记删除，保留在内存中
    if let Some(block) = self.blocks.get_mut(&event.entity) {
        block.metadata.deleted = true;
    }
}
```

**影响**：
- ✅ 协作友好：其他用户仍能看到引用
- ✅ 可恢复：误删除可以撤销
- ✅ GC 清理：定期清理不可达的 deleted blocks

### 5.3 实现双维度 GC

**新增功能**（`actor.rs`）：
```rust
impl EngineActor {
    pub async fn gc_orphaned_blocks(&mut self) -> Result<usize> {
        let orphans = self.find_orphans();
        let count = orphans.len();

        for block_id in orphans {
            self.archive.remove_block(&block_id).await?;
        }

        Ok(count)
    }

    fn find_orphans(&self) -> Vec<String> {
        let roots = self.get_root_set();
        let reachable_fs = self.reachable_via_filesystem(&roots);
        let reachable_graph = self.reachable_via_children(&roots);

        let all_reachable: HashSet<_> = reachable_fs
            .union(&reachable_graph)
            .cloned()
            .collect();

        self.state.blocks.iter()
            .filter(|(id, block)| {
                block.metadata.deleted
                && !all_reachable.contains(*id)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
}
```

**影响**：
- ✅ 空间优化：自动清理不再需要的 Blocks
- ✅ 双维度保护：任一维度可达即保留
- ✅ 用户可控：手动或定期触发

---

## 6. 总结

### 核心设计原则

1. ✅ **扁平存储**：所有 Block 平等，无层级
2. ✅ **索引分离**：Directory.entries 是索引，不是容器
3. ✅ **关系正交**：文件系统（系统管理）和知识图谱（用户管理）分离
4. ✅ **权限独立**：每个 Block 的权限独立于引用关系
5. ✅ **软删除 + GC**：删除标记 + 双维度可达性分析

### 创新性

1. ✅ **去中心化协作**：本地优先 + 向量时钟 + P2P 同步
2. ✅ **多智能体原生支持**：AI Agent 作为一等公民
3. ✅ **双维度索引**：文件系统 + 知识图谱正交
4. ✅ **完整审计**：事件溯源提供不可篡改的历史

### 用户体验

1. ✅ **快速响应**：内存投影，读写操作 < 1ms
2. ✅ **灵活组织**：多路径访问，重组不破坏关联
3. ✅ **安全协作**：Block 级权限，细粒度控制
4. ✅ **存证友好**：完整历史，不可抵赖

### 最佳实践借鉴

| 系统 | 借鉴点 | Elfiee 的应用 |
|------|--------|--------------|
| Unix inode | 扁平 + 引用计数 | 扁平存储 + 软删除 |
| Git objects | 内容寻址 + 可达性 GC | 双维度可达性分析 |
| KeyKOS | CBAC + 对象空间 | Block 级权限 + 扁平存储 |
| Obsidian | 文件夹 + Wikilinks | Directory.entries + Block.children |

这个设计为 Elfiee 在**去中心化协作**和**人机协同**场景下提供了坚实的基础，既保证了数据安全和操作审计，又提供了灵活的组织方式和流畅的用户体验。

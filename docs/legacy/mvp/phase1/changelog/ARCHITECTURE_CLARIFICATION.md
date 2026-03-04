# 架构澄清：验证逻辑的正确边界

**日期**: 2025-12-29
**背景**: 在清理前端验证逻辑时，重新审视了哪些验证应该在前端，哪些应该在后端

---

## 核心原则确认

### ✅ Event Sourcing核心规则

**Event是唯一真相来源**：
- 所有**数据状态**和**业务规则**由后端（Event Store + StateProjector）管理
- 前端只负责**UI展示**和**用户交互**

**但是**：
- 前端可以有**UI层面的验证**（不是业务规则）
- 前端管理的**纯UI状态**（如localStorage）可以有自己的验证

---

## 验证逻辑分类

### 1. 后端验证（Backend Validation）

**定义**：涉及**Event Store数据完整性**的验证

**应该在后端的验证**：
- ✅ 路径安全性（`validate_virtual_path` - 防止路径遍历）
- ✅ Block类型推断（`infer_block_type` - 数据转换逻辑）
- ✅ Block/Editor是否存在（查询StateProjector）
- ✅ CBAC权限检查（`is_authorized`）
- ✅ 文件名非法字符检查（数据完整性）

**实现位置**：
```rust
// Capability Handler (纯函数)
fn handle(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    // ✅ 基于输入的验证
    validate_virtual_path(&payload.path)?;
    let block_type = infer_block_type(&extension);
    // ...
}

// Engine Actor (需要访问state的验证)
async fn process_command(&mut self, cmd: Command) -> Result<Vec<Event>, String> {
    // Step 3: CBAC授权
    if !self.state.is_authorized(...) { return Err(...); }

    // Step 3.5: 需要state的业务规则（如果需要的话）
    // 注：见下方"Editor重名"讨论

    // Step 4: 执行Handler
    let events = handler.handler(&cmd, block)?;
}
```

---

### 2. 前端验证（Frontend Validation）

**定义**：纯**UI交互**验证，不涉及后端数据完整性

**可以在前端的验证**：
- ✅ 空输入检查（禁用提交按钮）
- ✅ 输入trim（防止传递空白字符串）
- ✅ 前端管理的UI状态验证（如localStorage项目列表）

**实现位置**：
```typescript
// ✅ 合理的UI验证
const handleCreate = () => {
  const trimmed = name.trim()
  if (!trimmed) return  // 空输入直接返回

  // 调用后端
  await backend.create(trimmed)
}

// ✅ 前端UI状态验证（不是后端数据）
const isDuplicate = localStorageProjects.some(p => p.name === newName)
if (isDuplicate) {
  setError('Project name already exists')
}
```

---

## 关键案例分析

### 案例1：Editor重名检查 ❌ 不需要

**之前的误解**：
- ❌ 认为需要在Engine Actor添加重名检查
- ❌ 认为Editor.name必须唯一

**正确理解**：

```rust
pub struct Editor {
    pub editor_id: String,  // ← 这才是唯一标识（UUID）
    pub name: String,       // ← 仅用于UI展示，可以重复
    pub editor_type: EditorType,
}
```

**Event系统只认editor_id**：
```rust
// Event的attribute格式
attribute: "{editor_id}/{cap_id}"  // 用ID不用name

// Vector Clock冲突检测
timestamp: HashMap<String, u64>  // Key是editor_id

// 授权检查
is_authorized(editor_id: &str, ...)  // 参数是ID
```

**离线协作场景**（重名完全可行）：
```
设备A:
  创建 Editor { id: "uuid-1", name: "Bob" }
  Event: { timestamp: {"uuid-1": 1} }

设备B:
  创建 Editor { id: "uuid-2", name: "Bob" }  ← 同名！
  Event: { timestamp: {"uuid-2": 1} }

合并后:
  两个"Bob"并存，ID不同，完全没问题 ✅
  Vector Clock: {"uuid-1": 1, "uuid-2": 1}
```

**UI展示问题由前端处理**：
```typescript
// 前端自行处理重名显示
const displayName = editors.filter(e => e.name === editor.name).length > 1
  ? `${editor.name} (${editor.editor_id.slice(0, 8)}...)`
  : editor.name
```

**结论**：
- ✅ **后端不需要Editor重名检查**
- ✅ **Editor.name可以重复**
- ✅ 前端AddCollaboratorDialog的重复检查已正确删除

---

### 案例2：Directory Block重名自动加后缀 ✅ 前端处理

**当前实现**（前端）：
```typescript
// FilePanel.tsx:246-252
const existingNames = linkedRepos.map((r) => r.name)
let uniqueName = itemName
let counter = 1
while (existingNames.includes(uniqueName)) {
  uniqueName = `${itemName} (${counter++})`  // "Name (1)", "Name (2)"
}
```

**为什么在前端？**
1. 这是**UI层面的用户体验**优化
2. 后端创建Directory Block时只需要一个name，不关心是否重复
3. 不同的Directory Block可以同名（它们有不同的block_id）
4. 自动加后缀是**UI决策**，不是数据完整性要求

**重要**：
- 多次导入同一个目录是允许的
- 每次导入创建新的Directory Block（不同block_id）
- 后端不需要检查"是否已导入过"

**结论**：
- ✅ **前端保留当前实现**（自动加后缀）
- ✅ **后端不需要重名检查**

---

### 案例3：项目名称重复检查 ✅ 前端保留

**之前的误解**：
- ❌ 认为项目名称是后端数据，应该删除前端检查

**正确理解**：

**"项目"是前端UI概念**：
```typescript
// 前端localStorage存储
interface Project {
  name: string,      // ← 前端管理的显示名称
  path: string,      // ← .elf文件路径（后端关心的）
  lastOpened: Date
}
```

**后端只管理.elf文件路径**：
```rust
// Tauri Commands
fn open_file(path: String) -> FileId  // 只要路径
fn save_file(file_id: String)         // 不关心"项目名"
```

**所以**：
- "项目名称"存储在前端localStorage
- 后端根本不知道"项目名称"这个概念
- 前端检查项目名重复是**合理的UI验证**

**结论**：
- ✅ **恢复CreateProjectModal的重名检查**
- ✅ **恢复ImportProjectModal的重名检查**
- 这是前端管理自己的UI状态，不违反"Event是唯一真相来源"

---

## 最终修改总结

### Backend修改（1处）

✅ `editor_delete.rs` - 移除冗余payload
```diff
- serde_json::json!({ "deleted": true })
+ serde_json::json!({})
```

### Frontend修改（3处删除 + 2处恢复）

**删除的验证**（这些应该由后端处理）：
1. ✅ `FilePanel.tsx` - 删除`validateFilename()`（后端已有）
2. ✅ `FilePanel.tsx` - 删除`inferBlockType()`（后端已有）
3. ✅ `AddCollaboratorDialog.tsx` - 删除Editor重名检查（不需要）

**恢复的验证**（这些是合理的前端UI验证）：
4. ✅ `CreateProjectModal.tsx` - **恢复**项目名称重复检查（前端UI状态）
5. ✅ `ImportProjectModal.tsx` - **恢复**项目名称重复检查（前端UI状态）

**保留的处理**（合理的UI逻辑）：
6. ✅ `FilePanel.tsx` - 保留Directory Block自动加后缀（UI体验）
7. ✅ `CreateEntryDialog.tsx` - 保留trim处理（UI输入清理）
8. ✅ `VfsTree.tsx` - 保留trim处理（UI输入清理）

---

## 架构决策记录

### 决策1：Capability Handler保持纯函数

**规则**：Handler不应该访问全局state

```rust
// ✅ 正确：Handler是纯函数
fn handle(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>>

// ❌ 错误：Handler访问全局state
fn handle(cmd: &Command, block: Option<&Block>, state: &StateProjector) -> ...
```

**原因**：
- 保持可测试性
- 避免副作用
- 清晰的职责边界

### 决策2：需要state的验证放在Engine Actor

**如果某个验证需要访问StateProjector**：

```rust
// Engine Actor: process_command()
// Step 3.5: 需要state的业务规则验证（如果需要的话）
if cmd.cap_id == "some.capability" {
    // 访问 self.state 进行检查
    if self.state.some_check() {
        return Err("Validation failed");
    }
}
```

**但注意**：
- 目前我们确认**不需要**Editor重名检查
- 未来如果有其他需要state的验证，放在这里

### 决策3：前端验证边界清晰

**前端可以验证**：
- ✅ UI交互（空输入、格式提示）
- ✅ 前端管理的状态（localStorage项目列表）
- ✅ 用户体验优化（自动加后缀）

**前端不应该验证**：
- ❌ 后端数据完整性（路径安全、文件名非法字符）
- ❌ 业务规则（Block是否存在、权限检查）
- ❌ 数据转换（类型推断、格式化）

---

## 测试建议

### 前端测试场景

1. **文件创建**：
   - 输入非法路径 `../../../etc/passwd`
   - 预期：后端返回"Invalid path (traversal forbidden)"

2. **文件重命名**：
   - 输入非法字符 `test<>.md`
   - 预期：后端返回"Filename contains illegal characters"

3. **创建Editor**：
   - 创建两个同名的Editor "Alice"
   - 预期：✅ 成功创建，两个不同的editor_id
   - 前端UI显示：`Alice (uuid-1...)` 和 `Alice (uuid-2...)`

4. **导入Directory**：
   - 多次导入同一个目录
   - 预期：✅ 创建多个Directory Block
   - 前端自动命名："my-repo", "my-repo (1)", "my-repo (2)"

5. **创建项目**：
   - 输入已存在的项目名
   - 预期：前端显示"Project name already exists"（localStorage检查）

### 后端测试（已通过）

```bash
cd src-tauri && cargo test
# 所有测试通过 ✅
```

---

## 参考文档

- `docs/mvp/changelog/COMPREHENSIVE_CODE_REVIEW.md` - 完整代码审查
- `docs/mvp/changelog/FRONTEND_CLEANUP_SUMMARY.md` - 前端清理总结
- `docs/concepts/ARCHITECTURE_OVERVIEW.md` - 架构概述

---

**撰写**: Claude Sonnet 4.5
**审核**: 用户确认
**最终决策日期**: 2025-12-29

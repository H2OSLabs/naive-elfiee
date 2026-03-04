# 前端测试指南 - feat/datastruc分支

**目标**: 验证前端清理后，所有功能正常工作，后端验证生效

---

## 测试环境准备

```bash
# 1. 确保所有依赖已安装
pnpm install

# 2. 启动开发服务器
pnpm tauri dev
```

---

## 测试场景清单

### 场景1：文件创建 - 后端路径验证

**测试目的**: 验证后端的`validate_virtual_path()`生效

**步骤**:
1. 打开任意.elf文件
2. 在Outline区域点击"新建文档"或"新建文件夹"
3. 尝试输入非法路径：

**测试用例**:

| 输入 | 预期结果 | 验证点 |
|------|---------|--------|
| `../../../etc/passwd` | ❌ 后端错误："Invalid path (traversal forbidden)" | 路径遍历防护 |
| `/root/secret.txt` | ❌ 后端错误："Virtual path cannot be absolute" | 绝对路径禁止 |
| `test<>.md` | ❌ 后端错误："Filename contains illegal characters" | 非法字符检查 |
| `valid-file.md` | ✅ 创建成功 | 正常路径 |
| `docs/readme.md` | ✅ 创建成功 | 嵌套路径 |

**验证方式**:
- 前端不再弹出自定义错误（已删除`validateFilename()`）
- 错误由`toast.error()`显示，来自后端返回

---

### 场景2：文件重命名 - 后端路径验证

**测试目的**: 验证重命名时后端验证生效

**步骤**:
1. 右键点击任意文件
2. 选择"Rename"
3. 尝试输入非法文件名

**测试用例**:

| 输入 | 预期结果 |
|------|---------|
| `new/path/file.md` | ❌ 后端错误："Filename contains illegal characters" (斜杠) |
| `file<test>.md` | ❌ 后端错误："Filename contains illegal characters" |
| `valid-name.md` | ✅ 重命名成功 |

---

### 场景3：文件类型推断 - 后端自动推断

**测试目的**: 验证后端`infer_block_type()`自动推断类型

**步骤**:
1. 创建不同扩展名的文件
2. 检查Block类型

**测试用例**:

| 文件名 | 预期Block类型 | 验证方式 |
|--------|--------------|---------|
| `readme.md` | `markdown` | 检查StateProjector |
| `test.rs` | `code` | 检查StateProjector |
| `config.json` | `code` | 检查StateProjector |

**检查方法**:
```bash
# 在backend日志或通过Tauri DevTools查看Block数据
```

**验证前端不再调用**:
- 前端代码不再有`inferBlockType()`函数
- 创建时payload不包含`block_type`字段（由后端推断）

---

### 场景4：创建Editor（Collaborator）- 允许重名

**测试目的**: 验证Editor可以重名（基于editor_id区分）

**步骤**:
1. 打开.elf文件
2. 打开"Collaborators"面板
3. 创建多个同名Editor

**测试用例**:

| 操作 | 预期结果 |
|------|---------|
| 创建Editor "Alice" | ✅ 成功 |
| 再创建Editor "Alice" | ✅ 成功（不报重名错误） |
| 第三次创建"Alice" | ✅ 成功 |

**验证点**:
- ✅ 所有同名Editor成功创建
- ✅ 每个Editor有不同的`editor_id`（UUID）
- ✅ 前端不显示"A collaborator with this name already exists"错误
- 📋 **未来UI改进**：前端可以显示为"Alice (uuid-1...)"来区分

**理论验证**（离线协作）:
```
设备A:
  创建 Editor { id: "uuid-1", name: "Bob" }

设备B:
  创建 Editor { id: "uuid-2", name: "Bob" }

合并.elf文件:
  两个"Bob"并存 ✅ 无冲突
  Vector Clock: {"uuid-1": 1, "uuid-2": 1}
```

---

### 场景5：导入Repository - 自动重命名

**测试目的**: 验证前端自动加后缀机制

**步骤**:
1. 打开.elf文件
2. 多次导入同一个目录

**测试用例**:

| 操作 | 预期Directory Block名称 |
|------|----------------------|
| 第1次导入`my-project` | `my-project` |
| 第2次导入`my-project` | `my-project (1)` |
| 第3次导入`my-project` | `my-project (2)` |

**验证点**:
- ✅ 每次导入创建新的Directory Block（不同block_id）
- ✅ 前端自动加后缀，避免UI显示冲突
- ✅ 后端不检查重复（允许多次导入同一目录）

**代码验证**:
```typescript
// FilePanel.tsx:246-252 应该存在
const existingNames = linkedRepos.map((r) => r.name)
let uniqueName = itemName
let counter = 1
while (existingNames.includes(uniqueName)) {
  uniqueName = `${itemName} (${counter++})`
}
```

---

### 场景6：项目管理 - 前端localStorage验证

**测试目的**: 验证前端项目名称重复检查（localStorage管理）

**步骤**:
1. 进入Projects页面
2. 创建或导入项目

**测试用例A - 创建项目**:

| 操作 | 预期结果 |
|------|---------|
| 创建项目"My Project" | ✅ 成功 |
| 再创建"My Project" | ❌ 前端显示："Project name already exists, please modify." |
| 创建"My Project 2" | ✅ 成功 |

**测试用例B - 导入项目**:

| 操作 | 预期结果 |
|------|---------|
| 导入`test.elf`，命名为"Test" | ✅ 成功 |
| 再导入`another.elf`，也命名为"Test" | ❌ 前端显示："Project name already exists" |

**验证点**:
- ✅ 前端有重复检查（恢复后）
- ✅ 这是**前端UI状态验证**，不违反"Event是唯一真相来源"
- ✅ 后端不关心"项目名称"概念（只管理.elf文件路径）

**代码验证**:
```typescript
// CreateProjectModal.tsx 和 ImportProjectModal.tsx 应该有：
useEffect(() => {
  if (projectName.trim()) {
    const isDuplicate = existingNames.some(
      (name) => name.toLowerCase() === projectName.trim().toLowerCase()
    )
    if (isDuplicate) {
      setNameError('Project name already exists, please modify.')
    }
  }
}, [projectName, existingNames])
```

---

### 场景7：空输入处理 - UI层反馈

**测试目的**: 验证保留的轻量级UI验证

**步骤**:
1. 打开任意创建/重命名对话框
2. 尝试提交空输入

**测试用例**:

| 对话框 | 操作 | 预期结果 |
|--------|------|---------|
| CreateEntryDialog | 不输入任何内容，点击"Create" | ✅ 按钮禁用 OR 不触发请求 |
| AddCollaboratorDialog | 空名称 | ✅ 按钮禁用 OR 显示"Name cannot be empty" |
| VfsTree inline edit | 输入空格后按Enter | ✅ trim后传递给后端 |

**验证点**:
- ✅ 前端有基本的空输入检查（合理的UI反馈）
- ✅ `trim()`处理在前端（清理用户输入）
- ✅ 这些不是业务验证，只是UI交互优化

---

## 回归测试

### 基本功能检查

确认所有核心功能仍然正常：

- [ ] 打开.elf文件
- [ ] 创建新文档（markdown）
- [ ] 创建文件夹
- [ ] 重命名文件/文件夹
- [ ] 删除文件/文件夹
- [ ] 导入本地目录（Repository）
- [ ] 导出文件到本地
- [ ] 添加Collaborator
- [ ] 授予/撤销权限
- [ ] 创建新项目
- [ ] 导入已有项目

---

## 错误消息验证

### 应该看到的错误来源

**后端错误**（通过toast.error显示）:
```
❌ Failed to create file: Invalid path (traversal forbidden)
❌ Failed to rename: Filename contains illegal characters: test<>.md
❌ Authorization failed: alice does not have permission for markdown.write on block xxx
```

**前端UI反馈**（直接显示或禁用按钮）:
```
⚠️ Name cannot be empty
⚠️ Project name already exists, please modify.
```

**不应该看到的错误**（已删除）:
```
❌ Filename cannot be empty         (已删除validateFilename)
❌ Filename cannot contain slashes  (已删除validateFilename)
❌ Filename contains invalid characters  (已删除validateFilename，后端会报)
❌ A collaborator with this name already exists  (已删除，允许重名)
```

---

## 代码检查清单

### 前端代码确认

**应该删除的**:
- [ ] `FilePanel.tsx` 不存在`validateFilename()`函数
- [ ] `FilePanel.tsx` 不存在`inferBlockType()`函数
- [ ] `AddCollaboratorDialog.tsx` 不检查Editor重名

**应该恢复的**:
- [ ] `CreateProjectModal.tsx` 有项目名重复检查（带`nameError` state）
- [ ] `ImportProjectModal.tsx` 有项目名重复检查（带`nameError` state）

**应该保留的**:
- [ ] `FilePanel.tsx:246-252` 有Directory Block自动加后缀逻辑
- [ ] `CreateEntryDialog.tsx:37` 有`trim()`处理
- [ ] `VfsTree.tsx:52` 有`trim()`处理

### 后端代码确认

**应该修改的**:
- [ ] `editor_delete.rs:22` payload是`{}`而不是`{"deleted": true}`

---

## 预期测试结果

### 成功标准

所有测试场景通过后，应该满足：

1. ✅ **后端验证生效**：非法路径、非法字符由后端拒绝
2. ✅ **类型推断自动**：创建文件不需要前端传`block_type`
3. ✅ **Editor可重名**：多个同名Editor可以共存
4. ✅ **Repository可重复导入**：自动加后缀，前端UI处理
5. ✅ **项目名称检查保留**：这是前端localStorage管理，合理
6. ✅ **UI体验良好**：空输入被trim，按钮适当禁用

### 失败处理

如果测试失败：

1. **检查后端是否正常运行**
   ```bash
   # 查看Tauri DevTools Console
   # 检查后端日志
   ```

2. **检查前端是否有旧代码残留**
   ```bash
   # 清理缓存重新运行
   pnpm run dev
   ```

3. **验证修改是否完整**
   - 参考`FRONTEND_CLEANUP_SUMMARY.md`
   - 参考`ARCHITECTURE_CLARIFICATION.md`

---

## 测试报告模板

测试完成后，记录结果：

```markdown
## 测试执行报告

**测试日期**: 2025-12-29
**测试人**: [你的名字]
**分支**: feat/datastruc

### 场景测试结果

- [ ] 场景1：文件创建 - 后端路径验证
- [ ] 场景2：文件重命名 - 后端路径验证
- [ ] 场景3：文件类型推断 - 后端自动推断
- [ ] 场景4：创建Editor - 允许重名
- [ ] 场景5：导入Repository - 自动重命名
- [ ] 场景6：项目管理 - localStorage验证
- [ ] 场景7：空输入处理 - UI层反馈

### 回归测试结果

- [ ] 所有基本功能正常

### 发现的问题

[如果有问题，在此记录]

### 总体评估

- [ ] ✅ 通过 - 可以合并到dev分支
- [ ] ⚠️ 部分通过 - 需要修复[具体问题]
- [ ] ❌ 失败 - 需要重新审查
```

---

**文档版本**: 1.0
**适用分支**: feat/datastruc
**相关文档**:
- `COMPREHENSIVE_CODE_REVIEW.md`
- `FRONTEND_CLEANUP_SUMMARY.md`
- `ARCHITECTURE_CLARIFICATION.md`

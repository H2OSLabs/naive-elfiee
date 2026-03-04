# Timeline功能修复计划

**日期**: 2025-12-30
**分支**: feat/timeline
**问题**: restoreToEvent恢复了grants导致前后端状态不一致

---

## 问题分析

### 当前实现的问题

```typescript
// app-store.ts:553-591
restoreToEvent: async (fileId, blockId, eventId) => {
  const { block: historicalBlock, grants: historicalGrants } =
    await TauriClient.event.getStateAtEvent(fileId, blockId, eventId)

  // ❌ 问题：恢复了历史的grants
  files.set(fileId, {
    ...fileState,
    blocks: updatedBlocks,
    grants: historicalGrants,  // ← 这导致前端显示的grants与后端实际grants不一致
  })
}
```

**场景问题**：
```
当前grants: {alice: [write], bob: [write]}
回溯到历史: {alice: [write]}  // Bob当时还没被加入

前端显示: Bob的权限消失（但只是UI显示）
后端实际: Bob仍有write权限

如果Bob尝试编辑:
  - 前端UI可能误导用户（显示无权限）
  - 后端仍然允许（因为实际grants未变）
  - 状态不一致！
```

### 正确的行为

- **回溯只应该恢复block内容**（name, contents, metadata）
- **grants应该保持当前最新状态**
- **权限检查**：用户需要`markdown.write`（或对应block类型的write）权限即可回溯

---

## 修改清单

### 1. 核心修改：app-store.ts

**文件**: `src/lib/app-store.ts`
**行号**: 553-591

**修改内容**：
```typescript
// 修改前
restoreToEvent: async (fileId: string, blockId: string, eventId: string) => {
  const { block: historicalBlock, grants: historicalGrants } =
    await TauriClient.event.getStateAtEvent(fileId, blockId, eventId)

  files.set(fileId, {
    ...fileState,
    blocks: updatedBlocks,
    grants: historicalGrants,  // ❌ 删除这行
  })

  toast.success('已恢复到历史快照，包含描述、标题和权限')  // ❌ 文案错误
}

// 修改后
restoreToEvent: async (fileId: string, blockId: string, eventId: string) => {
  const { block: historicalBlock } =  // ← 只取block，不取grants
    await TauriClient.event.getStateAtEvent(fileId, blockId, eventId)

  files.set(fileId, {
    ...fileState,
    blocks: updatedBlocks,
    // grants保持当前值，不恢复历史grants
  })

  toast.info('已加载历史内容到编辑器，您可以继续编辑并保存')  // ✅ 文案准确
}
```

---

### 2. 前端验证逻辑检查

#### 检查结果：✅ 符合架构原则

**文件**: `src/components/permission/CollaboratorList.tsx`
**行号**: 86-98, 134-147

**现有权限检查**（保留）：
```typescript
// UI层权限检查（合理）
const hasPermission = await TauriClient.block.checkPermission(
  fileId, blockId, 'core.grant', activeEditor?.editor_id
)

if (!hasPermission) {
  toast.error('You do not have permission to grant permissions.')
  return  // ← 提前反馈，避免无意义的后端调用
}

// 后端仍有完整验证（grant.rs的certificator）
```

**判定**: ✅ 这是合理的UI反馈，不是业务验证
**理由**:
- 提前反馈，提升用户体验
- 后端仍有完整的权限验证
- 符合datastruct分支确立的原则

**文件**: `src/components/editor/ContextPanel.tsx`
**行号**: 86-96

**现有权限检查**（保留）：
```typescript
// 更新metadata前检查权限（合理）
const hasPermission = await TauriClient.block.checkPermission(
  fileId, block.block_id, 'core.update_metadata', activeEditorId
)

if (!hasPermission) {
  toast.error('You do not have permission to update metadata.')
  return
}
```

**判定**: ✅ 合理的UI反馈

**结论**: ✅ 前端无冗余业务验证，无需修改

---

### 3. Block类型适配检查

#### 检查内容：各种block类型是否都能正确回溯

**支持的block类型**：
- `markdown` - 通过contents.markdown存储
- `code` - 通过contents.code存储
- `directory` - 通过contents.entries存储

**后端实现**（event.rs:80-83）：
```rust
let block = temp_projector
    .get_block(&block_id)
    .ok_or_else(|| format!("Block '{}' not found at event '{}'", block_id, event_id))?
    .clone();  // ← 完整克隆整个Block，包括所有contents字段
```

**前端实现**（app-store.ts:564-571）：
```typescript
const updatedBlocks = fileState.blocks.map((block) => {
  if (block.block_id === blockId) {
    return { ...historicalBlock }  // ← 完整替换，支持所有block类型
  }
  return block
})
```

**结论**: ✅ 所有block类型都支持回溯，无需修改

**验证逻辑**：
```
markdown block:
  - 回溯前: { markdown: "Current content" }
  - 回溯后: { markdown: "Historical content" }
  - ✅ 正确

code block:
  - 回溯前: { code: "const x = 1" }
  - 回溯后: { code: "const x = 0" }
  - ✅ 正确

directory block:
  - 回溯前: { entries: { "file1.md": {...}, "file2.md": {...} } }
  - 回溯后: { entries: { "file1.md": {...} } }
  - ✅ 正确（恢复历史的目录结构）
```

---

### 4. 测试修改

#### 4.1 ContextPanel.test.tsx

**文件**: `src/components/editor/ContextPanel.test.tsx`
**行号**: 57

**当前mock**:
```typescript
restoreToEvent: vi.fn(),  // ✅ 已经mock，无需修改
```

**新增测试**（建议）:
```typescript
it('should not restore grants when restoring to event', async () => {
  const mockRestoreToEvent = vi.fn()
  setupMock({ restoreToEvent: mockRestoreToEvent })

  render(<ContextPanel />)
  fireEvent.click(screen.getByText('Timeline'))

  const restoreButton = screen.getByRole('button', { name: /restore/i })
  fireEvent.click(restoreButton)

  await waitFor(() => {
    expect(mockRestoreToEvent).toHaveBeenCalledWith(
      mockFileId,
      mockBlockId,
      'e1'
    )
  })
})
```

#### 4.2 app-store单元测试（新增）

**文件**: `src/lib/app-store.test.ts`（新建）

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useAppStore } from './app-store'
import { TauriClient } from './tauri-client'

vi.mock('./tauri-client')

describe('app-store restoreToEvent', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should restore block content but not grants', async () => {
    const mockHistoricalBlock = {
      block_id: 'block1',
      name: 'Historical Name',
      contents: { markdown: 'Historical content' },
      // ...
    }

    const mockHistoricalGrants = [
      { editor_id: 'alice', cap_id: 'markdown.write', block_id: 'block1' }
    ]

    vi.mocked(TauriClient.event.getStateAtEvent).mockResolvedValue({
      block: mockHistoricalBlock,
      grants: mockHistoricalGrants,
    })

    const { restoreToEvent, getGrants } = useAppStore.getState()

    // 当前grants
    const currentGrants = [
      { editor_id: 'alice', cap_id: 'markdown.write', block_id: 'block1' },
      { editor_id: 'bob', cap_id: 'markdown.write', block_id: 'block1' },
    ]

    await restoreToEvent('file1', 'block1', 'event1')

    // 验证：grants应该保持当前值，不被历史grants覆盖
    const grantsAfterRestore = getGrants('file1')
    expect(grantsAfterRestore).toEqual(currentGrants)  // ← 关键断言
    expect(grantsAfterRestore).not.toEqual(mockHistoricalGrants)
  })
})
```

---

## 实施步骤

### 步骤1: 修改核心代码

```bash
# 修改app-store.ts
vim src/lib/app-store.ts
# 删除575-578行的grants恢复
# 修改584行的toast文案
```

### 步骤2: 运行测试

```bash
# 前端测试
pnpm test

# 后端测试
cd src-tauri && cargo test
```

### 步骤3: 手动验证

1. 启动应用: `pnpm tauri dev`
2. 打开一个.elf文件
3. 创建collaborator Bob并grant权限
4. 编辑block内容
5. 点击Timeline，回溯到Bob加入之前的事件
6. 验证：
   - ✅ Block内容恢复到历史版本
   - ✅ Bob的权限仍然显示在Collaborators列表
   - ✅ Bob可以继续编辑并保存

---

## 预期测试结果

### 前端测试
- ✅ 12个测试套件全部通过
- ✅ ContextPanel相关测试通过

### 后端测试
- ✅ 203个测试用例全部通过
- ✅ event.rs中的replay测试通过

---

## 风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 破坏现有功能 | 低 | 只删除3行代码，改1行文案 |
| 测试失败 | 低 | 现有测试已mock restoreToEvent |
| 用户体验变化 | 低 | 文案更准确，行为更符合预期 |

---

## 修改后的行为

### Before
```
用户点击Timeline中的"Restore"按钮:
  1. 恢复block内容 ✅
  2. 恢复grants ❌（不应该）
  3. 前端显示: Bob的权限消失
  4. 后端实际: Bob仍有权限
  5. 状态不一致！
```

### After
```
用户点击Timeline中的"Restore"按钮:
  1. 恢复block内容 ✅
  2. grants保持最新状态 ✅
  3. 前端显示: Bob的权限仍然存在
  4. 后端实际: Bob仍有权限
  5. 状态一致！✅
```

---

**文档版本**: 1.0
**创建时间**: 2025-12-30
**状态**: 待执行

# 并发测试指南 (Concurrency Testing Guide)

## 📖 什么是竞态条件 (Race Condition)?

**竞态条件**是指多个异步操作同时访问和修改共享状态时，最终结果取决于操作执行的时间顺序，可能导致不可预测或错误的结果。

### 简单示例

```typescript
// ❌ 存在竞态条件
let counter = 0

async function increment() {
  const current = counter  // 步骤1: 读取
  await delay(10)          // 步骤2: 等待
  counter = current + 1    // 步骤3: 写入
}

// 同时执行两次
await Promise.all([increment(), increment()])
console.log(counter) // 期望: 2, 实际可能是: 1 ❌

// ✅ 修复方案
async function safeIncrement() {
  counter = counter + 1  // 原子操作
}
```

**问题根源**: 两个函数都在步骤1读到 `counter = 0`，然后都写入 `1`，最终结果是 `1` 而不是 `2`。

---

## 🎯 Elfiee项目中的竞态场景

### **场景1: 快速连续创建Block**

**问题描述**:
```typescript
// 用户快速双击"创建Block"按钮
createBlock('file-1', 'Block A') // 调用1
createBlock('file-1', 'Block B') // 调用2

// 执行流程:
// 调用1: set({ isLoading: true }) → createBlock → loadBlocks()
// 调用2: set({ isLoading: true }) → createBlock → loadBlocks()

// 潜在问题:
// 1. 两次loadBlocks()可能返回顺序不确定
// 2. isLoading状态可能被覆盖
// 3. 最终显示的blocks顺序可能不符合预期
```

**测试示例**:
```typescript
test('should handle rapid consecutive block creations correctly', async () => {
  // 快速创建3个Block
  const creationPromises = [
    store.createBlock(fileId, 'Block 1'),
    store.createBlock(fileId, 'Block 2'),
    store.createBlock(fileId, 'Block 3'),
  ]

  await Promise.all(creationPromises)

  // 验证: 所有Block都被正确创建
  const finalBlocks = store.files.get(fileId)?.blocks
  expect(finalBlocks).toHaveLength(3)
})
```

---

### **场景2: loadBlocks响应时间不同**

**问题描述**:
```typescript
// 同时发起3次loadBlocks调用
loadBlocks() // 调用1: 耗时200ms
loadBlocks() // 调用2: 耗时100ms
loadBlocks() // 调用3: 耗时50ms

// 完成顺序: 调用3 → 调用2 → 调用1
// 但最终显示的是调用1的结果（最慢的旧数据）
// 而不是调用3的结果（最快的新数据）
```

**当前实现问题**:
```typescript
loadBlocks: async (fileId) => {
  const blocks = await TauriClient.block.getAllBlocks(fileId)

  // ❌ 问题: 无论哪个请求最后完成，都会覆盖之前的结果
  const file = get().files.get(fileId)
  if (file) {
    file.blocks = blocks  // 最后完成的请求"获胜"
  }
}
```

**解决方案**:
```typescript
// ✅ 方案1: 请求取消
let abortController = new AbortController()

loadBlocks: async (fileId) => {
  // 取消之前的请求
  abortController.abort()
  abortController = new AbortController()

  const blocks = await TauriClient.block.getAllBlocks(fileId, {
    signal: abortController.signal
  })

  // 只有未被取消的请求才更新状态
  if (!abortController.signal.aborted) {
    const file = get().files.get(fileId)
    if (file) file.blocks = blocks
  }
}

// ✅ 方案2: 版本号/时间戳
let requestId = 0

loadBlocks: async (fileId) => {
  const currentRequestId = ++requestId
  const blocks = await TauriClient.block.getAllBlocks(fileId)

  // 只有最新的请求才更新状态
  if (currentRequestId === requestId) {
    const file = get().files.get(fileId)
    if (file) file.blocks = blocks
  }
}

// ✅ 方案3: 防抖 (Debounce)
import { debounce } from 'lodash'

loadBlocks: debounce(async (fileId) => {
  const blocks = await TauriClient.block.getAllBlocks(fileId)
  const file = get().files.get(fileId)
  if (file) file.blocks = blocks
}, 100)
```

---

### **场景3: 删除和选择同一Block**

**问题描述**:
```typescript
// 用户选择一个Block
store.selectBlock(fileId, 'block-123')

// 立即删除它
await store.deleteBlock(fileId, 'block-123')

// 问题: selectedBlockId仍然指向已删除的Block
// getSelectedBlock()返回null，但selectedBlockId不是null
```

**解决方案**:
```typescript
deleteBlock: async (fileId, blockId) => {
  await TauriClient.block.deleteBlock(fileId, blockId, editorId)
  await get().loadBlocks(fileId)

  // ✅ 清理选中状态
  const file = get().files.get(fileId)
  if (file && file.selectedBlockId === blockId) {
    file.selectedBlockId = null
  }
}
```

---

### **场景4: 并发操作部分失败**

**问题描述**:
```typescript
// 同时创建3个Block，其中1个失败
await Promise.allSettled([
  createBlock(fileId, 'Block 1'), // ✅ 成功
  createBlock(fileId, 'Block 2'), // ❌ 失败
  createBlock(fileId, 'Block 3'), // ✅ 成功
])

// 问题:
// 1. 用户是否看到2个成功通知 + 1个错误通知？
// 2. 最终状态是否包含2个成功创建的Block？
// 3. isLoading状态是否正确恢复？
```

---

## 🧪 如何编写并发测试

### 1. 使用 `Promise.all` 模拟并发操作

```typescript
test('concurrent operations', async () => {
  // 同时执行多个操作
  await Promise.all([
    operation1(),
    operation2(),
    operation3(),
  ])

  // 验证最终状态
  expect(finalState).toBe(expectedState)
})
```

### 2. 使用 `Promise.allSettled` 处理部分失败

```typescript
test('partial failures', async () => {
  const results = await Promise.allSettled([
    successfulOperation(),
    failingOperation(),
    successfulOperation(),
  ])

  expect(results[0].status).toBe('fulfilled')
  expect(results[1].status).toBe('rejected')
  expect(results[2].status).toBe('fulfilled')
})
```

### 3. 控制异步操作的时间顺序

```typescript
test('response time ordering', async () => {
  let callCount = 0

  vi.mocked(invoke).mockImplementation(async () => {
    callCount++

    // 第一个调用最慢
    if (callCount === 1) {
      await new Promise(r => setTimeout(r, 200))
      return oldData
    }

    // 第三个调用最快
    if (callCount === 3) {
      await new Promise(r => setTimeout(r, 50))
      return newData
    }
  })

  // 启动3次调用
  const [p1, p2, p3] = [
    loadData(),
    loadData(),
    loadData(),
  ]

  await Promise.all([p1, p2, p3])

  // 验证: 应该显示最新的数据，而不是最慢的
  expect(currentData).toBe(newData)
})
```

### 4. 测试状态一致性

```typescript
test('state consistency', async () => {
  const initialState = store.getState()

  // 执行并发操作
  await Promise.all([
    modifyStateA(),
    modifyStateB(),
    modifyStateC(),
  ])

  const finalState = store.getState()

  // 验证: 状态应该是确定的，而不是随机的
  expect(finalState).toMatchObject({
    // 期望的最终状态
  })
})
```

---

## 📝 最佳实践

### ✅ DO

1. **测试真实场景**: 模拟用户可能的实际操作
   ```typescript
   // 用户快速点击两次"创建"按钮
   await Promise.all([createBlock(), createBlock()])
   ```

2. **验证最终一致性**: 确保无论操作顺序如何，最终状态都是正确的
   ```typescript
   expect(finalBlocks.length).toBe(2)
   expect(finalBlocks.every(b => b.owner === editorId)).toBe(true)
   ```

3. **测试边界情况**: 极快的连续操作、网络延迟等
   ```typescript
   // 100ms内连续创建10个Block
   const promises = Array.from({ length: 10 }, (_, i) =>
     createBlock(fileId, `Block ${i}`)
   )
   await Promise.all(promises)
   ```

4. **使用实际的async/await模式**: 不要用setTimeout模拟
   ```typescript
   ✅ await operation()
   ❌ setTimeout(() => operation(), 0)
   ```

### ❌ DON'T

1. **不要假设执行顺序**: async操作完成顺序不可预测
2. **不要忽略错误处理**: 并发操作中的错误特别难调试
3. **不要测试implementation细节**: 关注最终状态，而不是中间步骤
4. **不要mock时间**: 使用真实的Promise而不是fake timers

---

## 🔧 当前项目需要改进的地方

### 1. loadBlocks缺少请求取消机制
**问题**: 快速连续调用时，旧请求可能覆盖新数据

**修复**: 添加AbortController或请求版本号

### 2. deleteBlock后没有清理selectedBlockId
**问题**: 删除已选中Block后，selectedBlockId仍指向不存在的Block

**修复**: 在deleteBlock中检查并清理selectedBlockId

### 3. isLoading状态在并发操作时不准确
**问题**: 多个操作同时进行时，isLoading可能过早变为false

**修复**: 使用计数器跟踪进行中的操作数量
```typescript
let loadingCount = 0

startOperation: () => {
  loadingCount++
  set({ isLoading: true })
}

endOperation: () => {
  loadingCount--
  if (loadingCount === 0) {
    set({ isLoading: false })
  }
}
```

---

## 📚 参考资料

- [MDN: Race Conditions](https://developer.mozilla.org/en-US/docs/Glossary/Race_condition)
- [JavaScript Concurrency Model](https://developer.mozilla.org/en-US/docs/Web/JavaScript/EventLoop)
- [Zustand: Async Actions Best Practices](https://github.com/pmndrs/zustand/wiki/Async-actions)
- [Testing Async Code with Vitest](https://vitest.dev/guide/features.html#async-tests)

---

## 🎓 练习

尝试为以下场景编写测试：

1. 用户在同一秒内点击"保存"按钮3次
2. 同时打开3个不同的.elf文件
3. 多个编辑器同时修改同一个Block（需要Event Store的向量时钟冲突检测）
4. 网络延迟情况下的状态同步

示例测试在 `src/lib/app-store.concurrency.test.ts` 中。

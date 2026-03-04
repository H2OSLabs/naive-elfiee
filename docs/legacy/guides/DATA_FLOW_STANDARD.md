# 数据流规范 (Data Flow Standard)

## 概述

本文档定义 Elfiee 前端组件与后端交互的标准数据流模式，确保代码的可维护性、可测试性和一致性。

## 标准数据流架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        Component Layer                           │
│  (React Components: EditorCanvas, Sidebar, FilePanel, etc.)     │
└─────────────────────────────────────────────────────────────────┘
                              ↓
                    useAppStore() hook
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                         State Layer                              │
│              app-store.ts (Zustand Store)                        │
│  • 管理应用状态 (files, blocks, editors)                         │
│  • 提供状态操作函数 (loadBlocks, updateBlock, etc.)              │
│  • 处理错误和 toast 通知                                          │
└─────────────────────────────────────────────────────────────────┘
                              ↓
                    调用 TauriClient 方法
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                         Client Layer                             │
│              tauri-client.ts (Wrapper)                           │
│  • 封装 bindings.commands 调用                                    │
│  • 处理 Result 类型解包                                           │
│  • 提供友好的 API 接口                                            │
└─────────────────────────────────────────────────────────────────┘
                              ↓
                    调用 bindings.commands
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                        Bindings Layer                            │
│            bindings.ts (Auto-generated)                          │
│  • Tauri Specta 自动生成                                          │
│  • 类型安全的 Rust-TypeScript 桥接                                │
└─────────────────────────────────────────────────────────────────┘
                              ↓
                         Tauri IPC
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                        Backend Layer                             │
│               Rust (Tauri Commands)                              │
│  • 权限验证 (CBAC)                                                │
│  • 业务逻辑处理                                                   │
│  • 数据库操作                                                     │
└─────────────────────────────────────────────────────────────────┘
```

## 核心原则

### 1. 单向数据流 (Unidirectional Data Flow)

**✅ 正确做法**:
```typescript
// Component
const BlockList = () => {
  const { blocks, loadBlocks } = useAppStore()

  useEffect(() => {
    loadBlocks(fileId)  // 通过 store action 获取数据
  }, [fileId])

  return <div>{blocks.map(...)}</div>
}
```

**❌ 错误做法**:
```typescript
// Component
import { TauriClient } from '@/lib/tauri-client'  // ❌ 组件不应直接导入 TauriClient

const BlockList = () => {
  const [blocks, setBlocks] = useState([])

  useEffect(() => {
    TauriClient.block.getAllBlocks(fileId)  // ❌ 绕过了 store
      .then(setBlocks)
  }, [fileId])
}
```

### 2. 状态集中管理 (Centralized State)

所有应用状态都应在 `app-store.ts` 中管理：
- ✅ `files` - 打开的文件状态
- ✅ `blocks` - Block 列表
- ✅ `editors` - Editor 列表
- ✅ `grants` - 权限列表
- ✅ `events` - 事件历史

**组件不应该维护后端数据的本地副本**。

### 3. 权限验证在后端 (Backend Authorization)

前端**不负责**权限验证逻辑，只负责：
1. 调用 store actions
2. 显示/隐藏 UI 元素（基于 store 中的数据）
3. 显示后端返回的错误信息

权限验证由后端 Rust 代码的 CBAC 系统处理。

## 数据流分类

### 读操作 (Read Operations)

**标准流程**:
```typescript
// 1. Store 定义 action
const loadBlocks = async (fileId: string) => {
  try {
    const blocks = await TauriClient.block.getAllBlocks(fileId, editorId)
    // 后端已经根据 editorId 过滤了权限
    set((state) => {
      const files = new Map(state.files)
      const fileState = files.get(fileId)
      if (fileState) {
        files.set(fileId, { ...fileState, blocks })
        return { files }
      }
    })
  } catch (error) {
    toast.error('Failed to load blocks')
  }
}

// 2. Component 使用 store
const blocks = useAppStore((state) => {
  const fileState = state.files.get(currentFileId)
  return fileState?.blocks || []
})
```

### 写操作 (Write Operations)

**标准流程**:
```typescript
// 1. Store 定义 action
const updateBlock = async (fileId: string, blockId: string, content: string) => {
  try {
    // TauriClient 调用后端 Command
    const events = await TauriClient.block.writeBlock(fileId, blockId, content)

    // 重新加载数据（因为后端是事件溯源，需要重放事件）
    await loadBlocks(fileId)

    toast.success('Block updated')
  } catch (error) {
    toast.error(error.message)
  }
}

// 2. Component 调用 store action
const { updateBlock } = useAppStore()
const handleSave = () => {
  updateBlock(fileId, blockId, content)
}
```

## 特殊情况处理

### 情况 1: 一次性配置数据

对于**不经常变化**的配置数据（如 systemEditorId），可以：

**方案 A (推荐)**: 在 store 初始化时加载
```typescript
// app-store.ts
interface AppStore {
  systemEditorId: string | null
  initializeConfig: () => Promise<void>
}

const useAppStore = create<AppStore>((set) => ({
  systemEditorId: null,

  initializeConfig: async () => {
    const systemEditorId = await TauriClient.file.getSystemEditorId()
    set({ systemEditorId })
  }
}))

// Component
const systemEditorId = useAppStore((state) => state.systemEditorId)
```

**方案 B**: 组件内直接调用（仅限一次性配置）
```typescript
// 仅限于不会改变的配置数据，且不需要在多个组件间共享
const [systemEditorId, setSystemEditorId] = useState<string | null>(null)

useEffect(() => {
  TauriClient.file.getSystemEditorIdFromConfig()
    .then(setSystemEditorId)
}, [])
```

### 情况 2: 时间线回溯 (Timeline Revert)

这是一个**特殊操作**，因为它需要：
1. 获取历史状态（读操作）
2. 更新当前状态（写操作到 store）

**正确做法**（已在 app-store.ts 实现）:
```typescript
// app-store.ts
restoreToEvent: async (fileId: string, blockId: string, eventId: string) => {
  try {
    // 1. 通过 TauriClient 获取历史状态
    const { block, grants } = await TauriClient.event.getStateAtEvent(
      fileId, blockId, eventId
    )

    // 2. 更新 store 中的状态
    const files = new Map(get().files)
    const fileState = files.get(fileId)
    if (fileState) {
      const updatedBlocks = fileState.blocks.map((b) =>
        b.block_id === blockId ? block : b
      )
      files.set(fileId, { ...fileState, blocks: updatedBlocks, grants })
      set({ files })
    }

    toast.success('Restored to historical snapshot')
  } catch (error) {
    toast.error('Failed to restore')
  }
}
```

## 规范检查清单

在编写新功能时，检查以下事项：

### ✅ DO (应该做)

1. **组件从 store 读取数据**
   ```typescript
   const blocks = useAppStore((state) => state.files.get(fileId)?.blocks)
   ```

2. **组件调用 store actions 修改数据**
   ```typescript
   const { updateBlock, deleteBlock } = useAppStore()
   ```

3. **Store actions 调用 TauriClient**
   ```typescript
   const blocks = await TauriClient.block.getAllBlocks(fileId)
   ```

4. **错误处理在 store action 中**
   ```typescript
   try {
     // ...
   } catch (error) {
     toast.error(error.message)
   }
   ```

5. **使用 Zustand selector 订阅状态变化**
   ```typescript
   const selectedBlock = useAppStore((state) => {
     const fileState = state.files.get(currentFileId)
     return fileState?.blocks.find((b) => b.block_id === selectedBlockId)
   })
   ```

### ❌ DON'T (不应该做)

1. **组件直接导入 TauriClient**（除非是一次性配置）
   ```typescript
   import { TauriClient } from '@/lib/tauri-client'  // ❌
   ```

2. **组件维护后端数据的本地副本**
   ```typescript
   const [blocks, setBlocks] = useState([])  // ❌ 应使用 store
   ```

3. **在组件中处理权限逻辑**
   ```typescript
   if (user.role === 'admin') {  // ❌ 权限应由后端处理
     // ...
   }
   ```

4. **绕过 store 直接修改数据**
   ```typescript
   await TauriClient.block.writeBlock(...)  // ❌ 应通过 store action
   ```

5. **使用 useEffect 依赖 getBlock 函数**
   ```typescript
   useEffect(() => {
     const block = getBlock(fileId, blockId)  // ❌ getBlock 是稳定引用
   }, [getBlock])  // 不会触发重新渲染
   ```

## 测试规范

### Store 测试
```typescript
// Mock TauriClient responses
setupCommandMocks({
  getAllBlocks: [mockBlock1, mockBlock2]
})

// Test store action
await useAppStore.getState().loadBlocks(fileId)

// Verify state update
expect(useAppStore.getState().files.get(fileId)?.blocks).toEqual([...])
```

### Component 测试
```typescript
// Mock useAppStore
vi.mock('@/lib/app-store', () => ({
  useAppStore: vi.fn()
}))

// Setup mock store state
vi.mocked(useAppStore).mockReturnValue({
  blocks: [mockBlock],
  loadBlocks: vi.fn()
})
```

## 现有代码迁移

### 需要重构的组件

以下组件直接使用 TauriClient，需要重构：

1. **Sidebar.tsx** - `getSystemEditorIdFromConfig()`
   - 建议：将 systemEditorId 移到 store

2. **EditorCanvas.tsx** - 同上

3. **FilePanel.tsx** - 检查是否有直接调用

4. **ContextPanel.tsx** - 检查是否有直接调用

5. **CollaboratorList.tsx** - 检查是否有直接调用

### 重构示例

**Before (直接调用)**:
```typescript
const [systemEditorId, setSystemEditorId] = useState<string | null>(null)

useEffect(() => {
  TauriClient.file.getSystemEditorIdFromConfig()
    .then(setSystemEditorId)
}, [])
```

**After (通过 store)**:
```typescript
// app-store.ts
interface AppStore {
  systemEditorId: string | null
  loadSystemEditorId: () => Promise<void>
}

// Component
const systemEditorId = useAppStore((state) => state.systemEditorId)

useEffect(() => {
  useAppStore.getState().loadSystemEditorId()
}, [])
```

## 数据流图示例

### 用户切换 Editor

```
User clicks editor in dropdown
         ↓
Component calls setActiveEditor(fileId, editorId)
         ↓
Store action: setActiveEditor
         ↓
TauriClient.editor.setActiveEditor(fileId, editorId)
         ↓
Backend validates permission & updates active_editor
         ↓
Store updates activeEditorId in state
         ↓
Store calls loadBlocks(fileId) to reload with new permissions
         ↓
Component re-renders with filtered blocks
```

### 保存 Block

```
User presses Ctrl+S
         ↓
Component calls handleSave()
         ↓
Store action: updateBlock(fileId, blockId, content)
         ↓
TauriClient.block.writeBlock(fileId, blockId, content)
         ↓
Backend: Authorization → Handler → Generate Event → Commit
         ↓
Backend returns events
         ↓
Store calls loadBlocks(fileId) to get updated state
         ↓
Store calls loadEvents(fileId) to refresh timeline
         ↓
Component re-renders with new content
```

## 总结

遵循此数据流规范可以确保：

1. **可测试性**: 可以轻松 mock store，无需 mock 复杂的组件内部逻辑
2. **可维护性**: 数据流清晰，易于追踪和调试
3. **权限安全**: 权限验证集中在后端，前端不能绕过
4. **状态一致性**: 单一数据源，避免状态不同步
5. **响应式更新**: Zustand 自动触发组件重新渲染

**核心原则**: Component → Store → TauriClient → Backend，单向数据流，状态集中管理。

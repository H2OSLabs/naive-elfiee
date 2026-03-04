# Directory Extension 前端设计文档

> **版本**: v2.0 (2025-12-24)
> **状态**: 待实现
> **目标**: 基于 Directory Extension 后端实现完整的文件树 UI（Outline + Linked Repos）

---

## 1. 核心概念与架构

### 1.1 Block Types（后端实际类型）

**当前存在的 Block Types**：
- `"markdown"` - Markdown 文档，内容存储在 `contents.markdown`
- `"code"` - 代码文件（**即将**添加 extension），内容存储在 `contents.text`
- `"directory"` - 虚拟文件系统目录，内容是 `contents.entries`（扁平Map）
- `"terminal"` - 终端会话

**⚠️ 易混淆点**：
- ❌ **不存在** `"file"` block type
- ✅ `"file"` 是 **directory entries** 中的 `type` 字段（与 `"directory"` entry 对应）

### 1.2 Directory Block 数据结构

**后端存储（扁平 Map）**：
```json
{
  "block_id": "__system_outline__",
  "block_type": "directory",
  "contents": {
    "entries": {
      "README.md": {
        "id": "block-md-1",
        "type": "file",
        "source": "outline",
        "updated_at": "2025-12-24T..."
      },
      "docs": {
        "id": "dir-uuid-1",
        "type": "directory",
        "source": "outline"
      },
      "docs/guide.md": {
        "id": "block-md-2",
        "type": "file",
        "source": "outline"
      },
      "src/main.rs": {
        "id": "block-code-1",
        "type": "file",
        "source": "linked"
      }
    }
  }
}
```

**前端渲染（嵌套树）**：
```typescript
interface VfsNode {
  path: string           // "docs/guide.md"
  name: string           // "guide.md"
  type: 'file' | 'directory'
  blockId: string | null // file有block，directory可能没有
  blockType?: string     // "markdown" | "code" (仅 type=file 时有效)
  source: 'outline' | 'linked'
  children: VfsNode[]    // 嵌套子节点
  isExpanded?: boolean
}
```

### 1.3 Outline vs Linked Repos

| 特性 | Outline | Linked Repos |
|------|---------|--------------|
| **Block ID** | 固定：`__system_outline__` | 动态生成（每次导入） |
| **用途** | 笔记、文档入口（Notion-like） | 外部代码库（VSCode-like） |
| **创建方式** | 前端自动补齐 | 用户手动导入 |
| **重名检测** | 无需检测（单一根节点） | 需要检测，添加 `(1)`, `(2)` 后缀 |
| **内容来源** | 用户在 Outline 中创建 | 从外部文件系统导入 |

---

## 2. 后端 API 与 Capabilities

### 2.1 已实现的 Capabilities

**Directory Extension**（所有类型已导出到 `bindings.ts`）：

1. **`directory.create`**
   ```typescript
   interface DirectoryCreatePayload {
     path: string              // "docs/README.md"
     type: 'file' | 'directory'
     content?: string          // 可选初始内容（仅 file）
     block_type?: string       // "markdown" | "code"（可选，默认 "markdown"）
   }
   ```

2. **`directory.delete`**
   ```typescript
   interface DirectoryDeletePayload {
     path: string  // 要删除的虚拟路径
   }
   ```

3. **`directory.rename`**
   ```typescript
   interface DirectoryRenamePayload {
     old_path: string
     new_path: string
   }
   ```

4. **`directory.import`**
   ```typescript
   interface DirectoryImportPayload {
     source_path: string      // 外部文件系统路径
     target_path?: string     // 可选目标路径（默认根目录）
   }
   ```

5. **`directory.export`**
   ```typescript
   interface DirectoryExportPayload {
     target_path: string      // 导出到的外部路径
     source_path?: string     // 可选源路径（导出部分目录）
   }
   ```

**Core Capabilities**（需要配合使用）：

- **`core.create`** - 创建 directory block（用于 Linked Repos）
  ```typescript
  interface CreateBlockPayload {
    name: string
    block_type: "directory"
    metadata?: Record<string, any>
  }
  ```

### 2.2 文件类型推断（后端已实现）

后端在 `src-tauri/src/utils/block_type_inference.rs` 中已实现：
- `.md`, `.markdown` → `"markdown"`
- `.rs`, `.py`, `.js`, `.ts`, ... → `"code"`
- 二进制文件（`.png`, `.exe`, ...） → `None`（跳过导入）

**前端无需实现类型推断逻辑**，后端会自动处理。

---

## 3. 前端实现方案

### 3.1 数据转换核心算法

**扁平 Map → 嵌套树**（`utils/vfs-tree.ts`）：

```typescript
/**
 * 将 Directory Block 的扁平 entries 转换为嵌套树结构
 *
 * 算法：
 * 1. 遍历所有 entries，按路径深度排序
 * 2. 使用 Map 缓存已创建的节点（path → VfsNode）
 * 3. 对每个 entry，找到父节点并插入
 *
 * UI 渲染规则：
 * - type === 'directory' -> 显示 Folder 图标 + '+' 按钮（添加子项）
 * - type === 'file' -> 显示 FileText/FileCode 图标（根据 blockType），不显示 '+' 按钮
 */
export function buildTreeFromEntries(
  entries: Record<string, DirectoryEntry>,
  blocks: Block[]  // 需要查询 block_type
): VfsNode[] {
  const nodeMap = new Map<string, VfsNode>()
  const roots: VfsNode[] = []

  // 按路径深度排序（浅到深）
  const sortedPaths = Object.keys(entries).sort((a, b) => {
    const depthA = a.split('/').length
    const depthB = b.split('/').length
    return depthA - depthB
  })

  for (const path of sortedPaths) {
    const entry = entries[path]
    const segments = path.split('/')
    const name = segments[segments.length - 1]

    // 构建节点
    const node: VfsNode = {
      path,
      name,
      type: entry.type as 'file' | 'directory',
      blockId: entry.type === 'file' ? entry.id : null,
      blockType: entry.type === 'file'
        ? blocks.find(b => b.block_id === entry.id)?.block_type
        : undefined,
      source: entry.source as 'outline' | 'linked',
      children: [],
      isExpanded: false
    }

    nodeMap.set(path, node)

    // 找父节点
    if (segments.length === 1) {
      // 根节点
      roots.push(node)
    } else {
      const parentPath = segments.slice(0, -1).join('/')
      const parent = nodeMap.get(parentPath)
      if (parent) {
        parent.children.push(node)
      } else {
        // 父节点不存在（数据不一致），放到根
        console.warn(`Parent not found for ${path}, adding to root`)
        roots.push(node)
      }
    }
  }

  return roots
}
```

### 3.2 TauriClient 扩展（`lib/tauri-client.ts`）

```typescript
import {
  DirectoryCreatePayload,
  DirectoryDeletePayload,
  DirectoryRenamePayload,
  DirectoryImportPayload,
  DirectoryExportPayload,
} from '@/bindings'

export class DirectoryOperations {
  /**
   * 在 directory block 中创建文件或文件夹
   */
  static async createEntry(
    fileId: string,
    directoryBlockId: string,
    path: string,
    type: 'file' | 'directory',
    blockType?: string,
    content?: string,
    editorId?: string
  ): Promise<Event[]> {
    const activeEditorId =
      editorId ||
      (await EditorOperations.getActiveEditor(fileId)) ||
      (await getSystemEditorId())

    const payload: DirectoryCreatePayload = {
      path,
      type,
      block_type: blockType,
      content,
    }

    const cmd = createCommand(
      activeEditorId,
      'directory.create',
      directoryBlockId,
      payload as unknown as JsonValue
    )
    return await BlockOperations.executeCommand(fileId, cmd)
  }

  /**
   * 删除 directory entry
   */
  static async deleteEntry(
    fileId: string,
    directoryBlockId: string,
    path: string,
    editorId?: string
  ): Promise<Event[]> {
    const activeEditorId =
      editorId ||
      (await EditorOperations.getActiveEditor(fileId)) ||
      (await getSystemEditorId())

    const payload: DirectoryDeletePayload = { path }
    const cmd = createCommand(
      activeEditorId,
      'directory.delete',
      directoryBlockId,
      payload as unknown as JsonValue
    )
    return await BlockOperations.executeCommand(fileId, cmd)
  }

  /**
   * 重命名或移动 entry
   */
  static async renameEntry(
    fileId: string,
    directoryBlockId: string,
    oldPath: string,
    newPath: string,
    editorId?: string
  ): Promise<Event[]> {
    const activeEditorId =
      editorId ||
      (await EditorOperations.getActiveEditor(fileId)) ||
      (await getSystemEditorId())

    const payload: DirectoryRenamePayload = {
      old_path: oldPath,
      new_path: newPath,
    }
    const cmd = createCommand(
      activeEditorId,
      'directory.rename',
      directoryBlockId,
      payload as unknown as JsonValue
    )
    return await BlockOperations.executeCommand(fileId, cmd)
  }

  /**
   * 导入外部目录到 directory block
   */
  static async importDirectory(
    fileId: string,
    directoryBlockId: string,
    sourcePath: string,
    targetPath?: string,
    editorId?: string
  ): Promise<Event[]> {
    const activeEditorId =
      editorId ||
      (await EditorOperations.getActiveEditor(fileId)) ||
      (await getSystemEditorId())

    const payload: DirectoryImportPayload = {
      source_path: sourcePath,
      target_path: targetPath,
    }
    const cmd = createCommand(
      activeEditorId,
      'directory.import',
      directoryBlockId,
      payload as unknown as JsonValue
    )
    return await BlockOperations.executeCommand(fileId, cmd)
  }

  /**
   * 导出 directory block 到外部文件系统
   */
  static async exportDirectory(
    fileId: string,
    directoryBlockId: string,
    targetPath: string,
    sourcePath?: string,
    editorId?: string
  ): Promise<Event[]> {
    const activeEditorId =
      editorId ||
      (await EditorOperations.getActiveEditor(fileId)) ||
      (await getSystemEditorId())

    const payload: DirectoryExportPayload = {
      target_path: targetPath,
      source_path: sourcePath,
    }
    const cmd = createCommand(
      activeEditorId,
      'directory.export',
      directoryBlockId,
      payload as unknown as JsonValue
    )
    return await BlockOperations.executeCommand(fileId, cmd)
  }
}

// 导出到主 TauriClient
export const TauriClient = {
  file: FileOperations,
  block: BlockOperations,
  editor: EditorOperations,
  directory: DirectoryOperations,  // ← 新增
}
```

### 3.3 AppStore 扩展（`lib/app-store.ts`）

```typescript
import { buildTreeFromEntries } from '@/utils/vfs-tree'

interface FileState {
  // ... 现有字段
  directoryTrees: Map<string, VfsNode[]>  // blockId → tree
}

interface AppStore {
  // ... 现有字段和方法

  /**
   * 从 directory block 构建树结构
   */
  buildDirectoryTree: (fileId: string, blockId: string) => VfsNode[]

  /**
   * 获取 Outline 树（__system_outline__）
   */
  getOutlineTree: (fileId: string) => VfsNode[]

  /**
   * 获取所有 Linked Repos 树
   */
  getLinkedReposTrees: (fileId: string) => { blockId: string; name: string; tree: VfsNode[] }[]

  /**
   * 初始化 __system_outline__（如果不存在）
   */
  ensureSystemOutline: (fileId: string) => Promise<void>
}

// 实现
buildDirectoryTree: (fileId: string, blockId: string) => {
  const block = get().getBlock(fileId, blockId)
  if (!block || block.block_type !== 'directory') {
    return []
  }

  const entries = block.contents?.entries || {}
  const allBlocks = get().getBlocks(fileId)

  return buildTreeFromEntries(entries, allBlocks)
},

getOutlineTree: (fileId: string) => {
  return get().buildDirectoryTree(fileId, '__system_outline__')
},

getLinkedReposTrees: (fileId: string) => {
  const blocks = get().getBlocks(fileId)
  const directoryBlocks = blocks.filter(
    b => b.block_type === 'directory' && b.block_id !== '__system_outline__'
  )

  return directoryBlocks.map(block => ({
    blockId: block.block_id,
    name: block.name,
    tree: get().buildDirectoryTree(fileId, block.block_id)
  }))
},

ensureSystemOutline: async (fileId: string) => {
  const blocks = get().getBlocks(fileId)
  const outline = blocks.find(b => b.block_id === '__system_outline__')

  if (!outline) {
    // 创建固定ID的directory block
    // 注意：需要在后端支持指定block_id的创建，或使用约定命名
    await TauriClient.block.executeCommand(fileId, {
      cmd_id: crypto.randomUUID(),
      editor_id: await getSystemEditorId(),
      cap_id: 'core.create',
      block_id: '__system_outline__',  // 固定ID
      payload: {
        name: 'Outline',
        block_type: 'directory',
        metadata: {
          description: 'System outline - auto-created'
        }
      },
      timestamp: new Date().toISOString()
    })

    // 强制重载 blocks
    await get().loadBlocks(fileId)
  }
}

/**
 * 补齐逻辑触发时机：
 *
 * 1. 文件打开时 (openFile 成功后)
 * 2. loadBlocks 完成后
 * 3. FilePanel 组件挂载时 (useEffect)
 *
 * 逻辑流程：
 * 1. 检查当前 blocks 中是否存在 __system_outline__
 * 2. 不存在则调用 core.create 创建 directory block
 * 3. 重新加载 blocks 以反映新创建的 Outline
 */
```

### 3.4 FilePanel 改造（`components/editor/FilePanel.tsx`）

```typescript
export const FilePanel = () => {
  const { currentFileId, getBlocks, selectBlock, selectedBlockId } = useAppStore()
  const [isImportModalOpen, setIsImportModalOpen] = useState(false)

  // 初始化 Outline
  useEffect(() => {
    if (currentFileId) {
      useAppStore.getState().ensureSystemOutline(currentFileId)
    }
  }, [currentFileId])

  // 获取数据
  const outlineTree = currentFileId
    ? useAppStore.getState().getOutlineTree(currentFileId)
    : []

  const linkedRepos = currentFileId
    ? useAppStore.getState().getLinkedReposTrees(currentFileId)
    : []

  // Outline 添加文件/文件夹
  const handleOutlineAdd = async (parentPath: string, type: 'file' | 'directory') => {
    if (!currentFileId) return

    const name = prompt(`Enter ${type} name:`)
    if (!name) return

    const path = parentPath ? `${parentPath}/${name}` : name

    await TauriClient.directory.createEntry(
      currentFileId,
      '__system_outline__',
      path,
      type,
      type === 'file' ? 'markdown' : undefined
    )

    // 重载数据
    await useAppStore.getState().loadBlocks(currentFileId)
  }

  // Linked Repos 导入
  const handleImportRepo = async (sourcePath: string) => {
    if (!currentFileId) return

    // 1. 提取文件夹名
    const folderName = sourcePath.split('/').pop() || 'Imported'

    // 2. 重名检测（只在 Linked Repos 区域）
    const existingNames = linkedRepos.map(r => r.name)
    const uniqueName = getUniqueName(folderName, existingNames)

    // 3. 创建 directory block
    const events = await TauriClient.block.createBlock(
      currentFileId,
      uniqueName,
      'directory'
    )

    // 4. 从events中提取新创建的block ID
    const createEvent = events.find(e => e.attribute.endsWith('/core.create'))
    const newBlockId = createEvent?.entity

    if (!newBlockId) {
      toast.error('Failed to create directory block')
      return
    }

    // 5. 导入文件
    await TauriClient.directory.importDirectory(
      currentFileId,
      newBlockId,
      sourcePath
    )

    // 6. 重载数据
    await useAppStore.getState().loadBlocks(currentFileId)
    toast.success(`Imported "${uniqueName}"`)
  }

  return (
    <aside>
      {/* Outline Section */}
      <div>
        <div className="header">
          <span>Outline</span>
          <Button onClick={() => handleOutlineAdd('', 'file')}>
            <Plus />
          </Button>
        </div>
        <VfsTree
          nodes={outlineTree}
          activeNodeId={selectedBlockId}
          onSelect={(node) => node.blockId && selectBlock(node.blockId)}
          onAddChild={(node) => handleOutlineAdd(node.path, 'file')}
          onDelete={async (node) => {
            await TauriClient.directory.deleteEntry(
              currentFileId!,
              '__system_outline__',
              node.path
            )
            await useAppStore.getState().loadBlocks(currentFileId!)
          }}
          // ... 其他handlers
        />
      </div>

      {/* Linked Repos Section */}
      <div>
        <div className="header">
          <span>Linked Repos</span>
          <Button onClick={() => setIsImportModalOpen(true)}>
            <Plus />
          </Button>
        </div>
        {linkedRepos.map(repo => (
          <VfsTree key={repo.blockId} nodes={repo.tree} {...handlers} />
        ))}
      </div>

      <ImportRepositoryModal
        open={isImportModalOpen}
        onOpenChange={setIsImportModalOpen}
        onImport={handleImportRepo}
      />
    </aside>
  )
}

// 辅助函数
function getUniqueName(baseName: string, existingNames: string[]): string {
  let name = baseName
  let counter = 1
  while (existingNames.includes(name)) {
    name = `${baseName} (${counter++})`
  }
  return name
}
```

### 3.5 VfsTree 通用组件（改造 OutlineTree）

**关键改动**：
- 支持 `VfsNode` 接口
- 区分文件和文件夹图标
- 文件夹显示 `+` 按钮，文件不显示
- 菜单支持 Export 选项

```typescript
interface VfsTreeProps {
  nodes: VfsNode[]
  activeNodeId: string | null
  onSelect: (node: VfsNode) => void
  onAddChild?: (node: VfsNode) => void  // 仅文件夹
  onRename?: (node: VfsNode, newName: string) => void
  onDelete?: (node: VfsNode) => void
  onExport?: (node: VfsNode) => void
}

export const VfsTree = ({ nodes, ...handlers }: VfsTreeProps) => {
  // 渲染单个节点
  const renderNode = (node: VfsNode, depth: number) => {
    // 图标选择
    const icon = node.type === 'directory'
      ? <Folder className="h-4 w-4" />
      : node.blockType === 'markdown'
        ? <FileText className="h-4 w-4" />
        : <FileCode className="h-4 w-4" />

    // '+' 按钮逻辑：只有文件夹显示
    const showAddButton = node.type === 'directory'

    return (
      <div>
        <div className="flex items-center">
          {/* 图标 */}
          {icon}

          {/* 标题 */}
          <span onClick={() => onSelect(node)}>{node.name}</span>

          {/* 操作按钮 */}
          <div className="actions">
            {/* 只有文件夹显示 + 按钮 */}
            {showAddButton && (
              <Button onClick={() => onAddChild?.(node)}>
                <Plus />
              </Button>
            )}

            {/* 更多菜单（三个点） */}
            <DropdownMenu>
              <DropdownMenuItem onClick={() => onRename?.(node)}>
                Rename
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => onDelete?.(node)}>
                Delete
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => onExport?.(node)}>
                Export
              </DropdownMenuItem>
            </DropdownMenu>
          </div>
        </div>

        {/* 递归渲染子节点 */}
        {node.children?.map(child => renderNode(child, depth + 1))}
      </div>
    )
  }

  return <>{nodes.map(node => renderNode(node, 0))}</>
}
```

### 3.6 Export (Checkout) 完整交互流程

**用户操作流程**：

```typescript
// 在 VfsTree 或 FilePanel 中
const handleExport = async (node: VfsNode) => {
  if (!currentFileId) return

  // 步骤 1: 使用 Tauri Dialog 选择导出目标路径
  const targetPath = await open({
    directory: true,  // 选择文件夹
    multiple: false,
    title: `Export ${node.name} to...`
  })

  if (!targetPath || typeof targetPath !== 'string') {
    return  // 用户取消
  }

  // 步骤 2: 调用后端 checkout_workspace 命令
  try {
    // 找到当前节点所属的 directory block ID
    const directoryBlockId = findDirectoryBlockId(node)  // 辅助函数

    await commands.checkoutWorkspace(
      currentFileId,
      directoryBlockId,
      {
        target_path: targetPath,
        source_path: node.path  // 可选，如果只导出部分子树
      }
    )

    // 步骤 3: 成功提示 + 可选的"打开文件夹"操作
    const result = await confirm({
      title: 'Export Successful',
      message: `Exported to ${targetPath}. Open folder?`,
      okLabel: 'Open Folder',
      cancelLabel: 'Close'
    })

    if (result) {
      // 使用 Tauri shell 打开文件夹
      await shell.open(targetPath)
    }
  } catch (error) {
    toast.error(`Export failed: ${error}`)
  }
}

/**
 * 辅助函数：找到节点所属的 directory block ID
 *
 * 逻辑：
 * - 如果是 Outline 中的节点 → '__system_outline__'
 * - 如果是 Linked Repo 中的节点 → 当前 repo 的 block ID
 */
function findDirectoryBlockId(node: VfsNode): string {
  // 实现逻辑：根据 node.source 或当前上下文判断
  if (node.source === 'outline') {
    return '__system_outline__'
  } else {
    // 需要从上下文或 props 中传入当前 repo 的 blockId
    return currentRepoBlockId
  }
}
```

**Tauri 依赖**：
```typescript
import { open } from '@tauri-apps/plugin-dialog'
import { open as shellOpen } from '@tauri-apps/plugin-shell'
import { commands } from '@/bindings'
```

**关键要点**：
1. ✅ 导出前必须选择目标路径（不能静默导出）
2. ✅ 支持部分导出（通过 `source_path` 参数）
3. ✅ 导出后提供"打开文件夹"的便捷操作
4. ✅ 错误处理和用户反馈

---

## 4. 实施步骤

### Phase 1: 基础设施（2-3小时）
1. ✅ 创建 `utils/vfs-tree.ts` - 实现 `buildTreeFromEntries`
2. ✅ 扩展 `lib/tauri-client.ts` - 添加 `DirectoryOperations` 类
3. ✅ 测试数据转换算法（单元测试）

### Phase 2: State Management（1-2小时）
4. ✅ 扩展 `app-store.ts` - 添加 directory 相关方法
5. ✅ 实现 `ensureSystemOutline` 自动补齐机制
6. ✅ 测试 store 逻辑

### Phase 3: UI 组件（3-4小时）
7. ✅ 改造 `OutlineTree.tsx` → `VfsTree.tsx`（通用组件）
8. ✅ 重构 `FilePanel.tsx`（使用新的数据源）
9. ✅ 实现 Outline 添加/删除/重命名逻辑
10. ✅ 实现 Linked Repos 导入逻辑（含重名检测）

### Phase 4: 高级功能（2-3小时）
11. ✅ 集成 Tauri `dialog.save` 实现 Export 功能
12. ✅ 权限检查（根据 grants 禁用/隐藏按钮）
13. ✅ 错误处理和用户反馈

### Phase 5: 测试与优化（1-2小时）
14. ✅ 端到端测试（创建、导入、导出、删除）
15. ✅ 性能优化（大型项目加载）
16. ✅ 边界情况处理

---

## 5. 关键注意事项

### 5.1 数据一致性
- ⚠️ **扁平 entries 的顺序问题**：后端 Map 无序，前端需要按路径深度排序
- ⚠️ **父节点缺失**：如果 entries 中有 `"a/b/c.md"` 但缺少 `"a"` 和 `"a/b"` 的 directory entry，需要容错处理

### 5.2 类型安全
- ✅ 使用 `bindings.ts` 的类型，不要手动定义
- ✅ 区分 `entry.type` 和 `block.block_type`
- ✅ `blockId` 对于 directory entry 可能为 null

### 5.3 性能优化
- 对于大型项目（>1000 文件），考虑虚拟滚动
- 缓存已构建的树（避免重复计算）
- 按需展开（默认折叠深层目录）

### 5.4 用户体验
- 导入大型项目时显示进度（后端目前无进度回调，考虑添加 loading 状态）
- Export 后给予成功提示，并提供"打开文件夹"选项
- 重名冲突时自动处理，无需用户干预

---

## 6. 未来扩展

### 6.1 短期（与 Code Extension 配合）
- Code block 编辑器集成
- 语法高亮和代码折叠

### 6.2 中期
- 文件搜索和过滤
- 拖拽移动文件/文件夹
- 批量操作（多选删除/导出）

### 6.3 长期
- 实时协作（多用户同时编辑）
- 冲突解决 UI
- 版本历史可视化

# Directory Extension 前端开发指南

## 1. 架构概览

### 1.1 当前前端技术栈

- **框架**: React 18.3.1 + TypeScript 5.6.2
- **状态管理**: Zustand 5.0.8 (单一 store 模式)
- **UI 组件库**: shadcn/ui (基于 Radix UI)
- **图标**: Lucide React
- **样式**: Tailwind CSS 4.1.16
- **构建工具**: Vite 6.0.3
- **测试框架**: Vitest 4.0.2
- **后端通信**: Tauri Specta v2 (自动生成 TypeScript 绑定)

### 1.2 现有布局结构

当前应用采用 3-panel 布局 (参考 `src/App.tsx`):

```
┌─────────────────────────────────────────┐
│           Toolbar (顶部工具栏)            │
├─────────────┬───────────────────────────┤
│             │                           │
│  BlockList  │     BlockEditor           │
│   (1/3)     │       (2/3)               │
│             │                           │
│  - 显示所有  │  - 显示选中 block 的内容   │
│    blocks   │  - 提供编辑功能            │
│  - 创建按钮  │  - 保存按钮                │
│  - 删除按钮  │                           │
│             │                           │
└─────────────┴───────────────────────────┘
```

### 1.3 Directory Extension 集成方案

Directory extension 将复用现有布局：

- **左侧 BlockList**: 显示所有 blocks（包括 directory 类型）
- **右侧编辑区**: 根据选中 block 的类型动态渲染
  - `block_type === "markdown"` → 渲染 `<BlockEditor />` (textarea 编辑器)
  - `block_type === "directory"` → 渲染 `<DirectoryBlock />` (文件浏览器)

---

## 2. 前置准备：TypeScript 类型生成

### 2.1 问题分析

**关键发现**: 如果你是首次开发或 `src/bindings.ts` 中没有 Directory 相关的 Payload 类型，需要先生成这些类型。

已有类型：
- ✅ `Block`, `Command`, `Event`, `Editor`, `Grant`
- ✅ `CreateBlockPayload`, `GrantPayload`, `LinkBlockPayload`, `MarkdownWritePayload`, `RevokePayload`, `UnlinkBlockPayload`
- ✅ **应该包含**: `DirectoryListPayload`, `DirectoryCreatePayload`, `DirectoryDeletePayload`, `DirectoryRenamePayload`, `DirectoryRefreshPayload`, `DirectoryWatchPayload`, `DirectorySearchPayload`, `DirectoryRootPayload` 等 8 个类型

**原因**: tauri-specta 只会导出被 Tauri commands 引用的类型。由于 Directory capabilities 通过 `executeCommand()` 的通用接口调用（payload 作为 `JsonValue` 传递），这些类型需要在 `lib.rs` 中通过 `.typ::<T>()` 显式注册。

**好消息**: Directory extension 的所有 Payload 类型已经在 `src-tauri/src/lib.rs` 第 54-60 行正确注册：

```rust
.typ::<extensions::directory::DirectorySearchPayload>()
.typ::<extensions::directory::DirectoryWatchPayload>()
.typ::<extensions::directory::DirectoryRefreshPayload>()
.typ::<extensions::directory::DirectoryRenamePayload>()
.typ::<extensions::directory::DirectoryDeletePayload>()
.typ::<extensions::directory::DirectoryCreatePayload>()
.typ::<extensions::directory::DirectoryListPayload>()
```

因此**无需手动维护类型定义**，只需触发自动生成即可。

### 2.2 生成 TypeScript 绑定

**关键理解**: Tauri Specta 的类型导出不是在编译时触发，而是在**应用启动时**触发（见 `lib.rs` 第 72-77 行）。

运行以下命令启动应用（会自动生成 bindings.ts）：

```bash
pnpm tauri dev
```

应用启动后，`src/bindings.ts` 会自动更新，包含所有 8 个 Directory Payload 类型：

```typescript
export type DirectoryListPayload = {
  path: string
  recursive?: boolean
  include_hidden?: boolean
  max_depth?: number | null
}
export type DirectoryCreatePayload = {
  path: string
  item_type: string // 实际类型为 string，前端约定仅使用 'file' | 'dir'
  content?: string
}
export type DirectoryDeletePayload = {
  path: string
  recursive?: boolean
}
export type DirectoryRenamePayload = {
  old_path: string
  new_path: string
}
export type DirectoryRefreshPayload = {
  recursive?: boolean
}
export type DirectoryWatchPayload = {
  enabled: boolean
}
export type DirectorySearchPayload = {
  pattern: string
  recursive?: boolean
  include_hidden?: boolean
}
export type DirectoryRootPayload = {
  root: string              // 用户选择的绝对路径
  recursive?: boolean       // 是否递归加载（默认true）
  include_hidden?: boolean  // 是否包含隐藏文件（默认false）
  max_depth?: number | null // 最大加载深度（默认3）
}
```

### 2.3 验证类型生成

检查 `src/bindings.ts` 是否包含 Directory 类型：

```bash
grep -E "export type Directory.*Payload" src/bindings.ts
```

应该看到 8 行输出（每个 Payload 类型一行）。如果看到类型定义，说明成功。

**注意**: 每次修改 Payload 结构后（如添加新字段），重新运行 `pnpm tauri dev` 会自动更新 TypeScript 类型，保持前后端类型同步。

---

## 3. DirectoryBlock 组件设计

### 3.1 组件职责

`DirectoryBlock` 是专门显示和操作 directory 类型 block 的组件，提供类似 VSCode 文件浏览器的交互体验。

**核心功能**:
1. 显示目录树（文件/文件夹列表）
2. 支持 8 个后端 capabilities：root, list, create, delete, rename, refresh, watch, search
3. 提供工具栏按钮：Choose Root, Refresh, Create File, Create Folder, Delete, Search
4. 显示基础元数据（名称、是否目录、相对路径）
5. 错误处理和加载状态

### 3.2 数据加载策略

**加载模式：递归加载 + 前端构建树**

类似VSCode打开workspace的行为，初始加载多层目录结构，前端解析为树形展示：

```typescript
// 初始加载（创建block或刷新时）
directory.list {
  path: ".",
  recursive: true,
  max_depth: 3,  // 初始加载3层，避免大项目卡顿
  include_hidden: false
}

// 后端返回平铺数据（带相对路径）
block.contents.entries = [
  { name: "src", path: "src", is_dir: true, is_file: false },
  { name: "main.rs", path: "src/main.rs", is_dir: false, is_file: true },
  { name: "utils", path: "src/utils", is_dir: true, is_file: false },
  { name: "helper.rs", path: "src/utils/helper.rs", is_dir: false, is_file: true },
  { name: "README.md", path: "README.md", is_dir: false, is_file: true }
]
```

**Block.contents 数据格式**:

```typescript
interface DirectoryBlockContents {
  root: string              // 绝对路径（如 "/home/user/project"）
  recursive: boolean        // true（采用递归加载）
  include_hidden: boolean   // false（默认不显示隐藏文件）
  max_depth: number | null  // 3（初始加载深度）
  entries: DirectoryEntry[] // 平铺数组（带path字段）
  last_updated: string      // ISO 8601时间戳
  watch_enabled?: boolean   // 可选，watch状态
}

interface DirectoryEntry {
  name: string    // 文件/文件夹名称
  path: string    // 相对路径（递归模式必有此字段）
  is_dir: boolean
  is_file: boolean
}
```

**前端树结构**:

```typescript
interface TreeNode {
  name: string
  path: string
  is_dir: boolean
  is_file: boolean
  children: TreeNode[]  // 子节点（前端构建）
  depth: number         // 节点深度（用于判断是否需要懒加载）
}

// 解析函数
function buildTree(entries: DirectoryEntry[]): TreeNode {
  const root: TreeNode = { name: "", path: "", is_dir: true, is_file: false, children: [], depth: 0 }

  // 按path长度排序，确保父节点先处理
  const sorted = [...entries].sort((a, b) => a.path.split('/').length - b.path.split('/').length)

  sorted.forEach(entry => {
    const parts = entry.path.split('/')
    let current = root

    // 查找或创建父节点路径
    for (let i = 0; i < parts.length - 1; i++) {
      const part = parts[i]
      let child = current.children.find(c => c.name === part)
      if (!child) {
        child = {
          name: part,
          path: parts.slice(0, i + 1).join('/'),
          is_dir: true,
          is_file: false,
          children: [],
          depth: i + 1
        }
        current.children.push(child)
      }
      current = child
    }

    // 添加当前节点
    current.children.push({
      name: entry.name,
      path: entry.path,
      is_dir: entry.is_dir,
      is_file: entry.is_file,
      children: [],
      depth: parts.length
    })
  })

  return root
}
```

### 3.3 组件结构

```typescript
// src/components/DirectoryBlock.tsx
import { useState, useEffect, useMemo } from 'react'
import { useAppStore } from '@/lib/app-store'
import { Button } from '@/components/ui/button'
import { Folder, File, RefreshCw, Plus, Trash2, Search, FolderPlus } from 'lucide-react'
import type { Block } from '@/bindings'

interface TreeNode {
  name: string
  path: string
  is_dir: boolean
  is_file: boolean
  children: TreeNode[]
  depth: number
}

interface DirectoryBlockContents {
  root: string
  recursive: boolean
  include_hidden: boolean
  max_depth: number | null
  entries: DirectoryEntry[]
  last_updated: string
  watch_enabled?: boolean
}

export function DirectoryBlock({ block, fileId }: { block: Block; fileId: string }) {
  const contents = block.contents as DirectoryBlockContents
  const addNotification = useAppStore(state => state.addNotification)
  const getActiveEditor = useAppStore(state => state.getActiveEditor)

  const activeEditorId = useMemo(() => {
    return getActiveEditor(fileId)?.editor_id ?? 'default-editor'
  }, [fileId, getActiveEditor])

  // 状态管理
  const [tree, setTree] = useState<TreeNode | null>(null)
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set())
  const [selectedPath, setSelectedPath] = useState<string | null>(null)
  const [isRefreshing, setIsRefreshing] = useState(false)

  // 构建树结构（当 block.contents 更新时）
  useEffect(() => {
    if (contents?.entries) {
      const treeRoot = buildTree(contents.entries)
      setTree(treeRoot)
    }
  }, [contents])

  // 工具栏按钮处理函数
  const handleRefresh = async () => { /* 见5.2节 */ }
  const handleCreateFile = async () => { /* 见5.3节 */ }
  const handleCreateFolder = async () => { /* 见5.3节 */ }
  const handleDelete = async () => { /* 见5.4节 */ }
  const handleSearch = async () => { /* 见5.6节 */ }

  // 展开/折叠处理（纯前端状态切换）
  const handleToggleExpand = (path: string) => {
    setExpandedPaths(prev => {
      const next = new Set(prev)
      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
      }
      return next
    })
  }

  return (
    <div className="flex h-full flex-col">
      {/* 顶部工具栏 */}
      <DirectoryToolbar
        onRefresh={handleRefresh}
        onCreateFile={handleCreateFile}
        onCreateFolder={handleCreateFolder}
        onDelete={handleDelete}
        onSearch={handleSearch}
        isRefreshing={isRefreshing}
        hasSelection={!!selectedPath}
      />

      {/* 中间树形视图 */}
      <DirectoryTree
        tree={tree}
        expandedPaths={expandedPaths}
        selectedPath={selectedPath}
        onToggleExpand={handleToggleExpand}
        onSelect={setSelectedPath}
      />

      {/* 底部状态栏 */}
      <DirectoryStatusBar
        root={contents?.root}
        lastUpdated={contents?.last_updated}
        watchEnabled={contents?.watch_enabled}
        totalNodes={contents?.entries.length || 0}
      />
    </div>
  )
}
```

**组件职责**：
- **DirectoryBlock**：主容器，管理树状态（tree, expandedPaths, selectedPath）
- **DirectoryToolbar**：提供初始化与后续操作；当 `contents.root` 为空时仅显示 “Choose Root”，设置成功后才展示 Refresh / Create File / Create Folder / Delete / Search 等按钮
- **DirectoryTree**：递归渲染树节点，处理展开/折叠交互
- **DirectoryStatusBar**：显示根路径、更新时间、watch 状态、节点总数

### 3.4 使用流程

**1. 创建 directory block**

通过 `core.create` 创建 block（`contents` 初始为空）。BlockList 统一使用 `NewBlockDialog` 收集名称/类型；当类型为 directory 时，仅创建空 block，后续由 Toolbar 完成 root 配置。

**2. 设置 root**

在 DirectoryBlock 中，Toolbar 发现 `contents.root` 为空时，仅显示 “Choose Root”：
1. 点击后调用 `@tauri-apps/plugin-dialog.open({ directory: true, multiple: false })` 让用户选择根目录。
2. 依次发送两条 command：
   - `directory.root`（payload 包含 root/recursive/include_hidden/max_depth）→ 将配置写入 `block.contents`。
   - `directory.list { path: ".", recursive: true, include_hidden: false, max_depth: 3 }` → 获取初始目录树。
3. 成功后 Toolbar 才解锁其他操作（Refresh、Create、Delete、Search 等）。

**3. 初始加载**

完成 root 设置后，每次 DirectoryBlock 挂载都会自动调用：
```typescript
directory.list {
  path: ".",
  recursive: true,
  max_depth: 3,
  include_hidden: false
}
```

后端返回平铺的 `entries` 数组，前端构建树结构并渲染。

**3. 展开/折叠文件夹**

点击文件夹图标触发 `handleToggleExpand(path)`：
- 已展开 → 从 `expandedPaths` 移除（折叠）
- 未展开 → 添加到 `expandedPaths`（展开）
- **不调用后端**（数据已在初始加载时获取）

**4. 刷新**

点击工具栏 Refresh 按钮：
```typescript
directory.refresh { recursive: true }
```
- 后端重新扫描文件系统（保持 include_hidden、max_depth 配置）
- 前端清空 `expandedPaths` 和 `selectedPath`
- 重新构建树

**5. 创建/删除/重命名**

操作完成后，不调用 refresh，而是直接清空状态并重新调用初始加载逻辑，确保树结构完整更新。

---

## 4. VSCode 风格交互实现

### 4.1 树形视图组件

```typescript
// DirectoryTree.tsx
import { Folder, File, ChevronRight, ChevronDown } from 'lucide-react'
import { cn } from '@/lib/utils'

interface DirectoryTreeProps {
  tree: TreeNode | null
  expandedPaths: Set<string>
  selectedPath: string | null
  onToggleExpand: (path: string) => void
  onSelect: (path: string) => void
}

export function DirectoryTree({
  tree,
  expandedPaths,
  selectedPath,
  onToggleExpand,
  onSelect
}: DirectoryTreeProps) {
  if (!tree) {
    return <div className="p-4 text-muted-foreground">Loading...</div>
  }

  return (
    <div className="flex-1 overflow-y-auto p-2">
      {tree.children.map(node => (
        <TreeNodeItem
          key={node.path}
          node={node}
          expandedPaths={expandedPaths}
          selectedPath={selectedPath}
          onToggleExpand={onToggleExpand}
          onSelect={onSelect}
          depth={0}
        />
      ))}
    </div>
  )
}

function TreeNodeItem({
  node,
  expandedPaths,
  selectedPath,
  onToggleExpand,
  onSelect,
  depth
}: {
  node: TreeNode
  expandedPaths: Set<string>
  selectedPath: string | null
  onToggleExpand: (path: string) => void
  onSelect: (path: string) => void
  depth: number
}) {
  const isExpanded = expandedPaths.has(node.path)
  const isSelected = selectedPath === node.path
  const Icon = node.is_dir ? Folder : File
  const ChevronIcon = isExpanded ? ChevronDown : ChevronRight

  return (
    <>
      <div
        className={cn(
          "flex items-center gap-2 px-2 py-1 rounded cursor-pointer hover:bg-accent",
          isSelected && "bg-accent border-l-2 border-primary"
        )}
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
        onClick={() => onSelect(node.path)}
      >
        {node.is_dir && (
          <ChevronIcon
            className="h-4 w-4 shrink-0"
            onClick={(e) => {
              e.stopPropagation()
              onToggleExpand(node.path)
            }}
          />
        )}
        {!node.is_dir && <div className="w-4" />}
        <Icon className="h-4 w-4 shrink-0" />
        <span className="text-sm truncate">{node.name}</span>
      </div>

      {/* 递归渲染子节点 */}
      {node.is_dir && isExpanded && node.children.map(child => (
        <TreeNodeItem
          key={child.path}
          node={child}
          expandedPaths={expandedPaths}
          selectedPath={selectedPath}
          onToggleExpand={onToggleExpand}
          onSelect={onSelect}
          depth={depth + 1}
        />
      ))}
    </>
  )
}
```

### 4.2 交互模式

**基础交互**：
- **单击节点**：选中（高亮显示）
- **点击箭头**：展开/折叠文件夹（不触发选中）
- **右键节点**：显示上下文菜单（见下方）

**工具栏按钮**（始终可用）：

| 按钮 | 图标 | 功能 | 禁用条件 |
|------|------|------|---------|
| Refresh | RefreshCw | 重新扫描文件系统 | 无 |
| Create File | File | 在选中文件夹内创建文件 | 无（未选中则在根目录） |
| Create Folder | FolderPlus | 在选中文件夹内创建文件夹 | 无（未选中则在根目录） |
| Delete | Trash2 | 删除选中项 | 未选中时禁用 |
| Search | Search | 打开搜索对话框 | 无 |

**右键菜单**（仅在节点上右键时显示）：

```typescript
// DirectoryContextMenu.tsx
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from '@/components/ui/context-menu'

export function DirectoryContextMenu({
  node,
  children,
  onCreateFile,
  onCreateFolder,
  onRename,
  onDelete
}: {
  node: TreeNode
  children: React.ReactNode
  onCreateFile: (parentPath: string) => void
  onCreateFolder: (parentPath: string) => void
  onRename: (path: string) => void
  onDelete: (path: string) => void
}) {
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        {children}
      </ContextMenuTrigger>
      <ContextMenuContent>
        {node.is_dir && (
          <>
            <ContextMenuItem onClick={() => onCreateFile(node.path)}>
              <File className="mr-2 h-4 w-4" />
              New File in "{node.name}"
            </ContextMenuItem>
            <ContextMenuItem onClick={() => onCreateFolder(node.path)}>
              <FolderPlus className="mr-2 h-4 w-4" />
              New Folder in "{node.name}"
            </ContextMenuItem>
          </>
        )}
        <ContextMenuItem onClick={() => onRename(node.path)}>
          <Edit3 className="mr-2 h-4 w-4" />
          Rename
        </ContextMenuItem>
        <ContextMenuItem onClick={() => onDelete(node.path)}>
          <Trash2 className="mr-2 h-4 w-4" />
          Delete
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}
```

**注意**：
- 空白区域右键无菜单
- 在文件上右键时，Create操作在其父文件夹内执行
- Refresh不在右键菜单中（仅工具栏可用）

---

## 5. 后端集成

### 5.1 Command 执行模式

所有 directory 操作通过 `TauriClient.block.executeCommand()` 发送：

```typescript
// src/lib/directory-operations.ts
import { TauriClient } from '@/lib/tauri-client'
import type {
  Command,
  DirectoryListPayload,
  JsonValue,
} from '@/bindings'

type DirectoryCapability =
  | 'directory.root'
  | 'directory.list'
  | 'directory.create'
  | 'directory.delete'
  | 'directory.rename'
  | 'directory.refresh'
  | 'directory.watch'
  | 'directory.search'

function createDirectoryCommand(
  editorId: string,
  blockId: string,
  capId: DirectoryCapability,
  payload: JsonValue
): Command {
  return {
    cmd_id: crypto.randomUUID(),
    editor_id: editorId,
    cap_id: capId,
    block_id: blockId,
    payload,
    timestamp: new Date().toISOString(),
  }
}

export async function executeDirectoryList(
  fileId: string,
  blockId: string,
  editorId: string,
  payload: DirectoryListPayload
) {
  const command = createDirectoryCommand(
    editorId,
    blockId,
    'directory.list',
    payload as unknown as JsonValue
  )
  return await TauriClient.block.executeCommand(fileId, command)
}

// 类似地定义其他 7 个操作函数：root/create/delete/rename/refresh/watch/search
```

`TauriClient.block.executeCommand` 已经会在后端返回 `status === 'error'` 时抛出异常，因此调用方只需使用 `try/catch` 处理失败场景，无需再手动检查返回值。

### 5.2 操作示例：Refresh

```typescript
// DirectoryBlock.tsx
const handleRefresh = async () => {
  setIsRefreshing(true)
  try {
    // 使用 directory.refresh（保持配置，重新扫描）
    const payload: DirectoryRefreshPayload = {
      recursive: true  // 递归刷新，加载多层
    }

    await executeDirectoryRefresh(
      fileId,
      block.block_id,
      activeEditorId,
      payload
    )

    // 后端返回Event更新 block.contents
    // useEffect监听contents变化，自动重新构建树

    // 清空前端状态
    setExpandedPaths(new Set())
    setSelectedPath(null)

    addNotification('success', 'Directory refreshed')
  } catch (error) {
    addNotification('error', `Refresh failed: ${error}`)
  } finally {
    setIsRefreshing(false)
  }
}
```

### 5.3 操作示例：Create File/Folder

```typescript
const handleCreateFile = async () => {
  const fileName = prompt('Enter file name:')
  if (!fileName) return

  try {
    // 计算目标路径：如果选中了文件夹，则在其内创建；否则在根目录
    const parentPath = selectedPath && tree ? findNode(tree, selectedPath) : null
    const targetPath = parentPath?.is_dir ? `${parentPath.path}/${fileName}` : fileName

    const payload: DirectoryCreatePayload = {
      path: targetPath,
      item_type: 'file',
      content: ''
    }

    await executeDirectoryCreate(fileId, block.block_id, activeEditorId, payload)

    // 重新加载以更新树（保持配置）
    await handleRefresh()

    addNotification('success', `File "${fileName}" created`)
  } catch (error) {
    addNotification('error', `Create failed: ${error}`)
  }
}

const handleCreateFolder = async () => {
  const folderName = prompt('Enter folder name:')
  if (!folderName) return

  try {
    const parentPath = selectedPath && tree ? findNode(tree, selectedPath) : null
    const targetPath = parentPath?.is_dir ? `${parentPath.path}/${folderName}` : folderName

    const payload: DirectoryCreatePayload = {
      path: targetPath,
      item_type: 'dir',
      content: ''  // 文件夹忽略content字段
    }

    await executeDirectoryCreate(fileId, block.block_id, activeEditorId, payload)
    await handleRefresh()

    addNotification('success', `Folder "${folderName}" created`)
  } catch (error) {
    addNotification('error', `Create failed: ${error}`)
  }
}

// 辅助函数：在树中查找节点
function findNode(node: TreeNode, path: string): TreeNode | null {
  if (node.path === path) return node
  for (const child of node.children) {
    const found = findNode(child, path)
    if (found) return found
  }
  return null
}
```

### 5.4 操作示例：Delete

```typescript
const handleDelete = async () => {
  if (!selectedPath || !tree) return

  const node = findNode(tree, selectedPath)
  if (!node) return

  const confirmed = confirm(`Delete "${node.name}"?${node.is_dir ? '\nThis will delete the folder and all its contents.' : ''}`)
  if (!confirmed) return

  try {
    const payload: DirectoryDeletePayload = {
      path: node.path,
      recursive: node.is_dir  // 文件夹自动递归删除
    }

    await executeDirectoryDelete(fileId, block.block_id, activeEditorId, payload)

    // 重新加载树
    await handleRefresh()

    addNotification('success', `Deleted "${node.name}"`)
  } catch (error) {
    addNotification('error', `Delete failed: ${error}`)
  }
}
```

### 5.5 操作示例：Rename

```typescript
const handleRename = async () => {
  if (!selectedPath || !tree) return

  const node = findNode(tree, selectedPath)
  if (!node) return

  const newName = prompt('Enter new name:', node.name)
  if (!newName || newName === node.name) return

  try {
    // 计算新路径：替换最后一个路径段
    const pathParts = node.path.split('/')
    pathParts[pathParts.length - 1] = newName
    const newPath = pathParts.join('/')

    const payload: DirectoryRenamePayload = {
      old_path: node.path,
      new_path: newPath
    }

    await executeDirectoryRename(fileId, block.block_id, activeEditorId, payload)

    // 重新加载树
    await handleRefresh()

    addNotification('success', `Renamed to "${newName}"`)
  } catch (error) {
    addNotification('error', `Rename failed: ${error}`)
  }
}
```

### 5.6 操作示例：Search

```typescript
const [searchPattern, setSearchPattern] = useState('')
const [searchResults, setSearchResults] = useState<Array<{ name: string; path?: string; is_dir: boolean; is_file: boolean }>>([])
const [isSearching, setIsSearching] = useState(false)
const [showSearchDialog, setShowSearchDialog] = useState(false)

const handleSearch = async (pattern: string) => {
  if (!pattern.trim()) {
    setSearchResults([])
    return
  }

  setIsSearching(true)
  try {
    const payload: DirectorySearchPayload = {
      pattern: pattern,
      recursive: true,
      include_hidden: contents.include_hidden  // 继承配置
    }

    const events = await executeDirectorySearch(fileId, block.block_id, activeEditorId, payload)

    // directory.search 不更新 block.contents，直接在 Event.value.matches 中返回结果
    const searchEvent = events[0]
    const matches = searchEvent.value.matches as Array<{ name: string; path?: string; is_dir: boolean; is_file: boolean }>
    setSearchResults(matches)

    addNotification('success', `Found ${matches.length} match${matches.length !== 1 ? 'es' : ''}`)
  } catch (error) {
    addNotification('error', `Search failed: ${error}`)
  } finally {
    setIsSearching(false)
  }
}

// 在组件中使用 Dialog 显示搜索结果
<Dialog open={showSearchDialog} onOpenChange={setShowSearchDialog}>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Search Files</DialogTitle>
    </DialogHeader>
    <Input
      placeholder="Pattern (e.g., *.rs, test?.txt)"
      value={searchPattern}
      onChange={(e) => setSearchPattern(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          handleSearch(searchPattern)
        }
      }}
    />
    <div className="max-h-96 overflow-y-auto">
      {searchResults.map((result, i) => (
        <div key={i} className="flex items-center gap-2 p-2 hover:bg-accent rounded">
          {result.is_dir ? <Folder className="h-4 w-4" /> : <File className="h-4 w-4" />}
          <span className="text-sm">{result.path || result.name}</span>
        </div>
      ))}
    </div>
  </DialogContent>
</Dialog>
```

**注意**：
- `directory.search` 不会修改 `block.contents`，结果在 Event.value.matches 中
- 支持通配符：`*`（任意字符序列）、`?`（单个字符）
- 最多支持10个通配符（后端限制）

---

## 6. App.tsx 集成

### 6.1 动态渲染逻辑

修改 `src/App.tsx` 中的 BlockEditor 渲染逻辑：

```typescript
// src/App.tsx
import { BlockEditor } from '@/components/BlockEditor'
import { DirectoryBlock } from '@/components/DirectoryBlock'

function EditorPanel() {
  const { activeFileId, getSelectedBlock } = useAppStore()
  const selectedBlock = activeFileId ? getSelectedBlock(activeFileId) : null

  if (!selectedBlock) {
    return <div>No block selected</div>
  }

  // 根据 block_type 动态渲染组件
  switch (selectedBlock.block_type) {
    case 'markdown':
      return <BlockEditor />
    case 'directory':
      return <DirectoryBlock block={selectedBlock} fileId={activeFileId!} />
    default:
      return <div>Unsupported block type: {selectedBlock.block_type}</div>
  }
}
```

### 6.2 BlockList 增强

在 `BlockList.tsx` 中为 directory 类型 block 显示特殊图标：

```typescript
// BlockList.tsx
import { Folder } from 'lucide-react'

function BlockItem({ block, fileId }: { block: Block; fileId: string }) {
  // ...

  return (
    <div className="...">
      {/* 显示类型图标 */}
      {block.block_type === 'directory' && <Folder className="h-4 w-4" />}

      {/* 显示 directory 的根路径 */}
      {block.block_type === 'directory' && (
        <div className="text-xs text-muted-foreground">
          Root: {(block.contents as any)?.root || 'N/A'}
        </div>
      )}
    </div>
  )
}
```

### 6.3 New Block 对话框 & Directory 初始流程

为了避免 BlockList 上出现多个入口，`New Block` 统一弹出一个 `NewBlockDialog`，字段：`Name`（默认 `Block ${timestamp}`）+ `Type`（select：Markdown / Terminal / Directory）。  
Markdown 与 Terminal 两类 block 会直接调用 `createBlock(fileId, name, blockType)`；Directory 类型当前也走同一路径——先注册一个空的 directory block，随后在 Toolbar 中设置 root（见 6.4）。未来新增 block 类型时，只需在该对话框中继续扩展选项。

### 6.4 DirectoryToolbar：Set Root 操作（新增）

Directory block 打开后，如果 `block.contents.root` 不存在，Toolbar 只显示一个 “Choose Root” 按钮；点击后：
1. 调用 `@tauri-apps/plugin-dialog.open({ directory: true, multiple: false })` 让用户选根目录；
2. 依次发送两条命令：
   - `directory.root`（payload：`{ root, recursive: true, include_hidden: false, max_depth: 3 }`）\n     > 用于把 root/config 写入 `block.contents`；
   - `directory.list { path: ".", recursive: true, include_hidden: false, max_depth: 3 }`，完成首屏加载；
3. 成功后 Toolbar 才展示刷新/创建/删除等其他按钮；失败则提示错误并允许用户重试。

> **当前状态**：仅规划，尚未实现。详见 progress 文档 1.5 节任务。

---

## 7. 测试策略

### 7.1 单元测试 (Vitest)

**测试 directory-operations.ts**:

```typescript
// src/lib/directory-operations.test.ts
import { describe, it, expect, vi } from 'vitest'
import { executeDirectoryList } from './directory-operations'
import { setupCommandMocks } from '@/test/mock-tauri-invoke'

describe('Directory Operations', () => {
  it('should execute directory.list command', async () => {
    const mockEvents = [
      {
        event_id: 'evt1',
        entity: 'block1',
        attribute: 'editor1/directory.list',
        value: {
          entries: [
            { name: 'file1.txt', is_dir: false, is_file: true }
          ],
          last_updated: '2025-01-01T00:00:00Z'
        },
        timestamp: { editor1: 1 }
      }
    ]

    setupCommandMocks({ executeCommand: mockEvents })

    const result = await executeDirectoryList('file1', 'block1', 'editor1', {
      path: '.',
      recursive: false,
      include_hidden: false,
      max_depth: null
    })

    expect(result).toHaveLength(1)
    expect(result[0].value.entries).toHaveLength(1)
  })
})
```

**测试 DirectoryBlock 组件**:

```typescript
// src/components/DirectoryBlock.test.tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { DirectoryBlock } from './DirectoryBlock'
import { createMockBlock } from '@/test/setup'

describe('DirectoryBlock', () => {
  it('should render directory entries', () => {
    const block = createMockBlock({
      block_type: 'directory',
        contents: {
          root: '/test',
          entries: [
            { name: 'file1.txt', is_dir: false, is_file: true }
          ],
          last_updated: '2025-01-01T00:00:00Z'
        }
    })

    render(<DirectoryBlock block={block} fileId="file1" />)

    expect(screen.getByText('file1.txt')).toBeInTheDocument()
  })

  it('should handle refresh button click', async () => {
    const block = createMockBlock({ block_type: 'directory', contents: { root: '/test', entries: [] } })
    const mockExecute = vi.fn()

    // Mock directory operations
    vi.mock('@/lib/directory-operations', () => ({
      executeDirectoryRefresh: mockExecute
    }))

    render(<DirectoryBlock block={block} fileId="file1" />)

    const refreshButton = screen.getByRole('button', { name: /refresh/i })
    fireEvent.click(refreshButton)

    expect(mockExecute).toHaveBeenCalled()
  })
})
```

### 7.2 测试策略

**测试方案**：使用 Vitest 进行完整的测试覆盖

**测试层级**：
1. **单元测试**：buildTree(), findNode(), directory-operations 等函数
2. **组件测试**：使用 @testing-library/react 测试所有 Directory 组件
3. **集成测试**：Mock 后端响应，测试完整操作流程（创建、删除、搜索等）

**测试工具**：
- Vitest：测试运行器（配置在 `vite.config.ts` 第 22-38 行）
- @testing-library/react：组件测试工具
- jsdom：DOM 环境模拟

**参考现有测试**：
- `src/components/ui/button.test.tsx` - 组件测试示例
- `src/test/setup.ts` - Vitest 配置
- `src/test/mock-tauri-invoke.ts` - Tauri Mock

### 7.3 测试覆盖率目标

- **单元测试**: 覆盖所有 directory-operations 函数 (目标 >90%)
- **组件测试**: 覆盖 DirectoryBlock 的所有交互 (目标 >80%)
- **集成测试**: 覆盖 3 个核心场景 (完整创建流程、创建/删除文件、搜索功能)

---

## 8. 分阶段实施计划

### Phase 1: 基础渲染和树构建 (2 天)

**目标**: 显示递归加载的directory tree

- [ ] 确保 Directory Payload 类型在 bindings.ts 中生成（运行 `pnpm tauri dev`）
- [ ] 创建 `DirectoryBlock.tsx` 主组件
- [ ] 实现 `buildTree()` 函数（将平铺entries转为树结构）
- [ ] 实现 `DirectoryTree` 递归渲染组件
- [ ] 实现展开/折叠状态管理（纯前端）
- [ ] 在 `App.tsx` 中集成动态渲染逻辑
- [ ] 测试: 创建directory block，验证初始加载 `directory.list { path: ".", recursive: true, max_depth: 3 }`

**验收标准**:
- 能显示多层目录结构（树形视图）
- 文件和文件夹有不同图标和缩进
- 点击箭头展开/折叠文件夹（不调用后端）
- 单击节点高亮选中

### Phase 2: Refresh 和工具栏 (1 天)

**目标**: 支持全量刷新

- [ ] 创建 `directory-operations.ts` 工具函数
- [ ] 实现 `executeDirectoryRefresh`（payload: `{ recursive: true }`）
- [ ] 创建 `DirectoryToolbar` 组件
- [ ] 添加 Refresh 按钮（重新扫描，清空展开状态）
- [ ] 实现加载状态和错误处理
- [ ] 测试: 外部修改文件系统，点击Refresh验证更新

**验收标准**:
- 点击Refresh按钮调用 `directory.refresh { recursive: true }`
- 后端保持 `include_hidden`、`max_depth` 配置
- 前端清空 `expandedPaths` 和 `selectedPath`
- 显示加载指示器和通知

### Phase 3: Create 和 Delete (1-2 天)

**目标**: 支持文件/文件夹创建和删除

- [ ] 实现 `executeDirectoryCreate` 和 `executeDirectoryDelete`
- [ ] 添加Create File / Create Folder按钮到工具栏
- [ ] 实现输入对话框（使用prompt或shadcn Dialog）
- [ ] 实现路径计算逻辑（选中文件夹内 vs 根目录）
- [ ] 添加Delete按钮（未选中时禁用）
- [ ] 实现确认对话框（文件夹删除提示递归）
- [ ] 操作完成后调用 `handleRefresh()` 更新树
- [ ] 测试: 创建/删除文件、空文件夹、非空文件夹

**验收标准**:
- Create在选中文件夹内创建，未选中时在根目录创建
- Delete文件夹自动设置 `recursive: true`
- 操作后树结构正确更新
- 错误处理和通知完善

### Phase 4: 右键菜单和Rename (1 天)

**目标**: 添加右键菜单，支持重命名

- [ ] 创建 `DirectoryContextMenu` 组件
- [ ] 在TreeNodeItem中集成右键菜单（仅节点上可用）
- [ ] 实现 `executeDirectoryRename`
- [ ] 实现Rename对话框（提示当前名称）
- [ ] 实现路径计算（替换最后一个路径段）
- [ ] 测试: 右键各种节点，重命名文件和文件夹

**验收标准**:
- 空白区域右键无菜单
- 文件夹右键显示Create选项，文件不显示
- Rename正确处理路径（支持子目录）
- Refresh不在右键菜单中

### Phase 5: Search和状态栏 (1 天)

**目标**: 支持搜索和完善状态栏

**任务**：
- [ ] 在 directory-operations.ts 中实现 `executeDirectorySearch`
- [ ] 在 DirectoryBlock 中添加 Search Dialog（使用 shadcn Dialog + Input 内联）
- [ ] 在 DirectoryBlock 中添加搜索状态管理（showSearchDialog, searchResults, isSearching）
- [ ] 工具栏添加 Search 按钮
- [ ] 实现 handleSearch 函数（调用 executeDirectorySearch）
- [ ] 实现搜索结果显示（在 Dialog 中列表渲染）
- [ ] 创建 DirectoryStatusBar.tsx 组件
- [ ] 在 DirectoryStatusBar 中显示 root路径、last_updated、watch_enabled、节点总数
- [ ] 编写搜索功能测试（整合到 DirectoryBlock.test.tsx）

**验收标准**:
- 搜索结果在 Dialog 中显示（不修改主树）
- 支持通配符 `*` 和 `?`（最多10个`*`）
- 状态栏显示所有元数据
- 搜索时显示加载状态
- 搜索测试通过

### Phase 6: Polish和优化 (1 天)

**目标**: 优化交互和性能

- [ ] 优化树渲染性能（React.memo, useMemo）
- [ ] 添加节点hover效果
- [ ] 改进加载状态动画（Skeleton或Spinner）
- [ ] 统一错误提示样式
- [ ] 添加空状态提示（空目录）
- [ ] 处理边界情况（权限错误、路径不存在等）
- [ ] 测试: 大目录（1000+文件）性能

**验收标准**:
- 1000个节点的树流畅渲染
- 所有交互有视觉反馈
- 错误提示清晰友好
- 无明显卡顿

### Phase 7: 测试和文档 (1-2 天)

**目标**: 完善测试和文档

- [ ] 编写 `buildTree()` 和 `findNode()` 单元测试
- [ ] 编写 directory-operations 单元测试
- [ ] 编写 DirectoryBlock 组件测试（@testing-library/react）
- [ ] 编写至少3个 Vitest 集成测试
- [ ] 更新 `docs/guides/FRONTEND_DEVELOPMENT.md`
- [ ] 创建使用示例截图

**验收标准**:
- 单元测试覆盖率 >80%
- 集成测试覆盖：完整创建流程 → 创建/删除文件 → 搜索功能
- 文档准确且易懂

---

## 9. 组件库使用指南

### 9.1 shadcn/ui 组件推荐

**已安装的组件** (参考 `components.json`):

```json
{
  "ui": {
    "button": "@/components/ui/button",
    "dialog": "@/components/ui/dialog",
    "input": "@/components/ui/input",
    "label": "@/components/ui/label",
    "select": "@/components/ui/select",
    "context-menu": "@/components/ui/context-menu",
    "toast": "@/components/ui/toast"
  }
}
```

**Directory 功能需要的新组件**:

```bash
# 逐个安装（shadcn CLI 不支持批量）
npx shadcn@latest add dialog
npx shadcn@latest add context-menu
npx shadcn@latest add tooltip
```

**说明**：
- 每个组件安装时会自动添加相关依赖到 package.json
- 组件文件会生成在 `src/components/ui/` 目录
- 安装命令使用 `shadcn@latest`，不是 `shadcn-ui@latest`

**组件用途**：
- `dialog`: Search Dialog（Phase 5.2）
- `context-menu`: 右键菜单（Phase 4.1）
- `tooltip`: StatusBar路径截断显示（Phase 5.3）

### 9.2 Lucide React 图标

**推荐使用的图标**:

```typescript
import {
  Folder,           // 文件夹
  File,             // 文件
  FileText,         // 文本文件
  FileCode,         // 代码文件
  Image,            // 图片文件
  RefreshCw,        // 刷新
  Plus,             // 创建
  Trash2,           // 删除
  Edit3,            // 重命名
  Search,           // 搜索
  FolderPlus,       // 创建文件夹
  Eye,              // Watch 启用
  EyeOff,           // Watch 禁用
  ChevronRight,     // 折叠箭头
  ChevronDown,      // 展开箭头
  Loader2,          // 加载指示器
} from 'lucide-react'
```

### 9.3 Tailwind 样式建议

**常用样式类**:

```css
/* 布局 */
.flex .flex-col .h-full
.overflow-y-auto
.space-y-2

/* 交互状态 */
.hover:bg-accent
.cursor-pointer
.border-l-2 .border-primary  /* 选中状态 */

/* 文字 */
.text-sm .text-xs
.text-muted-foreground
.font-mono  /* 文件路径 */
.truncate   /* 长文件名截断 */

/* 间距 */
.p-2 .px-4 .py-1
.gap-2 .space-x-2
```

---

## 10. 常见问题和解决方案

### Q1: bindings.ts 中没有 Directory Payload 类型

**A**: 运行 `pnpm tauri dev` 启动应用即可自动生成。Tauri Specta 在应用启动时导出类型，不是在编译时。所有 Directory Payload 类型已在 `src-tauri/src/lib.rs` 中正确注册（第 54-60 行），无需手动维护。

### Q2: executeCommand 返回错误 "Capability not found"

**A**: 检查：
1. Capability 是否在 `registry.rs` 中注册
2. `cap_id` 是否拼写正确（应为 `"directory.list"` 而非 `"DirectoryList"`）
3. Backend 是否重新编译

### Q3: Block.contents 不更新

**A**: 确认：
1. Backend 返回的 Event 是否包含正确的 `value` (新的 contents)
2. DirectoryBlock 是否在操作完成后重新触发 `handleRefresh()`（它会调用 `executeDirectoryList/Refresh` 并推动最新 contents）
3. useEffect 的依赖数组是否包含 `contents`

### Q4: 文件路径安全问题

**A**: Backend 已使用 `canonicalize()` 防止路径遍历攻击。Frontend 无需额外验证。如需改善用户体验：
- 在输入验证中拒绝包含 `..` 的文件名
- 检测绝对路径并提示用户使用相对路径

### Q5: 如何运行集成测试

**A**: 使用 Vitest 运行集成测试：
```bash
# 运行所有测试
pnpm test

# 运行特定集成测试
pnpm test -- DirectoryBlock.integration.test.tsx

# Watch 模式
pnpm test -- --watch
```

---

## 11. 性能优化建议

### 11.1 大目录处理

**问题**: 如果目录有 10000+ 文件，渲染会卡顿

**解决方案**:

1. **虚拟滚动**: 使用 `react-window` 或 `@tanstack/react-virtual`
   ```bash
   pnpm add @tanstack/react-virtual
   ```

2. **分页**: 修改 backend list payload 增加 `offset` 和 `limit`
   ```rust
   pub struct DirectoryListPayload {
       pub root: String,
       pub recursive: bool,
       pub include_hidden: bool,
       pub max_depth: Option<usize>,
       pub offset: Option<usize>,  // 新增
       pub limit: Option<usize>,   // 新增
   }
   ```

3. **懒加载**: 默认只加载第一层，点击文件夹时再加载子文件夹

### 11.2 防抖搜索（可选）

**问题**: 用户输入搜索词时频繁调用 backend

**解决方案**: 使用 debounce（可选，用于性能优化）

```typescript
import { useDebounce } from '@/hooks/useDebounce'

const [searchPattern, setSearchPattern] = useState('')
const debouncedPattern = useDebounce(searchPattern, 500)

useEffect(() => {
  if (debouncedPattern) {
    handleSearch(debouncedPattern)
  }
}, [debouncedPattern])
```

### 11.3 缓存优化

**问题**: 频繁 refresh 浪费资源

**解决方案**: 使用 `last_updated` 判断是否需要刷新

```typescript
const handleRefreshIfStale = async () => {
  const lastUpdated = new Date(contents.last_updated)
  const now = new Date()
  const diffMinutes = (now.getTime() - lastUpdated.getTime()) / 60000

  if (diffMinutes < 5) {
    // 5 分钟内不刷新
    return
  }

  await handleRefresh()
}
```

---

## 12. 总结

### 关键要点

1. **递归加载 + 前端构建树**: 初始加载多层（max_depth: 3），前端解析为树结构，展开/折叠为纯前端状态切换
2. **类型安全第一**: 通过 `pnpm tauri dev` 生成 Directory Payload 类型到 bindings.ts
3. **Refresh保持配置**: 使用 `directory.refresh { recursive: true }` 保持 include_hidden 和 max_depth 配置
4. **测试驱动**: 从 Phase 1 开始编写测试，保持 >80% 覆盖率
5. **VSCode风格交互**: 工具栏操作 + 节点右键菜单，展开/折叠不调用后端

### 开发检查清单

开始开发前：
- [ ] 确认 `pnpm tauri dev` 能正常启动
- [ ] 确认 Directory Payload 类型在 `src/bindings.ts` 中存在
- [ ] 阅读 `docs/guides/FRONTEND_DEVELOPMENT.md`
- [ ] 阅读现有 `BlockEditor.tsx` 和 `BlockList.tsx` 代码

开发中：
- [ ] 每个功能完成后编写单元测试
- [ ] 提交前运行 `pnpm test` 和 `pnpm lint`
- [ ] 使用 `console.log` 调试 backend 通信
- [ ] 频繁测试实际交互（手动测试）

完成后：
- [ ] 所有 Phase 1-7 验收标准通过
- [ ] 测试覆盖率 >80%
- [ ] Vitest 集成测试通过
- [ ] 更新 `docs/analyze/extension/directory-fe-progress.md`

---

## 附录A: 完整文件结构

```
src/
├── components/
│   ├── ui/                       # shadcn/ui 基础组件
│   ├── BlockEditor.tsx           # Markdown 编辑器 (已有)
│   ├── BlockList.tsx             # Block 列表 (已有)
│   ├── DirectoryBlock.tsx        # Directory 主组件 (新增)
│   ├── DirectoryBlock.test.tsx   # DirectoryBlock 单元测试 (新增)
│   ├── DirectoryBlock.integration.test.tsx # DirectoryBlock 集成测试 (新增)
│   ├── DirectoryTree.tsx         # 文件树视图 (新增)
│   ├── DirectoryTree.test.tsx    # DirectoryTree 测试 (新增)
│   ├── DirectoryToolbar.tsx      # 工具栏 (新增)
│   ├── DirectoryToolbar.test.tsx # DirectoryToolbar 测试 (新增)
│   ├── DirectoryStatusBar.tsx    # 状态栏 (新增)
│   ├── DirectoryStatusBar.test.tsx # DirectoryStatusBar 测试 (新增)
│   ├── DirectoryContextMenu.tsx  # 右键菜单 (新增)
│   └── DirectoryContextMenu.test.tsx # DirectoryContextMenu 测试 (新增)
├── lib/
│   ├── app-store.ts              # Zustand store (已有)
│   ├── tauri-client.ts           # Tauri 封装 (已有)
│   ├── directory-operations.ts   # Directory 操作函数 (新增)
│   ├── directory-operations.test.ts # 操作函数测试 (新增)
│   ├── directory-utils.ts        # buildTree, findNode (新增)
│   ├── directory-utils.test.ts   # 工具函数测试 (新增)
│   └── utils.ts                  # 通用工具函数 (已有)
├── hooks/
│   └── useDebounce.ts            # Debounce hook (可选，用于性能优化)
├── test/
│   ├── setup.ts                  # Vitest 配置 (已有)
│   └── mock-tauri-invoke.ts      # Tauri Mock (已有)
├── App.tsx                       # 主应用 (修改)
└── bindings.ts                   # 自动生成 (验证)

docs/analyze/extension/
├── directory-fe.md               # 本文档
└── directory-fe-progress.md      # 进度跟踪文档
```

**说明**:
- 测试文件和组件文件在同一目录（遵循项目惯例，参考 `src/components/ui/button.test.tsx`）
- 集成测试使用 `.integration.test.tsx` 后缀
- useDebounce.ts 为可选组件，用于搜索防抖优化（Phase 6）
- 专注于 Vitest 测试框架（单元测试 + 组件测试 + 集成测试）

---

## 附录B: 参考资源

**项目文档**:
- `docs/guides/FRONTEND_DEVELOPMENT.md` - Tauri Specta 使用指南
- `docs/concepts/ARCHITECTURE_OVERVIEW.md` - 整体架构
- `docs/guides/EXTENSION_DEVELOPMENT.md` - 扩展开发指南

**外部文档**:
- [Tauri 2 Documentation](https://v2.tauri.app/learn/)
- [Tauri Specta](https://github.com/oscartbeaumont/tauri-specta)
- [Zustand Documentation](https://docs.pmnd.rs/zustand/getting-started/introduction)
- [shadcn/ui Components](https://ui.shadcn.com/docs/components)
- [Lucide Icons](https://lucide.dev/icons/)
- [Vitest Documentation](https://vitest.dev/)
- [React Testing Library](https://testing-library.com/react)

**代码参考**:
- `src/components/BlockEditor.tsx` - Markdown block 实现模式
- `src/lib/app-store.ts` - 状态管理和 command 执行
- `src-tauri/src/extensions/markdown/` - Markdown extension 参考

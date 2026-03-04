# Directory Extension 前端开发进度

本文档记录 directory extension 前端开发进度，遵循 TDD (Test-Driven Development) 原则。

**更新日期**：2025-11-07

**前端架构**：递归加载 + 前端构建树 + 纯状态切换

---

## 总体进度

### 核心功能
- [ ] 基础渲染和树构建（Phase 1）
- [ ] Refresh 和工具栏（Phase 2）
- [ ] Create 和 Delete（Phase 3）
- [ ] 右键菜单和 Rename（Phase 4）
- [ ] Search 和状态栏（Phase 5）
- [ ] Polish 和优化（Phase 6）
- [ ] 测试和文档（Phase 7）

### 测试覆盖
- [ ] 单元测试覆盖率 >80%
- [ ] 组件测试通过
- [ ] 集成测试通过（至少3个场景）

---

## 准备工作

### shadcn 组件安装

**首次开发前需要安装 UI 组件**：

```bash
# 逐个安装（shadcn CLI 不支持批量）
npx shadcn@latest add dialog
npx shadcn@latest add context-menu
npx shadcn@latest add tooltip
```

**验证安装**：
```bash
# 检查组件文件是否生成
ls src/components/ui/ | grep -E "dialog|context-menu|tooltip"
```

**说明**：
- 每个组件安装时会自动添加相关依赖到 package.json
- 无需手动添加 npm 包（项目已使用 `crypto.randomUUID()` 生成 ID）

### 环境验证

**验证 Tauri 开发环境**：
```bash
cd /home/yaosh/projects/elfiee
pnpm tauri dev
```

**验证清单**：
- [ ] shadcn 组件已安装（dialog, context-menu, tooltip）
- [ ] 应用成功启动（端口 1420）
- [ ] 前端热重载正常
- [ ] 后端编译成功
- [ ] 无编译错误或警告

### TypeScript 类型生成

**生成 Directory Payload 类型**：
```bash
pnpm tauri dev  # 应用启动时自动生成
```

**验证 `src/bindings.ts`**：
```bash
grep -E "export type Directory.*Payload" src/bindings.ts
```

**验证清单**：
- [ ] `DirectoryListPayload` 存在
- [ ] `DirectoryCreatePayload` 存在
- [ ] `DirectoryDeletePayload` 存在
- [ ] `DirectoryRenamePayload` 存在
- [ ] `DirectoryRefreshPayload` 存在
- [ ] `DirectoryWatchPayload` 存在
- [ ] `DirectorySearchPayload` 存在

**确认类型定义正确**：
```typescript
// 应该看到：
export type DirectoryListPayload = {
  path: string
  recursive?: boolean
  include_hidden?: boolean
  max_depth?: number | null
}
// ... 其他6个类型
```

---

## Phase 1: 基础渲染和树构建 (2 天)

**目标**：显示递归加载的 directory tree

**说明**：类型定义遵循项目惯例，直接在组件文件内部定义（参考BlockEditor.tsx），不创建单独的src/types/directory.ts文件。

### 1.1 TDD: buildTree 函数 ✅（已完成）

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [x] 创建 `src/lib/directory-utils.test.ts`
- [x] 编写测试：空 entries → 返回空树
- [x] 编写测试：单层文件 → 正确树结构
- [x] 编写测试：多层嵌套 → 正确父子关系
- [x] 编写测试：路径排序 → 父节点优先
- [x] 编写测试：深度计算 → depth 正确
- [x] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [x] 创建 `src/lib/directory-utils.ts`
- [x] 定义 TreeNode 和 DirectoryEntry 接口（在文件顶部）
- [x] 实现 `buildTree(entries: DirectoryEntry[]): TreeNode`
- [x] 按 path 长度排序（确保父节点先处理）
- [x] 递归构建子节点
- [x] 计算节点深度
- [x] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [x] 优化性能
- [x] 确保测试仍然通过

**预期覆盖率**：>90%

### 1.2 TDD: DirectoryBlock 主组件（Red ✅）

**文件**：`src/components/DirectoryBlock.tsx`

**任务**：
- [x] 创建组件骨架（接受 block 和 fileId props）
- [x] 实现状态管理：
  - [x] `tree: TreeNode | null`
  - [x] `expandedPaths: Set<string>`
  - [x] `selectedPath: string | null`
  - [x] `isRefreshing: boolean`
- [x] 实现 useEffect 监听 `contents.entries` 变化
- [x] 调用 `buildTree()` 构建树
- [x] 实现三段式布局：
  - [x] `<DirectoryToolbar />` (顶部)
  - [x] `<DirectoryTree />` (中间)
  - [x] `<DirectoryStatusBar />` (底部)

**测试**：`src/components/DirectoryBlock.test.tsx`
- [x] 测试组件挂载（Red → Green ✅）
- [x] 测试 contents 更新触发 buildTree（Red → Green ✅）
- [x] 测试 tree 状态正确（Red → Green ✅）
- [x] 测试布局渲染（Red → Green ✅）

### 1.3 TDD: DirectoryTree 组件

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [x] 创建 `src/components/DirectoryTree.test.tsx`
- [x] 编写测试：空树渲染显示"Loading..."
- [x] 编写测试：单层文件渲染
- [x] 编写测试：多层递归渲染
- [x] 编写测试：展开/折叠交互
- [x] 编写测试：选中状态高亮
- [x] 编写测试：缩进样式正确
- [x] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [x] 创建 `src/components/DirectoryTree.tsx`
- [x] 创建 `DirectoryTree` 组件（接受 tree, expandedPaths, selectedPath, onToggleExpand, onSelect props）
- [x] 创建 `TreeNodeItem` 递归组件
- [x] 实现节点渲染：
  - [x] 文件夹图标 vs 文件图标（暂用箭头字符占位）
  - [x] 展开箭头（ChevronRight/ChevronDown 占位符）
  - [x] 缩进（`paddingLeft: ${depth * 16 + 8}px`）
  - [x] 选中高亮（`border-l-2 border-primary`）
- [x] 实现交互：
  - [x] 点击节点 → onSelect(path)
  - [x] 点击箭头 → onToggleExpand(path) + stopPropagation
- [x] 递归渲染子节点
- [x] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 提取共通样式
- [ ] 确保测试仍然通过

### 1.5 BlockTypeDialog 扩展 Directory 类型 ✅（已完成）

**目标**：复用当前 BlockList 创建流程，只在 BlockTypeDialog 内新增 `Directory` 选项，并确保 BlockList 传递正确的类型。Toolbar 的 "Choose Root" / `directory.list` 逻辑仍属于 Phase 2，不在本阶段实现。

**涉及文件**：`src/components/BlockTypeDialog.tsx`, `src/components/BlockList.tsx`, `src/components/BlockList.test.tsx`

**说明**：
- BlockList 入口已可复用，测试集中在 BlockTypeDialog 以及 BlockList 的选择逻辑即可。
- 目录类型创建时仍调用 `createBlock(activeFileId, name, 'directory')`，后续 root 设置由 Toolbar 触发（Phase 2）。
- 所有新增行为以已有测试文件为基础进行扩展（无需新建测试文件）。

**TDD 流程**：

**步骤 1 - Red（扩展测试）**：
- [x] `src/components/BlockList.test.tsx`
  - [x] 更新 "shows block creation dialog…" 测试：断言 Directory 选项存在但在本测试中仍默认 Markdown。
  - [x] 新增断言：切换不同 BlockType 卡片时按钮状态正确。
- [x] `src/components/BlockTypeDialog.test.tsx`
  - [x] 覆盖 Directory 流程：选择 Directory → 点击 Create → 断言 `createBlock` 被调用且 payload type 为 `directory`。
  - [x] 确认成功/失败通知沿用 BlockList 现有逻辑（可通过 `mockStore.addNotification` 断言）。
- [x] 运行 `pnpm test BlockList`，预期失败（Red）。

**步骤 2 - Green（实现 BlockTypeDialog 扩展）**：
- [x] `src/components/BlockTypeDialog.tsx`
  - [x] 在 `BLOCK_TYPES` 数组中新增 `{ id: 'directory', name: 'Directory', description: 'Browse local folder tree', icon: FolderTree }`。
  - [x] 确认选择 Directory 后 `onConfirm(name, 'directory')` 被调用，按钮可用状态取决于名称 + 选项。
- [x] `src/components/BlockList.tsx`
  - [x] 无需新增状态，仅确认 `handleConfirmCreate` 可以向 store 传入 `'directory'`。
- [x] 运行相关测试确保通过（Green）。

**步骤 3 - Refactor / 验收**：
- [x] 检查 UI 文案与 `directory-fe.md` 描述一致（"New Block" → Dialog → Directory 选项 → Create）。
- [x] 更新文档（本条即为记录），确认 Phase 1.5 范围仅限 Dialog。
- [x] 提交代码前运行 `pnpm test BlockList`。

**验收标准**：
- [x] Dialog 中可见 Directory 卡片，点击后按钮可点。
- [x] 创建 Directory block 时仅调用 `createBlock`，不触发任何 root/list 逻辑。
- [x] 所有扩展测试通过。

### 1.6 集成到 App.tsx（已完成接入，待手动验证）

**文件**：`src/App.tsx`

**任务**：
- [x] 导入 `DirectoryBlock` 组件并在 `App.tsx` 中复用
- [x] 基于当前选中 block 的 `block_type` 判断渲染逻辑：`directory` 走 `<DirectoryBlock />`，其余维持 `BlockEditor`
- [x] 当 `block_type` 不在 `['markdown', 'terminal', 'directory']` 内时渲染统一的 Unsupported 提示

**测试**：手动（待执行）
- [ ] 创建 directory block（通过 BlockList → Directory 类型）
- [ ] 验证初始加载（`contents.entries` 构建树是否渲染在 DirectoryBlock 中）
- [ ] 验证树形视图显示、展开/折叠、选中状态
- [ ] 切换到非 directory block，确认 BlockEditor 正常渲染
- [ ] 切换到未知 block_type，确认显示 Unsupported 提示

### 1.7 Phase 1 验收

**验收标准**：
- [ ] 能显示多层目录结构（树形视图）
- [ ] 文件和文件夹有不同图标和缩进
- [ ] 点击箭头展开/折叠文件夹（不调用后端）
- [ ] 单击节点高亮选中
- [ ] 所有单元测试通过
- [ ] 组件测试通过

---

## Phase 2: Refresh 和工具栏 (1 天)

**目标**：支持全量刷新

### 2.1 创建 directory-operations

**文件**：`src/lib/directory-operations.ts`

**任务**：
- [ ] 实现 `executeDirectoryRefresh()`
  ```typescript
  export async function executeDirectoryRefresh(
    fileId: string,
    blockId: string,
    editorId: string,
    payload: DirectoryRefreshPayload
  ): Promise<Event[]>
  ```
- [ ] 构建 Command 对象
- [ ] 调用 `TauriClient.block.executeCommand()`
- [ ] 错误处理

**测试**：`src/lib/directory-operations.test.ts`
- [ ] Mock `executeCommand`（使用 `setupCommandMocks`）
- [ ] 测试 payload 正确
- [ ] 测试 command 结构
- [ ] 测试错误处理

### 2.2 创建 DirectoryToolbar 组件

**文件**：`src/components/DirectoryToolbar.tsx`

**任务**：
- [ ] 创建组件（接受 handlers、状态 props、`hasRoot` 标记）
- [ ] 当 `hasRoot === false` 时只渲染 “Choose Root” 按钮，点击触发 `onChooseRoot`
- [ ] 当 `hasRoot === true` 时渲染完整工具栏：
  - [ ] Refresh (RefreshCw 图标)
  - [ ] Create File (File 图标)
  - [ ] Create Folder (FolderPlus 图标)
  - [ ] Delete (Trash2 图标，未选中时禁用)
  - [ ] Search (Search 图标)
- [ ] 根据 `isRefreshing` 显示加载状态（按钮禁用 + Spinner）
- [ ] 使用 shadcn Button 组件

**测试**：`src/components/DirectoryToolbar.test.tsx`
- [ ] root 未设置时仅显示 “Choose Root”
- [ ] root 设置后按钮全部可见且可点击
- [ ] 测试 Delete 禁用状态、加载状态

### 2.3 实现 handleRefresh

**文件**：`src/components/DirectoryBlock.tsx`

**任务**：
- [ ] 实现 `handleRefresh()` 函数
- [ ] 调用 `executeDirectoryRefresh(fileId, blockId, editorId, { recursive: true })`
- [ ] 清空状态：
  - [ ] `setExpandedPaths(new Set())`
  - [ ] `setSelectedPath(null)`
- [ ] 错误处理和通知
- [ ] 加载状态管理
- [ ] 当 `contents.root` 为空时，禁用 refresh 并提示用户先设置 root

**测试**：集成到组件测试
- [ ] 测试调用 executeDirectoryRefresh
- [ ] 测试状态清空
- [ ] 测试错误处理

### 2.4 Phase 2 验收

**验收标准**：
- [ ] 点击 Refresh 调用 `directory.refresh { recursive: true }`
- [ ] 后端保持 `include_hidden`、`max_depth` 配置
- [ ] 前端清空 `expandedPaths` 和 `selectedPath`
- [ ] 显示加载指示器和通知
- [ ] root 未设置前，Toolbar 只显示 “Choose Root”，其它操作不可用
- [ ] 外部修改文件系统后 Refresh 能更新树

---

## Phase 3: Create 和 Delete (1-2 天)

**目标**：支持文件/文件夹创建和删除

### 3.1 TDD: directory-operations 函数（Create 和 Delete）

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 创建 `src/lib/directory-operations.test.ts`
- [ ] 编写测试：`executeDirectoryCreate` 生成正确的 Create payload
- [ ] 编写测试：`executeDirectoryDelete` 生成正确的 Delete payload
- [ ] 编写测试：Delete recursive 参数正确传递
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 在 `src/lib/directory-operations.ts` 中实现 `executeDirectoryCreate()`
- [ ] 在 `src/lib/directory-operations.ts` 中实现 `executeDirectoryDelete()`
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 提取共通逻辑
- [ ] 确保测试仍然通过

### 3.2 TDD: findNode 辅助函数

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 在 `src/lib/directory-utils.test.ts` 中添加测试
- [ ] 编写测试：根节点查找（path = "" 或 "."）返回 root
- [ ] 编写测试：深层节点查找（path = "a/b/c"）返回正确节点
- [ ] 编写测试：不存在节点（path = "nonexistent"）返回 null
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 在 `src/lib/directory-utils.ts` 中实现 `findNode(node: TreeNode, path: string): TreeNode | null`
- [ ] 使用递归查找算法
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 优化递归性能
- [ ] 确保测试仍然通过

### 3.3 TDD: handleCreateFile 功能

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 在 `src/components/DirectoryBlock.test.tsx` 中添加测试
- [ ] Mock `window.prompt` 返回 "newfile.txt"
- [ ] 编写测试：选中文件夹时路径计算为 `selectedPath/newfile.txt`
- [ ] 编写测试：未选中时路径计算为 `newfile.txt`
- [ ] 编写测试：调用 `executeDirectoryCreate` 传递正确参数
- [ ] 编写测试：调用 `handleRefresh` 刷新树
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 在 DirectoryBlock 中实现 `handleCreateFile()` 函数
- [ ] 使用 `window.prompt('Enter file name:')` 获取文件名
- [ ] 实现路径计算逻辑：
  - 选中文件夹：`${selectedPath}/${fileName}`
  - 未选中：`${fileName}`（根目录）
- [ ] 调用 `executeDirectoryCreate(path, 'file', fileId, blockId, editorId)`
- [ ] 调用 `handleRefresh()`
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 添加错误处理（用户取消输入）
- [ ] 添加成功/失败通知
- [ ] 确保测试仍然通过

### 3.4 TDD: handleCreateFolder 功能

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 在 `src/components/DirectoryBlock.test.tsx` 中添加测试
- [ ] Mock `window.prompt` 返回 "newfolder"
- [ ] 编写测试：路径计算逻辑正确
- [ ] 编写测试：调用 `executeDirectoryCreate` 时 `item_type: 'dir'`
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 在 DirectoryBlock 中实现 `handleCreateFolder()` 函数
- [ ] 复用 handleCreateFile 的路径计算逻辑
- [ ] 调用 `executeDirectoryCreate(path, 'dir', fileId, blockId, editorId)`
- [ ] 调用 `handleRefresh()`
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 提取路径计算为共通函数 `calculateTargetPath()`
- [ ] 确保测试仍然通过

### 3.5 TDD: handleDelete 功能

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 在 `src/components/DirectoryBlock.test.tsx` 中添加测试
- [ ] Mock `window.confirm` 返回 true
- [ ] 编写测试：文件夹删除时 `recursive: true`
- [ ] 编写测试：文件删除时 `recursive: false`
- [ ] 编写测试：未选中节点时不执行删除
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 在 DirectoryBlock 中实现 `handleDelete()` 函数
- [ ] 检查 `selectedPath` 存在，不存在则直接返回
- [ ] 使用 `findNode(treeRoot, selectedPath)` 获取节点
- [ ] 使用 `window.confirm()` 确认删除（文件夹提示"递归删除所有内容"）
- [ ] 调用 `executeDirectoryDelete(selectedPath, node.is_dir, fileId, blockId, editorId)`
- [ ] 调用 `handleRefresh()`
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 添加错误处理
- [ ] 添加删除成功通知
- [ ] 确保测试仍然通过

### 3.6 Phase 3 验收

**验收标准**：
- [ ] Create 在选中文件夹内创建，未选中时在根目录创建
- [ ] Delete 文件夹自动设置 `recursive: true`
- [ ] 操作后树结构正确更新
- [ ] 确认对话框显示正确信息
- [ ] 错误处理和通知完善

---

## Phase 4: 右键菜单和 Rename (1 天)

**目标**：添加右键菜单，支持重命名

### 4.1 TDD: DirectoryContextMenu 组件

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 创建 `src/components/DirectoryContextMenu.test.tsx`
- [ ] 编写测试：文件夹节点显示 Create File、Create Folder、Rename、Delete
- [ ] 编写测试：文件节点只显示 Rename、Delete（不显示 Create）
- [ ] 编写测试：点击菜单项调用对应 handler
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 创建 `src/components/DirectoryContextMenu.tsx`
- [ ] 使用 shadcn ContextMenu 组件
- [ ] 接受 props: `node: TreeNode`, `handlers: { onCreateFile, onCreateFolder, onRename, onDelete }`, `children: ReactNode`
- [ ] 条件渲染菜单项：
  - 文件夹：`node.is_dir === true` 时显示 Create File、Create Folder
  - 所有节点：显示 Rename、Delete
- [ ] 包装 `children` 在 `<ContextMenuTrigger>` 中
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 添加菜单项图标
- [ ] 优化样式
- [ ] 确保测试仍然通过

### 4.2 TDD: 集成右键菜单到 TreeNodeItem

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 在 `src/components/DirectoryTree.test.tsx` 中添加测试
- [ ] 编写测试：TreeNodeItem 被 DirectoryContextMenu 包装
- [ ] 编写测试：右键点击节点触发菜单显示
- [ ] 编写测试：空白区域无右键菜单
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 在 `src/components/DirectoryTree.tsx` 中修改 TreeNodeItem 渲染
- [ ] 包装 TreeNodeItem 内容到 `<DirectoryContextMenu>`
- [ ] 传递 handlers: `onCreateFile`, `onCreateFolder`, `onRename`, `onDelete`
- [ ] 确保空白区域不渲染 ContextMenu（只有 TreeNodeItem 被包装）
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 确保测试仍然通过

### 4.3 TDD: executeDirectoryRename 函数

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 在 `src/lib/directory-operations.test.ts` 中添加测试
- [ ] 编写测试：`executeDirectoryRename` 生成正确的 Rename payload
- [ ] 编写测试：old_path 和 new_path 正确传递
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 在 `src/lib/directory-operations.ts` 中实现 `executeDirectoryRename()`
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 确保测试仍然通过

### 4.4 TDD: handleRename 功能

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 在 `src/components/DirectoryBlock.test.tsx` 中添加测试
- [ ] Mock `window.prompt` 返回 "newname.txt"
- [ ] 编写测试：根目录文件重命名（path = "file.txt" → "newname.txt"）
- [ ] 编写测试：子目录文件重命名（path = "a/b/file.txt" → "a/b/newname.txt"）
- [ ] 编写测试：调用 `executeDirectoryRename` 传递正确参数
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 在 DirectoryBlock 中实现 `handleRename(node: TreeNode)` 函数
- [ ] 使用 `window.prompt('Enter new name:', currentName)` 获取新名称
- [ ] 计算新路径（替换最后一个路径段）：
  ```typescript
  const pathParts = node.path.split('/')
  pathParts[pathParts.length - 1] = newName
  const newPath = pathParts.join('/')
  ```
- [ ] 调用 `executeDirectoryRename(node.path, newPath, fileId, blockId, editorId)`
- [ ] 调用 `handleRefresh()`
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 添加错误处理（用户取消、空名称）
- [ ] 添加成功通知
- [ ] 确保测试仍然通过

### 4.5 Phase 4 验收

**验收标准**：
- [ ] 空白区域右键无菜单
- [ ] 文件夹右键显示 Create 选项，文件不显示
- [ ] Rename 正确处理路径（支持子目录）
- [ ] Refresh 不在右键菜单中
- [ ] 所有右键操作正常工作

---

## Phase 5: Search 和状态栏 (1 天)

**目标**：支持搜索和完善状态栏

### 5.1 TDD: executeDirectorySearch 函数

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 在 `src/lib/directory-operations.test.ts` 中添加测试
- [ ] 编写测试：`executeDirectorySearch` 生成正确的 Search payload
- [ ] 编写测试：pattern 和 recursive 参数正确传递
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 在 `src/lib/directory-operations.ts` 中实现 `executeDirectorySearch()`
- [ ] 返回 Event[]（结果在 Event.value.matches）
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 确保测试仍然通过

### 5.2 TDD: Search Dialog 和 handleSearch

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 在 `src/components/DirectoryBlock.test.tsx` 中添加测试
- [ ] 编写测试：点击 Search 按钮打开 Dialog
- [ ] 编写测试：Dialog 关闭按钮功能
- [ ] 编写测试：输入模式并按 Enter 键触发搜索
- [ ] 编写测试：搜索结果正确显示在 Dialog 中
- [ ] 编写测试：调用 `executeDirectorySearch` 传递正确参数
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 在 DirectoryBlock 中添加状态：
  - `showSearchDialog: boolean`（初始值 false）
  - `searchResults: DirectoryEntry[]`（初始值 []）
  - `isSearching: boolean`（初始值 false）
- [ ] 在 DirectoryBlock JSX 中添加 Search Dialog UI（使用 shadcn Dialog + Input，内联在组件中）
- [ ] 实现 `handleSearch(pattern: string)` 函数：
  - 设置 `isSearching = true`
  - 调用 `executeDirectorySearch(pattern, true, fileId, blockId, editorId)`
  - 提取 `Event[0].value.matches`
  - 更新 `searchResults` 状态
  - 设置 `isSearching = false`
- [ ] 在 Dialog 中渲染搜索结果列表
- [ ] 显示通知（"Found X matches"）
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 添加错误处理
- [ ] 添加空结果提示
- [ ] 确保测试仍然通过

### 5.3 TDD: DirectoryStatusBar 组件

**TDD 流程**：

**步骤 1 - Red（编写测试）**：
- [ ] 创建 `src/components/DirectoryStatusBar.test.tsx`
- [ ] 编写测试：显示 root 路径（截断显示）
- [ ] 编写测试：鼠标悬停显示完整路径
- [ ] 编写测试：显示 Last updated 相对时间（如 "2 minutes ago"）
- [ ] 编写测试：Watch 状态图标（Eye/EyeOff）
- [ ] 编写测试：显示节点总数
- [ ] 运行测试确认失败（Red）

**步骤 2 - Green（编写实现）**：
- [ ] 创建 `src/components/DirectoryStatusBar.tsx`
- [ ] 接受 props: `root: string`, `lastUpdated: number`, `watchEnabled: boolean`, `totalNodes: number`
- [ ] 实现路径截断显示（超过40字符截断）
- [ ] 使用 Tooltip 显示完整路径
- [ ] 实现相对时间格式化（使用 date-fns 或手动实现）
- [ ] 使用 Eye/EyeOff 图标显示 Watch 状态
- [ ] 显示节点总数
- [ ] 运行测试确认通过（Green）

**步骤 3 - Refactor（重构优化）**：
- [ ] 优化样式
- [ ] 确保测试仍然通过

### 5.4 集成 StatusBar 到 DirectoryBlock

**文件**：`src/components/DirectoryBlock.tsx`

**任务**：
- [ ] 在 DirectoryBlock 底部渲染 `<DirectoryStatusBar>`
- [ ] 传递 props: `root`, `lastUpdated`, `watchEnabled`, `totalNodes`
- [ ] 计算 totalNodes: `treeRoot ? countNodes(treeRoot) : 0`

### 5.5 Phase 5 验收

**验收标准**：
- [ ] 搜索结果在独立 Dialog 显示（不修改主树）
- [ ] 支持通配符 `*` 和 `?`（最多10个 `*`）
- [ ] 状态栏显示所有元数据
- [ ] 搜索时显示加载状态
- [ ] 搜索结果正确

---

## Phase 6: Polish 和优化 (1 天)

**目标**：优化交互和性能

### 6.1 性能优化（测试驱动）

**测试先行**：

**步骤 1 - 编写性能测试**：
- [ ] 在 `src/components/DirectoryTree.test.tsx` 中添加性能测试
- [ ] 编写测试：渲染 1000 个节点的树（时间 <100ms）
- [ ] 编写测试：展开/折叠操作响应时间 <50ms
- [ ] 运行测试确认性能基准

**步骤 2 - 实现优化**：
- [ ] 在 `src/components/DirectoryTree.tsx` 中使用 `React.memo` 包装 TreeNodeItem
- [ ] 在 `src/components/DirectoryBlock.tsx` 中使用 `useMemo` 缓存 buildTree 结果
- [ ] 在 DirectoryBlock 中使用 `useCallback` 包装所有 handlers
- [ ] 运行性能测试确认改进

**步骤 3 - 验证**：
- [ ] 使用 React DevTools Profiler 测量渲染性能
- [ ] 确保性能测试通过

### 6.2 视觉优化

**文件**：`src/components/DirectoryTree.tsx`, `DirectoryBlock.tsx`, `DirectoryToolbar.tsx`

**任务**：
- [ ] 在 TreeNodeItem 添加 hover 效果（`hover:bg-accent` 类）
- [ ] 改进加载状态：使用 Spinner 或 Skeleton 组件
- [ ] 统一错误提示：使用 shadcn Toast 组件
- [ ] 添加空状态提示：空目录显示 "No files" 占位符
- [ ] 添加展开/折叠平滑过渡动画（CSS transition）

### 6.3 边界情况处理

**文件**：`src/components/DirectoryBlock.tsx`, `src/lib/directory-operations.ts`

**任务**：
- [ ] 权限错误处理：后端返回错误时显示友好提示（Toast）
- [ ] 路径不存在处理：捕获错误并提示用户
- [ ] 文件名冲突处理：Create 时检测冲突并提示用户
- [ ] 超长文件名截断：超过50字符显示省略号（...）
- [ ] 特殊字符处理：验证文件名不包含 `/`, `\`, `:` 等非法字符

### 6.4 Phase 6 验收

**验收标准**：
- [ ] 1000 个节点的树流畅渲染（<100ms）
- [ ] 所有交互有视觉反馈
- [ ] 错误提示清晰友好
- [ ] 无明显卡顿
- [ ] 空状态友好

---

## Phase 7: 测试和文档 (1-2 天)

**目标**：完善测试和文档

### 7.1 单元测试

**测试覆盖**：
- [ ] `buildTree()` 单元测试（>90% 覆盖率）
- [ ] `findNode()` 单元测试（100% 覆盖率）
- [ ] `directory-operations.ts` 单元测试（>80% 覆盖率）

**运行**：
```bash
pnpm test -- directory-utils
pnpm test -- directory-operations
```

### 7.2 组件测试

**使用 @testing-library/react**

**测试文件**：
- [ ] `DirectoryBlock.test.tsx`
  - [ ] 测试初始渲染
  - [ ] 测试 contents 更新
  - [ ] 测试 buildTree 调用
  - [ ] 测试所有 handlers
- [ ] `DirectoryTree.test.tsx`
  - [ ] 测试树渲染
  - [ ] 测试展开/折叠
  - [ ] 测试选中状态
- [ ] `DirectoryToolbar.test.tsx`
  - [ ] 测试按钮点击
  - [ ] 测试禁用状态

**运行**：
```bash
pnpm test -- DirectoryBlock
pnpm test -- DirectoryTree
```

### 7.3 集成测试

**测试策略**：使用 Vitest + @testing-library/react 进行集成测试

**测试场景**：

**场景 1: 完整创建流程**
```typescript
test('complete creation workflow', () => {
  // 1. Mock block with directory contents
  // 2. 渲染 DirectoryBlock
  // 3. 模拟 Refresh 操作
  // 4. 验证 buildTree 被调用
  // 5. 验证树结构正确
})
```

**场景 2: 创建和删除文件**
```typescript
test('create and delete file workflow', () => {
  // 1. 渲染 DirectoryBlock
  // 2. Mock prompt 返回文件名
  // 3. 模拟 Create File 点击
  // 4. 验证 executeDirectoryCreate 被调用
  // 5. 模拟 Delete 操作
  // 6. 验证 executeDirectoryDelete 被调用
})
```

**场景 3: 搜索功能**
```typescript
test('search workflow', () => {
  // 1. 渲染 DirectoryBlock
  // 2. 打开 Search Dialog
  // 3. Mock executeDirectorySearch 响应
  // 4. 验证搜索结果显示
  // 5. 验证结果列表渲染
})
```

**任务**：
- [ ] 编写 3 个集成测试场景
- [ ] 使用 Mock 模拟后端响应
- [ ] 验证完整交互流程

**运行**：
```bash
pnpm test -- DirectoryBlock.integration.test.tsx
```

### 7.4 测试覆盖率报告

**运行**：
```bash
pnpm test -- --coverage
```

**目标**：
- [ ] 整体覆盖率 >80%
- [ ] 关键函数覆盖率 >90%（buildTree, findNode）

### 7.5 文档更新

**任务**：
- [ ] 更新 `docs/guides/FRONTEND_DEVELOPMENT.md`
  - [ ] 添加 Directory Block 使用说明
  - [ ] 添加 buildTree 实现说明
- [ ] 创建使用示例截图
  - [ ] 树形视图展示
  - [ ] 右键菜单展示
  - [ ] 搜索 Dialog 展示
- [ ] 更新本文档（fe-progress.md）最终状态

### 7.6 Phase 7 验收

**验收标准**：
- [ ] 单元测试覆盖率 >80%
- [ ] 集成测试覆盖：完整创建流程 → 创建/删除文件 → 搜索功能
- [ ] 所有测试通过（unit + component + integration）
- [ ] 文档准确且易懂
- [ ] 截图完整清晰

---

## 测试方案总结

### 测试金字塔

```
       /\
      /集成\        3 个场景（Vitest + Mock）
     /------\
    /组件测试 \     5 个组件（@testing-library/react）
   /------------\
  /单元测试      \   10+ 函数（Vitest）
 /----------------\
```

### 单元测试（Vitest）

**测试目标**：纯函数逻辑

**文件**：
- `src/lib/directory-utils.test.ts`
  - `buildTree()`: 5 个测试
  - `findNode()`: 3 个测试
- `src/lib/directory-operations.test.ts`
  - 每个 operation: 3 个测试（正常、错误、边界）

**覆盖率目标**：>90%

**参考**：现有测试 `src/components/ui/button.test.tsx`

### 组件测试（React Testing Library）

**测试目标**：组件交互和渲染

**文件**：
- `DirectoryBlock.test.tsx`: 10 个测试（包含 Search Dialog 测试）
- `DirectoryTree.test.tsx`: 6 个测试
- `DirectoryToolbar.test.tsx`: 4 个测试
- `DirectoryStatusBar.test.tsx`: 4 个测试
- `DirectoryContextMenu.test.tsx`: 3 个测试

**覆盖率目标**：>80%

**测试配置**：使用 `src/test/setup.ts` 和 `vite.config.ts` 配置

**测试文件位置**：与组件在同一目录（遵循项目惯例，参考 `src/components/ui/button.test.tsx`）

### 集成测试（Vitest）

**测试目标**：完整操作流程

**场景**：
1. 完整创建流程
2. 创建和删除文件
3. 搜索功能

**运行环境**：Mock 后端响应，Vitest 环境

---

## TDD 开发原则

### Test-Driven Development (测试驱动开发)

**核心理念**：先写测试，再写实现

**TDD 开发流程**：
1. **Red（红灯）**: 编写测试，运行测试（失败）
2. **Green（绿灯）**: 编写最小实现，使测试通过
3. **Refactor（重构）**: 优化代码，保持测试通过

### Phase-by-Phase TDD 应用

**Phase 1: 基础渲染和树构建**
1. 编写 `buildTree()` 单元测试（directory-utils.test.ts）
2. 实现 `buildTree()` 函数使测试通过
3. 编写 DirectoryTree 组件测试（DirectoryTree.test.tsx）
4. 实现 DirectoryTree 组件使测试通过
5. 编写 DirectoryBlock 组件测试
6. 实现 DirectoryBlock 组件使测试通过

**Phase 2: Refresh 和工具栏**
1. 编写 `executeDirectoryRefresh()` 单元测试
2. 实现函数使测试通过
3. 编写 DirectoryToolbar 组件测试
4. 实现组件使测试通过
5. 编写 handleRefresh 集成测试
6. 实现功能使测试通过

**Phase 3-5: 功能实现**
- 每个功能遵循 TDD 流程：测试 → 实现 → 重构
- 确保每个 Phase 完成后测试覆盖率 >80%

**Phase 6: Polish 和优化**
- 在保持测试通过的前提下进行性能优化
- 添加性能测试（如渲染1000节点的响应时间）

**Phase 7: 测试和文档**
- 补充遗漏的测试用例
- 确保整体覆盖率 >80%

### TDD Checklist

**每个功能开始前**：
- [ ] 明确功能需求和验收标准
- [ ] 编写测试用例（单元测试 + 组件测试）
- [ ] 运行测试确认失败（Red）

**开发过程中**：
- [ ] 编写最小实现使测试通过（Green）
- [ ] 重构代码提高质量（Refactor）
- [ ] 保持测试一直通过

**每个功能完成后**：
- [ ] 所有测试通过
- [ ] 覆盖率达标
- [ ] 代码已重构优化

---

## 开发命令快速参考

### 启动开发环境
```bash
pnpm tauri dev
```

### 运行测试
```bash
# 所有测试
pnpm test

# 单个文件
pnpm test -- directory-utils.test.ts

# 覆盖率
pnpm test -- --coverage

# Watch 模式
pnpm test -- --watch
```

### Lint 和格式化
```bash
# Lint
pnpm lint

# 格式化
pnpm format
```

### 集成测试
```bash
# 运行集成测试
pnpm test -- DirectoryBlock.integration.test.tsx

# 所有测试（单元 + 组件 + 集成）
pnpm test
```

---

## 进度跟踪

**更新本节在完成每个 Phase 后**

### Phase 1: 基础渲染和树构建
- **开始日期**：
- **完成日期**：
- **实际工时**：
- **遇到的问题**：
- **解决方案**：

### Phase 2: Refresh 和工具栏
- **开始日期**：
- **完成日期**：
- **实际工时**：
- **遇到的问题**：
- **解决方案**：

### Phase 3: Create 和 Delete
- **开始日期**：
- **完成日期**：
- **实际工时**：
- **遇到的问题**：
- **解决方案**：

### Phase 4: 右键菜单和 Rename
- **开始日期**：
- **完成日期**：
- **实际工时**：
- **遇到的问题**：
- **解决方案**：

### Phase 5: Search 和状态栏
- **开始日期**：
- **完成日期**：
- **实际工时**：
- **遇到的问题**：
- **解决方案**：

### Phase 6: Polish 和优化
- **开始日期**：
- **完成日期**：
- **实际工时**：
- **遇到的问题**：
- **解决方案**：

### Phase 7: 测试和文档
- **开始日期**：
- **完成日期**：
- **实际工时**：
- **遇到的问题**：
- **解决方案**：

---

## 最终检查清单

**提交前验证**：

### 功能完整性
- [ ] 所有 7 个 Phase 完成
- [ ] 所有验收标准通过
- [ ] 无已知 Bug

### 代码质量
- [ ] 所有测试通过（unit + component + integration）
- [ ] 测试覆盖率 >80%
- [ ] Lint 无错误
- [ ] TypeScript 无类型错误

### 用户体验
- [ ] 所有交互流畅（无卡顿）
- [ ] 错误提示友好
- [ ] 加载状态清晰
- [ ] 视觉设计一致

### 文档
- [ ] FRONTEND_DEVELOPMENT.md 更新
- [ ] 截图完整
- [ ] 本文档状态最新

### 性能
- [ ] 1000 节点树渲染 <100ms
- [ ] 首次加载 <500ms
- [ ] 无内存泄漏

---

## 参考资源

**项目文档**：
- `docs/analyze/extension/directory-fe.md` - 前端开发指南
- `docs/guides/FRONTEND_DEVELOPMENT.md` - Tauri Specta 使用
- `docs/concepts/ARCHITECTURE_OVERVIEW.md` - 整体架构

**外部资源**：
- [Tauri 2 Documentation](https://v2.tauri.app/)
- [React Testing Library](https://testing-library.com/react)
- [Vitest](https://vitest.dev/)
- [shadcn/ui](https://ui.shadcn.com/)
- [Lucide Icons](https://lucide.dev/icons/)

---

## 后续事项

- Playwright E2E 测试：等 Directory 前端全部验收后再单独立项，当前计划仅保留该提醒，不包含任何实现细节。

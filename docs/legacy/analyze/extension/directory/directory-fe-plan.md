# Directory Extension 前端实施计划

> **版本**: v1.0 (2025-12-24)
> **状态**: 实施中
> **目标**: 严格按照设计文档实现 VFS 树状管理 UI

---

## Phase 1: 基础设施与算法 (Infrastructure)

### 1.1 VFS 转换算法实现
- **文件**: `src/utils/vfs-tree.ts`
- **任务**: 实现 `buildTreeFromEntries` 函数。
- **逻辑**:
    1. 接收 `entries` (扁平 Map) 和 `blocks` (用于查找 block_type)。
    2. 按路径深度（`/` 数量）对路径进行排序。
    3. 递归或迭代构建嵌套的 `VfsNode` 结构。
- **测试**: 创建 `src/utils/vfs-tree.test.ts`，测试简单层级、多层嵌套、父节点缺失容错。

### 1.2 TauriClient 封装补全
- **文件**: `src/lib/tauri-client.ts`
- **任务**: 封装 `DirectoryOperations` 类。
- **包含方法**: `createEntry`, `deleteEntry`, `renameEntry`, `importDirectory`, `checkoutWorkspace`。
- **测试**: 验证调用参数构造正确。

---

## Phase 2: 状态管理重构 (State Management)

### 2.1 AppStore 扩展
- **文件**: `src/lib/app-store.ts`
- **任务**:
    1. 增加 `getOutlineTree` 和 `getLinkedReposTrees` 选择器。
    2. 增加 `ensureSystemOutline` 方法：在 `loadBlocks` 或 `openFile` 后触发，检查并补齐 `__system_outline__`。
- **测试**: 模拟 blocks 列表，验证树转换逻辑集成正确。

---

## Phase 3: UI 组件升级 (UI Components)

### 3.1 通用 VfsTree 组件开发
- **文件**: `src/components/editor/VfsTree.tsx` (基于 `OutlineTree.tsx` 重构)
- **任务**:
    1. 渲染 `VfsNode[]` 数据。
    2. 文件夹逻辑：图标为 `Folder`，点击箭头展开，提供 `+` 按钮。
    3. 文件逻辑：图标根据 `blockType` 切换（`markdown` -> `FileText`, `code` -> `FileCode`），不提供 `+` 按钮。
    4. 对接 `DropdownMenu`：提供 `Rename`, `Delete`, `Export` 选项。
- **测试**: 使用不同 mock 数据验证图标和按钮的显示/隐藏。

---

## Phase 4: 业务工作流集成 (Workflows)

### 4.1 Outline 逻辑集成
- **文件**: `src/components/editor/FilePanel.tsx`
- **任务**:
    1. 绑定 `getOutlineTree`。
    2. 实现 Outline 顶部的 `+` 菜单（Add Folder / Add Document）。
    3. 对接 `deleteEntry` (级联删除)。

### 4.2 Linked Repos 逻辑集成
- **文件**: `src/components/editor/FilePanel.tsx`
- **任务**:
    1. 动态过滤并循环渲染所有 Repo Block。
    2. 实现 `handleImportRepo`：
        - 计算唯一根目录名称（冲突加后缀）。
        - 依次调用 `core.create` 和 `directory.import`。

### 4.3 Export (Checkout) 交互实现
- **任务**:
    1. 集成 `@tauri-apps/plugin-dialog`。
    2. 实现导出前的文件系统目录选择。
    3. 调用 `checkoutWorkspace` 并处理结果反馈。

---

## Phase 5: 验证与清理 (Finalization)

- [ ] 运行全量测试。
- [ ] 验证 `bindings.ts` 一致性。
- [ ] 修复 UI 样式细节。

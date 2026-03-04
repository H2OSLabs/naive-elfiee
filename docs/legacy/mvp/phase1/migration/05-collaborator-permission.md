# 协作者和权限迁移方案

## 文档信息

- **功能模块**：协作者和权限
- **文档版本**：1.0
- **最后更新**：2025-12-12
- **预计工时**：6 人时

---

## 功能概述

实现协作者管理和权限控制功能，支持添加/移除协作者、设置权限（read/write/delete）。

### 功能列表

| 细分功能 | 开发任务 | 预计人时 |
|---------|---------|---------|
| 基础功能 | 协作者增删 | 1 |
| | 权限管理 | 1 |
| 权限显示 | 权限页面修改 | 4 |

---

## 后端数据结构

### 1. Editor 结构

**文件位置**：`src-tauri/src/models/editor.rs`

**数据结构定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Editor {
    pub editor_id: String,  // UUID，唯一标识符
    pub name: String,       // 显示名称
}
```

**数据示例**：
```rust
Editor {
    editor_id: "editor-uuid-123",
    name: "Alice".to_string(),
}
```

**前端 TypeScript 类型**（自动生成到 `bindings.ts`）：
```typescript
export type Editor = {
  editor_id: string
  name: string
}
```

### 2. Grant 结构

**文件位置**：`src-tauri/src/models/grant.rs`

**数据结构定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Grant {
    pub editor_id: String,  // 被授权的编辑者 ID
    pub cap_id: String,     // 能力 ID（如 "markdown.write", "core.delete"）
    pub block_id: String,   // 目标 Block ID，或 "*" 表示所有 Block（通配符）
}
```

**数据示例**：
```rust
// 授予特定 Block 的权限
Grant {
    editor_id: "editor-alice".to_string(),
    cap_id: "markdown.write".to_string(),
    block_id: "block-123".to_string(),
}

// 授予所有 Block 的权限（通配符）
Grant {
    editor_id: "editor-bob".to_string(),
    cap_id: "markdown.read".to_string(),
    block_id: "*".to_string(),
}
```

**前端 TypeScript 类型**（自动生成到 `bindings.ts`）：
```typescript
export type Grant = {
  editor_id: string
  cap_id: string
  block_id: string  // "*" 表示通配符
}
```

### 3. GrantPayload 和 RevokePayload 结构

**文件位置**：`src-tauri/src/models/payloads.rs`

**GrantPayload 定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GrantPayload {
    pub target_editor: String,  // 目标编辑者 ID
    pub capability: String,      // 能力 ID
    pub target_block: String,    // 目标 Block ID 或 "*"（默认 "*"）
}
```

**RevokePayload 定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RevokePayload {
    pub target_editor: String,  // 目标编辑者 ID
    pub capability: String,      // 能力 ID
    pub target_block: String,    // 目标 Block ID 或 "*"（默认 "*"）
}
```

**数据示例**：
```rust
// 授予权限
GrantPayload {
    target_editor: "editor-alice".to_string(),
    capability: "markdown.write".to_string(),
    target_block: "block-123".to_string(),
}

// 撤销权限
RevokePayload {
    target_editor: "editor-alice".to_string(),
    capability: "markdown.write".to_string(),
    target_block: "block-123".to_string(),
}
```

**前端调用方式**：
```typescript
// 授予权限
await TauriClient.editor.grantCapability(
  fileId,
  "editor-alice",
  "markdown.write",
  "block-123",
  editorId
)

// 撤销权限
await TauriClient.editor.revokeCapability(
  fileId,
  "editor-alice",
  "markdown.write",
  "block-123",
  editorId
)
```

---

## 后端开发任务

### 1. 现有能力确认

**任务描述**：协作者和权限相关能力已全部实现

**已实现的能力和命令**：
- ✅ `core.grant` 能力已实现（`src-tauri/src/capabilities/builtins/grant.rs`）
- ✅ `core.revoke` 能力已实现（`src-tauri/src/capabilities/builtins/revoke.rs`）
- ✅ `list_grants` 命令已实现（`src-tauri/src/commands/editor.rs:191`）
- ✅ `get_editor_grants` 命令已实现（`src-tauri/src/commands/editor.rs:226`）
- ✅ `get_block_grants` 命令已实现（`src-tauri/src/commands/editor.rs:262`）

**结论**：无需新增后端接口，现有能力已足够支持所有权限管理功能。

---

## 前端开发任务

### 1. TauriClient 封装确认

**文件位置**：`elfiee/src/lib/tauri-client.ts`

**已实现的方法**：
```typescript
export class EditorOperations {
  // 列出所有授权（已实现，第475行）
  static async listGrants(fileId: string): Promise<Grant[]>
  
  // 获取编辑者的授权（已实现，第490行）
  static async getEditorGrants(fileId: string, editorId: string): Promise<Grant[]>
  
  // 获取 Block 的授权（已实现，第508行）
  static async getBlockGrants(fileId: string, blockId: string): Promise<Grant[]>
  
  // 授予能力（已实现，第529行）
  static async grantCapability(
    fileId: string,
    targetEditor: string,
    capability: string,
    targetBlock: string = '*',
    editorId: string = DEFAULT_EDITOR_ID
  ): Promise<Event[]>
  
  // 撤销能力（已实现，第559行）
  static async revokeCapability(
    fileId: string,
    targetEditor: string,
    capability: string,
    targetBlock: string = '*',
    editorId: string = DEFAULT_EDITOR_ID
  ): Promise<Event[]>
}
```

**结论**：所有权限相关的 TauriClient 方法已完整实现，可直接使用。

---

### 2. AppStore 状态管理确认

**文件位置**：`elfiee/src/lib/app-store.ts`

**已实现的方法**：
```typescript
interface AppStore {
  // 加载授权列表（已实现，第479行）
  loadGrants: (fileId: string) => Promise<void>
  
  // 授予能力（已实现，第497行）
  grantCapability: (
    fileId: string,
    targetEditor: string,
    capability: string,
    targetBlock?: string
  ) => Promise<void>
  
  // 撤销能力（已实现，第531行）
  revokeCapability: (
    fileId: string,
    targetEditor: string,
    capability: string,
    targetBlock?: string
  ) => Promise<void>
  
  // 获取授权列表（已实现，第720行）
  getGrants: (fileId: string) => Grant[]
}
```

**结论**：所有权限相关的 AppStore 方法已完整实现，可直接使用。

---

### 3. 创建权限管理组件

#### 协作者列表组件

**文件位置**：`elfiee/src/components/permission/CollaboratorList.tsx`

```typescript
// src/components/permission/CollaboratorList.tsx
import { useAppStore } from '@/lib/app-store'
import type { Editor, Grant } from '@/bindings'

interface CollaboratorListProps {
  fileId: string
}

export const CollaboratorList = ({ fileId }: CollaboratorListProps) => {
  const { 
    getEditors, 
    getGrants,
    grantCapability,
    revokeCapability 
  } = useAppStore()

  const editors = getEditors(fileId)
  const grants = getGrants(fileId)

  // 获取每个编辑者的权限
  const getEditorPermissions = (editorId: string): string[] => {
    return grants
      .filter(g => g.editor_id === editorId)
      .map(g => g.capability)
  }

  const handleGrant = async (
    editorId: string,
    capability: string,
    blockId?: string
  ) => {
    await grantCapability(fileId, editorId, capability, blockId)
  }

  const handleRevoke = async (
    editorId: string,
    capability: string,
    blockId?: string
  ) => {
    await revokeCapability(fileId, editorId, capability, blockId)
  }

  return (
    <div className="collaborator-list">
      <h3>协作者</h3>
      {editors.map(editor => (
        <CollaboratorItem
          key={editor.editor_id}
          editor={editor}
          permissions={getEditorPermissions(editor.editor_id)}
          onGrant={handleGrant}
          onRevoke={handleRevoke}
        />
      ))}
    </div>
  )
}
```

#### 协作者项组件

**文件位置**：`elfiee/src/components/permission/CollaboratorItem.tsx`

```typescript
// src/components/permission/CollaboratorItem.tsx
import type { Editor } from '@/bindings'

interface CollaboratorItemProps {
  editor: Editor
  permissions: string[]
  onGrant: (editorId: string, capability: string, blockId?: string) => Promise<void>
  onRevoke: (editorId: string, capability: string, blockId?: string) => Promise<void>
}

export const CollaboratorItem = ({
  editor,
  permissions,
  onGrant,
  onRevoke
}: CollaboratorItemProps) => {
  const capabilities = ['markdown.read', 'markdown.write', 'core.delete']

  const hasPermission = (capability: string) => {
    return permissions.includes(capability)
  }

  const handleTogglePermission = async (capability: string) => {
    if (hasPermission(capability)) {
      await onRevoke(editor.editor_id, capability)
    } else {
      await onGrant(editor.editor_id, capability)
    }
  }

  return (
    <div className="collaborator-item">
      <div className="collaborator-info">
        <span className="collaborator-name">{editor.name}</span>
        <span className="collaborator-id">({editor.editor_id})</span>
      </div>
      <div className="permissions">
        {capabilities.map(cap => (
          <label key={cap} className="permission-checkbox">
            <input
              type="checkbox"
              checked={hasPermission(cap)}
              onChange={() => handleTogglePermission(cap)}
            />
            <span>{getCapabilityLabel(cap)}</span>
          </label>
        ))}
      </div>
    </div>
  )
}

const getCapabilityLabel = (capability: string): string => {
  const labels: Record<string, string> = {
    'markdown.read': '读取',
    'markdown.write': '写入',
    'core.delete': '删除',
  }
  return labels[capability] || capability
}
```

#### 权限管理面板

**文件位置**：`elfiee/src/components/permission/PermissionPanel.tsx`

```typescript
// src/components/permission/PermissionPanel.tsx
import { CollaboratorList } from './CollaboratorList'
import { AddCollaboratorDialog } from './AddCollaboratorDialog'

interface PermissionPanelProps {
  fileId: string
  blockId?: string  // 可选，如果指定则只显示该 Block 的权限
}

export const PermissionPanel = ({ fileId, blockId }: PermissionPanelProps) => {
  const [showAddDialog, setShowAddDialog] = useState(false)

  return (
    <div className="permission-panel">
      <div className="permission-header">
        <h2>权限管理</h2>
        <Button onClick={() => setShowAddDialog(true)}>
          添加协作者
        </Button>
      </div>
      
      <CollaboratorList fileId={fileId} />
      
      {showAddDialog && (
        <AddCollaboratorDialog
          fileId={fileId}
          onClose={() => setShowAddDialog(false)}
        />
      )}
    </div>
  )
}
```

#### 添加协作者对话框

**文件位置**：`elfiee/src/components/permission/AddCollaboratorDialog.tsx`

```typescript
// src/components/permission/AddCollaboratorDialog.tsx
import { useAppStore } from '@/lib/app-store'

interface AddCollaboratorDialogProps {
  fileId: string
  onClose: () => void
}

export const AddCollaboratorDialog = ({ fileId, onClose }: AddCollaboratorDialogProps) => {
  const { createEditor } = useAppStore()
  const [name, setName] = useState('')

  const handleAdd = async () => {
    if (!name.trim()) return
    
    await createEditor(fileId, name)
    onClose()
  }

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>添加协作者</DialogTitle>
        </DialogHeader>
        <div className="dialog-body">
          <label>名称</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="输入协作者名称"
          />
        </div>
        <DialogFooter>
          <Button onClick={handleAdd}>添加</Button>
          <Button onClick={onClose} variant="outline">取消</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

---

## 开发检查清单

### 后端开发

- [x] 确认 `core.grant` 能力已实现
- [x] 确认 `core.revoke` 能力已实现
- [x] 确认相关命令已实现

### 前端开发

- [x] `TauriClient` 中的权限相关方法已封装（第475-578行）
- [x] `AppStore` 中的权限相关方法已实现（第479-563行，第720行）
- [x] 创建 `CollaboratorList` 组件
- [x] 创建 `CollaboratorItem` 组件
- [ ] 创建 `PermissionPanel` 组件
- [x] 创建 `AddCollaboratorDialog` 组件
- [x] 实现权限切换功能
- [x] 实现添加协作者功能
- [x] 实现移除协作者功能
- [x] 添加错误处理
- [x] 编写组件测试
- [x] 实现 Agent 状态切换功能（Enable/Disable）

---

## 测试要点

### 前端测试

1. **协作者列表测试**：
   - 显示协作者列表
   - 显示每个协作者的权限

2. **权限管理测试**：
   - 授予权限
   - 撤销权限
   - 权限状态正确更新

3. **添加协作者测试**：
   - 添加新协作者
   - 验证协作者创建成功

4. **Agent 状态切换测试**：
   - 所有 Bot 编辑者均显示 Agent Switch 开关（无论是否已有 Agent Block）
   - Human 编辑者不显示 Agent 开关
   - 有关联 Agent Block 时，切换开关调用 `enableAgent` / `disableAgent`
   - 无关联 Agent Block 时，切换开关调用 `createAgent`（自动创建）
   - 正确显示 Active / Inactive 状态文字

---

## Agent Enable/Disable 功能

### 概述

在 `CollaboratorItem` 组件中，为所有 Bot 编辑者显示一个 "Agent Capabilities" Switch 开关，
允许手动控制 Agent 的启用/禁用状态。无论是否已有关联的 Agent Block，开关均显示。

### 数据流

1. `CollaboratorList` 从 store 订阅 agent blocks（`block_type === 'agent'`）
2. 对每个 Bot 编辑者，通过 `editor.name` 和 `agentBlock.contents.target_project_id` 匹配关联的 Agent Block
3. 将匹配到的 `agentBlock`（或 `undefined`）作为 prop 传递给 `CollaboratorItem`
4. `CollaboratorItem` 对所有 Bot 编辑者（`editor_type === 'Bot'`）渲染 `Switch` 组件
5. **有 Agent Block 时**：切换开关调用 `AppStore.enableAgent()` 或 `AppStore.disableAgent()`
6. **无 Agent Block 时**：切换开关调用 `AppStore.createAgent()` 自动创建 Agent（默认 enabled）

### 多 Agent 支持

同一项目可以有多个 Agent Block，每个对应不同的 bot editor。唯一性约束为 `(target_project_id, editor_id)` 对。

### Agent 身份归因

`AgentContents` 包含 `editor_id` 字段，MCP Server 通过 `resolve_agent_editor_id()` 解析已启用 Agent 的 bot editor 身份，确保 MCP 操作归因到正确的 bot editor 而非 GUI owner。

### 使用的 API

- `AppStore.createAgent(fileId, blockId, name, editorId)` — 创建 Agent（自动 enabled）
- `AppStore.enableAgent(fileId, agentBlockId)` — 启用 Agent
- `AppStore.disableAgent(fileId, agentBlockId)` — 禁用 Agent
- Agent blocks 通过 `blocks.filter(b => b.block_type === 'agent')` 获取

### 相关文件

- `src/components/permission/CollaboratorList.tsx` — 解析 Agent Block 并传递给子组件
- `src/components/permission/CollaboratorItem.tsx` — 渲染 Switch 开关和状态指示器（Sparkles 图标 + "Agent Capabilities" 标签）

---

## 参考文档

- [迁移标准方案](../migration-standard.md)
- [Grant 模型文档](../../elfiee/src-tauri/src/models/grant.rs)

---

**文档维护**：本文档应与代码实现同步更新。


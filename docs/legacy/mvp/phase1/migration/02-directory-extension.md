# Directory Extension 迁移方案

## 文档信息

- **功能模块**：Directory Extension（目录扩展）
- **文档版本**：1.0
- **最后更新**：2025-12-12
- **预计工时**：30 人时

---

## 功能概述

实现目录管理功能，包括内部目录和外部目录的管理，支持创建、删除、重命名、导入、导出、刷新等操作。

### 功能列表

| 细分功能 | 开发任务 | 预计人时 |
|---------|---------|---------|
| 基础能力 | 导入 | 4 |
| | 创建 | 2 |
| | 删除 | 2 |
| | 重命名 | 2 |
| | 刷新 | 4 |
| | 导出 | 4 |
| 页面目录栏 | 内部目录 | 6 |
| | 外部目录 | 6 |

---

## 后端数据结构

### 1. DirectoryEntry 结构

**文件位置**：`src-tauri/src/commands/directory.rs`

**数据结构定义**：
```rust
#[derive(Serialize, Deserialize, specta::Type)]
pub struct DirectoryEntry {
    pub name: String,              // 文件或目录名称
    pub path: String,              // 完整路径
    pub is_directory: bool,         // 是否为目录
    pub is_external: bool,          // 是否为外部目录
    pub size: Option<u64>,          // 文件大小（字节），目录为 None
    pub modified_at: Option<String>, // 最后修改时间（ISO 8601 格式）
}
```

**数据示例**：
```rust
// 目录示例
DirectoryEntry {
    name: "src".to_string(),
    path: "/project/src".to_string(),
    is_directory: true,
    is_external: false,
    size: None,
    modified_at: Some("2025-12-12T10:00:00Z".to_string()),
}

// 文件示例
DirectoryEntry {
    name: "main.rs".to_string(),
    path: "/project/src/main.rs".to_string(),
    is_directory: false,
    is_external: false,
    size: Some(1024),
    modified_at: Some("2025-12-12T15:30:00Z".to_string()),
}
```

**前端 TypeScript 类型**（自动生成到 `bindings.ts`）：
```typescript
export type DirectoryEntry = {
  name: string
  path: string
  is_directory: boolean
  is_external: boolean
  size: number | null
  modified_at: string | null
}
```

### 2. 目录操作 Payload 结构

**文件位置**：`src-tauri/src/models/payloads.rs`（需要新增）

**DirectoryCreatePayload**：
```rust
#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct DirectoryCreatePayload {
    pub name: String,                    // 目录名称
    pub parent_path: Option<String>,     // 父目录路径，None 表示根目录
    pub is_external: bool,               // true 表示外部目录
}
```

**DirectoryRenamePayload**：
```rust
#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct DirectoryRenamePayload {
    pub old_path: String,    // 旧路径
    pub new_name: String,    // 新名称
}
```

**DirectoryImportPayload**：
```rust
#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct DirectoryImportPayload {
    pub source_path: String,    // 源路径
    pub target_path: String,    // 目标路径
    pub is_external: bool,      // 是否为外部目录
}
```

**DirectoryExportPayload**：
```rust
#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct DirectoryExportPayload {
    pub source_path: String,    // 源路径
    pub target_path: String,    // 目标路径
}
```

---

## 后端开发任务

### 1. 目录管理能力（Capability）

**任务描述**：实现目录的 CRUD 操作能力

**实现步骤**：

#### 步骤 1：定义目录操作 Payload

**文件位置**：`src-tauri/src/models/payloads.rs`

```rust
#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct DirectoryCreatePayload {
    pub name: String,
    pub parent_path: Option<String>,  // None 表示根目录
    pub is_external: bool,  // true 表示外部目录
}

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct DirectoryRenamePayload {
    pub old_path: String,
    pub new_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct DirectoryImportPayload {
    pub source_path: String,  // 源路径
    pub target_path: String,  // 目标路径
    pub is_external: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct DirectoryExportPayload {
    pub source_path: String,  // 源路径
    pub target_path: String,  // 目标路径
}
```

#### 步骤 2：创建目录能力处理器

**文件位置**：`src-tauri/src/extensions/directory/mod.rs`

```rust
use crate::capabilities::core::CapabilityHandler;
use crate::models::payloads::*;

pub struct DirectoryCreateCapability;

impl CapabilityHandler for DirectoryCreateCapability {
    fn cap_id(&self) -> &str {
        "directory.create"
    }

    fn handler(&self, cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
        let payload: DirectoryCreatePayload = serde_json::from_value(cmd.payload.clone())?;
        // 实现创建目录逻辑
        // 生成 Event
    }
}

pub struct DirectoryRenameCapability;

impl CapabilityHandler for DirectoryRenameCapability {
    fn cap_id(&self) -> &str {
        "directory.rename"
    }

    fn handler(&self, cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
        let payload: DirectoryRenamePayload = serde_json::from_value(cmd.payload.clone())?;
        // 实现重命名逻辑
    }
}

pub struct DirectoryDeleteCapability;

impl CapabilityHandler for DirectoryDeleteCapability {
    fn cap_id(&self) -> &str {
        "directory.delete"
    }

    fn handler(&self, cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
        // 实现删除逻辑
    }
}
```

#### 步骤 3：注册能力

**文件位置**：`src-tauri/src/capabilities/registry.rs`

```rust
impl CapabilityRegistry {
    fn register_extensions(&mut self) {
        use crate::extensions::directory::*;
        
        self.register(Arc::new(DirectoryCreateCapability));
        self.register(Arc::new(DirectoryRenameCapability));
        self.register(Arc::new(DirectoryDeleteCapability));
    }
}
```

#### 步骤 4：新增目录查询命令

**文件位置**：`src-tauri/src/commands/directory.rs`（新建文件）

```rust
#[tauri::command]
#[specta::specta]
pub async fn list_directory(
    state: tauri::State<'_, AppState>,
    file_id: String,
    path: Option<String>,  // None 表示根目录
    is_external: bool,
) -> Result<Vec<DirectoryEntry>, String> {
    // 实现列出目录内容
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub is_external: bool,
    pub size: Option<u64>,
    pub modified_at: Option<String>,
}
```

#### 步骤 5：注册命令和类型

**文件位置**：`src-tauri/src/lib.rs`

```rust
// 注册命令
.commands(tauri_specta::collect_commands![
    commands::directory::list_directory,
])

// 注册类型
.typ::<commands::directory::DirectoryEntry>()
.typ::<models::DirectoryCreatePayload>()
.typ::<models::DirectoryRenamePayload>()
.typ::<models::DirectoryImportPayload>()
.typ::<models::DirectoryExportPayload>()
```

---

### 2. 目录导入/导出功能

**任务描述**：实现目录的导入和导出功能

#### 步骤 1：新增导入/导出命令

**文件位置**：`src-tauri/src/commands/directory.rs`

```rust
#[tauri::command]
#[specta::specta]
pub async fn import_directory(
    state: tauri::State<'_, AppState>,
    file_id: String,
    source_path: String,
    target_path: String,
    is_external: bool,
) -> Result<(), String> {
    // 实现导入逻辑
}

#[tauri::command]
#[specta::specta]
pub async fn export_directory(
    state: tauri::State<'_, AppState>,
    file_id: String,
    source_path: String,
    target_path: String,
) -> Result<(), String> {
    // 实现导出逻辑
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_directory(
    state: tauri::State<'_, AppState>,
    file_id: String,
    path: String,
    is_external: bool,
) -> Result<Vec<DirectoryEntry>, String> {
    // 实现刷新逻辑（重新扫描目录）
}
```

---

## 前端开发任务

### 1. 封装 TauriClient 方法

**文件位置**：`elfiee/src/lib/tauri-client.ts`

#### 步骤 1：查看 bindings.ts

运行 `cargo run` 后，查看 `src/bindings.ts` 中的新命令接口。

#### 步骤 2：创建 DirectoryOperations 类

```typescript
// src/lib/tauri-client.ts
import { commands, type DirectoryEntry } from '@/bindings'

export class DirectoryOperations {
  /**
   * 列出目录内容
   * 
   * @param fileId - 文件 ID
   * @param path - 目录路径（可选，默认根目录）
   * @param isExternal - 是否为外部目录
   * @returns 目录条目列表
   */
  static async listDirectory(
    fileId: string,
    path?: string,
    isExternal: boolean = false
  ): Promise<DirectoryEntry[]> {
    const result = await commands.listDirectory(fileId, path || null, isExternal)
    if (result.status === 'ok') {
      return result.data
    } else {
      throw new Error(result.error)
    }
  }

  /**
   * 创建目录
   */
  static async createDirectory(
    fileId: string,
    name: string,
    parentPath: string | null,
    isExternal: boolean,
    editorId: string = DEFAULT_EDITOR_ID
  ): Promise<Event[]> {
    const payload = {
      name,
      parent_path: parentPath,
      is_external: isExternal,
    }
    const cmd = createCommand(
      editorId,
      'directory.create',
      fileId,  // 使用 fileId 作为 blockId
      payload as unknown as JsonValue
    )
    return await BlockOperations.executeCommand(fileId, cmd)
  }

  /**
   * 重命名目录
   */
  static async renameDirectory(
    fileId: string,
    oldPath: string,
    newName: string,
    editorId: string = DEFAULT_EDITOR_ID
  ): Promise<Event[]> {
    const payload = {
      old_path: oldPath,
      new_name: newName,
    }
    const cmd = createCommand(
      editorId,
      'directory.rename',
      fileId,
      payload as unknown as JsonValue
    )
    return await BlockOperations.executeCommand(fileId, cmd)
  }

  /**
   * 删除目录
   */
  static async deleteDirectory(
    fileId: string,
    path: string,
    editorId: string = DEFAULT_EDITOR_ID
  ): Promise<Event[]> {
    const cmd = createCommand(
      editorId,
      'directory.delete',
      fileId,
      { path } as unknown as JsonValue
    )
    return await BlockOperations.executeCommand(fileId, cmd)
  }

  /**
   * 导入目录
   */
  static async importDirectory(
    fileId: string,
    sourcePath: string,
    targetPath: string,
    isExternal: boolean
  ): Promise<void> {
    const result = await commands.importDirectory(fileId, sourcePath, targetPath, isExternal)
    if (result.status === 'error') {
      throw new Error(result.error)
    }
  }

  /**
   * 导出目录
   */
  static async exportDirectory(
    fileId: string,
    sourcePath: string,
    targetPath: string
  ): Promise<void> {
    const result = await commands.exportDirectory(fileId, sourcePath, targetPath)
    if (result.status === 'error') {
      throw new Error(result.error)
    }
  }

  /**
   * 刷新目录
   */
  static async refreshDirectory(
    fileId: string,
    path: string,
    isExternal: boolean
  ): Promise<DirectoryEntry[]> {
    const result = await commands.refreshDirectory(fileId, path, isExternal)
    if (result.status === 'ok') {
      return result.data
    } else {
      throw new Error(result.error)
    }
  }
}

// 更新 TauriClient
export const TauriClient = {
  file: FileOperations,
  block: BlockOperations,
  editor: EditorOperations,
  terminal: TerminalOperations,
  directory: DirectoryOperations,  // 新增
}
```

---

### 2. 更新 AppStore

**文件位置**：`elfiee/src/lib/app-store.ts`

```typescript
interface AppStore {
  // ... 现有状态

  // 目录状态
  directories: Map<string, DirectoryEntry[]>  // path -> entries

  // ... 现有方法

  // 目录操作
  loadDirectory: (fileId: string, path?: string, isExternal?: boolean) => Promise<void>
  createDirectory: (fileId: string, name: string, parentPath?: string, isExternal?: boolean) => Promise<void>
  renameDirectory: (fileId: string, oldPath: string, newName: string) => Promise<void>
  deleteDirectory: (fileId: string, path: string) => Promise<void>
  importDirectory: (fileId: string, sourcePath: string, targetPath: string, isExternal: boolean) => Promise<void>
  exportDirectory: (fileId: string, sourcePath: string, targetPath: string) => Promise<void>
  refreshDirectory: (fileId: string, path: string, isExternal: boolean) => Promise<void>
}
```

---

### 3. 创建目录组件

#### 目录树组件

**文件位置**：`elfiee/src/components/directory/DirectoryTree.tsx`

```typescript
// src/components/directory/DirectoryTree.tsx
import { useAppStore } from '@/lib/app-store'
import type { DirectoryEntry } from '@/bindings'

interface DirectoryTreeProps {
  fileId: string
  isExternal: boolean
  rootPath?: string
}

export const DirectoryTree = ({ fileId, isExternal, rootPath }: DirectoryTreeProps) => {
  const { loadDirectory, directories } = useAppStore()
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set())

  useEffect(() => {
    loadDirectory(fileId, rootPath, isExternal)
  }, [fileId, rootPath, isExternal])

  const entries = directories.get(rootPath || '/') || []

  return (
    <div className="directory-tree">
      {entries.map(entry => (
        <DirectoryNode
          key={entry.path}
          entry={entry}
          isExpanded={expandedPaths.has(entry.path)}
          onToggle={() => {
            // 切换展开/收起
          }}
        />
      ))}
    </div>
  )
}
```

#### 目录操作工具栏

**文件位置**：`elfiee/src/components/directory/DirectoryToolbar.tsx`

```typescript
// src/components/directory/DirectoryToolbar.tsx
import { useAppStore } from '@/lib/app-store'

interface DirectoryToolbarProps {
  fileId: string
  currentPath: string
  isExternal: boolean
}

export const DirectoryToolbar = ({ fileId, currentPath, isExternal }: DirectoryToolbarProps) => {
  const {
    createDirectory,
    importDirectory,
    exportDirectory,
    refreshDirectory,
  } = useAppStore()

  const handleCreate = async () => {
    const name = prompt('请输入目录名称')
    if (name) {
      await createDirectory(fileId, name, currentPath, isExternal)
    }
  }

  const handleImport = async () => {
    // 打开文件选择对话框
    const sourcePath = await openDirectoryDialog()
    if (sourcePath) {
      await importDirectory(fileId, sourcePath, currentPath, isExternal)
    }
  }

  const handleExport = async () => {
    const targetPath = await saveDirectoryDialog()
    if (targetPath) {
      await exportDirectory(fileId, currentPath, targetPath)
    }
  }

  const handleRefresh = async () => {
    await refreshDirectory(fileId, currentPath, isExternal)
  }

  return (
    <div className="directory-toolbar">
      <Button onClick={handleCreate}>创建</Button>
      <Button onClick={handleImport}>导入</Button>
      <Button onClick={handleExport}>导出</Button>
      <Button onClick={handleRefresh}>刷新</Button>
    </div>
  )
}
```

#### 目录面板组件

**文件位置**：`elfiee/src/components/directory/DirectoryPanel.tsx`

```typescript
// src/components/directory/DirectoryPanel.tsx
import { DirectoryTree } from './DirectoryTree'
import { DirectoryToolbar } from './DirectoryToolbar'

interface DirectoryPanelProps {
  fileId: string
  isExternal: boolean
}

export const DirectoryPanel = ({ fileId, isExternal }: DirectoryPanelProps) => {
  const [currentPath, setCurrentPath] = useState<string>('/')

  return (
    <div className="directory-panel">
      <div className="directory-header">
        <h3>{isExternal ? '外部目录' : '内部目录'}</h3>
        <DirectoryToolbar
          fileId={fileId}
          currentPath={currentPath}
          isExternal={isExternal}
        />
      </div>
      <DirectoryTree
        fileId={fileId}
        isExternal={isExternal}
        rootPath={currentPath}
      />
    </div>
  )
}
```

---

## 开发检查清单

### 后端开发

- [ ] 定义目录操作 Payload 类型
- [ ] 创建目录能力处理器（create, rename, delete）
- [ ] 注册目录能力到 CapabilityRegistry
- [ ] 实现 `list_directory` 命令
- [ ] 实现 `import_directory` 命令
- [ ] 实现 `export_directory` 命令
- [ ] 实现 `refresh_directory` 命令
- [ ] 在 `lib.rs` 中注册所有命令和类型
- [ ] 运行 `cargo run` 生成 `bindings.ts`
- [ ] 编写单元测试

### 前端开发

- [ ] 查看 `bindings.ts` 中的新命令接口
- [ ] 创建 `DirectoryOperations` 类
- [ ] 在 `TauriClient` 中添加 `directory` 属性
- [ ] 在 `app-store.ts` 中添加目录状态管理
- [ ] 创建 `DirectoryTree` 组件
- [ ] 创建 `DirectoryToolbar` 组件
- [ ] 创建 `DirectoryPanel` 组件
- [ ] 实现内部目录展示
- [ ] 实现外部目录展示
- [ ] 实现目录操作（创建、删除、重命名）
- [ ] 实现目录导入/导出功能
- [ ] 实现目录刷新功能
- [ ] 添加错误处理和加载状态
- [ ] 编写组件测试

---

## 测试要点

### 后端测试

1. **目录操作测试**：
   - 创建目录
   - 重命名目录
   - 删除目录
   - 列出目录内容

2. **导入/导出测试**：
   - 导入目录
   - 导出目录
   - 刷新目录

### 前端测试

1. **目录树测试**：
   - 显示目录树
   - 展开/收起目录
   - 选择目录

2. **目录操作测试**：
   - 创建目录
   - 重命名目录
   - 删除目录
   - 导入目录
   - 导出目录
   - 刷新目录

---

## 参考文档

- [迁移标准方案](../migration-standard.md)
- [Tauri 文件系统 API](https://tauri.app/v1/api/js/fs/)

---

**文档维护**：本文档应与代码实现同步更新。


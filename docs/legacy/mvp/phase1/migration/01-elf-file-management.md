# .elf 文件管理迁移方案

## 文档信息

- **功能模块**：.elf 文件管理
- **文档版本**：1.0
- **最后更新**：2025-12-12
- **预计工时**：13 人时

---

## 功能概述

实现 .elf 文件的完整生命周期管理，包括创建、导入、重命名、删除和展示功能。

### 功能列表

| 细分功能 | 开发任务 | 预计人时 |
|---------|---------|---------|
| .elf文件CRUD | 创建 | 1 |
| | 导入 | 1 |
| | 重命名 | 3 |
| | 删除 | 2 |
| .elf文件展示 | 获取.elf文件信息 | 2 |
| | 展示名称、协作者、路径 | 3 |
| | dashboard页面 | 4 |

---

## 后端数据结构

### 1. FileInfo 结构（新增）

**文件位置**：`src-tauri/src/commands/file.rs`

**数据结构定义**：
```rust
#[derive(Serialize, Deserialize, specta::Type)]
pub struct FileInfo {
    pub file_id: String,           // 文件唯一标识符
    pub name: String,               // 文件名称
    pub path: String,               // 文件路径
    pub collaborators: Vec<String>, // 协作者列表（editor_id 列表）
    pub created_at: String,         // 创建时间（ISO 8601 格式）
    pub last_modified: String,      // 最后修改时间（ISO 8601 格式）
}
```

**数据示例**：
```rust
FileInfo {
    file_id: "file-uuid-123",
    name: "demo-project.elf",
    path: "/path/to/demo-project.elf",
    collaborators: vec!["editor-alice".to_string(), "editor-bob".to_string()],
    created_at: "2025-12-12T10:00:00Z".to_string(),
    last_modified: "2025-12-12T15:30:00Z".to_string(),
}
```

**前端 TypeScript 类型**（自动生成到 `bindings.ts`）：
```typescript
export type FileInfo = {
  file_id: string
  name: string
  path: string
  collaborators: string[]
  created_at: string
  last_modified: string
}
```

---

## 后端开发任务

### 1. 重命名 .elf 文件

**任务描述**：实现重命名已存在的 .elf 文件功能

**实现步骤**：

#### 步骤 1：新增 Tauri 命令

**文件位置**：`src-tauri/src/commands/file.rs`

**新增命令**：
```rust
#[tauri::command]
#[specta::specta]
pub async fn rename_file(
    state: tauri::State<'_, AppState>,
    file_id: String,
    new_name: String,
) -> Result<(), String> {
    // 实现重命名逻辑
    // 1. 验证文件是否存在
    // 2. 验证新名称是否合法
    // 3. 更新文件元数据
    // 4. 更新文件系统（如果需要）
}
```

#### 步骤 2：注册命令

**文件位置**：`src-tauri/src/lib.rs`

**在 Debug 模式注册**：
```rust
.commands(tauri_specta::collect_commands![
    // ... 现有命令
    commands::file::rename_file,  // 新增
])
```

**在 Release 模式注册**：
```rust
tauri::generate_handler![
    // ... 现有命令
    commands::file::rename_file,  // 新增
]
```

#### 步骤 3：运行 cargo run

运行 `cargo run` 后，会自动生成 `src/bindings.ts`，包含新的 `renameFile` 命令。

---

### 2. 获取 .elf 文件详细信息

**任务描述**：获取 .elf 文件的完整信息，包括名称、协作者、路径等

**实现步骤**：

#### 步骤 1：新增 Tauri 命令

**文件位置**：`src-tauri/src/commands/file.rs`

**新增命令**：
```rust
#[derive(Serialize, Deserialize, specta::Type)]
pub struct FileInfo {
    pub file_id: String,
    pub name: String,
    pub path: String,
    pub collaborators: Vec<String>,  // 协作者列表
    pub created_at: String,
    pub last_modified: String,
}

#[tauri::command]
#[specta::specta]
pub async fn get_file_info(
    state: tauri::State<'_, AppState>,
    file_id: String,
) -> Result<FileInfo, String> {
    // 实现获取文件信息逻辑
    // 1. 从引擎中获取文件元数据
    // 2. 获取协作者列表
    // 3. 获取文件路径
    // 4. 返回 FileInfo
}
```

#### 步骤 2：注册命令和类型

**文件位置**：`src-tauri/src/lib.rs`

**注册命令**：
```rust
.commands(tauri_specta::collect_commands![
    commands::file::get_file_info,  // 新增
])
```

**注册类型**：
```rust
.typ::<commands::file::FileInfo>()  // 新增
```

---

## 前端开发任务

### 1. 封装 TauriClient 方法

**文件位置**：`elfiee/src/lib/tauri-client.ts`

#### 步骤 1：查看 bindings.ts

运行 `cargo run` 后，查看 `src/bindings.ts` 中的新命令接口：

```typescript
// bindings.ts（自动生成）
export const commands = {
  async renameFile(
    fileId: string,
    newName: string
  ): Promise<Result<null, string>> {
    // ...
  },
  async getFileInfo(
    fileId: string
  ): Promise<Result<FileInfo, string>> {
    // ...
  }
}
```

#### 步骤 2：在 FileOperations 类中添加封装方法

```typescript
// src/lib/tauri-client.ts
import { commands, type FileInfo } from '@/bindings'

export class FileOperations {
  // ... 现有方法

  /**
   * 重命名 .elf 文件
   * 
   * @param fileId - 文件 ID
   * @param newName - 新文件名
   * 
   * 依据：bindings.ts 中的 commands.renameFile()
   */
  static async renameFile(fileId: string, newName: string): Promise<void> {
    const result = await commands.renameFile(fileId, newName)
    if (result.status === 'error') {
      throw new Error(result.error)
    }
  }

  /**
   * 获取 .elf 文件详细信息
   * 
   * @param fileId - 文件 ID
   * @returns 文件信息（名称、协作者、路径等）
   * 
   * 依据：bindings.ts 中的 commands.getFileInfo()
   */
  static async getFileInfo(fileId: string): Promise<FileInfo> {
    const result = await commands.getFileInfo(fileId)
    if (result.status === 'ok') {
      return result.data
    } else {
      throw new Error(result.error)
    }
  }
}
```

---

### 2. 更新 AppStore

**文件位置**：`elfiee/src/lib/app-store.ts`

#### 添加状态管理方法

```typescript
// src/lib/app-store.ts
interface AppStore {
  // ... 现有状态

  // 文件信息
  fileInfo: Map<string, FileInfo>  // fileId -> FileInfo

  // ... 现有方法

  // 文件操作
  renameFile: (fileId: string, newName: string) => Promise<void>
  loadFileInfo: (fileId: string) => Promise<void>
  getFileInfo: (fileId: string) => FileInfo | null
}

export const useAppStore = create<AppStore>((set, get) => ({
  // ... 现有实现

  renameFile: async (fileId: string, newName: string) => {
    try {
      set({ isLoading: true, error: null })
      await TauriClient.file.renameFile(fileId, newName)
      
      // 更新本地状态
      const files = new Map(get().files)
      const fileState = files.get(fileId)
      if (fileState) {
        // 更新文件名称
        await get().loadFileInfo(fileId)
      }
    } catch (error) {
      get().addNotification('error', String(error))
    } finally {
      set({ isLoading: false })
    }
  },

  loadFileInfo: async (fileId: string) => {
    try {
      set({ isLoading: true, error: null })
      const info = await TauriClient.file.getFileInfo(fileId)
      
      const fileInfo = new Map(get().fileInfo)
      fileInfo.set(fileId, info)
      set({ fileInfo })
    } catch (error) {
      get().addNotification('error', String(error))
    } finally {
      set({ isLoading: false })
    }
  },

  getFileInfo: (fileId: string) => {
    return get().fileInfo.get(fileId) || null
  },
}))
```

---

### 3. 创建 Dashboard 页面组件

**文件位置**：`elfiee/src/components/dashboard/Dashboard.tsx`

#### 组件结构

```typescript
// src/components/dashboard/Dashboard.tsx
import { useAppStore } from '@/lib/app-store'
import { FileList } from './FileList'
import { FileInfoPanel } from './FileInfoPanel'

export const Dashboard = () => {
  const { 
    listOpenFiles, 
    renameFile, 
    loadFileInfo,
    getFileInfo 
  } = useAppStore()

  // 加载文件列表
  useEffect(() => {
    loadFiles()
  }, [])

  const handleRename = async (fileId: string, newName: string) => {
    await renameFile(fileId, newName)
  }

  return (
    <div className="dashboard">
      <FileList onRename={handleRename} />
      <FileInfoPanel />
    </div>
  )
}
```

#### 文件列表组件

**文件位置**：`elfiee/src/components/dashboard/FileList.tsx`

```typescript
// src/components/dashboard/FileList.tsx
import { useAppStore } from '@/lib/app-store'

interface FileListProps {
  onRename: (fileId: string, newName: string) => Promise<void>
}

export const FileList = ({ onRename }: FileListProps) => {
  const { listOpenFiles, createFile, openFile, closeFile } = useAppStore()
  const [files, setFiles] = useState<string[]>([])

  // 加载文件列表
  const loadFiles = async () => {
    const fileIds = await listOpenFiles()
    setFiles(fileIds)
  }

  return (
    <div className="file-list">
      {/* 文件列表 UI */}
    </div>
  )
}
```

#### 文件信息面板组件

**文件位置**：`elfiee/src/components/dashboard/FileInfoPanel.tsx`

```typescript
// src/components/dashboard/FileInfoPanel.tsx
import { useAppStore } from '@/lib/app-store'
import type { FileInfo } from '@/bindings'

export const FileInfoPanel = () => {
  const { getFileInfo, activeFileId } = useAppStore()
  const fileInfo = activeFileId ? getFileInfo(activeFileId) : null

  if (!fileInfo) {
    return <div>请选择一个文件</div>
  }

  return (
    <div className="file-info-panel">
      <h2>{fileInfo.name}</h2>
      <p>路径：{fileInfo.path}</p>
      <div>
        <h3>协作者</h3>
        <ul>
          {fileInfo.collaborators.map(collab => (
            <li key={collab}>{collab}</li>
          ))}
        </ul>
      </div>
      <p>创建时间：{fileInfo.created_at}</p>
      <p>最后修改：{fileInfo.last_modified}</p>
    </div>
  )
}
```

---

## 开发检查清单

### 后端开发

- [ ] 实现 `rename_file` 命令
- [ ] 实现 `get_file_info` 命令
- [ ] 定义 `FileInfo` 类型
- [ ] 在 `lib.rs` 中注册命令
- [ ] 在 `lib.rs` 中注册 `FileInfo` 类型
- [ ] 运行 `cargo run` 生成 `bindings.ts`
- [ ] 编写单元测试

### 前端开发

- [ ] 查看 `bindings.ts` 中的新命令接口
- [ ] 在 `tauri-client.ts` 中封装 `renameFile` 方法
- [ ] 在 `tauri-client.ts` 中封装 `getFileInfo` 方法
- [ ] 在 `app-store.ts` 中添加状态管理方法
- [ ] 创建 `Dashboard` 组件
- [ ] 创建 `FileList` 组件
- [ ] 创建 `FileInfoPanel` 组件
- [ ] 实现文件重命名 UI
- [ ] 实现文件信息展示 UI
- [ ] 添加错误处理和加载状态
- [ ] 编写组件测试

---

## 测试要点

### 后端测试

1. **重命名文件测试**：
   - 正常重命名
   - 重命名为已存在的名称（应失败）
   - 重命名为非法名称（应失败）
   - 重命名不存在的文件（应失败）

2. **获取文件信息测试**：
   - 获取存在的文件信息
   - 获取不存在的文件信息（应失败）

### 前端测试

1. **文件列表测试**：
   - 显示文件列表
   - 创建新文件
   - 导入文件
   - 删除文件

2. **文件信息展示测试**：
   - 显示文件名称
   - 显示文件路径
   - 显示协作者列表
   - 显示创建和修改时间

3. **文件重命名测试**：
   - 重命名成功
   - 重命名失败（显示错误提示）

---

## 参考文档

- [迁移标准方案](../migration-standard.md)
- [Tauri 命令文档](https://tauri.app/v1/guides/features/command)

---

**文档维护**：本文档应与代码实现同步更新。


# Block 数据结构迁移方案

## 文档信息

- **功能模块**：Block 数据结构
- **文档版本**：1.0
- **最后更新**：2025-12-12
- **预计工时**：17 人时

---

## 功能概述

扩展 Block 数据结构，添加 title、owner、description、time 等字段，并更新相关显示和编辑功能。

### 功能列表

| 细分功能 | 开发任务 | 预计人时 |
|---------|---------|---------|
| 数据结构修改 | title/owner/description/time | 4 |
| | 引擎修改 | 6 |
| 显示信息 | payload修改 | 3 |
| | info页面修改 | 4 |

---

## 后端数据结构

### 1. Block 结构（扩展后）

**说明**：根据 `maekdown-editor.md` 中的定义，Block.contents 应包含 `markdown` 和 `metadata` 字段。

**文件位置**：`src-tauri/src/models/block.rs`

**扩展后的数据结构定义**：
```rust
pub struct Block {
    pub block_id: String,           // UUID，唯一标识符
    pub name: String,               // 显示名称
    pub block_type: String,         // 类型：markdown/code/terminal/document
    pub contents: serde_json::Value, // 动态内容
    pub children: HashMap<String, Vec<String>>, // 关系图
    pub owner: String,              // 所有者 editor_id（已存在）
    
    // 新增字段
    pub title: Option<String>,      // 标题
    pub description: Option<String>, // 描述
    pub created_at: Option<String>,  // 创建时间（ISO 8601 格式）
    pub last_modified: Option<String>, // 最后修改时间（ISO 8601 格式）
}
```

**完整 Block 示例**：
```rust
Block {
    block_id: "block-uuid-123",
    name: "需求文档",
    block_type: "markdown",
    contents: json!({
        "markdown": "# Activity Rules\n\nThe activity rules are defined as follows:\n\n## Point System\n\nPoints are awarded based on:\n- **Daily login**: 10 points\n- **Complete task**: 50 points\n\n```python\ndef calculate_score(actions):\n    score = 0\n    for action in actions:\n        if action == 'login':\n            score += 10\n        elif action == 'task':\n            score += 50\n    return score\n```\n\nRun the code above to test.",
        "metadata": {
            "created_at": "2025-12-12T10:00:00Z",
            "last_modified": "2025-12-12T15:30:00Z",
            "word_count": 128
        }
    }),
    children: HashMap::new(),
    owner: "editor-alice",
    title: Some("项目需求文档".to_string()),
    description: Some("描述项目的核心需求".to_string()),
    created_at: Some("2025-12-12T10:00:00Z".to_string()),
    last_modified: Some("2025-12-12T15:30:00Z".to_string()),
}
```

**前端 TypeScript 类型**（自动生成到 `bindings.ts`）：
```typescript
export type Block = {
  block_id: string
  name: string
  block_type: string
  contents: JsonValue  // { markdown: string, metadata?: { created_at, last_modified, word_count } }
  children: Record<string, string[]>
  owner: string
  title?: string        // 新增
  description?: string  // 新增
  created_at?: string   // 新增
  last_modified?: string // 新增
}

// 前端读取方式
const block = await TauriClient.block.getBlock(fileId, blockId)
const markdown = (block.contents as { markdown?: string })?.markdown || ""
const metadata = (block.contents as { metadata?: { created_at?: string, last_modified?: string, word_count?: number } })?.metadata
```

### 2. UpdateMetadataPayload 结构（扩展后）

**文件位置**：`src-tauri/src/models/payloads.rs`

**扩展后的数据结构定义**：
```rust
#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct UpdateMetadataPayload {
    pub title: Option<String>,              // 新增：标题
    pub description: Option<String>,        // 新增：描述
    pub metadata: Option<Record<String, String>>, // 保留：其他元数据
}
```

**数据示例**：
```rust
// 只更新标题
UpdateMetadataPayload {
    title: Some("新标题".to_string()),
    description: None,
    metadata: None,
}

// 只更新描述
UpdateMetadataPayload {
    title: None,
    description: Some("新描述".to_string()),
    metadata: None,
}

// 同时更新标题和描述
UpdateMetadataPayload {
    title: Some("新标题".to_string()),
    description: Some("新描述".to_string()),
    metadata: None,
}
```

**前端调用方式**：
```typescript
await TauriClient.block.updateMetadata(
  fileId,
  blockId,
  {
    title: "新标题",
    description: "新描述"
  },
  editorId
)
```

---

## 后端开发任务

### 1. 扩展 Block 模型

**任务描述**：在 Block 结构中添加新字段

#### 步骤 1：修改 Block 模型

**文件位置**：`src-tauri/src/models/block.rs`

```rust
#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct Block {
    pub block_id: String,
    pub name: String,  // 保留原有字段
    pub block_type: String,
    pub contents: serde_json::Value,
    pub children: HashMap<String, Vec<String>>,
    pub owner: String,  // 新增：所有者（editor_id）
    
    // 新增字段
    pub title: Option<String>,  // 标题
    pub description: Option<String>,  // 描述
    pub created_at: Option<String>,  // 创建时间
    pub last_modified: Option<String>,  // 最后修改时间
}
```

#### 步骤 2：更新 StateProjector

**文件位置**：`src-tauri/src/engine/state.rs`

```rust
impl StateProjector {
    pub fn apply_event(&mut self, event: &Event) {
        // 处理 Block 创建事件时，设置新字段
        if event.attribute.ends_with("core.create") {
            if let Some(block) = self.blocks.get_mut(&event.entity) {
                // 设置创建时间
                block.created_at = Some(chrono::Utc::now().to_rfc3339());
                block.last_modified = Some(chrono::Utc::now().to_rfc3339());
            }
        }
        
        // 处理 Block 更新事件时，更新 last_modified
        if event.attribute.endsWith("core.update_metadata") {
            if let Some(block) = self.blocks.get_mut(&event.entity) {
                block.last_modified = Some(chrono::Utc::now().to_rfc3339());
            }
        }
    }
}
```

---

### 2. 更新元数据更新能力

**任务描述**：扩展 `core.update_metadata` 能力，支持更新 title 和 description

#### 步骤 1：更新 UpdateMetadataPayload

**文件位置**：`src-tauri/src/models/payloads.rs`

```rust
#[derive(Serialize, Deserialize, Debug, Clone, specta::Type)]
pub struct UpdateMetadataPayload {
    pub title: Option<String>,  // 新增
    pub description: Option<String>,  // 新增
    pub metadata: Option<Record<String, String>>,  // 保留原有字段
}
```

#### 步骤 2：更新能力处理器

**文件位置**：`src-tauri/src/capabilities/builtins/mod.rs`

```rust
impl CapabilityHandler for CoreUpdateMetadataCapability {
    fn handler(&self, cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
        let payload: UpdateMetadataPayload = serde_json::from_value(cmd.payload.clone())?;
        let block = block.ok_or("Block required")?;
        
        let mut event_value = json!({});
        
        // 更新 title
        if let Some(title) = payload.title {
            event_value["title"] = json!(title);
        }
        
        // 更新 description
        if let Some(description) = payload.description {
            event_value["description"] = json!(description);
        }
        
        // 更新其他 metadata
        if let Some(metadata) = payload.metadata {
            event_value["metadata"] = json!(metadata);
        }
        
        // 生成 Event
        let event = create_event(
            block.block_id.clone(),
            "core.update_metadata",
            event_value,
            &cmd.editor_id,
            1,
        );
        
        Ok(vec![event])
    }
}
```

---

### 3. 注册类型

**文件位置**：`src-tauri/src/lib.rs`

```rust
.typ::<models::UpdateMetadataPayload>()  // 确保已注册
```

---

## 前端开发任务

### 1. 更新类型定义

**任务描述**：确认 bindings.ts 中的 Block 类型已更新

#### 步骤 1：运行 cargo run

运行 `cargo run` 后，`src/bindings.ts` 会自动更新，包含新的 Block 字段。

#### 步骤 2：检查类型

```typescript
// src/bindings.ts（自动生成）
export type Block = {
  block_id: string
  name: string
  block_type: string
  contents: JsonValue
  children: Record<string, string[]>
  owner: string  // 新增
  title?: string  // 新增
  description?: string  // 新增
  created_at?: string  // 新增
  last_modified?: string  // 新增
}
```

---

### 2. 更新 TauriClient

**文件位置**：`elfiee/src/lib/tauri-client.ts`

#### 更新 updateMetadata 方法

```typescript
// src/lib/tauri-client.ts
export class BlockOperations {
  /**
   * 更新 Block 元数据（包括 title 和 description）
   */
  static async updateMetadata(
    fileId: string,
    blockId: string,
    metadata: {
      title?: string
      description?: string
      metadata?: Record<string, string>
    },
    editorId: string = DEFAULT_EDITOR_ID
  ): Promise<Event[]> {
    const payload = {
      title: metadata.title,
      description: metadata.description,
      metadata: metadata.metadata,
    }
    const cmd = createCommand(
      editorId,
      'core.update_metadata',
      blockId,
      payload as unknown as JsonValue
    )
    return await this.executeCommand(fileId, cmd)
  }
}
```

---

### 3. 更新 AppStore

**文件位置**：`elfiee/src/lib/app-store.ts`

```typescript
interface AppStore {
  // ... 现有状态和方法

  // Block 元数据操作
  updateBlockMetadata: (
    fileId: string,
    blockId: string,
    metadata: {
      title?: string
      description?: string
    }
  ) => Promise<void>
}

export const useAppStore = create<AppStore>((set, get) => ({
  // ... 现有实现

  updateBlockMetadata: async (
    fileId: string,
    blockId: string,
    metadata: {
      title?: string
      description?: string
    }
  ) => {
    try {
      set({ isLoading: true, error: null })
      
      const activeEditor = get().getActiveEditor(fileId)
      const editorId = activeEditor?.editor_id
      
      if (!editorId) {
        throw new Error('No active editor found')
      }

      await TauriClient.block.updateMetadata(
        fileId,
        blockId,
        metadata,
        editorId
      )
      
      await get().loadBlocks(fileId)
    } catch (error) {
      get().addNotification('error', String(error))
    } finally {
      set({ isLoading: false })
    }
  },
}))
```

---

### 4. 创建 Info 页面组件

**文件位置**：`elfiee/src/components/info/BlockInfoPanel.tsx`

```typescript
// src/components/info/BlockInfoPanel.tsx
import { useAppStore } from '@/lib/app-store'
import type { Block } from '@/bindings'

interface BlockInfoPanelProps {
  block: Block
  fileId: string
}

export const BlockInfoPanel = ({ block, fileId }: BlockInfoPanelProps) => {
  const { updateBlockMetadata, getEditorName } = useAppStore()
  const [isEditingTitle, setIsEditingTitle] = useState(false)
  const [isEditingDescription, setIsEditingDescription] = useState(false)
  const [title, setTitle] = useState(block.title || '')
  const [description, setDescription] = useState(block.description || '')

  const handleSaveTitle = async () => {
    await updateBlockMetadata(fileId, block.block_id, { title })
    setIsEditingTitle(false)
  }

  const handleSaveDescription = async () => {
    await updateBlockMetadata(fileId, block.block_id, { description })
    setIsEditingDescription(false)
  }

  const ownerName = getEditorName(fileId, block.owner)

  return (
    <div className="block-info-panel">
      <div className="info-section">
        <label>标题</label>
        {isEditingTitle ? (
          <div className="edit-group">
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              onBlur={handleSaveTitle}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  handleSaveTitle()
                }
              }}
              autoFocus
            />
          </div>
        ) : (
          <div 
            onClick={() => setIsEditingTitle(true)}
            className="editable-field"
          >
            {block.title || '未设置'}
          </div>
        )}
      </div>

      <div className="info-section">
        <label>描述</label>
        {isEditingDescription ? (
          <div className="edit-group">
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              onBlur={handleSaveDescription}
              autoFocus
            />
          </div>
        ) : (
          <div 
            onClick={() => setIsEditingDescription(true)}
            className="editable-field"
          >
            {block.description || '未设置'}
          </div>
        )}
      </div>

      <div className="info-section">
        <label>所有者</label>
        <div>{ownerName}</div>
      </div>

      <div className="info-section">
        <label>创建时间</label>
        <div>
          {block.created_at 
            ? new Date(block.created_at).toLocaleString()
            : '未知'
          }
        </div>
      </div>

      <div className="info-section">
        <label>最后修改</label>
        <div>
          {block.last_modified
            ? new Date(block.last_modified).toLocaleString()
            : '未知'
          }
        </div>
      </div>
    </div>
  )
}
```

---

## 开发检查清单

### 后端开发

- [ ] 扩展 Block 模型，添加新字段
- [ ] 更新 StateProjector，处理新字段
- [ ] 更新 UpdateMetadataPayload
- [ ] 更新 CoreUpdateMetadataCapability
- [ ] 在 `lib.rs` 中注册类型
- [ ] 运行 `cargo run` 生成 `bindings.ts`
- [ ] 编写单元测试

### 前端开发

- [ ] 查看 `bindings.ts` 中的更新后的 Block 类型
- [ ] 更新 `tauri-client.ts` 中的 `updateMetadata` 方法
- [ ] 在 `app-store.ts` 中添加 `updateBlockMetadata` 方法
- [ ] 创建 `BlockInfoPanel` 组件
- [ ] 实现标题编辑功能
- [ ] 实现描述编辑功能
- [ ] 显示所有者信息
- [ ] 显示创建和修改时间
- [ ] 添加错误处理
- [ ] 编写组件测试

---

## 测试要点

### 后端测试

1. **Block 创建测试**：
   - 创建 Block 时设置 created_at 和 last_modified
   - 设置 owner 字段

2. **元数据更新测试**：
   - 更新 title
   - 更新 description
   - 更新后 last_modified 自动更新

### 前端测试

1. **Info 面板测试**：
   - 显示 Block 信息
   - 编辑标题
   - 编辑描述
   - 显示时间信息

---

## 参考文档

- [迁移标准方案](../migration-standard.md)
- [Block 模型文档](../../elfiee/src-tauri/src/models/block.rs)

---

**文档维护**：本文档应与代码实现同步更新。


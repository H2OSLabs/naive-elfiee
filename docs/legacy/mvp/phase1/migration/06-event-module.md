# Event 模块迁移方案

## 文档信息

- **功能模块**：Event 模块（事件记录和回溯）
- **文档版本**：1.0
- **最后更新**：2025-12-12
- **预计工时**：32 人时

---

## 功能概述

实现事件记录、排序、回溯和显示功能，支持查看操作历史、回溯到历史状态。

### 功能列表

| 细分功能 | 开发任务 | 预计人时 |
|---------|---------|---------|
| event解析 | 操作人员/名称/内容 | 4 |
| | event排序 | 7 |
| event回溯 | 回溯功能 | 7 |
| | 快照功能（optional） | 6 |
| 显示和交互 | 查看列表 | 6 |
| | 回溯操作 | 6 |
| | 对比显示（optional） | 2 |

---

## 后端数据结构

### 1. Event 结构

**文件位置**：`src-tauri/src/models/event.rs`

**数据结构定义**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Event {
    pub event_id: String,                    // UUID，唯一标识符
    pub entity: String,                      // 实体 ID（通常是 block_id）
    pub attribute: String,                   // 属性格式："{editor_id}/{cap_id}"
    pub value: serde_json::Value,            // 事件值（动态 JSON）
    pub timestamp: HashMap<String, i64>,     // 向量时钟
}
```

**数据示例**：
```rust
// markdown.write 事件示例
Event {
    event_id: "event-uuid-123",
    entity: "block-uuid-456",
    attribute: "editor-alice/markdown.write",
    value: json!({
        "contents": {
            "markdown": "# New Content\n\nThis is the new markdown."
        }
    }),
    timestamp: {
        "editor-alice".to_string() => 1,
        "editor-bob".to_string() => 0,
    },
}

// core.create 事件示例
Event {
    event_id: "event-uuid-789",
    entity: "block-uuid-999",
    attribute: "editor-alice/core.create",
    value: json!({
        "name": "新文档",
        "block_type": "markdown"
    }),
    timestamp: {
        "editor-alice".to_string() => 2,
        "editor-bob".to_string() => 0,
    },
}
```

**前端 TypeScript 类型**（自动生成到 `bindings.ts`）：
```typescript
export type Event = {
  event_id: string
  entity: string
  attribute: string  // 格式："{editor_id}/{cap_id}"
  value: JsonValue
  timestamp: Record<string, number>  // 向量时钟
}
```

**Event 解析说明**：
- `attribute` 格式：`"{editor_id}/{cap_id}"`
  - 例如：`"editor-alice/markdown.write"` 表示 editor-alice 执行了 markdown.write 操作
- `value` 结构根据不同的 `cap_id` 而不同：
  - `markdown.write`: `{ "contents": { "markdown": "..." } }`
  - `core.create`: `{ "name": "...", "block_type": "..." }`
  - `core.grant`: `{ "editor": "...", "capability": "...", "block": "..." }`

### 2. 向量时钟（Vector Clock）

**数据结构**：`HashMap<String, i64>`

**说明**：
- 键（String）：editor_id
- 值（i64）：该编辑者的逻辑时钟值
- 用途：用于事件排序，解决并发事件的顺序问题

**示例**：
```rust
// 编辑者 alice 和 bob 的向量时钟
HashMap::from([
    ("editor-alice".to_string(), 5),
    ("editor-bob".to_string(), 3),
])
```

**前端排序逻辑**：
```typescript
// 比较两个向量时钟
function compareVectorClocks(
  vc1: Record<string, number>,
  vc2: Record<string, number>
): number {
  // 实现向量时钟比较算法
  // 返回 -1（小于）、0（等于）、1（大于）
}
```

---

## 后端开发任务

### 1. Event 解析和排序

**任务描述**：实现 Event 的解析和按向量时钟排序

#### 步骤 1：确认现有命令

**检查项**：
- ✅ `get_all_events` 命令已实现（`src-tauri/src/commands/file.rs`）
- ✅ Event 模型包含向量时钟字段（`timestamp: HashMap<String, i64>`）

**无需新增后端接口**，但需要确认排序逻辑。

#### 步骤 2：实现 Event 排序（如果需要）

**文件位置**：`src-tauri/src/commands/event.rs`（可能需要新建）

```rust
use std::collections::HashMap;

/// 按向量时钟对事件进行排序
pub fn sort_events_by_vector_clock(events: &mut Vec<Event>) {
    events.sort_by(|a, b| {
        compare_vector_clocks(&a.timestamp, &b.timestamp)
    });
}

/// 比较两个向量时钟
fn compare_vector_clocks(
    vc1: &HashMap<String, i64>,
    vc2: &HashMap<String, i64>
) -> std::cmp::Ordering {
    // 实现向量时钟比较逻辑
    // 返回 Less, Equal, 或 Greater
}
```

---

### 2. Event 回溯功能

**任务描述**：实现回溯到指定 Event 时刻的 Block 内容

#### 步骤 1：新增回溯命令

**文件位置**：`src-tauri/src/commands/event.rs`（新建文件）

```rust
#[tauri::command]
#[specta::specta]
pub async fn get_block_content_at_event(
    state: tauri::State<'_, AppState>,
    file_id: String,
    block_id: String,
    event_id: String,
) -> Result<String, String> {
    let engines = state.engines.lock().await;
    let handle = engines.get(&file_id).ok_or("File not found")?;

    // 1. 获取所有事件
    let all_events = handle.tx.get_all_events().await?;

    // 2. 找到目标事件的索引
    let target_index = all_events
        .iter()
        .position(|e| e.event_id == event_id)
        .ok_or("Event not found")?;

    // 3. 重放事件到目标点
    let mut temp_state = StateProjector::new();
    for event in &all_events[..=target_index] {
        temp_state.apply_event(event);
    }

    // 4. 提取 Block 的 Markdown 内容
    let block = temp_state.blocks.get(&block_id)
        .ok_or("Block not found at that event")?;

    let markdown = block.contents.get("markdown")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(markdown)
}
```

#### 步骤 2：注册命令

**文件位置**：`src-tauri/src/lib.rs`

```rust
.commands(tauri_specta::collect_commands![
    commands::event::get_block_content_at_event,  // 新增
])
```

---

### 3. Event 快照功能（可选）

**任务描述**：实现 Event 快照，提高回溯性能

#### 步骤 1：定义快照结构

**文件位置**：`src-tauri/src/models/event.rs`

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventSnapshot {
    pub event_id: String,
    pub block_snapshots: HashMap<String, Block>,  // block_id -> Block
    pub timestamp: HashMap<String, i64>,
}
```

#### 步骤 2：实现快照生成

**文件位置**：`src-tauri/src/commands/event.rs`

```rust
#[tauri::command]
#[specta::specta]
pub async fn create_event_snapshot(
    state: tauri::State<'_, AppState>,
    file_id: String,
    event_id: String,
) -> Result<(), String> {
    // 实现快照生成逻辑
    // 保存到文件或数据库
}
```

---

## 前端开发任务

### 1. 封装 TauriClient 方法

**文件位置**：`elfiee/src/lib/tauri-client.ts`

#### 步骤 1：查看 bindings.ts

运行 `cargo run` 后，查看 `src/bindings.ts` 中的新命令接口。

#### 步骤 2：添加 Event 操作方法

```typescript
// src/lib/tauri-client.ts
export class EventOperations {
  /**
   * 获取所有事件
   */
  static async getAllEvents(fileId: string): Promise<Event[]> {
    const result = await commands.getAllEvents(fileId)
    if (result.status === 'ok') {
      return result.data
    } else {
      throw new Error(result.error)
    }
  }

  /**
   * 获取指定 Event 时刻的 Block 内容（回溯）
   */
  static async getContentAtEvent(
    fileId: string,
    blockId: string,
    eventId: string
  ): Promise<string> {
    const result = await commands.getBlockContentAtEvent(fileId, blockId, eventId)
    if (result.status === 'ok') {
      return result.data
    } else {
      throw new Error(result.error)
    }
  }

  /**
   * 按向量时钟对事件排序
   */
  static sortEventsByVectorClock(events: Event[]): Event[] {
    // 实现向量时钟排序逻辑
    return events.sort((a, b) => {
      // 比较向量时钟
      return compareVectorClocks(a.timestamp, b.timestamp)
    })
  }

  /**
   * 解析 Event，提取操作信息
   */
  static parseEvent(event: Event): {
    operator: string  // 操作人员
    action: string    // 操作名称
    content: string   // 操作内容（如果有）
  } {
    const [editorId, capId] = event.attribute.split('/')
    
    return {
      operator: editorId,
      action: getActionLabel(capId),
      content: extractContent(event.value),
    }
  }
}

// 更新 TauriClient
export const TauriClient = {
  file: FileOperations,
  block: BlockOperations,
  editor: EditorOperations,
  terminal: TerminalOperations,
  directory: DirectoryOperations,
  event: EventOperations,  // 新增
}
```

---

### 2. 更新 AppStore

**文件位置**：`elfiee/src/lib/app-store.ts`

```typescript
interface AppStore {
  // ... 现有状态

  // Event 状态
  events: Map<string, Event[]>  // fileId -> events
  isRestoredMode: boolean
  restoredEventId: string | null

  // ... 现有方法

  // Event 操作
  loadEvents: (fileId: string) => Promise<void>
  restoreToEvent: (fileId: string, blockId: string, eventId: string) => Promise<string>
  exitRestoreMode: () => void
  getEvents: (fileId: string) => Event[]
}
```

---

### 3. 创建 Event 显示组件

#### Event 列表组件

**文件位置**：`elfiee/src/components/event/EventList.tsx`

```typescript
// src/components/event/EventList.tsx
import { useAppStore } from '@/lib/app-store'
import { TauriClient } from '@/lib/tauri-client'
import type { Event } from '@/bindings'

interface EventListProps {
  fileId: string
  blockId?: string  // 可选，如果指定则只显示该 Block 的事件
}

export const EventList = ({ fileId, blockId }: EventListProps) => {
  const { getEvents, loadEvents } = useAppStore()
  const [sortedEvents, setSortedEvents] = useState<Event[]>([])

  useEffect(() => {
    loadEvents(fileId)
  }, [fileId])

  useEffect(() => {
    const events = getEvents(fileId)
    const filtered = blockId
      ? events.filter(e => e.entity === blockId)
      : events
    
    // 按向量时钟排序
    const sorted = TauriClient.event.sortEventsByVectorClock([...filtered])
    setSortedEvents(sorted)
  }, [fileId, blockId, getEvents])

  return (
    <div className="event-list">
      <h3>事件历史</h3>
      {sortedEvents.map(event => (
        <EventItem key={event.event_id} event={event} fileId={fileId} />
      ))}
    </div>
  )
}
```

#### Event 项组件

**文件位置**：`elfiee/src/components/event/EventItem.tsx`

```typescript
// src/components/event/EventItem.tsx
import { TauriClient } from '@/lib/tauri-client'
import type { Event } from '@/bindings'

interface EventItemProps {
  event: Event
  fileId: string
  onRestore?: (eventId: string) => void
}

export const EventItem = ({ event, fileId, onRestore }: EventItemProps) => {
  const parsed = TauriClient.event.parseEvent(event)
  const timestamp = formatTimestamp(event.timestamp)

  return (
    <div className="event-item">
      <div className="event-header">
        <span className="event-operator">{parsed.operator}</span>
        <span className="event-action">{parsed.action}</span>
        <span className="event-time">{timestamp}</span>
      </div>
      {parsed.content && (
        <div className="event-content">{parsed.content}</div>
      )}
      {onRestore && (
        <Button 
          onClick={() => onRestore(event.event_id)}
          size="sm"
        >
          回溯到此
        </Button>
      )}
    </div>
  )
}
```

#### Timeline 组件

**文件位置**：`elfiee/src/components/event/Timeline.tsx`

```typescript
// src/components/event/Timeline.tsx
import { EventList } from './EventList'
import { useAppStore } from '@/lib/app-store'

interface TimelineProps {
  fileId: string
  blockId?: string
}

export const Timeline = ({ fileId, blockId }: TimelineProps) => {
  const { 
    restoreToEvent, 
    exitRestoreMode,
    isRestoredMode,
    restoredEventId 
  } = useAppStore()

  const handleRestore = async (eventId: string) => {
    if (!blockId) return
    
    try {
      const content = await restoreToEvent(fileId, blockId, eventId)
      // 内容已更新到编辑器
    } catch (error) {
      console.error('Failed to restore:', error)
    }
  }

  return (
    <div className="timeline">
      {isRestoredMode && (
        <div className="restore-mode-banner">
          <span>正在查看历史版本（Event: {restoredEventId}）</span>
          <Button onClick={exitRestoreMode}>返回当前版本</Button>
        </div>
      )}
      <EventList 
        fileId={fileId} 
        blockId={blockId}
        onRestore={handleRestore}
      />
    </div>
  )
}
```

---

## 开发检查清单

### 后端开发

- [ ] 确认 `get_all_events` 命令已实现
- [ ] 实现 `get_block_content_at_event` 命令
- [ ] 实现事件排序逻辑（如果需要）
- [ ] 在 `lib.rs` 中注册新命令
- [ ] 运行 `cargo run` 生成 `bindings.ts`
- [ ] 实现快照功能（可选）
- [ ] 编写单元测试

### 前端开发

- [ ] 查看 `bindings.ts` 中的新命令接口
- [ ] 创建 `EventOperations` 类
- [ ] 实现事件排序逻辑
- [ ] 实现事件解析逻辑
- [ ] 在 `TauriClient` 中添加 `event` 属性
- [ ] 在 `app-store.ts` 中添加事件状态管理
- [ ] 创建 `EventList` 组件
- [ ] 创建 `EventItem` 组件
- [ ] 创建 `Timeline` 组件
- [ ] 实现回溯功能
- [ ] 实现回溯模式切换
- [ ] 实现事件对比显示（可选）
- [ ] 添加错误处理
- [ ] 编写组件测试

---

## 测试要点

### 后端测试

1. **事件排序测试**：
   - 按向量时钟正确排序
   - 处理并发事件

2. **回溯功能测试**：
   - 回溯到指定事件
   - 获取历史内容
   - 处理不存在的事件

### 前端测试

1. **事件列表测试**：
   - 显示事件列表
   - 正确排序
   - 显示操作信息

2. **回溯功能测试**：
   - 回溯到历史版本
   - 退出回溯模式
   - 回溯后内容正确显示

---

## 参考文档

- [迁移标准方案](../migration-standard.md)
- [Event 模型文档](../../elfiee/src-tauri/src/models/event.rs)
- [向量时钟算法](https://en.wikipedia.org/wiki/Vector_clock)

---

**文档维护**：本文档应与代码实现同步更新。


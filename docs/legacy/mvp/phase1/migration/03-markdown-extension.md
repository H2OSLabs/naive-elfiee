# Markdown Extension 迁移方案

## 文档信息

- **功能模块**：Markdown Extension（Markdown 扩展）
- **文档版本**：1.0
- **最后更新**：2025-12-12
- **预计工时**：12 人时

---

## 功能概述

实现 Markdown 编辑和显示功能，支持 MyST 语法解析和渲染。

### 功能列表

| 细分功能 | 开发任务 | 预计人时 |
|---------|---------|---------|
| md 编辑 | 数据格式修改 | 6 |
| md 显示 | myst语法和适配 | 6 |

---

## 后端数据结构

### 1. Block 结构（Markdown 类型）

**说明**：根据 `maekdown-editor.md` 中的定义，Block.contents 应包含 `markdown` 和 `metadata` 字段。

**文件位置**：`src-tauri/src/models/block.rs`

**数据结构定义**：
```rust
pub struct Block {
    pub block_id: String,           // UUID，唯一标识符
    pub name: String,               // 显示名称，如 "需求文档"
    pub block_type: String,         // 类型：markdown/code/terminal/document
    pub contents: serde_json::Value, // 动态内容，根据类型不同
    pub children: HashMap<String, Vec<String>>, // 关系图
    pub owner: String,              // 所有者 editor_id
}
```

**Markdown Block 的 contents 结构**：
```rust
// Block.contents 存储格式（标准 Markdown Block）
json!({
    "markdown": "# Activity Rules\n\nThe activity rules are defined as follows:\n\n## Point System\n\nPoints are awarded based on:\n- **Daily login**: 10 points\n- **Complete task**: 50 points\n\n```python\ndef calculate_score(actions):\n    score = 0\n    for action in actions:\n        if action == 'login':\n            score += 10\n        elif action == 'task':\n            score += 50\n    return score\n```\n\nRun the code above to test.",
    "metadata": {
        "created_at": "2025-12-12T10:00:00Z",
        "last_modified": "2025-12-12T15:30:00Z",
        "word_count": 128
    }
})
```

**完整 Block 示例**：
```rust
Block {
    block_id: "block-uuid-123",
    name: "需求验收标准",
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
    owner: "editor-alice"
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
}

// 前端读取方式
const block = await TauriClient.block.getBlock(fileId, blockId)
const markdown = (block.contents as { markdown?: string })?.markdown || ""
const metadata = (block.contents as { metadata?: { created_at?: string, last_modified?: string, word_count?: number } })?.metadata
```

### 2. MarkdownWritePayload 结构

**文件位置**：`src-tauri/src/extensions/markdown/markdown_write.rs`

**数据结构定义**：
```rust
#[derive(Serialize, Deserialize, specta::Type)]
pub struct MarkdownWritePayload {
    pub content: String,  // 原始 Markdown 字符串
}
```

**数据示例**：
```rust
MarkdownWritePayload {
    content: "# Hello World\n\nThis is a **bold** text.".to_string(),
}
```

**前端调用方式**：
```typescript
await TauriClient.block.writeBlock(
  fileId,
  blockId,
  "# Hello World\n\nThis is a **bold** text.",
  editorId
)
```

---

## 后端开发任务

### 1. 确认现有能力

**任务描述**：确认 `markdown.write` 和 `markdown.read` 能力已实现

**检查项**：
- ✅ `markdown.write` 能力已实现（`src-tauri/src/extensions/markdown/markdown_write.rs`）
- ✅ `markdown.read` 能力已实现（`src-tauri/src/extensions/markdown/markdown_read.rs`）
- ✅ Block.contents 结构包含 `markdown` 字段

**无需新增后端接口**，只需确保现有能力正常工作。

---

## 前端开发任务

### 1. 数据格式适配

**任务描述**：确保前端能够正确读取和写入 Markdown 数据

#### 步骤 1：确认 TauriClient 封装

**文件位置**：`elfiee/src/lib/tauri-client.ts`

**检查现有方法**：
```typescript
// 应该已经存在
export class BlockOperations {
  static async writeBlock(
    fileId: string,
    blockId: string,
    content: string,
    editorId: string = DEFAULT_EDITOR_ID
  ): Promise<Event[]> {
    // 使用 markdown.write 能力
  }
}
```

**确认数据格式**：
- Block.contents.markdown 存储原始 Markdown 字符串
- 前端直接使用字符串，无需格式转换

---

### 2. MyST 语法解析和渲染

**任务描述**：实现 MyST 语法的解析和渲染

#### 步骤 1：安装依赖

```bash
npm install myst-parser @myst-theme/react
```

#### 步骤 2：创建 MyST 文档组件

**文件位置**：`elfiee/src/components/editor/MySTDocument.tsx`

```typescript
// src/components/editor/MySTDocument.tsx
import { mystParse } from 'myst-parser'
import { MyST } from '@myst-theme/react'
import { useState } from 'react'

interface MySTDocumentProps {
  content: string
  onContentChange?: (content: string) => void
  editable?: boolean
}

export const MySTDocument = ({ 
  content, 
  onContentChange, 
  editable = false 
}: MySTDocumentProps) => {
  const [isEditing, setIsEditing] = useState(false)
  const [editContent, setEditContent] = useState(content)

  // 解析 MyST Markdown
  const ast = useMemo(() => {
    if (!content || !content.trim()) {
      return null
    }
    try {
      return mystParse(content)
    } catch (e) {
      console.error('MyST parse error:', e)
      return null
    }
  }, [content])

  // 双击进入编辑模式
  const handleDoubleClick = () => {
    if (editable) {
      setIsEditing(true)
      setEditContent(content)
    }
  }

  // 保存编辑
  const handleSave = () => {
    if (onContentChange) {
      onContentChange(editContent)
    }
    setIsEditing(false)
  }

  // 取消编辑
  const handleCancel = () => {
    setEditContent(content)
    setIsEditing(false)
  }

  if (isEditing) {
    return (
      <div className="myst-editor">
        <textarea
          value={editContent}
          onChange={(e) => setEditContent(e.target.value)}
          className="w-full h-full p-4 font-mono"
        />
        <div className="editor-actions">
          <Button onClick={handleSave}>保存</Button>
          <Button onClick={handleCancel} variant="outline">取消</Button>
        </div>
      </div>
    )
  }

  if (!ast) {
    return <div>解析错误或内容为空</div>
  }

  return (
    <div 
      className="myst-document"
      onDoubleClick={handleDoubleClick}
    >
      <MyST ast={ast} />
    </div>
  )
}
```

#### 步骤 3：创建代码块组件

**文件位置**：`elfiee/src/components/editor/CodeBlockWithRun.tsx`

```typescript
// src/components/editor/CodeBlockWithRun.tsx
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism'

interface CodeBlockWithRunProps {
  code: string
  language: string
  blockId: string
}

export const CodeBlockWithRun = ({ code, language, blockId }: CodeBlockWithRunProps) => {
  const [output, setOutput] = useState<string>('')
  const [isRunning, setIsRunning] = useState(false)

  const handleRun = async () => {
    setIsRunning(true)
    try {
      // 执行代码逻辑（如果需要）
      // 这里可以根据语言类型执行不同的逻辑
      setOutput('代码执行结果...')
    } catch (error) {
      setOutput(`错误: ${error}`)
    } finally {
      setIsRunning(false)
    }
  }

  return (
    <div className="code-block">
      <div className="code-block-header">
        <span className="language-tag">{language}</span>
        <Button 
          onClick={handleRun} 
          disabled={isRunning}
          size="sm"
        >
          {isRunning ? '运行中...' : '运行'}
        </Button>
      </div>
      <SyntaxHighlighter
        language={language}
        style={vscDarkPlus}
      >
        {code}
      </SyntaxHighlighter>
      {output && (
        <div className="code-output">
          <pre>{output}</pre>
        </div>
      )}
    </div>
  )
}
```

#### 步骤 4：更新 EditorCanvas 组件

**文件位置**：`elfiee/src/components/editor/EditorCanvas.tsx`

```typescript
// src/components/editor/EditorCanvas.tsx
import { MySTDocument } from './MySTDocument'
import { useAppStore } from '@/lib/app-store'

export const EditorCanvas = () => {
  const { 
    getSelectedBlock, 
    getActiveFile, 
    writeBlockContent 
  } = useAppStore()
  
  const activeFile = getActiveFile()
  const selectedBlock = activeFile ? getSelectedBlock(activeFile.fileId) : null
  const content = selectedBlock ? getBlockContent(selectedBlock) : ''

  const handleContentChange = async (newContent: string) => {
    if (!activeFile || !selectedBlock) return
    
    await writeBlockContent(
      activeFile.fileId,
      selectedBlock.block_id,
      newContent
    )
  }

  if (!selectedBlock) {
    return <div>请选择一个文档</div>
  }

  return (
    <div className="editor-canvas">
      <MySTDocument
        content={content}
        onContentChange={handleContentChange}
        editable={true}
      />
    </div>
  )
}
```

---

## 开发检查清单

### 后端开发

- [x] 确认 `markdown.write` 能力已实现
- [x] 确认 `markdown.read` 能力已实现
- [x] 确认 Block.contents.markdown 结构正确

### 前端开发

- [ ] 安装 MyST 相关依赖
- [ ] 创建 `MySTDocument` 组件
- [ ] 创建 `CodeBlockWithRun` 组件
- [ ] 更新 `EditorCanvas` 组件
- [ ] 实现编辑模式切换
- [ ] 实现保存功能
- [ ] 实现代码块高亮
- [ ] 实现代码块运行功能（可选）
- [ ] 添加错误处理
- [ ] 编写组件测试

---

## 测试要点

### 前端测试

1. **MyST 解析测试**：
   - 解析标准 Markdown
   - 解析 MyST 扩展语法
   - 处理解析错误

2. **编辑功能测试**：
   - 双击进入编辑模式
   - 编辑内容
   - 保存内容
   - 取消编辑

3. **代码块测试**：
   - 代码高亮显示
   - 代码块运行（如果实现）

---

## 参考文档

- [迁移标准方案](../migration-standard.md)
- [MyST 文档](https://myst-tools.org/)
- [MyST Parser](https://github.com/executablebooks/mystjs)

---

**文档维护**：本文档应与代码实现同步更新。


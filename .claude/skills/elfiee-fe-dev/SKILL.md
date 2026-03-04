---
name: elfiee-fe-dev
description: Elfiee 前端开发专家 skill。适用于 React/TypeScript 前端开发任务，包括：(1) React 组件开发 - UI 界面和交互逻辑；(2) Zustand 状态管理 - Actions 和 Store 设计；(3) Tauri IPC 调用 - 通过 TauriClient 与后端通信。遵循三大硬性规则：只通过 Zustand Actions 操作数据、禁止手动编辑 bindings.ts、不直接修改状态对象。触发示例："修改 UI 组件"、"添加前端功能"、"实现状态管理"。
---

# Elfiee 前端开发指南

## 概览

Elfiee 前端使用 React 18 + TypeScript + Vite 构建，采用 Zustand 进行状态管理，Shadcn/ui + Tailwind CSS 处理样式。前端通过 Tauri IPC 与 Rust 后端通信，类型由 tauri-specta 自动生成。

## 三大硬性规则

1. **只能通过 Zustand Actions 操作数据** - 组件禁止直接调用 TauriClient
2. **禁止手动编辑 bindings.ts** - 由 tauri-specta 自动生成
3. **不要直接修改状态对象** - 必须通过 Actions

## 目录结构

```
src/
├── bindings.ts              # 【自动生成】禁止手动编辑
├── main.tsx                 # React 入口
├── App.tsx                  # 主应用组件
├── lib/
│   ├── app-store.ts         # 【核心】Zustand Store
│   ├── tauri-client.ts      # TauriClient（只在 Actions 中使用）
│   └── utils.ts             # 工具函数（cn 等）
├── components/
│   ├── ui/                  # Shadcn UI 组件
│   ├── editor/              # 编辑器组件
│   └── dashboard/           # 仪表板组件
├── hooks/
│   ├── use-toast.ts         # Toast 提示
│   └── use-mobile.tsx       # 移动端检测
├── utils/
│   └── vfs-tree.ts          # 虚拟文件系统工具
└── assets/                  # 静态资源
    └── images/
```

## 1. 数据流架构

```
组件 → Zustand Action → TauriClient → Tauri IPC → 后端
  ↑                                                  │
  └────────── State 变化触发重新渲染 ←────────────────┘
```

**关键点**：
- 组件只调用 Zustand Actions，不直接使用 TauriClient
- TauriClient 只在 `app-store.ts` 的 Actions 内部使用
- 数据从 Zustand Store 读取，UI 自动响应状态变化

## 2. Zustand Store 开发

### 2.1 Store 结构

```typescript
// src/lib/app-store.ts
import { create } from 'zustand';
import { TauriClient } from '@/lib/tauri-client';
import type { Block, Editor } from '@/bindings';

interface AppStore {
  // ============ State ============
  blocks: Map<string, Block>;
  editors: Map<string, Editor>;
  activeFileId: string | null;
  activeEditorId: string | null;

  // ============ Actions ============
  loadAllBlocks: (fileId: string) => Promise<void>;
  createBlock: (fileId: string, name: string, type: string) => Promise<string>;
  writeBlock: (fileId: string, blockId: string, content: string) => Promise<void>;
  deleteBlock: (fileId: string, blockId: string) => Promise<void>;
}

export const useAppStore = create<AppStore>((set, get) => ({
  // Initial State
  blocks: new Map(),
  editors: new Map(),
  activeFileId: null,
  activeEditorId: null,

  // Actions
  loadAllBlocks: async (fileId) => {
    const blocks = await TauriClient.block.getAllBlocks(fileId);
    const blocksMap = new Map(blocks.map(b => [b.block_id, b]));
    set({ blocks: blocksMap });
  },

  createBlock: async (fileId, name, type) => {
    const events = await TauriClient.block.createBlock(fileId, name, type);
    const blockId = events[0].entity;
    await get().loadAllBlocks(fileId);  // 重新加载
    return blockId;
  },

  writeBlock: async (fileId, blockId, content) => {
    await TauriClient.block.writeBlock(fileId, blockId, content, 'markdown');
    const updatedBlock = await TauriClient.block.getBlock(fileId, blockId);
    set((state) => {
      const newBlocks = new Map(state.blocks);
      newBlocks.set(blockId, updatedBlock);
      return { blocks: newBlocks };
    });
  },

  deleteBlock: async (fileId, blockId) => {
    await TauriClient.block.deleteBlock(fileId, blockId);
    set((state) => {
      const newBlocks = new Map(state.blocks);
      newBlocks.delete(blockId);
      return { blocks: newBlocks };
    });
  },
}));
```

### 2.2 在组件中使用 Store

```typescript
// src/components/editor/BlockEditor.tsx
import { useAppStore } from '@/lib/app-store';
import { Button } from '@/components/ui/button';
import { toast } from '@/hooks/use-toast';

function BlockEditor({ fileId, blockId }: Props) {
  // 从 Store 读取状态
  const block = useAppStore((state) => state.blocks.get(blockId));

  // 获取 Actions
  const writeBlock = useAppStore((state) => state.writeBlock);
  const deleteBlock = useAppStore((state) => state.deleteBlock);

  async function handleSave(content: string) {
    try {
      await writeBlock(fileId, blockId, content);
      toast({ title: '保存成功' });
    } catch (error) {
      toast({
        title: '保存失败',
        description: error.message,
        variant: 'destructive',
      });
    }
  }

  return (
    <div>
      <Editor value={block?.contents.markdown} onSave={handleSave} />
      <Button onClick={() => deleteBlock(fileId, blockId)}>删除</Button>
    </div>
  );
}
```

## 3. 类型安全

### 3.1 从 bindings.ts 导入类型

```typescript
// ✅ 正确：导入自动生成的类型
import type {
  Block,
  Command,
  Event,
  Editor,
  MarkdownWritePayload,
  CreateBlockPayload,
} from '@/bindings';
```

### 3.2 处理 JsonValue 类型

Block 的 `contents` 是 `JsonValue`，需要类型断言：

```typescript
const block: Block = ...;

// Markdown 块
if (block.block_type === 'markdown') {
  const markdown = (block.contents as { markdown?: string }).markdown || '';
}

// Code 块
if (block.block_type === 'code') {
  const code = (block.contents as { code?: string }).code || '';
  const language = (block.contents as { language?: string }).language || 'plaintext';
}
```

### 3.3 使用类型化 Payload

```typescript
import type { MarkdownWritePayload } from '@/bindings';

// ✅ 正确：使用类型化 Payload
const payload: MarkdownWritePayload = {
  content: 'Hello World',
};

// TypeScript 会检查类型
const wrongPayload: MarkdownWritePayload = {
  content: { type: 'text' },  // ❌ 编译错误
};
```

## 4. UI 组件开发

### 4.1 使用 Shadcn UI 组件

```typescript
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';

function MyComponent() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>标题</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <Input placeholder="输入内容" />
        <Button>提交</Button>
      </CardContent>
    </Card>
  );
}
```

### 4.2 Tailwind CSS 样式

```typescript
// 响应式设计
<div className="w-full md:w-1/2 lg:w-1/3">
  {/* 移动端全宽，平板一半，桌面三分之一 */}
</div>

// 状态变体
<button className="bg-blue-500 hover:bg-blue-600 active:bg-blue-700 disabled:bg-gray-300">
  Click me
</button>

// 条件类名
import { cn } from '@/lib/utils';

<span className={cn(
  'px-3 py-1 rounded-full text-sm',
  variant === 'error' && 'bg-red-100 text-red-800',
  variant === 'success' && 'bg-green-100 text-green-800',
)}>
  {children}
</span>
```

### 4.3 静态资源使用

```typescript
// ✅ 应用内置资源：使用 import
import logoUrl from '@/assets/images/logo.svg';
<img src={logoUrl} alt="Logo" />

// ✅ 用户数据资源：使用 convertFileSrc
import { convertFileSrc } from '@tauri-apps/api/core';
const url = convertFileSrc(filePath);
<img src={url} alt="User Image" />
```

## 5. 错误处理

### 5.1 在组件中处理错误

```typescript
import { toast } from '@/hooks/use-toast';

async function handleAction() {
  try {
    await someAction();
    toast({ title: '操作成功' });
  } catch (error) {
    toast({
      title: '操作失败',
      description: error.message,
      variant: 'destructive',
    });
  }
}
```

### 5.2 权限错误处理

```typescript
try {
  await writeBlock(fileId, blockId, content);
} catch (error) {
  if (error.message.includes('Permission denied')) {
    toast({
      title: '权限不足',
      description: '您没有权限编辑此块',
      variant: 'destructive',
    });
  } else {
    toast({
      title: '保存失败',
      description: error.message,
      variant: 'destructive',
    });
  }
}
```

## 6. 性能优化

### 6.1 选择性订阅

```typescript
// ❌ 错误：订阅整个 Store
function MyComponent() {
  const store = useAppStore();  // 任何变化都会重新渲染
}

// ✅ 正确：只订阅需要的状态
function MyComponent() {
  const blockCount = useAppStore((state) => state.blocks.size);
}

// ✅ 更好：使用 shallow 比较
import { shallow } from 'zustand/shallow';

function MyComponent() {
  const { blocks, editors } = useAppStore(
    (state) => ({ blocks: state.blocks, editors: state.editors }),
    shallow
  );
}
```

## 7. 常见错误与陷阱

| 错误 | 后果 | 解决 |
|------|------|------|
| 组件直接调用 TauriClient | 状态不一致 | 通过 Zustand Actions 调用 |
| 手动编辑 bindings.ts | 被覆盖 | 在后端定义类型 |
| 直接修改状态对象 | 不触发渲染 | 通过 Actions 修改 |
| 订阅整个 Store | 性能问题 | 选择性订阅 |
| 忘记处理错误 | 用户困惑 | try-catch + toast |

## 8. 开发检查清单

- [ ] 组件只调用 Zustand Actions，不直接使用 TauriClient
- [ ] TauriClient 只在 `app-store.ts` 的 Actions 内部使用
- [ ] 没有手动编辑 `bindings.ts`
- [ ] Payload 类型从 `bindings.ts` 导入
- [ ] 在组件中使用 try-catch 处理 Action 错误
- [ ] 不直接修改状态对象，必须通过 Actions
- [ ] 使用选择性订阅优化性能
- [ ] 静态资源放在 `src/assets/` 并通过 import 使用

## 9. 需要后端支持时

如果需要新功能但 bindings.ts 中没有对应接口：

1. **不要**尝试在前端绕过限制
2. 在后端添加 Tauri Command 或 Capability
3. 在 lib.rs 注册命令和类型
4. 运行 `pnpm tauri dev` 生成 bindings.ts
5. 在 `app-store.ts` 添加对应的 Action
6. 组件调用新的 Action

## 相关文档

- **完整规范**：`docs/mvp/guidelines/前端开发规范.md`
- **开发流程**：`docs/mvp/guidelines/开发流程.md`
- **Tauri Specta**：`docs/guides/FRONTEND_DEVELOPMENT.md`
- **架构概览**：`docs/concepts/ARCHITECTURE_OVERVIEW.md`

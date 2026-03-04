# Typed Tauri Events Migration

**Date**: 2026-02-03
**Branch**: `feat/agent-block-new`
**Scope**: Backend (Rust) + Frontend (TypeScript)

## Background

tauri-specta 自动生成的 `__makeEvents__` 和 `events` 导出在 `bindings.ts` 中一直存在，但从未注册过任何事件（Part 6 预留基础设施）。之前用 `// @ts-nocheck` 压制了 TypeScript 未使用警告（见 `relation.md` Build Fix 部分）。

项目中仅有一个 Tauri 推送事件 `pty-out`，使用原始 `app.emit("pty-out", payload)` + 前端 `listen<T>("pty-out", cb)` 手动类型标注模式。

在实现 GUI 自动刷新（Fix 1: `state_changed` 广播）时，决定统一迁移到 tauri-specta typed event 系统。

## Changes

### 新增文件

- **`src-tauri/src/events.rs`** — 集中定义所有 Tauri 推送事件
  - `StateChangedEvent { file_id: String }` — 后端状态变更通知
  - `PtyOutputEvent { data: String, block_id: String }` — 终端 PTY 输出
  - 使用 `#[derive(specta::Type, tauri_specta::Event)]` 实现 typed event

### 修改文件

#### Backend

- **`src-tauri/src/lib.rs`**
  - 添加 `pub mod events;`
  - 添加 `use tauri_specta::Event;` trait import
  - specta builder 注册 `.events(collect_events![StateChangedEvent, PtyOutputEvent])`

- **`src-tauri/src/extensions/terminal/commands.rs`**
  - 删除本地 `PtyOutputPayload` struct（`#[derive(Clone, Serialize)]`）
  - 改用 `crate::events::PtyOutputEvent`（typed event）
  - 发送方式从 `app.emit("pty-out", payload)` 改为 `PtyOutputEvent { data, block_id }.emit(&app)`
  - 移除不再需要的 `use serde::Serialize` 和 `use tauri::Emitter`

#### Frontend

- **`src/components/editor/TerminalPanel.tsx`**
  - 移除 `import { listen } from '@tauri-apps/api/event'`
  - 添加 `import { events } from '@/bindings'`
  - 监听方式从 `listen<{ data: string; block_id: string }>("pty-out", cb)` 改为 `events.ptyOutputEvent.listen(cb)`
  - 类型安全：不再需要手动 TypeScript 类型标注

- **`src/pages/DocumentEditor.tsx`**（之前已修改）
  - 使用 `events.stateChangedEvent.listen(cb)` 监听状态变更

- **`src/bindings.ts`**（自动生成）
  - 新增 `StateChangedEvent` 和 `PtyOutputEvent` 类型定义
  - `events` 导出包含 `stateChangedEvent` 和 `ptyOutputEvent`

#### Tests

- **`src/test/setup.ts`**
  - 添加 `@tauri-apps/api/event` mock（`listen`, `once`, `emit`）
  - 解决 typed event 在测试环境中调用 Tauri API 的问题

## Architecture

### Before (raw events, 散落)

```
terminal/commands.rs:  app.emit("pty-out", PtyOutputPayload { ... })
TerminalPanel.tsx:     listen<{ data: string; block_id: string }>("pty-out", cb)
```

- 事件名是字符串，前端类型靠手动标注
- `PtyOutputPayload` 定义在 `commands.rs` 内部，不可复用

### After (typed events, 集中)

```
events.rs:             PtyOutputEvent { data, block_id }.emit(&app)
                       StateChangedEvent { file_id }.emit(&app_handle)

TerminalPanel.tsx:     events.ptyOutputEvent.listen(cb)
DocumentEditor.tsx:    events.stateChangedEvent.listen(cb)
```

- 所有 Tauri 推送事件统一定义在 `events.rs`
- 前端类型由 tauri-specta 自动生成，零手动标注
- 事件名由 struct name 派生（如 `PtyOutputEvent` → `"pty-output-event"`）

### Data Flow

```
Tauri Command / MCP Server
       |
       | state_changed_tx.send(file_id)
       v
  broadcast channel (tokio)
       |
  state_rx.recv()
       |
  StateChangedEvent { file_id }.emit(&app_handle)
       |
  events.stateChangedEvent.listen(cb)  [Frontend]


PTY Reader Thread
       |
  PtyOutputEvent { data, block_id }.emit(&app)
       |
  events.ptyOutputEvent.listen(cb)  [Frontend]
```

## Test Results

- Backend: 397 unit tests + 3 doc tests passed
- Frontend: 12 test files, 98 tests passed, 0 errors

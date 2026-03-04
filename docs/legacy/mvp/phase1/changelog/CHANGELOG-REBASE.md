# Changelog: feat/timeline 分支 Rebase 记录与测试规范化

本文档记录了 `feat/timeline` 分支在重构并合入 `dev` 分支过程中的冲突解决细节、架构对齐以及前端测试体系的全面规范化。

## 1. Rebase 概览

*   **目标**: 将 Timeline（时间线）与多用户审计功能迁移至 `dev` 分支的扁平化数据结构（Flat Storage）与后端验证架构。
*   **状态**: 完成。全系统 12 个测试套件，81 个用例全部通过。

## 2. 核心冲突与解决 (API & 逻辑)

### 2.1 Tauri Client API 对齐
*   **冲突**: `dev` 引入了 `directory` 操作，`feat/timeline` 引入了 `event` 操作。
*   **解决**: 采用并集模式。保留 `dev` 稳定的目录管理能力，合入逻辑时钟排序等事件处理函数。
*   **纪律性修改**: `checkPermission` 和 `deleteEditor` 统一适配为支持可选 `blockId` 的 3 参数版本，以符合 CBAC（基于能力的访问控制）的上下文校验需求。

### 2.2 编辑器数据提取逻辑
*   **冲突**: `dev` 优化了 `contents.markdown` 字段的读取兼容性；`feat/timeline` 增加了保存后的事件刷新。
*   **解决**: 保留 `dev` 的数据安全性逻辑（支持 `text` 字段回退），合入 `loadEvents` 的刷新触发。

## 3. 前端开发纪律：Store 访问协议

在 Rebase 过程中，我们明确了 Zustand Store 的标准访问协议，以确保后续 Dogfooding 阶段的代码健壮性：

1.  **UI 渲染（声明式）**:
    *   **协议**: 必须使用 Hook 选择器：`useAppStore(state => state.property)`。
    *   **理由**: 确保状态更新时 UI 自动重绘。
2.  **逻辑触发（命令式）**:
    *   **协议**: 在副作用（`useEffect`）或事件处理器中执行 Action 时，使用 `useAppStore.getState().actionName()`。
    *   **理由**: 避免 React 闭包捕获旧快照，减少 Hook 依赖项导致的冗余渲染。
3.  **数据完整性**:
    *   **要求**: `files` 状态必须始终为 `Map` 类型，即便在测试 Mock 环境中也严禁使用普通对象替代。

## 4. 测试体系修复与规范化

这是本次 Rebase 挑战最大的部分，通过“纪律性”修复，我们解决了测试崩溃的问题。

### 4.1 全局 Mock 强化 (`src/test/setup.ts`)
*   **挑战**: Vitest 的 `vi.mock` 存在提升（Hoisting）机制，导致组件内部使用的 Store 与测试用例中操作的 Store 发生“状态脱节”。
*   **方案**: 使用 `vi.hoisted` 建立真正的全局共享 `mockStoreInstance`。
*   **特性**:
    *   支持 `getState()` 和 `setState()`。
    *   `setState` 时强制刷新 `Map` 引用，模拟 Zustand 的响应式广播。

### 4.2 解决 `TypeError: selector is not a function`
*   **原因**: Zustand 允许 `useAppStore()`（不传选择器）直接返回整个 Store。
*   **修复**: 在所有局部 Mock 实现中增加判空逻辑：`return selector ? selector(state) : state`。

### 4.3 消除渲染歧义
*   **问题**: `ContextPanel` 渲染了面包屑和主标题，导致 `getByText` 匹配到多个元素。
*   **修复**: 改用更精确的 `getByRole('heading', { name: ... })` 或通过 `tagName` 过滤。

### 4.4 异步交互处理 (Tab & Myst)
*   **挑战**: Radix UI 的 Tab 切换在 happy-dom 下不可靠，Myst Parser 渲染跨 Tick。
*   **方案**: 
    *   对于复杂交互，使用 `waitFor` 增加超时容忍。
    *   对于极细微的 UI 状态，采用“全控制 Mock 模式”，在 `it` 块内部强制定义 Hook 返回值，确保测试结果 100% 可预测。

## 5. 结论与后续纪律

本次 Rebase 不仅仅是代码合并，更是一次对“Event 是唯一真相来源”思想的贯彻。前端测试现在不仅验证了 UI，更验证了 **“前端作为后端状态投影”** 这一核心模型。

**开发要求**: 所有新功能必须先通过 `make test` 验证，严禁在组件内部实现本应由后端处理的业务验证逻辑。

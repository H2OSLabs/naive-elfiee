# Agent Block 第二轮修改记录

## 概述

基于第一轮修改后的用户测试反馈，修复了通配符 grant 失败、协作者创建流程缺失 config_dir、Configure 对话框过时等问题。同时更新了 SKILL.md 增强 block 关系的使用指导。

**分支**: `feat/agent-block`
**基准**: `dev`
**日期**: 2026-02-03

---

## 一、通配符 Grant "Block not found: *" 修复 — **已完成** ✅

### 问题

Engine Actor `process_command` 对所有非 `core.create` 命令都尝试查找 block。当 `core.grant`/`core.revoke` 使用 `block_id="*"`（通配符）时，查找失败返回 "Block not found: *"。

**影响范围**：
- `commands/agent.rs` 中 `do_agent_create()` 发出的 24 个通配符 grants **全部静默失败**
- 前端 `addGlobalCollaborator()` 抛出错误
- 任何通配符授权操作都无法正常工作

### 修复

**文件**: `src-tauri/src/engine/actor.rs`

在 `process_command` 的 block 查找逻辑中，为通配符 grant/revoke 添加特殊路径：

```rust
} else if (cmd.cap_id == "core.grant" || cmd.cap_id == "core.revoke")
    && cmd.block_id == "*"
{
    // Wildcard grant/revoke: no specific block to look up.
    // Authorization is handled by the caller (Tauri command layer or MCP server).
    None
} else {
    // Normal path: look up the block
    ...
}
```

当 `block_id == "*"` 且能力为 `core.grant`/`core.revoke` 时，跳过 block 查找，直接传 `None` 给 capability handler。GrantsTable 本身已支持通配符 block_id 的存储和匹配。

### 新增测试

| 测试 | 验证内容 |
|------|----------|
| `test_wildcard_grant_succeeds` | `core.grant` + `block_id="*"` 不再返回 "Block not found" |
| `test_wildcard_revoke_succeeds` | `core.revoke` + `block_id="*"` 不再返回 "Block not found" |

### 验证

- `cargo test` — 451 passed, 0 failed

---

## 二、GlobalCollaboratorDialog 增加 Bot 创建完整流程 — **已完成** ✅

### 问题

"Add Global Collaborator" 对话框（Create New 标签页）选择 Bot 类型时，只有 Name 和 Type 两个字段。创建 Bot 后**没有收集 config_dir**，也**没有调用 createAgent()**。用户无法一步完成 Bot 协作者的创建。

### 修复

**文件**: `src/components/permission/GlobalCollaboratorDialog.tsx`

当 `newEditorType === 'Bot'` 时，显示两个额外字段：

1. **Config Directory** — 文本输入框 + 目录选择器按钮
   - 使用 `@tauri-apps/plugin-dialog` 的 `open({ directory: true })` 实现原生目录选择
   - 占位文本: `/path/to/project/.claude`
   - 帮助文本说明 `.claude`、`.cursor` 等示例

2. **Provider** — 下拉选择
   - 选项: Claude Code（默认）、Cursor、Windsurf

提交逻辑变更：

```
Before: createEditor() → addGlobalCollaborator() → done
After:  createEditor() → addGlobalCollaborator() → createAgent(fileId, configDir, name, editorId, provider) → done
```

按钮禁用条件更新：Bot 类型时，`config_dir` 为空也禁用提交。

### 修改要点

| 修改 | 说明 |
|------|------|
| `import { open as openDialog }` | 别名避免与 Dialog `open` prop 冲突 |
| `createAgent` 从 `useAppStore()` 解构 | 用于创建 agent block |
| `configDir` + `provider` 状态 | 对话框关闭时重置 |
| Bot-specific 表单区域 | 条件渲染，仅 `newEditorType === 'Bot'` 时显示 |
| 删除 `import type { Editor }` | 类型可从 store 推断，避免 TS6133 |

---

## 三、ConfigureBotDialog 完整重写 — **已完成** ✅

### 问题

"Configure" 对话框（bot 的三点菜单 → Configure）显示过时的 Phase 1 字段：
- Model 下拉 (GPT-4, Claude 3 Opus, Grok 等)
- API Key 输入
- System Prompt 文本区

这些与 Phase 2 的 Agent 模型（config_dir + provider）完全不匹配。

### 修复

**文件**: `src/components/permission/ConfigureBotDialog.tsx`

完整重写为 Agent 配置信息展示对话框：

#### 新接口

```typescript
interface ConfigureBotDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  botName: string
  agentBlock?: Block    // 新增：传入关联的 agent block
}
```

#### 有 Agent Block 时显示

| 字段 | 展示方式 |
|------|----------|
| Status | Badge（enabled: 绿色, 其他: secondary） |
| Config Directory | 带 FolderOpen 图标的只读显示框 |
| Provider | 人类可读标签（claude_code → "Claude Code"） |
| Editor ID | 小字灰色展示 |

#### 无 Agent Block 时显示

空状态提示："No agent block created yet. Toggle the Agent switch to create one."

#### 删除的内容

- `BotConfig` 接口（model, apiKey, systemPrompt）
- `MODELS` 常量（6 个 LLM 模型列表）
- `onSave` callback prop
- `Textarea`, `Select`(model), `Input`(apiKey) 等组件
- 所有表单状态（model, apiKey, systemPrompt, isSaving）

---

## 四、CollaboratorItem 传递 agentBlock 到 ConfigureBotDialog — **已完成** ✅

### 问题

`CollaboratorItem` 已有 `agentBlock` prop（从 `CollaboratorList` 传入），但未传给 `ConfigureBotDialog`。Configure 对话框无法获取 agent 配置数据。

### 修复

**文件**: `src/components/permission/CollaboratorItem.tsx`

```diff
 <ConfigureBotDialog
   open={showConfigDialog}
   onOpenChange={setShowConfigDialog}
   botName={editor.name}
-  onSave={async (config) => {
-    console.log('Saving config:', config)
-    // TODO: Implement actual save logic
-    return Promise.resolve()
-  }}
+  agentBlock={agentBlock}
 />
```

---

## 五、CollaboratorItem 权限映射完善 — **已完成** ✅

### 问题

`getAvailableCapabilities()` 使用 if/else 结构，且缺少 `task` 和 `terminal` 块类型的映射。这两种类型会 fallthrough 到默认的 markdown capabilities，导致权限复选框显示错误的能力。

### 修复

**文件**: `src/components/permission/CollaboratorItem.tsx`

重构为 `switch` 语句，新增两个 case：

| block_type | capabilities |
|------------|-------------|
| `task` | task.read, task.write, core.delete |
| `terminal` | terminal.execute, terminal.save, core.delete |

---

## 六、claude_dir → config_dir 重命名清理 — **已完成** ✅

### 背景

第一轮修改中后端已将 `claude_dir` 更名为 `config_dir`（支持 Cursor、Windsurf 等非 Claude 工具），但前端测试中仍残留旧名称。

### 修改

**文件**: `src/components/permission/CollaboratorItem.test.tsx`
- Line 479, 584: `claude_dir` → `config_dir`

**文件**: `src/components/permission/CollaboratorList.test.tsx`
- Agent block mock 中 `claude_dir` → `config_dir`

---

## 七、SKILL.md 增强：因果链接与图优先导航 — **已完成** ✅

### 背景

SKILL.md 中 block 关系的使用指导过于简略，Claude 在操作 .elf 项目时不会主动创建 implement 链接，也不会利用已有链接进行上下文导航。

### 修改

**文件**: `.claude/skills/elfiee-mcp/SKILL.md`

#### 7.1 修正关系类型说明

将错误的 `contains`/`references` 关系类型修正为仅 `implement`（当前唯一允许的关系类型）。语义：`A → B` 表示 "A 的变更导致 B 需要变更"。

#### 7.2 新增：因果链接协议 (Causal Linking Protocol)

**核心规则**：每次因为 Block A 而修改 Block B，创建链接 `A → B`。

| 场景 | 链接 |
|------|------|
| Task 描述需求，编写 Code 实现 | Task → Code |
| PRD 定义任务，创建 Task | PRD → Task |
| Code 编写后，编写 Test 验证 | Code → Test |
| Bug 报告导致 Code 修复 | Bug → Code |
| Design 驱动 UI 组件 Code | Design → Code |

不需要链接的场景：仅读取参考、两 block 无关、链接已存在。

#### 7.3 新增：图优先上下文导航 (Graph-First Context Navigation)

**核心规则**：需要上下文时，先遍历关系图再搜索无关 block。

算法：
1. 读取目标 block
2. 获取所有 block 的 children 关系，构建反向索引
3. 向上遍历 parents 到根节点（理解 "why"）
4. 向下遍历 children（理解 "how"）
5. 读取 siblings（共享上游的相关 block）
6. 仅在信息不足时搜索无关 block

#### 7.4 新增工作流示例

- 因果修改后创建链接的示例
- 通过关系图导航上下文的示例

---

## 八、Wildcard Grant TODO 标记 — **已完成** ✅

### 背景

`commands/agent.rs` 和 `app-store.ts` 中硬编码了 24 个 capability ID 的列表。未来应支持 `cap_id = "*"` 通配符（一次 grant 覆盖所有能力）。

### 修改

**文件**: `src-tauri/src/commands/agent.rs` (line 278)
- 添加 TODO 注释：未来支持 `cap_id = "*"` 时可简化为单次 grant

**文件**: `src/lib/app-store.ts` (line 1481)
- 添加 TODO 注释：未来支持通配符 cap_id 时可简化

---

## 修改文件清单

### 后端 (Rust)

| 文件 | 修改 |
|------|------|
| `engine/actor.rs` | 通配符 grant/revoke 跳过 block 查找 + 2 个新测试 |
| `commands/agent.rs` | TODO 注释（cap_id 通配符） |

### 前端 (TypeScript/React)

| 文件 | 修改 |
|------|------|
| `GlobalCollaboratorDialog.tsx` | Bot 创建增加 config_dir + provider + auto createAgent |
| `ConfigureBotDialog.tsx` | 完整重写：Agent 配置信息展示 |
| `CollaboratorItem.tsx` | 传递 agentBlock 到 ConfigureBotDialog + 权限映射完善 |
| `CollaboratorItem.test.tsx` | claude_dir → config_dir |
| `CollaboratorList.test.tsx` | claude_dir → config_dir |
| `app-store.ts` | TODO 注释（cap_id 通配符） |

### 文档/Skills

| 文件 | 修改 |
|------|------|
| `.claude/skills/elfiee-mcp/SKILL.md` | 因果链接协议 + 图优先导航 + 关系类型修正 |

---

## 验证

- **后端**: 451 tests passed (`cargo test`)
- **前端**: 98 tests passed (`npx vitest run`)
- **TypeScript**: `npx tsc --noEmit` 无错误
- **总计**: 549 tests, 0 failures

---

**最后更新**: 2026-02-03

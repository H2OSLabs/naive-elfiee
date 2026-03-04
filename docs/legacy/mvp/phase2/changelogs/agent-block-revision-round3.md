# Agent Block 第三轮修改记录

## 概述

基于第二轮修改后的 README 审查反馈，将因果链接协议、DAG/TDD 最佳实践、图优先上下文导航等核心编辑规则写入 SKILL.md 模板，确保每个通过 Agent 连接的 Claude Code 实例自动获取这些规则。同时修正了 README 中的 5 处问题。

**分支**: `feat/agent-block-new`
**基准**: `feat/agent-block`
**日期**: 2026-02-03

---

## 一、SKILL.md 模板：因果链接协议 — **已完成** ✅

### 背景

SKILL.md 模板只有工具参考（怎么用），没有编辑规则（什么时候该创建链接）。Claude Code 不知道修改 block 时应先建立因果链接，也不知道链接方向规则。

### 修改

**文件**: `src-tauri/templates/elf-meta/agents/elfiee-client/SKILL.md`

新增 `## Causal Linking Protocol` 章节，包含：

| 规则 | 说明 |
|------|------|
| **Link Before Modify** | 修改 Block B 之前，先创建 `A → B` 链接声明因果来源 |
| **何时建链接** | Task→Code, PRD→Task, Code→Test, Bug→Code, Design→Code |
| **何时不建链接** | 仅读取参考、两 block 无关、链接已存在 |
| **DAG 约束** | 关系图是严格 DAG，环路被自动拒绝 |
| **链接方向** | 箭头从 cause 指向 effect（上游→下游） |
| **环路处理** | 返回 cycle error 说明方向有误，重新审视因果关系 |
| **TDD 最佳实践** | `Task → Code → Test`，不要创建 `Test → Code` 反向链接 |
| **多 block 修改** | 先 create task → link 所有 block → 逐一修改 → task commit |

---

## 二、SKILL.md 模板：图优先上下文导航 — **已完成** ✅

### 背景

Claude Code 读取上下文时默认搜索全部 block，没有利用已有的关系图。应优先沿关系图导航（向上理解 why、向下理解 how、横向看 siblings），仅在信息不足时搜索无关 block。

### 修改

**文件**: `src-tauri/templates/elf-meta/agents/elfiee-client/SKILL.md`

新增 `## Graph-First Context Navigation` 章节：

1. 读取目标 block
2. 通过 `elfiee_block_list` 构建 parent-child 映射
3. 向上遍历 parents（理解 why）
4. 向下遍历 children（理解 how）
5. 读取 siblings（相关模块）
6. 仅在不足时搜索无关 block

附带完整示例：通过 Task → Code → Test 关系图获取 auth 模块的完整上下文。

---

## 三、SKILL.md 模板：关系类型修正 — **已完成** ✅

### 问题

Line 100 写了 `Relation types: contains, references, implement, or custom strings.`，实际只有 `implement` 一种关系类型。

### 修改

**文件**: `src-tauri/templates/elf-meta/agents/elfiee-client/SKILL.md`

```diff
- Relation types: `contains`, `references`, `implement`, or custom strings.
+ Relation type: `implement` (the only allowed relation type). Semantic: `A → B` means "A's change caused B's change".
```

---

## 四、SKILL.md 模板：Agent 权限限制说明 — **已完成** ✅

### 问题

Permission 部分列出了 `elfiee_grant` / `elfiee_revoke` 工具，但 Agent 实际不具备 `core.grant` / `core.revoke` 能力。Claude 可能尝试调用并失败。

### 修改

**文件**: `src-tauri/templates/elf-meta/agents/elfiee-client/SKILL.md`

- 在 Capability IDs 列表后添加 blockquote 说明 Agent 无 grant/revoke 权限
- 更新 description 字段，移除 `elfiee_grant/revoke` 触发词，加入 causal linking 相关触发词

---

## 五、README.md 5 处修正 — **已完成** ✅

### 背景

用户审阅 README 后提出 5 个问题，需要修正文档内容。

### 修改

**文件**: `docs/mvp/phase2/README.md`

| # | 问题 | 修改 |
|---|------|------|
| 1 | 不清楚 Claude Code 如何使用 elfiee-client skill | 新增 "Skill 自动发现机制" 章节：Claude Code 通过 description 语义匹配自动加载，无需 `/elfiee-client` 前缀 |
| 2 | MCP 工具表错误列出 grant/revoke | 删除 `elfiee_grant` / `elfiee_revoke` 行，添加说明 Agent 无权限管理能力 |
| 3 | 图优先上下文导航是否已实现 | 新增 "上下文导航（图优先）" 章节，标注为 SKILL.md 提示词级别指导，非代码强制 |
| 4 | TDD 场景下 DAG 环路问题 | 扩展 "DAG 约束" 为完整章节，包含关系方向规则、TDD 最佳实践、环路拒绝处理 |
| 5 | 多 Agent 同时工作的支持情况 | 新增 "多 Agent 协作" 章节，说明独立端口/editor_id、无自动合并限制、最佳实践 |

额外修改：权限管理章节补充说明 Agent 不包含 grant/revoke 权限。

---

## 修改文件清单

| 文件 | 修改 |
|------|------|
| `src-tauri/templates/elf-meta/agents/elfiee-client/SKILL.md` | 因果链接协议 + 图优先导航 + 关系类型修正 + Agent 权限说明 + description 更新 |
| `docs/mvp/phase2/README.md` | 5 处修正（skill 发现、grant/revoke 表、图导航、DAG/TDD、多 Agent） |

---

**最后更新**: 2026-02-03

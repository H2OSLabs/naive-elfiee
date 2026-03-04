# Elfiee User Journey Index

本文档列出 Elfiee 二阶段 Dogfooding 中所有核心 User Journey，供内部团队验证和度量。

## 受众
Elfiee 开发团队 (Dogfooding)

## Persona
- **开发者 (Dev)**：使用 Elfiee 编写需求、代码、运行测试的工程师
- **AI Agent**：接入 Elfiee 的 AI 协作者（如 Claude Code），通过 MCP 工具消费上下文并生成代码

---

## Journey 目录

### 基础操作

| # | Journey | Persona | 描述 | 验证重点 |
|---|---------|---------|------|----------|
| J1 | [创建 .elf 项目](./J1-create-project.md) | Dev | 从零创建一个 .elf 项目并初始化结构 | 项目能成功创建、保存、重新打开 |
| J2 | [导入外部代码项目](./J2-import-project.md) | Dev | 将现有代码目录导入 Elfiee 管理 | 文件正确扫描、Block 正确创建、类型正确识别 |

### Record（记录）

| # | Journey | Persona | 描述 | 验证重点 |
|---|---------|---------|------|----------|
| J3 | [记录产品需求](./J3-record-requirement.md) | Dev | 创建 Markdown Block 记录需求，建立需求结构 | Block 创建、内容编辑、保存完整性 |
| J4 | [编写代码并关联需求](./J4-write-code-with-relation.md) | Dev | 创建 Code Block 实现需求，并通过 Relation 关联到需求 Block | 代码编辑、Link 创建、因果链完整 |
| J5 | [建立因果关系链](./J5-build-relation-chain.md) | Dev | 在需求、代码、测试 Block 之间建立完整的 Relation 链 | Relation 类型正确、链条可追溯 |
| J6 | [查看事件历史与回溯](./J6-event-history.md) | Dev | 回溯某个 Block 的完整变更历史，查看谁在什么时候做了什么 | 事件完整、归因正确、时间线清晰 |

### Learn（学习）

| # | Journey | Persona | 描述 | 验证重点 |
|---|---------|---------|------|----------|
| J7 | [配置 AI Agent 协作者](./J7-setup-agent.md) | Dev | 添加 AI Agent 为协作者，配置权限，启用 MCP 连接 | Agent 创建、权限授予、MCP 连通 |
| J8 | [AI Agent 读取需求并生成代码](./J8-agent-coding.md) | Dev + Agent | Agent 消费需求上下文，生成代码写入 Code Block，建立 Relation | Proposal FPY > 60%、Relation 自动建立 |
| J9 | [Terminal 验证代码](./J9-terminal-verify.md) | Dev / Agent | 在 Elfiee 内通过 Terminal Block 运行测试，验证代码正确性 | 命令执行成功、输出捕获、不离开 Elfiee |
| J10 | [Task 管理与 Git 提交](./J10-task-commit.md) | Dev | 使用 Task Block 跟踪开发任务，完成后通过 task.commit 提交 Git | Task 状态更新、Git 提交成功、消息规范 |

### 协作管理

| # | Journey | Persona | 描述 | 验证重点 |
|---|---------|---------|------|----------|
| J11 | [管理协作者与权限](./J11-manage-collaborators.md) | Dev | 邀请人类/AI 协作者，配置 Block 级别权限 (CBAC) | Grant/Revoke 生效、权限边界正确 |

### 端到端 Dogfooding

| # | Journey | Persona | 描述 | 验证重点 |
|---|---------|---------|------|----------|
| J12 | [Dogfooding 全流程](./J12-dogfooding-e2e.md) | Dev + Agent | 完整的「需求记录 → AI 编码 → Terminal 验证 → 决策回溯」闭环 | 覆盖所有 Phase 2 验证指标 |

---

## 与 Phase 2 验证指标的映射

| 验证指标 | 成功基准 | 对应 Journey |
|----------|----------|-------------|
| **Proposal 首次通过率 (FPY)** | > 60% | J8, J12 |
| **逻辑回溯时间** | < 30 秒 | J5, J6, J12 |
| **Terminal 修复闭环率** | > 90% | J9, J12 |
| **Summary 采纳率** | > 80% | J3, J12 |
| **Memo 使用频次** | > 3 条/功能 | J3, J12 |

---

## 编写状态

| Journey | 状态 |
|---------|------|
| J1 - 创建 .elf 项目 | ⬜ 待编写 |
| J2 - 导入外部代码项目 | ⬜ 待编写 |
| J3 - 记录产品需求 | ⬜ 待编写 |
| J4 - 编写代码并关联需求 | ⬜ 待编写 |
| J5 - 建立因果关系链 | ⬜ 待编写 |
| J6 - 查看事件历史与回溯 | ⬜ 待编写 |
| J7 - 配置 AI Agent 协作者 | ⬜ 待编写 |
| J8 - AI Agent 读取需求并生成代码 | ⬜ 待编写 |
| J9 - Terminal 验证代码 | ⬜ 待编写 |
| J10 - Task 管理与 Git 提交 | ⬜ 待编写 |
| J11 - 管理协作者与权限 | ⬜ 待编写 |
| J12 - Dogfooding 全流程 | ⬜ 待编写 |

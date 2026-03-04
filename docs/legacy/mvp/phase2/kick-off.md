# Phase 2 kick-off

## 1. Phase 1 回顾

### 1.1 阶段成果

Phase 1 聚焦"统一工作面"和"决策资产化"的基础能力验证，在**单人单机多角色协作**场景下完成核心架构搭建：

**研发成果**：
- Dashboard：.elf 文件管理（创建、导入、重命名、删除）
- Editor 核心模块：Directory（内外部目录）、Markdown 编辑、协作者权限（CBAC）、Event 记录与回溯（Timeline）
- 核心架构：Event Sourcing、Block-based、Capability-based、Actor 模型

**产品成果**：
- 竞品分析：对比大厂工作流、Google 工作流、Vibe Coding 工作流
- 用户实验：验证记录行为的价值和不同记录形式的效果
- 核心发现：Summary + Traceability 是方向，单纯 Log 和 Tag 效果不佳

### 1.2 关键洞察

**决策资产化的价值不在于给人读，而在于让 AI 学**

### 1.3 核心 Gap

| Gap | 描述 |
| :--- | :--- |
| **AI 未接入** | 当前系统只有人类角色，无法验证 AI 原生编辑器定位 |
| **验证闭环缺失** | 没有 Terminal 连接外部工具链，无法完成需求→实现→测试闭环 |
| **逻辑链条不可见** | Event 已记录但因果关系未建模，无法追溯决策依赖 |
| **决策表达层薄弱** | 缺少结构化决策表达，无法验证可学习性 |

---

## 2. Phase 2 项目目标

### 2.1 大目标

**通过 Dogfooding（用 Elfiee 开发 Elfiee）验证 AI 辅助的闭环开发能力**

具体验证：
1. AI Agent 能作为协作者参与完整开发流程（需求理解 → 代码生成 → 测试运行）
2. 决策记录能被 AI 消费并产生价值（而非仅供人类阅读）
3. Elfiee 能成为团队的日常开发工具

### 2.2 三大方向

| 方向 | 描述 | 对应模块 |
| :--- | :--- | :--- |
| **AI 接入协作** | AI Agent 以"编辑者"身份参与协作，消费历史决策 | Agent Block Extension |
| **打通验证闭环** | 连接外部工具链，完成"需求→实现→测试"闭环 | Terminal Extension |
| **建立因果索引** | Event 之间建立因果关系，支撑 DAG 可视化和 AI 上下文优化 | Relation 增强 |

### 2.3 关键假设验证

| 假设 | 验证方式 | 成功标准 |
| :--- | :--- | :--- |
| AI 能理解历史决策 | Agent 读取 Event + Relation，生成符合项目规范的代码 | Proposal 首次通过率 > 60% |
| Terminal 闭环提效 | 在 Elfiee 内完成需求→代码→测试全流程 | 不离开 Elfiee 完成 1 个真实功能 |
| 因果链有助于理解 | 新成员通过 Relation Graph 快速理解项目背景 | 新成员理解时间 < 1 小时 |

---

## 3. Phase 2 用户故事

### 3.1 角色设定

- **PM（产品经理）**: 提出需求，定义验收标准
- **Agent（AI 助手）**: 根据需求生成代码，关联测试
- **Developer（开发者）**: 审核 AI 代码，运行测试，提交 PR

### 3.2 Dogfooding 场景

**场景**：团队需要为 Elfiee 开发"PDF 导出"功能，使用 Elfiee 自身完成这个开发任务。

**完整工作流**：
```
PM: 创建需求 Block
  ↓
PM: @Agent 生成开发 Prompt
  ↓
Agent: 读取需求 + 项目代码
  ↓
Agent: 生成 Proposal（包含代码变更）
  ↓
PM/Dev: 审核 Proposal（查看 Diff）
  ↓ Approve
Agent: 执行 Commands，修改 Block
  ↓
Dev: 在 Terminal 运行测试
  ↓ Pass
Dev: 导出文件，git commit
  ↓
完成：需求→代码→测试的完整闭环
```

### 3.3 详细步骤

#### (1) 环境准备
- PM 创建 `.elf` 文件：`Elfiee-PDF-Feature.elf`
- PM 创建 `Directory Block`，导入 Elfiee 项目源码
- PM 创建 `Agent Block`，配置 LLM（Provider=Anthropic, Model=claude-sonnet-4-5）

**价值验证**：用户能否快速挂载外部项目和 AI Agent

#### (2) 需求定义
- PM 创建 `Markdown Block`："PDF 导出需求"
- 编写需求和验收标准
- PM 关联需求 Block 到 Agent

**价值验证**：需求能否被结构化记录并关联到 Agent

#### (3) Agent 思考与提案
- Agent 读取需求内容和项目代码结构
- Agent 调用 LLM API，生成 Proposal：
  - Reasoning（解释）
  - Proposed Commands（创建文件、写入代码、建立 Relation）

**价值验证**：AI 能否理解需求并生成可预览的方案

#### (4) 人工审批
- PM/Dev 看到 Proposal Card，包含：
  - Reasoning（AI 的解释）
  - Diff 预览（Monaco Editor 显示新增代码）
  - 关系图（需求 → 代码的 implement 关系）
- Dev 点击 Approve

**价值验证**：用户能否理解 AI 意图并做出决策

#### (5) Agent 执行
- 系统依次执行 Proposal 中的 Commands
- 创建代码 Block，写入内容
- 建立 `需求 Block → 代码 Block` 的 implement 关系

**价值验证**：系统能否正确执行 AI 指令

#### (6) 测试与验证
- Dev 在 Terminal 中运行 `cargo test pdf`
- Terminal 在外部目录（挂载的 Directory Block）执行测试
- 测试通过后，Dev 导出文件到外部项目
- 在外部终端 `git commit`

**价值验证**：闭环验证能否在 Elfiee 内完成

#### (7) 决策追溯
- PM/Dev 查看 Relation Graph
- 从"PDF 导出需求"沿 implement 关系找到所有相关代码
- 通过 Event Timeline 回溯每次修改的决策上下文

**价值验证**：决策能否被追溯和理解

### 3.4 用户故事拆分和价值点

#### F1: Terminal 灵活执行（P0）
- **价值点**：Terminal 支持 cwd 到外部目录执行命令
- **验收标准**：
  - Terminal 可关联 Directory Block
  - 支持 `cd ~` 解析到 Block 解压目录
  - 显示命令输出

#### F2: Terminal-Block 交互能力（P1）
- **价值点**：Workspace 挂载和自动同步
- **验收标准**：
  - `terminal.workspace_mount` 导出物理文件
  - `terminal.workspace_sync` 监听文件变化并同步回 Block
  - 配置 Agent 调用方式（API/Terminal）

#### F3: Agent Block 创建与配置（P0）
- **价值点**：快速创建和配置 AI 助手
- **验收标准**：
  - 创建 Agent Block 自动生成 editor_id
  - 配置 Provider、Model、API Key、System Prompt
  - 验证 API Key 环境变量

#### F4: LLM API 集成（P0）
- **价值点**：调用 LLM 生成代码
- **验收标准**：
  - 封装 Anthropic API 调用
  - 支持流式响应
  - 解析结构化输出（XML/JSON）
  - 错误处理和重试

#### F5: Proposal 交互机制（P0）
- **价值点**：AI 建议可预览、可审批
- **验收标准**：
  - `agent.invoke` 生成 Proposal Event
  - Proposal Card 显示 Reasoning 和 Commands
  - Diff 预览代码变更
  - Approve 执行 Commands

#### F6: Agent 上下文感知与执行（P0）
- **价值点**：AI 能读取关联 Block 和 Event 历史
- **验收标准**：
  - 读取 `Block.children` 中的 reference 关系
  - 收集关联 Block 内容作为上下文
  - Token 截断策略（基础版本和 Event 优化版本）

#### F7: Relation 数据结构与索引（P0）
- **价值点**：建立 Block 之间的因果关系
- **验收标准**：
  - RelationGraph 双向索引（outgoing/incoming）
  - 环检测算法
  - Relation 查询 API（get_parents/get_children）
  - Event 逻辑整理 API（get_events_by_relation）

#### F8: Relation Graph 可视化（P1）
- **价值点**：可视化决策链条
- **验收标准**：
  - React Flow 渲染 DAG
  - DAG 布局算法（dagre）
  - 节点交互（点击跳转、Hover 显示信息）
  - Event 逻辑链展示（高亮因果链路径）

---

## 4. Phase 2 MVP 功能模块支持和优先级排序

### 4.1 研发任务规划（183 人时）

| 功能模块 | 细分功能 | 主要文件 | 预计人时 |
|---------|---------|---------|---------|
| **Terminal Extension** | Terminal 基本功能 | `pty.rs` | 8 |
|  | Workspace 挂载 | `workspace_mount.rs` (新建) | 6 |
|  | 文件监听与同步 | `workspace_sync.rs` (新建) | 8 |
|  | Agent 调用配置 | `agent_config.rs` (新建) | 3 |
|  | 前端 UI | `TerminalBlock.tsx`, `WorkspaceStatus.tsx` | 10 |
| **Agent Extension** | Agent 模块骨架 | `mod.rs` (新建) | 2 |
|  | agent.create | `agent_create.rs` (新建) | 4 |
|  | LLM Client | `llm/anthropic.rs` (新建) | 8 |
|  | 结构化输出解析 | `llm/parser.rs` (新建) | 4 |
|  | agent.invoke/approve | `agent_invoke.rs`, `agent_approve.rs` (新建) | 10 |
|  | 上下文收集 | `context/collector.rs` (新建) | 6 |
|  | 上下文优化 | `context/truncator.rs`, `context/event_optimizer.rs` (新建) | 9 |
|  | 前端 UI | `AgentConfigForm.tsx`, `ProposalCard.tsx` 等 | 28 |
| **Relation 模块** | RelationGraph 结构 | `state.rs` (修改) | 4 |
|  | 双向索引 | `state.rs` (修改) | 4 |
|  | 环检测 | `state.rs` (修改) | 5 |
|  | Relation API | `commands/relation.rs` (新建) | 5 |
|  | Event 逻辑整理 | `commands/relation.rs` (扩展) | 6 |
|  | 前端可视化 | `RelationGraph.tsx`, `EventLogicChain.tsx` | 18 |
| **测试与集成** | 单元测试 | `terminal/tests.rs`, `agent/tests.rs` 等 | 8 |
|  | 集成测试 | `tests/integration/` | 8 |
|  | E2E 测试 | `src/__tests__/e2e/` | 4 |
| **总计** |  |  | **183** |

### 4.2 产品任务规划（85 人时）

| 功能模块 | 细分功能 | 预计人时 |
|---------|---------|---------|
| **需求与交互设计** | Terminal 交互规范 | 5 |
|  | Agent 交互规范 | 8 |
|  | Proposal UI 原型 | 8 |
|  | Markdown @agent 交互设计 | 5 |
|  | Relation 可视化规范 | 5 |
|  | Relation Graph 设计 | 4 |
| **用户研究** | AI 协作竞品分析 | 10 |
|  | Dogfooding 场景 1 | 8 |
|  | Dogfooding 场景 2 | 7 |
| **知识沉淀** | 交互规范文档输出 | 6 |
|  | Dogfooding 效果报告 | 8 |
|  | 竞品分析报告 | 5 |
|  | Agent Prompt 模板 | 6 |
| **总计** |  | **85** |

### 4.3 里程碑与时间安排

**总工期**：3 周（268 人时）

| 里程碑 | 时间节点 | 验收标准 |
| :--- | :--- | :--- |
| **M1: 基础功能就绪** | Week 1 Day 1-3 | Terminal cwd、Agent 基本配置、RelationGraph 结构完成 |
| **M2: 交互验证** | Week 1 Day 4-5 | 在 Markdown 中 @agent 调用成功，Terminal 执行命令 |
| **M3: 完整 Agent 功能** | Week 2 Day 1-3 | Agent 复杂上下文、Workspace 挂载、Proposal 解析执行 |
| **M4: Relation 可视化** | Week 2 Day 4-5 | RelationGraph 渲染 DAG，节点交互 |
| **M5: Dogfooding 追踪计划** | Week 2 末 | 产品完成场景设计和指标定义 |
| **M6: Event 逻辑整理** | Week 3 Day 1-2 | Relation 追溯 Event 因果关系 |
| **M7: Dogfooding 执行** | Week 3 Day 3-4 | 产品执行 Dogfooding 场景 2，研发修复 Bug |
| **M8: 上下文优化** | Week 3 Day 5 | Event 逻辑裁剪算法，Dogfooding 报告输出 |

### 4.4 验收标准

**研发验收**：
- 核心功能完成率 100%（F1-F8）
- Dogfooding 完成 1 个真实功能开发（PR 合并 + Event 记录完整）
- Proposal 首次通过率 > 60%
- Terminal 闭环验证（不离开 Elfiee 完成需求→代码→测试）

**产品验收**：
- 交互规范完成（Agent、Terminal、Relation）
- Dogfooding 问题收集至少 20 个改进点
- 新成员理解效率 < 1 小时
- 需求澄清次数 < 5 次/功能

**整体验收**：
- ✅ 功能完整：F1-F8 全部实现，P0 功能 100% 可用
- ✅ 闭环验证：团队成员能用 Elfiee + Agent 完成日常开发任务
- ✅ AI 效果：Proposal 首次通过率 > 60%
- ✅ 可追溯性：至少 3 个关键决策能通过 Relation Graph 回溯
- ✅ 持续改进：Dogfooding 发现并整理至少 20 个核心改进方向

---

## 5. 风险与应对

### 5.1 开发侧风险

| 风险类型 | 预留时间 | 应对策略 |
| :--- | :--- | :--- |
| LLM 集成调试 | 8-12 人时 | 优先实现 Mock 测试，延后真实 API 调用 |
| 文件同步机制复杂度 | 10-15 人时 | 优先单向同步，双向同步推迟；降级为手动触发 |
| Event 逻辑优化算法 | 6-10 人时 | 优先简单版本，高级优化推迟到 Phase 3 |
| 规范修改返工 | 5-10 人时 | 每周规范评审，快速调整 |
| 产品方向调整 | 3-10 人时 | Week 2 初中期评审，及时止损 |

**风险预留**：32-57 人时（约 12-21% 缓冲）

### 5.2 关键节点

- **Week 1 结束（M1-M2）**：M2 交互验证不通过需立即返工
- **Week 2 结束（M3-M5）**：M5 Dogfooding 追踪计划必须完成
- **Week 3 Day 2（M6）**：Event 逻辑整理不成熟则降级为简单版本
- **Week 3 Day 5（M8）**：上下文优化复杂度高则推迟到 Phase 3

### 5.3 降级策略

- **F2 文件同步**：降级为手动触发同步（"Sync to Block" 按钮）
- **M8 上下文优化**：降级为基于时间的简单截断（B-AGENT-11）
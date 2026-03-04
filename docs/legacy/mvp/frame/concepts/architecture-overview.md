# Architecture Overview: Elfiee Refactoring

> 基于三层架构（`frame.jpg` + `uni-frame_v2.md`）的重构方案概览。
> 本文档不涉及具体技术实现，聚焦于架构定位、设计理念和子文档索引。

---

## 一、重构动机

### 1.1 现状问题

Phase 1 的 Elfiee 是一个独立的桌面编辑器，承担了过多职责：文件系统管理、终端进程托管、Git 集成、Agent MCP 服务器分配等。这导致：

- **定位模糊**：既是编辑器、又是运行时、又是文件管理器
- **扩展困难**：每增加一个外部能力（如新 Agent Provider），都需要修改 Elfiee 核心代码
- **无法融入更大的生态**：作为独立桌面应用，无法被外部系统（如 AgentChannel、AgentContext）调用和编排

### 1.2 重构目标

将 Elfiee 从"全能桌面编辑器"收束为 **EventWeaver（事件织机）**，专注于四件事：

1. **Event Sourcing** — 记录决策事实
2. **Literate Programming** — 将决策、实现、验证编织成可读的叙事
3. **Agent Building** — 创造和编排智能体
4. **Native Dogfooding** — 用自身开发自身

其他能力（文件系统、终端执行、Git 操作、Agent 运行环境）委托给三层架构中的 AgentContext 层。

---

## 二、产品理念契合

重构方案与 Phase 2 PRD（`docs/mvp/phase2/prd/`）的核心命题保持一致：

### 2.1 "让决策可学习" (Record → Learn)

| PRD 命题 | 重构中的体现 |
|---|---|
| **Record：把决策记录下来** | Event Sourcing 的 EAVT 模型记录所有变更事实。三种 Block（document / task / session）覆盖产出物、工作分配和执行过程 |
| **Learn：让记录可学习** | Agent 通过 DAG + Vector Clock 遍历事件链，进行上下文压缩和 Skill 生产。不依赖人工提炼 |

### 2.2 产品价值的落地

| 产品价值（`Product Value.md`） | 架构支撑 |
|---|---|
| **动作即资产**：工作过程自动沉淀为 Skill | Session Block 自动记录执行过程（对话、命令、决策）。Event 链条 + DAG 关系使得 Skill 可追溯、可评估、可迭代 |
| **Source of Truth for AI Reasoning**：为 Agent 提供因果地图 | Event 是事实唯一来源。Block 间的 `implement` 关系构成因果链。Agent 通过回溯 DAG 获得完整上下文，而非概率猜测 |

### 2.3 Dogfooding 验证标准的支撑

| Dogfooding 指标 | 架构如何支撑 |
|---|---|
| **Proposal 首次通过率 > 60%** | Agent 消费的上下文来自 Event + DAG，而非碎片化的文件内容。结构化的决策链提高 Agent 的理解准确度 |
| **逻辑回溯时间 < 30秒** | Snapshot 机制 + DAG 遍历，支持任意时间点的状态查看。不需要人工翻阅历史 |
| **Session 修复闭环率 > 90%** | Session Block 记录完整的命令执行历史。通过 AgentContext 提供的 bash session 直接执行修复 |
| **Summary 采纳率 > 80%** | Agent 基于 Session + Document 的 Event 链自动生成 Summary，人只需审查确认（Reviewer 模式） |
| **Memo 使用频次 > 3条/功能** | Document Block 支持轻量级的随手记，通过 link 关联到 Task，不增加额外操作负担 |

---

## 三、三层架构定位

重构基于 `frame.jpg` 定义的三层架构，明确 Elfiee 在其中的边界：

```
┌───────────────────────────────────────────────────┐
│  AgentChannel (Matrix 生态层)                       │
│  Baths(Synapse) · Agent Register · Agent Router    │
│  职责：消息路由、Agent 注册、身份管理                    │
└───────────────────────┬───────────────────────────┘
                        │
┌───────────────────────┴───────────────────────────┐
│  Agent (业务层)                                     │
│  ┌─────────┐  ┌───────────┐  ┌────────┐           │
│  │ Elfiee  │  │Synnovator │  │Ezagent │           │
│  │ 编排·决策 │  │ 社群·内容  │  │ 前端UI  │           │
│  └─────────┘  └───────────┘  └────────┘           │
└───────────────────────┬───────────────────────────┘
                        │
┌───────────────────────┴───────────────────────────┐
│  AgentContext (资源层) / OneSystem                   │
│  Machine · Credentials · FileSystem · Logs          │
│  职责：运行环境、文件管理、凭据管理、日志                  │
└───────────────────────────────────────────────────┘
```

### Elfiee 做什么 / 不做什么

| 做 | 不做（委托给谁） |
|---|---|
| Event Sourcing：决策事实的记录与回放 | 文件系统管理 → AgentContext |
| CBAC：能力授权与权限隔离 | 终端/Bash 执行 → AgentContext |
| Block DAG：内容的结构化组织与关联 | Git 操作 → AgentContext |
| 工作模板（Editor 角色 + 权限矩阵） | Agent 运行时托管 → AgentContext |
| 文学式编程叙事的编排与渲染 | Agent 注册与路由 → AgentChannel |
| Skill 的生产与演化 | 社群分享与模板市场 → Synnovator |

---

## 四、核心四大支柱

### 4.1 Event Sourcing — 决策的事实来源

Event 是系统中一切状态的唯一权威记录。不存储推理（reasoning），只存储事实（fact）。推理由消费方（Agent）通过 DAG 遍历和 Vector Clock 关联自行推断。

Event 系统需要支持：
- 四种内容模式（全量 / 增量 / 引用 / 追加）覆盖文本、二进制和会话记录
- Checkpoint 快照机制，支持任意时间点的断点回溯
- Agent 可读的结构化格式，便于上下文压缩

详见 → [`event-system.md`](#子文档索引)

### 4.2 Literate Programming — 决策的叙事编织

文学式编程不再通过"把所有内容写进一篇 Markdown"来实现，而是通过 **Block DAG + MyST 渲染**：

- **数据层**：Block 是独立的内容单元（代码、任务、会话记录），通过 DAG link 建立因果关系
- **叙事层**：Document Block 使用 MyST directive/role 引用其他 Block，编排成一篇可读的叙事文档
- **渲染层**：前端解析 directive，从 Block DAG 中拉取内容，拼接渲染为完整的文学式文档

这使得叙事本身也是一个 Block，享有 CBAC、Event Sourcing 的所有特性。

详见 → [`literate-programming.md`](#子文档索引)

### 4.3 Agent Building — 智能体的创造与演化

Elfiee 的核心差异化能力：不只是 **使用** Agent，而是 **创造** Agent。

- Agent 以 Editor 身份参与，受 CBAC 保护
- 工作模板（`.elf/templates/`）定义参与者角色、权限矩阵、演化策略（Socialware 声明）
- Skill 从执行过程中自动提炼：Session Event → 模式识别 → Skill Block
- Skill 的演化路线：Local Case → Shared Skill → Organized Skill（对应 `uni-frame.md` 中的 Agent 0→1→2）

详见 → [`agent-building.md`](#子文档索引)

### 4.4 Native Dogfooding — 用自身开发自身

Phase 2 的核心验证策略是 Dogfooding（`Dogfooding Plan.md`）。架构必须原生支持：

- Elfiee 作为 Agent 可被 AgentChannel 调用，自身也是 Agent 生态的一员
- 开发 Elfiee 的过程产生的 Event 和 Session 记录，本身就是 Dogfooding 数据
- Missing Tools（`Missing tools and Manual Substitutes.md`）中标记的缺失能力（FPY、task.commit、Summary 采纳率等）需要在新架构中原生支持
- 统一的通讯协议使得本地开发和远程协作使用同一套工具链

详见 → [`dogfooding-support.md`](#子文档索引)

---

## 五、子文档索引

以下文档按 **从内核到外层的构建顺序** 排列。每一层的模块仅依赖前序层级，可在该阶段建立自洽的测试体系，无需依赖后续文档。

```
Layer 0  ─── data-model ──────────────────────── 基础定义
Layer 1  ─── event-system ─── cbac ────────────── 核心机制（平行，仅依赖 L0）
Layer 2  ─── elf-format ──────────────────────── 存储格式（依赖 L1-event）
Layer 3  ─── engine ──────────────────────────── 引擎整合（依赖 L0 + L1 + L2）
Layer 4  ─── extension-system ─── communication ─ 扩展与通讯（依赖 L3，MCP SSE）
Layer 5  ─── literate-programming ─── agent-building ─ 应用层（依赖 L4）
Layer 6  ─── dogfooding-support ─── migration ─── 验证与迁移
```

| # | 层级 | 文档 | 依赖 | 可独立测试 | 覆盖内容 |
|---|---|---|---|---|---|
| 1 | L0 基础 | **[`data-model.md`](./data-model.md)** | 无 | 实体序列化/反序列化、字段校验、关系图构建 | **核心数据模型**。定义系统中所有核心实体及其关系：**Block**（三类：Content / Orchestration / Record，统一 markdown 和 code 为 Document，明确 Agent / Task / Session 的职责边界）、**Event**（EAVT 四元组的字段定义与语义）、**Editor**（人类与 Bot 的身份模型，与 AgentChannel 身份体系的映射）、**Command**（意图的结构定义，从 Tauri Command 到统一消息的演进）、**Grant**（CBAC 授权记录的数据结构）、**Capability**（操作单元的元数据定义）。实体间的关系图：Block↔Event、Editor↔Event、Block↔Block DAG、Editor↔Grant↔Block。与 Phase 1 数据模型的对比与迁移路径 |
| 2 | L1 机制 | **[`event-system.md`](./event-system.md)** | L0 | Event 存储 CRUD、replay 回放、snapshot 快照、mode 判断 | **事件系统设计**。EAVT 模型的延续与升级。四种内容模式（`full` / `delta` / `ref` / `append`）的定义与适用场景。Checkpoint 快照机制的触发策略与查询流程。Event 与 Git 的关系：Event 是高频决策记录，Git 是低频 checkpoint 锚点。大文件与二进制文件的引用策略 |
| 3 | L1 机制 | **[`cbac.md`](./cbac.md)** | L0 | in-memory Grants 表、authorization check、Owner/Grant 两层判断 | **能力授权模型**。CBAC 在新 Block 分类下的适配。Task 粒度的权限隔离（Agent 只能操作被分配 Task 关联的 Block）。工作模板中的权限矩阵声明。Owner / Grant 两层授权（纯 Event-sourced，无配置文件 bypass）。Grant/Revoke 本身也是 Event（依赖 L1-event 的语义定义，但 CBAC 逻辑可独立测试）。与 AgentChannel 身份体系的映射 |
| 4 | L2 存储 | **[`elf-format.md`](./elf-format.md)** | L1-event | .elf/ 目录创建、config 读写、eventstore.db 初始化 | **`.elf` 文件格式重定义**。从 ZIP 归档改为类 `.git/` 的元数据目录。目录结构：eventstore.db + templates/ + config.toml。不再存储文件内容（文本内容在 Event 中，二进制在项目文件系统中）。可分享性：打包导出机制（类似 git bundle）。与 AgentContext 文件系统的关系 |
| 5 | L3 引擎 | **[`engine.md`](./engine.md)** | L0 + L1 + L2 | 完整 command → authorize → execute → persist → project 集成测试 | **引擎架构演进**。Actor 模型在新场景下的适用性：一个 `.elf` 一个 Actor，跨 Actor 通过消息通讯。EngineManager 的职责收束。StateProjector 对新 Event 模式（delta / ref）的适配。Snapshot 表的引入。冲突处理：当前保持 OCC，预留 CRDT 扩展接口 |
| 6 | L4 扩展 | **[`extension-system.md`](./extension-system.md)** | L3 | 单个 Capability handler 的输入→事件输出测试 | **扩展系统规范**。明确本版核心边界：Event Sourcing + CBAC + Block DAG + Actor Model 是内核，Extension 是在内核之上的功能扩展。Extension 的定位：纯事件生产者，不做 I/O（I/O 委托给 AgentContext）。Extension 开发者需要定义的三件事：Block 内容 Schema、Capability 集合、MyST 渲染 Directive。新 Extension 的方向指引：围绕三类 Block 扩展。与 Phase 1 扩展系统的对比：去掉 I/O 副作用后，Extension 变为可测试、可组合的纯函数单元 |
| 7 | L4 通讯 | **[`communication.md`](./communication.md)** | L3 | MCP SSE 连接管理、per-connection 认证 | **统一通讯架构**。MCP SSE 作为唯一传输协议（所有客户端平等）。`elf serve` 持久进程模型。ConnectionRegistry 管理 per-connection editor_id。`elfiee_auth` tool 实现连接级身份认证。状态通知通过 MCP notification 扇出。与 AgentChannel 的对接方式 |
| 8 | L5 应用 | **[`literate-programming.md`](./literate-programming.md)** | L4-ext + L0 | MyST directive 解析、Block DAG 遍历渲染 | **文学式编程的实现**。从"单文档嵌入一切"到"Block DAG + 叙事渲染"的范式转换。MyST directive/role 在 Document Block 中的使用：嵌入 Task / Code / Session 内容。前端渲染流程：解析 directive → 查询 Block → 拼接叙事。与传统文学式编程（Knuth）和 Phase 1 方案（MyST 单文档）的对比 |
| 9 | L5 应用 | **[`agent-building.md`](./agent-building.md)** | L4-ext + L1-cbac | 模板加载、Task 分配、Skill 提炼流程 | **Agent 创造与编排**。Editor 作为统一身份模型（人类和 Agent 平等）。Task Block 作为工作分配的枢纽。工作模板（templates/）的定义：参与者角色、权限矩阵、演化策略（Socialware 声明）。Session Block 到 Skill 的提炼路径。Skill 演化三阶段（Local → Shared → Organized）。与 Synnovator 模板市场的对接 |
| 10 | L6 验证 | **[`dogfooding-support.md`](./dogfooding-support.md)** | 全部 | Phase 2 指标的端到端验证 | **Dogfooding 原生支持**。Phase 2 Dogfooding Plan 中要求的能力在新架构中的实现路径。FPY 度量、task.commit 语义、Summary 生成、Memo 支持。Missing Tools 清单的逐项覆盖 |
| 11 | L6 迁移 | **[`migration.md`](./migration.md)** | 全部 | 无（规划文档） | **迁移与瘦身清单**。Phase 1 代码的删除/保留/改造清单。迁移顺序与风险评估 |

---

## 六、与 Phase 1 Concepts 的关系

本文档及子文档是对 `docs/concepts/` 中 Phase 1 设计的**演进而非替代**：

| Phase 1 Concept | 延续 | 演进 |
|---|---|---|
| 区块化编辑 (Block-based) | Block 仍然是一切内容的基本单元 | 从 6 种类型收束为 3 种（document / task / session），block_type 为 String 可自由扩展 |
| 事件溯源 (Event Sourcing) | EAVT 模型不变，Event 仍是事实唯一来源 | 新增内容模式（full / delta / ref）、Checkpoint 快照、Snapshot 表 |
| 能力驱动 (Capability-based) | Capability 作为操作的最小单元不变 | 删除 I/O 密集型 capability（directory.import/export, terminal.init/execute），新增编排型 capability |
| Actor 模型 | 一个 .elf 一个 Actor 不变 | 通讯层统一为 MCP SSE（`elf serve` 单端口，所有客户端平等） |
| .elf 文件格式 | eventstore.db 作为核心不变 | 从 ZIP 归档演进为 `.elf/` 元数据目录 |

---

## 七、文档编写原则

所有子文档遵循以下原则：

1. **只讲"为什么"和"是什么"**，不讲"怎么写代码"
2. **每个设计决策都关联到产品理念**（Record/Learn、动作即资产、Source of Truth）
3. **每个变更都说明与 Phase 1 的差异**和迁移路径
4. **使用 Mermaid 图表**而非代码块来表达架构关系
5. **保持与 `docs/concepts/` 相同的文档风格**：理念先行、示例辅助

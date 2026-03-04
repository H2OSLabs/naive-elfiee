# Unified Framework: 四平台架构与数据流

## 一、四平台定位

| 平台 | 关注点 | 核心能力 | 运行位置 |
|------|--------|----------|----------|
| **Synnovator** | 社群 & 内容 | 帖子发布、页面管理、模板市场、社区展示 | 云端 Web |
| **Matrix Com** | 通讯 & 路由 | /命令分发、Agent 托管、群聊/单聊、事件桥接 | 自托管/云端 |
| **Elfiee** | 本地智能 & Agent 编排 | 事件溯源、CBAC 权限、多 Agent 模板、Skill 演化 | 本地桌面 |
| **One System** | 资源 & 运行时 | Secret 管理、SSH 隧道、卷挂载、Agent 运行环境 | 自托管 |

**核心设计原则**：Matrix 是消息总线，不是业务逻辑层。各平台保持独立可运行，Matrix 负责连接和路由。

---

## 二、分层架构

```mermaid
graph TD
    subgraph L0["Layer 0: User Interfaces"]
        UI_SYN["Synnovator<br/>(Web)"]
        UI_MAT["Matrix Client<br/>(Element/自研)"]
        UI_ELF["Elfiee GUI<br/>(Desktop)"]
    end

    subgraph L1["Layer 1: Communication Hub"]
        MATRIX["Matrix Server<br/>(Synapse/自研)"]
        ROUTER["/cmd 路由器"]
        AGENT_HOST["Agent 宿主"]
        BRIDGE["事件桥接"]
        MATRIX --- ROUTER
        MATRIX --- AGENT_HOST
        MATRIX --- BRIDGE
    end

    subgraph L2["Layer 2: Service Backends"]
        SYN_API["Synnovator API<br/>内容 CRUD · 模板市场 · 用户画像"]
        ONE_API["One System API<br/>Secret Store · SSH Tunnel<br/>Volume Mount · Agent Runtime"]
        ELF_ENGINE["Elfiee Engine (Local)<br/>Event Sourcing · CBAC 权限<br/>Multi-Agent Orchestration<br/>Template Engine"]
    end

    subgraph L3["Layer 3: Infrastructure"]
        SVR_A["Server A"]
        SVR_B["Server B"]
        GPU["GPU Node"]
        STORE["Storage"]
    end

    UI_SYN --> MATRIX
    UI_MAT --> MATRIX
    UI_ELF --> MATRIX

    MATRIX --> SYN_API
    MATRIX --> ELF_ENGINE
    MATRIX --> ONE_API

    ONE_API --> ELF_ENGINE
    ELF_ENGINE -- "REST + SSH" --> ONE_API

    ONE_API --> SVR_A
    ONE_API --> SVR_B
    ONE_API --> GPU
    ONE_API --> STORE
```

---

## 三、Matrix 中心路由模型

Matrix 作为消息总线，不持有业务状态，只做三件事：**转发、路由、托管 Agent**。

### 3.1 /命令路由表

```mermaid
flowchart LR
    USER["用户消息"] --> ROUTER{"Matrix<br/>/cmd Router"}

    ROUTER -- "/post · /page · /community" --> SYN["Synnovator API"]
    SYN --> SYN_R["创建帖子<br/>列出页面<br/>邀请成员"]

    ROUTER -- "/task · /agent · /template" --> ELF["Elfiee MCP"]
    ELF --> ELF_R["创建任务块<br/>查询 Agent 状态<br/>发布模板"]

    ROUTER -- "/env · /secret · /deploy" --> ONE["One System API"]
    ONE --> ONE_R["列出环境<br/>设置密钥<br/>部署服务"]

    ROUTER -- "@agent-name 自然语言" --> AGT["托管 Agent"]
    AGT --> AGT_R["自主处理"]
```

### 3.2 Agent 在 Matrix 中的角色

```mermaid
flowchart TD
    subgraph ROOM["Matrix Room"]
        HUMAN["人类用户<br/>(Element/自研客户端)<br/>发送消息 · /命令 · 审批"]
        BOT_R["路由 Bot (系统)<br/>解析 /命令 → 转发到对应 API"]
        BOT_A["托管 Agent (AI)<br/>接收 @mention<br/>调用 Elfiee MCP · One System · Synnovator<br/>将结果发回聊天室"]
        BOT_W["Webhook Bot (桥接)<br/>从 Synnovator/Elfiee 接收事件 → 发到聊天室"]
    end

    HUMAN --> BOT_R
    HUMAN --> BOT_A
    BOT_W --> ROOM
```

---

## 四、核心数据流

### 4.1 全局数据流向

```mermaid
flowchart TD
    ELF["Elfiee<br/>(本地智能)"]
    MAT["Matrix<br/>(通讯中心)"]
    SYN["Synnovator<br/>(社群)"]
    ONE["One System<br/>(资源运行时)"]

    ELF -- "执行结果 · 状态通知<br/>Matrix Client SDK" --> MAT
    MAT -- "/命令 · @mention 转发<br/>MCP over SSE" --> ELF

    MAT -- "/post · /page<br/>REST API" --> SYN
    SYN -- "新帖通知 · 评论提醒<br/>Webhook" --> MAT

    MAT -- "/env · /secret · /deploy<br/>REST API" --> ONE
    ONE -- "部署状态 · 告警<br/>Webhook" --> MAT

    ELF -- "请求运行环境<br/>REST API + SSH" --> ONE
    ONE -- "运行结果 · 日志<br/>SSH + Stream" --> ELF

    ELF -. "发布模板" .-> SYN
    SYN -. "社区反馈" .-> ELF
```

### 4.2 四平台综合数据流时序图

```mermaid
sequenceDiagram
    actor User as 用户
    participant MAT as Matrix
    participant SYN as Synnovator
    participant ELF as Elfiee (Local)
    participant ONE as One System
    participant SVR as Remote Server

    Note over User, SVR: === 社群操作 (Matrix → Synnovator) ===
    User ->> MAT: /post "新功能发布公告"
    MAT ->> SYN: REST API: 创建帖子
    SYN -->> MAT: Webhook: 帖子已发布
    MAT -->> User: 帖子发布成功，链接: ...

    Note over User, SVR: === Agent 任务 (Matrix → Elfiee → One System) ===
    User ->> MAT: @dev-agent 实现用户认证
    MAT ->> ELF: MCP: 转发任务到托管 Agent
    ELF ->> ELF: 解析 .elf 模板，分配子 Agent
    ELF ->> ONE: REST: 请求 GPU 运行环境
    ONE ->> SVR: SSH: 建立隧道 + 挂载目录
    SVR -->> ONE: 环境就绪
    ONE -->> ELF: SSH 连接信息
    ELF ->> ELF: coder 编写 → reviewer 审查 → tester 测试
    ELF -->> MAT: Agent 执行完成，分支 feat/auth
    MAT -->> User: dev-agent: 完成，3 个文件已提交

    Note over User, SVR: === 审批与提交 (Matrix → Elfiee → Git) ===
    User ->> MAT: /approve feat/auth
    MAT ->> ELF: MCP: task.commit
    ELF ->> ELF: 导出代码 → git commit → git push
    ELF -->> MAT: commit hash: abc1234
    MAT -->> User: 已提交到 feat/auth (abc1234)

    Note over User, SVR: === 模板分享 (Elfiee → Matrix → Synnovator) ===
    User ->> MAT: /template share my-team.elf
    MAT ->> ELF: 读取模板文件
    ELF -->> MAT: 模板数据 + 执行指标
    MAT ->> SYN: REST: 上传模板到市场
    SYN -->> MAT: 发布成功，模板 ID: tmpl-42
    MAT -->> User: 模板已发布到 Synnovator 市场

    Note over User, SVR: === 资源管理 (Matrix → One System) ===
    User ->> MAT: /env list
    MAT ->> ONE: REST: 列出已注册环境
    ONE -->> MAT: 3 台服务器在线
    MAT -->> User: Server A (CPU) · Server B (CPU) · GPU Node
```

### 4.3 典型场景：Agent 协作开发

```mermaid
sequenceDiagram
    actor User as 用户 (Matrix)
    participant MAT as Matrix Server
    participant ELF as Elfiee (Local)
    participant ONE as One System

    User ->> MAT: @dev-agent 实现登录功能
    MAT ->> ELF: route to dev-agent

    ELF ->> ELF: 解析 .elf 模板
    Note right of ELF: 分配子 Agent:<br/>- coder (写代码)<br/>- reviewer (审查)<br/>- tester (测试)

    ELF ->> ONE: 请求运行环境
    ONE -->> ELF: 返回 SSH 连接

    ELF ->> ELF: coder 编写代码
    ELF ->> ELF: reviewer 审查
    ELF ->> ELF: tester 运行测试

    ELF -->> MAT: 结果 + 事件记录
    MAT -->> User: dev-agent: 完成。<br/>已创建分支 feat/login，3 个文件

    User ->> MAT: /approve feat/login
    MAT ->> ELF: 转发审批
    ELF ->> ELF: task.commit → git push
    ELF -->> MAT: 提交成功
    MAT -->> User: 已合并到 feat/login
```

### 4.4 典型场景：模板分享与演化

```mermaid
sequenceDiagram
    participant A as 开发者 A (Elfiee)
    participant MAT as Matrix
    participant SYN as Synnovator
    participant B as 开发者 B (Elfiee)

    Note over A: 本地模板表现好<br/>(成功率 85%)

    A ->> MAT: /template share my-team.elf
    MAT ->> SYN: 上传模板
    Note over SYN: 发布到模板市场<br/>含指标：成功率、<br/>适用场景、版本

    B ->> SYN: 浏览模板市场
    SYN -->> B: 下载 my-team.elf

    Note over B: 导入模板<br/>本地适配<br/>执行任务<br/>收集指标

    B ->> SYN: 发布改进版本 my-team-v2.elf
    Note over SYN: 版本迭代<br/>社区评分更新
```

---

## 五、Elfiee 模板系统：多 Agent 编排

这是整个架构中最核心的创新点。Elfiee 不仅是编辑器，更是 **Agent 组织的定义和演化平台**。

### 5.1 模板文件结构

`.elf` 模板文件定义一个多 Agent 组织的完整规格：

```
team-template.elf/
├── _eventstore.db              # 事件日志（含初始化事件）
├── agents/
│   ├── coordinator.md          # 协调者：任务拆分、分配、汇总
│   ├── coder.md                # 执行者：代码编写
│   ├── reviewer.md             # 审查者：代码审查
│   └── tester.md               # 验证者：测试执行
├── rules/
│   ├── workflow.md             # 工作流定义（DAG）
│   ├── permissions.md          # 权限矩阵
│   └── evolution-policy.md     # 演化策略
└── skills/
    ├── code-review.md          # 可复用 skill
    └── test-driven.md          # TDD 流程 skill
```

### 5.2 Agent 演化路径

对应图中的 Agent 0 → 1 → 2 进化路线：

```mermaid
flowchart TD
    S0["Stage 0: Local Case → Local Skill"]
    S0D["单个 Agent 本地执行任务<br/>积累经验 → 提炼为 Skill<br/>Skill 存储在 .elf 文件中"]

    S1["Stage 1: Shared Case → Shared Skill"]
    S1D["Skill 通过 Synnovator 模板市场分享<br/>其他用户导入 Skill<br/>社区反馈 → Skill 迭代优化"]

    S2["Stage 2: Organized Case → Organized Skill"]
    S2D["多个 Agent 组成团队（模板定义）<br/>团队级 Skill = 工作流 + 权限 + 演化策略"]

    EVO["模板自我演化（遗传算法）"]
    MUT["变异：调整角色/权限/工作流"]
    SEL["选择：保留成功率高的配置"]
    CRS["交叉：合并不同模板的优势"]
    PUB["最优模板通过 Synnovator 社区传播"]

    S0 --> S0D --> S1
    S1 --> S1D --> S2
    S2 --> S2D --> EVO
    EVO --> MUT
    EVO --> SEL
    EVO --> CRS
    MUT & SEL & CRS --> PUB
```

### 5.3 模板在四平台间的流转

```mermaid
flowchart LR
    ELF["Elfiee<br/>定义 · 执行 · 演化"]
    MAT["Matrix<br/>触发 · 通知 · 协调"]
    SYN["Synnovator<br/>版本 · 评分 · 讨论"]
    ONE["One System<br/>隔离运行环境<br/>Secret 注入 · 资源配额"]

    ELF -- "模板 .elf" --> MAT
    MAT -- "分享到市场" --> SYN
    SYN -- "下载/反馈" --> MAT
    MAT -- "任务/反馈" --> ELF
    ELF -- "请求运行环境" --> ONE
    ONE -- "运行结果" --> ELF
```

---

## 六、One System 的精准定位

**原则**：不做重型资源编排（交给 k8s/docker-compose），专注 Agent 运行时的三个痛点。

| 职责 | 做什么 | 不做什么 |
|------|--------|----------|
| **Secret Store** | API Key、Token、SSH Key 的统一管理 | 不做 Vault 级别的密钥轮转 |
| **SSH Tunnel** | 本地 ↔ 远程服务器的安全通道 | 不做 VPN 或网络编排 |
| **Volume Mount** | 将本地 .elf/代码目录挂载到远程 | 不做分布式文件系统 |
| **Agent Runtime** | 为 Agent 提供隔离执行环境 | 不做容器编排/调度 |

```mermaid
sequenceDiagram
    participant ELF as Elfiee (Local)
    participant ONE as One System
    participant SVR as Remote Server

    ELF ->> ONE: coder agent 需要 GPU 环境
    ONE ->> ONE: 查找已注册的 GPU Server
    ONE ->> ONE: 注入 Secret (API keys)
    ONE ->> SVR: 建立 SSH Tunnel + Mount 项目目录
    Note right of SVR: Agent 在隔离<br/>环境中执行
    SVR -->> ONE: Stream 日志和结果
    ONE -->> ELF: 运行结果
    Note left of ELF: 更新 .elf 事件记录
```

---

## 七、评估与建议

### 7.1 架构优势

1. **Matrix 作为消息总线是好的选择**：开放协议、联邦化、端到端加密、原生支持 Bot。比自建消息系统成本低且生态好。

2. **关注点分离清晰**：四个平台各司其职，没有功能重叠。社群归社群、通讯归通讯、智能归本地、资源归运行时。

3. **模板作为 Agent 组织的 "基因"**：`.elf` 文件天然适合这个角色——事件溯源提供审计，CBAC 提供权限隔离，Block 结构提供模块化。

4. **渐进式复杂度**：用户可以只用 Elfiee（纯本地），也可以接入 Matrix（协作），也可以发布到 Synnovator（社区），复杂度按需增长。

### 7.2 需要注意的风险

| 风险 | 描述 | 建议 |
|------|------|------|
| **Matrix 单点依赖** | 所有跨平台通讯都经过 Matrix，宕机影响全局 | 各平台保留直连 API（REST/gRPC），Matrix 是增强层不是必需层。Elfiee ↔ One System 的高频通讯（SSH 日志流）应走直连，不走 Matrix |
| **/命令爆炸** | Synnovator 功能多了以后 /命令数量失控 | 分层设计：高频简单操作走 /命令，复杂操作走 /open（在 Matrix 中打开 Synnovator 嵌入视图或链接） |
| **Agent 生命周期归属** | 模板定义在 Elfiee，运行时在 One System，入口在 Matrix——谁管生死？ | 明确：Elfiee 拥有 Agent **定义**（模板），Matrix 拥有 Agent **会话**（托管），One System 拥有 Agent **进程**（运行时）。生命周期由 Elfiee 模板声明，Matrix 和 One System 执行 |
| **模板版本演化** | 自我演化的模板需要版本管理和回滚 | Synnovator 模板市场做语义化版本，Elfiee 事件日志本身就是完整历史，可回溯任意时刻的模板状态 |
| **本地 ↔ 云端延迟** | Elfiee 在本地，One System/Matrix 在云端，复杂任务需要频繁通讯 | 区分实时路径和异步路径。实时：Elfiee ↔ One System SSH 直连。异步：通过 Matrix 消息队列 |

### 7.3 建议的实施优先级

```mermaid
flowchart LR
    A["Phase A<br/>本地闭环"]
    B["Phase B<br/>通讯层接入"]
    C["Phase C<br/>社群层接入"]
    D["Phase D<br/>资源层接入"]
    E["Phase E<br/>演化闭环"]

    A --> B --> C --> D --> E

    A -.- AD["Elfiee 独立运行<br/>+ Claude Code MCP"]
    B -.- BD["Matrix Bot 接入 Elfiee MCP<br/>基础 /命令路由<br/>Agent 在 Matrix 中托管"]
    C -.- CD["Synnovator API ← Matrix /命令<br/>模板市场基础版<br/>模板上传/下载"]
    D -.- DD["One System Secret + SSH<br/>Agent Runtime 隔离<br/>Elfiee 模板 → 运行环境"]
    E -.- ED["模板执行指标收集<br/>自动演化策略<br/>社区排行和推荐"]
```
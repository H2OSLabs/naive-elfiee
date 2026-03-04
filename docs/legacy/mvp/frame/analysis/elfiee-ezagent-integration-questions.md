# Elfiee → ezagent 适配：待确认问题清单

> 用于下午讨论。从底层架构到顶层策略，逐层梳理。
> 日期：2026-03-03

---

## 背景

- **Elfiee 现状**：L0-L5 重构完成，287 tests，纯 Rust（Tauri），SQLite eventstore.db，MCP SSE 通信
- **ezagent 现状**：Phase 0+1 完成，206 tests，Rust Engine + PyO3，yrs CRDT + Zenoh P2P
- **目标**：Elfiee（EventWeaver 定位）接入 ezagent 生态
- **提议**：在 Elfiee 添加 diff 语义，以 operation 为基础，支持 CRDT 后，通过简单适配和转译接入

---

## Layer 0: 存储层

### Q1: eventstore.db 保留还是替换？

**现状**：Elfiee 用 SQLite `eventstore.db` 追加日志，ezagent 用 yrs `Y.Doc` CRDT 文档。

**选项**：
- A) **保留 eventstore.db，新增 yrs 副本**：双写（eventstore.db + Y.Doc），Elfiee 保持独立运行能力，yrs 作为同步通道。
- B) **替换为 yrs**：去掉 SQLite，所有 Event 直接写入 Y.Doc。Elfiee 完全依赖 ezagent CRDT 基础设施。
- C) **eventstore.db 作为 yrs 的持久化层**：yrs updates 的序列化存储放在 SQLite 中（类似 y-leveldb 但用 SQLite），保留 SQL 查询能力。

**需要确认**：Elfiee 是否需要保持独立运行能力（不依赖 ezagent 也能用）？

### Q2: Event 的 Delta 模式用什么格式？

**现状**：Elfiee 的 `mode: Delta` 目前存的是 unified diff 字符串（文本 patch）。

**选项**：
- A) **保持 unified diff**：在 Bridge 层转换为 yrs 操作（损耗型，无法精确还原字符级 CRDT 操作）
- B) **改为 yrs 原生 delta**：Event.value 直接存 `Y.Text` 的 delta 格式 `[{insert: "..."}, {retain: N}, {delete: N}]`（与 CRDT 原生对齐）
- C) **改为 Operation-based 中间格式**：定义一个 Elfiee 自己的 Operation schema（如 OT 的 insert/delete/retain），可双向转换为 yrs delta 和 unified diff

**需要确认**：Delta 精度要求——字符级（适合协作编辑）还是行级（适合代码文件记录）？

### Q3: Elfiee 的 Snapshot/CacheStore 机制在 CRDT 下还需要吗？

**现状**：Elfiee 用 `~/.elf/cache/{project-hash}/cache.db` 存 per-block 快照，加速 replay。

**分析**：
- yrs 天然支持 state snapshot（`encode_state_as_update_v1`），不需要 Elfiee 自己维护快照
- 但如果保留 eventstore.db（Q1-A），则 CacheStore 仍有价值

**需要确认**：取决于 Q1 的选择。

---

## Layer 1: 身份与密钥

### Q4: Editor ID 格式统一还是映射？

**现状**：
- Elfiee: UUID editor_id（如 `550e8400-e29b-41d4-a716-446655440000`）
- ezagent: Entity ID `@{local_part}:{relay_domain}`（如 `@alice:relay.ezagent.dev`）+ Ed25519 keypair

**选项**：
- A) **映射表**：Elfiee 内部继续用 UUID，Bridge 层维护 UUID ↔ Entity ID 映射
- B) **Elfiee 原生采用 Entity ID 格式**：改 Editor 模型，editor_id 改为 `@local:relay` 格式
- C) **Entity ID 存入 Editor metadata**：editor_id 保持 UUID，但 metadata 中记录对应的 ezagent Entity ID

**需要确认**：Elfiee 是否需要独立于 ezagent 的身份体系？（影响离线场景）

### Q5: 签名与验证

**现状**：
- Elfiee: 无签名，信任 MCP 连接级认证（`elfiee_auth`）
- ezagent: 每个消息都有 Ed25519 签名（Signed Envelope），所有操作不可伪造

**选项**：
- A) **Elfiee 不加签名**：Bridge 层在转译时补签名（Bridge 持有 Elfiee 的代理密钥）
- B) **Elfiee 原生加签名**：每个 Event 添加 Ed25519 签名字段，Engine 处理时验证
- C) **分层签名**：Elfiee 内部不签名（保持现有效率），对外同步到 ezagent 时由 Bridge 签名

**需要确认**：安全级别要求——信任本地进程（现状）还是需要密码学不可伪造？

---

## Layer 2: 并发模型

### Q6: OCC 与 CRDT 如何共存？

**现状**：
- Elfiee: Actor 串行 + OCC（Vector Clock 冲突检测，拒绝 + re-base）
- ezagent: CRDT 自动合并（无冲突，最终一致）

**关键矛盾**：Elfiee 的 OCC 会拒绝某些并发写入，但 CRDT 不拒绝——它总是合并。如果 Elfiee 拒绝了一个 Command，但 CRDT 侧已经合并了对应的 update，两边状态就不一致。

**选项**：
- A) **Elfiee 作为唯一写入者**：所有写入必须经过 Elfiee Engine（OCC 生效），yrs 只做同步传输（不接受来自其他 peer 的直接写入）
- B) **放弃 OCC，采用 CRDT 语义**：改 Elfiee 的冲突处理为 CRDT 自动合并（重大改动）
- C) **OCC 在前，CRDT 在后**：Elfiee 先做 OCC 检查（保证业务语义正确），通过后再写入 yrs（CRDT 只负责同步，不做业务决策）

**需要确认**：是否存在"多个 Elfiee 实例对同一个 .elf 项目同时写入"的场景？如果不存在，Q6-A 是最简方案。

---

## Layer 3: 通信协议

### Q7: Elfiee 的两层协议架构

**约束（非选项）**：Agent（Claude Code / OpenClaw）只会说 MCP 协议。Zenoh 是节点间 P2P 同步协议，Agent 不会用 Zenoh。因此架构一定是两层：

```
Agent (Claude Code)                    其他 ezagent peer
  │                                        │
  │ MCP SSE（Agent 唯一能说的协议）          │ Zenoh P2P（节点间同步）
  ▼                                        ▼
Elfiee ◄──────── Zenoh / Bridge ────────► ezagent Bus
```

- **对 Agent 的接口**：MCP SSE 永远保留，这是约束不是选项
- **对 ezagent 的接口**：新增 Zenoh 同步能力

**真正的问题是**：Zenoh 同步层放在哪里？

- A) **Elfiee 进程内嵌 Zenoh peer**：`elf serve` 既是 MCP Server 又是 Zenoh peer，一个进程搞定
- B) **独立 Bridge 进程**：Bridge 进程分别连接 Elfiee（MCP SSE）和 ezagent（Zenoh），解耦部署
- C) **ezagent peer 监听 Elfiee 的 eventstore.db**：不改 Elfiee，由 ezagent 侧的 adapter 主动拉取 Elfiee 的 Event

**需要确认**：部署偏好——一个进程（简单）还是分离进程（灵活）？

### Q8: Elfiee → ezagent 的同步频率

**背景**：这里说的是 Elfiee 产生的 Event 多快同步到 ezagent 侧（通过 Zenoh），跟 Agent 通信无关（Agent 始终通过 MCP SSE 实时操作 Elfiee）。

**选项**：
- A) **实时**：每个 Elfiee Event 立即转译并通过 Zenoh 推送到 ezagent
- B) **批量**：积累一批 Event 后一次性同步（如每 10 秒或每 N 个 Event）
- C) **混合**：结构事件（create/delete/grant）实时，内容事件（document.write）批量

**需要确认**：ezagent 侧的消费者（Observer/其他 Socialware）对 Elfiee 变更的实时性要求是什么？

---

## Layer 4: 数据模型映射

### Q9: Block 是否保留为独立概念？

**现状**：
- Elfiee: Block 是一等实体（有 block_id、contents、children、owner）
- ezagent: 零浮空概念原则——所有领域概念都是具有特定 content_type 的 Message

**选项**：
- A) **Block → Message 映射**：Block 的创建/修改都表达为特定 content_type 的 Message（如 `ew:doc.create`, `ew:doc.write`）。Block 的"当前状态"由 State Cache 从 Message 序列派生。
- B) **Block 保留为 Elfiee 内部概念**：ezagent 侧只看到 Event 流（`ew:event.record`），不感知 Block 结构
- C) **Block → Y.Map 文档**：每个 Block 对应一个独立的 yrs Y.Map 文档（保持 Block 的实体性）

**需要确认**：ezagent 侧的消费者（Observer/其他 Socialware）需要理解 Block 结构吗？还是只需要事件流？

### Q10: Task Block 交给 TaskArena 还是保留？

**现状**：
- Elfiee: task.write, task.commit, task.read — 自己管任务
- ezagent: TaskArena Socialware — 专门的任务管理 Socialware

**选项**：
- A) **保留**：Elfiee 继续自己管 Task，同步到 ezagent 时作为 `ew:event.record` 事件
- B) **迁移**：Elfiee 的 Task 功能交给 TaskArena，Elfiee 只做 document + session + event DAG
- C) **渐进**：Phase 1 保留，Phase 2 迁移到 TaskArena

**需要确认**：Elfiee 的 Task 功能和 TaskArena 有多大功能重叠？是否值得合并？

### Q11: DAG 关系类型扩展

**现状**：Elfiee 仅支持 `implement` 关系。EventWeaver PRD 定义了 `causality`（因果链）。

**问题**：
- Elfiee 的 `implement`（因果关系：A 导致 B）和 EventWeaver 的 `causality`（因果前驱链）语义是否等价？
- 是否需要新增关系类型？

**需要确认**：DAG 关系类型在 ezagent 侧如何表达——用 CRDT 的 `ext.ew.causality` annotation 还是 Elfiee 的 `children` 字段？

---

## Layer 5: 权限模型

### Q12: CBAC 与 Role 的对接方式

**现状**：
- Elfiee: GrantsTable（editor_id + cap_id + block_id），由 grant/revoke Event 投影
- ezagent: Role（capability 集合）+ Arena（边界）+ Hook Pipeline（pre_send 检查）

**选项**：
- A) **CBAC 保留为 Elfiee 内部权限**：ezagent 侧用 Role 表达等价权限，Bridge 层做映射
- B) **CBAC 替换为 Role**：重写 Elfiee 的权限模型为 Role-based
- C) **CBAC 作为 Role 的细粒度实现**：Role 是粗粒度（角色级），CBAC 是细粒度（per-block 级），两者互补

**需要确认**：per-block 细粒度权限在 ezagent Role 体系中如何表达？（ezagent 的 Arena 能否做到 per-Message 级权限？）

---

## Layer 6: 应用层

### Q13: CLI 工具如何演进？

**现状**：
- Elfiee: `elf init/register/serve/run/scan/status/grant/revoke/block`
- ezagent: Phase 3 计划 CLI，尚未实现

**选项**：
- A) **保留 `elf` CLI**：独立于 ezagent CLI，通过 Bridge 同步
- B) **合并为 `ezagent` CLI 的子命令**：如 `ezagent ew init`, `ezagent ew serve`
- C) **`elf` CLI 调用 ezagent SDK**：保持 `elf` 命令入口，底层通过 PyO3 调用 ezagent Engine

**需要确认**：用户体验上是保持独立工具还是统一入口？

### Q14: Skill/Workflow 模板如何映射到 Socialware 声明？

**现状**：
- Elfiee: `.elf/templates/workflows/*.toml`（参与者 + 权限矩阵 + 任务）
- ezagent: Socialware 声明格式（Part A + Part B + Part C）

**问题**：Elfiee 的 Workflow 模板和 Socialware 声明在概念上高度相似（都是"角色 + 权限 + 流程"），但格式完全不同。

**需要确认**：是否需要双格式支持（过渡期），还是直接迁移到 Socialware 声明格式？

### Q15: Tauri 桌面应用的归属

**现状**：
- Elfiee: Tauri 2 桌面应用（Rust + React）
- ezagent: `app/` 子项目（React 桌面应用，计划在 Phase 4）

**选项**：
- A) **Elfiee 桌面保持独立**：作为 EventWeaver 的专用客户端
- B) **合并到 `app/`**：Elfiee 的前端代码移入 ezagent 的 `app/` 子项目
- C) **Elfiee 桌面嵌入 ezagent**：Elfiee Tauri 应用内嵌 ezagent peer，作为 ezagent 的桌面入口之一

**需要确认**：长期是一个桌面应用（ezagent app 统一入口）还是多个桌面应用？

---

## 战略层

### Q16: 适配顺序（Phase 划分）

**提议的渐进路径**：

| Phase | 内容 | 目标 |
|-------|------|------|
| **Phase A: 最小桥接** | Elfiee Event → yrs Update 单向转译 | 验证概念可行性 |
| **Phase B: Delta 增强** | Event.value 改为 Operation-based 格式 | CRDT 原生支持 |
| **Phase C: 身份对接** | Editor ID ↔ Entity ID 映射 | 统一身份体系 |
| **Phase D: 实时同步** | Zenoh 传输层集成 | P2P 同步能力 |
| **Phase E: 完整融合** | CBAC → Role，Task → TaskArena | 成为真正的 Socialware |

**需要确认**：这个顺序合理吗？是否有某些步骤可以跳过或合并？

### Q17: 两个项目的维护关系

**选项**：
- A) **Elfiee 保持独立仓库**：通过 Bridge 与 ezagent 交互，独立发布和版本
- B) **Elfiee 移入 monorepo**：成为 `monorepo/elfiee/` 子项目，与 ezagent 统一管理
- C) **Elfiee Core 拆分**：Engine 部分移入 monorepo（作为 EventWeaver 的 Rust 实现），Tauri 桌面部分保持独立

**需要确认**：代码组织和团队协作的偏好？

### Q18: MVP 目标定义

**问题**：适配的第一个可用版本（MVP）应该做到什么程度？

**候选 MVP 定义**：
- A) "Elfiee 的 Event 能出现在 ezagent 的 Timeline 中"（最小验证）
- B) "ezagent Observer 能查询 Elfiee 产生的 Event DAG"（有实际价值）
- C) "Agent 可以通过 ezagent 协议操作 Elfiee 的 Block"（功能完整）

**需要确认**：第一个里程碑想达到什么效果？

---

## 总结：18 个问题按优先级分组

### 必须先确认（阻塞设计方向）
1. **Q1** — eventstore.db 保留还是替换？
2. **Q6** — OCC 与 CRDT 如何共存？
3. **Q16** — 适配顺序
4. **Q18** — MVP 目标定义

### 尽早确认（影响实现方案）
5. **Q2** — Delta 格式
6. **Q4** — Editor ID 格式
7. **Q7** — Zenoh 同步层的部署位置（MCP SSE 对 Agent 保留是约束）
8. **Q9** — Block 概念保留还是映射
9. **Q17** — 仓库维护关系

### 可以延后（不影响第一步）
10. **Q3** — Snapshot 机制
11. **Q5** — 签名验证
12. **Q8** — 同步实时/批量
13. **Q10** — Task 功能归属
14. **Q11** — DAG 关系类型
15. **Q12** — CBAC vs Role 对接
16. **Q13** — CLI 演进
17. **Q14** — Workflow → Socialware 声明
18. **Q15** — 桌面应用归属

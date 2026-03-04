# Event Sourcing 上下文优化架构

> 日期: 2026-01-30
> 阶段: Phase 3+ 算法优化方向

## 1. 核心设想

### 传统多轮对话 vs One-shot

```
传统多轮对话:
  Agent 读状态 → 尝试修改 → 失败 → 读错误 → 再改 → ... → 成功
  （多轮消耗 token，上下文膨胀，每轮都可能偏移）

Event Sourcing One-shot:
  Event Chain ──压缩──► CBAC 上下文沙箱 ──一次性推送──► Agent ──one-shot──► 正确修改
```

### 关键洞察

Event Sourcing 不仅记录了"是什么"（当前状态），还记录了"为什么"（因果链）和"谁做的"（editor 归属）。传统文件系统只给 Agent 看当前快照（没有因果链），Agent 被迫通过多轮试错来补全缺失的上下文。完整的 event chain 本身就是最好的上下文。

### One-shot 成立的前提条件

1. **Event chain 提供充分的因果信息** — Event Sourcing 天然满足
2. **CBAC 沙箱限定修改范围** — 减少 Agent 的决策空间，降低出错概率
3. **压缩算法保留关键信息，丢弃噪声** — 算法优化的核心难点

## 2. 信息漏斗：从全量 Events 到 Agent 上下文

```
全量 Events（完整但巨大）
    ↓ ① Relation Graph 选择因果相关
相关 Events（几百条）
    ↓ ② 压缩算法提取关键变迁
结构化摘要（几段文字）
    ↓ CBAC 过滤（已有，自动生效）
Agent 上下文（精准、完整、有界）
    ↓
One-shot 修改
```

**注意**: CBAC 过滤不是独立的层。现有的 `get_all_events()` / `get_block()` / `get_all_blocks()` 查询接口已内置 CBAC 权限过滤（详见 cbac-read-filtering-status.md）。只要数据通过现有接口获取，CBAC 自动生效。

## 3. 两层优化架构

### 层 ①：事件选择（Event Selection）— Engine 层

**问题**: 一个 .elf 文件可能有上万条 Events，全推给 Agent 不现实。

**做什么**: 给定一个 Task（或意图），从 Event Store 中选出因果相关的 Events 子集。

```
输入: Task "修改登录逻辑" + Relation Graph（implement 链）
  ↓
  1. 从 Task Block 出发，沿 implement 关系找到关联的 Code/Markdown Blocks
  2. 按 block_id 过滤 Events → 只保留相关 Block 的修改历史
  3. 按时间窗口裁剪（如只看最近 N 个 transaction）
  ↓
输出: Event 子集（几十条而非上万条）
```

**放在哪里**: `engine/context_projector.rs`（新模块），与现有 `StateProjector` 平行。
- StateProjector 投影"当前状态"
- ContextProjector 投影"因果上下文"

### 层 ②：上下文压缩（Context Compression）— 新 context/ 模块

**问题**: 即使筛选后的 Events 也可能很长。同一个 Block 被改了 20 次，Agent 不需要看 20 个 diff。

**做什么**: 把 Event 序列压缩成结构化摘要。

```
原始（20 条 Events）:
  e1: alice/markdown.write block-abc "# Login\n初始版本"
  e2: alice/markdown.write block-abc "# Login\n添加了 OAuth"
  ...
  e20: bob/markdown.write block-abc "# Login\n修复了 token 过期"

压缩后:
  Block block-abc (markdown, "Login")
  - 创建者: alice, 当前修改者: bob
  - 变迁摘要: 初始版本 → 添加 OAuth → 修复 token 过期
  - 当前内容: "# Login\n修复了 token 过期"
  - 权限: alice(owner), bob(markdown.write)
```

**可插拔压缩策略**:

| 策略 | 适用场景 | 复杂度 |
|---|---|---|
| 最新状态 + 变更计数 | 简单修改 | 低 |
| 关键节点摘要（create + 每个 editor 首次修改 + 最新） | 多人协作 | 中 |
| LLM 自摘要（用小模型压缩 event chain） | 复杂因果链 | 高 |
| 向量相似度选择（embedding events，选与 task 最相关的） | 大规模 event store | 高 |

## 4. 代码架构位置

```
src-tauri/src/
├── engine/
│   ├── state.rs              # ✅ 现有: StateProjector（投影当前状态）
│   ├── context_projector.rs  # 🔲 新增: ContextProjector（投影因果上下文）
│   │                         #     ← 层①事件选择
│   └── event_store.rs        # ✅ 现有: Event 存储
│
├── context/                  # 🔲 新增模块: 上下文优化
│   ├── mod.rs
│   ├── compressor.rs         # ← 层②压缩算法
│   ├── strategies/           # ← 可插拔的压缩策略
│   │   ├── latest_state.rs   #    最简策略
│   │   ├── key_events.rs     #    关键节点策略
│   │   └── semantic.rs       #    语义压缩（LLM 辅助）
│   └── scope.rs              # ← CBAC 辅助（仅在绕过查询接口时需要）
│
├── mcp/
│   └── server.rs             # 修改: 新增 context Resource
│                             #   elfiee://{project}/context/{task_id}
│                             #   返回编译好的 one-shot 上下文
│
├── extensions/
│   └── task/                 # Task Block 是上下文构建的起点
│       └── task_commit.rs
```

## 5. MCP 出口

新增 MCP Resource:

```
elfiee://{project}/context/{task_block_id}
```

返回为特定 Task 编译好的完整上下文（经过事件选择 + 压缩 + CBAC 过滤），Agent 读一次就够。

## 6. 实施建议

- **Phase 2**: 先用最简策略（最新状态 + CBAC 过滤）跑通管道
- **Phase 3**: 插入更复杂的压缩算法，用 Dogfooding 指标（P-METRIC）量化优化效果
- **管道架构**（context/ 模块 + 可插拔 strategies）现在就应该设计好，即使初始策略简单

## 7. 可量化指标

漏斗每一级都可独立优化，效果可量化：
- **事件选择精度**: 选出的 Events 中与 Task 真正相关的比例
- **压缩保真度**: 压缩后上下文 vs 全量上下文，Agent 修改正确率的差异
- **one-shot 成功率**: 给定压缩上下文后，Agent 一次修改正确的比例
- **token 效率**: one-shot 上下文 token 数 vs 多轮对话总 token 数

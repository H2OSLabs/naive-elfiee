---
name: elfiee-be-refactor
description: >
  Elfiee 后端重构 skill。将 Elfiee 从全能桌面编辑器收束为编排型 Agent，
  通过删除 I/O 代码、合并 Extension、替换通讯层，实现后端能力的完整封装。
  使用场景：(1) 执行重构步骤（删除/合并/新增代码）；
  (2) 验证重构结果（API 覆盖度、无用代码扫描、依赖清理）；
  (3) 处理产品 user-story testcase 到 API 的映射验证。
  触发示例："执行重构步骤 X"、"验证 API 覆盖"、"清理无用代码"、
  "检查 user-story 是否可通过 API 组合实现"。
---

# Elfiee 后端重构

## 硬性规则

### R1: block_type 不是枚举

`block_type` 是 `String`，由 Extension 注册决定。不要创建 `BlockType` 枚举。新增 block type 通过 Extension 注册流程（elfiee-ext-gen）添加，核心类型（document/agent/task/session）也是通过 Extension 注册的。

### R2: 不做向后兼容

删除旧代码，不添加兼容层。具体：
- 不写 `from_str_compat` / legacy mapping 函数
- 不写 ALTER TABLE 迁移旧 schema
- 不写 `#[serde(default)]` 容错旧数据
- 不写 `unwrap_or_else` fallback 旧格式
- 旧测试直接删除，不修改为兼容新旧两种格式

### R3: 测试跟着需求走

测试用例反映当前需求，不为兼容牺牲：
- 测试失败 → 考虑是否符合新需求 → 符合则修改测试，不符合则删除
- 不写 "legacy compat" 类测试
- 测试数据使用新的类型名和格式

### R4: 每步写 changelog

每个 concepts/ 文档对应的重构完成后，在 `docs/mvp/frame/changelogs/` 下写 checklist 格式的变更记录。文件名与 concepts/ 下对应文件名一致。

## 重构目标

将 Elfiee 后端从"全能桌面编辑器"收束为"编排型 Agent 内核"：

- **产品同事提供** user-story + testcase（自然语言）
- **重构目标**：后端 API 完整封装，可通过 API 组合满足所有 testcase
- **不做**：前端 UI 改动。只做后端能力可调用、可验证、可独立测试

## 工作流

```
接到重构任务
    │
    ├── 是删除/清理类？
    │   → 读 references/refactor-scope.md 确认范围
    │   → 执行删除
    │   → 运行 scripts/scan_unused_code.sh 验证
    │   → 运行 scripts/scan_unused_deps.sh 检查依赖
    │
    ├── 是新增/改造类？
    │   → 读 references/target-api-surface.md 确认目标 API
    │   → 读 references/architecture-invariants.md 确认约束
    │   → 实现代码
    │   → 运行 scripts/verify_api_coverage.sh 验证
    │
    └── 是 user-story 验证类？
        → 读 references/target-api-surface.md §八（映射示例）
        → 将 user-story 分解为 API 调用序列
        → 检查每个 API 是否已实现
        → 缺失的 API 标记为新增任务
```

## 重构边界

### 可以修改的

| 类别 | 范围 | 参考 |
|------|------|------|
| **删除** | directory extension, terminal extension, MCP SSE transport, agent MCP config, task Git integration, file/checkout commands | `references/refactor-scope.md` §一 |
| **合并** | markdown + code → document extension | `references/refactor-scope.md` §三 |
| **精简** | agent commands (移除 MCP 副作用), task commands (移除 Git) | `references/refactor-scope.md` §三 |
| **新增** | document extension, session extension, WebSocket adapter, message router, checkpoint | `references/refactor-scope.md` §四 |
| **改造** | models (block_type 4种, event mode), engine (StateProjector), MCP server tools | `references/refactor-scope.md` §三 |

### 不能修改的

| 约束 | 内容 | 参考 |
|------|------|------|
| **Event Sourcing** | EAVT 模型、append-only、Handler 纯函数 | `references/architecture-invariants.md` §一 |
| **Actor 模型** | 一 .elf 一 Actor、串行处理、tokio mpsc | `references/architecture-invariants.md` §二 |
| **CBAC** | 授权检查顺序、Grant 三元组、删除 Editor 拒绝 | `references/architecture-invariants.md` §三 |
| **Capability 宏** | `#[capability(id, target)]` 注册方式 | `references/architecture-invariants.md` §四 |
| **Payload 类型安全** | typed struct + specta export，禁止手动 JSON | `references/architecture-invariants.md` §四 |
| **bindings.ts** | 禁止手动编辑，tauri-specta 自动生成 | `references/architecture-invariants.md` §六 |

## 迁移顺序

按 Layer 依赖顺序执行，每步可独立验证：

| Step | 层级 | 内容 | 验证标准 |
|------|------|------|---------|
| 1 | L0 数据模型 | block_type 保持 String（由 Extension 注册）, Event mode 字段 | `cargo test` models 模块通过 |
| 2 | L1 Event 系统 | mode 处理, snapshots 表 | event_store 测试通过 |
| 3 | L2 .elf/ 格式 | ZIP → 目录 | .elf/ 初始化测试通过 |
| 4 | L3 Engine 适配 | StateProjector, Manager 收束 | 完整 command 流程测试通过 |
| 5 | L4 Extension 重组 | 删 directory/terminal, 合并 document, 新增 session | extension 单元测试通过 |
| 6 | L4 通讯层 | 删 MCP SSE, 新增 WebSocket + Router | 连接+路由测试通过 |
| 7 | — Commands 清理 | 移除 file/checkout, 精简 agent/task | 集成测试通过 |

**每步完成后必须运行：**
```bash
cd src-tauri && cargo test
scripts/scan_unused_code.sh
scripts/verify_api_coverage.sh
```

## 验证准则

### 编译级

- `cargo check` 零错误
- `cargo clippy` 零 warning（`--deny warnings`）
- `cargo test` 全部通过

### 代码级

- Handler 中无 `std::fs`/`std::process`/`std::net`（纯函数约束）
- 无已删除模块的残留引用（`scripts/scan_unused_code.sh`）
- 无未使用的 crate 依赖（`scripts/scan_unused_deps.sh`）
- 所有 Capability 在 Registry 注册（`scripts/verify_api_coverage.sh`）
- 所有 Payload 在 specta export 注册

### API 级

- 每个 Capability 有至少 1 个单元测试
- 每个 user-story 可分解为已实现的 API 调用序列
- 所有 API 可通过 Message Router 独立调用

### 安全级

- CBAC 授权检查顺序不变
- 删除的 Editor 无法操作（`deleted_editors` 检查）
- Block.children DAG 无环

## 数据清理流程

重构中遇到存量代码时的清理步骤：

1. **深度扫描**：运行 `scripts/scan_unused_code.sh` 识别残留引用
2. **依赖清理**：运行 `scripts/scan_unused_deps.sh` 识别可移除的 crate
3. **模块清理**：删除 `extensions/mod.rs`、`commands/mod.rs`、`lib.rs` 中的过时注册
4. **测试清理**：删除依赖已删除模块的测试文件
5. **编译验证**：`cargo check` 确认无 broken import
6. **测试验证**：`cargo test` 确认无 broken test

## 参考文档

| 文档 | 内容 | 何时读 |
|------|------|--------|
| [refactor-scope.md](references/refactor-scope.md) | 每个文件的删除/保留/改造清单，精确到行数 | 执行删除/改造前 |
| [target-api-surface.md](references/target-api-surface.md) | 重构后的完整 API 定义、消息格式、user-story 映射 | 新增 API 或验证 user-story 时 |
| [architecture-invariants.md](references/architecture-invariants.md) | 禁止修改的核心约束列表 | 改造核心代码前 |

## 验证脚本

| 脚本 | 用途 | 运行时机 |
|------|------|---------|
| `scripts/scan_unused_code.sh` | 扫描残留引用、Handler I/O 违规、dead_code | 每次删除操作后 |
| `scripts/scan_unused_deps.sh` | 扫描可移除的 Cargo 依赖 | Step 5/6 完成后 |
| `scripts/verify_api_coverage.sh` | 验证 Capability 注册、测试覆盖、编译 | 每步完成后 |

所有脚本接受项目根目录作为参数：`./scripts/scan_unused_code.sh /path/to/elfiee`

## 概念文档索引

重构设计文档位于 `docs/mvp/frame/concepts/`，按 Layer 0→6 排列：

| Layer | 文档 | 核心内容 |
|-------|------|---------|
| L0 | `data-model.md` | 6 个实体定义（Block/Event/Editor/Command/Grant/Capability） |
| L1 | `event-system.md` | Event mode (full/delta/ref/append)，snapshots |
| L1 | `cbac.md` | 三层授权：editor存在 → system → owner → grant |
| L2 | `elf-format.md` | .elf/ 目录格式（取代 ZIP） |
| L3 | `engine.md` | Actor 模型、StateProjector、冲突处理 |
| L4 | `extension-system.md` | 内核 vs 扩展边界、Handler 纯函数约束 |
| L4 | `communication.md` | Message Router、Tauri IPC + WebSocket 双适配 |
| L5 | `literate-programming.md` | MyST directive 叙事渲染 |
| L5 | `agent-building.md` | Agent/Task/Session 协作、模板系统 |
| L6 | `dogfooding-support.md` | Phase 2 度量指标映射 |
| L6 | `migration.md` | 删除/保留/改造清单总览 |

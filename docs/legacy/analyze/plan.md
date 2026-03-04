# Elfiee 未来工作计划

> 基于 docs/plans/ 目录下文档的未来规划总结

## 目录

- [当前状态](#当前状态)
- [已推迟功能（Post-MVP）](#已推迟功能post-mvp)
- [优先级分级](#优先级分级)
- [长期规划](#长期规划)
- [提案和研究](#提案和研究)

---

## 当前状态

### MVP 完成度

**状态**：✅ 100% 完成（6 个部分 + 后 MVP 增强）
**完成时间**：2025-10-28
**测试覆盖**：60 个测试，100% 通过

| 部分 | 状态 | 提交哈希 |
|------|------|----------|
| Part 1: 核心数据模型 | ✅ | `69b491e` |
| Part 2: 事件存储 | ✅ | `652d649` |
| Part 3: .elf 文件格式 | ✅ | `b7eb88a` |
| Part 4: 扩展接口与 CBAC | ✅ | `6d24d3c` |
| Part 5: Elfile 引擎 | ✅ | `f46db44` |
| Part 6: Tauri 应用 | ✅ | 2025-10-24 |
| 后 MVP: 强类型 Payload | ✅ | 2025-10-28 |

### 核心功能已实现

✅ **后端**：
- 事件溯源系统（EAVT 模式）
- Actor 模型并发引擎
- 能力系统和 CBAC 授权
- SQLite 异步持久化（sqlx）
- ZIP 归档文件格式
- 强类型 Payload 系统

✅ **前端**：
- React + TypeScript UI
- Tauri Specta v2 自动类型生成
- Zustand 状态管理
- Shadcn UI 组件库
- 文件和区块 CRUD 操作

---

## 已推迟功能（Post-MVP）

### 1. 性能优化

#### Snapshot 缓存系统
**状态**：❌ 未实现（已推迟）
**优先级**：⭐⭐⭐⭐

**目标**：
- 加速大文件加载速度
- 提供快速预览功能
- 避免每次打开文件都重放所有事件

**实现方案**：
```
example.elf/
├── events.db              # 事件存储（唯一真相源）
├── _snapshot              # 缓存的 Markdown 预览 ← 待实现
├── _blocks_hash           # 区块哈希缓存 ← 待实现
└── _blocks_relation       # 关系图缓存 ← 待实现
```

**收益**：
- 大文件（10,000+ 事件）加载时间从秒级降至毫秒级
- 无需修改核心架构，纯加法优化

**工作量估算**：2-3 天

---

#### 批量命令处理
**状态**：❌ 未实现
**优先级**：⭐⭐⭐

**问题**：
- 当前每个命令都单独处理和持久化
- 导入大量数据时性能受限

**实现方案**：
```rust
// 扩展 Actor 消息类型
pub enum EngineMessage {
    BatchCommand {
        cmds: Vec<Command>,
        reply: oneshot::Sender<Result<Vec<Event>, String>>,
    },
    // ...
}

// 批量处理
async fn handle_batch_command(&mut self, cmds: Vec<Command>) -> Result<Vec<Event>, String> {
    let mut all_events = Vec::new();

    for cmd in cmds {
        // 1. 授权 + 执行（内存）
        let events = self.process_single_command(cmd)?;
        all_events.extend(events);

        // 2. 应用到状态（内存）
        self.state.replay(&all_events);
    }

    // 3. 一次性批量持久化
    append_events(&self.event_pool, &all_events).await?;

    Ok(all_events)
}
```

**收益**：
- 导入性能提升 5-10 倍
- 减少数据库 I/O 次数

**工作量估算**：3-4 天

---

#### 增量 ZIP 更新
**状态**：❌ 未实现
**优先级**：⭐⭐

**问题**：
- 当前每次保存都重新打包完整 ZIP
- 大文件保存时间较长

**实现方案**：
- 使用 `zip` crate 的 append 模式
- 仅追加新事件到 `events.db`
- 周期性触发完整重新打包（碎片整理）

**收益**：
- 保存时间从 O(n) 降至 O(1)
- 适用于频繁保存场景

**工作量估算**：5-7 天

---

### 2. 高级功能

#### 完整向量时钟冲突检测
**状态**：⚠️ MVP 简化版已实现
**优先级**：⭐⭐⭐⭐

**当前实现**：
```rust
// MVP 版本：仅检查命令编辑者计数
let expected_count = self.state.editor_counts.get(&cmd.editor_id).copied().unwrap_or(0);
let cmd_count = cmd.timestamp.get(&cmd.editor_id).copied().unwrap_or(0);

if cmd_count != expected_count + 1 {
    return Err("Stale command".into());
}
```

**完整版本方案**：
```rust
// 检查所有编辑者的向量时钟
for (editor_id, cmd_count) in &cmd.timestamp {
    let state_count = self.state.editor_counts.get(editor_id).copied().unwrap_or(0);

    if *cmd_count < state_count {
        return Err(ConflictError::Stale);
    } else if *cmd_count > state_count + 1 {
        return Err(ConflictError::Future);
    }
}
```

**收益**：
- 更强的并发冲突检测
- 支持真正的多编辑者协作
- 为实时协作打基础

**工作量估算**：2-3 天

---

#### 实时协作（事件广播）
**状态**：❌ 未实现
**优先级**：⭐⭐⭐

**目标**：
- 多个用户编辑同一文件时实时同步
- 利用 Tauri 事件系统广播状态变更

**实现方案**：
```rust
// Actor 提交事件后广播
async fn handle_command(&mut self, cmd: Command) -> Result<Vec<Event>, String> {
    // ... 处理命令 ...

    // 持久化
    append_events(&self.event_pool, &events).await?;

    // 应用到状态
    self.state.replay(&events);

    // 广播事件（新增）
    self.emit_events(&events).await?;

    Ok(events)
}

// Tauri 端监听
listen('state_changed', (event) => {
    // 更新 UI
    useAppStore.getState().addEvents(event.payload.events)
})
```

**收益**：
- 实时协作基础设施
- 更好的用户体验

**工作量估算**：1 周

---

#### 撤销/重做
**状态**：❌ 未实现
**优先级**：⭐⭐⭐

**优势**：
- 事件溯源天然支持撤销/重做
- 不需要额外存储

**实现方案**：
```rust
// 在 StateProjector 中添加
pub struct StateProjector {
    pub blocks: HashMap<String, Block>,
    pub current_index: usize,  // 当前事件索引（新增）
    all_events: Vec<Event>,    // 所有事件（新增）
}

impl StateProjector {
    pub fn undo(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.rebuild_state();
        }
    }

    pub fn redo(&mut self) {
        if self.current_index < self.all_events.len() {
            self.current_index += 1;
            self.rebuild_state();
        }
    }

    fn rebuild_state(&mut self) {
        self.blocks.clear();
        self.replay(&self.all_events[..self.current_index]);
    }
}
```

**收益**：
- 用户友好的编辑体验
- 利用事件溯源优势

**工作量估算**：3-4 天

---

### 3. UI 增强

#### 多文件标签页界面
**状态**：❌ 未实现（基础架构已支持）
**优先级**：⭐⭐⭐⭐

**当前状态**：
- 后端 `EngineManager` 已支持多文件
- 前端单文件 UI

**实现方案**：
```typescript
// src/App.tsx
function App() {
  const { openFiles, currentFileId } = useAppStore()

  return (
    <div>
      <Tabs value={currentFileId}>
        <TabsList>
          {openFiles.map(file => (
            <TabsTrigger key={file.id} value={file.id}>
              {file.name}
            </TabsTrigger>
          ))}
        </TabsList>

        {openFiles.map(file => (
          <TabsContent key={file.id} value={file.id}>
            <BlockList fileId={file.id} />
          </TabsContent>
        ))}
      </Tabs>
    </div>
  )
}
```

**收益**：
- 多文件并行编辑
- 更好的工作流

**工作量估算**：2-3 天

---

#### 图形化区块关系可视化
**状态**：❌ 未实现
**优先级**：⭐⭐⭐

**目标**：
- 可视化 `Block.children` 关系图
- 交互式拖拽查看

**技术选型**：
- [react-flow](https://reactflow.dev/) - 节点图编辑器
- [cytoscape.js](https://js.cytoscape.org/) - 图可视化库

**实现示例**：
```typescript
import ReactFlow, { Node, Edge } from 'reactflow'

function BlockGraph() {
  const blocks = useAppStore(state => state.blocks)

  // 转换为 react-flow 格式
  const nodes: Node[] = blocks.map(block => ({
    id: block.blockId,
    data: { label: block.name },
    position: { x: 0, y: 0 }, // 自动布局
  }))

  const edges: Edge[] = blocks.flatMap(block =>
    Object.entries(block.children).flatMap(([relation, targets]) =>
      targets.map(target => ({
        id: `${block.blockId}-${target}`,
        source: block.blockId,
        target: target,
        label: relation,
      }))
    )
  )

  return <ReactFlow nodes={nodes} edges={edges} />
}
```

**收益**：
- 知识图谱可视化
- 更直观的区块关系管理

**工作量估算**：1 周

---

#### 拖放式区块链接
**状态**：❌ 未实现
**优先级**：⭐⭐

**目标**：
- 拖动区块 A 到区块 B 创建链接
- 可视化关系类型选择

**技术选型**：
- `@dnd-kit/core` - React 拖放库（推荐）
- `react-dnd` - 经典拖放库

**实现示例**：
```typescript
import { useDraggable, useDroppable } from '@dnd-kit/core'

function DraggableBlock({ block }) {
  const { attributes, listeners, setNodeRef } = useDraggable({
    id: block.blockId,
  })

  return (
    <div ref={setNodeRef} {...listeners} {...attributes}>
      {block.name}
    </div>
  )
}

function DroppableBlock({ block, onLink }) {
  const { setNodeRef } = useDroppable({
    id: block.blockId,
  })

  return (
    <div ref={setNodeRef}>
      {block.name}
    </div>
  )
}
```

**收益**：
- 更快的链接创建
- 更好的用户体验

**工作量估算**：4-5 天

---

#### 权限管理 UI
**状态**：❌ 未实现（后端已完成）
**优先级**：⭐⭐

**当前状态**：
- 后端 CBAC 系统完整
- 前端无可视化界面

**实现方案**：
```typescript
function PermissionManager({ blockId }) {
  const { editors, grants } = useAppStore()

  const handleGrant = async (editorId: string, capId: string) => {
    await BlockOperations.grantPermission(
      currentFileId,
      blockId,
      editorId,
      capId
    )
  }

  return (
    <div>
      <h3>授权管理</h3>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>编辑者</TableHead>
            <TableHead>能力</TableHead>
            <TableHead>操作</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {grants.map(grant => (
            <TableRow key={grant.id}>
              <TableCell>{grant.editorId}</TableCell>
              <TableCell>{grant.capId}</TableCell>
              <TableCell>
                <Button onClick={() => handleRevoke(grant)}>
                  撤销
                </Button>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}
```

**收益**：
- 可视化权限管理
- 降低使用门槛

**工作量估算**：3-4 天

---

### 4. 扩展系统

#### 更多区块类型
**状态**：❌ 未实现（架构已支持）
**优先级**：⭐⭐⭐

**已有扩展**：
- ✅ Markdown（`markdown.write`, `markdown.read`）

**计划扩展**：

**代码块 (Code Block)**：
```rust
// src/extensions/code/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CodeWritePayload {
    pub code: String,
    pub language: String,  // "rust", "typescript", etc.
}

#[capability(id = "code.write", target = "code")]
fn handle_code_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    // 实现...
}
```

**图片块 (Image Block)**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ImageWritePayload {
    pub url: String,
    pub alt_text: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[capability(id = "image.write", target = "image")]
fn handle_image_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    // 实现...
}
```

**任务块 (Task Block)**：
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TaskWritePayload {
    pub title: String,
    pub completed: bool,
    pub due_date: Option<String>,
}

#[capability(id = "task.write", target = "task")]
fn handle_task_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    // 实现...
}
```

**工作量估算**：每个扩展 2-3 天

---

#### 插件热加载系统
**状态**：❌ 未实现
**优先级**：⭐

**目标**：
- 运行时加载外部扩展
- 不需要重新编译主程序

**技术挑战**：
- Rust 不支持动态链接库热加载（需要 FFI）
- 可能需要 WASM 插件系统

**可行方案**：

**方案 1：WASM 插件**
```rust
// 使用 wasmer 或 wasmtime
use wasmer::{Store, Module, Instance};

pub struct WasmCapability {
    instance: Instance,
}

impl CapabilityTrait for WasmCapability {
    fn execute(&self, cmd: &Command, block: Option<&Block>, state: &StateProjector)
        -> CapResult<Vec<Event>>
    {
        // 调用 WASM 模块
        let result = self.instance.exports
            .get_function("handle")?
            .call(&[/* 序列化参数 */])?;

        // 反序列化结果
        Ok(result)
    }
}
```

**方案 2：编译时注册（当前方案）**
- 保持简单，手动添加到 `CapabilityRegistry`
- MVP 阶段足够

**工作量估算**：2-3 周（复杂度高）

---

## 优先级分级

### 优先级 1（高价值 + 低成本）

| 功能 | 价值 | 成本 | 工作量 |
|------|------|------|--------|
| **批量命令处理** | 提升导入性能 5-10 倍 | 3-4 天 | ⭐⭐⭐⭐ |
| **事件广播** | 实时协作基础 | 1 周 | ⭐⭐⭐⭐ |
| **Snapshot 缓存** | 大文件加载加速 | 2-3 天 | ⭐⭐⭐⭐ |
| **多文件标签页** | 多文件编辑体验 | 2-3 天 | ⭐⭐⭐⭐ |

### 优先级 2（中价值 + 中成本）

| 功能 | 价值 | 成本 | 工作量 |
|------|------|------|--------|
| **完整向量时钟** | 更强冲突检测 | 2-3 天 | ⭐⭐⭐ |
| **图形化关系可视化** | 知识图谱体验 | 1 周 | ⭐⭐⭐ |
| **更多区块类型** | 扩展应用场景 | 2-3 天/个 | ⭐⭐⭐ |
| **撤销/重做** | 用户体验提升 | 3-4 天 | ⭐⭐⭐ |
| **权限管理 UI** | 降低使用门槛 | 3-4 天 | ⭐⭐ |

### 优先级 3（长期目标）

| 功能 | 价值 | 成本 | 工作量 |
|------|------|------|--------|
| **插件热加载** | 完整插件生态 | 2-3 周 | ⭐ |
| **增量 ZIP 更新** | 保存性能优化 | 5-7 天 | ⭐⭐ |
| **拖放式链接** | 交互体验 | 4-5 天 | ⭐⭐ |

---

## 长期规划

### 阶段 1：性能和体验（1-2 个月）

**目标**：优化核心性能，提升用户体验

**任务**：
1. ✅ 批量命令处理
2. ✅ Snapshot 缓存
3. ✅ 多文件标签页
4. ✅ 完整向量时钟

**验收标准**：
- 导入 1000 个区块 < 5 秒
- 打开 10,000 事件文件 < 1 秒
- 多文件切换流畅

---

### 阶段 2：协作和可视化（2-3 个月）

**目标**：支持多人协作，可视化知识图谱

**任务**：
1. ✅ 事件广播系统
2. ✅ 图形化区块关系
3. ✅ 撤销/重做
4. ✅ 权限管理 UI

**验收标准**：
- 两个用户可以同时编辑同一文件
- 冲突自动检测和提示
- 图谱可视化 < 2 秒加载

---

### 阶段 3：扩展生态（3-6 个月）

**目标**：构建丰富的区块类型生态

**任务**：
1. ✅ 代码块扩展（语法高亮、执行）
2. ✅ 图片块扩展（上传、预览）
3. ✅ 任务块扩展（TODO 列表）
4. ✅ 表格块扩展（数据表格）
5. ✅ 图表块扩展（Mermaid、PlantUML）
6. ⚠️ 插件热加载系统（可选）

**验收标准**：
- 至少 5 种常用区块类型
- 扩展开发文档完善
- 社区贡献至少 2 个扩展

---

### 阶段 4：企业级功能（6-12 个月）

**目标**：支持企业级应用场景

**任务**：
1. ✅ 增量 ZIP 更新
2. ✅ 完整冲突解决 UI
3. ✅ 批量导出（PDF、Markdown、HTML）
4. ✅ 权限细粒度控制（字段级）
5. ✅ 审计日志查看器
6. ✅ 多语言支持（i18n）
7. ✅ 云存储集成（S3、WebDAV）

**验收标准**：
- 支持 100+ 用户并发编辑
- 完整的审计追踪
- 企业级安全认证

---

## 提案和研究

### Part 7: 内容模式设计提案
**文档**：`docs/plans/part7-content-schema-proposal.md`
**状态**：❌ 提案阶段（未实施）
**优先级**：⭐⭐

#### 问题陈述

当前 `Block.contents` 是 `serde_json::Value`（无类型 JSON）：
- 没有编译时类型检查
- 运行时验证容易出错
- 难以文档化和维护

#### 提案方案

**混合方案**：保持灵活性 + 可选类型安全

```rust
// 1. 添加类型化访问器
impl Block {
    pub fn contents_as<T: DeserializeOwned>(&self) -> Result<T, Error> {
        serde_json::from_value(self.contents.clone())
    }

    pub fn set_contents<T: Serialize>(&mut self, value: &T) -> Result<(), Error> {
        self.contents = serde_json::to_value(value)?;
        Ok(())
    }
}

// 2. 定义内容类型（可选）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownContent {
    pub markdown: String,
    #[serde(default)]
    pub metadata: Option<MarkdownMetadata>,
}

// 3. 在能力中使用
let mut content: MarkdownContent = block.contents_as_or_default();
content.markdown = new_text.to_string();
```

#### 优势

- ✅ 编译时类型检查
- ✅ IDE 自动补全
- ✅ 自文档化
- ✅ 向后兼容
- ✅ 模式演进支持

#### 缺点

- ❌ 更多样板代码
- ❌ 序列化/反序列化开销
- ❌ 混合方法增加复杂性

#### 决策

**推荐**：采用混合方案
- 核心保持 `serde_json::Value`
- 提供类型化访问器
- 扩展可选择使用类型

**工作量估算**：1 周

---

### 其他研究方向

#### 1. WebAssembly 插件系统
**状态**：研究阶段
**优先级**：⭐

**研究问题**：
- WASM 性能开销多大？
- 如何序列化复杂类型？
- 安全沙箱如何实现？

**技术选型**：
- `wasmer` vs `wasmtime`
- `wit-bindgen` 用于类型生成

---

#### 2. 分布式协作架构
**状态**：研究阶段
**优先级**：⭐⭐

**研究问题**：
- CRDT vs OT 算法选择？
- 如何处理大规模冲突？
- P2P vs 中心化服务器？

**参考项目**：
- Yjs (CRDT 实现)
- Automerge (JSON CRDT)
- Diamond Types (文本 CRDT)

---

#### 3. AI 辅助编辑
**状态**：早期构思
**优先级**：⭐

**潜在功能**：
- 自动总结区块内容
- 智能链接推荐
- 内容生成和补全
- 知识图谱分析

**技术路径**：
- 集成本地 LLM（Ollama）
- 或调用云 API（OpenAI、Claude）

---

## 实施建议

### 短期（1-3 个月）

**聚焦**：性能和用户体验

**推荐顺序**：
1. 批量命令处理（性价比最高）
2. Snapshot 缓存（快速见效）
3. 多文件标签页（用户需求高）
4. 事件广播（协作基础）

### 中期（3-6 个月）

**聚焦**：协作和可视化

**推荐顺序**：
1. 完整向量时钟
2. 图形化关系可视化
3. 撤销/重做
4. 权限管理 UI

### 长期（6-12 个月）

**聚焦**：生态和企业级功能

**推荐顺序**：
1. 更多区块类型（3-5 个）
2. 插件热加载系统
3. 批量导出功能
4. 云存储集成

---

## 风险评估

### 技术风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| **向量时钟冲突复杂** | 中 | 中 | MVP 简化版已验证可行 |
| **WASM 插件性能** | 中 | 高 | 先实现编译时注册 |
| **大文件内存占用** | 高 | 中 | Snapshot 缓存解决 |
| **实时协作冲突** | 高 | 高 | 完整向量时钟 + UI 提示 |

### 资源风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| **开发时间不足** | 高 | 中 | 优先级 1 功能优先 |
| **测试覆盖不足** | 中 | 中 | TDD 工作流强制要求 |
| **文档滞后** | 中 | 高 | 每个功能完成即更新文档 |

---

## 成功指标

### 性能指标

| 指标 | 当前 | 目标 |
|------|------|------|
| 命令吞吐量 | 3,700 cmd/s | 10,000 cmd/s（批量） |
| 文件加载时间 | 1s (1000 事件) | 0.1s（缓存） |
| 内存占用 | 1-2MB/文件 | <5MB/文件 |
| 保存时间 | O(n) | O(1)（增量） |

### 功能指标

| 指标 | 当前 | 目标 |
|------|------|------|
| 区块类型数量 | 1 (Markdown) | 5+ |
| 并发编辑者 | 1 | 10+ |
| 测试覆盖率 | 60 个测试 | 150+ 个测试 |
| 文档页数 | 15 页 | 30+ 页 |

### 用户体验指标

| 指标 | 目标 |
|------|------|
| 学习曲线 | <30 分钟上手 |
| 崩溃率 | <0.1% |
| 数据丢失率 | 0%（事件溯源保证） |
| 用户满意度 | >4.0/5.0 |

---

## 总结

### 核心策略

1. **MVP 已完成**：100% 核心功能实现，60 个测试通过
2. **优先性能**：先优化体验，再添加功能
3. **渐进增强**：逐步添加高级特性，保持稳定性
4. **社区驱动**：根据用户反馈调整优先级
5. **文档同步**：每个功能都有完整文档

### 下一步行动

**立即开始**（推荐）：
1. 实现批量命令处理（3-4 天）
2. 添加 Snapshot 缓存（2-3 天）
3. 实现多文件标签页（2-3 天）

**预期收益**：
- 性能提升 5-10 倍
- 用户体验显著改善
- 为协作功能打基础

---

**文档版本**：v1.0
**上次更新**：2025-10-28
**下次审核**：2025-11-28（或完成优先级 1 功能后）

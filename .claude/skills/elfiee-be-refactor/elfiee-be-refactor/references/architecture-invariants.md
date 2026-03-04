# 架构不变量（禁止修改项）

> 重构过程中必须保持的核心约束。违反任何一条都可能破坏 Event Sourcing 一致性或 CBAC 安全性。

---

## 一、Event Sourcing 不变量

### 1.1 Event 是唯一事实来源

- **禁止**：直接修改 Block/Editor/Grant 的内存状态而不通过 Event
- **禁止**：在 Handler 中产生 I/O 副作用（写文件、网络请求、进程操作）
- **必须**：所有状态变更先写 Event 到 eventstore.db，再由 StateProjector 投影
- **必须**：Event 不可变（append-only），已写入的 Event 不可修改或删除

### 1.2 EAVT 模型

- **禁止**：修改 Event 结构中 entity/attribute/value/timestamp 四个字段的语义
- **必须**：attribute 格式保持 `"{editor_id}/{cap_id}"`
- **必须**：timestamp 保持 Vector Clock 格式 `{ editor_id: count }`

### 1.3 Command → Event 流程

```
Command → Certificator → Handler → Event[] → EventStore → StateProjector
```

- **禁止**：跳过 Certificator 直接执行 Handler
- **禁止**：在 Command 处理流程外修改 eventstore.db
- **必须**：Handler 返回 `Vec<Event>`，由 Actor 统一持久化

---

## 二、Actor 模型不变量

### 2.1 串行处理保证

- **禁止**：同一 .elf 的 Command 并行处理
- **禁止**：绕过 Actor 邮箱直接操作 EventStore 或 StateProjector
- **必须**：一个 .elf 文件对应一个 Actor（tokio mpsc channel）
- **必须**：EngineManager 管理 Actor 生命周期（创建、查询、销毁）

### 2.2 Actor 内部组件

- `StateProjector`：维护内存状态，只能通过 `apply_events()` 修改
- `EventStore`：SQLite 连接，只能通过 `append_events()` 写入
- `CapabilityRegistry`：能力注册表，只读

---

## 三、CBAC 不变量

### 3.1 授权检查顺序（不可更改）

```rust
fn is_authorized(editor_id, cap_id, block_id) -> bool {
    // 1. editor 必须存在（防止删除后的权限继承攻击）
    if !editors.contains(editor_id) { return false; }

    // 2. system editor 始终有权
    if is_system_editor(editor_id) { return true; }

    // 3. block owner 始终有权
    if block.owner == editor_id { return true; }

    // 4. 检查 GrantsTable
    grants.has(editor_id, cap_id, block_id)
      || grants.has(editor_id, cap_id, "*")
}
```

### 3.2 Grant 约束

- **禁止**：绕过 `core.grant`/`core.revoke` Event 直接修改 GrantsTable
- **禁止**：在 Agent Block contents 中存储 capabilities 列表（与 CBAC 冲突）
- **必须**：Grant 三元组 (editor_id, cap_id, block_id) 唯一
- **必须**：删除 Editor 后，该 Editor 的所有操作被拒绝（即使是 owner）

---

## 四、Capability 系统不变量

### 4.1 Capability 定义

- **禁止**：在 Handler 中做 I/O（文件、网络、进程、数据库——eventstore.db 除外）
- **必须**：每个 Capability 有唯一的 `cap_id`（格式 `"{extension}.{action}"`）
- **必须**：每个 Capability 声明 `target`（作用的 block_type 模式）
- **必须**：使用 `#[capability(id = "...", target = "...")]` 宏注册

### 4.2 Payload 类型安全

- **禁止**：手动 JSON 解析 payload（必须使用 typed struct + `serde_json::from_value`）
- **禁止**：手动编写 TypeScript payload 接口（必须由 tauri-specta 自动生成）
- **必须**：Payload struct 使用 `#[derive(Serialize, Deserialize, Type)]`
- **必须**：Extension-specific payload 定义在 `extensions/{name}/mod.rs`

---

## 五、数据模型不变量

### 5.1 Block

- `block_id`：UUID，创建后不可变
- `owner`：创建者 editor_id，创建后不可变
- `children`：DAG（有向无环图），禁止环和自引用
- `block_type`：只允许 `document` / `agent` / `task` / `session`（重构后）

### 5.2 Editor

- `editor_id`：UUID，创建后不可变
- `editor_type`：`Human` 或 `Bot`，创建后不可变
- 删除后放入 `deleted_editors` 集合，不可恢复

### 5.3 Event

- `event_id`：UUID，创建后不可变
- 所有字段创建后不可变（append-only log）
- 物理时钟 `created_at` 只用于显示，不用于排序

---

## 六、TypeScript 绑定不变量

- **绝对禁止**：手动编辑 `src/bindings.ts`（tauri-specta 自动生成）
- **必须**：修改 Rust Command/Payload 后重新构建以更新 bindings
- **必须**：所有 Payload 类型在 `lib.rs` 中注册到 specta export

---

## 七、文件结构不变量

### .elf/ 目录结构（重构后）

```
project.elf/
├── eventstore.db       # 唯一事实来源
├── config.toml         # 本地配置
├── templates/          # 工作模板
└── snapshots/          # 派生快照（可重建）
```

- `eventstore.db` 是唯一不可丢失的文件
- 其余文件均可从 Event 链重建

---

## 八、验证检查清单

重构的每一步完成后，运行以下检查：

- [ ] `cargo test` 全部通过
- [ ] `cargo clippy` 无 warning
- [ ] Handler 中无 `std::fs`、`std::process`、`std::net` 使用
- [ ] 所有 Capability 都在 Registry 中注册
- [ ] 所有 Payload 都在 specta export 中注册
- [ ] GrantsTable 只通过 Event 修改
- [ ] Block.children 无环（cycle detection 测试通过）
- [ ] 删除的 Editor 无法通过授权检查

# Changelog: event-system.md 重构

> 对应概念文档：`docs/mvp/frame/concepts/event-system.md`

---

## L1 事件系统重构 Checklist

### EventStore 部分回放查询

- [x] 新增 `get_events_after_event_id(pool, event_id)` — 基于 rowid 子查询的增量回放
- [x] 新增 `get_latest_event_id(pool)` — 获取最新 event ID（用于快照保存位置）
- [x] 4 个新测试：after_event_id、after_last_returns_empty、latest_event_id、nonexistent_returns_empty

### CacheStore 快照缓存（新模块）

- [x] 新建 `src-tauri/src/engine/cache_store.rs`
- [x] `snapshots` 表 schema：`(block_id, event_id, state)`，联合主键 `(block_id, event_id)`
- [x] 无 `created_at` 字段——时序由 event_id 对应的 Vector Clock 决定
- [x] `create(path)` — 创建/打开缓存数据库
- [x] `save_snapshot(pool, block_id, event_id, state)` — INSERT OR REPLACE
- [x] `save_snapshots_batch(pool, event_id, states)` — 批量保存
- [x] `get_latest_snapshot(pool, block_id)` — 按 rowid 降序取最新
- [x] `get_all_latest_snapshots(pool)` — 每个 block 的最新快照
- [x] `delete_snapshots_for_block(pool, block_id)` — Block 删除时清理
- [x] `clear_all(pool)` — 测试/重建用
- [x] `cache_path_for_project(project_path)` — 计算 `~/.elf/cache/{hash}/cache.db`
- [x] 注册到 `engine/mod.rs`，导出 `CacheStore`
- [x] 13 个单元测试全部通过

### StateProjector mode 感知处理

- [x] 导入 `EventMode`
- [x] write 事件根据 `event.mode` 分支处理：
  - `Full`：合并 contents（原有行为，不变）
  - `Delta`：placeholder，存储 diff 内容，log 提示 Step 5 实现
  - `Ref`：整个 contents 替换为 ref 元数据
  - `Append`：追加 entry 到 `contents.entries` 数组
- [x] 新增 `to_snapshot_state(block_id)` — 序列化 Block 为快照 JSON
- [x] 新增 `all_snapshot_states()` — 序列化所有 Block
- [x] 新增 `restore_from_snapshot(block_id, state)` — 从快照恢复 Block（含 reverse index）
- [x] 13 个新测试：4 个 mode 测试（full/delta/ref/append）+ 6 个快照测试 + 3 个辅助测试

### 未改动（不在 L1 范围）

- [x] Actor 生命周期集成（启动加载/关闭保存）→ L3 Engine 步骤
- [x] 运行时快照触发（task.completed、checkpoint、每 N 次 write）→ L3 Engine 步骤
- [x] Delta 真正的 diff apply 逻辑 → L4 Extension 步骤（document extension）
- [x] Session append handler → L4 Extension 步骤（session extension）

### 测试验证

- [x] `cargo check` 零错误
- [x] `cargo test` 全部通过（457 unit + 54 integration = 0 failures）
- [x] 无越界修改（仅 engine/ 和 engine/mod.rs）

---

## 修改文件清单

| 文件 | 操作 |
|------|------|
| `src-tauri/src/engine/event_store.rs` | 新增 2 个查询方法 + 4 个测试 |
| `src-tauri/src/engine/cache_store.rs` | **新建** — 快照缓存存储 + 13 个测试 |
| `src-tauri/src/engine/state.rs` | mode 感知处理 + 快照序列化/恢复 + 13 个测试 |
| `src-tauri/src/engine/mod.rs` | 注册 cache_store 模块，导出 CacheStore |

# CBAC 读取过滤现状

> 日期: 2026-01-30
> 状态: 已实现，无需额外工作

## 1. 结论

**CBAC 对 Events、Blocks、Directory entries 的读取过滤已经全部实现。** 不需要在上下文优化管道中单独添加 CBAC 裁剪层。

## 2. 已实现的三层读取过滤

### 2.1 单个 Block 读取 — `get_block()`

- 文件: `src-tauri/src/commands/block.rs`
- 按 block_type 检查对应的 read 能力（`markdown.read` / `code.read` / `directory.read`）
- 无权限 → 返回错误

### 2.2 Block 列表 — `get_all_blocks()`

- 文件: `src-tauri/src/commands/block.rs`
- 逐个检查 `core.read` 权限
- 无权限的 Block 直接排除
- 目录 Block 特殊处理：有 `core.read` 但无 `directory.read` → 可以看到目录存在，但 entries 被清空

### 2.3 Event 历史 — `get_all_events()`

- 文件: `src-tauri/src/commands/file.rs`
- Editor 事件（`entity.starts_with("editor-")`）→ 无条件保留
- Block 事件 → 逐个检查 `core.read`，无权限的 Block 事件被过滤

```rust
// get_all_events 中的过滤逻辑
for event in all_events {
    if event.entity.starts_with("editor-") {
        filtered_events.push(event);  // editor 事件不过滤
        continue;
    }
    let has_core_read = handle.check_grant(
        effective_editor_id.clone(),
        "core.read".to_string(),
        event.entity.clone(),
    ).await;
    if has_core_read {
        filtered_events.push(event);
    }
}
```

## 3. 权限检查链路

```
Tauri 命令（查询层）
    ↓
get_block() / get_all_blocks() / get_all_events()
    ↓
确定 effective editor_id（参数或活跃编辑器）
    ↓
EngineHandle::check_grant(editor_id, cap_id, block_id)
    ↓
通过消息发给 Engine Actor
    ↓
StateProjector::is_authorized()
    ↓
├─ 块所有者 == editor_id → true
└─ 否则 → GrantsTable::has_grant()
   ├─ 精确匹配: (editor_id, cap_id, block_id)
   ├─ 块通配符: (editor_id, cap_id, "*")
   ├─ 编辑器通配符: ("*", cap_id, block_id)
   └─ 全通配符: ("*", cap_id, "*")
```

## 4. 对上下文优化的影响

上下文优化管道（context_projector + compressor）不需要单独的 CBAC 裁剪层，因为：

1. 如果通过现有查询接口（`get_all_events()` / `get_block()`）获取数据 → CBAC 自动生效
2. 如果为性能直接访问 `StateProjector` 内存数据 → 需要手动调用 `is_authorized()` 检查

```rust
// ContextProjector 正确做法:
// 方案 A: 走查询接口（自动 CBAC）
let events = self.handle.get_all_events().await;

// 方案 B: 直接访问内存（手动 CBAC）
for (block_id, block) in &self.state.blocks {
    if !self.state.is_authorized(editor_id, "core.read", block_id) {
        continue;
    }
    // ... 构建上下文
}
```

只要守住原则 — **任何数据出口都走 `is_authorized` 检查** — 就不需要额外的 CBAC 层。

## 5. 现有限制

| 限制 | 说明 | 影响 |
|---|---|---|
| 事件粒度全有或全无 | 有 `core.read` 则看到该 Block 的所有事件，否则一条都看不到 | 无法按能力类型过滤事件（如"只看 write 不看 revoke"） |
| Block 内容无子级权限 | 有 `markdown.read` 则看到整个 Block 内容 | 无法限制只看部分字段 |
| 读能力不生成事务性事件 | `code.read` 返回空事件向量，`markdown.read` 仅生成审计事件 | 读操作的审计粒度不一致 |

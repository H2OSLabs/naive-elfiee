# Changelog: data-model.md 重构

> 对应概念文档：`docs/mvp/frame/concepts/data-model.md`

---

## L0 数据模型重构 Checklist

### Block 类型系统

- [x] `block_type` 保持 `String` 类型 — 由 Extension 注册决定，不使用枚举
- [x] Block struct 字段符合 data-model.md 定义（block_id, name, block_type, contents, children, owner, metadata）
- [x] `RELATION_IMPLEMENT` 常量保留
- [x] 删除 `BlockType` 枚举及所有相关代码（enum、impl、Serialize/Deserialize、Display）
- [x] 删除 `from_str_compat()` 向后兼容函数
- [x] 删除 `is_valid()` / `is_canonical()` 验证函数
- [x] 删除 12 个 BlockType 相关测试
- [x] `mod.rs` 移除 `BlockType` 导出

### Event mode 字段

- [x] 新增 `EventMode` 枚举：Full / Delta / Ref / Append
- [x] `Event` struct 新增 `mode: EventMode` 字段
- [x] `Event::new()` 默认 mode = Full
- [x] 新增 `Event::new_with_mode()` 构造函数
- [x] EventMode 自定义 Serialize/Deserialize（小写字符串）
- [x] `mod.rs` 导出 `EventMode`
- [x] 删除向后兼容反序列化测试

### EventStore 适配

- [x] Schema 新增 `mode TEXT NOT NULL DEFAULT 'full'` 列
- [x] `append_events` INSERT 包含 mode 字段
- [x] `get_all_events` / `get_events_by_entity` SELECT 包含 mode 列
- [x] `row_to_event` 读取 mode 列并转换为 EventMode
- [x] 删除 ALTER TABLE 旧数据库兼容代码
- [x] 删除 row_to_event 中的 fallback 容错逻辑

### CreateBlockPayload 扩展

- [x] 新增 `contents: Option<serde_json::Value>` — 初始内容
- [x] 新增 `format: Option<String>` — 文档格式标识（md/rs/py 等）
- [x] 更新注释：block_type 由 Extension 注册决定
- [x] 删除旧类型名兼容注释和测试
- [x] 新增 4 个测试（with_contents, task_type, session_type, with_metadata 已有）

### .elftypes 类型映射

- [x] `[markdown]` + `[code]` 合并为 `[document]`
- [x] block_type_inference 默认回退改为 `"document"`
- [x] 更新 block_type_inference 测试

### 未改动（符合 data-model.md，无需修改）

- [x] Editor struct — editor_id, name, editor_type（human/bot）
- [x] Command struct — cmd_id, editor_id, cap_id, block_id, payload, timestamp
- [x] Grant struct — editor_id, cap_id, block_id
- [x] Capability struct — cap_id, target

### 测试验证

- [x] `cargo check` 零错误
- [x] `cargo test` 全部通过（487 tests, 0 failures）
- [x] 无 BlockType 枚举残留引用
- [x] 无 `from_str_compat` 残留引用

---

## 修改文件清单

| 文件 | 操作 |
|------|------|
| `src-tauri/src/models/block.rs` | 删除 BlockType 枚举 + 所有 impl + 12 个测试 |
| `src-tauri/src/models/event.rs` | 新增 EventMode 枚举 + mode 字段 + 删除 compat 测试 |
| `src-tauri/src/models/payloads.rs` | 新增 contents/format 字段 + 删除 legacy 测试 |
| `src-tauri/src/models/mod.rs` | 移除 BlockType 导出，新增 EventMode 导出 |
| `src-tauri/src/engine/event_store.rs` | Schema/INSERT/SELECT/row_to_event 适配 mode |
| `src-tauri/src/utils/block_type_inference.rs` | 默认类型改为 document |
| `src-tauri/.elftypes` | 合并为 [document] |

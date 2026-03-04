# 时间工具模块重构总结

## 概述

本次重构将所有时间相关的功能提取到统一的工具模块中（`src-tauri/src/utils/time.rs`），确保：
- 除向量时钟（Vector Clock）外，所有业务时间使用包含时区的 RFC 3339 格式
- 统一的时间处理接口
- 避免直接使用 chrono 导致的不一致

## 新增文件

### 1. `src-tauri/src/utils/time.rs`
时间工具模块，提供以下功能：

#### DateTime 对象生成
- `now_utc_datetime()` - 返回 `DateTime<Utc>` 对象
- `now_local_datetime()` - 返回 `DateTime<Local>` 对象

#### 时间戳字符串生成（RFC 3339 格式）
- `now_utc()` - UTC 时间戳字符串 (e.g., "2025-12-17T02:30:00Z")
- `now_local()` - 本地时区时间戳字符串 (e.g., "2025-12-17T10:30:00+08:00")

#### 时间戳解析
- `parse_to_utc(timestamp: &str)` - 解析 RFC 3339 时间戳并转换为 UTC
- `parse_to_local(timestamp: &str)` - 解析 RFC 3339 时间戳并转换为本地时区

#### 系统时间转换
- `system_time_to_utc(system_time: SystemTime)` - 将文件系统时间戳转换为 UTC 字符串
- `system_time_to_local(system_time: SystemTime)` - 将文件系统时间戳转换为本地时区字符串

#### 格式化
- `format_utc(datetime: DateTime<Utc>)` - 格式化 UTC DateTime 为字符串
- `format_local(datetime: DateTime<Local>)` - 格式化本地 DateTime 为字符串

### 2. `src-tauri/src/utils/mod.rs`
工具模块入口，导出常用时间函数。

## 修改文件

### 1. `src-tauri/src/lib.rs`
- 添加 `pub mod utils;` 注册工具模块

### 2. `src-tauri/src/commands/file.rs`
**修改内容**：
- 添加 `use crate::utils::time;`
- 重构 `get_file_info()` 中的时间戳获取逻辑

**修改前**：
```rust
let created_at = metadata
    .created()
    .map(|t| {
        chrono::DateTime::<chrono::Utc>::from(t)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    })
    .unwrap_or_else(|_| "Unknown".to_string());
```

**修改后**：
```rust
let created_at = metadata
    .created()
    .ok()
    .and_then(|t| time::system_time_to_utc(t).ok())
    .unwrap_or_else(|| "Unknown".to_string());
```

**优势**：
- 更简洁的代码
- 统一的错误处理
- 使用工具函数确保格式一致

### 3. `src-tauri/src/models/command.rs`
**修改内容**：
- 添加 `use crate::utils::time;`
- 重构 `Command::new()` 中的时间戳生成

**修改前**：
```rust
timestamp: chrono::Utc::now(),
```

**修改后**：
```rust
timestamp: time::now_utc_datetime(),
```

**优势**：
- 统一时间生成接口
- 保持 `DateTime<Utc>` 类型不变（用于时间计算）
- 序列化时自动转换为 RFC 3339 格式

## 向量时钟（Vector Clock）说明

以下代码**不在重构范围内**，保持不变：
- `Event.timestamp: HashMap<String, i64>` - 向量时钟用于冲突检测
- 所有使用向量时钟的代码（`engine/actor.rs`, `engine/event_store.rs`, `capabilities/core.rs` 等）

**理由**：向量时钟是分布式系统的逻辑时钟，不是物理时间戳，用途完全不同。

## 测试结果

### 新增测试（8 个）
所有测试位于 `src-tauri/src/utils/time.rs`：
- `test_now_utc` - 测试 UTC 时间戳生成
- `test_now_local` - 测试本地时间戳生成
- `test_parse_to_utc` - 测试时间戳解析和时区转换
- `test_parse_to_local` - 测试本地时区转换
- `test_parse_invalid_timestamp` - 测试错误处理
- `test_system_time_conversion` - 测试系统时间转换
- `test_format_functions` - 测试格式化函数
- `test_roundtrip_utc` - 测试往返转换一致性

### 测试结果
```
✅ 所有 79 个库测试通过
✅ 时间工具模块测试全部通过
✅ 现有功能未受影响
```

## 使用指南

### 业务代码中使用时间

**唯一推荐做法**：
```rust
use crate::utils::time;

// 获取当前 UTC 时间戳（RFC 3339 格式）
let timestamp = time::now_utc();  // "2025-12-17T02:30:00Z"
```

**就这么简单！其他所有时间函数都是内部的。**

### 内部函数（crate 内部可用）

```rust
// 在 crate 内部（如 commands/file.rs）可以使用：
use crate::utils::time;

// 获取 DateTime 对象（用于 Command.timestamp）
let dt = time::now_utc_datetime();

// 转换文件系统时间
let created = time::system_time_to_utc(metadata.created()?)?;

// 测试用的辅助函数
let local = time::now_local();
let parsed = time::parse_to_utc(&timestamp)?;
```

**避免直接使用 chrono**：
```rust
// ❌ 避免：需要额外 import，容易格式不一致
use chrono::Utc;
let ts = Utc::now().to_rfc3339();

// ✅ 推荐：统一格式
let ts = time::now_utc();
```

### 前端集成

前端应该使用自动生成的 TypeScript 类型（`src/bindings.ts`），时间戳字段会自动以 RFC 3339 字符串形式传输：

```typescript
// 自动生成的类型
export type FileMetadata = {
  file_id: string;
  name: string;
  path: string;
  collaborators: string[];
  created_at: string;      // RFC 3339 格式
  last_modified: string;   // RFC 3339 格式
};
```

## 时区安全性

所有业务时间戳现在都：
- ✅ 包含明确的时区信息
- ✅ 使用 RFC 3339 (ISO 8601) 标准格式
- ✅ 可在不同时区环境下正确解析
- ✅ 避免时区转换错误

**示例**：
- UTC: `"2025-12-17T02:30:00Z"`
- 东八区: `"2025-12-17T10:30:00+08:00"`
- 这两个时间戳表示同一时刻，可以正确互相转换

## 未来扩展

如果需要添加新的时间相关功能，应该：
1. 在 `utils/time.rs` 中添加新函数
2. 在 `utils/mod.rs` 中重新导出（如果是常用函数）
3. 添加相应的测试
4. 更新本文档

## 相关文档

- 时间工具模块源码：`src-tauri/src/utils/time.rs`
- 时间格式规范：参考 `docs/` 目录中的相关文档
- RFC 3339 标准：https://www.rfc-editor.org/rfc/rfc3339

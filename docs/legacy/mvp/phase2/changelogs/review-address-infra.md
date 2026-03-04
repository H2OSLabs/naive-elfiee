# 基础设施 Code Review 修复记录

## 概述

针对 Phase 2 基础设施模块（2.1 快照、2.3 Relation、2.4 .elf/ 元数据）的 code review 反馈进行修复。

## 变更文件清单

### 1. 日志统一：`eprintln!` → `log::warn!`

**问题**：生产代码中混用 `eprintln!` 和 `log::warn!`，`eprintln!` 无法被日志框架捕获/过滤。

**文件**: `src-tauri/src/engine/actor.rs`
- L176: Snapshot error for content writes (`markdown.write` / `code.write`)
- L190: Snapshot error for directory writes (`directory.write`)
- L204: Snapshot error for new blocks (`core.create`)
- L477: Vector clock conflict detection warning（同时移除了多余的 `"Warning: "` 前缀）

**文件**: `src-tauri/src/extensions/terminal/commands.rs`
- L253: PTY read error in spawned thread

**保留不变**：`src-tauri/src/engine/manager.rs` 中 3 处 `eprintln!` 均在 `#[cfg(test)]` 测试代码中，测试输出到 stderr 是合理的。

### 2. 魔法字符串提取为常量

**问题**：`.elf` block name 和 description 硬编码在 `bootstrap_elf_meta()` 中，后续 Agent、Session 等模块也需要引用。

**文件**: `src-tauri/src/extensions/directory/elf_meta.rs`
- 新增 `pub const ELF_META_BLOCK_NAME: &str = ".elf";`
- 新增 `pub const ELF_META_DESCRIPTION: &str = "Elfiee system metadata directory";`
- `bootstrap_elf_meta()` 中 `json!` 使用常量替代硬编码字符串

## 未修改的反馈项（及理由）

| 反馈项 | 结论 | 理由 |
|--------|------|------|
| Snapshot 失败是否应 fail command | 不改 | 设计明确 snapshot 是 derived data，event store 是 source of truth |
| Reverse Index 提取辅助方法 | 不改 | 4 个分支上下文不同（create/write/unlink/delete），过度抽象反增复杂度 |
| Wildcard Grant 权限升级风险 | 不改 | `core.grant` 的 certificator 已限制只有 block owner 可授权 |
| Cycle Detection 仅在 core.link | 不改 | `Block.children` 修改只能通过 `core.link`/`core.unlink`，设计正确 |
| Snapshot filename 无扩展名文件 | 不改 | `body.txt` fallback 保留可读性，MIME 推断收益有限 |
| Changelog 硬编码测试数字 | 不改 | Changelog 是历史快照，数字反映写入时状态 |
| 性能（反向索引内存 / Snapshot I/O） | 不改 | 桌面场景下完全可接受 |
| Path Traversal 安全 | 不改 | block_id 是系统生成的 UUID，无用户输入风险 |

## 测试结果

全部 **311 个后端测试通过**（267 单元 + 34 集成 + 5 文档 + 3 snapshot + 2 ignored）。

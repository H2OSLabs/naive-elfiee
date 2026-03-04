# PR Review 反馈修复

## 概述

根据 PR review 反馈，对 Agent Block Phase 2 实现进行了 7 项改进。涵盖后端基础设施优化和前端协作者列表 bug 修复。

**分支**: `feat/agent-block-new`
**日期**: 2026-02-04

---

## 修改清单

| # | 问题 | 来源 | 改动范围 |
|---|------|------|----------|
| 1 | 硬编码 capability 列表 | Reviewer | 后端 |
| 2 | Agent 恢复静默失败 | Reviewer | 后端 |
| 3 | 端口回收 + 耗尽处理 | Reviewer | 后端 |
| 4 | 魔法字符串 `"elfiee"` | Reviewer | 后端 |
| 5 | Symlink 重复创建 | Reviewer | 后端 |
| 6 | 系统编辑者不出现在协作者列表 | 自测 Bug | 前端 |
| 7 | Agent 块显示错误权限 | 自测 Bug | 前端 |

---

## 1. 硬编码 capability 列表 → 动态获取

**问题**: `commands/agent.rs` 中 26 行硬编码的 `default_caps` 数组，每新增 capability 需手动更新。

**修复**:

- `capabilities/registry.rs`: 新增 `get_grantable_cap_ids(&[&str]) -> Vec<String>` 方法，从 registry 动态获取所有已注册 capability，排除指定列表
- `commands/agent.rs`: `do_agent_create` 改为调用 `CapabilityRegistry::new().get_grantable_cap_ids(&["core.grant", "core.revoke", "editor.create", "editor.delete"])`
- 新增单元测试 `test_get_grantable_cap_ids_excludes_specified`

**影响**: 后续新增 extension/capability 时，agent 自动获得权限，无需手动维护列表。

---

## 2. Agent 恢复静默失败 → 返回错误列表

**问题**: `recover_agent_servers` 中错误只 `eprintln!`，调用方无感知。

**修复**:

- `commands/agent.rs`: `recover_agent_servers` 返回类型从 `()` 改为 `Vec<(String, String)>`（agent 名称 + 错误信息）
- `commands/file.rs`: `open_file` 捕获返回值并记录失败数量和详情

**影响**: 恢复失败不再静默。后续可将错误信息推送到前端 toast。

---

## 3. 端口回收 + 耗尽处理

**问题**: `next_agent_port` 只递增不回绕。在应用生命周期内反复 enable/disable 超过 99 次后，即使没有活跃 agent 也会报端口耗尽。

**修复**:

- `mcp/transport.rs`: `allocate_agent_port` 改为循环分配，超过 47299 时回绕到 47201
- 最大尝试次数限制为 99（端口总数），避免无限循环
- 通过 `agent_servers` DashMap 检查端口是否真正被占用

**之前**:
```rust
let port = app_state.next_agent_port.fetch_add(1, Ordering::SeqCst);
if port > 47299 {
    return Err("Agent port range exhausted".to_string());
}
```

**之后**:
```rust
for _ in 0..99 {
    let port = fetch_add(1);
    if port > 47299 { /* 回绕到 47201 */ }
    if !in_use(port) { return Ok(port); }
}
Err("all 99 ports are in use")
```

---

## 4. 魔法字符串 `"elfiee"` → 常量

**问题**: MCP 服务名 `"elfiee"` 在 `agent.rs` 和 `server.rs` 中硬编码 6 处。

**修复**:

- `commands/agent.rs`: 新增 `pub const MCP_SERVER_NAME: &str = "elfiee"`
- `perform_enable_io` 和 `perform_disable_io` 中 4 处 `"elfiee"` 替换为 `MCP_SERVER_NAME`
- `mcp/server.rs`: `get_info()` 中服务名替换为 `crate::commands::agent::MCP_SERVER_NAME`

**未改动**: 测试代码和 `mcp_config.rs` 中的字面量保留（测试应验证实际值）。

---

## 5. Symlink 重复创建 → early return

**问题**: `create_symlink_dir` 每次调用都删除并重建 symlink，即使已指向正确目标。

**修复**:

```rust
// 之前
if dst.exists() || dst.read_link().is_ok() {
    remove_symlink_dir(dst)?; // 总是删除
}
// 创建新 symlink...

// 之后
if let Ok(current_target) = dst.read_link() {
    if current_target == src {
        return Ok(()); // 已正确指向，跳过
    }
    remove_symlink_dir(dst)?; // 指向错误目标，删除
} else if dst.exists() {
    remove_symlink_dir(dst)?; // 非 symlink，删除
}
// 创建新 symlink...
```

**影响**: 幂等性增强，减少不必要的文件系统操作。

---

## 6. 系统编辑者不出现在协作者列表

**问题**: `CollaboratorList.tsx` 按 `block.owner` 过滤协作者。当 bot 通过 MCP 创建 block 时，`block.owner = bot_id`，系统编辑者（文件所有者）不显示，尽管后端赋予其完整权限。

**修复**:

- `CollaboratorList.tsx`:
  - 新增 `systemEditorId` state + `useEffect` 调用 `getSystemEditorId()`
  - 过滤逻辑增加 `systemEditorId` 匹配
  - 排序：系统编辑者 → block owner → active editor → 其他
  - 传递 `isFileOwner` prop 给 `CollaboratorItem`

- `CollaboratorItem.tsx`:
  - 新增 `isFileOwner` prop
  - 新增 `hasFullAccess = isOwner || isFileOwner` 统一判断
  - File Owner 显示 Crown 图标 + "File Owner" 徽章
  - 权限复选框全选且禁用（与 Owner 行为一致）
  - 不显示下拉菜单（无法移除 File Owner 权限）

- 测试：
  - `CollaboratorList.test.tsx`: 新增 bot 创建的 block 中系统编辑者始终可见的测试
  - `CollaboratorItem.test.tsx`: 新增 5 个 `isFileOwner` 行为测试

---

## 7. Agent 块显示错误权限

**问题**: `getAvailableCapabilities()` 无 `case 'agent'`，fallthrough 到 default 显示 `markdown.read/markdown.write/core.delete`。

**修复**:

- `CollaboratorItem.tsx`: 新增 `case 'agent'` 返回:
  - `core.read` → Read（查看 agent 状态）
  - `agent.enable` → Manage（启用/禁用/配置，仅 owner）
  - `core.delete` → Delete（仅 owner）

- 测试：
  - `CollaboratorItem.test.tsx`: 新增 2 个 agent block 权限测试

---

## 未修改项（记录决策）

| # | 问题 | 决策 | 原因 |
|---|------|------|------|
| A | `do_agent_create` 165 行过长 | 不拆分 | 逻辑为顺序流程，拆分不减少复杂度 |
| B | SSE 重连热加载 | 创建 issue | Claude Code 不支持 `.mcp.json` 热加载，需端口持久化方案，改动量大，提交新的issue下次解决 |
| C | 多 agent 集成测试 | 创建 issue | 需异步测试框架支持，编写成本高，下一个迭代 |
| D | 旧 `.elf` 文件迁移文档 | 不需要 | Agent block 是新增类型，无 schema 变更 |

---

## 测试结果

```
Rust:       所有测试通过 (unit + integration + doc-tests)
Frontend:   106/106 测试通过
TypeScript: 类型检查通过 (tsc --noEmit)
```

---

## 改动文件清单

### 后端 (Rust)

| 文件 | 改动 |
|------|------|
| `src-tauri/src/capabilities/registry.rs` | +`get_grantable_cap_ids()` 方法 + 测试 |
| `src-tauri/src/commands/agent.rs` | 动态 cap 列表 + `MCP_SERVER_NAME` 常量 + symlink early return + recovery 返回错误 |
| `src-tauri/src/commands/file.rs` | 捕获 recovery 错误 |
| `src-tauri/src/mcp/server.rs` | 使用 `MCP_SERVER_NAME` 常量 |
| `src-tauri/src/mcp/transport.rs` | 端口回绕分配 |

### 前端 (TypeScript/React)

| 文件 | 改动 |
|------|------|
| `src/components/permission/CollaboratorList.tsx` | 系统编辑者过滤 + 排序 + `isFileOwner` prop |
| `src/components/permission/CollaboratorItem.tsx` | `isFileOwner` + `hasFullAccess` + agent block 权限 + File Owner 徽章 |
| `src/components/permission/CollaboratorList.test.tsx` | 系统编辑者测试 |
| `src/components/permission/CollaboratorItem.test.tsx` | isFileOwner + agent block 测试 |
| `src/test/setup.ts` | `getSystemEditorId` mock 默认返回值 |

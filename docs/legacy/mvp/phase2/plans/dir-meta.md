# 2.4 .elf/ 元数据管理 — 设计文档

## 概述

`.elf/` 是每个 `.elf` 文件的系统级 Dir Block，在 `create_file` 时自动创建。它提供一个固定的目录骨架（仅虚拟目录 entries，不含文件 Block），供后续模块（F7 Skills、F1 Agent、F10 Session、F16 Task）在其中创建实际内容。

## 目录结构

```
.elf/                              # 唯一系统级 Dir Block (flat namespace)
├── agents/
│   └── elfiee-client/
│       ├── scripts/               # 预留：自定义脚本
│       ├── assets/                # 预留：静态资源
│       └── references/            # 预留：参考文档
├── session/                       # 预留：Session JSONL 存储
└── git/                           # 预留：Git hooks 模板
```

所有 entries 均为 `"type": "directory"` 虚拟条目，**不创建任何 content Block**。

### git/ 目录用途

`git/` 存放 pre-commit / pre-push hook 模板。当 `agent.enable` 连接外部项目时，注入这些 hooks 到 `.git/hooks/`，防止用户绕过 Elfiee 直接 `git commit/push`（导致变更未记录到 EventStore）。

具体内容由 3.5 Task Block 模块（F16）和 3.1 Agent 模块（F1/F3）在后续阶段填充。

## 实现任务

### I10-01a: GrantsTable editor_id 通配符

**文件**: `src-tauri/src/capabilities/grants.rs`

**变更**: `has_grant()` 支持 `editor_id = "*"` 通配符，表示"任意 editor"。

**当前逻辑**:
```rust
// grants.rs:125 — 仅 block_id 支持通配符
.any(|(cap, blk)| cap == cap_id && (blk == block_id || blk == "*"))
```

**修改后**:
```rust
pub fn has_grant(&self, editor_id: &str, cap_id: &str, block_id: &str) -> bool {
    // 1. 精确匹配：查找 editor_id 的显式 grant
    let exact = self.grants.get(editor_id).map_or(false, |grants| {
        grants
            .iter()
            .any(|(cap, blk)| cap == cap_id && (blk == block_id || blk == "*"))
    });
    if exact {
        return true;
    }

    // 2. 通配符匹配：查找 editor_id = "*" 的 grant（"所有人"）
    self.grants.get("*").map_or(false, |grants| {
        grants
            .iter()
            .any(|(cap, blk)| cap == cap_id && (blk == block_id || blk == "*"))
    })
}
```

**语义**: `core.grant(target_editor="*", capability="directory.write", target_block="{elf_block_id}")` 表示"所有 editor 对此 block 拥有 directory.write 权限"。

**影响范围**: `is_authorized()` 调用 `has_grant()` → 自动生效。`core.revoke` 也通过 `remove_grant()` 匹配 `"*"`，逻辑对称。

### I10-01b: .elf/ Dir Block 初始化

**文件**: `src-tauri/src/extensions/directory/elf_meta.rs` (新建)

**触发时机**: `create_file` 命令中，`bootstrap_editors` 之后调用。

**流程**:

```
create_file(path)
  ├── ElfArchive::new()
  ├── archive.save(path)
  ├── spawn_engine(file_id, event_pool)
  ├── store file info
  ├── bootstrap_editors()          ← 已有
  └── bootstrap_elf_meta()         ← 新增
        ├── core.create Dir Block (".elf", type="directory", source="outline")
        ├── directory.write 写入目录 entries
        └── core.grant("*", "directory.write", elf_block_id)
```

**生成的 Events** (共 3 个):

1. **core.create** — 创建 `.elf/` Dir Block
```json
{
  "entity": "{elf_block_id}",
  "attribute": "{system_editor}/core.create",
  "value": {
    "name": ".elf",
    "type": "directory",
    "owner": "{system_editor}",
    "contents": { "source": "outline" },
    "children": {},
    "metadata": { "description": "Elfiee system metadata directory" }
  }
}
```

2. **directory.write** — 写入目录骨架 entries
```json
{
  "entity": "{elf_block_id}",
  "attribute": "{system_editor}/directory.write",
  "value": {
    "contents": {
      "entries": {
        "agents/": {
          "id": "dir-{uuid}", "type": "directory",
          "source": "outline", "updated_at": "..."
        },
        "agents/elfiee-client/": {
          "id": "dir-{uuid}", "type": "directory",
          "source": "outline", "updated_at": "..."
        },
        "agents/elfiee-client/scripts/": {
          "id": "dir-{uuid}", "type": "directory",
          "source": "outline", "updated_at": "..."
        },
        "agents/elfiee-client/assets/": {
          "id": "dir-{uuid}", "type": "directory",
          "source": "outline", "updated_at": "..."
        },
        "agents/elfiee-client/references/": {
          "id": "dir-{uuid}", "type": "directory",
          "source": "outline", "updated_at": "..."
        },
        "session/": {
          "id": "dir-{uuid}", "type": "directory",
          "source": "outline", "updated_at": "..."
        },
        "git/": {
          "id": "dir-{uuid}", "type": "directory",
          "source": "outline", "updated_at": "..."
        }
      },
      "source": "outline"
    }
  }
}
```

3. **core.grant** — 授予所有人 directory.write 权限
```json
{
  "entity": "{elf_block_id}",
  "attribute": "{system_editor}/core.grant",
  "value": {
    "editor": "*",
    "capability": "directory.write",
    "block": "{elf_block_id}"
  }
}
```

**注意事项**:
- `source: "outline"` — 系统内部创建，非外部导入
- 虚拟目录的 `id` 使用 `"dir-{uuid}"` 格式（与 directory_import 一致），仅作标识，不对应实际 Block
- `.elf/` Block 的 owner 是 system editor，但通过 wildcard grant 所有人可写
- entries 中不包含文件条目，后续模块添加文件时更新 entries

### I10-01c: 修改 create_file 调用链

**文件**: `src-tauri/src/commands/file.rs`

**变更**: 在 `bootstrap_editors()` 之后增加 `bootstrap_elf_meta()` 调用。

```rust
pub async fn create_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    // ... 现有逻辑 ...
    bootstrap_editors(&file_id, &state).await?;
    bootstrap_elf_meta(&file_id, &state).await?;  // 新增
    Ok(file_id)
}
```

**仅 create_file 触发**，open_file 不触发（已有 .elf 文件已包含 .elf/ Block 的 events）。

## 实现方式

`bootstrap_elf_meta` 通过 Engine 的 `process_command` 发送命令序列，而不是直接构造 Event。这保证：
- 经过完整的 capability 授权检查
- vector clock 正确递增
- 快照自动生成
- events 正确持久化

```rust
async fn bootstrap_elf_meta(file_id: &str, state: &AppState) -> Result<(), String> {
    let handle = state.engine_manager.get_engine(file_id)
        .ok_or("File not open")?;
    let editor_id = state.get_active_editor(file_id)
        .ok_or("No active editor")?;

    // Step 1: core.create — 创建 .elf/ Dir Block
    let create_cmd = Command::new(
        editor_id.clone(),
        "core.create".to_string(),
        "".to_string(),  // core.create 不需要已有 block_id
        json!({
            "name": ".elf",
            "block_type": "directory",
            "source": "outline",
            "metadata": { "description": "Elfiee system metadata directory" }
        }),
    );
    let events = handle.process_command(create_cmd).await?;
    let elf_block_id = events.first()
        .ok_or("No event from core.create")?.entity.clone();

    // Step 2: directory.write — 写入目录骨架
    let write_cmd = Command::new(
        editor_id.clone(),
        "directory.write".to_string(),
        elf_block_id.clone(),
        build_elf_entries(),  // 构造 entries JSON
    );
    handle.process_command(write_cmd).await?;

    // Step 3: core.grant — 所有人可写
    let grant_cmd = Command::new(
        editor_id.clone(),
        "core.grant".to_string(),
        elf_block_id.clone(),
        json!({
            "target_editor": "*",
            "capability": "directory.write",
            "target_block": elf_block_id,
        }),
    );
    handle.process_command(grant_cmd).await?;

    Ok(())
}
```

`build_elf_entries()` 是纯函数，返回固定的目录骨架 JSON。可放在 `extensions/directory/elf_meta.rs` 中。

## 测试计划

### 单元测试

| 测试 | 位置 | 内容 |
|------|------|------|
| `test_wildcard_editor_grant` | `grants.rs` | `"*"` editor grant 匹配任意 editor_id |
| `test_wildcard_editor_revoke` | `grants.rs` | revoke `"*"` editor grant |
| `test_wildcard_editor_no_false_positive` | `grants.rs` | 精确 grant 不被 `"*"` 影响；`"*"` 不覆盖被 revoke 的精确 grant |
| `test_build_elf_entries` | `elf_meta.rs` | 验证 entries JSON 包含所有预期目录路径 |

### 集成测试

| 测试 | 位置 | 内容 |
|------|------|------|
| `test_create_file_has_elf_block` | `tests/elf_meta_integration.rs` | create_file 后能查到 `.elf/` Dir Block |
| `test_elf_block_entries_structure` | `tests/elf_meta_integration.rs` | `.elf/` Block 的 entries 包含所有目录路径 |
| `test_elf_block_wildcard_write` | `tests/elf_meta_integration.rs` | 非 owner editor 可对 `.elf/` 执行 directory.write |
| `test_open_file_no_duplicate_elf` | `tests/elf_meta_integration.rs` | open_file 不重复创建 `.elf/` Block |

## 与 v3 任务对照

| v3 编号 | v3 任务 | 对应实现 | 状态 |
|---------|---------|---------|------|
| I10-01 | .elf/ Dir Block 初始化 | I10-01a (grant 通配符) + I10-01b (初始化逻辑) + I10-01c (调用链) | 待实现 |

## 设计决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 权限方案 | editor_id `"*"` 通配符 (方案 A) | 对称设计，改动最小，语义清晰 |
| entries 内容 | 仅虚拟目录，无文件 Block | .elf/ 是骨架模板，内容由后续模块填充 |
| 触发时机 | 仅 create_file | open_file 已有 events，无需重复 |
| git/ 目录 | 预留 pre-commit/pre-push hook 模板 | 防止绕过 Elfiee 直接提交（3.5 Task Block 需要） |
| 实现方式 | 通过 process_command 而非直接构造 Event | 保证 capability 检查、vector clock、快照一致性 |

## 不做的事

- 不创建任何文件 Block（SKILL.md, mcp.json 等由 F7 创建）
- 不创建 git hook 内容（由 F16/F1 填充）
- 不修改 open_file 流程
- 不做 `.elf/` Block 的 UI 过滤（前端正常显示）

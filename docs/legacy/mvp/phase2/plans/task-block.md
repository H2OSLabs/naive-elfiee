# 3.5 Task Block 模块 — 开发计划

> 日期: 2026-01-30
> 预估总工时: 20 人时（Task extension 11h + git 工具 4h + 前端 3h + 测试 2h）
> 前置条件: Section 2 基础设施已完成（快照 ✅、Relation ✅、环检测 ✅、.elf/ ✅）
> 并行条件: 不依赖 3.1 Agent / 3.2 MCP / 3.3 Skills，可独立开发
> 开发工具: 使用 `elfiee-ext-gen` 生成扩展骨架，TDD 流程开发

## 一、现状确认

### 已完成的基础设施

| 组件 | 状态 | 位置 |
|------|------|------|
| Block 快照（write 时同步物理文件） | ✅ | `utils/snapshot.rs` + `engine/actor.rs:write_snapshots()` |
| RELATION_IMPLEMENT 常量 | ✅ | `models/block.rs:12` |
| DAG 环检测（DFS） | ✅ | `engine/actor.rs:220-253` (`check_link_cycle`) |
| 反向索引 parents HashMap | ✅ | `engine/state.rs:27` |
| .elf/ Dir Block 初始化 | ✅ | `extensions/directory/elf_meta.rs` |
| directory.export（授权 + 审计事件） | ✅ | `extensions/directory/directory_export.rs` |
| EngineHandle.process_command() | ✅ | `engine/actor.rs:531` |

### 需要新建

| 模块 | 说明 |
|------|------|
| `extensions/task/` | Task Block 的完整 extension（数据结构 + 4 个 Capabilities） |
| `utils/git.rs` | Git 命令执行工具（branch / add / commit） |
| `utils/git_hooks.rs` | Git hooks 注入/撤销（core.hooksPath 方案） |
| 前端 Task 区域 | Outline 区域增加 Task 子区域 |

## 二、开发流程：使用 elfiee-ext-gen

### 2.0 生成扩展骨架

```bash
# 在项目根目录执行
cd /home/yaosh/projects/elfiee

elfiee-ext-gen create \
  -n task \
  -b task \
  -c write,read,commit,archive
```

生成文件：
```
src-tauri/src/extensions/task/
├── mod.rs                  # Payload 结构（含 TODO 占位）
├── task_write.rs           # write handler（todo!()）
├── task_read.rs            # read handler（todo!()）
├── task_commit.rs          # commit handler（todo!()）
├── task_archive.rs         # archive handler（todo!()）
├── tests.rs                # 完整测试套件（payload + 功能 + 授权 + 工作流）
└── DEVELOPMENT_GUIDE.md    # TODO 清单
```

自动完成的注册：
- `extensions/mod.rs` → `pub mod task;`
- `capabilities/registry.rs` → 4 个 Capability 注册

### 2.1 TDD 开发循环

```bash
# Step 1: 查看开发指南，了解失败测试和下一步
elfiee-ext-gen guide task

# Step 2: 运行测试（会失败）
cd src-tauri && cargo test task::tests -- --nocapture

# Step 3: 按 TODO 实现 → 重新测试 → 循环
# ...

# Step 4: 验证结构和注册完整性
cd .. && elfiee-ext-gen validate task
```

## 三、Task Block 数据结构（F16-01，2h）

### 3.1 TaskContents — 无状态字段

**设计决策：去掉 TaskStatus，Phase 2 只存 title + description。**

理由：
- 显式状态需要回退机制（Committed → InProgress），额外引入 `task.reopen` capability
- 每个 capability 都需要前置状态检查，增加 handler 复杂度
- 状态变更散布在多个 event 中，重建逻辑复杂
- **Event Sourcing 天然有隐式状态**：有 `task.commit` event → 已提交；有 `task.archive` event → 已归档

```rust
// extensions/task/mod.rs（在 ext-gen 生成的 TODO 基础上填充）

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TaskWritePayload {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TaskCommitPayload {
    pub target_path: String,    // 外部项目路径（git repo 根目录）
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TaskArchivePayload {} // 无额外参数
```

Task Block 的 `contents` 结构：
```json
{
  "title": "实现登录功能",
  "description": "## 需求\n\n添加 OAuth 登录..."
}
```

状态推导规则（前端/查询层实现）：
- 无 `task.commit` event → **Pending**（可进一步细分：有 implement 下游 → InProgress）
- 有 `task.commit` event → **Committed**
- 有 `task.archive` event → **Archived**

### 3.2 快照格式

Task Block 快照写入 `block-{uuid}/body.md`：

```markdown
# {title}

{description}
```

快照通过 `utils/snapshot.rs` 的 `write_block_snapshot()` 自动处理。需要在该函数中添加 `"task"` block_type 分支。

### 3.3 注册点（ext-gen 自动处理）

1. `extensions/mod.rs` — `pub mod task;`（ext-gen 生成）
2. `capabilities/registry.rs` — 4 个 Capability 注册（ext-gen 生成）
3. `utils/snapshot.rs` — 手动添加 `"task"` block_type 分支

## 四、task.write / task.read（F16-02，3h）

### 4.1 task.write

```rust
// extensions/task/task_write.rs
#[capability(id = "task.write", target = "task")]
fn handle_task_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for task.write")?;
    if block.block_type != "task" {
        return Err(format!("Expected task block, got '{}'", block.block_type));
    }

    let payload: TaskWritePayload = serde_json::from_value(cmd.payload.clone())?;

    let mut new_contents = block.contents.clone();
    new_contents["title"] = json!(payload.title);
    new_contents["description"] = json!(payload.description);

    let mut new_metadata = block.metadata.clone();
    new_metadata.touch();

    let event = create_event(
        block.block_id.clone(),
        "task.write",
        json!({ "contents": new_contents, "metadata": new_metadata.to_json() }),
        &cmd.editor_id,
        1,
    );
    Ok(vec![event])
}
```

### 4.2 task.read

与 `markdown.read` 类似，返回空事件向量（仅权限检查）：

```rust
#[capability(id = "task.read", target = "task")]
fn handle_task_read(_cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let _block = block.ok_or("Block required for task.read")?;
    Ok(vec![])
}
```

### 4.3 测试计划（ext-gen 自动生成骨架）

- **Payload 测试**: TaskWritePayload JSON 反序列化
- **功能测试**: 写入 title + description → 验证 event value
- **授权测试**: owner 通过 / 非 owner 无 grant 拒绝 / 有 grant 通过
- **block_type 校验**: 对非 task 类型 block 调用 → 拒绝

## 五、task.commit（F16-03，5h）

### 5.1 核心流程

```
task.commit(target_path)
    │
    ├─ 1. 验证有 implement 下游 Blocks
    │     └─ block.children.get("implement") → Vec<block_id>
    │     └─ 无下游 → 拒绝
    │
    ├─ 2. 生成审计 Event（capability handler 层）
    │
    ├─ 3. 导出下游 Blocks 到外部项目（Tauri command 层）
    │     └─ 对每个 block_id：读取 block-{uuid}/body.* 快照
    │     └─ 复制到 target_path 下对应路径
    │
    ├─ 4. Git 操作（在 target_path 执行）
    │     ├─ git checkout -b feat/{title}（如分支已存在则 checkout）
    │     ├─ git add {exported_files}
    │     └─ git commit -m "{description}"（ELFIEE_TASK_COMMIT=1 放行 hook）
    │
    └─ 5. 返回结果（commit hash、分支名）
```

**无显式状态检查**：不阻止重复 commit。用户可以多次 commit 同一 task（追加修改），event history 自然记录每次 commit。

### 4.2 架构决策：Split Pattern

遵循 `directory.export` 的拆分模式：

- **Capability Handler** (`task_commit.rs`): 验证状态 + 查询下游 + 生成审计 Event
- **Tauri Command** (`commands/task.rs`): 执行实际 I/O（文件复制 + git 操作）

为什么拆分：
- Event log 保持纯净（无 I/O 副作用）
- git 操作需要 async + 文件系统访问，不适合在同步 handler 中执行
- 失败时可以只回滚 I/O 不影响 Event Store

### 5.3 Capability Handler

```rust
#[capability(id = "task.commit", target = "task")]
fn handle_task_commit(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for task.commit")?;
    if block.block_type != "task" {
        return Err(format!("Expected task block, got '{}'", block.block_type));
    }

    let payload: TaskCommitPayload = serde_json::from_value(cmd.payload.clone())?;

    // 查询 implement 下游（必须有关联 block 才能 commit）
    let downstream_ids = block.children
        .get(RELATION_IMPLEMENT)
        .cloned()
        .unwrap_or_default();
    if downstream_ids.is_empty() {
        return Err("No downstream blocks linked via 'implement' relation".to_string());
    }

    // 审计 event（不修改 contents，无状态变更）
    let event = create_event(
        block.block_id.clone(),
        "task.commit",
        json!({
            "target_path": payload.target_path,
            "downstream_block_ids": downstream_ids,
        }),
        &cmd.editor_id,
        1,
    );
    Ok(vec![event])
}
```

### 4.4 Tauri Command

```rust
// commands/task.rs（新建）
#[tauri::command]
#[specta]
pub async fn commit_task(
    state: State<'_, AppState>,
    file_id: String,
    task_block_id: String,
    target_path: String,
) -> Result<TaskCommitResult, String> {
    let handle = state.engine_manager.get_engine(&file_id)
        .ok_or("File not open")?;

    // 1. 调用 task.commit capability（验证 + 审计）
    let cmd = Command::new(editor_id, "task.commit".into(), task_block_id.clone(), json!({
        "target_path": target_path,
    }));
    let events = handle.process_command(cmd).await?;

    // 2. 从 event 中获取 downstream_block_ids
    let commit_event = &events[0];
    let downstream_ids: Vec<String> = serde_json::from_value(
        commit_event.value.get("downstream_block_ids").cloned().unwrap()
    )?;

    // 3. 复制快照文件到外部项目
    let task_block = handle.get_block(task_block_id).await.ok_or("Task not found")?;
    let title = task_block.contents.get("title").and_then(|v| v.as_str()).unwrap_or("untitled");
    let description = task_block.contents.get("description").and_then(|v| v.as_str()).unwrap_or("");

    for block_id in &downstream_ids {
        let block = handle.get_block(block_id.clone()).await
            .ok_or(format!("Block {} not found", block_id))?;
        copy_block_snapshot(&block, &target_path)?;
    }

    // 4. Git 操作
    let branch_name = format!("feat/{}", sanitize_branch_name(title));
    let commit_hash = git_commit_flow(&target_path, &branch_name, description).await?;

    Ok(TaskCommitResult { commit_hash, branch_name })
}
```

### 4.5 Git 工具函数

```rust
// utils/git.rs（新建）

/// 创建或切换分支 → add → commit 的完整流程
pub async fn git_commit_flow(
    repo_path: &str,
    branch_name: &str,
    message: &str,
) -> Result<String, String> {
    // 检查分支是否已存在
    let branch_exists = git_exec(repo_path, &["branch", "--list", branch_name], &[]).await?;
    if branch_exists.trim().is_empty() {
        git_exec(repo_path, &["checkout", "-b", branch_name], &[]).await?;
    } else {
        git_exec(repo_path, &["checkout", branch_name], &[]).await?;
    }

    // git add -A（添加所有变更）
    git_exec(repo_path, &["add", "-A"], &[]).await?;

    // git commit（设置 ELFIEE_TASK_COMMIT=1，让 hook 放行）
    git_exec(
        repo_path,
        &["commit", "-m", message],
        &[("ELFIEE_TASK_COMMIT", "1")],
    ).await?;

    // 获取 commit hash
    let hash = git_exec(repo_path, &["rev-parse", "HEAD"], &[]).await?;
    Ok(hash.trim().to_string())
}

/// 执行 git 命令的底层函数（支持环境变量注入）
async fn git_exec(
    repo_path: &str,
    args: &[&str],
    env: &[(&str, &str)],
) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.args(args).current_dir(repo_path);
    for (k, v) in env {
        cmd.env(k, v);
    }

    let output = cmd.output().await
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {} failed: {}", args[0], stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 清洗分支名（去掉非法字符）
pub fn sanitize_branch_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
        .to_lowercase()
}
```

### 4.6 测试计划

- **单元测试**:
  - task.commit handler：状态验证（Pending/InProgress 允许，Committed/Archived 拒绝）
  - task.commit handler：无下游 blocks → 拒绝
  - task.commit handler：有下游 blocks → 返回正确 event
  - `sanitize_branch_name` 各种输入边界
- **集成测试**（需要临时 git repo）:
  - 完整 commit 流程：create task → link blocks → commit → 验证 git log
  - 分支创建：验证 `feat/{title}` 分支存在
  - 重复 commit：已 committed 的 task 再 commit → 拒绝

## 六、task.archive（F16-04，3h）

### 6.1 核心流程

```
task.archive(task_block_id)
    │
    ├─ 1. 收集归档信息
    │     ├─ task title + description
    │     ├─ implement 下游 block 列表
    │     └─ 归档时间戳
    │
    ├─ 2. 写入归档元数据到 contents
    │     └─ 增加 archived_at 字段
    │
    └─ 3. 返回 Event
```

**无状态前置检查**：不要求必须先 commit 才能 archive。Event history 中是否有 task.commit event 可由查询层推导。

### 6.2 归档写入

归档信息写入 task block 自身的 contents（增加 `archived_at` 字段），快照自动更新：

```json
{
  "title": "实现登录功能",
  "description": "...",
  "archived_at": "2026-01-30T12:00:00Z"
}
```

Phase 3+ 可扩展为在 `.elf/Archives/` 下创建独立归档 Block。

### 6.3 测试计划

- archive → 生成归档事件，contents 包含 archived_at
- 对非 task block → 拒绝
- 授权测试（ext-gen 自动生成）

## 六、Git Hooks 注入机制（4h）

### 6.1 方案：core.hooksPath

**为什么选这个方案**：
- ✅ 不修改原有 `.git/hooks/` 目录
- ✅ 单行 config 变更，关闭时 unset 即可恢复
- ✅ 原有 hooks 关闭后自动生效
- ✅ 可在 Elfiee 中配置/调整

**工作流**：
```
导入外部项目（directory.import）
    │
    ├─ 检测 target_path/.git 是否存在
    │
    ├─ 存在 → 注入 hooks
    │     ├─ 1. 在 .elf/git/hooks/ 生成 pre-commit 脚本
    │     ├─ 2. git config --local core.hooksPath {elf_work_dir}/.elf/git/hooks/
    │     └─ 3. 记录原始 hooksPath（如果有）到 .elf/git/original-hooks-path
    │
    └─ 关闭 Elfiee（或 agent.disable）
          ├─ 如果有 original-hooks-path → 恢复
          └─ 否则 → git config --local --unset core.hooksPath
```

### 6.2 Hook 脚本内容（pre-commit）— 链式调用

**关键设计**：`core.hooksPath` 覆盖后，git 只看 `.elf/git/hooks/`，**原有 hooks 不再自动执行**。因此 Elfiee 的 hook 脚本必须**先链式调用原始 hook**，再执行自身逻辑。

```bash
#!/bin/sh
# Elfiee managed hook — chain to original, then check Elfiee workflow
# Auto-removed when Elfiee closes
# Bypass all hooks: git commit --no-verify

# ── Step 1: 链式调用原始 hook ──
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ORIGINAL_HOOKS_DIR=""

# 情况 A: 原项目设置了 core.hooksPath（如 husky → .husky/）
if [ -f "$SCRIPT_DIR/../original-hooks-path" ]; then
    ORIGINAL_HOOKS_DIR=$(cat "$SCRIPT_DIR/../original-hooks-path")
# 情况 B: 原项目用默认 .git/hooks/
elif [ -x ".git/hooks/pre-commit" ]; then
    ORIGINAL_HOOKS_DIR=".git/hooks"
fi

# 执行原始 hook（lint / format / test 等规则继续生效）
if [ -n "$ORIGINAL_HOOKS_DIR" ] && [ -x "$ORIGINAL_HOOKS_DIR/pre-commit" ]; then
    "$ORIGINAL_HOOKS_DIR/pre-commit" "$@"
    RESULT=$?
    if [ $RESULT -ne 0 ]; then
        exit $RESULT  # 原规则失败 → 直接拒绝，不到 Elfiee 检查
    fi
fi

# ── Step 2: Elfiee 工作流检查 ──
# 检查是否由 task.commit 发起（环境变量标记）
if [ "$ELFIEE_TASK_COMMIT" = "1" ]; then
    exit 0  # task.commit 流程，放行
fi

echo "[Elfiee] Direct commit detected outside Elfiee workflow."
echo "[Elfiee] Use task.commit in Elfiee for tracked commits."
echo "[Elfiee] Bypass: git commit --no-verify"
exit 1
```

**三个要点**：
1. **原项目规则完整保留** — lint / format / test hooks 先执行，失败则直接拒绝
2. **task.commit 流程放行** — git 操作时设置 `ELFIEE_TASK_COMMIT=1` 环境变量，hook 识别后放行
3. **建议性拦截** — 用户可用 `--no-verify` 绕过所有 hooks（包括原始和 Elfiee 的）

task.commit 中的 git 调用需要设置环境变量：

```rust
// utils/git.rs 中的 git_exec 需要支持环境变量
async fn git_exec_with_env(
    repo_path: &str,
    args: &[&str],
    env: &[(&str, &str)],
) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.args(args).current_dir(repo_path);
    for (k, v) in env {
        cmd.env(k, v);
    }
    // ...
}

// task.commit 调用时
git_exec_with_env(repo_path, &["commit", "-m", message], &[("ELFIEE_TASK_COMMIT", "1")]).await?;
```

### 6.3 工具函数

```rust
// utils/git_hooks.rs（新建）

/// 注入 git hooks（设置 core.hooksPath）
pub async fn inject_git_hooks(
    repo_path: &str,
    elf_hooks_dir: &str,
) -> Result<(), String> {
    // 检查是否有现存的 hooksPath 设置
    let existing = git_exec(repo_path, &["config", "--local", "--get", "core.hooksPath"]).await;
    if let Ok(ref path) = existing {
        let path = path.trim();
        if !path.is_empty() {
            // 保存原始路径
            std::fs::write(
                format!("{}/../original-hooks-path", elf_hooks_dir),
                path
            ).map_err(|e| format!("Failed to save original hooks path: {}", e))?;
        }
    }

    // 生成 pre-commit hook
    let hook_path = format!("{}/pre-commit", elf_hooks_dir);
    std::fs::write(&hook_path, PRE_COMMIT_HOOK_CONTENT)
        .map_err(|e| format!("Failed to write hook: {}", e))?;

    // 设置可执行权限（Unix）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set hook permissions: {}", e))?;
    }

    // 设置 core.hooksPath
    git_exec(repo_path, &["config", "--local", "core.hooksPath", elf_hooks_dir]).await?;

    Ok(())
}

/// 撤销 git hooks（恢复 core.hooksPath）
pub async fn remove_git_hooks(
    repo_path: &str,
    elf_hooks_dir: &str,
) -> Result<(), String> {
    let original_path_file = format!("{}/../original-hooks-path", elf_hooks_dir);

    if std::path::Path::new(&original_path_file).exists() {
        // 恢复原始 hooksPath
        let original = std::fs::read_to_string(&original_path_file)
            .map_err(|e| format!("Failed to read original hooks path: {}", e))?;
        git_exec(repo_path, &["config", "--local", "core.hooksPath", original.trim()]).await?;
        let _ = std::fs::remove_file(&original_path_file);
    } else {
        // 移除设置
        let _ = git_exec(repo_path, &["config", "--local", "--unset", "core.hooksPath"]).await;
    }

    Ok(())
}
```

### 6.4 注入/撤销时机

| 事件 | 动作 | 触发位置 |
|------|------|---------|
| `directory.import`（导入外部项目） | 检测 .git → 注入 hooks | `commands/file.rs` import 流程 |
| Elfiee 关闭 | 撤销所有已注入的 hooks | `lib.rs` Tauri app close handler |
| `agent.disable` | 撤销对应项目的 hooks | 3.1 Agent 模块（未来） |

**Phase 2 最小实现**：仅在 import 时注入、app 关闭时撤销。agent.enable/disable 集成留给 3.1。

### 6.5 测试计划

- 注入 hooks 后 `git config --get core.hooksPath` 返回正确路径
- 撤销后 `core.hooksPath` 被 unset
- 有原始 hooksPath → 撤销后恢复原始值
- Hook 脚本可执行且阻止直接 commit
- `--no-verify` 可绕过

## 七、前端 Task 区域（3h）

### 7.1 Outline 区域改造

当前 FilePanel 的 Outline 区域结构：
```
Outline [+]
├── {outline dir block 1}
│   └── VfsTree
├── {outline dir block 2}
│   └── VfsTree
```

改造后：
```
Outline [+]
├── {outline dir block 1}         ← source='outline', 非 task 相关
│   └── VfsTree
├── Tasks [+ New Task]            ← 新增子区域
│   ├── {task block 1}            ← block_type='task'
│   ├── {task block 2}
│   └── {task dir block}          ← 文件夹
```

### 7.2 具体改动

**FilePanel.tsx**：
1. 在 Outline 区域下方添加 "Tasks" 子区域
2. Tasks 区域显示所有 `block_type === 'task'` 的 blocks
3. Tasks 区域的 "+" 按钮默认创建 task block
4. 支持在 Tasks 区域创建文件夹（dir block）

**app-store.ts**：
1. 新增 `getTaskBlocks()` 方法，筛选 `block_type === 'task'`
2. Task block 不需要 `source` 字段区分（通过 `block_type` 识别）

**VfsTree.tsx**：
1. Task block 使用专属图标（如 CheckSquare）
2. Task block 显示状态标记（Pending/InProgress/Committed/Archived）

### 7.3 创建交互

| 区域 | "+" 行为 | 结果 |
|------|---------|------|
| Outline 顶层 "+" | 弹出对话框创建 outline dir block | 与现在一致 |
| Tasks 区域 "+" | 默认创建 task block | `core.create { block_type: "task" }` |
| Tasks 区域菜单 | "New Task" / "New Folder" | 支持两种 |
| 其他区域 | 按扩展名分类 | 与现在一致 |

## 八、任务执行顺序

```
Step 0: ext-gen 生成骨架（0.5h）
    │
    └── elfiee-ext-gen create -n task -b task -c write,read,commit,archive
    │
    ↓
Step 1: 填充 Payload + task.write + task.read（3h）
    │
    ├── mod.rs: 填充 TaskWritePayload / TaskCommitPayload / TaskArchivePayload
    ├── task_write.rs / task_read.rs: 实现 handler
    ├── snapshot.rs: 添加 "task" block_type 分支
    ├── cargo test task::tests -- --nocapture
    └── elfiee-ext-gen guide task → 验证进度
    │
    ↓（可并行）
Step 2a: Git 工具 + Hooks（4h）       Step 2b: task.archive（3h）
    │                                    │
    ├── utils/git.rs                     ├── task_archive.rs: 实现 handler
    ├── utils/git_hooks.rs               └── 测试
    └── 单元测试
    │
    ↓
Step 3: task.commit（5h）
    │
    ├── task_commit.rs: capability handler（审计 event）
    ├── commands/task.rs: Tauri command（I/O + git）
    ├── 注册 Tauri command 到 lib.rs
    └── 集成测试（临时 git repo）
    │
    ↓
Step 4: 前端 Task 区域（3h）
    │
    ├── FilePanel.tsx: 添加 Tasks 子区域
    ├── app-store.ts: 添加 getTaskBlocks()
    └── VfsTree.tsx: Task 图标
    │
    ↓
Step 5: 端到端验证 + validate（1.5h）
    │
    ├── elfiee-ext-gen validate task
    └── 完整流程：创建 Task → 写入 → link blocks → commit → archive
```

## 九、设计决策汇总

| 决策 | 选择 | 理由 |
|------|------|------|
| Task Block 类型 | 独立 `block_type: "task"` | 与 markdown/code 平级，有专属 capabilities |
| **无显式状态字段** | contents 只有 title + description | 避免状态回退复杂度，Event history 隐式推导状态 |
| task.commit 架构 | Split Pattern（handler + Tauri command） | 保持 event log 纯净，git I/O 在命令层 |
| Git 分支策略 | `feat/{sanitized_title}` | 固定命名，简单可预测 |
| Git hooks 注入 | `core.hooksPath` + 链式调用 | 不污染原项目 hooks，保留原有 lint/test 规则 |
| task.archive 实现 | 信息写入自身 contents | Phase 2 简化，不创建独立 Archive Block |
| 前端 Task 区域 | Outline 子区域 | 与 .elf/ meta 同层，不混入 Linked Repos |
| Hook 拦截方式 | 建议性（--no-verify 可绕过） | Phase 2 不做强制，避免影响用户习惯 |
| 开发工具 | elfiee-ext-gen + TDD | 生成骨架 + guide/validate 流程保证质量 |

## 十、不做的事

| 排除项 | 原因 |
|--------|------|
| **TaskStatus 显式状态字段** | 增加回退/状态机复杂度，Event history 可推导 |
| git push | Phase 2 不自动推送，用户手动 |
| 强制 git hook 拦截 | 建议性提示即可，保留用户灵活性 |
| 独立 Archive Block | 简化实现，归档信息在 task 自身 |
| Block 删除业务流程 | 用户明确说先不考虑 |
| agent.enable/disable 联动 | 属于 3.1 范围，此处只做 import 时注入 + app 关闭时撤销 |

## 十一、风险评估

| 风险 | 等级 | 缓解 |
|------|------|------|
| Git 命令执行失败（无 git、权限等） | 中 | task.commit 前检测 git 可用性，友好报错 |
| core.hooksPath 被用户手动修改 | 低 | 撤销时检查是否仍指向 .elf/git/hooks/ |
| 快照文件路径映射复杂 | 中 | 依赖现有 snapshot.rs 机制，只增加 task 分支 |
| 分支名冲突 | 低 | checkout 前检查分支是否存在，存在则切换 |

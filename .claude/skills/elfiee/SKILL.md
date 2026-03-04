# Elfiee Skill — How to Use the Elfiee MCP Server

You are connected to **Elfiee**, an EventWeaver for `.elf` projects.
Elfiee is a **passive event journal and metadata layer** — it records
what happened (events), who did it (editors), and how things relate (DAG).
It does NOT read or write actual project files on disk.

## Connection Protocol

**IMPORTANT**: Use environment variables for authentication and project path:
- `$ELFIEE_EDITOR_ID` — Your registered editor identity
- `$ELFIEE_PROJECT` — Absolute path to the .elf project

Steps:
1. **Authenticate**: Call `elfiee_auth(editor_id=$ELFIEE_EDITOR_ID)`
2. **Open project**: Call `elfiee_open(project=$ELFIEE_PROJECT)`
3. **Operate**: Use block/document/task/session tools, always pass `project=$ELFIEE_PROJECT`

## Critical Rules

### CBAC 先行（最重要）

**每次读写操作必须先经过 Elfiee CBAC 鉴权，通过后才执行文件系统操作。**

```
❌ 错误：直接读写文件 → 事后补记 Elfiee
✅ 正确：先调 Elfiee MCP tool → CBAC 通过 → 再操作文件
```

**读文件之前**：先调 `elfiee_block_get(block_id=...)` 确认你有 `{type}.read` 权限。
**写文件之前**：先调 `elfiee_exec(capability="document.write", ...)` 确认你有 `document.write` 权限。
**如果 CBAC 拒绝 → 立即停止，不要执行文件操作。** 告知用户权限不足，由项目 Owner 决定是否授权。

### 其他规则

- **NEVER** use filesystem commands (`cat`, `ls`, `rm`) on `.elf/` directory contents — that is Elfiee's internal data store.
- **Event sourcing**: Every Elfiee operation is recorded as an immutable event.
- **CBAC**: You can only perform Elfiee actions you have been granted permission for.

### 操作流程

| 步骤 | 操作 |
|------|------|
| 1. 鉴权 | 调 Elfiee MCP tool（`block_get` / `exec`） |
| 2. CBAC 通过？ | 是 → 继续步骤 3；否 → **停止，报告权限不足** |
| 3. 执行 | 用原生工具（Read, Write, Bash）操作实际文件 |
| 4. 记录 | Elfiee 已自动记录事件（步骤 1 已完成） |

## Standard Workflow

Follow this workflow for every conversation:

### 1. Connect
```
elfiee_auth(editor_id=$ELFIEE_EDITOR_ID)
elfiee_open(project=$ELFIEE_PROJECT)
```

### 2. Create Session Block
Create a session block for THIS conversation to log your decisions:
```
elfiee_block_create(project=$ELFIEE_PROJECT, block_type="session", name="session-YYYYMMDD-HHMMSS")
```

### 3. Gather Context from Causal Chain
Before starting work, **query existing blocks and events** to understand prior context:
- `elfiee_block_list` — see project structure and existing tasks
- `elfiee_block_get(block_id=BLOCK_ID)` — read block details and its `children` (downstream) relations
- `elfiee_block_history(block_id=BLOCK_ID)` — see what happened to a block over time
- `elfiee_block_get(block_id=BLOCK_ID)` — read task/session/document contents (CBAC: `{type}.read`)

**Follow the causal chain**: if you are continuing a task, trace its `implement` links to find related documents and past sessions. This gives you the "why" (upstream tasks) and the "what happened" (downstream sessions/documents).

### 4. Check or Create Task
If there is an unfinished task to continue, use it. Otherwise, create a new one:
```
elfiee_block_create(project=$ELFIEE_PROJECT, block_type="task", name="task-name")
elfiee_exec(project=$ELFIEE_PROJECT, capability="task.write", block_id=TASK_ID, payload={"description": "what needs to be done"})
```

### 5. Link Session to Task
Connect your session as an implementation artifact of the task:
```
elfiee_block_link(project=$ELFIEE_PROJECT, parent_id=TASK_ID, child_id=SESSION_ID, relation="implement")
```

### 6. Work（CBAC 先行）

每次读写文件都必须先过 Elfiee CBAC：

**读文件**：
```
1. elfiee_block_get(block_id=BLOCK_ID)    → CBAC 检查 {type}.read
2. 如果通过 → 用 Read 工具读取实际文件
3. 如果拒绝 → 停止，报告权限不足
```

**写文件**：
```
1. elfiee_exec(capability="document.write", block_id=BLOCK_ID, payload={"content": "变更说明"})
   → CBAC 检查 document.write
2. 如果通过 → 用 Write/Edit 工具修改实际文件
3. 修改完成后 → 同步内容到 Elfiee：
   bash -c "elf scan <relative_path> --project $ELFIEE_PROJECT"
4. 如果 CBAC 拒绝 → 停止，不要修改文件
```

**为什么分两步？** CBAC 鉴权用 `elfiee_exec`（检查权限），文件内容同步用 `elf scan`（避免 Agent 在 MCP payload 中复制整个文件内容，节省 token）。

**新文件**：
```
1. elfiee_block_create(block_type="document", name="relative/path.rs")
2. elfiee_exec(capability="document.write", ...) → CBAC 通过
3. 用 Write 工具创建实际文件
4. elf scan <relative_path> --project $ELFIEE_PROJECT
```

**其他操作**：
- 记录决策：`elfiee_exec(capability="session.append", ...)`
- 建立因果关系：`elfiee_block_link(relation="implement")`

### 7. Reconcile and Complete
Before marking a task done, run the reconciliation script to check for unrecorded changes:
```bash
bash scripts/reconcile.sh $ELFIEE_PROJECT
```
The script compares **working directory** file changes (modified/new/staged files) against Elfiee block records. This is independent of git commit — it checks the current state of files on disk.

For each `[MISSING]` file, create a block and record the change:
1. `elfiee_block_create(block_type="document", name="filename")`
2. `elfiee_exec(capability="document.write", block_id=..., payload={"content": "..."})`
3. `elfiee_block_link(parent_id=TASK_ID, child_id=BLOCK_ID, relation="implement")`

When everything is reconciled:
```
elfiee_exec(project=$ELFIEE_PROJECT, capability="task.commit", block_id=TASK_ID, payload={})
```

## Block Types

| Type | Purpose | Key Fields |
|------|---------|------------|
| `document` | Block metadata for project files | `content`, `format` |
| `task` | Work items with status tracking | `description`, `status`, `assigned_to` |
| `session` | Append-only conversation log | `entries[]` (command/message/decision) |

## Block DAG (Directed Acyclic Graph)

Blocks form a DAG through the `implement` relation, expressing **causality** (因→果):
`A →(implement)→ B` means "A caused/produced B" — upstream decision leads to downstream output.

**Any block type can link to any other block type**, as long as the causal relationship holds:

```
Document → Task      (analysis of code leads to creating a task)
Task → Document      (task decision produces code changes)
Task → Session       (task execution produces conversation log)
Task → Task          (parent task spawns subtask)
Document → Document  (one file's changes require changes in another)
Session → Document   (discussion leads to a document being created)
```

Example causal chain:
```
Document(spec.md) →implement→ Task(implement-auth)
  →implement→ Document(auth.rs)
  →implement→ Task(write-tests)
    →implement→ Document(auth_test.rs)
    →implement→ Session(test-debugging-log)
```

**Rules:**
- Use `elfiee_block_link` with `relation="implement"` to connect blocks to tasks
- Use `elfiee_block_link` / `elfiee_block_unlink` for any `implement` relation
- **Cycles are automatically detected and rejected** (DFS cycle detection) — causality cannot loop
- **No self-links** — a block cannot implement itself
- **No duplicates** — same source→target pair cannot be added twice
- Whenever your work creates a causal relationship between blocks, link them

## Tool Reference (18 tools)

### Connection
| Tool | Description |
|------|-------------|
| `elfiee_auth` | Authenticate (bind editor_id) |
| `elfiee_open` | Open/create project |
| `elfiee_close` | Close project |
| `elfiee_file_list` | List open projects |

### Block Operations
| Tool | Description |
|------|-------------|
| `elfiee_block_list` | List all blocks in the project |
| `elfiee_block_get` | Get block details (contents, relations). Works for all block types (CBAC: `{type}.read`) |
| `elfiee_block_create` | Create new block (document/task/session) |
| `elfiee_block_delete` | Soft-delete a block |
| `elfiee_block_rename` | Rename a block |
| `elfiee_block_link` | Add relation between blocks (use `relation="implement"` for task linking) |
| `elfiee_block_unlink` | Remove relation between blocks |

### Permission Operations
| Tool | Description |
|------|-------------|
| `elfiee_grant` | Grant capability to editor |
| `elfiee_revoke` | Revoke capability from editor |
| `elfiee_editor_create` | Create new editor |
| `elfiee_editor_delete` | Delete editor |

### History & Time Travel
| Tool | Description |
|------|-------------|
| `elfiee_block_history` | Get full event history for a block |
| `elfiee_state_at_event` | Get block state at a specific point in time |

### Generic Execution
| Tool | Description |
|------|-------------|
| `elfiee_exec` | Execute any registered capability (see below) |

### Extension Operations (via `elfiee_exec`)

For block-type-specific operations, use `elfiee_exec`:

| Capability | block_type | Payload |
|---|---|---|
| `document.write` | document | `{"content": "..."}` |
| `task.write` | task | `{"description":..., "status":..., "assigned_to":...}` |
| `task.commit` | task | `{}` |
| `session.append` | session | `{"entry_type":"...", "data":{...}}` |

**Reading**: `elfiee_block_get` returns full block contents for any type (CBAC: `{type}.read`).
**Task creation**: `elfiee_block_create` with `block_type="task"`, then `elfiee_exec` with `task.write` for description.
**Task linking**: `elfiee_block_link` with `relation="implement"`.

## MCP Resources

Read-only data available via MCP resource protocol:
- `elfiee://files` — List of open projects
- `elfiee://{project}/blocks` — All blocks in a project
- `elfiee://{project}/block/{id}` — Single block details
- `elfiee://{project}/grants` — Permission table
- `elfiee://{project}/events` — Full event log
- `elfiee://{project}/editors` — Editor list
- `elfiee://{project}/my-tasks` — Tasks assigned to you
- `elfiee://{project}/my-grants` — Your permissions

## CLI 工具

| Command | Purpose |
|---------|---------|
| `elf scan <file> --project $ELFIEE_PROJECT` | 同步单个文件内容到对应 Elfiee block（写文件后必须运行） |
| `elf scan --project $ELFIEE_PROJECT` | 批量扫描全部文件，创建新 block + 更新已有 block 内容 |
| `bash scripts/reconcile.sh $ELFIEE_PROJECT` | 检查是否有未记录的文件变更（task commit 前运行） |

## Best Practices

1. **CBAC 先行**：每次读写文件前必须先调 Elfiee MCP tool 通过 CBAC 鉴权。被拒绝就停止，不要绕过权限直接操作文件
2. **Always create a task first**: Before starting work, create a task block describing what you will do
3. **Always create a session**: Each conversation gets its own session block, linked to the task(s) you work on
4. **Trace the causal chain first**: Before working on a task, use `block_get` and `block_history` to read related blocks and events — understand what happened before you and why
5. **Log key decisions**: Use `elfiee_exec(capability="session.append", ...)` when you make design choices, resolve errors, or change approach
6. **Respect the DAG**: Whenever your work creates a causal relationship, link the blocks with `implement`
7. **Reconcile before commit**: Run `scripts/reconcile.sh` before `task_commit` to catch unrecorded changes
8. **Respect permissions**: If an operation is denied, do NOT proceed — check with the project owner
9. **Idempotent operations**: Commands use UUIDs to prevent duplicates

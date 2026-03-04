# pi-mono 架构分析

> 项目地址：https://github.com/badlogic/pi-mono
> 分析目的：提取架构灵感，指导 Elfiee 重构

---

## 一、项目定位

pi 是一个**开源 AI coding agent**，由 Mario Zechner（libGDX 作者）开发。与 Claude Code / Cursor 等闭源工具竞争，核心定位是**可扩展、可嵌入的编码助手**。

关键特点：
- TypeScript monorepo，分三层 package
- 完全开源，所有 LLM provider 可替换
- Extension API 让第三方可以拦截和增强 agent 行为
- 所有文件操作通过 Pluggable Operations 接口，支持本地/远程执行

---

## 二、架构总览

### 2.1 Monorepo 三层结构

```
pi-mono/
├── packages/ai/              # Layer 1: LLM 抽象层
│   └── @mariozechner/pi-ai
├── packages/agent/           # Layer 2: Agent Loop 抽象
│   └── @mariozechner/pi-agent-core
└── packages/coding-agent/    # Layer 3: Coding Agent 具体实现
    └── @mariozechner/pi-coding-agent
```

| Package | 职责 | 依赖 |
|---------|------|------|
| **pi-ai** | 多 provider 流式 LLM 调用（Anthropic, OpenAI, Google, etc.） | 无 |
| **pi-agent-core** | 通用 agent loop：消息处理、tool 执行、streaming | pi-ai |
| **pi-coding-agent** | 具体工具实现、Session 管理、Extension 系统、TUI | pi-agent-core |

**设计理念**：pi-agent-core 是一个不知道"编码"概念的通用 agent 循环。所有编码相关的逻辑（文件编辑、bash 执行、session 持久化）都在 pi-coding-agent 中。

### 2.2 内外系统边界

```
┌────────────────────────────────────────────────────┐
│  内部系统 (pi 进程内)                                │
│                                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐ │
│  │ Agent Loop   │  │ Session Mgr │  │ Extension    │ │
│  │ (消息循环)   │  │ (JSONL 树)  │  │ Runner       │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬───────┘ │
│         │                │                 │          │
│  ┌──────┴────────────────┴─────────────────┴───────┐ │
│  │              Tool Execution Layer                │ │
│  │  bash · read · write · edit · grep · find · ls  │ │
│  └──────────────────────┬──────────────────────────┘ │
│                         │                             │
│              ┌──────────┴──────────┐                  │
│              │ Pluggable Operations │ ← 注入点         │
│              └──────────┬──────────┘                  │
└─────────────────────────┼────────────────────────────┘
                          │
┌─────────────────────────┼────────────────────────────┐
│  外部系统                │                             │
│                         ▼                             │
│  ┌────────────┐  ┌──────────┐  ┌────────────┐        │
│  │ 本地文件系统 │  │ LLM API  │  │ 远程 SSH   │        │
│  └────────────┘  └──────────┘  └────────────┘        │
└──────────────────────────────────────────────────────┘
```

**边界定义清晰**：
- **内部**：Agent Loop + Session + Extensions + Tool definitions
- **外部**：文件系统、LLM API、远程系统
- **桥接层**：Pluggable Operations — 每个 tool 的实际 I/O 通过接口注入

---

## 三、Tool 执行流程

### 3.1 Tool 类型定义

```typescript
// packages/agent/src/types.ts
export interface AgentTool<TParameters extends TSchema = TSchema, TDetails = any> {
    name: string;
    label: string;
    description: string;
    parameters: TParameters;  // TypeBox schema，用于参数验证
    execute: (
        toolCallId: string,
        params: Static<TParameters>,
        signal?: AbortSignal,
        onUpdate?: AgentToolUpdateCallback<TDetails>,
    ) => Promise<AgentToolResult<TDetails>>;
}
```

每个 tool 自带 TypeBox schema，LLM 返回的参数会在执行前被 validate。

### 3.2 执行路径（串行，非并行）

```
LLM 返回 assistant message (含 tool_calls)
    │
    ▼
executeToolCalls()  [agent-loop.ts:294-378]
    │
    for 每个 tool_call（顺序执行）:
    │
    ├── 1. 查找 tool: tools.find(t => t.name === toolCall.name)
    ├── 2. 验证参数: validateToolArguments(tool, toolCall)
    ├── 3. Extension tool_call hook: emitToolCall()
    │       └── 如果返回 { block: true } → 抛异常，跳过执行
    ├── 4. 执行: tool.execute(toolCallId, validatedArgs, signal, onUpdate)
    ├── 5. Extension tool_result hook: emitToolResult()
    │       └── 可以修改返回内容（链式传递）
    ├── 6. 构建 ToolResultMessage
    └── 7. Steering 检查: getSteeringMessages()
            └── 如果用户发了新消息 → skip 剩余 tools → break
```

**关键发现**：
1. **Tool 是串行执行的**，不是并行。这保证了文件编辑的顺序一致性
2. **每个 tool 执行后检查 steering**，用户可以随时中断
3. **Extension 可以在执行前阻断（block）或执行后修改结果**

### 3.3 七个内置 Tool 及其 Operations 接口

| Tool | Operations 接口 | 方法 |
|------|-----------------|------|
| **bash** | `BashOperations` | `exec(command, cwd, options) → { exitCode }` |
| **read** | `ReadOperations` | `readFile(path) → Buffer` |
| **edit** | `EditOperations` | `readFile`, `writeFile`, `access` |
| **write** | `WriteOperations` | `writeFile`, `mkdir` |
| **grep** | `GrepOperations` | `exec` (ripgrep) |
| **find** | `FindOperations` | `exec` (find) |
| **ls** | `LsOperations` | `exec` (ls) |

每个 tool 通过工厂函数创建：
```typescript
createEditTool(cwd, { operations: customEditOps });
createWriteTool(cwd, { operations: customWriteOps });
createBashTool(cwd, { operations: customBashOps });
```

默认 operations 使用本地 fs/child_process，但可以替换为远程实现。

---

## 四、Session 管理

### 4.1 JSONL 文件格式

Session 文件存储在 `~/.pi/agent/sessions/--{encoded-cwd}--/` 目录下。
文件名格式：`{timestamp}_{uuid}.jsonl`。

每个文件是 append-only 的 JSONL（每行一个 JSON）：

```jsonl
{"type":"session","id":"...","timestamp":"...","cwd":"/path/to/project"}
{"type":"message","id":"e1","parentId":null,"message":{"role":"user","content":[...]}}
{"type":"message","id":"e2","parentId":"e1","message":{"role":"assistant","content":[...]}}
{"type":"message","id":"e3","parentId":"e2","message":{"role":"user","content":[...]}}
{"type":"compaction","id":"e4","parentId":"e3","summary":"...","firstKeptEntryId":"e2"}
```

第一行是 SessionHeader：
```typescript
interface SessionHeader {
    type: "session";
    version?: number;
    id: string;
    timestamp: string;
    cwd: string;
    parentSession?: string;
}
```

后续行的 entry 类型：
- `message` — 对话消息（user/assistant/toolResult）
- `compaction` — 压缩摘要（替代旧消息）
- `branch_summary` — 分支摘要
- `thinking_level_change` — 思考级别变更
- `model_change` — 模型切换
- `custom` / `custom_message` — Extension 注入的自定义内容
- `label` — 标签标记
- `session_info` — 元信息

### 4.2 树结构与分支

每个 entry 有 `id` 和 `parentId`，构成树结构。`leafId` 指向当前位置。

**Branch 操作极其简单**：
```typescript
branch(branchFromId: string): void {
    this.leafId = branchFromId;  // 只是移动指针
}
```

下次 `appendMessage()` 时，新 entry 的 `parentId` = `branchFromId`，自动形成分支。不需要复制数据，不需要创建新文件。

**获取当前分支路径**：
```typescript
getBranch(fromId?: string): SessionEntry[] {
    const path: SessionEntry[] = [];
    let current = this.byId.get(fromId ?? this.leafId);
    while (current) {
        path.unshift(current);
        current = current.parentId ? this.byId.get(current.parentId) : undefined;
    }
    return path;
}
```

从 leaf 沿 parentId 链向根回溯。

### 4.3 延迟写盘策略

```typescript
_persist(entry: SessionEntry): void {
    // 只在第一条 assistant 消息到来后才写盘
    const hasAssistant = this.fileEntries.some(
        e => e.type === "message" && e.message.role === "assistant"
    );
    if (!hasAssistant) {
        this.flushed = false;
        return;  // 暂不写盘
    }

    if (!this.flushed) {
        // 第一次写入：把所有 entries 一次性写入
        for (const e of this.fileEntries) {
            appendFileSync(this.sessionFile, `${JSON.stringify(e)}\n`);
        }
        this.flushed = true;
    } else {
        // 后续：增量 append
        appendFileSync(this.sessionFile, `${JSON.stringify(entry)}\n`);
    }
}
```

**设计意图**：避免创建空 session 文件。如果用户发了消息但 LLM 没回复（网络错误等），不会留下垃圾文件。

### 4.4 Compaction（上下文压缩）

当 context 接近 token 上限时触发压缩：

1. **findCutPoint()**：从最新消息向前回溯，保留最近 ~20000 token 的原始消息
2. **generateSummary()**：用 LLM 对被丢弃的消息生成结构化摘要
3. 追加文件操作历史（read/modified files 列表）
4. 写入 `compaction` entry 到 session JSONL

压缩后的 context 结构：
```
[compaction summary] + [保留的最近消息]
```

---

## 五、Extension 系统

### 5.1 注册方式

Extension 通过工厂函数注册：
```typescript
type ExtensionFactory = (pi: ExtensionAPI) => void | Promise<void>;

// 使用方式
function myExtension(pi: ExtensionAPI) {
    pi.on("tool_call", async (event, ctx) => {
        if (event.toolName === "bash" && event.input.command.includes("rm -rf")) {
            return { block: true, reason: "Dangerous command blocked" };
        }
    });

    pi.on("tool_result", async (event, ctx) => {
        // 可以修改 tool 返回结果
        return { content: [...event.content, extraInfo] };
    });
}
```

### 5.2 事件类型

| 事件 | 时机 | 能力 |
|------|------|------|
| `tool_call` | tool 执行前 | 可 block（阻断执行） |
| `tool_result` | tool 执行后 | 可修改返回内容 |
| `input` | 用户输入时 | 可转换输入 |
| `context` | LLM 调用前 | 可修改 context 消息列表 |
| `before_agent_start` | agent 开始前 | 可注入自定义消息 |

### 5.3 Extension Runner

```typescript
class ExtensionRunner {
    async emitToolCall(event: ToolCallEvent): Promise<ToolCallEventResult | undefined> {
        for (const ext of this.extensions) {
            const handlers = ext.handlers.get("tool_call");
            for (const handler of handlers) {
                const result = await handler(event, ctx);
                if (result?.block) {
                    return result;  // 立即返回，阻断执行
                }
            }
        }
        return result;
    }

    async emitToolResult(event: ToolResultEvent): Promise<ToolResultEventResult | undefined> {
        // 链式传递：每个 handler 可以修改 content/details
        // 不能 block，只能修改
    }
}
```

### 5.4 Tool Wrapping

Extension 通过 wrapping 注入到 tool 执行流程：
```typescript
function wrapToolWithExtensions(tool: AgentTool, runner: ExtensionRunner): AgentTool {
    return {
        ...tool,
        execute: async (toolCallId, params, signal, onUpdate) => {
            // 1. emitToolCall() — 可能 block
            // 2. tool.execute() — 实际执行
            // 3. emitToolResult() — 可能修改结果
        }
    };
}
```

---

## 六、Agent Loop

### 6.1 双层循环结构

```
外层 while(true):       // follow-up messages（用户在执行期间发送的新消息）
│
└── 内层 while(hasMoreToolCalls || pendingMessages.length > 0):
    │
    ├── 处理 pending messages（用户中断/steering）
    ├── streamAssistantResponse()
    │   ├── transformContext()      ← extension 可改消息
    │   ├── convertToLlm()         ← AgentMessage[] → Message[]
    │   ├── 构建 LLM context
    │   └── streamSimple()         ← 流式 LLM 调用
    ├── 如果有 tool calls:
    │   ├── executeToolCalls()     ← 串行执行
    │   └── tool results → context
    └── 检查 steering messages

    如果有 follow-up → continue 外层循环
    否则 break

emit agent_end
```

### 6.2 Context 构建三层转换

```
Session JSONL entries
    │
    ▼  buildSessionContext() [session-manager.ts]
    │  沿 parentId 链回溯 → 处理 compaction → 提取设置变更
    │
AgentMessage[]（包含 bashExecution, custom, branchSummary 等自定义类型）
    │
    ▼  convertToLlm() [messages.ts]
    │  bashExecution → user message（格式化命令+输出）
    │  branchSummary → user message（包裹 <summary> 标签）
    │  compactionSummary → user message（包裹 <summary> 标签）
    │  user/assistant/toolResult → 原样传递
    │
Message[]（标准 LLM 消息格式：user/assistant/toolResult）
    │
    ▼  构建 LLM context
    │  systemPrompt + messages + tools
    │
LLM API 调用
```

---

## 七、与文件系统的对接

### 7.1 Pluggable Operations 模式

pi 通过 Operations 接口抽象文件系统访问，**这是它最核心的架构创新之一**。

以 Edit Tool 为例：
```typescript
interface EditOperations {
    readFile: (absolutePath: string) => Promise<Buffer>;
    writeFile: (absolutePath: string, content: string) => Promise<void>;
    access: (absolutePath: string) => Promise<void>;
}

// 默认：本地文件系统
const defaultEditOperations: EditOperations = {
    readFile: (path) => fsReadFile(path),
    writeFile: (path, content) => fsWriteFile(path, content, "utf-8"),
    access: (path) => fsAccess(path, constants.R_OK | constants.W_OK),
};

// 自定义：SSH 远程
const sshEditOperations: EditOperations = {
    readFile: (path) => sshClient.readFile(path),
    writeFile: (path, content) => sshClient.writeFile(path, content),
    access: (path) => sshClient.access(path),
};
```

### 7.2 文件操作不走 Event 系统

pi **没有 event sourcing**。文件操作是直接修改——tool 执行后文件就变了，没有"事件记录 → 回放"的机制。Session JSONL 记录的是对话历史，不是文件变更历史。

这与 Elfiee 的核心理念不同：Elfiee 的每次操作都通过 Event 记录，文件状态可以从 Event 回放得到。

### 7.3 _block_dir 的类比

pi 的 tool 通过 `cwd` 参数确定工作目录，所有路径基于 cwd 解析：
```typescript
function resolveToCwd(path: string, cwd: string): string {
    return isAbsolute(path) ? path : resolve(cwd, path);
}
```

Elfiee 的 `_block_dir` 是类似概念——每个 Block 有自己的工作目录。但 pi 是全局一个 cwd，Elfiee 是 per-block cwd。

---

## 八、与 Agent 的对接

### 8.1 pi 自身就是 Agent

pi 不是一个"被 Agent 调用的工具"，它本身就是 Agent。它的对接模式：

```
用户 → TUI → AgentSession → Agent Loop → LLM API
                                 │
                                 ▼
                          Tool Execution → 文件系统
```

### 8.2 Extension 作为 Agent 扩展点

第三方可以通过 Extension 注入行为：
- 在 tool 执行前后拦截
- 修改 LLM context
- 注入自定义消息
- 添加自定义 tool

### 8.3 多 Provider 支持

pi-ai 抽象了 LLM provider：
```typescript
// 支持的 provider
Anthropic | OpenAI | Google | OpenRouter | VertexAI | ...
```

每个 provider 实现统一的流式接口，上层 agent 不关心具体用哪个 LLM。

---

## 九、对 Elfiee 的启发

### 9.1 Pluggable Operations — 高度相关 ⭐⭐⭐

**pi 的做法**：每个 tool 的 I/O 通过 Operations 接口注入。默认本地 fs，可替换为远程。

**Elfiee 可借鉴**：
- Extension handler 的纯函数约束可以通过类似模式实现
- `AgentContext` 可以实现为一组 Operations 接口（FileOps, BashOps, GitOps）
- MCP Tool 的实现可以同时记录 Event 和委托 I/O 给 Operations

```
MCP Tool: document.write
    ├── 1. 生成 Event（纯函数，Elfiee 内核）
    └── 2. 执行 I/O（委托给 Operations 接口）
         ├── 本地: fs.writeFile()
         └── 远程: ssh.writeFile() / OneSystem API
```

### 9.2 串行 Tool 执行 + Steering — 中度相关 ⭐⭐

**pi 的做法**：tool 串行执行，每个 tool 之间有 steering 检查点。

**Elfiee 可借鉴**：
- Elfiee 的 Engine Actor 已经是串行处理命令（mpsc channel）
- 但可以参考 steering 机制：在长操作中允许用户中断
- Agent 发送多个 Command 时，支持"中间插入"优先级更高的命令

### 9.3 Extension 事件拦截 — 中度相关 ⭐⭐

**pi 的做法**：Extension 可以 block tool_call、修改 tool_result。

**Elfiee 可借鉴**：
- CBAC certificator 已经是"执行前检查"的机制
- 但可以增加 Extension-level 的 pre/post hook
- 例如：agent.enable 前检查配置完整性，document.write 后触发 snapshot 更新

### 9.4 Session 树结构 — 低度相关 ⭐

**pi 的做法**：JSONL append-only 树，branch = 移动 leaf 指针。

**Elfiee 对比**：
- Elfiee 的 Event 已经是 append-only 的
- Session Block 用 `entries` 数组记录，用 append 模式的 Event
- Elfiee 不需要 pi 式的 branch（因为 Elfiee 的 Block DAG 本身就是多分支结构）

### 9.5 三层 Context 转换 — 低度相关 ⭐

**pi 的做法**：Session entries → AgentMessage[] → LLM Message[]

**Elfiee 对比**：
- Elfiee 不直接构建 LLM context（那是 Agent 的事）
- 但 Elfiee 的 MCP Tool 返回的数据可以参考这种分层：
  - Event 原始数据 → Block 投影状态 → MCP Tool response format

### 9.6 不值得借鉴的部分

| pi 的做法 | 为什么不适合 Elfiee |
|-----------|-------------------|
| 无 event sourcing，直接修改文件 | Elfiee 的核心是 event sourcing |
| 全局单 cwd | Elfiee 是 per-block 作用域 |
| JSONL 作为持久化格式 | Elfiee 用 SQLite (eventstore.db) |
| TUI 交互 | Elfiee 是 GUI (Tauri) |
| 自身是 agent | Elfiee 是被 agent 调用的编排工具 |

---

## 十、关键代码文件索引

| 文件 | 内容 |
|------|------|
| `packages/agent/src/types.ts` | AgentTool 类型定义 |
| `packages/agent/src/agent-loop.ts` | Agent Loop 核心循环 + executeToolCalls |
| `packages/coding-agent/src/core/session-manager.ts` | Session 管理（JSONL 读写、树结构、分支、compaction） |
| `packages/coding-agent/src/core/messages.ts` | AgentMessage 类型 + convertToLlm 转换 |
| `packages/coding-agent/src/core/extensions/runner.ts` | Extension Runner（事件分发） |
| `packages/coding-agent/src/core/extensions/wrapper.ts` | Tool wrapping（Extension 注入点） |
| `packages/coding-agent/src/core/extensions/types.ts` | Extension API 类型定义 |
| `packages/coding-agent/src/core/tools/edit.ts` | Edit Tool + EditOperations 接口 |
| `packages/coding-agent/src/core/tools/write.ts` | Write Tool + WriteOperations 接口 |
| `packages/coding-agent/src/core/tools/bash.ts` | Bash Tool + BashOperations 接口 |
| `packages/coding-agent/src/core/compaction/compaction.ts` | Context 压缩逻辑 |

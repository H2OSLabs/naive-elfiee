# entireio/cli 架构分析

> 项目地址：https://github.com/entireio/cli
> 分析目的：提取架构灵感，指导 Elfiee 重构

---

## 一、项目定位

Entire CLI **不是一个 coding agent**，而是一个 **AI agent 会话追踪/检查点（checkpoint）工具**。它通过 hook 机制挂入 AI coding agent（目前支持 Claude Code 和 Gemini CLI）的生命周期，在每次 git push 时捕获 AI 会话数据（transcript、prompt、修改的文件等），将会话记录索引到 git commit 旁边。

核心价值主张：**为 AI 辅助编程提供可审计的历史记录**——让团队知道代码是"怎么写出来的"。

关键特点：
- Go 语言，单二进制
- 不执行任何 AI 推理，只做记录和检查点管理
- 通过 Hook 被动接收事件，不主动调用 agent API
- 使用 Git 作为存储后端（shadow branches + orphan branch）

---

## 二、架构总览

### 2.1 技术栈

| 层面 | 技术 |
|------|------|
| 语言 | Go 1.25.x |
| CLI 框架 | `github.com/spf13/cobra` |
| TUI | `github.com/charmbracelet/huh` |
| Git 操作 | `github.com/go-git/go-git/v5`（附 git CLI fallback） |
| 构建 | mise |
| 遥测 | PostHog |

### 2.2 模块结构

```
cmd/entire/cli/
├── root.go                          # Cobra 根命令
├── hooks.go                         # Hook 处理器（共享逻辑）
├── hooks_claudecode_handlers.go     # Claude Code 专属 hook handler
├── hooks_geminicli_handlers.go      # Gemini CLI 专属 hook handler
├── setup.go / enable.go / disable.go  # 项目配置命令
├── rewind.go / resume.go           # 检查点回退/恢复
├── explain.go                       # 会话解释
│
├── agent/                           # Agent 抽象层
│   ├── agent.go                     # 核心 Agent 接口（14 方法）
│   ├── types.go                     # HookInput, SessionChange, TokenUsage
│   ├── registry.go                  # Agent 注册表（工厂模式）
│   ├── session.go                   # AgentSession 数据结构
│   ├── claudecode/                  # Claude Code 实现
│   │   ├── claude.go                # Agent 接口实现 + init() 自注册
│   │   ├── hooks.go                 # Hook 安装/卸载
│   │   ├── transcript.go            # JSONL transcript 解析
│   │   └── types.go                 # Claude 专有类型
│   └── geminicli/                   # Gemini CLI 实现
│
├── strategy/                        # 策略层
│   ├── strategy.go                  # Strategy 接口 + 9 个可选接口
│   ├── registry.go                  # Strategy 注册表
│   ├── manual_commit*.go            # manual-commit 策略（默认）
│   └── auto_commit.go               # auto-commit 策略
│
├── checkpoint/                      # 检查点存储
│   ├── checkpoint.go                # Store 接口 + 数据类型
│   ├── temporary.go                 # Shadow branch 操作
│   └── committed.go                 # Metadata branch 操作
│
├── session/                         # 会话状态管理
│   ├── state.go                     # StateStore + State 结构
│   └── phase.go                     # 会话阶段状态机（纯函数）
│
├── settings/                        # 配置管理
├── logging/                         # 结构化日志
└── paths/                           # 路径工具
```

### 2.3 内外系统边界

```
┌──────────────────────────────────────────────────────────┐
│  外部系统（Entire 不控制）                                  │
│                                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Claude Code   │  │ Gemini CLI   │  │ 用户 Git     │     │
│  │ (AI Agent)    │  │ (AI Agent)   │  │ 操作         │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
│         │ settings.json     │                 │ git hooks   │
│         │ hook events       │ hook events     │             │
└─────────┼──────────────────┼─────────────────┼─────────────┘
          │                  │                 │
          ▼                  ▼                 ▼
┌─────────────────────────────────────────────────────────────┐
│  Entire CLI 内部                                             │
│                                                               │
│  ┌─────────────────────────────────────────────────────┐     │
│  │                  Hook Handlers                       │     │
│  │  session-start · stop · prompt-submit · pre/post-task│     │
│  │  prepare-commit-msg · post-commit · pre-push         │     │
│  └─────────────────────────┬───────────────────────────┘     │
│                             │                                 │
│  ┌──────────┐  ┌───────────┴──────────┐  ┌──────────────┐   │
│  │  Agent    │  │    State Machine     │  │  Strategy    │   │
│  │  Layer    │  │ (Phase × Event →     │  │  Layer       │   │
│  │ (解析)    │  │  NewPhase + Actions) │  │ (存储)       │   │
│  └──────────┘  └──────────────────────┘  └──────────────┘   │
│                                                               │
│  ┌─────────────────────────────────────────────────────┐     │
│  │               Storage Layer                          │     │
│  │  .entire/ (配置)                                      │     │
│  │  .git/entire-sessions/ (活跃状态)                      │     │
│  │  entire/<hash>-<wt> (shadow branches，临时检查点)      │     │
│  │  entire/checkpoints/v1 (orphan branch，永久元数据)     │     │
│  └─────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────┘
```

**边界定义**：
- **外部**：AI Agent 进程、用户的 Git 操作
- **边界桥梁**：Hook 机制（安装到 agent 配置 + git hooks）
- **内部**：Hook 解析 → 状态机转移 → 策略执行 → 持久化

---

## 三、Agent 抽象层

### 3.1 核心 Agent 接口

```go
// agent/agent.go
type Agent interface {
    Name() AgentName                    // 注册表键（"claude-code"）
    Type() AgentType                    // 显示名（"Claude Code"）
    DetectPresence() (bool, error)      // 检测 agent 是否安装
    ParseHookInput(hookType HookType, reader io.Reader) (*HookInput, error)
    GetSessionDir(repoPath string) (string, error)
    ReadSession(input *HookInput) (*AgentSession, error)
    WriteSession(session *AgentSession) error
    FormatResumeCommand(sessionID string) string
}
```

### 3.2 可选接口（Interface Segregation）

除核心 Agent 接口外，还有多个可选接口。调用者通过 Go 的类型断言检查支持情况：

```go
// 可选接口定义
type HookSupport interface {
    InstallHooks(repoPath string) error
    UninstallHooks(repoPath string) error
    GetHookConfig() HookConfig
}

type HookHandler interface {
    GetHookTypes() []HookType
}

type FileWatcher interface {
    WatchForChanges(ctx context.Context, dir string) (<-chan FileEvent, error)
}

type TranscriptAnalyzer interface {
    AnalyzeTranscript(session *AgentSession) (*TranscriptAnalysis, error)
}

type TranscriptChunker interface {
    ChunkTranscript(session *AgentSession) ([]TranscriptChunk, error)
}
```

使用方式：
```go
if hookSupport, ok := agent.(HookSupport); ok {
    hookSupport.InstallHooks(repoPath)
}
```

**设计优势**：
- 核心接口极小（8 方法），新 agent 实现门槛低
- 可选能力按需提供，不强制实现
- 编译时类型安全

### 3.3 自注册工厂模式

```go
// agent/registry.go
var registry = make(map[AgentName]Factory)

func Register(name AgentName, factory Factory) {
    registry[name] = factory
}

func Get(name AgentName) (Agent, error) {
    factory, ok := registry[name]
    if !ok {
        return nil, fmt.Errorf("unknown agent: %s", name)
    }
    return factory()
}

// claudecode/claude.go — 每个实现在 init() 中自注册
func init() {
    agent.Register(agent.AgentNameClaudeCode, NewClaudeCodeAgent)
}
```

只要 import 包，就会自动注册。主程序不需要显式列出所有实现。

### 3.4 NativeData 设计

```go
type AgentSession struct {
    ID         string
    AgentName  AgentName
    RepoPath   string
    StartedAt  time.Time
    // ...
    NativeData interface{}  // agent 原生格式，不做跨 agent 转换
}
```

**务实选择**：不试图统一所有 agent 的 session 格式，保留原生数据。需要时由各 agent 实现自己的序列化。

---

## 四、Strategy 模式

### 4.1 核心 Strategy 接口

```go
// strategy/strategy.go
type Strategy interface {
    Name() string
    SaveChanges(ctx SaveContext) error
    SaveTaskCheckpoint(ctx TaskCheckpointContext) error
    GetRewindPoints(limit int) ([]RewindPoint, error)
    Rewind(point RewindPoint) error
    CanRewind() (bool, string, error)
    PreviewRewind(point RewindPoint) (*RewindPreview, error)
    PreviewSave(ctx PreviewContext) (*SavePreview, error)
    ApplyCheckpointMetadata(meta *checkpoint.CheckpointMetadata, ctx ApplyMetadataContext) error
    // ...共约 15 个核心方法
}
```

### 4.2 可选接口（9 个）

```go
type SessionInitializer interface { InitializeSession(ctx SessionContext) error }
type PrepareCommitMsgHandler interface { HandlePrepareCommitMsg(ctx CommitContext) (string, error) }
type PostCommitHandler interface { HandlePostCommit(ctx CommitContext) error }
type PrePushHandler interface { HandlePrePush(ctx PushContext) error }
type TurnEndHandler interface { HandleTurnEnd(ctx TurnContext) error }
type LogsOnlyRestorer interface { RestoreLogsOnly(point RewindPoint) error }
type SessionResetter interface { ResetSession(ctx ResetContext) error }
type SessionCondenser interface { CondenseSession(ctx CondenseContext) error }
type ConcurrentSessionChecker interface { CheckConcurrentSessions() ([]string, error) }
```

### 4.3 两种策略实现

| 策略 | 工作方式 | 适用场景 |
|------|----------|----------|
| **manual-commit**（默认） | 不修改用户分支，使用 shadow branch 存储中间状态。用户手动 commit 时附加元数据 | 希望保持 git history 干净 |
| **auto-commit** | 在用户分支上自动创建 commit，每次 agent turn 结束后保存 | 希望自动记录一切 |

---

## 五、Session 状态机（核心亮点）

### 5.1 阶段定义

```go
// session/phase.go
type Phase string

const (
    PhaseIdle             Phase = "idle"              // 无活跃会话
    PhaseActive           Phase = "active"            // 会话进行中
    PhaseActiveCommitted  Phase = "active_committed"  // 会话进行中，已有 commit
    PhaseEnded            Phase = "ended"             // 会话结束
)
```

### 5.2 事件定义

```go
type Event string

const (
    EventTurnStart    Event = "turn_start"     // Agent turn 开始
    EventTurnEnd      Event = "turn_end"       // Agent turn 结束
    EventGitCommit    Event = "git_commit"     // 用户执行 git commit
    EventSessionStart Event = "session_start"  // Agent session 开始
    EventSessionStop  Event = "session_stop"   // Agent session 结束
)
```

### 5.3 纯函数转移

```go
type TransitionResult struct {
    NewPhase Phase
    Actions  []Action
}

// 核心：纯函数，无副作用
func Transition(current Phase, event Event, ctx TransitionContext) TransitionResult {
    switch current {
    case PhaseIdle:
        switch event {
        case EventSessionStart:
            return TransitionResult{
                NewPhase: PhaseActive,
                Actions:  []Action{ActionUpdateLastInteraction},
            }
        case EventGitCommit:
            return TransitionResult{NewPhase: PhaseIdle, Actions: nil}
        // ...
        }
    case PhaseActive:
        switch event {
        case EventTurnEnd:
            return TransitionResult{
                NewPhase: PhaseActive,
                Actions:  []Action{ActionUpdateLastInteraction},
            }
        case EventGitCommit:
            return TransitionResult{
                NewPhase: PhaseActiveCommitted,
                Actions:  []Action{ActionMigrateShadowBranch},
            }
        case EventSessionStop:
            return TransitionResult{
                NewPhase: PhaseEnded,
                Actions:  []Action{ActionCondense},
            }
        // ...
        }
    // ...
    }
}
```

### 5.4 声明式 Actions

```go
type Action string

const (
    ActionCondense             Action = "condense"                // 压缩会话到永久存储
    ActionMigrateShadowBranch  Action = "migrate_shadow_branch"   // 迁移 shadow branch
    ActionWarnStaleSession     Action = "warn_stale_session"      // 警告过期会话
    ActionUpdateLastInteraction Action = "update_last_interaction" // 更新最后交互时间
)
```

状态机只返回"应该做什么"（Actions 列表），**不执行任何副作用**。由调用者根据 Actions 执行实际操作：

```go
func ApplyCommonActions(actions []Action, state *State) []Action {
    var remaining []Action
    for _, action := range actions {
        switch action {
        case ActionUpdateLastInteraction:
            state.LastInteraction = time.Now()
        default:
            remaining = append(remaining, action)
        }
    }
    return remaining  // 返回需要策略层处理的 actions
}
```

**设计优势**：
- **100% 可测试**：给定 (Phase, Event, Context) → 验证 (NewPhase, Actions)
- **无副作用**：状态机逻辑与 I/O 完全分离
- **可组合**：通用 Actions 由公共函数处理，策略特定 Actions 由策略自己处理

---

## 六、与文件系统的对接

### 6.1 自身文件操作

Entire 操作的文件全在 Git 仓库内部：

```
项目仓库/
├── .entire/              # 配置目录
│   ├── config.toml       # 项目配置
│   └── metadata/         # 活跃会话元数据
├── .git/
│   ├── entire-sessions/  # 活跃会话状态（JSON 文件）
│   │   └── <session-id>.json
│   └── hooks/            # Git hooks（由 Entire 安装）
│       ├── prepare-commit-msg
│       ├── post-commit
│       └── pre-push
└── ...（用户代码，Entire 不修改）
```

### 6.2 Git 作为存储后端

```
                ┌──────────────────────────┐
                │  Shadow Branches          │
                │  entire/<hash>-<wt>       │
                │  临时检查点（开发中）       │
                └────────────┬─────────────┘
                             │ condense
                             ▼
                ┌──────────────────────────┐
                │  entire/checkpoints/v1    │
                │  (orphan branch)          │
                │  永久元数据               │
                │  ├── <id[:2]>/            │ ← sharded 目录
                │  │   └── <id[2:]>/        │
                │  │       ├── metadata.json│
                │  │       ├── full.jsonl   │ ← 完整 transcript
                │  │       ├── prompt.txt   │
                │  │       └── context.md   │
                │  └── ...                  │
                └──────────────────────────┘
```

### 6.3 原子写入

状态文件使用 tmp + rename 模式，保证写入原子性：
```go
func (s *StateStore) Save(state *State) error {
    tmpFile := s.path + ".tmp"
    os.WriteFile(tmpFile, data, 0644)
    os.Rename(tmpFile, s.path)
}
```

### 6.4 不操作用户代码

Entire **不读写用户的源代码文件**（除了 rewind 时恢复快照）。它只操作自己的元数据文件。

---

## 七、与 Agent 的对接

### 7.1 Hook 机制（核心对接方式）

Entire 通过修改 Agent 的配置文件来安装 hook。以 Claude Code 为例：

```json
// .claude/settings.json（由 Entire 注入）
{
  "hooks": {
    "SessionStart": [{
      "type": "command",
      "command": "entire hooks session-start"
    }],
    "Stop": [{
      "type": "command",
      "command": "entire hooks stop"
    }],
    "UserPromptSubmit": [{
      "type": "command",
      "command": "entire hooks user-prompt-submit"
    }],
    "PreToolUse": [{
      "type": "command",
      "command": "entire hooks pre-task"
    }],
    "PostToolUse": [{
      "type": "command",
      "command": "entire hooks post-task"
    }]
  }
}
```

### 7.2 Hook 数据流

```
Claude Code 执行过程中触发 hook
    │
    ├── stdin: JSON 数据（会话 ID、工具名、参数等）
    │
    ▼
entire hooks <hook-type>
    │
    ├── 1. 解析 stdin JSON → HookInput
    ├── 2. agent.ReadSession(input) → AgentSession
    ├── 3. Transition(currentPhase, event) → (newPhase, actions)
    ├── 4. 执行 actions（保存状态、创建检查点等）
    └── 5. stdout: JSON 响应（可注入 systemMessage）
```

### 7.3 不使用 MCP

Entire **不是 MCP server/client**。它完全通过 hook（命令行调用）与 agent 交互。这种被动模式意味着：
- 不需要保持长连接
- 不需要端口管理
- 不需要认证
- 但也不能主动与 agent 交互

### 7.4 权限/安全

```json
// 通过 Claude Code 的 permissions.deny 防止 AI 读取内部数据
{
  "permissions": {
    "deny": [".entire/metadata/"]
  }
}
```

---

## 八、设计模式详解

### 8.1 Interface Segregation（接口隔离）

核心模式：1 个必须接口 + N 个可选接口

```
Agent 接口 (8 方法，必须)
    ├── HookSupport (可选)
    ├── HookHandler (可选)
    ├── FileWatcher (可选)
    ├── TranscriptAnalyzer (可选)
    └── TranscriptChunker (可选)

Strategy 接口 (~15 方法，必须)
    ├── SessionInitializer (可选)
    ├── PrepareCommitMsgHandler (可选)
    ├── PostCommitHandler (可选)
    ├── PrePushHandler (可选)
    ├── TurnEndHandler (可选)
    ├── LogsOnlyRestorer (可选)
    ├── SessionResetter (可选)
    ├── SessionCondenser (可选)
    └── ConcurrentSessionChecker (可选)
```

### 8.2 自注册工厂

```go
// 注册表
var agentRegistry = map[AgentName]AgentFactory{}
var strategyRegistry = map[string]StrategyFactory{}

// 每个实现在 init() 中自注册
// claudecode/claude.go
func init() {
    agent.Register("claude-code", NewClaudeCodeAgent)
}

// strategy/manual_commit.go
func init() {
    strategy.Register("manual-commit", NewManualCommitStrategy)
}
```

### 8.3 Context 传递

每种操作有专用的 Context struct，避免函数参数膨胀：

```go
type SaveContext struct {
    Session      *agent.AgentSession
    State        *session.State
    RepoPath     string
    CheckpointID string
    // ...
}

type TurnContext struct {
    Session   *agent.AgentSession
    State     *session.State
    TurnIndex int
    // ...
}
```

---

## 九、对 Elfiee 的启发

### 9.1 纯函数状态机 ⭐⭐⭐

**Entire 的做法**：`Transition(Phase, Event, Context) → (NewPhase, Actions)`

**Elfiee 可借鉴**：
- Engine 的命令处理流程可以拆为纯函数：
  ```
  ProcessCommand(State, Command) → (Events, Actions)
  ```
  其中 Events 是给 EventStore 的，Actions 是副作用（写 snapshot、通知前端）
- 测试变得极其简单：给定输入，验证输出，无需 mock I/O
- 当前 `process_command` 在 actor 内部直接执行副作用（persist、apply、notify），可以考虑分离

### 9.2 Interface Segregation（可选接口）⭐⭐⭐

**Entire 的做法**：核心接口最小化 + 可选接口按需实现

**Elfiee 可借鉴**：
- CapabilityHandler trait 目前是：`cap_id()`, `target()`, `handler()`, `certificator()`
- 可以增加可选 trait：
  ```rust
  trait BatchCapability { fn handle_batch(&self, cmds: &[Command]) -> Vec<Event>; }
  trait VersionedCapability { fn migrate(&self, old_version: u32) -> MigrationResult; }
  trait RenderableCapability { fn render_directive(&self, block: &Block) -> String; }  // MyST
  ```
- Rust 的 trait 本身就支持这种模式（`dyn CapabilityHandler + BatchCapability`）

### 9.3 自注册工厂 ⭐⭐

**Entire 的做法**：Go 的 `init()` + 全局 registry

**Elfiee 可借鉴**：
- 当前 `CapabilityRegistry::register_extensions()` 是手动列出所有 extension
- Rust 可以用 `inventory` 或 `linkme` crate 实现编译时自注册：
  ```rust
  #[inventory::submit]
  static MY_CAP: &dyn CapabilityHandler = &MyCapability;
  ```
- 或者用 Cargo feature flags 控制 extension 的编译包含

### 9.4 Context Struct 传递 ⭐⭐

**Entire 的做法**：专用 Context struct 替代大量函数参数

**Elfiee 可借鉴**：
- 当前 `handler(&cmd, Some(&block))` 参数较少
- 但 `process_command` 需要访问 state、grants、registry、event_store 等
- 可以定义 `CommandContext`：
  ```rust
  struct CommandContext<'a> {
      state: &'a StateProjector,
      grants: &'a GrantsTable,
      registry: &'a CapabilityRegistry,
      elf_path: &'a Path,
  }
  ```

### 9.5 NativeData 保留原生格式 ⭐⭐

**Entire 的做法**：`AgentSession.NativeData` 保留 agent 原生数据，不做跨 agent 转换

**Elfiee 可借鉴**：
- Session Block 的 entries 可以保留 Agent 原生的消息格式
- 不必定义统一的"AI 对话"Schema
- 查询时由前端/consumer 根据 agent type 自行解析

### 9.6 Hook-based 被动集成 ⭐

**Entire 的做法**：不主动连接 agent，而是安装 hook 被动接收事件

**Elfiee 对比**：
- Elfiee 的 Agent extension 也在 Claude Code 的 settings.json 中安装 hook/MCP
- 但 Elfiee 更进一步——通过 MCP 提供双向交互，不仅接收事件还能被调用
- Entire 的 hook 思路验证了"让工具适配 agent，而不是让 agent 适配工具"的原则

### 9.7 不值得借鉴的部分

| Entire 的做法 | 为什么不适合 Elfiee |
|---------------|-------------------|
| Git 作为存储后端 | Elfiee 用 SQLite (eventstore.db)，Git 只是 checkpoint 锚点 |
| 无运行时插件 | Elfiee 需要更灵活的 extension 机制 |
| 不使用 MCP | Elfiee 需要 MCP 来与 agent 双向交互 |
| 无 event sourcing | Elfiee 的核心就是 event sourcing |
| Go 的 init() 自注册 | Rust 没有 init() 机制，需要 inventory/linkme crate |

---

## 十、与 pi-mono 的对比

| 维度 | pi-mono | entireio/cli |
|------|---------|-------------|
| **定位** | AI coding agent | AI session tracker |
| **语言** | TypeScript | Go |
| **是否执行 AI 推理** | 是（调用 LLM API） | 否（只记录） |
| **文件系统操作** | 直接读写源代码 | 只操作自己的元数据 |
| **与 Agent 关系** | 自身就是 Agent | 挂在 Agent 旁边观察 |
| **扩展机制** | Extension API（运行时 hook） | 编译时注册（init()） |
| **Session 存储** | JSONL 文件 | Git branch |
| **状态管理** | Session tree（树结构） | Phase state machine（有限状态机） |
| **对 Elfiee 启发** | Pluggable Operations, Extension 拦截 | 纯函数状态机, Interface Segregation |

---

## 十一、关键代码文件索引

| 文件 | 内容 |
|------|------|
| `cli/agent/agent.go` | Agent 核心接口 + 可选接口定义 |
| `cli/agent/registry.go` | Agent 自注册工厂 |
| `cli/agent/claudecode/claude.go` | Claude Code Agent 实现 |
| `cli/agent/claudecode/hooks.go` | Claude Code hook 安装/卸载 |
| `cli/agent/claudecode/transcript.go` | JSONL transcript 解析 |
| `cli/strategy/strategy.go` | Strategy 核心接口 + 9 个可选接口 |
| `cli/strategy/registry.go` | Strategy 注册表 |
| `cli/strategy/manual_commit*.go` | manual-commit 策略实现 |
| `cli/session/phase.go` | **纯函数状态机**（Phase × Event → NewPhase + Actions） |
| `cli/session/state.go` | StateStore（JSON 文件持久化） |
| `cli/checkpoint/checkpoint.go` | CheckpointStore 接口 |
| `cli/checkpoint/temporary.go` | Shadow branch 操作 |
| `cli/checkpoint/committed.go` | Orphan branch 永久元数据 |
| `cli/hooks.go` | Hook 处理器入口 |

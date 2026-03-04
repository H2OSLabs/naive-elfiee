# Agent Building

This document covers how agents interact with Elfiee: the identity model, task lifecycle, session semantics, workflow templates, and the division of labor between Elfiee and the agent context.

## Editor Identity Model (Socialware)

In Elfiee, human users and AI agents are architecturally equal. Both are `Editor` entities:

```rust
struct Editor {
    editor_id: String,       // UUID
    name: String,            // "Alice" or "Claude Code"
    editor_type: EditorType, // Human | Bot
}
```

The `editor_type` field is informational only. CBAC treats both types identically -- permissions are granted per editor, not per type. This is the **Socialware principle**: tools should not privilege one participant type over another.

### Registration

Agents are registered via the CLI:

```bash
elf register claude --name "Claude Code" --config-dir ~/.claude
```

This creates an Editor (type: Bot), grants wildcard capabilities, injects MCP config, and copies the skill document.

### Authentication

Once the MCP server is running, agents authenticate per-connection:

```text
Agent -> elfiee_auth({ editor_id: "bot-001" })
      <- { editor: { ... }, skill: "..." }
```

The response includes the skill document -- an instruction set tailored to the agent's role.

## Task State Machine

Task blocks have an implicit state derived from event history (no explicit status field):

```text
  [Created]
      |
      | (task.write, core.link)
      v
  [In Progress]  -- has downstream blocks via "implement"
      |
      | (task.commit)
      v
  [Committed]    -- has task.commit event in history
```

### State Derivation Rules

| State | Condition |
|-------|-----------|
| Created | Block exists, no downstream links, no commit event |
| In Progress | Has `implement` children (linked code/document blocks) |
| Committed | Has at least one `task.commit` event in history |

Multiple commits are allowed. Each `task.commit` records a snapshot of the downstream block IDs at that point in time.

### Task Workflow Example

```text
1. Agent creates task:     core.create({ name: "Implement login", block_type: "task" })
2. Agent writes spec:      task.write({ content: "## Requirements\n..." })
3. Agent creates code:     core.create({ name: "auth.rs", block_type: "document" })
4. Agent links:            core.link({ relation: "implement", target_id: "doc-id" })
5. Agent writes code:      document.write({ content: "fn login() { ... }" })
6. Agent commits:          task.commit({})  -- records downstream IDs, triggers Git
```

## Session Append Semantics

Session blocks use append-only semantics. Entries are never modified or deleted, only added:

```text
session.append({
    entry: {
        role: "user",
        content: "Please review the authentication module.",
        timestamp: "2025-01-15T10:00:00Z"
    }
})
```

The session's `contents.entries` array grows monotonically. This provides:

- A complete audit trail of agent-human conversations.
- Natural ordering by append time.
- No conflict potential (appends never collide with each other).

## Workflow Templates (Socialware)

Workflows are declarative TOML files that define multi-agent collaboration:

```toml
[workflow]
name = "Code Review"
description = "Automated code review workflow"

[[participants]]
name = "reviewer"
type = "bot"
config_dir = "~/.claude"

[[participants]]
name = "developer"
type = "human"

[[tasks]]
name = "Review PR #42"
assignee = "reviewer"
grants = ["document.read", "session.append"]

[[tasks]]
name = "Fix Issues"
assignee = "developer"
grants = ["document.write", "task.commit"]
```

### What `elf run` Does

The `elf run <template>` command:

1. Parses the TOML workflow file.
2. Registers each participant as an Editor (if not already registered).
3. Creates task blocks for each defined task.
4. Grants specified capabilities to each participant on their assigned tasks.
5. Starts the MCP server.

This is purely a convenience script -- equivalent to running multiple `elf register`, `elf grant`, and `elf serve` commands manually.

## Skill System

Skills are markdown documents that instruct agents on how to use Elfiee's MCP tools. They are injected at registration time and returned during authentication.

### Skill Resolution

When an agent authenticates, the server resolves the skill document:

1. Check `{project}/.elf/templates/skills/{role}.md` -- role-specific skill.
2. Fall back to `{project}/.elf/templates/skills/default.md` -- project default.
3. Fall back to the built-in default skill (compiled into the binary).

### Skill Content

A typical skill document includes:

- Available MCP tools and their parameters.
- Block type schemas and content format.
- Workflow conventions (how to structure tasks, when to commit).
- Constraints (which operations are forbidden, which require approval).

## Elfiee vs AgentContext

Elfiee has a clear boundary of responsibility:

| Elfiee Does | Elfiee Does NOT Do |
|-------------|-------------------|
| Event sourcing (store, replay, project) | File system operations |
| CBAC (authorize, grant, revoke) | Git operations (beyond audit events) |
| Block DAG (create, link, traverse) | Terminal / shell execution |
| Declarative templates (parse, seed) | Agent orchestration |
| MCP server (expose tools, resources) | LLM API calls |
| Skill documents (store, serve) | Process spawning |

The agent's runtime context (file I/O, Git, terminal, LLM) is provided by the **AgentContext** -- an external system that the agent uses alongside Elfiee. Elfiee records *what happened* (events); the AgentContext does *the actual work*.

### Three-Layer Model

```text
  AgentChannel   -- Routing (which agent gets which message)
       |
     Agent       -- Elfiee (EventWeaver), Synnovator, Ezagent, etc.
       |
  AgentContext   -- OneSystem (file I/O, Git, terminal, LLM API)
```

Elfiee sits in the Agent layer as the EventWeaver. It does not reach into the AgentContext layer, and the AgentChannel layer does not reach into Elfiee.

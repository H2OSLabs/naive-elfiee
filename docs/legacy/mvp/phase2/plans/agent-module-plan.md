# Agent 模块开发计划

## 一、模块概述

### 1.1 模块定位

Agent 模块是 Phase 2 的核心功能模块，负责实现 AI 助手与 Elfiee 系统的深度集成。Agent 通过 LLM API 理解用户意图，生成 Proposal（命令提案），经用户审批后执行系统命令。

### 1.2 核心特性

- **Agent Block**: 可配置的 AI 助手实体，包含 provider、model、API key 等配置
- **LLM 调用**: 封装 Anthropic API，支持流式响应和结构化输出解析
- **Proposal 机制**: 将 LLM 输出转换为待审批的命令提案
- **上下文收集**: 基于 Block 关系图收集相关上下文
- **Token 截断**: 智能截断上下文以适应模型限制

### 1.3 对应用户故事

| 功能编号 | 功能名称 | 优先级 |
|---------|---------|-------|
| F3 | Agent Block 创建与配置 | P0 |
| F4 | LLM API 调用 | P0 |
| F5 | Proposal 机制 | P0 |
| F6 | 上下文收集与优化 | P0 |

---

## 二、目录结构

```
src-tauri/src/extensions/agent/
├── mod.rs                      # 模块入口，AgentConfig 结构体定义
├── agent_create.rs             # agent.create capability
├── agent_configure.rs          # agent.configure capability
├── agent_invoke.rs             # agent.invoke capability
├── agent_approve.rs            # agent.approve capability
├── llm/
│   ├── mod.rs                  # LLM 模块入口
│   ├── anthropic.rs            # Anthropic API 客户端
│   ├── parser.rs               # 结构化输出解析器
│   └── error.rs                # LLM 错误类型定义
├── context/
│   ├── mod.rs                  # 上下文模块入口
│   ├── collector.rs            # 上下文收集器
│   ├── truncator.rs            # Token 截断器
│   └── event_optimizer.rs      # 基于 Event 的上下文优化（M8 里程碑）
└── tests.rs                    # 单元测试
```

---

## 三、详细任务分解

### 3.1 Phase 1: 模块骨架（B-AGENT-01）

**预估工时**: 2 人时

#### 3.1.1 任务目标

创建 Agent 模块的基础结构，定义核心数据类型。

#### 3.1.2 实现内容

**文件**: `src-tauri/src/extensions/agent/mod.rs`

```rust
//! Agent Extension
//!
//! AI assistant integration for Elfiee.
//!
//! ## Architecture
//!
//! - `agent_create` - Create Agent Block with Editor
//! - `agent_configure` - Configure Agent settings
//! - `agent_invoke` - Invoke LLM and generate Proposal
//! - `agent_approve` - Approve and execute Proposal
//!
//! ## Payload Types
//!
//! - `AgentCreatePayload` - Parameters for agent.create
//! - `AgentConfigurePayload` - Parameters for agent.configure
//! - `AgentInvokePayload` - Parameters for agent.invoke
//! - `AgentApprovePayload` - Parameters for agent.approve

use serde::{Deserialize, Serialize};
use specta::Type;

pub mod agent_create;
pub mod agent_configure;
pub mod agent_invoke;
pub mod agent_approve;
pub mod llm;
pub mod context;

// --- Core Types ---

/// Agent configuration stored in Block.contents
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentConfig {
    /// Associated Editor ID for the agent (format: "agent-{uuid}")
    pub editor_id: String,
    /// LLM provider (e.g., "anthropic", "openai")
    pub provider: String,
    /// Model name (e.g., "claude-sonnet-4-20250514")
    pub model: String,
    /// Environment variable name for API key
    pub api_key_env: String,
    /// System prompt for the agent
    pub system_prompt: String,
}

/// A proposed command from LLM output
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProposedCommand {
    /// Capability ID (e.g., "code.write", "terminal.execute")
    pub cap_id: String,
    /// Target block ID
    pub block_id: String,
    /// Command payload
    pub payload: serde_json::Value,
    /// Human-readable description
    pub description: Option<String>,
}

/// Proposal status
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
}

// --- Payload Types ---

/// Payload for agent.create capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentCreatePayload {
    /// Agent display name
    pub name: String,
    /// LLM provider
    pub provider: String,
    /// Model name
    pub model: String,
    /// Environment variable name for API key
    pub api_key_env: String,
    /// System prompt
    pub system_prompt: Option<String>,
}

/// Payload for agent.configure capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentConfigurePayload {
    /// Optional: Update provider
    pub provider: Option<String>,
    /// Optional: Update model
    pub model: Option<String>,
    /// Optional: Update API key env var
    pub api_key_env: Option<String>,
    /// Optional: Update system prompt
    pub system_prompt: Option<String>,
}

/// Payload for agent.invoke capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentInvokePayload {
    /// User prompt/message
    pub prompt: String,
    /// Optional: Max tokens for context
    pub max_context_tokens: Option<u32>,
    /// Optional: Related block IDs for context
    pub context_block_ids: Option<Vec<String>>,
}

/// Payload for agent.approve capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentApprovePayload {
    /// Event ID of the Proposal to approve
    pub proposal_event_id: String,
    /// Approve or reject
    pub approved: bool,
}

#[cfg(test)]
mod tests;
```

#### 3.1.3 注册模块

**文件**: `src-tauri/src/extensions/mod.rs`

```rust
pub mod code;
pub mod directory;
pub mod markdown;
pub mod terminal;
pub mod agent;  // 新增
```

#### 3.1.4 验收标准

- [ ] `AgentConfig` 结构体编译通过
- [ ] `ProposedCommand` 结构体包含完整字段
- [ ] 所有 Payload 类型使用 `#[derive(Serialize, Deserialize, Type)]`
- [ ] 模块在 `extensions/mod.rs` 中注册

---

### 3.2 Phase 2: agent.create Capability（B-AGENT-02）

**预估工时**: 4 人时

#### 3.2.1 任务目标

实现 `agent.create` capability，创建 Agent Block 并自动生成关联的 Editor。

#### 3.2.2 实现内容

**文件**: `src-tauri/src/extensions/agent/agent_create.rs`

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;

use super::{AgentConfig, AgentCreatePayload};

/// Handler for agent.create capability.
///
/// Creates an Agent Block with the following behavior:
/// 1. Generates a unique editor_id for the agent (format: "agent-{uuid}")
/// 2. Creates the Agent Block with AgentConfig in contents
/// 3. Emits an editor.create event to create the associated Editor
///
/// # Payload
/// Uses `AgentCreatePayload` with name, provider, model, api_key_env, system_prompt.
#[capability(id = "agent.create", target = "*")]
fn handle_agent_create(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: AgentCreatePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for agent.create: {}", e))?;

    // Generate agent editor_id
    let agent_editor_id = format!("agent-{}", uuid::Uuid::new_v4());

    // Create AgentConfig
    let config = AgentConfig {
        editor_id: agent_editor_id.clone(),
        provider: payload.provider,
        model: payload.model,
        api_key_env: payload.api_key_env,
        system_prompt: payload.system_prompt.unwrap_or_default(),
    };

    // Generate block_id for the new Agent Block
    let block_id = format!("block-{}", uuid::Uuid::new_v4());

    // Event 1: Create Editor for the Agent
    let editor_event = create_event(
        agent_editor_id.clone(),
        "editor.create",
        serde_json::json!({
            "editor_id": agent_editor_id,
            "editor_type": "agent",
            "name": payload.name,
        }),
        &cmd.editor_id,
        1,
    );

    // Event 2: Create Agent Block
    let block_event = create_event(
        block_id.clone(),
        "agent.create",
        serde_json::json!({
            "block_id": block_id,
            "name": payload.name,
            "block_type": "agent",
            "owner": cmd.editor_id,
            "contents": config,
            "children": {},
            "metadata": {
                "created_at": crate::utils::time::now_utc(),
                "updated_at": crate::utils::time::now_utc(),
            }
        }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![editor_event, block_event])
}
```

#### 3.2.3 验收标准

- [ ] 创建 Agent Block 时自动生成 `editor_id = "agent-{uuid}"`
- [ ] 同时触发 `editor.create` 事件创建关联 Editor
- [ ] AgentConfig 正确存储在 Block.contents
- [ ] 单元测试覆盖正常和异常场景

---

### 3.3 Phase 3: agent.configure Capability（B-AGENT-03）

**预估工时**: 4 人时

#### 3.3.1 任务目标

实现 `agent.configure` capability，支持更新 Agent 配置，包括 API Key 环境变量验证。

#### 3.3.2 实现内容

**文件**: `src-tauri/src/extensions/agent/agent_configure.rs`

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;
use std::env;

use super::{AgentConfig, AgentConfigurePayload};

/// Handler for agent.configure capability.
///
/// Updates Agent configuration with validation:
/// - Validates API key environment variable exists (if provided)
/// - Partial updates supported (only provided fields are updated)
#[capability(id = "agent.configure", target = "agent")]
fn handle_agent_configure(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for agent.configure")?;

    let payload: AgentConfigurePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for agent.configure: {}", e))?;

    // Parse existing config
    let mut config: AgentConfig = serde_json::from_value(block.contents.clone())
        .map_err(|e| format!("Invalid AgentConfig in block: {}", e))?;

    // Validate API key env var if provided
    if let Some(ref api_key_env) = payload.api_key_env {
        if env::var(api_key_env).is_err() {
            return Err(format!(
                "Environment variable '{}' not found. Please set it before configuring.",
                api_key_env
            ).into());
        }
        config.api_key_env = api_key_env.clone();
    }

    // Apply partial updates
    if let Some(provider) = payload.provider {
        config.provider = provider;
    }
    if let Some(model) = payload.model {
        config.model = model;
    }
    if let Some(system_prompt) = payload.system_prompt {
        config.system_prompt = system_prompt;
    }

    // Update metadata
    let mut metadata = block.metadata.clone();
    metadata.touch();

    let event = create_event(
        block.block_id.clone(),
        "agent.configure",
        serde_json::json!({
            "contents": config,
            "metadata": metadata.to_json(),
        }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

#### 3.3.3 验收标准

- [ ] 环境变量验证：如果 `api_key_env` 对应的环境变量不存在，返回错误
- [ ] 支持部分更新（只更新提供的字段）
- [ ] `editor_id` 不可修改
- [ ] 更新 `metadata.updated_at`

---

### 3.4 Phase 4: LLM Client（B-AGENT-04）

**预估工时**: 8 人时

#### 3.4.1 任务目标

封装 Anthropic API 调用，支持流式响应。

#### 3.4.2 新增依赖

**文件**: `src-tauri/Cargo.toml`

```toml
[dependencies]
# ... existing dependencies ...
reqwest = { version = "0.11", features = ["json", "stream"] }
futures = "0.3"
tiktoken-rs = "0.5"  # Token 计算
```

#### 3.4.3 实现内容

**文件**: `src-tauri/src/extensions/agent/llm/mod.rs`

```rust
pub mod anthropic;
pub mod parser;
pub mod error;

pub use anthropic::AnthropicClient;
pub use parser::parse_structured_output;
pub use error::LlmError;
```

**文件**: `src-tauri/src/extensions/agent/llm/anthropic.rs`

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::extensions::agent::llm::error::LlmError;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: Option<String>,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

pub struct AnthropicClient {
    client: Client,
    api_key: String,
    model: String,
}

impl AnthropicClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    pub async fn send_message(
        &self,
        system_prompt: &str,
        user_message: &str,
        max_tokens: u32,
    ) -> Result<String, LlmError> {
        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens,
            system: Some(system_prompt.to_string()),
            messages: vec![Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        };

        let response = self.client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::Network(e.to_string()))?;

        if response.status() == 429 {
            return Err(LlmError::RateLimit);
        }

        if response.status() == 401 {
            return Err(LlmError::Unauthorized);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::InvalidResponse(error_text));
        }

        let api_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        // Extract text from response
        let text = api_response
            .content
            .into_iter()
            .filter_map(|block| {
                if block.content_type == "text" {
                    block.text
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(text)
    }
}
```

**文件**: `src-tauri/src/extensions/agent/llm/error.rs`

```rust
use std::fmt;

#[derive(Debug)]
pub enum LlmError {
    /// Network error during API call
    Network(String),
    /// Rate limit exceeded
    RateLimit,
    /// Invalid or unexpected response
    InvalidResponse(String),
    /// Unauthorized (invalid API key)
    Unauthorized,
    /// Parsing error
    ParseError(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::Network(msg) => write!(f, "Network error: {}", msg),
            LlmError::RateLimit => write!(f, "Rate limit exceeded"),
            LlmError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            LlmError::Unauthorized => write!(f, "Unauthorized: Invalid API key"),
            LlmError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for LlmError {}
```

#### 3.4.4 验收标准

- [ ] 成功调用 Anthropic API 并返回响应
- [ ] 正确处理 429 (RateLimit) 和 401 (Unauthorized) 错误
- [ ] 支持 system prompt 和 user message
- [ ] 集成测试使用 Mock 服务器

---

### 3.5 Phase 5: 结构化输出解析（B-AGENT-05）

**预估工时**: 4 人时

#### 3.5.1 任务目标

解析 LLM 返回的结构化输出，提取命令提案。

#### 3.5.2 设计说明

LLM 输出格式（XML 标签方式）：

```xml
<thinking>
用户想要创建一个新的 Markdown 文件...
</thinking>

<command>
  <cap_id>markdown.write</cap_id>
  <block_id>block-xxx</block_id>
  <payload>{"content": "# Hello World"}</payload>
  <description>创建新的 Markdown 内容</description>
</command>

<command>
  <cap_id>code.write</cap_id>
  <block_id>block-yyy</block_id>
  <payload>{"content": "fn main() {}", "language": "rust"}</payload>
  <description>添加 Rust 代码示例</description>
</command>
```

#### 3.5.3 实现内容

**文件**: `src-tauri/src/extensions/agent/llm/parser.rs`

```rust
use regex::Regex;
use crate::extensions::agent::ProposedCommand;
use crate::extensions::agent::llm::error::LlmError;

/// Parse structured output from LLM response.
///
/// Extracts <command> blocks and converts them to ProposedCommand structs.
pub fn parse_structured_output(response: &str) -> Result<Vec<ProposedCommand>, LlmError> {
    let command_regex = Regex::new(r"<command>([\s\S]*?)</command>")
        .map_err(|e| LlmError::ParseError(e.to_string()))?;

    let cap_id_regex = Regex::new(r"<cap_id>(.*?)</cap_id>")
        .map_err(|e| LlmError::ParseError(e.to_string()))?;
    let block_id_regex = Regex::new(r"<block_id>(.*?)</block_id>")
        .map_err(|e| LlmError::ParseError(e.to_string()))?;
    let payload_regex = Regex::new(r"<payload>([\s\S]*?)</payload>")
        .map_err(|e| LlmError::ParseError(e.to_string()))?;
    let desc_regex = Regex::new(r"<description>(.*?)</description>")
        .map_err(|e| LlmError::ParseError(e.to_string()))?;

    let mut commands = Vec::new();

    for cap in command_regex.captures_iter(response) {
        let command_block = &cap[1];

        let cap_id = cap_id_regex
            .captures(command_block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .ok_or_else(|| LlmError::ParseError("Missing cap_id".to_string()))?;

        let block_id = block_id_regex
            .captures(command_block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .ok_or_else(|| LlmError::ParseError("Missing block_id".to_string()))?;

        let payload_str = payload_regex
            .captures(command_block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim())
            .unwrap_or("{}");

        let payload: serde_json::Value = serde_json::from_str(payload_str)
            .map_err(|e| LlmError::ParseError(format!("Invalid payload JSON: {}", e)))?;

        let description = desc_regex
            .captures(command_block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string());

        commands.push(ProposedCommand {
            cap_id,
            block_id,
            payload,
            description,
        });
    }

    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_command() {
        let response = r#"
<thinking>I need to write markdown</thinking>

<command>
  <cap_id>markdown.write</cap_id>
  <block_id>block-123</block_id>
  <payload>{"content": "# Hello"}</payload>
  <description>Write heading</description>
</command>
"#;
        let commands = parse_structured_output(response).unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].cap_id, "markdown.write");
        assert_eq!(commands[0].block_id, "block-123");
    }

    #[test]
    fn test_parse_multiple_commands() {
        let response = r#"
<command>
  <cap_id>markdown.write</cap_id>
  <block_id>block-1</block_id>
  <payload>{}</payload>
</command>

<command>
  <cap_id>code.write</cap_id>
  <block_id>block-2</block_id>
  <payload>{"content": "code"}</payload>
</command>
"#;
        let commands = parse_structured_output(response).unwrap();
        assert_eq!(commands.len(), 2);
    }
}
```

#### 3.5.4 新增依赖

```toml
regex = "1.10"
```

#### 3.5.5 验收标准

- [ ] 正确解析单个 `<command>` 块
- [ ] 正确解析多个 `<command>` 块
- [ ] 处理无效 JSON payload 的错误
- [ ] 忽略 `<thinking>` 等非命令标签

---

### 3.6 Phase 6: API 错误处理（B-AGENT-06）

**预估工时**: 4 人时

**已在 Phase 4 中实现**（`llm/error.rs`）

补充重试逻辑：

**文件**: `src-tauri/src/extensions/agent/llm/anthropic.rs` (扩展)

```rust
impl AnthropicClient {
    /// Send message with retry logic
    pub async fn send_message_with_retry(
        &self,
        system_prompt: &str,
        user_message: &str,
        max_tokens: u32,
        max_retries: u32,
    ) -> Result<String, LlmError> {
        let mut last_error = LlmError::Network("No attempts made".to_string());

        for attempt in 0..max_retries {
            match self.send_message(system_prompt, user_message, max_tokens).await {
                Ok(response) => return Ok(response),
                Err(LlmError::RateLimit) => {
                    // Exponential backoff
                    let delay = 2u64.pow(attempt) * 1000;
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    last_error = LlmError::RateLimit;
                }
                Err(LlmError::Network(msg)) => {
                    // Retry on network errors
                    let delay = 1000 * (attempt as u64 + 1);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    last_error = LlmError::Network(msg);
                }
                Err(e) => return Err(e), // Don't retry other errors
            }
        }

        Err(last_error)
    }
}
```

---

### 3.7 Phase 7: Proposal Event 定义（B-AGENT-07）

**预估工时**: 4 人时

#### 3.7.1 任务目标

在 Event 的 value 字段中定义 Proposal 结构。

#### 3.7.2 设计说明

Proposal Event 存储在 `Event.value` 中：

```json
{
  "proposal": {
    "proposed_commands": [
      {
        "cap_id": "markdown.write",
        "block_id": "block-xxx",
        "payload": {"content": "..."},
        "description": "..."
      }
    ],
    "status": "pending",
    "prompt": "用户原始请求",
    "raw_response": "LLM 原始响应"
  }
}
```

#### 3.7.3 实现内容

在 `mod.rs` 中添加：

```rust
/// Proposal structure stored in Event.value
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Proposal {
    /// List of proposed commands
    pub proposed_commands: Vec<ProposedCommand>,
    /// Current status
    pub status: ProposalStatus,
    /// Original user prompt
    pub prompt: String,
    /// Raw LLM response (for debugging)
    pub raw_response: Option<String>,
}
```

---

### 3.8 Phase 8: agent.invoke Capability（B-AGENT-08）

**预估工时**: 5 人时

#### 3.8.1 任务目标

实现 `agent.invoke` capability，调用 LLM API，生成 Proposal Event。

#### 3.8.2 实现内容

**文件**: `src-tauri/src/extensions/agent/agent_invoke.rs`

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;
use std::env;

use super::{
    AgentConfig, AgentInvokePayload, Proposal, ProposalStatus,
    llm::{AnthropicClient, parse_structured_output},
    context::collect_context,
};

/// Handler for agent.invoke capability.
///
/// 1. Collects context from related blocks
/// 2. Calls LLM API with system prompt + context + user prompt
/// 3. Parses structured output to extract ProposedCommands
/// 4. Creates Proposal Event (status = pending)
///
/// NOTE: This handler is async and requires special handling in the engine.
#[capability(id = "agent.invoke", target = "agent")]
fn handle_agent_invoke(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for agent.invoke")?;

    let payload: AgentInvokePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for agent.invoke: {}", e))?;

    let config: AgentConfig = serde_json::from_value(block.contents.clone())
        .map_err(|e| format!("Invalid AgentConfig in block: {}", e))?;

    // Get API key from environment
    let api_key = env::var(&config.api_key_env)
        .map_err(|_| format!("Environment variable '{}' not set", config.api_key_env))?;

    // Note: Actual LLM call is async and handled differently
    // This is a placeholder - real implementation uses tokio::block_on or async handler

    // For now, create a "pending" proposal that will be filled in by async handler
    let proposal = Proposal {
        proposed_commands: vec![], // Will be filled by async handler
        status: ProposalStatus::Pending,
        prompt: payload.prompt,
        raw_response: None,
    };

    let event = create_event(
        block.block_id.clone(),
        "agent.invoke",
        serde_json::json!({
            "proposal": proposal,
        }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}
```

#### 3.8.3 异步调用说明

由于 capability handler 是同步的，实际的 LLM 调用需要通过以下方式处理：

1. **方案 A**: 创建专用 Tauri Command（类似 terminal 模块）
2. **方案 B**: 在 Engine Actor 中特殊处理 `agent.invoke`

推荐 **方案 A**，创建 `invoke_agent` Tauri Command。

---

### 3.9 Phase 9: agent.approve Capability（B-AGENT-09）

**预估工时**: 5 人时

#### 3.9.1 任务目标

实现 `agent.approve` capability，解析 Proposal 中的命令并执行。

#### 3.9.2 实现内容

**文件**: `src-tauri/src/extensions/agent/agent_approve.rs`

```rust
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;

use super::{AgentApprovePayload, Proposal, ProposalStatus};

/// Handler for agent.approve capability.
///
/// 1. Finds the Proposal Event by event_id
/// 2. If approved, creates events for each proposed command
/// 3. Updates Proposal status to "approved" or "rejected"
///
/// NOTE: Command execution is handled by the engine after approval.
#[capability(id = "agent.approve", target = "agent")]
fn handle_agent_approve(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for agent.approve")?;

    let payload: AgentApprovePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for agent.approve: {}", e))?;

    // Create status update event
    let new_status = if payload.approved {
        ProposalStatus::Approved
    } else {
        ProposalStatus::Rejected
    };

    let event = create_event(
        block.block_id.clone(),
        "agent.approve",
        serde_json::json!({
            "proposal_event_id": payload.proposal_event_id,
            "status": new_status,
            "approved_by": cmd.editor_id,
        }),
        &cmd.editor_id,
        1,
    );

    // Note: Actual command execution is handled separately by the engine
    // after this event is committed, based on the Proposal content

    Ok(vec![event])
}
```

---

### 3.10 Phase 10: 上下文收集器（B-AGENT-10）

**预估工时**: 6 人时

#### 3.10.1 任务目标

实现上下文收集器，遍历 Block 关系图收集相关内容。

#### 3.10.2 实现内容

**文件**: `src-tauri/src/extensions/agent/context/mod.rs`

```rust
pub mod collector;
pub mod truncator;

pub use collector::collect_context;
pub use truncator::truncate_context;
```

**文件**: `src-tauri/src/extensions/agent/context/collector.rs`

```rust
use crate::models::Block;
use std::collections::{HashMap, HashSet};

/// Context item for LLM
#[derive(Debug, Clone)]
pub struct ContextItem {
    pub block_id: String,
    pub block_type: String,
    pub name: String,
    pub content: String,
    pub depth: u32,
}

/// Collect context from block and its related blocks.
///
/// Traverses Block.children to find related blocks via "reference" relation.
///
/// # Arguments
/// * `block` - Starting block
/// * `blocks` - All blocks in the file (for lookup)
/// * `max_depth` - Maximum traversal depth
pub fn collect_context(
    block: &Block,
    blocks: &HashMap<String, Block>,
    max_depth: u32,
) -> Vec<ContextItem> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut context_items: Vec<ContextItem> = Vec::new();

    collect_recursive(block, blocks, 0, max_depth, &mut visited, &mut context_items);

    context_items
}

fn collect_recursive(
    block: &Block,
    blocks: &HashMap<String, Block>,
    current_depth: u32,
    max_depth: u32,
    visited: &mut HashSet<String>,
    context_items: &mut Vec<ContextItem>,
) {
    if current_depth > max_depth || visited.contains(&block.block_id) {
        return;
    }

    visited.insert(block.block_id.clone());

    // Extract content based on block type
    let content = extract_content(block);

    context_items.push(ContextItem {
        block_id: block.block_id.clone(),
        block_type: block.block_type.clone(),
        name: block.name.clone(),
        content,
        depth: current_depth,
    });

    // Traverse children with "reference" relation
    for (relation, child_ids) in &block.children {
        if relation == "reference" || relation == "implement" {
            for child_id in child_ids {
                if let Some(child_block) = blocks.get(child_id) {
                    collect_recursive(
                        child_block,
                        blocks,
                        current_depth + 1,
                        max_depth,
                        visited,
                        context_items,
                    );
                }
            }
        }
    }
}

fn extract_content(block: &Block) -> String {
    match block.block_type.as_str() {
        "markdown" => block
            .contents
            .get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "code" => block
            .contents
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => serde_json::to_string_pretty(&block.contents).unwrap_or_default(),
    }
}

/// Format context items as markdown for LLM
pub fn format_context_as_markdown(items: &[ContextItem]) -> String {
    let mut output = String::new();

    for item in items {
        output.push_str(&format!(
            "## {} ({})\n\n{}\n\n---\n\n",
            item.name, item.block_type, item.content
        ));
    }

    output
}
```

---

### 3.11 Phase 11: Token 截断器（B-AGENT-11）

**预估工时**: 3 人时

#### 3.11.1 实现内容

**文件**: `src-tauri/src/extensions/agent/context/truncator.rs`

```rust
use tiktoken_rs::cl100k_base;
use super::collector::ContextItem;

/// Truncate context to fit within token limit.
///
/// Strategy: Remove oldest (deepest) blocks first.
pub fn truncate_context(items: &[ContextItem], max_tokens: u32) -> Vec<ContextItem> {
    let bpe = cl100k_base().unwrap();

    // Sort by depth (ascending, keeping shallower items)
    let mut sorted_items: Vec<_> = items.to_vec();
    sorted_items.sort_by_key(|item| item.depth);

    let mut result: Vec<ContextItem> = Vec::new();
    let mut total_tokens: u32 = 0;

    for item in sorted_items {
        let item_tokens = bpe.encode_with_special_tokens(&item.content).len() as u32;

        if total_tokens + item_tokens <= max_tokens {
            total_tokens += item_tokens;
            result.push(item);
        } else {
            // Try to include partial content
            break;
        }
    }

    result
}

/// Count tokens in a string
pub fn count_tokens(text: &str) -> usize {
    let bpe = cl100k_base().unwrap();
    bpe.encode_with_special_tokens(text).len()
}
```

---

### 3.12 Phase 12: 基于 Event 的上下文优化（B-AGENT-12）

**预估工时**: 6 人时

**属于 M8 里程碑**，在基础功能完成后实现。

---

### 3.13 Phase 13: 注册 Agent Capabilities（B-AGENT-13）

**预估工时**: 2 人时

#### 3.13.1 更新 extensions/mod.rs

```rust
pub mod code;
pub mod directory;
pub mod markdown;
pub mod terminal;
pub mod agent;  // 新增
```

#### 3.13.2 更新 lib.rs

```rust
// 在 tauri_specta::collect_commands! 中添加（如果需要 Tauri Command）
// extensions::agent::invoke_agent,

// 在 .typ::<>() 中注册 Payload 类型
.typ::<extensions::agent::AgentCreatePayload>()
.typ::<extensions::agent::AgentConfigurePayload>()
.typ::<extensions::agent::AgentInvokePayload>()
.typ::<extensions::agent::AgentApprovePayload>()
.typ::<extensions::agent::AgentConfig>()
.typ::<extensions::agent::ProposedCommand>()
.typ::<extensions::agent::Proposal>()
```

#### 3.13.3 更新 capabilities/registry.rs

```rust
// 在 register_extensions() 中添加
use crate::extensions::agent::*;

self.register(Arc::new(AgentCreateCapability));
self.register(Arc::new(AgentConfigureCapability));
self.register(Arc::new(AgentInvokeCapability));
self.register(Arc::new(AgentApproveCapability));
```

---

## 四、依赖关系图

```
┌─────────────────────────────────────────────────────────────────┐
│                        Agent 模块依赖图                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  B-AGENT-01 (模块骨架)                                           │
│       │                                                         │
│       ├──────────────────────────┐                              │
│       │                          │                              │
│       v                          v                              │
│  B-AGENT-02              B-AGENT-04 (LLM Client)                │
│  (agent.create)                 │                               │
│       │                         ├──────────┐                    │
│       v                         │          │                    │
│  B-AGENT-03                     v          v                    │
│  (agent.configure)        B-AGENT-05  B-AGENT-06                │
│                           (Parser)    (Error)                   │
│                                 │                               │
│                                 └──────────┐                    │
│                                            │                    │
│  B-AGENT-07 (Proposal Event) ─────────────┤                    │
│                                            │                    │
│  B-AGENT-10 (Context Collector) ──────────┤                    │
│       │                                    │                    │
│       v                                    │                    │
│  B-AGENT-11 (Truncator) ─────────────────┬┘                    │
│                                          │                      │
│                                          v                      │
│                                   B-AGENT-08                    │
│                                   (agent.invoke)                │
│                                          │                      │
│                                          v                      │
│                                   B-AGENT-09                    │
│                                   (agent.approve)               │
│                                          │                      │
│                                          v                      │
│                                   B-AGENT-13                    │
│                                   (注册)                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 五、里程碑对照

| 里程碑 | Agent 模块任务 | 验收标准 |
|-------|---------------|---------|
| **M1** (Week 1 Day 1-3) | B-AGENT-01 ~ B-AGENT-04 | Agent Block 可创建，配置 API Key，`agent.invoke` 返回 LLM 回显 |
| **M3** (Week 2 Day 1-3) | B-AGENT-08 ~ B-AGENT-11 | Agent 可读取关联 Block 上下文，`agent.approve` 可执行 Commands |
| **M8** (Week 3 Day 5) | B-AGENT-12 | Token 截断策略基于 Event 逻辑关系 |

---

## 六、技术风险与降级策略

### 6.1 风险点

| 风险 | 影响 | 降级策略 |
|-----|------|---------|
| LLM API 不稳定 | agent.invoke 失败 | 优先实现 Mock 测试，延后真实 API |
| 流式响应复杂度 | 开发延期 | 先实现非流式调用 |
| 结构化输出不可靠 | 解析失败 | 增强错误恢复，返回原始响应 |

### 6.2 测试策略

- **单元测试**: 每个 capability 独立测试
- **Mock LLM**: 使用 `wiremock` 模拟 Anthropic API
- **集成测试**: 端到端 `agent.invoke` → Proposal → `agent.approve` 流程

---

## 七、开发检查清单

### 7.1 开发前

- [ ] 阅读 `EXTENSION_DEVELOPMENT.md`
- [ ] 确认 Cargo.toml 依赖
- [ ] 创建 `extensions/agent/` 目录结构

### 7.2 开发中

- [ ] 每个 Payload 类型使用 `#[derive(Serialize, Deserialize, Type)]`
- [ ] 运行 `cargo test` 确保编译通过
- [ ] 运行 `pnpm tauri dev` 生成 TypeScript bindings

### 7.3 开发后

- [ ] 在 `lib.rs` 中注册所有 Payload 类型
- [ ] 在 `registry.rs` 中注册所有 Capability
- [ ] 完成单元测试覆盖

---

## 八、附录：System Prompt 模板

```
你是 Elfiee 编辑器中的 AI 助手。你可以帮助用户编辑文档、编写代码和管理知识块。

当用户请求你执行操作时，你需要生成结构化的命令提案。使用以下格式：

<thinking>
分析用户请求，说明你的计划
</thinking>

<command>
  <cap_id>capability.name</cap_id>
  <block_id>目标 block 的 ID</block_id>
  <payload>{"key": "value"}</payload>
  <description>人类可读的描述</description>
</command>

可用的 capability 包括：
- markdown.write: 写入 Markdown 内容
- code.write: 写入代码
- core.create: 创建新 Block
- core.link: 链接 Block

请确保 payload 是有效的 JSON 格式。
```

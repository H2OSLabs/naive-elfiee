//! `elf run <template>` — 按 Socialware 模板注册角色并启动 MCP server
//!
//! 解析 TOML Socialware 模板，注册角色（Editor + Grants + MCP 配置），启动 MCP server。
//! Task/Session 创建等编排工作由外部 Coordinator 完成。

use crate::services;
use crate::state::AppState;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;

/// 内置 code-review 模板（编译进二进制）
pub const BUILTIN_CODE_REVIEW: &str = include_str!("../../templates/workflows/code-review.toml");

// ============================================================================
// Socialware 模板数据结构
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SocialwareTemplate {
    pub socialware: SocialwareInfo,
    pub roles: Vec<Role>,
    /// Passthrough: Elfiee 不解析，由 Coordinator 读取
    #[serde(default)]
    pub flows: Vec<toml::Value>,
    /// Passthrough: Elfiee 不解析，由 Coordinator 读取
    #[serde(default)]
    pub commitments: Vec<toml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SocialwareInfo {
    pub name: String,
    #[serde(default)]
    pub namespace: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct Role {
    pub id: String,
    pub agent_type: String,
    /// Agent 配置目录（可选，默认推断）
    pub config_dir: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// 细粒度 per-block 权限（可选）
    #[serde(default)]
    pub grants: Vec<TemplateGrantEntry>,
}

/// 模板中的细粒度权限条目
#[derive(Debug, Deserialize)]
pub struct TemplateGrantEntry {
    pub capability: String,
    /// Block name、id 或 "*"
    pub block: String,
}

// ============================================================================
// 执行逻辑
// ============================================================================

/// 执行 `elf run <template>`
///
/// Elfiee 只做两件事：注册角色 + 启动 MCP server。
/// Task/Session 创建等编排工作由 Coordinator 通过 MCP tools 完成。
pub async fn run(template_name: &str, project: &str, port: u16) -> Result<(), String> {
    let project_dir = Path::new(project);

    // 确保项目已初始化
    if !project_dir.join(".elf").exists() {
        super::init::run(project).await?;
    }

    let project_dir = crate::utils::safe_canonicalize(&project_dir)
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;
    let project_str = project_dir.to_str().unwrap();

    // 读取模板
    let template = load_template(template_name, &project_dir)?;

    println!(
        "Starting socialware: {} — {}",
        template.socialware.name, template.socialware.description
    );

    // 为每个 role 执行 register
    let mut editor_ids = Vec::new();
    for role in &template.roles {
        let caps: Vec<&str> = role.capabilities.iter().map(|s| s.as_str()).collect();

        let editor_id = if role.grants.is_empty() {
            // 仅 wildcard 权限
            super::register::run_with_caps(
                &role.agent_type,
                Some(&role.id),
                role.config_dir.as_deref(),
                project_str,
                &caps,
                port,
            )
            .await?
        } else {
            // wildcard + 细粒度权限
            let grant_entries: Vec<super::register::GrantEntry> = role
                .grants
                .iter()
                .map(|g| super::register::GrantEntry {
                    capability: g.capability.clone(),
                    block: g.block.clone(),
                })
                .collect();

            super::register::run_with_grants(
                &role.agent_type,
                Some(&role.id),
                role.config_dir.as_deref(),
                project_str,
                &caps,
                &grant_entries,
                port,
            )
            .await?
        };

        editor_ids.push((role.id.clone(), editor_id));
    }

    println!();
    println!("Socialware '{}' ready.", template.socialware.name);
    println!();

    // 输出 Agent 启动指令
    for (role_id, editor_id) in &editor_ids {
        println!("  Start {}: ELFIEE_EDITOR_ID={} claude", role_id, editor_id);
    }

    println!();
    println!("Starting MCP Server on port {}...", port);

    // 启动 MCP server
    let state = Arc::new(AppState::new());
    services::project::open_project(project_str, &state).await?;

    crate::mcp::start_mcp_server(state, port).await?;

    println!("Ready. MCP Server: http://127.0.0.1:{}", port);
    println!("Press Ctrl+C to stop.");

    tokio::signal::ctrl_c()
        .await
        .map_err(|e| format!("Failed to listen for ctrl-c: {}", e))?;

    println!("\nShutting down...");
    Ok(())
}

/// 加载 Socialware 模板
fn load_template(name: &str, project_dir: &Path) -> Result<SocialwareTemplate, String> {
    // 先查项目级模板
    let project_template_path = project_dir
        .join(".elf/templates/workflows")
        .join(format!("{}.toml", name));

    let content = if project_template_path.exists() {
        std::fs::read_to_string(&project_template_path)
            .map_err(|e| format!("Failed to read template: {}", e))?
    } else {
        // 尝试内置模板
        match name {
            "code-review" => BUILTIN_CODE_REVIEW.to_string(),
            _ => {
                return Err(format!(
                    "Template '{}' not found. \
                     Expected at: {} \
                     Built-in templates: code-review",
                    name,
                    project_template_path.display()
                ));
            }
        }
    };

    toml::from_str(&content).map_err(|e| format!("Failed to parse template: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_code_review_parses() {
        let template: SocialwareTemplate =
            toml::from_str(BUILTIN_CODE_REVIEW).expect("Failed to parse built-in template");

        assert_eq!(template.socialware.name, "Code Review");
        assert_eq!(template.socialware.namespace, "code-review");
        assert_eq!(template.roles.len(), 2);
        assert_eq!(template.roles[0].id, "coder");
        assert_eq!(template.roles[1].id, "reviewer");
        assert!(!template.roles[0].capabilities.is_empty());
    }

    #[test]
    fn test_load_template_builtin() {
        // 使用不存在的项目目录，应 fallback 到内置模板
        let result = load_template("code-review", Path::new("/nonexistent"));
        assert!(result.is_ok());
        let template = result.unwrap();
        assert_eq!(template.socialware.name, "Code Review");
    }

    #[test]
    fn test_load_template_not_found() {
        let result = load_template("nonexistent-template", Path::new("/nonexistent"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}

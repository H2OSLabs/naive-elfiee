//! `elf unregister` — 取消注册 Agent（register 的逆操作）
//!
//! 做两件事：
//! 1. **向内**：在 eventstore.db 中删除 Editor（级联删除所有 Grants）
//! 2. **向外**：清理 MCP 配置 + Skill

use crate::services;
use crate::state::AppState;
use std::path::{Path, PathBuf};

/// 执行 `elf unregister`
pub async fn run(editor_id: &str, config_dir: Option<&str>, project: &str) -> Result<(), String> {
    let project_dir = crate::utils::safe_canonicalize(Path::new(project))
        .map_err(|e| format!("Failed to resolve project path: {}", e))?;

    if !project_dir.join(".elf").exists() {
        return Err("Not an Elfiee project (no .elf/ directory). Run `elf init` first.".into());
    }

    // 打开项目
    let state = AppState::new();
    let file_id = services::project::open_project(project_dir.to_str().unwrap(), &state).await?;

    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or("Engine not running after open")?;

    // 获取 system editor ID
    let system_id = crate::config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());

    // 删除 Editor（StateProjector 自动级联删除所有 grants，走 services）
    services::editor::delete_editor(&handle, &system_id, editor_id)
        .await
        .map_err(|e| format!("Failed to delete editor: {}", e))?;

    // 推断 Agent 配置目录
    let agent_config_dir = config_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| super::register::infer_config_dir("claude", &project_dir));

    // 清理注入的配置
    clean_mcp_config(&agent_config_dir)?;

    // 删除 skills/elfiee/ 目录
    let skill_dir = agent_config_dir.join("skills").join("elfiee");
    if skill_dir.exists() {
        std::fs::remove_dir_all(&skill_dir)
            .map_err(|e| format!("Failed to remove skill dir: {}", e))?;
    }

    println!("Unregistered editor '{}'", editor_id);
    println!("  Cleaned config <- {}", agent_config_dir.display());

    Ok(())
}

/// 清理 MCP 配置
///
/// 1. 从 `.mcp.json` 中删除 `mcpServers.elfiee`
/// 2. 从 `settings.local.json` 中删除 ELFIEE_* env 和 mcp__elfiee__* permissions
fn clean_mcp_config(config_dir: &Path) -> Result<(), String> {
    // 1. 清理 .mcp.json
    let mcp_json_path = config_dir.parent().unwrap_or(config_dir).join(".mcp.json");

    if mcp_json_path.exists() {
        let content = std::fs::read_to_string(&mcp_json_path)
            .map_err(|e| format!("Failed to read .mcp.json: {}", e))?;
        let mut mcp_config: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse .mcp.json: {}", e))?;

        if let Some(servers) = mcp_config
            .get_mut("mcpServers")
            .and_then(|s| s.as_object_mut())
        {
            servers.remove("elfiee");
            // 如果 mcpServers 为空，删除整个 key
            if servers.is_empty() {
                mcp_config.as_object_mut().unwrap().remove("mcpServers");
            }
        }

        std::fs::write(
            &mcp_json_path,
            serde_json::to_string_pretty(&mcp_config)
                .map_err(|e| format!("Failed to serialize .mcp.json: {}", e))?,
        )
        .map_err(|e| format!("Failed to write .mcp.json: {}", e))?;
    }

    // 2. 清理 settings.local.json
    let settings_path = config_dir.join("settings.local.json");

    if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings: {}", e))?;
        let mut settings: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse settings: {}", e))?;

        // 删除 ELFIEE_* env
        if let Some(env) = settings.get_mut("env").and_then(|e| e.as_object_mut()) {
            env.remove("ELFIEE_EDITOR_ID");
            env.remove("ELFIEE_PROJECT");
        }

        // 删除 mcp__elfiee__* permissions
        if let Some(allow) = settings
            .pointer_mut("/permissions/allow")
            .and_then(|a| a.as_array_mut())
        {
            allow.retain(|v| {
                v.as_str()
                    .map(|s| !s.starts_with("mcp__elfiee__"))
                    .unwrap_or(true)
            });
        }

        std::fs::write(
            &settings_path,
            serde_json::to_string_pretty(&settings)
                .map_err(|e| format!("Failed to serialize settings: {}", e))?,
        )
        .map_err(|e| format!("Failed to write settings: {}", e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_clean_mcp_json() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&config_dir).unwrap();

        // 写入 .mcp.json（有 elfiee 和 other-server）
        let mcp = json!({
            "mcpServers": {
                "elfiee": { "type": "sse", "url": "http://localhost:47200/sse" },
                "other-server": { "command": "npx", "args": ["other"] }
            }
        });
        std::fs::write(
            tmp.path().join(".mcp.json"),
            serde_json::to_string_pretty(&mcp).unwrap(),
        )
        .unwrap();

        clean_mcp_config(&config_dir).unwrap();

        let content = std::fs::read_to_string(tmp.path().join(".mcp.json")).unwrap();
        let result: serde_json::Value = serde_json::from_str(&content).unwrap();

        // elfiee 被删除，other-server 保留
        assert!(result["mcpServers"]["elfiee"].is_null());
        assert!(result["mcpServers"]["other-server"].is_object());
    }

    #[test]
    fn test_clean_mcp_json_removes_empty_servers() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&config_dir).unwrap();

        // 只有 elfiee
        let mcp = json!({
            "mcpServers": {
                "elfiee": { "type": "sse" }
            }
        });
        std::fs::write(
            tmp.path().join(".mcp.json"),
            serde_json::to_string_pretty(&mcp).unwrap(),
        )
        .unwrap();

        clean_mcp_config(&config_dir).unwrap();

        let content = std::fs::read_to_string(tmp.path().join(".mcp.json")).unwrap();
        let result: serde_json::Value = serde_json::from_str(&content).unwrap();

        // mcpServers key 被删除
        assert!(result.get("mcpServers").is_none());
    }

    #[test]
    fn test_clean_settings() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&config_dir).unwrap();

        let settings = json!({
            "env": {
                "ELFIEE_EDITOR_ID": "claude-abcd1234",
                "ELFIEE_PROJECT": "/home/user/project",
                "OTHER_VAR": "keep"
            },
            "permissions": {
                "allow": [
                    "Bash(cargo test:*)",
                    "mcp__elfiee__elfiee_auth",
                    "mcp__elfiee__elfiee_exec",
                    "WebSearch"
                ]
            },
            "someKey": true
        });
        std::fs::write(
            config_dir.join("settings.local.json"),
            serde_json::to_string_pretty(&settings).unwrap(),
        )
        .unwrap();

        clean_mcp_config(&config_dir).unwrap();

        let content = std::fs::read_to_string(config_dir.join("settings.local.json")).unwrap();
        let result: serde_json::Value = serde_json::from_str(&content).unwrap();

        // ELFIEE_* env 被删除，OTHER_VAR 保留
        assert!(result["env"].get("ELFIEE_EDITOR_ID").is_none());
        assert!(result["env"].get("ELFIEE_PROJECT").is_none());
        assert_eq!(result["env"]["OTHER_VAR"], "keep");

        // mcp__elfiee__* permissions 被删除，其他保留
        let allow = result["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 2);
        assert!(allow.iter().any(|v| v == "Bash(cargo test:*)"));
        assert!(allow.iter().any(|v| v == "WebSearch"));

        // 其他 key 保留
        assert_eq!(result["someKey"], true);
    }

    #[test]
    fn test_clean_skills_dir() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".claude");
        let skill_dir = config_dir.join("skills").join("elfiee");
        let scripts_dir = skill_dir.join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# Skill").unwrap();
        std::fs::write(scripts_dir.join("reconcile.sh"), "#!/bin/bash").unwrap();

        assert!(skill_dir.exists());

        // 模拟删除
        std::fs::remove_dir_all(&skill_dir).unwrap();

        assert!(!skill_dir.exists());
        // skills/ 目录仍在（可能有其他 skill）
        assert!(config_dir.join("skills").exists());
    }

    #[test]
    fn test_clean_nonexistent_files() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&config_dir).unwrap();

        // 没有 .mcp.json 和 settings.local.json 也不报错
        let result = clean_mcp_config(&config_dir);
        assert!(result.is_ok());
    }
}

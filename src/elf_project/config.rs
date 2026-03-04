/// 项目级配置（.elf/config.toml）
///
/// 只存储静态项目信息。Editor 身份从全局配置（~/.elf/config.json）读取，
/// 权限从 Event 投影。类似 Git：~/.gitconfig 存全局身份，.git/config 只存项目配置。
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 项目配置结构，对应 .elf/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectInfo,
    pub extensions: ExtensionsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// 项目显示名称
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionsConfig {
    /// 启用的 Extension 列表
    pub enabled: Vec<String>,
}

impl ProjectConfig {
    /// 创建默认项目配置
    pub fn new(project_name: &str) -> Self {
        Self {
            project: ProjectInfo {
                name: project_name.to_string(),
            },
            extensions: ExtensionsConfig {
                enabled: vec![
                    "document".to_string(),
                    "task".to_string(),
                    "session".to_string(),
                ],
            },
        }
    }

    /// 从 .elf/config.toml 加载配置
    pub fn load(config_path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read config.toml: {}", e))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse config.toml: {}", e))
    }

    /// 保存配置到 .elf/config.toml
    pub fn save(&self, config_path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(config_path, content)
            .map_err(|e| format!("Failed to write config.toml: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_project_config_new() {
        let config = ProjectConfig::new("my-project");
        assert_eq!(config.project.name, "my-project");
        assert_eq!(
            config.extensions.enabled,
            vec!["document", "task", "session"]
        );
    }

    #[test]
    fn test_project_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let original = ProjectConfig::new("test-project");
        original.save(&config_path).unwrap();

        let loaded = ProjectConfig::load(&config_path).unwrap();
        assert_eq!(loaded.project.name, "test-project");
        assert_eq!(
            loaded.extensions.enabled,
            vec!["document", "task", "session"]
        );
    }

    #[test]
    fn test_project_config_toml_format() {
        let config = ProjectConfig::new("my-project");
        let toml_str = toml::to_string_pretty(&config).unwrap();

        // 验证 TOML 格式包含预期的 section（无 [editor]）
        assert!(toml_str.contains("[project]"));
        assert!(!toml_str.contains("[editor]"));
        assert!(toml_str.contains("[extensions]"));
        assert!(toml_str.contains("name = \"my-project\""));
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = ProjectConfig::load(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }
}

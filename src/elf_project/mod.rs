/// ElfProject — 目录式项目格式（替代 ZIP 式 ElfArchive）
///
/// .elf/ 目录结构：
/// ```text
/// project/
/// ├── .elf/
/// │   ├── eventstore.db    # Event 日志（SQLite）
/// │   ├── config.toml      # 项目配置
/// │   └── templates/       # Agent 工作模板
/// ```
///
/// 与 Phase 1 的 ElfArchive（ZIP 归档 + TempDir）不同：
/// - 直接读写 .elf/ 目录，无需解压/压缩
/// - SQLite WAL 模式直接持久化，无需 save
/// - 嵌入项目目录（类似 .git/）
pub mod config;

use crate::engine::{EventPoolWithPath, EventStore};
use config::ProjectConfig;
use std::path::{Path, PathBuf};

/// 默认 Skill 内容（编译进二进制）
pub const DEFAULT_SKILL: &str = include_str!("../../templates/skills/default.md");

/// Reconciliation 脚本（编译进二进制）
pub const RECONCILE_SCRIPT: &str = include_str!("../../templates/skills/scripts/reconcile.sh");

/// ElfProject 代表一个已初始化的 .elf/ 项目
#[derive(Debug)]
pub struct ElfProject {
    /// 项目根目录（包含 .elf/ 的目录）
    project_dir: PathBuf,
    /// .elf/ 目录路径
    elf_dir: PathBuf,
    /// eventstore.db 路径
    db_path: PathBuf,
    /// 项目配置
    config: ProjectConfig,
}

impl ElfProject {
    /// 初始化新项目：创建 .elf/ 目录结构、eventstore.db 和 config.toml
    ///
    /// 对应 `elf init` 命令。不写 bootstrap events（由调用方负责）。
    pub async fn init(project_dir: &Path) -> Result<Self, String> {
        let elf_dir = project_dir.join(".elf");
        let db_path = elf_dir.join("eventstore.db");
        let config_path = elf_dir.join("config.toml");
        let templates_dir = elf_dir.join("templates");

        // 不允许重复初始化
        if elf_dir.exists() {
            return Err(format!(
                ".elf/ directory already exists at {}",
                elf_dir.display()
            ));
        }

        // 创建目录结构
        std::fs::create_dir_all(&elf_dir)
            .map_err(|e| format!("Failed to create .elf/ directory: {}", e))?;
        std::fs::create_dir_all(&templates_dir)
            .map_err(|e| format!("Failed to create templates/ directory: {}", e))?;

        // 创建 skills 目录并写入默认 Skill
        let skills_dir = templates_dir.join("skills");
        std::fs::create_dir_all(&skills_dir)
            .map_err(|e| format!("Failed to create skills/ directory: {}", e))?;
        std::fs::write(skills_dir.join("default.md"), DEFAULT_SKILL)
            .map_err(|e| format!("Failed to write default skill: {}", e))?;

        // 初始化 eventstore.db
        EventStore::create(db_path.to_str().unwrap())
            .await
            .map_err(|e| format!("Failed to create eventstore.db: {}", e))?;

        // 生成项目配置（不存 editor 信息，类似 Git 模式）
        let project_name = project_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed");

        let config = ProjectConfig::new(project_name);
        config.save(&config_path)?;

        Ok(Self {
            project_dir: project_dir.to_path_buf(),
            elf_dir,
            db_path,
            config,
        })
    }

    /// 打开已有项目：读取 .elf/ 目录
    pub fn open(project_dir: &Path) -> Result<Self, String> {
        let elf_dir = project_dir.join(".elf");
        let db_path = elf_dir.join("eventstore.db");
        let config_path = elf_dir.join("config.toml");

        if !elf_dir.exists() {
            return Err(format!(
                ".elf/ directory not found at {}",
                elf_dir.display()
            ));
        }

        if !db_path.exists() {
            return Err(format!("eventstore.db not found at {}", db_path.display()));
        }

        let config = if config_path.exists() {
            ProjectConfig::load(&config_path)?
        } else {
            // 兼容：如果没有 config.toml，生成默认并保存
            let project_name = project_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unnamed");
            let config = ProjectConfig::new(project_name);
            config.save(&config_path)?;
            config
        };

        Ok(Self {
            project_dir: project_dir.to_path_buf(),
            elf_dir,
            db_path,
            config,
        })
    }

    /// 获取 EventPoolWithPath（用于 Engine）
    pub async fn event_pool(&self) -> Result<EventPoolWithPath, String> {
        EventStore::create(self.db_path.to_str().unwrap())
            .await
            .map_err(|e| format!("Failed to open eventstore.db: {}", e))
    }

    /// 项目根目录
    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// .elf/ 目录路径
    pub fn elf_dir(&self) -> &Path {
        &self.elf_dir
    }

    /// eventstore.db 路径
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// 项目配置
    pub fn config(&self) -> &ProjectConfig {
        &self.config
    }

    /// Skills 模板目录
    pub fn skills_dir(&self) -> PathBuf {
        self.elf_dir.join("templates").join("skills")
    }

    /// 读取 Skill 内容
    ///
    /// 查找顺序：role.md → default.md → DEFAULT_SKILL（编译内嵌 fallback）
    pub fn read_skill(&self, role: Option<&str>) -> String {
        let skills_dir = self.skills_dir();

        // 尝试 role 特定的 Skill
        if let Some(role) = role {
            let role_path = skills_dir.join(format!("{}.md", role));
            if role_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&role_path) {
                    return content;
                }
            }
        }

        // 尝试项目级 default.md
        let default_path = skills_dir.join("default.md");
        if default_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&default_path) {
                return content;
            }
        }

        // Fallback 到编译内嵌的默认 Skill
        DEFAULT_SKILL.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EventStore;
    use crate::models::Event;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_creates_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("my-project");
        std::fs::create_dir_all(&project_dir).unwrap();

        let project = ElfProject::init(&project_dir).await.unwrap();

        // 验证目录结构
        assert!(project.elf_dir().exists());
        assert!(project.db_path().exists());
        assert!(project.elf_dir().join("config.toml").exists());
        assert!(project.elf_dir().join("templates").exists());
        assert!(project.skills_dir().exists());
        assert!(project.skills_dir().join("default.md").exists());

        // 验证配置（无 [editor] section，Git 模式）
        assert_eq!(project.config().project.name, "my-project");
        assert_eq!(
            project.config().extensions.enabled,
            vec!["document", "task", "session"]
        );
    }

    #[tokio::test]
    async fn test_init_rejects_existing_elf_dir() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("existing");
        std::fs::create_dir_all(project_dir.join(".elf")).unwrap();

        let result = ElfProject::init(&project_dir).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[tokio::test]
    async fn test_open_existing_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("open-test");
        std::fs::create_dir_all(&project_dir).unwrap();

        // 先 init
        let _project = ElfProject::init(&project_dir).await.unwrap();

        // 再 open（模拟下次打开）
        let opened = ElfProject::open(&project_dir).unwrap();
        assert_eq!(opened.config().project.name, "open-test");
    }

    #[tokio::test]
    async fn test_open_nonexistent_project() {
        let result = ElfProject::open(Path::new("/nonexistent/project"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_event_pool_works() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("pool-test");
        std::fs::create_dir_all(&project_dir).unwrap();

        let project = ElfProject::init(&project_dir).await.unwrap();
        let pool = project.event_pool().await.unwrap();

        // 写入 event
        let mut ts = HashMap::new();
        ts.insert("editor1".to_string(), 1);
        let event = Event::new(
            "block-1".to_string(),
            "editor1/core.create".to_string(),
            serde_json::json!({"name": "Test"}),
            ts,
        );
        EventStore::append_events(&pool.pool, &[event])
            .await
            .unwrap();

        // 读回
        let events = EventStore::get_all_events(&pool.pool).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, "block-1");
    }

    #[tokio::test]
    async fn test_persistence_across_open() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("persist-test");
        std::fs::create_dir_all(&project_dir).unwrap();

        // 创建并写入数据
        {
            let project = ElfProject::init(&project_dir).await.unwrap();
            let pool = project.event_pool().await.unwrap();

            let mut ts = HashMap::new();
            ts.insert("editor1".to_string(), 1);
            let event = Event::new(
                "block-1".to_string(),
                "editor1/core.create".to_string(),
                serde_json::json!({"name": "Persistent"}),
                ts,
            );
            EventStore::append_events(&pool.pool, &[event])
                .await
                .unwrap();
        }
        // project 和 pool 被 drop

        // 重新打开，验证数据持久化
        let project = ElfProject::open(&project_dir).unwrap();
        let pool = project.event_pool().await.unwrap();
        let events = EventStore::get_all_events(&pool.pool).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value["name"], "Persistent");
    }

    #[tokio::test]
    async fn test_read_skill_default() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("skill-test");
        std::fs::create_dir_all(&project_dir).unwrap();

        let project = ElfProject::init(&project_dir).await.unwrap();

        // 无 role 参数，应读取 default.md
        let skill = project.read_skill(None);
        assert!(skill.contains("Elfiee Skill"));
        assert!(skill.contains("elfiee_auth"));
    }

    #[tokio::test]
    async fn test_read_skill_role_specific() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("role-skill-test");
        std::fs::create_dir_all(&project_dir).unwrap();

        let project = ElfProject::init(&project_dir).await.unwrap();

        // 写入 role 特定 Skill
        std::fs::write(
            project.skills_dir().join("coder.md"),
            "# Coder Skill\nSpecialized instructions for coders.",
        )
        .unwrap();

        // 指定 role，应读取 coder.md
        let skill = project.read_skill(Some("coder"));
        assert!(skill.contains("Coder Skill"));

        // 不存在的 role，应 fallback 到 default.md
        let skill = project.read_skill(Some("nonexistent"));
        assert!(skill.contains("Elfiee Skill"));
    }

    #[tokio::test]
    async fn test_read_skill_fallback_to_builtin() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("fallback-skill-test");
        std::fs::create_dir_all(&project_dir).unwrap();

        let project = ElfProject::init(&project_dir).await.unwrap();

        // 删除项目级 default.md
        std::fs::remove_file(project.skills_dir().join("default.md")).unwrap();

        // 应 fallback 到编译内嵌的 DEFAULT_SKILL
        let skill = project.read_skill(None);
        assert!(skill.contains("Elfiee Skill"));
        assert_eq!(skill, DEFAULT_SKILL);
    }
}

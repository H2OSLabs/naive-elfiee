use clap::Args;

use crate::core::generator::Generator;
use crate::models::config::ExtensionConfig;
use crate::utils::file_ops::FileOperations;
use crate::utils::naming::to_pascal_case;

use std::fs;
use std::path::Path;

/// CLI command for generating a new extension.
#[derive(Args, Debug)]
pub struct CreateCommand {
    /// Extension name (snake_case)
    #[arg(short, long)]
    pub name: String,

    /// Target block type
    #[arg(short, long)]
    pub block_type: String,

    /// Comma separated capability list
    #[arg(short, long)]
    pub capabilities: String,

    /// Generate authorization tests
    #[arg(long, default_value = "true")]
    pub with_auth_tests: bool,

    /// Generate workflow tests
    #[arg(long, default_value = "true")]
    pub with_workflow_tests: bool,
}

impl CreateCommand {
    pub fn execute(&self) -> Result<(), String> {
        // Parse capabilities list
        let capabilities: Vec<String> = self
            .capabilities
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        for cap in &capabilities {
            let is_snake = cap
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            let starts_with_digit = cap
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false);
            if !is_snake || starts_with_digit {
                return Err(format!(
                    "invalid capability name '{}': must be snake_case (letters, digits, underscores)",
                    cap
                ));
            }
        }

        // Build configuration
        let mut config = ExtensionConfig::new(
            self.name.clone(),
            self.block_type.clone(),
            capabilities.clone(),
        );
        config.with_auth_tests = self.with_auth_tests;
        config.with_workflow_tests = self.with_workflow_tests;

        // Validate input
        if let Err(errors) = config.validate() {
            return Err(format!("invalid configuration: {}", errors.join(", ")));
        }

        // Generate files
        let generator = Generator::new()?;
        let generated = generator.generate_extension(&config)?;
        for (path, content) in generated.files {
            FileOperations::write_file(&path, &content)?;
        }

        // Update project registrations
        Self::ensure_extension_module(&self.name)?;
        Self::update_registry(&self.name, &capabilities)?;
        Self::update_specta_types(&self.name, &capabilities)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    use crate::test_support::{capture_original_dir, restore_original_dir, test_lock};

    fn copy_dir_recursive(src: &Path, dst: &Path) {
        for entry in walkdir::WalkDir::new(src) {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(src).unwrap();
            let target = dst.join(relative);

            if path.is_dir() {
                fs::create_dir_all(&target).unwrap();
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::copy(path, target).unwrap();
            }
        }
    }

    fn prepare_project_layout(temp: &TempDir) {
        let cwd = temp.path();
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Copy templates and rules so generator/analyzer can load them
        copy_dir_recursive(&manifest_dir.join("templates"), &cwd.join("templates"));
        copy_dir_recursive(&manifest_dir.join("rules"), &cwd.join("rules"));

        // Minimal project structure
        fs::create_dir_all(cwd.join("src-tauri/src/extensions")).unwrap();
        fs::create_dir_all(cwd.join("src-tauri/src/capabilities")).unwrap();
        fs::create_dir_all(cwd.join("src-tauri/src")).unwrap();

        fs::write(cwd.join("src-tauri/src/extensions/mod.rs"), "").unwrap();
        fs::write(
            cwd.join("src-tauri/src/capabilities/registry.rs"),
            "use std::sync::Arc;\n\npub struct CapabilityRegistry;\nimpl CapabilityRegistry {\n    fn register_extensions(&mut self) {\n    }\n    fn register<T>(&mut self, _cap: Arc<T>) {}\n}\n",
        )
        .unwrap();
        fs::write(
            cwd.join("src-tauri/src/lib.rs"),
            "#[cfg(debug_assertions)]\nfn register_types(builder: &mut Vec<String>) {\n}\n",
        )
        .unwrap();
    }

    #[test]
    fn test_execute_creates_files() {
        let _guard = test_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = capture_original_dir();
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        env::set_current_dir(temp.path()).unwrap();

        copy_dir_recursive(
            &manifest_dir.join("templates"),
            &temp.path().join("templates"),
        );
        copy_dir_recursive(&manifest_dir.join("rules"), &temp.path().join("rules"));
        prepare_project_layout(&temp);

        let cmd = CreateCommand {
            name: "todo".to_string(),
            block_type: "todo".to_string(),
            capabilities: "add_item,toggle_item".to_string(),
            with_auth_tests: true,
            with_workflow_tests: true,
        };

        let result = cmd.execute();
        assert!(result.is_ok());

        let ext_path = temp.path().join("src-tauri/src/extensions/todo");
        assert!(ext_path.join("mod.rs").exists());
        assert!(ext_path.join("todo_add_item.rs").exists());
        assert!(ext_path.join("tests.rs").exists());

        restore_original_dir(original_dir);
    }

    #[test]
    fn test_execute_invalid_name() {
        let _guard = test_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = capture_original_dir();
        env::set_current_dir(temp.path()).unwrap();

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        copy_dir_recursive(
            &manifest_dir.join("templates"),
            &temp.path().join("templates"),
        );
        copy_dir_recursive(&manifest_dir.join("rules"), &temp.path().join("rules"));
        prepare_project_layout(&temp);

        let cmd = CreateCommand {
            name: "invalid name".to_string(),
            block_type: "todo".to_string(),
            capabilities: "add".to_string(),
            with_auth_tests: true,
            with_workflow_tests: true,
        };

        let result = cmd.execute();
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("invalid"),
            "error message should mention invalid name"
        );

        restore_original_dir(original_dir);
    }

    #[test]
    fn test_execute_invalid_capability_name() {
        let _guard = test_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = capture_original_dir();
        env::set_current_dir(temp.path()).unwrap();

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        copy_dir_recursive(
            &manifest_dir.join("templates"),
            &temp.path().join("templates"),
        );
        copy_dir_recursive(&manifest_dir.join("rules"), &temp.path().join("rules"));
        prepare_project_layout(&temp);

        let cmd = CreateCommand {
            name: "valid_name".to_string(),
            block_type: "todo".to_string(),
            capabilities: "invalid-cap".to_string(),
            with_auth_tests: true,
            with_workflow_tests: true,
        };

        let result = cmd.execute();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("invalid capability name"),
            "error message should mention invalid capability name, got: {err}"
        );

        restore_original_dir(original_dir);
    }
}

impl CreateCommand {
    fn ensure_extension_module(extension_name: &str) -> Result<(), String> {
        let path = Path::new("src-tauri/src/extensions/mod.rs");
        let mut content = fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
        let line = format!("pub mod {};", extension_name);
        if !content.contains(&line) {
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(&line);
            content.push('\n');
            FileOperations::write_file(path, &content)?;
        }
        Ok(())
    }

    fn update_registry(extension_name: &str, capabilities: &[String]) -> Result<(), String> {
        let path = Path::new("src-tauri/src/capabilities/registry.rs");
        let mut content = fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;

        let use_line = format!("    use crate::extensions::{}::*;", extension_name);
        if !content.contains(&use_line) {
            if let Some(pos) = content.find("fn register_extensions") {
                if let Some(brace_pos) = content[pos..].find('{') {
                    let insert_pos = pos + brace_pos + 1;
                    content.insert_str(insert_pos, &format!("\n    {}\n", use_line.trim()));
                } else {
                    return Err("unable to locate register_extensions body".to_string());
                }
            } else {
                return Err("unable to locate register_extensions function".to_string());
            }
        }

        for capability in capabilities {
            let struct_name = format!(
                "{}Capability",
                to_pascal_case(&format!("{}_{}", extension_name, capability))
            );
            let register_line = format!("        self.register(Arc::new({}));", struct_name);
            if !content.contains(&register_line) {
                if let Some(pos) = content.rfind("self.register(") {
                    let insert_pos = content[pos..]
                        .find('\n')
                        .map(|i| pos + i + 1)
                        .unwrap_or(content.len());
                    content.insert_str(insert_pos, &format!("{}\n", register_line));
                } else if let Some(pos) = content.find("fn register_extensions") {
                    if let Some(body_pos) = content[pos..].find('{') {
                        let insert_pos = pos + body_pos + 1;
                        content.insert_str(insert_pos, &format!("\n        {}\n", register_line));
                    }
                }
            }
        }

        FileOperations::write_file(path, &content)?;
        Ok(())
    }

    fn update_specta_types(extension_name: &str, capabilities: &[String]) -> Result<(), String> {
        let path = Path::new("src-tauri/src/lib.rs");
        let mut content = fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;

        let mut added_any = false;
        for capability in capabilities {
            let payload_name = format!(
                "{}Payload",
                to_pascal_case(&format!("{}_{}", extension_name, capability))
            );
            let line = format!(
                "            .typ::<extensions::{}::{}>()",
                extension_name, payload_name
            );
            if !content.contains(&line) {
                if let Some(pos) = content.find(".typ::<") {
                    // insert before first typ line for readability
                    content.insert_str(pos, &format!("{}\n", line));
                } else if let Some(builder_pos) = content.find("let specta_builder") {
                    if let Some(insert_pos) = content[builder_pos..].find(';') {
                        let global_pos = builder_pos + insert_pos;
                        content.insert_str(global_pos, &format!("\n{}\n", line));
                    }
                }
                added_any = true;
            }
        }

        if added_any {
            FileOperations::write_file(path, &content)?;
        }
        Ok(())
    }
}

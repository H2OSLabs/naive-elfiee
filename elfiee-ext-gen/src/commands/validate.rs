use clap::Args;

use crate::core::validator::Validator;

use std::path::Path;

/// CLI command for validating an extension’s structure.
#[derive(Args, Debug)]
pub struct ValidateCommand {
    /// Extension name
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ValidateResult {
    pub passed: Vec<String>,
    pub failed: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidateCommand {
    pub fn execute(&self) -> Result<ValidateResult, String> {
        let extension_path = Path::new("src-tauri/src/extensions").join(&self.name);
        if !extension_path.exists() {
            return Err(format!(
                "extension '{}' not found. run generator first.",
                self.name
            ));
        }

        let report = Validator::validate_extension(&extension_path);
        if !report.failed.is_empty() {
            return Err(format!("validation failed: {}", report.failed.join(", ")));
        }

        Ok(ValidateResult {
            passed: report.passed,
            failed: report.failed,
            warnings: report.warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{capture_original_dir, restore_original_dir, test_lock};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

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

    fn prepare_environment(temp: &TempDir) {
        let cwd = temp.path();
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        copy_dir_recursive(&manifest_dir.join("templates"), &cwd.join("templates"));
        copy_dir_recursive(&manifest_dir.join("rules"), &cwd.join("rules"));

        fs::create_dir_all(cwd.join("src-tauri/src/extensions")).unwrap();
        fs::create_dir_all(cwd.join("src-tauri/src/capabilities")).unwrap();
        fs::create_dir_all(cwd.join("src-tauri/src")).unwrap();
    }

    fn seed_extension(name: &str, root: &Path) {
        let ext_dir = root.join("src-tauri/src/extensions").join(name);
        fs::create_dir_all(&ext_dir).unwrap();
        fs::write(
            ext_dir.join("mod.rs"),
            r#"
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TodoAddItemPayload {
    pub data: serde_json::Value,
}

pub mod todo_add_item;
pub use todo_add_item::*;
"#,
        )
        .unwrap();
        fs::write(
            ext_dir.join("todo_add_item.rs"),
            "
pub struct TodoAddItemCapability;
",
        )
        .unwrap();

        fs::write(
            root.join("src-tauri/src/extensions/mod.rs"),
            format!("pub mod {};\n", name),
        )
        .unwrap();
        fs::write(
            root.join("src-tauri/src/capabilities/registry.rs"),
            format!(
                "use std::sync::Arc;\nuse crate::extensions::{name}::*;\n\npub struct CapabilityRegistry;\nimpl CapabilityRegistry {{\n    fn register_extensions(&mut self) {{\n        self.register(Arc::new({}Capability));\n    }}\n    fn register<T>(&mut self, _cap: Arc<T>) {{}}\n}}\n",
                "TodoAddItem"
            ),
        )
        .unwrap();
        fs::write(
            root.join("src-tauri/src/lib.rs"),
            format!(
                "#[cfg(debug_assertions)]\nfn register_types(builder: &mut Vec<String>) {{\n    builder.push(format!(\"{{}}\", std::any::type_name::<extensions::{name}::TodoAddItemPayload>()));\n}}\n"
            ),
        )
        .unwrap();
    }

    #[test]
    fn test_validate_missing_extension() {
        let _guard = test_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = capture_original_dir();
        env::set_current_dir(temp.path()).unwrap();

        prepare_environment(&temp);

        let cmd = ValidateCommand {
            name: "todo".to_string(),
        };

        let result = cmd.execute();
        assert!(
            result.is_err(),
            "expected validation to fail for missing extension"
        );

        restore_original_dir(original_dir);
    }

    #[test]
    fn test_validate_success() {
        let _guard = test_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = capture_original_dir();
        env::set_current_dir(temp.path()).unwrap();

        prepare_environment(&temp);
        seed_extension("todo", temp.path());

        let cmd = ValidateCommand {
            name: "todo".to_string(),
        };

        let result = cmd.execute();
        assert!(result.is_ok(), "expected validation to succeed");

        let ValidateResult { failed, .. } = result.unwrap();
        assert!(failed.is_empty(), "expected no failures, got: {:?}", failed);

        restore_original_dir(original_dir);
    }
}

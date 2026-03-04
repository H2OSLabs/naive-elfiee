use clap::Args;

use crate::core::analyzer::TestAnalyzer;
use crate::core::guide_gen::GuideGenerator;

use std::path::Path;

/// CLI command for generating guidance from test failures.
#[derive(Args, Debug)]
pub struct GuideCommand {
    /// Extension name
    pub name: String,
}

impl GuideCommand {
    pub fn execute(&self) -> Result<String, String> {
        let extension_dir = Path::new("src-tauri/src/extensions").join(&self.name);
        if !extension_dir.exists() {
            return Err(format!(
                "extension '{}' not found. run generator first.",
                self.name
            ));
        }

        let analyzer = TestAnalyzer::new()?;
        let guide = GuideGenerator::new(analyzer);
        guide.generate_guide(&self.name)
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

        // Copy templates and rules for analyzer/guide usage
        copy_dir_recursive(&manifest_dir.join("templates"), &cwd.join("templates"));
        copy_dir_recursive(&manifest_dir.join("rules"), &cwd.join("rules"));

        fs::create_dir_all(cwd.join("src-tauri/src/extensions")).unwrap();
        fs::create_dir_all(cwd.join("src-tauri/src/capabilities")).unwrap();
        fs::create_dir_all(cwd.join("src-tauri/src")).unwrap();

        fs::write(cwd.join("src-tauri/src/extensions/mod.rs"), "").unwrap();
        fs::write(
            cwd.join("src-tauri/src/capabilities/registry.rs"),
            "use std::sync::Arc;\n\npub struct CapabilityRegistry;\nimpl CapabilityRegistry {\n    fn register_extensions(&mut self) {}\n    fn register<T>(&mut self, _cap: Arc<T>) {}\n}\n",
        )
        .unwrap();
        fs::write(
            cwd.join("src-tauri/src/lib.rs"),
            "#[cfg(debug_assertions)]\nfn register_types(builder: &mut Vec<String>) {}\n",
        )
        .unwrap();
    }

    #[test]
    fn test_execute_nonexistent_extension() {
        let _guard = test_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = capture_original_dir();
        env::set_current_dir(temp.path()).unwrap();

        prepare_environment(&temp);

        let cmd = GuideCommand {
            name: "unknown".to_string(),
        };

        let result = cmd.execute();
        assert!(result.is_err(), "expected failure for unknown extension");

        restore_original_dir(original_dir);
    }

    #[test]
    #[ignore] // Requires cargo test execution - integration test level
    fn test_execute_shows_guide() {
        // This test requires:
        // 1. Actual cargo environment with compiled tests
        // 2. Real extension code in src-tauri/src/extensions/
        // 3. Ability to execute `cargo test <extension>::tests`
        //
        // Cannot be run in unit test environment.
        // Should be moved to integration tests once we have a test harness.
        //
        // For now, manual testing procedure:
        // 1. cd /path/to/elfiee
        // 2. elfiee-ext-gen create --name test_ext --block-type test --capabilities "test_cap"
        // 3. elfiee-ext-gen guide test_ext
        // 4. Verify output contains test status and hints
    }
}

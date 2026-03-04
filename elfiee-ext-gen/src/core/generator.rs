/// Template engine for generating extension files.
///
/// Following generator-dev-plan.md Phase 2.1
use crate::models::config::ExtensionConfig;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tera::Tera;

// ============================================================================
// Data Structures (as per design doc)
// ============================================================================

/// Generated files structure.
///
/// Contains the generated file contents (not paths) and next step suggestions.
#[derive(Debug, Clone)]
pub struct GeneratedFiles {
    /// File path → File content (NOT path to file, but the actual content)
    pub files: HashMap<PathBuf, String>,
    /// Next steps for the developer
    pub next_steps: Vec<String>,
}

/// Field suggestion for payload.
#[derive(Debug, Clone, Serialize)]
pub struct FieldSuggestion {
    pub name: String,
    pub type_name: String,
    pub reason: String,
}

/// Template engine for generating extensions.
pub struct Generator {
    tera: Tera,
}

impl Generator {
    /// Create a new Generator instance.
    ///
    /// Loads templates from embedded directory or default path.
    /// NOTE: No parameters - templates should be bundled or use default location.
    ///
    /// # Examples
    /// ```no_run
    /// use elfiee_ext_gen::core::generator::Generator;
    ///
    /// let gen = Generator::new().unwrap();
    /// ```
    pub fn new() -> Result<Self, String> {
        // Try current working directory first, then fall back to manifest directory.
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let mut bases = Vec::new();
        bases.push(manifest_dir.clone());
        if let Ok(cwd) = std::env::current_dir() {
            if cwd != manifest_dir {
                bases.push(cwd);
            }
        }

        let mut attempts: Vec<String> = Vec::new();

        for base in bases {
            let template_root = base.join("templates");
            if !template_root.exists() {
                attempts.push(format!("{} (directory not found)", template_root.display()));
                continue;
            }

            let pattern = template_root.join("**/*.tera");
            let pattern_str = pattern.to_string_lossy().to_string();

            match Tera::new(&pattern_str) {
                Ok(tera) => return Ok(Self { tera }),
                Err(err) => {
                    attempts.push(format!("{} (failed to load: {})", pattern_str, err));
                }
            }
        }

        let details = if attempts.is_empty() {
            "No template directory found.".to_string()
        } else {
            format!(
                "No template directory found. Searched:\n  - {}",
                attempts.join("\n  - ")
            )
        };

        Err(details)
    }

    /// Generate all extension files.
    ///
    /// # Arguments
    /// * `config` - Extension configuration
    ///
    /// # Returns
    /// * `Ok(GeneratedFiles)` with file contents (HashMap) and next steps
    /// * `Err(String)` with error description
    ///
    /// # Examples
    /// ```no_run
    /// use elfiee_ext_gen::core::generator::Generator;
    /// use elfiee_ext_gen::models::config::ExtensionConfig;
    ///
    /// let config = ExtensionConfig::new("todo", "todo", vec!["add_item".to_string()]);
    /// let gen = Generator::new().unwrap();
    /// let result = gen.generate_extension(&config).unwrap();
    /// assert!(result.files.len() > 0);
    /// ```
    pub fn generate_extension(&self, config: &ExtensionConfig) -> Result<GeneratedFiles, String> {
        use crate::utils::naming::{to_pascal_case, to_snake_case};

        let context = self.prepare_context(config);
        let mut files = HashMap::new();

        // Generate mod.rs
        let mod_content = self.render_template("mod.rs.tera", &context)?;
        let mod_path = PathBuf::from(format!("src-tauri/src/extensions/{}/mod.rs", config.name));
        files.insert(mod_path, mod_content);

        // Generate capability files
        for cap_id in &config.capabilities {
            let mut cap_context = context.clone();
            cap_context.insert("capability_id", cap_id);
            cap_context.insert("capability_id_snake", &to_snake_case(cap_id));
            // Same as in prepare_context: struct name without suffix
            cap_context.insert(
                "capability_struct_name",
                &to_pascal_case(&format!("{}_{}", config.name, cap_id)),
            );

            let cap_content = self.render_template("capability.rs.tera", &cap_context)?;
            let cap_path = PathBuf::from(format!(
                "src-tauri/src/extensions/{}/{}_{}.rs",
                config.name,
                to_snake_case(&config.name),
                to_snake_case(cap_id)
            ));
            files.insert(cap_path, cap_content);
        }

        // Generate DEVELOPMENT_GUIDE.md
        let guide_content = self.render_template("DEVELOPMENT_GUIDE.md.tera", &context)?;
        let guide_path = PathBuf::from(format!(
            "src-tauri/src/extensions/{}/DEVELOPMENT_GUIDE.md",
            config.name
        ));
        files.insert(guide_path, guide_content);

        // Generate tests.rs
        let tests_content = self.render_template("tests.rs.tera", &context)?;
        let tests_path =
            PathBuf::from(format!("src-tauri/src/extensions/{}/tests.rs", config.name));
        files.insert(tests_path, tests_content);

        // Prepare next steps
        let next_steps = vec![
            format!("Run: cargo test {}::tests", config.name),
            format!(
                "Edit: src-tauri/src/extensions/{}/mod.rs - Define Payload fields",
                config.name
            ),
            format!(
                "Follow: src-tauri/src/extensions/{}/DEVELOPMENT_GUIDE.md",
                config.name
            ),
        ];

        Ok(GeneratedFiles { files, next_steps })
    }

    /// Render a single template.
    ///
    /// # Arguments
    /// * `template_name` - Template file name (e.g., "mod.rs.tera")
    /// * `context` - Tera context
    ///
    /// # Returns
    /// Rendered template content
    fn render_template(
        &self,
        template_name: &str,
        context: &tera::Context,
    ) -> Result<String, String> {
        self.tera
            .render(template_name, context)
            .or_else(|primary_err| {
                let fallback = format!("templates/{}", template_name);
                self.tera.render(&fallback, context).map_err(|_| {
                    format!(
                        "Failed to render template {} (and fallback {}): {}",
                        template_name, fallback, primary_err
                    )
                })
            })
    }

    /// Prepare template context from config.
    fn prepare_context(&self, config: &ExtensionConfig) -> tera::Context {
        use crate::utils::naming::{to_pascal_case, to_snake_case};

        let mut context = tera::Context::new();

        // Extension-level context
        context.insert("extension_name", &config.name);
        context.insert("extension_name_pascal", &to_pascal_case(&config.name));
        context.insert("block_type", &config.block_type);

        // Capabilities context
        let capabilities: Vec<_> = config.capabilities.iter().map(|cap_id| {
            let fields = self.infer_fields(cap_id);
            // For Payload struct name: "todo" + "add_item" -> "TodoAddItem"
            // Template will add "Payload" suffix
            let struct_name = to_pascal_case(&format!("{}_{}", config.name, cap_id));

            serde_json::json!({
                "id": cap_id,
                "id_snake": to_snake_case(cap_id),
                "struct_name": struct_name,
                "file_name": format!("{}_{}.rs", to_snake_case(&config.name), to_snake_case(cap_id)),
                "fields": fields,
            })
        }).collect();

        context.insert("capabilities", &capabilities);

        // Test configuration
        context.insert("with_auth_tests", &config.with_auth_tests);
        context.insert("with_workflow_tests", &config.with_workflow_tests);

        context
    }

    /// Infer payload fields based on capability name.
    ///
    /// Uses heuristics to suggest common fields.
    fn infer_fields(&self, capability_name: &str) -> Vec<FieldSuggestion> {
        // Pattern matching to infer fields based on capability semantics
        match capability_name {
            name if name.contains("add") || name.contains("create") => vec![
                FieldSuggestion {
                    name: "text".to_string(),
                    type_name: "String".to_string(),
                    reason: "The text content to add".to_string(),
                },
                FieldSuggestion {
                    name: "priority".to_string(),
                    type_name: "Option<u32>".to_string(),
                    reason: "Optional priority level".to_string(),
                },
            ],
            name if name.contains("toggle") || name.contains("update") => vec![
                FieldSuggestion {
                    name: "item_id".to_string(),
                    type_name: "String".to_string(),
                    reason: "ID of the item to toggle".to_string(),
                },
                FieldSuggestion {
                    name: "status".to_string(),
                    type_name: "bool".to_string(),
                    reason: "New status".to_string(),
                },
            ],
            _ => vec![FieldSuggestion {
                name: "data".to_string(),
                type_name: "serde_json::Value".to_string(),
                reason: "Generic data field - replace with specific fields".to_string(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::ExtensionConfig;

    // ========================================
    // Test: Generator::new()
    // (Following dev-plan.md line 392-395)
    // ========================================

    #[test]
    fn test_generator_new() {
        let generator = Generator::new();
        assert!(
            generator.is_ok(),
            "Generator should initialize successfully"
        );
    }

    // ========================================
    // Test: infer_fields()
    // (Following dev-plan.md line 397-412)
    // ========================================

    #[test]
    fn test_infer_fields_for_add_capability() {
        let generator = Generator::new().unwrap();
        let fields = generator.infer_fields("add_item");

        assert!(
            !fields.is_empty(),
            "Should infer at least one field for 'add_item'"
        );
        assert!(
            fields.iter().any(|f| f.name == "text"),
            "Should suggest 'text' field"
        );
        assert!(
            fields.iter().any(|f| f.type_name == "String"),
            "Should suggest String type"
        );
    }

    #[test]
    fn test_infer_fields_for_toggle_capability() {
        let generator = Generator::new().unwrap();
        let fields = generator.infer_fields("toggle_item");

        assert!(
            !fields.is_empty(),
            "Should infer at least one field for 'toggle_item'"
        );
        assert!(
            fields.iter().any(|f| f.name == "item_id"),
            "Should suggest 'item_id' field"
        );
    }

    // ========================================
    // Test: generate_extension()
    // (Following dev-plan.md line 415-439)
    // ========================================

    #[test]
    fn test_generate_extension_creates_all_files() {
        let generator = Generator::new().unwrap();
        let config = ExtensionConfig {
            name: "todo".to_string(),
            block_type: "todo".to_string(),
            capabilities: vec!["add_item".to_string(), "toggle_item".to_string()],
            with_auth_tests: true,
            with_workflow_tests: true,
        };

        let result = generator.generate_extension(&config).unwrap();

        // Should have 5 files: mod.rs + 2 capabilities + DEVELOPMENT_GUIDE.md + tests.rs
        assert_eq!(
            result.files.len(),
            5,
            "Should generate 5 files (including tests.rs)"
        );

        // Verify mod.rs exists in HashMap
        assert!(
            result
                .files
                .keys()
                .any(|p| p.to_str().unwrap().ends_with("mod.rs")),
            "Should have mod.rs"
        );

        // Verify capability files exist
        assert!(
            result
                .files
                .keys()
                .any(|p| p.to_str().unwrap().contains("todo_add_item.rs")),
            "Should have todo_add_item.rs"
        );
        assert!(
            result
                .files
                .keys()
                .any(|p| p.to_str().unwrap().contains("todo_toggle_item.rs")),
            "Should have todo_toggle_item.rs"
        );

        // Verify guide exists
        assert!(
            result
                .files
                .keys()
                .any(|p| p.to_str().unwrap().ends_with("DEVELOPMENT_GUIDE.md")),
            "Should have DEVELOPMENT_GUIDE.md"
        );
    }

    #[test]
    fn test_generated_mod_rs_contains_payload() {
        let generator = Generator::new().unwrap();
        let config = ExtensionConfig {
            name: "todo".to_string(),
            block_type: "todo".to_string(),
            capabilities: vec!["add_item".to_string()],
            with_auth_tests: true,
            with_workflow_tests: true,
        };

        let result = generator.generate_extension(&config).unwrap();

        // Find mod.rs content
        let mod_content = result
            .files
            .iter()
            .find(|(path, _)| path.to_str().unwrap().ends_with("mod.rs"))
            .map(|(_, content)| content)
            .expect("mod.rs should exist");

        assert!(
            mod_content.contains("TodoAddItemPayload"),
            "mod.rs should define TodoAddItemPayload"
        );
        assert!(
            mod_content.contains("#[derive(Debug, Clone, Serialize, Deserialize, Type)]"),
            "Payload should have required derives"
        );
    }

    #[test]
    fn test_generated_capability_has_todo_markers() {
        let generator = Generator::new().unwrap();
        let config = ExtensionConfig {
            name: "todo".to_string(),
            block_type: "todo".to_string(),
            capabilities: vec!["add_item".to_string()],
            with_auth_tests: true,
            with_workflow_tests: true,
        };

        let result = generator.generate_extension(&config).unwrap();

        // Find capability file content
        let cap_content = result
            .files
            .iter()
            .find(|(path, _)| path.to_str().unwrap().contains("todo_add_item.rs"))
            .map(|(_, content)| content)
            .expect("Capability file should exist");

        assert!(
            cap_content.contains("todo!("),
            "Capability should have todo!() markers"
        );
        assert!(
            cap_content.contains("TODO:"),
            "Capability should have TODO comments"
        );
        assert!(
            cap_content.contains("#[capability(id = \"todo.add_item\""),
            "Capability should have #[capability] macro"
        );
    }

    #[test]
    fn test_generate_extension_includes_tests_file() {
        let generator = Generator::new().unwrap();
        let config = ExtensionConfig {
            name: "todo".to_string(),
            block_type: "todo".to_string(),
            capabilities: vec!["add_item".to_string()],
            with_auth_tests: true,
            with_workflow_tests: true,
        };

        let result = generator.generate_extension(&config).unwrap();

        let has_tests_file = result.files.keys().any(|p| {
            p.to_string_lossy()
                .ends_with("src-tauri/src/extensions/todo/tests.rs")
        });

        assert!(
            has_tests_file,
            "Expected generator to emit tests.rs skeleton for the extension"
        );
    }
}

/// Extension code validator.
///
/// Following generator-dev-plan.md Phase 2.4 (line 744-851)
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::utils::naming::to_pascal_case;
use syn::visit::Visit;
use syn::{
    Expr, ExprMethodCall, ExprPath, File, ImplItem, Item, ItemMod, Stmt, Type, UseTree, Visibility,
};

// ============================================================================
// Data Structures
// ============================================================================

/// Validator for extension code quality.
pub struct Validator;

/// Validation report summarizing checks.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub passed: Vec<String>,
    pub failed: Vec<String>,
    pub warnings: Vec<String>,
}

impl Validator {
    /// Validate extension completeness and correctness.
    ///
    /// # Arguments
    /// * `path` - Path to the extension directory
    ///
    /// # Returns
    /// * `ValidationReport` with passed, failed, and warning checks
    pub fn validate_extension(path: &Path) -> ValidationReport {
        let mut report = ValidationReport {
            passed: Vec::new(),
            failed: Vec::new(),
            warnings: Vec::new(),
        };

        // Check 1: Files exist
        match Self::check_files_exist(path) {
            Ok(msgs) => report.passed.extend(msgs),
            Err(msgs) => report.failed.extend(msgs),
        }

        // Check 2: Validate payloads (read mod.rs)
        let mod_path = path.join("mod.rs");
        if mod_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&mod_path) {
                match Self::validate_payloads(&content) {
                    Ok(msgs) => report.passed.extend(msgs),
                    Err(msgs) => report.failed.extend(msgs),
                }
            } else {
                report.failed.push("Failed to read mod.rs".to_string());
            }
        }

        // Check 3: Test coverage
        match Self::check_test_coverage(path) {
            Ok(msgs) => report.passed.extend(msgs),
            Err(msgs) => report.warnings.extend(msgs),
        }

        // Check 4: Registration (extract extension name from path)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            match Self::check_registration(path, name) {
                Ok(msgs) => report.passed.extend(msgs),
                Err(msgs) => report.warnings.extend(msgs),
            }
        }

        report
    }

    /// Check that required files exist.
    ///
    /// # Arguments
    /// * `path` - Extension directory path
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of passed checks
    /// * `Err(Vec<String>)` - List of missing files
    fn check_files_exist(path: &Path) -> Result<Vec<String>, Vec<String>> {
        let mut errors = Vec::new();
        let mut passed = Vec::new();

        // Check if mod.rs exists
        let mod_path = path.join("mod.rs");
        if mod_path.exists() {
            passed.push("mod.rs exists".to_string());
        } else {
            errors.push("mod.rs is missing".to_string());
        }

        // Check for at least one .rs file (besides mod.rs)
        if let Ok(entries) = std::fs::read_dir(path) {
            let rs_files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension().and_then(|s| s.to_str()) == Some("rs")
                        && e.file_name() != "mod.rs"
                })
                .collect();

            if !rs_files.is_empty() {
                passed.push(format!("{} capability files found", rs_files.len()));
            } else {
                errors.push("No capability files found".to_string());
            }
        }

        if errors.is_empty() {
            Ok(passed)
        } else {
            Err(errors)
        }
    }

    /// Validate Payload struct definitions.
    ///
    /// # Arguments
    /// * `content` - Content of mod.rs file
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of valid payloads
    /// * `Err(Vec<String>)` - List of validation errors
    fn validate_payloads(content: &str) -> Result<Vec<String>, Vec<String>> {
        use regex::Regex;

        let mut errors = Vec::new();
        let mut passed = Vec::new();

        // Find all struct definitions with #[derive(...)]
        let derive_re = Regex::new(r"#\[derive\(([^)]+)\)\]\s*pub struct (\w+Payload)").unwrap();

        for cap in derive_re.captures_iter(content) {
            let derives = &cap[1];
            let struct_name = &cap[2];

            // Check if all required derives are present
            let has_serialize = derives.contains("Serialize");
            let has_deserialize = derives.contains("Deserialize");
            let has_type = derives.contains("Type");

            if has_serialize && has_deserialize && has_type {
                passed.push(format!("{} has correct derives", struct_name));
            } else {
                let mut missing = Vec::new();
                if !has_serialize {
                    missing.push("Serialize");
                }
                if !has_deserialize {
                    missing.push("Deserialize");
                }
                if !has_type {
                    missing.push("Type");
                }
                errors.push(format!(
                    "{} is missing derives: {}",
                    struct_name,
                    missing.join(", ")
                ));
            }
        }

        if errors.is_empty() && !passed.is_empty() {
            Ok(passed)
        } else if errors.is_empty() && passed.is_empty() {
            Err(vec!["No Payload structs found".to_string()])
        } else {
            Err(errors)
        }
    }

    /// Check extension registration.
    ///
    /// # Arguments
    /// * `extension_path` - Path to the extension directory
    /// * `extension_name` - Name of the extension
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of registration checks passed
    /// * `Err(Vec<String>)` - List of registration issues
    fn check_registration(
        extension_path: &Path,
        extension_name: &str,
    ) -> Result<Vec<String>, Vec<String>> {
        let mut passed = Vec::new();
        let mut errors = Vec::new();

        // Locate src-tauri/src directory from extension path
        let src_dir = extension_path
            .parent()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .ok_or_else(|| vec!["Unable to resolve src-tauri/src directory".to_string()])?;

        // 1. extensions/mod.rs should export the module
        let extensions_mod = src_dir.join("extensions/mod.rs");
        match std::fs::read_to_string(&extensions_mod) {
            Ok(content) => match syn::parse_file(&content) {
                Ok(parsed) => {
                    if module_declared(&parsed, extension_name) {
                        passed.push(format!(
                            "extensions/mod.rs exports module `{}`",
                            extension_name
                        ));
                    } else {
                        errors.push(format!(
                            "extensions/mod.rs missing `pub mod {}` declaration",
                            extension_name
                        ));
                    }
                }
                Err(err) => errors.push(format!(
                    "Failed to parse {}: {}",
                    extensions_mod.display(),
                    err
                )),
            },
            Err(e) => errors.push(format!(
                "Failed to read {}: {}",
                extensions_mod.display(),
                e
            )),
        }

        // Gather expected capability struct names based on file names
        let mut expected_capabilities = Vec::new();
        if let Ok(entries) = std::fs::read_dir(extension_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                    continue;
                }
                if path.file_name().and_then(|s| s.to_str()) == Some("mod.rs") {
                    continue;
                }
                if path.file_name().and_then(|s| s.to_str()) == Some("tests.rs") {
                    continue;
                }
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let struct_name = format!("{}Capability", to_pascal_case(stem));
                    expected_capabilities.push(struct_name);
                }
            }
        }

        // 2. capabilities/registry.rs: check use statement and register calls
        let registry_path = src_dir.join("capabilities/registry.rs");
        match std::fs::read_to_string(&registry_path) {
            Ok(content) => match syn::parse_file(&content) {
                Ok(parsed) => {
                    let analysis = analyze_registry(&parsed, extension_name);

                    if analysis.use_found {
                        passed.push(format!(
                            "capabilities/registry.rs imports crate::extensions::{}::*",
                            extension_name
                        ));
                    } else {
                        errors.push(format!(
                            "capabilities/registry.rs missing `use crate::extensions::{}::*;` import",
                            extension_name
                        ));
                    }

                    if !analysis.has_registry_impl {
                        errors.push(
                            "capabilities/registry.rs missing CapabilityRegistry implementation"
                                .to_string(),
                        );
                    } else if !analysis.has_register_method {
                        errors.push(
                            "capabilities/registry.rs missing register_extensions method"
                                .to_string(),
                        );
                    }

                    for struct_name in &expected_capabilities {
                        if analysis.registered_capabilities.contains(struct_name) {
                            passed.push(format!(
                                "capabilities/registry.rs registers {}",
                                struct_name
                            ));
                        } else {
                            errors.push(format!(
                                "capabilities/registry.rs missing registration for {}",
                                struct_name
                            ));
                        }
                    }
                }
                Err(err) => errors.push(format!(
                    "Failed to parse {}: {}",
                    registry_path.display(),
                    err
                )),
            },
            Err(e) => errors.push(format!("Failed to read {}: {}", registry_path.display(), e)),
        }

        // 3. lib.rs should export Specta types for the extension
        let lib_path = src_dir.join("lib.rs");
        match std::fs::read_to_string(&lib_path) {
            Ok(content) => {
                let needle = format!("extensions::{}::", extension_name);
                if content.contains(&needle) {
                    passed.push(format!(
                        "lib.rs registers Specta types for extensions::{}",
                        extension_name
                    ));
                } else {
                    errors.push(format!(
                        "lib.rs missing Specta type registration for extensions::{}::<Payload>",
                        extension_name
                    ));
                }
            }
            Err(e) => errors.push(format!("Failed to read {}: {}", lib_path.display(), e)),
        }

        if errors.is_empty() {
            Ok(passed)
        } else {
            Err(errors)
        }
    }

    /// Check test coverage.
    ///
    /// # Arguments
    /// * `path` - Extension directory path
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - Coverage checks passed
    /// * `Err(Vec<String>)` - Coverage issues
    fn check_test_coverage(path: &Path) -> Result<Vec<String>, Vec<String>> {
        let mod_path = path.join("mod.rs");

        if !mod_path.exists() {
            return Err(vec!["mod.rs not found".to_string()]);
        }

        let content = std::fs::read_to_string(&mod_path)
            .map_err(|e| vec![format!("Failed to read mod.rs: {}", e)])?;

        let has_cfg_test = content.contains("#[cfg(test)]");
        let has_test_mod = content.contains("mod tests");

        if has_cfg_test && has_test_mod {
            Ok(vec!["Test module found".to_string()])
        } else {
            Err(vec!["No test module found in mod.rs".to_string()])
        }
    }
}

/// Analysis result when examining `capabilities/registry.rs`.
struct RegistryAnalysis {
    use_found: bool,
    has_registry_impl: bool,
    has_register_method: bool,
    registered_capabilities: HashSet<String>,
}

fn module_declared(file: &File, extension_name: &str) -> bool {
    file.items.iter().any(|item| {
        if let Item::Mod(ItemMod { ident, vis, .. }) = item {
            ident == extension_name && matches!(vis, Visibility::Public(_))
        } else {
            false
        }
    })
}

fn analyze_registry(file: &File, extension_name: &str) -> RegistryAnalysis {
    let mut use_found = false;
    let mut has_registry_impl = false;
    let mut has_register_method = false;
    let mut registered_capabilities = HashSet::new();

    for item in &file.items {
        if let Item::Use(item_use) = item {
            if use_tree_contains_extension_glob(&item_use.tree, extension_name, Vec::new()) {
                use_found = true;
            }
        }

        if let Item::Impl(item_impl) = item {
            if item_impl.trait_.is_some() {
                continue;
            }

            if let Type::Path(type_path) = &*item_impl.self_ty {
                if type_path
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident == "CapabilityRegistry")
                    == Some(true)
                {
                    has_registry_impl = true;
                    for impl_item in &item_impl.items {
                        if let ImplItem::Fn(method) = impl_item {
                            if method.sig.ident == "register_extensions" {
                                has_register_method = true;
                                for stmt in &method.block.stmts {
                                    if let Stmt::Item(Item::Use(item_use)) = stmt {
                                        if use_tree_contains_extension_glob(
                                            &item_use.tree,
                                            extension_name,
                                            Vec::new(),
                                        ) {
                                            use_found = true;
                                        }
                                    }
                                }
                                let mut collector = RegisterCallCollector {
                                    registered: &mut registered_capabilities,
                                };
                                collector.visit_block(&method.block);
                            }
                        }
                    }
                }
            }
        }
    }

    RegistryAnalysis {
        use_found,
        has_registry_impl,
        has_register_method,
        registered_capabilities,
    }
}

fn use_tree_contains_extension_glob(
    tree: &UseTree,
    extension_name: &str,
    prefix: Vec<String>,
) -> bool {
    match tree {
        UseTree::Path(use_path) => {
            let mut prefix = prefix;
            prefix.push(use_path.ident.to_string());
            use_tree_contains_extension_glob(&use_path.tree, extension_name, prefix)
        }
        UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| use_tree_contains_extension_glob(item, extension_name, prefix.clone())),
        UseTree::Glob(_) => prefix == ["crate", "extensions", extension_name],
        UseTree::Name(_) | UseTree::Rename(_) => false,
    }
}

struct RegisterCallCollector<'a> {
    registered: &'a mut HashSet<String>,
}

impl<'a, 'ast> Visit<'ast> for RegisterCallCollector<'a> {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if node.method == "register" {
            if let Some(first_arg) = node.args.first() {
                if let Some(name) = extract_capability_name(first_arg) {
                    self.registered.insert(name);
                }
            }
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

fn extract_capability_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Call(call) => {
            if is_arc_new(call.func.as_ref()) {
                call.args.first().and_then(|arg| match arg {
                    Expr::Path(path) => path.path.segments.last().map(|seg| seg.ident.to_string()),
                    Expr::Struct(expr_struct) => expr_struct
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string()),
                    _ => None,
                })
            } else {
                None
            }
        }
        Expr::Path(path) => path.path.segments.last().map(|seg| seg.ident.to_string()),
        _ => None,
    }
}

fn is_arc_new(expr: &Expr) -> bool {
    if let Expr::Path(ExprPath { path, .. }) = expr {
        if path.segments.len() == 2 {
            let mut iter = path.segments.iter();
            if let (Some(first), Some(second)) = (iter.next(), iter.next()) {
                return first.ident == "Arc" && second.ident == "new";
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ========================================
    // Test: check_files_exist() - all present
    // (Following dev-plan.md line 788-797)
    // ========================================

    #[test]
    fn test_check_files_exist_all_present() {
        let temp = TempDir::new().unwrap();
        let ext_path = temp.path().join("todo");
        fs::create_dir(&ext_path).unwrap();
        fs::write(ext_path.join("mod.rs"), "").unwrap();
        fs::write(ext_path.join("todo_add.rs"), "").unwrap();

        let result = Validator::check_files_exist(&ext_path);
        assert!(result.is_ok());
    }

    // ========================================
    // Test: check_files_exist() - missing mod
    // (Following dev-plan.md line 799-809)
    // ========================================

    #[test]
    fn test_check_files_exist_missing_mod() {
        let temp = TempDir::new().unwrap();
        let ext_path = temp.path().join("todo");
        fs::create_dir(&ext_path).unwrap();
        // mod.rs 不存在

        let result = Validator::check_files_exist(&ext_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("mod.rs")));
    }

    // ========================================
    // Test: validate_payloads() - correct
    // (Following dev-plan.md line 811-822)
    // ========================================

    #[test]
    fn test_validate_payloads_correct() {
        let content = r#"
        #[derive(Serialize, Deserialize, Type)]
        pub struct TodoAddPayload {
            pub text: String,
        }
        "#;

        let result = Validator::validate_payloads(content);
        assert!(result.is_ok());
    }

    // ========================================
    // Test: validate_payloads() - missing Type
    // (Following dev-plan.md line 824-836)
    // ========================================

    #[test]
    fn test_validate_payloads_missing_type_derive() {
        let content = r#"
        #[derive(Serialize, Deserialize)]
        pub struct TodoAddPayload {
            pub text: String,
        }
        "#;

        let result = Validator::validate_payloads(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("Type")));
    }

    // ========================================
    // Test: ValidationReport structure
    // (Following dev-plan.md line 838-849)
    // ========================================

    #[test]
    fn test_validation_report_summary() {
        let report = ValidationReport {
            passed: vec!["Files exist".to_string(), "Payloads correct".to_string()],
            failed: vec!["Registration missing".to_string()],
            warnings: vec!["Missing doc comments".to_string()],
        };

        assert_eq!(report.passed.len(), 2);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.warnings.len(), 1);
    }

    // ========================================
    // Tests: check_registration()
    // ========================================

    fn create_minimal_project_structure(
        temp: &TempDir,
        extension_name: &str,
        capability_files: &[(&str, &str)],
    ) -> std::path::PathBuf {
        let src_dir = temp.path().join("src-tauri/src");
        let extensions_dir = src_dir.join("extensions");
        let capabilities_dir = src_dir.join("capabilities");

        fs::create_dir_all(&extensions_dir).unwrap();
        fs::create_dir_all(&capabilities_dir).unwrap();

        // extensions/mod.rs
        fs::write(
            extensions_dir.join("mod.rs"),
            format!("pub mod {};\n", extension_name),
        )
        .unwrap();

        // Extension directory + capability files
        let extension_path = extensions_dir.join(extension_name);
        fs::create_dir_all(&extension_path).unwrap();
        fs::write(extension_path.join("mod.rs"), "// payloads\n").unwrap();

        for (file_name, content) in capability_files {
            fs::write(extension_path.join(file_name), content).unwrap();
        }

        // capabilities/registry.rs
        let registry_content = format!(
            r#"
use crate::extensions::{ext}::*;
use std::sync::Arc;

pub struct CapabilityRegistry;

impl CapabilityRegistry {{
    pub fn register_extensions(&mut self) {{
        self.register(Arc::new({struct_name}));
    }}

    fn register<T>(&mut self, _cap: Arc<T>) {{}}
}}
"#,
            ext = extension_name,
            struct_name = "TodoAddCapability"
        );
        fs::write(capabilities_dir.join("registry.rs"), registry_content).unwrap();

        // lib.rs
        let lib_content = format!(
            r#"
#[cfg(debug_assertions)]
fn register_types(builder: &mut Vec<String>) {{
    builder.push(format!("{{}}", std::any::type_name::<extensions::{ext}::TodoAddPayload>()));
}}
"#,
            ext = extension_name
        );
        fs::write(src_dir.join("lib.rs"), lib_content).unwrap();

        extension_path
    }

    #[test]
    fn test_check_registration_success() {
        let temp = TempDir::new().unwrap();

        let extension_path = create_minimal_project_structure(
            &temp,
            "todo",
            &[("todo_add.rs", "// capability file")],
        );

        let result = Validator::check_registration(&extension_path, "todo");
        assert!(
            result.is_ok(),
            "expected registration checks to pass: {:?}",
            result
        );
    }

    #[test]
    fn test_check_registration_missing_entries() {
        let temp = TempDir::new().unwrap();

        // Create structure but omit registry/lib registrations
        let src_dir = temp.path().join("src-tauri/src");
        let extensions_dir = src_dir.join("extensions");
        fs::create_dir_all(&extensions_dir).unwrap();
        fs::write(extensions_dir.join("mod.rs"), "pub mod todo;\n").unwrap();
        let extension_path = extensions_dir.join("todo");
        fs::create_dir_all(&extension_path).unwrap();
        fs::write(extension_path.join("mod.rs"), "// payloads\n").unwrap();
        fs::write(extension_path.join("todo_add.rs"), "// capability\n").unwrap();

        // Missing registry/lib content
        fs::create_dir_all(src_dir.join("capabilities")).unwrap();
        fs::write(src_dir.join("capabilities/registry.rs"), "// empty").unwrap();
        fs::write(src_dir.join("lib.rs"), "// empty").unwrap();

        let result = Validator::check_registration(&extension_path, "todo");
        assert!(result.is_err(), "expected registration check to fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains("missing")),
            "expected missing registration errors, got {:?}",
            errors
        );
    }
}

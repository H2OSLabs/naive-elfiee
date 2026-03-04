/// Configuration model for Extension generation.
///
/// This module defines the structure and validation rules for extension configurations.
use serde::{Deserialize, Serialize};

/// Extension configuration structure.
///
/// Represents the configuration for a new extension, including its name and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfig {
    /// Extension name (must be valid identifier without spaces)
    pub name: String,

    /// Target block_type
    pub block_type: String,

    /// List of capability IDs (e.g., ["add_item", "toggle_item"])
    pub capabilities: Vec<String>,

    /// Whether to generate authorization tests
    #[serde(default = "default_true")]
    pub with_auth_tests: bool,

    /// Whether to generate workflow integration tests
    #[serde(default = "default_true")]
    pub with_workflow_tests: bool,
}

fn default_true() -> bool {
    true
}

impl ExtensionConfig {
    /// Create a new ExtensionConfig with defaults.
    ///
    /// # Arguments
    /// * `name` - Extension name
    /// * `block_type` - Block type
    /// * `capabilities` - List of capability IDs
    ///
    /// # Examples
    /// ```
    /// use elfiee_ext_gen::models::config::ExtensionConfig;
    ///
    /// let config = ExtensionConfig::new("todo", "todo", vec!["add_item".to_string()]);
    /// assert_eq!(config.name, "todo");
    /// assert_eq!(config.block_type, "todo");
    /// assert!(config.with_auth_tests);
    /// ```
    pub fn new(
        name: impl Into<String>,
        block_type: impl Into<String>,
        capabilities: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            block_type: block_type.into(),
            capabilities,
            with_auth_tests: true,
            with_workflow_tests: true,
        }
    }

    /// Validate the configuration.
    ///
    /// # Returns
    /// * `Ok(())` if configuration is valid
    /// * `Err(Vec<String>)` with list of validation errors if invalid
    ///
    /// # Validation Rules
    /// 1. Name must not be empty
    /// 2. Name must not contain spaces or invalid characters
    /// 3. Capabilities list must not be empty
    /// 4. Each capability must not be empty
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 1. Name must not be empty
        if self.name.is_empty() {
            errors.push("Extension name cannot be empty".to_string());
        }

        // 2. Name must not contain spaces
        if self.name.contains(' ') {
            errors.push("Extension name cannot contain spaces".to_string());
        }

        // 3. Name must be valid identifier (alphanumeric + underscore only)
        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            errors.push(format!(
                "Extension name '{}' contains invalid characters. Only alphanumeric and underscore allowed.",
                self.name
            ));
        }

        // 4. Name should not start with a digit
        if let Some(first_char) = self.name.chars().next() {
            if first_char.is_numeric() {
                errors.push("Extension name cannot start with a digit".to_string());
            }
        }

        // 5. Block type validation (same rules as name)
        if self.block_type.is_empty() {
            errors.push("Block type cannot be empty".to_string());
        }

        // 6. Capabilities list must not be empty
        if self.capabilities.is_empty() {
            errors.push("Extension must have at least one capability".to_string());
        }

        // 7. Each capability must not be empty
        for (idx, cap) in self.capabilities.iter().enumerate() {
            if cap.is_empty() {
                errors.push(format!("Capability at index {} is empty", idx));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Tests for ExtensionConfig validation
    // (Following dev-plan.md Phase 1.4)
    // ========================================

    #[test]
    fn test_valid_config() {
        let config = ExtensionConfig {
            name: "todo".to_string(),
            block_type: "todo".to_string(),
            capabilities: vec!["add_item".to_string()],
            with_auth_tests: true,
            with_workflow_tests: true,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_name_with_spaces() {
        let config = ExtensionConfig {
            name: "todo items".to_string(),
            block_type: "todo".to_string(),
            capabilities: vec!["add".to_string()],
            with_auth_tests: true,
            with_workflow_tests: true,
        };

        let errors = config.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("spaces")));
    }

    #[test]
    fn test_empty_capabilities() {
        let config = ExtensionConfig {
            name: "todo".to_string(),
            block_type: "todo".to_string(),
            capabilities: vec![],
            with_auth_tests: true,
            with_workflow_tests: true,
        };

        let errors = config.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("at least one capability")));
    }
}

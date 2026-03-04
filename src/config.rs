/// Global configuration management for Elfiee
///
/// This module handles persistent configuration stored in the user's home directory.
/// Configuration is stored at: `$USER_HOME/.elf/config.json`
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Global configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Unique system editor ID for this machine
    /// This ID is generated once and persists across all file operations
    pub system_editor_id: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            system_editor_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

/// Get the path to the global config file
///
/// Returns: `$USER_HOME/.elf/config.json`
/// Respects `ELF_TEST_CONFIG_PATH` environment variable for testing.
fn get_config_path() -> Result<PathBuf, String> {
    if let Ok(test_path) = std::env::var("ELF_TEST_CONFIG_PATH") {
        return Ok(PathBuf::from(test_path));
    }

    let home_dir = dirs::home_dir().ok_or("Failed to get user home directory")?;
    let config_dir = home_dir.join(".elf");
    let config_path = config_dir.join("config.json");
    Ok(config_path)
}

/// Load global configuration from disk
///
/// If the config file doesn't exist, creates a new one with default values.
pub fn load_config() -> Result<GlobalConfig, String> {
    let config_path = get_config_path()?;

    // If config file exists, read it
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let config: GlobalConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;

        Ok(config)
    } else {
        // Create new config with default values
        let config = GlobalConfig::default();

        // Save to disk
        save_config(&config)?;

        Ok(config)
    }
}

/// Save global configuration to disk
///
/// Creates the `.elf` directory if it doesn't exist.
pub fn save_config(config: &GlobalConfig) -> Result<(), String> {
    let config_path = get_config_path()?;

    // Ensure the parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Serialize config to JSON
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    // Write to file
    fs::write(&config_path, content).map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Get the system editor ID for this machine
///
/// This is a convenience function that loads the config and returns the system editor ID.
/// If no config exists, creates one with a new UUID.
pub fn get_system_editor_id() -> Result<String, String> {
    let config = load_config()?;
    Ok(config.system_editor_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mutex to serialize tests that modify environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    // Helper to override config path for testing
    fn with_temp_config<F>(f: F)
    where
        F: FnOnce(&TempDir),
    {
        // Acquire lock to ensure exclusive access to environment variable
        let _guard = ENV_MUTEX.lock().unwrap();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.json");

        // Set environment variable to override path
        unsafe {
            std::env::set_var("ELF_TEST_CONFIG_PATH", config_path.to_str().unwrap());
        }

        f(&temp_dir);

        // Clean up environment variable
        unsafe {
            std::env::remove_var("ELF_TEST_CONFIG_PATH");
        }
    }

    #[test]
    fn test_default_config_creates_uuid() {
        // Run in temp env to avoid reading real config
        with_temp_config(|_| {
            let config = GlobalConfig::default();
            // Verify it's a valid UUID format
            assert!(uuid::Uuid::parse_str(&config.system_editor_id).is_ok());
        });
    }

    #[test]
    fn test_get_config_path() {
        // Test with temp config (env var set)
        with_temp_config(|temp_dir| {
            let path = get_config_path().expect("Failed to get config path");
            let expected = temp_dir.path().join("config.json");
            assert_eq!(path, expected);
        });

        // Also test default path (when env var is unset)
        // Note: This relies on home_dir existing
        // Use mutex to ensure env var is not set by other concurrent tests
        let _guard = ENV_MUTEX.lock().unwrap();

        // Explicitly remove env var to ensure clean state
        unsafe {
            std::env::remove_var("ELF_TEST_CONFIG_PATH");
        }

        if let Some(home) = dirs::home_dir() {
            let path = get_config_path().expect("Failed to get config path");
            assert!(path.starts_with(home));
            assert!(path.to_string_lossy().contains(".elf"));
        }
    }

    #[test]
    fn test_save_and_load_config() {
        with_temp_config(|_temp_dir| {
            let original_config = GlobalConfig {
                system_editor_id: "test-id-12345".to_string(),
            };

            // Save config
            let save_result = save_config(&original_config);
            assert!(save_result.is_ok());

            // Load config
            let loaded_config = load_config().expect("Failed to load config");
            assert_eq!(loaded_config.system_editor_id, "test-id-12345");
        });
    }

    #[test]
    fn test_get_system_editor_id() {
        with_temp_config(|_| {
            let result = get_system_editor_id();
            // Should succeed with a UUID (it creates one if missing)
            assert!(result.is_ok());
            let id = result.unwrap();
            assert!(uuid::Uuid::parse_str(&id).is_ok());
        });
    }
}

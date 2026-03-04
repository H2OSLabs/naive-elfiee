/// File operation utilities for extension generation.
///
/// This module provides safe file system operations with proper error handling.
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// File operations wrapper with error handling.
pub struct FileOperations;

impl FileOperations {
    /// Create a directory and all parent directories if they don't exist.
    ///
    /// # Arguments
    /// * `path` - The directory path to create
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(String)` with error description if failed
    ///
    /// # Examples
    /// ```no_run
    /// use elfiee_ext_gen::utils::file_ops::FileOperations;
    /// use std::path::Path;
    ///
    /// let path = Path::new("/tmp/a/b/c");
    /// FileOperations::ensure_dir(path).unwrap();
    /// assert!(path.exists());
    /// ```
    pub fn ensure_dir(path: &Path) -> Result<(), String> {
        fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create directory {}: {}", path.display(), e))
    }

    /// Write content to a file, creating parent directories if needed.
    ///
    /// # Arguments
    /// * `path` - File path to write to
    /// * `content` - Content to write
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(String)` with error description if failed
    pub fn write_file(path: &Path, content: &str) -> Result<(), String> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            Self::ensure_dir(parent)?;
        }

        fs::write(path, content)
            .map_err(|e| format!("Failed to write file {}: {}", path.display(), e))
    }

    /// Read the entire contents of a file.
    ///
    /// # Arguments
    /// * `path` - File path to read from
    ///
    /// # Returns
    /// * `Ok(String)` with file contents
    /// * `Err(String)` with error description if failed
    pub fn read_file(path: &Path) -> Result<String, String> {
        fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))
    }

    /// Check if a file exists.
    ///
    /// # Arguments
    /// * `path` - File path to check
    ///
    /// # Returns
    /// * `true` if file exists
    /// * `false` otherwise
    pub fn file_exists(path: &Path) -> bool {
        path.exists() && path.is_file()
    }

    /// List all Rust files (.rs) in a directory (non-recursive).
    ///
    /// # Arguments
    /// * `dir` - Directory to search
    ///
    /// # Returns
    /// * `Ok(Vec<PathBuf>)` with list of .rs files
    /// * `Err(String)` with error description if failed
    pub fn list_rust_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
        if !dir.exists() {
            return Err(format!("Directory does not exist: {}", dir.display()));
        }

        let mut rust_files = Vec::new();

        for entry in WalkDir::new(dir)
            .max_depth(1) // Non-recursive
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                rust_files.push(path.to_path_buf());
            }
        }

        Ok(rust_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ========================================
    // Tests for ensure_dir
    // ========================================

    #[test]
    fn test_ensure_dir_creates_directory() {
        let temp = TempDir::new().unwrap();
        let nested_path = temp.path().join("a/b/c");

        assert!(!nested_path.exists());
        FileOperations::ensure_dir(&nested_path).unwrap();
        assert!(nested_path.exists());
        assert!(nested_path.is_dir());
    }

    #[test]
    fn test_ensure_dir_idempotent() {
        let temp = TempDir::new().unwrap();
        let nested_path = temp.path().join("a/b/c");

        // Create once
        FileOperations::ensure_dir(&nested_path).unwrap();
        // Create again - should not fail
        FileOperations::ensure_dir(&nested_path).unwrap();

        assert!(nested_path.exists());
    }

    // ========================================
    // Tests for write_file and read_file
    // ========================================

    #[test]
    fn test_write_and_read_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");

        let content = "Hello, World!";
        FileOperations::write_file(&file_path, content).unwrap();

        let read_content = FileOperations::read_file(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_write_file_creates_parent_dirs() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("a/b/c/test.txt");

        assert!(!file_path.parent().unwrap().exists());

        FileOperations::write_file(&file_path, "Test content").unwrap();

        assert!(file_path.exists());
        assert!(file_path.parent().unwrap().exists());
    }

    #[test]
    fn test_write_file_overwrites_existing() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");

        FileOperations::write_file(&file_path, "First").unwrap();
        FileOperations::write_file(&file_path, "Second").unwrap();

        let content = FileOperations::read_file(&file_path).unwrap();
        assert_eq!(content, "Second");
    }

    #[test]
    fn test_read_file_nonexistent() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("nonexistent.txt");

        let result = FileOperations::read_file(&file_path);
        assert!(result.is_err());
    }

    // ========================================
    // Tests for file_exists
    // ========================================

    #[test]
    fn test_file_exists_true() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("exists.txt");

        FileOperations::write_file(&file_path, "content").unwrap();

        assert!(FileOperations::file_exists(&file_path));
    }

    #[test]
    fn test_file_exists_false() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("does_not_exist.txt");

        assert!(!FileOperations::file_exists(&file_path));
    }

    // ========================================
    // Tests for list_rust_files
    // ========================================

    #[test]
    fn test_list_rust_files_finds_rs_files() {
        let temp = TempDir::new().unwrap();

        FileOperations::write_file(&temp.path().join("a.rs"), "").unwrap();
        FileOperations::write_file(&temp.path().join("b.rs"), "").unwrap();
        FileOperations::write_file(&temp.path().join("c.txt"), "").unwrap();
        FileOperations::write_file(&temp.path().join("d.md"), "").unwrap();

        let rust_files = FileOperations::list_rust_files(temp.path()).unwrap();

        assert_eq!(rust_files.len(), 2);
        assert!(rust_files.iter().any(|p| p.ends_with("a.rs")));
        assert!(rust_files.iter().any(|p| p.ends_with("b.rs")));
        assert!(!rust_files.iter().any(|p| p.ends_with("c.txt")));
    }

    #[test]
    fn test_list_rust_files_empty_directory() {
        let temp = TempDir::new().unwrap();

        let rust_files = FileOperations::list_rust_files(temp.path()).unwrap();

        assert_eq!(rust_files.len(), 0);
    }

    #[test]
    fn test_list_rust_files_ignores_subdirectories() {
        let temp = TempDir::new().unwrap();

        FileOperations::write_file(&temp.path().join("top.rs"), "").unwrap();
        FileOperations::write_file(&temp.path().join("sub/nested.rs"), "").unwrap();

        let rust_files = FileOperations::list_rust_files(temp.path()).unwrap();

        // Should only find top.rs, not nested.rs
        assert_eq!(rust_files.len(), 1);
        assert!(rust_files.iter().any(|p| p.ends_with("top.rs")));
    }

    #[test]
    fn test_list_rust_files_nonexistent_directory() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("does_not_exist");

        let result = FileOperations::list_rust_files(&nonexistent);
        assert!(result.is_err());
    }
}

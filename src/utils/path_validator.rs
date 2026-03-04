use std::fs;
use std::path::Path;

/// 检查路径是否安全（防止路径遍历攻击）
pub fn is_safe_path(path: &Path) -> Result<(), String> {
    // 1. Check for symlinks BEFORE canonicalization

    // canonicalize() resolves symlinks, so we must check first.

    let metadata =
        fs::symlink_metadata(path).map_err(|e| format!("Failed to read metadata: {}", e))?;

    if metadata.is_symlink() {
        return Err("Symbolic links are not allowed".to_string());
    }

    // 2. Resolve path to check against forbidden directories

    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;

    // 3. Reject system sensitive directories

    #[cfg(unix)]
    let forbidden = ["/etc", "/sys", "/proc", "/dev", "/bin", "/sbin"];

    #[cfg(windows)]
    let forbidden = ["C:\\Windows\\System32", "C:\\Windows\\SysWOW64"];

    #[cfg(not(any(unix, windows)))]
    let forbidden: &[&str] = &[];

    for forbidden_dir in &forbidden {
        if canonical.starts_with(forbidden_dir) {
            return Err(format!(
                "Access to system directory is forbidden: {}",
                forbidden_dir
            ));
        }
    }

    Ok(())
}

/// Validates a single filename segment.
///
/// - Prevents empty names
/// - Prevents reserved names (Windows)
/// - Prevents illegal characters
pub fn validate_filename(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }

    // Windows reserved names
    #[cfg(windows)]
    {
        let reserved = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];
        let upper_name = name.to_uppercase();
        if reserved.contains(&upper_name.as_str()) {
            return Err(format!("'{}' is a reserved filename", name));
        }
    }

    // Illegal characters
    let illegal = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    if name.chars().any(|c| illegal.contains(&c)) {
        return Err(format!("Filename contains illegal characters: {}", name));
    }

    Ok(())
}

/// Validates a virtual path (VFS path used inside Directory Blocks).
///
/// - Prevents path traversal (contains '..')
/// - Prevents absolute paths (starts with '/')
/// - Prevents empty paths
/// - Validates each component as a valid filename
pub fn validate_virtual_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    if path.starts_with('/') {
        return Err("Virtual path cannot be absolute (start with '/')".to_string());
    }

    use std::path::Component;
    for component in Path::new(path).components() {
        match component {
            Component::Normal(name) => {
                validate_filename(name.to_str().unwrap_or(""))?;
            }
            Component::ParentDir => {
                return Err(format!("Invalid path (traversal forbidden): {}", path));
            }
            Component::RootDir => {
                return Err("Virtual path cannot be absolute".to_string());
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let result = is_safe_path(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_nonexistent_path() {
        let path = Path::new("/nonexistent/path/12345");
        let result = is_safe_path(path);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_forbidden_directory() {
        let path = Path::new("/etc/passwd");
        if path.exists() {
            let result = is_safe_path(path);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .contains("Access to system directory is forbidden"));
        }
    }

    #[test]
    fn test_validate_virtual_path() {
        // Valid paths
        assert!(validate_virtual_path("file.txt").is_ok());
        assert!(validate_virtual_path("docs/readme.md").is_ok());
        assert!(validate_virtual_path("a/b/c/d.rs").is_ok());

        // Empty path
        assert!(validate_virtual_path("").is_err());

        // Absolute path
        assert!(validate_virtual_path("/root/file.txt").is_err());

        // Traversal path
        assert!(validate_virtual_path("../secret.txt").is_err());
        assert!(validate_virtual_path("docs/../../etc/passwd").is_err());
        assert!(validate_virtual_path("a/./../b").is_err());
    }
}

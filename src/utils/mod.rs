pub mod block_type_inference;
pub mod path_validator;
/// Utility modules for Elfiee application
///
/// This module provides common utility functions used throughout the application.
pub mod time;

/// Cross-platform canonicalize that strips the `\\?\` prefix on Windows.
///
/// `std::fs::canonicalize()` on Windows returns extended-length paths like
/// `\\?\C:\Users\...` which break URL parsers (SQLite connection strings),
/// `strip_prefix` comparisons, and some file APIs.
pub fn safe_canonicalize(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let canonical = std::fs::canonicalize(path)?;
    #[cfg(windows)]
    {
        let s = canonical.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return Ok(std::path::PathBuf::from(stripped));
        }
    }
    Ok(canonical)
}

// Re-export ONLY the single primary public API
// All other functions are internal (pub(crate)) to enforce consistent time format usage
pub use time::{
    now_utc, // The ONLY public function: Get current UTC timestamp in RFC 3339 format
};

/// Infers the block type based on file extension.
pub use block_type_inference::infer_block_type;

/// Validates file paths to prevent traversal attacks and access to sensitive directories.
pub use path_validator::{is_safe_path, validate_virtual_path};

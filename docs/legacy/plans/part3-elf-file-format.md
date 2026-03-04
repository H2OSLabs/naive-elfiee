# Part 3: ELF File Format Implementation

**Priority**: High | **Estimated Time**: 3-4 days

## Overview

Implement `.elf` file as ZIP archive. For MVP: just extract to temp directory, use the event store directly. Re-zip on save. No fancy streaming.

## Directory Structure

```
src-tauri/src/
├── elf/
│   ├── mod.rs
│   └── archive.rs
└── lib.rs
```

## Step 1: Add ZIP Dependency

**File**: `src-tauri/Cargo.toml`

```toml
[dependencies]
zip = "0.6"
tempfile = "3.8"
```

## Step 2: Implement ELF Archive Handler

**File**: `src-tauri/src/elf/archive.rs`

```rust
use crate::engine::EventStore;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use zip::ZipArchive;

pub struct ElfArchive {
    work_dir: TempDir,
    original_path: Option<PathBuf>,
}

impl ElfArchive {
    /// Open an existing .elf file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let work_dir = TempDir::new()?;
        let file = File::open(&path)?;
        let mut archive = ZipArchive::new(file)?;

        // Extract all files to temp directory
        archive.extract(work_dir.path())?;

        Ok(Self {
            work_dir,
            original_path: Some(path.as_ref().to_path_buf()),
        })
    }

    /// Create a new .elf file
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let work_dir = TempDir::new()?;

        // Create _eventstore.db
        let db_path = work_dir.path().join("_eventstore.db");
        EventStore::new(db_path.to_str().unwrap())?;

        Ok(Self {
            work_dir,
            original_path: None,
        })
    }

    /// Get path to the event store database
    pub fn event_store_path(&self) -> PathBuf {
        self.work_dir.path().join("_eventstore.db")
    }

    /// Get path to a block's asset directory
    pub fn block_assets_path(&self, block_id: &str) -> PathBuf {
        self.work_dir.path().join(format!("block-{}", block_id))
    }

    /// Ensure block asset directory exists
    pub fn ensure_block_dir(&self, block_id: &str) -> Result<PathBuf, std::io::Error> {
        let dir = self.block_assets_path(block_id);
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Save the archive to a .elf file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(&path)?;
        let mut zip = zip::ZipWriter::new(file);

        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Walk the working directory and add all files
        self.add_directory_to_zip(&mut zip, self.work_dir.path(), "", options)?;

        zip.finish()?;
        Ok(())
    }

    /// Save to the original path (if opened from file)
    pub fn save_to_original(&self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.original_path {
            Some(path) => self.save(path),
            None => Err("No original path to save to".into()),
        }
    }

    // Helper to recursively add directory contents to ZIP
    fn add_directory_to_zip(
        &self,
        zip: &mut zip::ZipWriter<File>,
        dir: &Path,
        prefix: &str,
        options: zip::write::FileOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            let zip_path = if prefix.is_empty() {
                name_str.to_string()
            } else {
                format!("{}/{}", prefix, name_str)
            };

            if path.is_file() {
                zip.start_file(&zip_path, options)?;
                let mut f = File::open(&path)?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)?;
                zip.write_all(&buffer)?;
            } else if path.is_dir() {
                self.add_directory_to_zip(zip, &path, &zip_path, options)?;
            }
        }
        Ok(())
    }
}

impl Drop for ElfArchive {
    fn drop(&mut self) {
        // TempDir will auto-cleanup
    }
}
```

**Simple approach**: Extract to temp, work with files directly, re-zip on save. Good enough for MVP.

## Step 3: Wire Up Module

**File**: `src-tauri/src/elf/mod.rs`

```rust
mod archive;

pub use archive::ElfArchive;
```

**File**: `src-tauri/src/lib.rs`

```rust
pub mod models;
pub mod engine;
pub mod elf;
```

## Step 4: Write Basic Test

**File**: `src-tauri/src/elf/archive.rs` (add at bottom)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_and_save() {
        let archive = ElfArchive::new().unwrap();

        // Check event store exists
        assert!(archive.event_store_path().exists());

        // Save to temp file
        let temp_path = std::env::temp_dir().join("test.elf");
        archive.save(&temp_path).unwrap();

        assert!(temp_path.exists());

        // Cleanup
        fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_open_and_read() {
        // Create an archive
        let archive = ElfArchive::new().unwrap();
        let temp_path = std::env::temp_dir().join("test_open.elf");
        archive.save(&temp_path).unwrap();

        // Open it again
        let archive2 = ElfArchive::open(&temp_path).unwrap();
        assert!(archive2.event_store_path().exists());

        // Cleanup
        fs::remove_file(temp_path).ok();
    }
}
```

## Step 5: Run Tests

```bash
cd src-tauri
cargo test
```

**Expected**: Tests pass.

## Done

Simple ZIP-based .elf format:
- ✅ Extract to temp directory
- ✅ Work with files directly
- ✅ Re-zip on save
- ✅ Block asset directory management
- ❌ No streaming (YAGNI)
- ❌ No incremental updates (YAGNI)
- ❌ No compression optimization (default is fine)

**Deferred to later**:
- `_snapshot` generation (Part 4)
- `_blocks_hash` cache (optimize later if slow)
- `_blocks_relation` cache (optimize later if slow)

**Next**: Part 4 - Extension Interface (capability system)

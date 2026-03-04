use crate::elf_project::ElfProject;
use crate::engine::EngineManager;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Information about an open .elf/ project
#[derive(Clone)]
pub struct FileInfo {
    pub project: Arc<ElfProject>,
}

/// Application state shared across MCP server and CLI.
///
/// This state manages multiple open .elf projects and their corresponding engine actors.
/// Each project has a unique file_id and is managed independently.
#[derive(Clone)]
pub struct AppState {
    /// Engine manager for processing commands on .elf projects
    pub engine_manager: EngineManager,

    /// Map of file_id -> FileInfo for open projects
    /// Using DashMap for thread-safe concurrent access
    pub files: Arc<DashMap<String, FileInfo>>,

    /// Map of file_id -> active editor_id
    /// This is UI state and is NOT persisted to .elf file
    /// Using DashMap for thread-safe concurrent access
    pub active_editors: Arc<DashMap<String, String>>,

    /// Broadcast sender for state change notifications.
    /// MCP server and CLI send file_id here after successful commands.
    pub state_changed_tx: broadcast::Sender<String>,
}

impl AppState {
    /// Create a new application state with empty file list.
    pub fn new() -> Self {
        Self {
            engine_manager: EngineManager::new(),
            files: Arc::new(DashMap::new()),
            active_editors: Arc::new(DashMap::new()),
            state_changed_tx: broadcast::channel(256).0,
        }
    }

    /// Get the active editor for a file.
    ///
    /// Returns the editor_id of the currently active editor for the given file,
    /// or None if no editor is set as active.
    pub fn get_active_editor(&self, file_id: &str) -> Option<String> {
        self.active_editors.get(file_id).map(|e| e.value().clone())
    }

    /// Set the active editor for a file.
    ///
    /// This updates the UI state to track which editor is currently active
    /// for the given file. This state is NOT persisted to the .elf file.
    pub fn set_active_editor(&self, file_id: String, editor_id: String) {
        self.active_editors.insert(file_id, editor_id);
    }

    /// List all open files.
    ///
    /// Returns a vector of (file_id, path) tuples for all currently open files.
    pub fn list_open_files(&self) -> Vec<(String, String)> {
        self.files
            .iter()
            .map(|entry| {
                (
                    entry.key().clone(),
                    entry
                        .value()
                        .project
                        .project_dir()
                        .to_string_lossy()
                        .to_string(),
                )
            })
            .collect()
    }

    /// Get project by file_id.
    pub fn get_project(&self, file_id: &str) -> Option<Arc<ElfProject>> {
        self.files
            .get(file_id)
            .map(|entry| entry.value().project.clone())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

//! Project management service.
//!
//! Unified project open/close/list logic used by all three transport layers
//! (MCP Server, CLI).

use crate::capabilities::registry::CapabilityRegistry;
use crate::config;
use crate::elf_project::ElfProject;
use crate::engine::{EventPoolWithPath, EventStore};
use crate::models::Event;
use crate::state::{AppState, FileInfo};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Seed bootstrap events directly to EventStore for a new project.
///
/// Writes editor.create + wildcard core.grant events directly to the database,
/// bypassing the actor's command processing pipeline. This solves the chicken-and-egg
/// problem: the system editor needs grants to issue grants, but bootstrap creates those grants.
///
/// Must be called BEFORE spawn_engine() so the engine replays these events during init.
/// For existing files (events already present), this is a no-op.
pub async fn seed_bootstrap_events(event_pool: &EventPoolWithPath) -> Result<(), String> {
    let existing = EventStore::get_all_events(&event_pool.pool)
        .await
        .map_err(|e| format!("Failed to check existing events: {}", e))?;

    if !existing.is_empty() {
        return Ok(());
    }

    let system_id = config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());

    let mut events = Vec::new();

    // 1. editor.create event
    let mut ts = HashMap::new();
    ts.insert(system_id.clone(), 1);

    events.push(Event::new(
        system_id.clone(),
        format!("{}/editor.create", system_id),
        serde_json::json!({
            "editor_id": system_id,
            "name": "Owner",
            "editor_type": "Human"
        }),
        ts,
    ));

    // 2. core.grant wildcard events
    let registry = CapabilityRegistry::new();
    let cap_ids = registry.get_grantable_cap_ids(&[]);

    for (i, cap_id) in cap_ids.iter().enumerate() {
        let mut grant_ts = HashMap::new();
        grant_ts.insert(system_id.clone(), (i + 2) as i64);

        events.push(Event::new(
            "*".to_string(),
            format!("{}/core.grant", system_id),
            serde_json::json!({
                "editor": system_id,
                "capability": cap_id,
                "block": "*"
            }),
            grant_ts,
        ));
    }

    EventStore::append_events(&event_pool.pool, &events)
        .await
        .map_err(|e| format!("Failed to seed bootstrap events: {}", e))?;

    Ok(())
}

/// Open an .elf project and spawn its engine actor.
///
/// If the project directory doesn't exist, creates a new one.
/// If the project is already open, returns its existing file_id.
pub async fn open_project(path: &str, state: &AppState) -> Result<String, String> {
    // Check if already open
    let files = list_open_projects(state);
    for (file_id, open_path) in &files {
        if open_path == path {
            return Ok(file_id.clone());
        }
    }

    let file_id = format!("file-{}", uuid::Uuid::new_v4());
    let project_dir = Path::new(path);

    // Open existing project (NEVER auto-init — use `elf init` explicitly)
    let project = if project_dir.join(".elf").exists() {
        ElfProject::open(project_dir)?
    } else {
        return Err(format!(
            "Not an Elfiee project: '{}' (no .elf/ directory). Run `elf init` first.",
            path
        ));
    };

    let event_pool = project.event_pool().await?;

    // Seed bootstrap events BEFORE engine spawn
    seed_bootstrap_events(&event_pool).await?;

    // Spawn engine actor
    state
        .engine_manager
        .spawn_engine(file_id.clone(), event_pool)
        .await?;

    // Store project info
    state.files.insert(
        file_id.clone(),
        FileInfo {
            project: Arc::new(project),
        },
    );

    Ok(file_id)
}

/// Close an .elf project by path and release resources.
pub async fn close_project(path: &str, state: &AppState) -> Result<(), String> {
    let file_id = {
        let files = list_open_projects(state);
        files
            .iter()
            .find(|(_, p)| p == path)
            .map(|(id, _)| id.clone())
            .ok_or_else(|| format!("Project '{}' is not open", path))?
    };

    close_project_by_id(&file_id, state).await
}

/// Close an .elf project by file_id and release resources.
pub async fn close_project_by_id(file_id: &str, state: &AppState) -> Result<(), String> {
    state.engine_manager.shutdown_engine(file_id).await?;
    state.files.remove(file_id);
    state.active_editors.remove(file_id);
    Ok(())
}

/// List all currently open projects.
///
/// Returns a list of (file_id, project_path) pairs.
pub fn list_open_projects(state: &AppState) -> Vec<(String, String)> {
    state.list_open_files()
}

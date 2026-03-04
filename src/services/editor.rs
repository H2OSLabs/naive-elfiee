//! Editor 服务 — Editor 管理

use crate::engine::EngineHandle;
use crate::models::{Command, Editor, EditorType};

/// 列出所有 editors
pub async fn list_editors(handle: &EngineHandle) -> Vec<Editor> {
    let editors_map = handle.get_all_editors().await;
    editors_map.values().cloned().collect()
}

/// 获取单个 editor
pub async fn get_editor(handle: &EngineHandle, editor_id: &str) -> Result<Editor, String> {
    let editors = handle.get_all_editors().await;
    editors
        .get(editor_id)
        .cloned()
        .ok_or_else(|| format!("Editor '{}' not found", editor_id))
}

/// 创建 editor，返回新创建的 Editor
pub async fn create_editor(
    handle: &EngineHandle,
    creator_id: &str,
    name: &str,
    editor_type: Option<&str>,
    editor_id: Option<&str>,
) -> Result<Editor, String> {
    let mut payload = serde_json::json!({ "name": name });
    if let Some(et) = editor_type {
        payload["editor_type"] = serde_json::json!(et);
    }
    if let Some(eid) = editor_id {
        payload["editor_id"] = serde_json::json!(eid);
    }

    let cmd = Command::new(
        creator_id.to_string(),
        "editor.create".to_string(),
        "".to_string(),
        payload,
    );

    let events = handle.process_command(cmd).await?;

    if let Some(event) = events.first() {
        let new_editor_id = event.entity.clone();
        let editor_name = event
            .value
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing name in event")?
            .to_string();

        let editor_type_str = event
            .value
            .get("editor_type")
            .and_then(|v| v.as_str())
            .ok_or("Missing editor_type in event")?;

        let editor_type = match editor_type_str {
            "Bot" => EditorType::Bot,
            "Human" => EditorType::Human,
            other => return Err(format!("Unknown editor_type '{}'", other)),
        };

        Ok(Editor {
            editor_id: new_editor_id,
            name: editor_name,
            editor_type,
        })
    } else {
        Err("No events generated for editor creation".to_string())
    }
}

/// 删除 editor
pub async fn delete_editor(
    handle: &EngineHandle,
    deleter_id: &str,
    target_editor_id: &str,
) -> Result<(), String> {
    let cmd = Command::new(
        deleter_id.to_string(),
        "editor.delete".to_string(),
        "".to_string(),
        serde_json::json!({ "editor_id": target_editor_id }),
    );

    handle.process_command(cmd).await?;
    Ok(())
}

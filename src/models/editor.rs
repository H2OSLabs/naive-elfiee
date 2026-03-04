use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum EditorType {
    #[default]
    Human,
    Bot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Editor {
    pub editor_id: String,
    pub name: String,
    pub editor_type: EditorType,
}

impl Editor {
    pub fn new(name: String) -> Self {
        Self {
            editor_id: uuid::Uuid::new_v4().to_string(),
            name,
            editor_type: EditorType::Human,
        }
    }

    pub fn new_with_type(name: String, editor_type: EditorType) -> Self {
        Self {
            editor_id: uuid::Uuid::new_v4().to_string(),
            name,
            editor_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_type_default() {
        let default_type = EditorType::default();
        assert_eq!(default_type, EditorType::Human);
    }

    #[test]
    fn test_editor_new_creates_human_by_default() {
        let editor = Editor::new("Alice".to_string());
        assert_eq!(editor.name, "Alice");
        assert_eq!(editor.editor_type, EditorType::Human);
        assert!(!editor.editor_id.is_empty());
    }

    #[test]
    fn test_editor_new_with_type_human() {
        let editor = Editor::new_with_type("Bob".to_string(), EditorType::Human);
        assert_eq!(editor.name, "Bob");
        assert_eq!(editor.editor_type, EditorType::Human);
    }

    #[test]
    fn test_editor_new_with_type_bot() {
        let editor = Editor::new_with_type("CodeReviewer".to_string(), EditorType::Bot);
        assert_eq!(editor.name, "CodeReviewer");
        assert_eq!(editor.editor_type, EditorType::Bot);
    }

    #[test]
    fn test_editor_serialization() {
        let editor = Editor::new_with_type("Alice".to_string(), EditorType::Bot);
        let json = serde_json::to_string(&editor).unwrap();

        // Verify JSON contains editor_type
        assert!(json.contains("\"editor_type\":\"Bot\""));
        assert!(json.contains("\"name\":\"Alice\""));
    }

    #[test]
    fn test_editor_deserialization_with_type() {
        let json = r#"{
            "editor_id": "test-id",
            "name": "Alice",
            "editor_type": "Bot"
        }"#;

        let editor: Editor = serde_json::from_str(json).unwrap();
        assert_eq!(editor.editor_id, "test-id");
        assert_eq!(editor.name, "Alice");
        assert_eq!(editor.editor_type, EditorType::Bot);
    }

    #[test]
    fn test_editor_deserialization_without_type_fails() {
        // editor_type 是必填字段，缺少时应反序列化失败
        let json = r#"{
            "editor_id": "test-id",
            "name": "Alice"
        }"#;

        let result: Result<Editor, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_editor_type_serialization() {
        let human = EditorType::Human;
        let bot = EditorType::Bot;

        assert_eq!(serde_json::to_string(&human).unwrap(), "\"Human\"");
        assert_eq!(serde_json::to_string(&bot).unwrap(), "\"Bot\"");
    }

    #[test]
    fn test_editor_type_deserialization() {
        let human: EditorType = serde_json::from_str("\"Human\"").unwrap();
        let bot: EditorType = serde_json::from_str("\"Bot\"").unwrap();

        assert_eq!(human, EditorType::Human);
        assert_eq!(bot, EditorType::Bot);
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Event 值的存储模式。
///
/// 决定 Event.value 中内容的格式和 StateProjector 的处理方式。
///
/// - `Full`：完整状态快照（创建时、Task Block）
/// - `Delta`：增量差异（Document Block 的文本修改）
/// - `Ref`：外部引用（二进制文件，只存 hash/path/size）
/// - `Append`：追加条目（Session Block 的 append-only 语义）
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EventMode {
    #[default]
    Full,
    Delta,
    Ref,
    Append,
}

impl EventMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventMode::Full => "full",
            EventMode::Delta => "delta",
            EventMode::Ref => "ref",
            EventMode::Append => "append",
        }
    }
}

impl fmt::Display for EventMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for EventMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EventMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "full" => Ok(EventMode::Full),
            "delta" => Ok(EventMode::Delta),
            "ref" => Ok(EventMode::Ref),
            "append" => Ok(EventMode::Append),
            other => Err(serde::de::Error::custom(format!(
                "unknown event mode '{}', expected one of: full, delta, ref, append",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub entity: String,
    pub attribute: String,
    pub value: serde_json::Value,
    pub timestamp: HashMap<String, i64>, // Vector clock
    pub created_at: String,              // Wall clock time (ISO 8601)

    /// Event 值的存储模式（full / delta / ref / append）。
    pub mode: EventMode,
}

impl Event {
    pub fn new(
        entity: String,
        attribute: String,
        value: serde_json::Value,
        timestamp: HashMap<String, i64>,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            entity,
            attribute,
            value,
            timestamp,
            created_at: crate::utils::time::now_utc(),
            mode: EventMode::Full,
        }
    }

    /// 创建指定 mode 的 Event。
    pub fn new_with_mode(
        entity: String,
        attribute: String,
        value: serde_json::Value,
        timestamp: HashMap<String, i64>,
        mode: EventMode,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            entity,
            attribute,
            value,
            timestamp,
            created_at: crate::utils::time::now_utc(),
            mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_mode_default_is_full() {
        assert_eq!(EventMode::default(), EventMode::Full);
    }

    #[test]
    fn test_event_mode_as_str() {
        assert_eq!(EventMode::Full.as_str(), "full");
        assert_eq!(EventMode::Delta.as_str(), "delta");
        assert_eq!(EventMode::Ref.as_str(), "ref");
        assert_eq!(EventMode::Append.as_str(), "append");
    }

    #[test]
    fn test_event_mode_display() {
        assert_eq!(format!("{}", EventMode::Full), "full");
        assert_eq!(format!("{}", EventMode::Append), "append");
    }

    #[test]
    fn test_event_mode_serialization() {
        assert_eq!(serde_json::to_string(&EventMode::Full).unwrap(), "\"full\"");
        assert_eq!(
            serde_json::to_string(&EventMode::Delta).unwrap(),
            "\"delta\""
        );
        assert_eq!(serde_json::to_string(&EventMode::Ref).unwrap(), "\"ref\"");
        assert_eq!(
            serde_json::to_string(&EventMode::Append).unwrap(),
            "\"append\""
        );
    }

    #[test]
    fn test_event_mode_deserialization() {
        let full: EventMode = serde_json::from_str("\"full\"").unwrap();
        assert_eq!(full, EventMode::Full);

        let delta: EventMode = serde_json::from_str("\"delta\"").unwrap();
        assert_eq!(delta, EventMode::Delta);

        let r: EventMode = serde_json::from_str("\"ref\"").unwrap();
        assert_eq!(r, EventMode::Ref);

        let append: EventMode = serde_json::from_str("\"append\"").unwrap();
        assert_eq!(append, EventMode::Append);
    }

    #[test]
    fn test_event_mode_deserialization_invalid() {
        let result: Result<EventMode, _> = serde_json::from_str("\"unknown\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_event_new_defaults_to_full() {
        let event = Event::new(
            "block-1".to_string(),
            "alice/document.write".to_string(),
            serde_json::json!({"content": "hello"}),
            HashMap::from([("alice".to_string(), 1)]),
        );
        assert_eq!(event.mode, EventMode::Full);
    }

    #[test]
    fn test_event_new_with_mode() {
        let event = Event::new_with_mode(
            "session-1".to_string(),
            "bot/session.append".to_string(),
            serde_json::json!({"entry_type": "command"}),
            HashMap::from([("bot".to_string(), 5)]),
            EventMode::Append,
        );
        assert_eq!(event.mode, EventMode::Append);
    }

    #[test]
    fn test_event_serialization_includes_mode() {
        let event = Event::new_with_mode(
            "block-1".to_string(),
            "alice/document.write".to_string(),
            serde_json::json!({}),
            HashMap::new(),
            EventMode::Delta,
        );

        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["mode"], "delta");
    }
}

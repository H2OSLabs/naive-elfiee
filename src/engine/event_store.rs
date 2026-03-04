use crate::models::{Event, EventMode};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::PathBuf;
use std::str::FromStr;

/// Event pool with database file path
///
/// This structure wraps SqlitePool with the database file path,
/// allowing the engine to derive temp_dir at runtime.
#[derive(Clone)]
pub struct EventPoolWithPath {
    /// SQLite connection pool for event storage
    pub pool: SqlitePool,

    /// Path to the events.db file (e.g., /tmp/xyz789/events.db)
    pub db_path: PathBuf,
}

/// Event store for persisting events to SQLite database.
///
/// This implementation uses sqlx for async database operations,
/// making it compatible with tokio runtime and safe to use across threads.
pub struct EventStore;

impl EventStore {
    /// Create a new event store and initialize the database schema.
    ///
    /// The path can be:
    /// - A file path like "events.db" or "./data/events.db"
    /// - ":memory:" for in-memory database (testing)
    ///
    /// Returns an EventPoolWithPath containing both the pool and db_path.
    pub async fn create(path: &str) -> Result<EventPoolWithPath, sqlx::Error> {
        let options = if path == ":memory:" {
            SqliteConnectOptions::from_str("sqlite::memory:")?
        } else {
            // Ensure parent directory exists
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent).map_err(sqlx::Error::Io)?;
            }

            // Use filename() to avoid URL-parsing issues with Windows paths
            SqliteConnectOptions::new().filename(path)
        }
        .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        // Initialize schema
        Self::init_schema(&pool).await?;

        // Return both pool and path
        Ok(EventPoolWithPath {
            pool,
            db_path: PathBuf::from(path),
        })
    }

    /// Initialize the database schema (tables and indexes).
    async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        // Create events table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS events (
                event_id TEXT PRIMARY KEY,
                entity TEXT NOT NULL,
                attribute TEXT NOT NULL,
                value TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                created_at TEXT NOT NULL,
                mode TEXT NOT NULL DEFAULT 'full'
            )",
        )
        .execute(pool)
        .await?;

        // Create index on entity for faster lookups
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_entity ON events(entity)")
            .execute(pool)
            .await?;

        // Create index on attribute
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_attribute ON events(attribute)")
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Append events to the database.
    pub async fn append_events(pool: &SqlitePool, events: &[Event]) -> Result<(), sqlx::Error> {
        for event in events {
            let timestamp_json = serde_json::to_string(&event.timestamp)
                .map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
            let value_json = serde_json::to_string(&event.value)
                .map_err(|e| sqlx::Error::Encode(Box::new(e)))?;

            sqlx::query(
                "INSERT INTO events (event_id, entity, attribute, value, timestamp, created_at, mode)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&event.event_id)
            .bind(&event.entity)
            .bind(&event.attribute)
            .bind(&value_json)
            .bind(&timestamp_json)
            .bind(&event.created_at)
            .bind(event.mode.as_str())
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    /// Get all events from the database, ordered by insertion order (rowid).
    pub async fn get_all_events(pool: &SqlitePool) -> Result<Vec<Event>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT event_id, entity, attribute, value, timestamp, created_at, mode
             FROM events
             ORDER BY rowid",
        )
        .fetch_all(pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            let event = Self::row_to_event(row)?;
            events.push(event);
        }

        Ok(events)
    }

    /// Get events for a specific entity.
    pub async fn get_events_by_entity(
        pool: &SqlitePool,
        entity: &str,
    ) -> Result<Vec<Event>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT event_id, entity, attribute, value, timestamp, created_at, mode
             FROM events
             WHERE entity = $1
             ORDER BY rowid",
        )
        .bind(entity)
        .fetch_all(pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            let event = Self::row_to_event(row)?;
            events.push(event);
        }

        Ok(events)
    }

    /// 获取指定 event_id 之后的所有 events（按插入顺序）。
    ///
    /// 用于快照基线回放：加载最近快照后，只回放后续的 events。
    /// 使用 rowid 子查询确保按插入顺序正确截断。
    pub async fn get_events_after_event_id(
        pool: &SqlitePool,
        after_event_id: &str,
    ) -> Result<Vec<Event>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT event_id, entity, attribute, value, timestamp, created_at, mode
             FROM events
             WHERE rowid > (SELECT rowid FROM events WHERE event_id = $1)
             ORDER BY rowid",
        )
        .bind(after_event_id)
        .fetch_all(pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            let event = Self::row_to_event(row)?;
            events.push(event);
        }

        Ok(events)
    }

    /// 获取最新 event 的 ID（按插入顺序）。
    ///
    /// 用于保存快照时记录当前位置。返回 None 表示 event store 为空。
    pub async fn get_latest_event_id(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT event_id FROM events ORDER BY rowid DESC LIMIT 1")
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| r.try_get::<String, _>(0).unwrap_or_default()))
    }

    /// Convert a database row to an Event.
    fn row_to_event(row: sqlx::sqlite::SqliteRow) -> Result<Event, sqlx::Error> {
        let event_id: String = row.try_get(0)?;
        let entity: String = row.try_get(1)?;
        let attribute: String = row.try_get(2)?;
        let value_json: String = row.try_get(3)?;
        let timestamp_json: String = row.try_get(4)?;
        let created_at: String = row.try_get(5)?;
        let mode_str: String = row.try_get(6)?;

        let value: serde_json::Value =
            serde_json::from_str(&value_json).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let timestamp =
            serde_json::from_str(&timestamp_json).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let mode = match mode_str.as_str() {
            "delta" => EventMode::Delta,
            "ref" => EventMode::Ref,
            "append" => EventMode::Append,
            _ => EventMode::Full,
        };

        Ok(Event {
            event_id,
            entity,
            attribute,
            value,
            timestamp,
            created_at,
            mode,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_event_pool_with_path_creation() {
        // 测试：EventPoolWithPath应该同时包含pool和db_path
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();

        let result = EventStore::create(db_path).await.unwrap();

        // 验证pool可用
        assert!(!result.pool.is_closed());

        // 验证db_path正确
        assert_eq!(result.db_path.to_str().unwrap(), db_path);

        // 验证可以从db_path推导temp_dir
        let temp_dir = result.db_path.parent().unwrap();
        assert!(temp_dir.exists());
    }

    #[tokio::test]
    async fn test_temp_dir_derivation() {
        // 测试：应该能从db_path推导回temp_dir
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("events.db");

        let result = EventStore::create(db_path.to_str().unwrap()).await.unwrap();

        // 验证可以推导回temp_dir
        let derived_temp_dir = result.db_path.parent().unwrap();
        assert_eq!(derived_temp_dir, temp_dir.path());
    }

    #[tokio::test]
    async fn test_event_pool_with_path_memory_db() {
        // 测试：内存数据库应该使用特殊路径
        let result = EventStore::create(":memory:").await.unwrap();

        assert!(!result.pool.is_closed());
        assert_eq!(result.db_path.to_str().unwrap(), ":memory:");
    }

    #[tokio::test]
    async fn test_append_and_retrieve_events() {
        let event_pool_with_path = EventStore::create(":memory:").await.unwrap();

        let mut timestamp = HashMap::new();
        timestamp.insert("editor1".to_string(), 1);

        let events = vec![
            Event::new(
                "block1".to_string(),
                "name".to_string(),
                serde_json::json!("My Block"),
                timestamp.clone(),
            ),
            Event::new(
                "block1".to_string(),
                "type".to_string(),
                serde_json::json!("document"),
                timestamp.clone(),
            ),
        ];

        EventStore::append_events(&event_pool_with_path.pool, &events)
            .await
            .unwrap();

        let retrieved = EventStore::get_all_events(&event_pool_with_path.pool)
            .await
            .unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].entity, "block1");
        assert_eq!(retrieved[0].attribute, "name");
        assert_eq!(retrieved[1].attribute, "type");
    }

    #[tokio::test]
    async fn test_get_events_by_entity() {
        let event_pool_with_path = EventStore::create(":memory:").await.unwrap();

        let mut timestamp = HashMap::new();
        timestamp.insert("editor1".to_string(), 1);

        let events = vec![
            Event::new(
                "block1".to_string(),
                "name".to_string(),
                serde_json::json!("Block 1"),
                timestamp.clone(),
            ),
            Event::new(
                "block2".to_string(),
                "name".to_string(),
                serde_json::json!("Block 2"),
                timestamp.clone(),
            ),
            Event::new(
                "block1".to_string(),
                "type".to_string(),
                serde_json::json!("document"),
                timestamp.clone(),
            ),
        ];

        EventStore::append_events(&event_pool_with_path.pool, &events)
            .await
            .unwrap();

        let block1_events = EventStore::get_events_by_entity(&event_pool_with_path.pool, "block1")
            .await
            .unwrap();
        assert_eq!(block1_events.len(), 2);
        assert_eq!(block1_events[0].attribute, "name");
        assert_eq!(block1_events[1].attribute, "type");
    }

    #[tokio::test]
    async fn test_get_events_after_event_id() {
        let pool = EventStore::create(":memory:").await.unwrap();

        let mut ts = HashMap::new();
        ts.insert("alice".to_string(), 1);

        let events = vec![
            Event::new(
                "block1".to_string(),
                "alice/core.create".to_string(),
                serde_json::json!({"name": "B1"}),
                ts.clone(),
            ),
            Event::new(
                "block2".to_string(),
                "alice/core.create".to_string(),
                serde_json::json!({"name": "B2"}),
                ts.clone(),
            ),
            Event::new(
                "block3".to_string(),
                "alice/core.create".to_string(),
                serde_json::json!({"name": "B3"}),
                ts.clone(),
            ),
        ];

        let pivot_event_id = events[0].event_id.clone();

        EventStore::append_events(&pool.pool, &events)
            .await
            .unwrap();

        // 获取第一个 event 之后的 events
        let after = EventStore::get_events_after_event_id(&pool.pool, &pivot_event_id)
            .await
            .unwrap();
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].entity, "block2");
        assert_eq!(after[1].entity, "block3");
    }

    #[tokio::test]
    async fn test_get_events_after_last_event_returns_empty() {
        let pool = EventStore::create(":memory:").await.unwrap();

        let mut ts = HashMap::new();
        ts.insert("alice".to_string(), 1);

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({}),
            ts,
        );
        let last_id = event.event_id.clone();

        EventStore::append_events(&pool.pool, &[event])
            .await
            .unwrap();

        let after = EventStore::get_events_after_event_id(&pool.pool, &last_id)
            .await
            .unwrap();
        assert!(after.is_empty());
    }

    #[tokio::test]
    async fn test_get_latest_event_id() {
        let pool = EventStore::create(":memory:").await.unwrap();

        // 空数据库返回 None
        let latest = EventStore::get_latest_event_id(&pool.pool).await.unwrap();
        assert!(latest.is_none());

        let mut ts = HashMap::new();
        ts.insert("alice".to_string(), 1);

        let events = vec![
            Event::new(
                "block1".to_string(),
                "alice/core.create".to_string(),
                serde_json::json!({}),
                ts.clone(),
            ),
            Event::new(
                "block2".to_string(),
                "alice/core.create".to_string(),
                serde_json::json!({}),
                ts.clone(),
            ),
        ];

        let expected_last_id = events[1].event_id.clone();

        EventStore::append_events(&pool.pool, &events)
            .await
            .unwrap();

        let latest = EventStore::get_latest_event_id(&pool.pool).await.unwrap();
        assert_eq!(latest, Some(expected_last_id));
    }

    #[tokio::test]
    async fn test_get_events_after_nonexistent_event_returns_empty() {
        let pool = EventStore::create(":memory:").await.unwrap();

        let mut ts = HashMap::new();
        ts.insert("alice".to_string(), 1);

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({}),
            ts,
        );
        EventStore::append_events(&pool.pool, &[event])
            .await
            .unwrap();

        // 不存在的 event_id：子查询返回 NULL，WHERE rowid > NULL 为 false
        let after = EventStore::get_events_after_event_id(&pool.pool, "nonexistent")
            .await
            .unwrap();
        assert!(after.is_empty());
    }
}

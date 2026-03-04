# Part 2: Event Structure Implementation

**Priority**: Critical | **Estimated Time**: 1 week

## Overview

Implement the SQLite event store using the EAVT schema. Keep it simple - just append events and read them back.

## Directory Structure

```
src-tauri/src/
├── engine/
│   ├── mod.rs
│   └── event_store.rs
└── lib.rs
```

## Step 1: Add SQLite Dependency

**File**: `src-tauri/Cargo.toml`

```toml
[dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
```

## Step 2: Implement Event Store

**File**: `src-tauri/src/engine/event_store.rs`

```rust
use crate::models::Event;
use rusqlite::{Connection, Result};
use std::collections::HashMap;

pub struct EventStore {
    conn: Connection,
}

impl EventStore {
    /// Create or open an event store at the given path
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Create table if not exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                event_id TEXT PRIMARY KEY,
                entity TEXT NOT NULL,
                attribute TEXT NOT NULL,
                value TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indexes for faster queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_entity ON events(entity)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_created_at ON events(created_at)",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Append events to the store
    pub fn append_events(&mut self, events: Vec<Event>) -> Result<()> {
        let tx = self.conn.transaction()?;

        for event in events {
            let timestamp_json = serde_json::to_string(&event.timestamp)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let value_json = serde_json::to_string(&event.value)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            tx.execute(
                "INSERT INTO events (event_id, entity, attribute, value, timestamp, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    event.event_id,
                    event.entity,
                    event.attribute,
                    value_json,
                    timestamp_json,
                    chrono::Utc::now().timestamp()
                ],
            )?;
        }

        tx.commit()
    }

    /// Get all events (for replaying state)
    pub fn get_all_events(&self) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT event_id, entity, attribute, value, timestamp
             FROM events
             ORDER BY created_at ASC"
        )?;

        let events = stmt.query_map([], |row| {
            let timestamp_str: String = row.get(4)?;
            let timestamp: HashMap<String, u64> = serde_json::from_str(&timestamp_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(e)
                ))?;

            let value_str: String = row.get(3)?;
            let value: serde_json::Value = serde_json::from_str(&value_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(e)
                ))?;

            Ok(Event {
                event_id: row.get(0)?,
                entity: row.get(1)?,
                attribute: row.get(2)?,
                value,
                timestamp,
            })
        })?
        .collect::<Result<Vec<Event>>>()?;

        Ok(events)
    }

    /// Get events for a specific entity
    pub fn get_events_by_entity(&self, entity: &str) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT event_id, entity, attribute, value, timestamp
             FROM events
             WHERE entity = ?1
             ORDER BY created_at ASC"
        )?;

        let events = stmt.query_map([entity], |row| {
            let timestamp_str: String = row.get(4)?;
            let timestamp: HashMap<String, u64> = serde_json::from_str(&timestamp_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(e)
                ))?;

            let value_str: String = row.get(3)?;
            let value: serde_json::Value = serde_json::from_str(&value_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(e)
                ))?;

            Ok(Event {
                event_id: row.get(0)?,
                entity: row.get(1)?,
                attribute: row.get(2)?,
                value,
                timestamp,
            })
        })?
        .collect::<Result<Vec<Event>>>()?;

        Ok(events)
    }
}
```

**That's it.** No connection pooling, no async, no caching. SQLite is fast enough for MVP.

## Step 3: Wire Up Module

**File**: `src-tauri/src/engine/mod.rs`

```rust
mod event_store;

pub use event_store::EventStore;
```

**File**: `src-tauri/src/lib.rs`

```rust
pub mod models;
pub mod engine;
```

## Step 4: Write Simple Test

**File**: `src-tauri/src/engine/event_store.rs` (add at bottom)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Event;
    use std::collections::HashMap;

    #[test]
    fn test_append_and_retrieve() {
        let mut store = EventStore::new(":memory:").unwrap();

        let mut timestamp = HashMap::new();
        timestamp.insert("editor1".to_string(), 1);

        let event = Event::new(
            "block123".to_string(),
            "editor1/markdown.write".to_string(),
            serde_json::json!({"body": "# Hello"}),
            timestamp,
        );

        store.append_events(vec![event.clone()]).unwrap();

        let events = store.get_all_events().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, "block123");
    }
}
```

## Step 5: Run Test

```bash
cd src-tauri
cargo test
```

**Expected**: Test passes.

## Done

Simple event store complete:
- ✅ SQLite with EAVT schema
- ✅ Append events atomically (transaction)
- ✅ Retrieve all events (ordered)
- ✅ Retrieve by entity
- ✅ Basic test coverage
- ❌ No connection pooling (YAGNI)
- ❌ No async (SQLite is synchronous, that's fine)
- ❌ No elaborate query API (add when needed)

**Next**: Part 3 - ELF File Format (ZIP archive handling)

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// 本机快照缓存存储。
///
/// 快照是本机派生缓存，不是事实来源。存储在 `~/.elf/cache/{project-hash}/cache.db`。
/// 快照可以安全删除——删除后重新打开项目时从 Event 重建。
///
/// 设计要点：
/// - 不存储在 `.elf/` 中（`.elf/` 是跨机同步的共识数据）
/// - 无 `created_at` 字段——时序由 `event_id` 对应的 Event 的 Vector Clock 决定
/// - 联合主键：`(block_id, event_id)`
pub struct CacheStore;

impl CacheStore {
    /// 创建或打开缓存数据库，初始化 schema。
    pub async fn create(path: &str) -> Result<SqlitePool, sqlx::Error> {
        let connection_string = if path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent).map_err(sqlx::Error::Io)?;
            }
            format!("sqlite://{}", path)
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::from_str(&connection_string)?
                    .create_if_missing(true),
            )
            .await?;

        Self::init_schema(&pool).await?;
        Ok(pool)
    }

    /// 初始化 snapshots 表。
    async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS snapshots (
                block_id TEXT NOT NULL,
                event_id TEXT NOT NULL,
                state TEXT NOT NULL,
                PRIMARY KEY (block_id, event_id)
            )",
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 保存一个 Block 的快照。
    ///
    /// 使用 INSERT OR REPLACE 实现 upsert：同一 (block_id, event_id) 只保留最新。
    pub async fn save_snapshot(
        pool: &SqlitePool,
        block_id: &str,
        event_id: &str,
        state: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        let state_json =
            serde_json::to_string(state).map_err(|e| sqlx::Error::Encode(Box::new(e)))?;

        sqlx::query(
            "INSERT OR REPLACE INTO snapshots (block_id, event_id, state)
             VALUES ($1, $2, $3)",
        )
        .bind(block_id)
        .bind(event_id)
        .bind(&state_json)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 批量保存多个 Block 的快照（同一 event_id）。
    ///
    /// 用于关闭时一次性写入所有 Block 的当前状态。
    pub async fn save_snapshots_batch(
        pool: &SqlitePool,
        event_id: &str,
        states: &HashMap<String, serde_json::Value>,
    ) -> Result<(), sqlx::Error> {
        for (block_id, state) in states {
            Self::save_snapshot(pool, block_id, event_id, state).await?;
        }
        Ok(())
    }

    /// 获取指定 Block 的最新快照。
    ///
    /// 按 rowid 降序取最新一条。返回 (event_id, state)。
    pub async fn get_latest_snapshot(
        pool: &SqlitePool,
        block_id: &str,
    ) -> Result<Option<(String, serde_json::Value)>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT event_id, state FROM snapshots
             WHERE block_id = $1
             ORDER BY rowid DESC
             LIMIT 1",
        )
        .bind(block_id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(r) => {
                let event_id: String = r.try_get(0)?;
                let state_json: String = r.try_get(1)?;
                let state: serde_json::Value = serde_json::from_str(&state_json)
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
                Ok(Some((event_id, state)))
            }
            None => Ok(None),
        }
    }

    /// 获取所有 Block 的最新快照。
    ///
    /// 返回 HashMap<block_id, (event_id, state)>。
    /// 用于启动时加载快照作为 replay 基线。
    pub async fn get_all_latest_snapshots(
        pool: &SqlitePool,
    ) -> Result<HashMap<String, (String, serde_json::Value)>, sqlx::Error> {
        // 对每个 block_id 取 rowid 最大的一条
        let rows = sqlx::query(
            "SELECT s.block_id, s.event_id, s.state
             FROM snapshots s
             INNER JOIN (
                 SELECT block_id, MAX(rowid) as max_rowid
                 FROM snapshots
                 GROUP BY block_id
             ) latest ON s.block_id = latest.block_id AND s.rowid = latest.max_rowid",
        )
        .fetch_all(pool)
        .await?;

        let mut result = HashMap::new();
        for row in rows {
            let block_id: String = row.try_get(0)?;
            let event_id: String = row.try_get(1)?;
            let state_json: String = row.try_get(2)?;
            let state: serde_json::Value =
                serde_json::from_str(&state_json).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
            result.insert(block_id, (event_id, state));
        }

        Ok(result)
    }

    /// 删除指定 Block 的所有快照。
    ///
    /// 当 Block 被删除时调用。
    pub async fn delete_snapshots_for_block(
        pool: &SqlitePool,
        block_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM snapshots WHERE block_id = $1")
            .bind(block_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 清空所有快照。
    ///
    /// 用于测试或缓存重建。
    pub async fn clear_all(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM snapshots").execute(pool).await?;
        Ok(())
    }

    /// 计算项目的缓存路径。
    ///
    /// 返回 `~/.elf/cache/{project-hash}/cache.db`。
    /// project-hash 使用项目路径的 SHA256 前 16 位。
    pub fn cache_path_for_project(project_path: &Path) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        project_path.hash(&mut hasher);
        let hash = format!("{:016x}", hasher.finish());

        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".elf").join("cache").join(&hash).join("cache.db")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_cache_store() {
        let pool = CacheStore::create(":memory:").await.unwrap();
        assert!(!pool.is_closed());
    }

    #[tokio::test]
    async fn test_save_and_get_snapshot() {
        let pool = CacheStore::create(":memory:").await.unwrap();

        let state = serde_json::json!({
            "name": "Test Block",
            "block_type": "document",
            "contents": {"content": "# Hello"}
        });

        CacheStore::save_snapshot(&pool, "block-1", "evt-1", &state)
            .await
            .unwrap();

        let result = CacheStore::get_latest_snapshot(&pool, "block-1")
            .await
            .unwrap();
        assert!(result.is_some());

        let (event_id, saved_state) = result.unwrap();
        assert_eq!(event_id, "evt-1");
        assert_eq!(saved_state["name"], "Test Block");
        assert_eq!(saved_state["contents"]["content"], "# Hello");
    }

    #[tokio::test]
    async fn test_get_latest_snapshot_returns_newest() {
        let pool = CacheStore::create(":memory:").await.unwrap();

        // 保存两个快照（不同 event_id）
        CacheStore::save_snapshot(
            &pool,
            "block-1",
            "evt-1",
            &serde_json::json!({"version": 1}),
        )
        .await
        .unwrap();

        CacheStore::save_snapshot(
            &pool,
            "block-1",
            "evt-5",
            &serde_json::json!({"version": 5}),
        )
        .await
        .unwrap();

        let (event_id, state) = CacheStore::get_latest_snapshot(&pool, "block-1")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(event_id, "evt-5");
        assert_eq!(state["version"], 5);
    }

    #[tokio::test]
    async fn test_get_latest_snapshot_empty() {
        let pool = CacheStore::create(":memory:").await.unwrap();

        let result = CacheStore::get_latest_snapshot(&pool, "nonexistent")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_all_latest_snapshots() {
        let pool = CacheStore::create(":memory:").await.unwrap();

        // block-1 有两个快照
        CacheStore::save_snapshot(&pool, "block-1", "evt-1", &serde_json::json!({"v": 1}))
            .await
            .unwrap();
        CacheStore::save_snapshot(&pool, "block-1", "evt-3", &serde_json::json!({"v": 3}))
            .await
            .unwrap();

        // block-2 有一个快照
        CacheStore::save_snapshot(&pool, "block-2", "evt-2", &serde_json::json!({"v": 2}))
            .await
            .unwrap();

        let all = CacheStore::get_all_latest_snapshots(&pool).await.unwrap();
        assert_eq!(all.len(), 2);

        let (evt1, state1) = all.get("block-1").unwrap();
        assert_eq!(evt1, "evt-3");
        assert_eq!(state1["v"], 3);

        let (evt2, state2) = all.get("block-2").unwrap();
        assert_eq!(evt2, "evt-2");
        assert_eq!(state2["v"], 2);
    }

    #[tokio::test]
    async fn test_delete_snapshots_for_block() {
        let pool = CacheStore::create(":memory:").await.unwrap();

        CacheStore::save_snapshot(&pool, "block-1", "evt-1", &serde_json::json!({}))
            .await
            .unwrap();
        CacheStore::save_snapshot(&pool, "block-2", "evt-1", &serde_json::json!({}))
            .await
            .unwrap();

        CacheStore::delete_snapshots_for_block(&pool, "block-1")
            .await
            .unwrap();

        assert!(CacheStore::get_latest_snapshot(&pool, "block-1")
            .await
            .unwrap()
            .is_none());
        assert!(CacheStore::get_latest_snapshot(&pool, "block-2")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_clear_all() {
        let pool = CacheStore::create(":memory:").await.unwrap();

        CacheStore::save_snapshot(&pool, "b1", "e1", &serde_json::json!({}))
            .await
            .unwrap();
        CacheStore::save_snapshot(&pool, "b2", "e1", &serde_json::json!({}))
            .await
            .unwrap();

        CacheStore::clear_all(&pool).await.unwrap();

        let all = CacheStore::get_all_latest_snapshots(&pool).await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_save_snapshots_batch() {
        let pool = CacheStore::create(":memory:").await.unwrap();

        let mut states = HashMap::new();
        states.insert("b1".to_string(), serde_json::json!({"name": "Block 1"}));
        states.insert("b2".to_string(), serde_json::json!({"name": "Block 2"}));

        CacheStore::save_snapshots_batch(&pool, "evt-10", &states)
            .await
            .unwrap();

        let all = CacheStore::get_all_latest_snapshots(&pool).await.unwrap();
        assert_eq!(all.len(), 2);

        let (evt, state) = all.get("b1").unwrap();
        assert_eq!(evt, "evt-10");
        assert_eq!(state["name"], "Block 1");
    }

    #[tokio::test]
    async fn test_upsert_same_key() {
        let pool = CacheStore::create(":memory:").await.unwrap();

        // 同一 (block_id, event_id) 写入两次，第二次覆盖
        CacheStore::save_snapshot(&pool, "b1", "evt-1", &serde_json::json!({"v": 1}))
            .await
            .unwrap();
        CacheStore::save_snapshot(&pool, "b1", "evt-1", &serde_json::json!({"v": 2}))
            .await
            .unwrap();

        let (_, state) = CacheStore::get_latest_snapshot(&pool, "b1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state["v"], 2);
    }

    #[test]
    fn test_cache_path_for_project() {
        let path = Path::new("/home/user/projects/my-project/.elf");
        let cache_path = CacheStore::cache_path_for_project(path);

        // 验证路径结构
        let path_str = cache_path.to_string_lossy();
        assert!(path_str.contains(".elf/cache/"));
        assert!(path_str.ends_with("cache.db"));
    }

    #[test]
    fn test_cache_path_deterministic() {
        let path = Path::new("/home/user/projects/test/.elf");
        let path1 = CacheStore::cache_path_for_project(path);
        let path2 = CacheStore::cache_path_for_project(path);
        assert_eq!(path1, path2);
    }

    #[test]
    fn test_cache_path_different_projects() {
        let p1 = CacheStore::cache_path_for_project(Path::new("/project-a/.elf"));
        let p2 = CacheStore::cache_path_for_project(Path::new("/project-b/.elf"));
        assert_ne!(p1, p2);
    }

    #[tokio::test]
    async fn test_file_based_cache_store() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("cache.db");

        let pool = CacheStore::create(db_path.to_str().unwrap()).await.unwrap();

        CacheStore::save_snapshot(&pool, "b1", "e1", &serde_json::json!({"data": "test"}))
            .await
            .unwrap();

        // 关闭连接后重新打开
        pool.close().await;

        let pool2 = CacheStore::create(db_path.to_str().unwrap()).await.unwrap();

        let result = CacheStore::get_latest_snapshot(&pool2, "b1").await.unwrap();
        assert!(result.is_some());
        let (_, state) = result.unwrap();
        assert_eq!(state["data"], "test");
    }
}

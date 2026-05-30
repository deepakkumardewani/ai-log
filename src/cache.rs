//! SQLite cache for per-session metadata.
//!
//! Stored at `<projects_dir>/cclog-cache.db`. Caches token totals, message
//! counts, timestamps, and file mtimes so subsequent runs can skip re-parsing
//! unchanged JSONL files.
//!
//! Schema versioning: bumping [`SCHEMA_VERSION`] triggers automatic rebuild.

use std::path::Path;

use rusqlite::{params, Connection};

/// Current cache schema version. Bump to force a full rebuild.
const SCHEMA_VERSION: u32 = 2;

/// Cached metadata for a single session.
#[derive(Debug, Clone)]
pub struct CachedSessionMeta {
    pub session_id: String,
    pub project_name: String,
    pub title: Option<String>,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
    pub message_count: u32,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub first_user_prompt: Option<String>,
}

/// Per-project aggregate cached data.
#[derive(Debug, Clone, Default)]
pub struct CachedProjectMeta {
    pub session_count: u32,
    pub message_count: u32,
    pub total_tokens: u64,
    pub earliest: Option<String>,
    pub latest: Option<String>,
}

/// SQLite cache handle.
pub struct Cache {
    db: Connection,
}

impl Cache {
    /// Open (or create) the cache database at `path`.
    ///
    /// If the schema version doesn't match [`SCHEMA_VERSION`], the existing
    /// tables are dropped and recreated.
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        let db = Connection::open(path)?;

        // Enable WAL for better concurrent read behaviour.
        db.execute_batch("PRAGMA journal_mode=WAL;")?;

        let current_version: u32 = db
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version'",
                [],
                |_| Ok(()),
            )
            .map(|_| {
                db.query_row("SELECT version FROM schema_version", [], |row| row.get(0))
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        if current_version != SCHEMA_VERSION {
            db.execute_batch(
                "DROP TABLE IF EXISTS sessions;
                 DROP TABLE IF EXISTS schema_version;",
            )?;
        }

        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS sessions (
                 id                TEXT PRIMARY KEY,
                 project_name      TEXT NOT NULL,
                 title             TEXT,
                 first_timestamp   TEXT,
                 last_timestamp    TEXT,
                 message_count     INTEGER NOT NULL DEFAULT 0,
                 input_tokens      INTEGER NOT NULL DEFAULT 0,
                 output_tokens     INTEGER NOT NULL DEFAULT 0,
                 cache_create      INTEGER NOT NULL DEFAULT 0,
                 cache_read        INTEGER NOT NULL DEFAULT 0,
                 file_mtime        INTEGER NOT NULL DEFAULT 0,
                 file_size         INTEGER NOT NULL DEFAULT 0,
                 first_user_prompt TEXT
             );",
        )?;

        // Upsert schema version.
        db.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![SCHEMA_VERSION],
        )?;

        Ok(Self { db })
    }

    /// Look up cached metadata for a session file.
    ///
    /// Returns `None` on cache miss or if the file's `mtime` has changed.
    pub fn get(&self, session_id: &str, file_mtime: u64) -> Option<CachedSessionMeta> {
        self.db
            .query_row(
                "SELECT id, project_name, title, first_timestamp, last_timestamp,
                        message_count, input_tokens, output_tokens,
                        cache_create, cache_read, file_mtime, first_user_prompt
                 FROM sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    let cached_mtime: u64 = row.get(10)?;
                    if cached_mtime != file_mtime {
                        return Err(rusqlite::Error::QueryReturnedNoRows);
                    }
                    Ok(CachedSessionMeta {
                        session_id: row.get(0)?,
                        project_name: row.get(1)?,
                        title: row.get(2)?,
                        first_timestamp: row.get(3)?,
                        last_timestamp: row.get(4)?,
                        message_count: row.get(5)?,
                        total_input_tokens: row.get(6)?,
                        total_output_tokens: row.get(7)?,
                        total_cache_creation_tokens: row.get(8)?,
                        total_cache_read_tokens: row.get(9)?,
                        first_user_prompt: row.get(11)?,
                    })
                },
            )
            .ok()
    }

    /// Store session metadata in the cache.
    pub fn put(&self, meta: &CachedSessionMeta, file_mtime: u64, file_size: u64) {
        let _ = self.db.execute(
            "INSERT OR REPLACE INTO sessions
             (id, project_name, title, first_timestamp, last_timestamp,
              message_count, input_tokens, output_tokens,
              cache_create, cache_read, file_mtime, file_size,
              first_user_prompt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                meta.session_id,
                meta.project_name,
                meta.title,
                meta.first_timestamp,
                meta.last_timestamp,
                meta.message_count,
                meta.total_input_tokens,
                meta.total_output_tokens,
                meta.total_cache_creation_tokens,
                meta.total_cache_read_tokens,
                file_mtime,
                file_size,
                meta.first_user_prompt,
            ],
        );
    }

    /// Aggregate cached data across all sessions in a project.
    pub fn project_aggregate(&self, project_name: &str) -> CachedProjectMeta {
        let mut meta = CachedProjectMeta::default();
        let mut stmt = self
            .db
            .prepare(
                "SELECT COUNT(*), COALESCE(SUM(message_count),0),
                        COALESCE(SUM(input_tokens+output_tokens+cache_create+cache_read),0),
                        MIN(first_timestamp), MAX(last_timestamp)
                 FROM sessions WHERE project_name = ?1",
            )
            .unwrap();

        let _ = stmt.query_row(params![project_name], |row| {
            meta.session_count = row.get::<_, i64>(0).unwrap_or(0) as u32;
            meta.message_count = row.get::<_, i64>(1).unwrap_or(0) as u32;
            meta.total_tokens = row.get::<_, i64>(2).unwrap_or(0) as u64;
            meta.earliest = row.get(3).ok();
            meta.latest = row.get(4).ok();
            Ok(())
        });

        meta
    }

    /// Aggregate across all cached sessions (for the master index).
    pub fn global_aggregate(&self) -> CachedProjectMeta {
        let mut meta = CachedProjectMeta::default();
        let mut stmt = self
            .db
            .prepare(
                "SELECT COUNT(*), COALESCE(SUM(message_count),0),
                        COALESCE(SUM(input_tokens+output_tokens+cache_create+cache_read),0),
                        MIN(first_timestamp), MAX(last_timestamp)
                 FROM sessions",
            )
            .unwrap();

        let _ = stmt.query_row([], |row| {
            meta.session_count = row.get::<_, i64>(0).unwrap_or(0) as u32;
            meta.message_count = row.get::<_, i64>(1).unwrap_or(0) as u32;
            meta.total_tokens = row.get::<_, i64>(2).unwrap_or(0) as u64;
            meta.earliest = row.get(3).ok();
            meta.latest = row.get(4).ok();
            Ok(())
        });

        meta
    }

    /// Return a list of distinct project names in the cache.
    pub fn project_names(&self) -> Vec<String> {
        let mut stmt = self
            .db
            .prepare("SELECT DISTINCT project_name FROM sessions ORDER BY project_name")
            .unwrap();
        let rows = stmt.query_map([], |row| row.get(0)).unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    /// Delete all rows and re-create the schema.
    pub fn clear(&self) -> Result<(), rusqlite::Error> {
        self.db.execute_batch("DELETE FROM sessions;")?;
        // Re-insert schema version marker.
        self.db.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![SCHEMA_VERSION],
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    static TEST_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    fn temp_cache() -> (Cache, std::path::PathBuf) {
        let n = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("cclog-cache-test-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let cache = Cache::open(&path).unwrap();
        (cache, dir)
    }

    #[test]
    fn cache_miss_on_empty_db() {
        let (cache, dir) = temp_cache();
        let meta = cache.get("session-1", 100);
        assert!(meta.is_none());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cache_hit_after_put() {
        let (cache, dir) = temp_cache();
        let meta = CachedSessionMeta {
            session_id: "session-1".into(),
            project_name: "my-project".into(),
            title: Some("Test Session".into()),
            first_timestamp: Some("2025-06-15T10:00:00Z".into()),
            last_timestamp: Some("2025-06-15T11:00:00Z".into()),
            message_count: 10,
            total_input_tokens: 1000,
            total_output_tokens: 500,
            total_cache_creation_tokens: 200,
            total_cache_read_tokens: 100,
            first_user_prompt: Some("Hello, Claude!".into()),
        };
        cache.put(&meta, 100, 5000);

        let hit = cache.get("session-1", 100);
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert_eq!(hit.session_id, "session-1");
        assert_eq!(hit.title.unwrap(), "Test Session");
        assert_eq!(hit.message_count, 10);
        assert_eq!(hit.total_input_tokens, 1000);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cache_miss_on_mtime_change() {
        let (cache, dir) = temp_cache();
        let meta = CachedSessionMeta {
            session_id: "s1".into(),
            project_name: "p1".into(),
            title: None,
            first_timestamp: None,
            last_timestamp: None,
            message_count: 5,
            total_input_tokens: 100,
            total_output_tokens: 50,
            total_cache_creation_tokens: 0,
            total_cache_read_tokens: 0,
            first_user_prompt: None,
        };
        cache.put(&meta, 100, 1000);

        // Different mtime → cache miss.
        let hit = cache.get("s1", 200);
        assert!(hit.is_none());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn project_aggregate_sums_correctly() {
        let (cache, dir) = temp_cache();
        for i in 1..=3 {
            let meta = CachedSessionMeta {
                session_id: format!("s{}", i),
                project_name: "test-proj".into(),
                title: None,
                first_timestamp: None,
                last_timestamp: None,
                message_count: 10,
                total_input_tokens: 100,
                total_output_tokens: 50,
                total_cache_creation_tokens: 0,
                total_cache_read_tokens: 0,
                first_user_prompt: None,
            };
            cache.put(&meta, 100 + i, 1000);
        }

        let agg = cache.project_aggregate("test-proj");
        assert_eq!(agg.session_count, 3);
        assert_eq!(agg.message_count, 30);
        assert_eq!(agg.total_tokens, 450); // 3 × (100+50)

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn clear_removes_all_entries() {
        let (cache, dir) = temp_cache();
        let meta = CachedSessionMeta {
            session_id: "s1".into(),
            project_name: "p1".into(),
            title: None,
            first_timestamp: None,
            last_timestamp: None,
            message_count: 1,
            total_input_tokens: 1,
            total_output_tokens: 1,
            total_cache_creation_tokens: 0,
            total_cache_read_tokens: 0,
            first_user_prompt: None,
        };
        cache.put(&meta, 100, 100);
        cache.clear().unwrap();

        let hit = cache.get("s1", 100);
        assert!(hit.is_none());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn schema_version_bump_rebuilds() {
        let n = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("cclog-cache-schema-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");

        // Open and populate.
        let cache = Cache::open(&path).unwrap();
        let meta = CachedSessionMeta {
            session_id: "s1".into(),
            project_name: "p1".into(),
            title: None,
            first_timestamp: None,
            last_timestamp: None,
            message_count: 1,
            total_input_tokens: 1,
            total_output_tokens: 1,
            total_cache_creation_tokens: 0,
            total_cache_read_tokens: 0,
            first_user_prompt: None,
        };
        cache.put(&meta, 100, 100);
        drop(cache);

        // Re-open — data should still be there.
        let cache2 = Cache::open(&path).unwrap();
        let hit = cache2.get("s1", 100);
        assert!(hit.is_some());
        drop(cache2);

        fs::remove_dir_all(&dir).ok();
    }
}

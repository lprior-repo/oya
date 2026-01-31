//! Database operations for session persistence
//!
//! This module provides SQLite-based persistence for ZJJ sessions with:
//! - Enhanced schema with status, metadata, and timestamps
//! - Thread-safe connection management
//! - Status lifecycle management
//! - Recovery from corruption
//!
//! All operations follow the zero-unwrap rule and return `zjj_core::Result`.

use std::{
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use rusqlite::Connection;
use zjj_core::{Error, Result};

use crate::session::{Session, SessionStatus, SessionUpdate};

/// Database wrapper for session storage with thread-safe connection management
pub struct SessionDb {
    conn: Arc<Mutex<Connection>>,
}

impl SessionDb {
    /// Open or create a session database at the given path
    ///
    /// Creates the database file if it doesn't exist and initializes the schema.
    /// Returns an error if the database is corrupted or cannot be accessed.
    ///
    /// # Errors
    ///
    /// Returns `Error::DatabaseError` if:
    /// - The database file cannot be opened
    /// - Schema creation fails
    /// - The database is corrupted
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| Error::DatabaseError(format!("Failed to open database: {e}")))?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.init_schema()?;
        Ok(db)
    }

    /// Initialize the database schema with tables, indexes, and triggers
    fn init_schema(&self) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::DatabaseError(format!("Lock error: {e}")))?;

        // Create sessions table with all required fields
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('creating', 'active', 'paused', 'completed', 'failed')),
                workspace_path TEXT NOT NULL,
                branch TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                last_synced INTEGER,
                metadata TEXT
            )",
            [],
        )
        .map_err(|e| Error::DatabaseError(format!("Failed to create sessions table: {e}")))?;

        // Create index on status for filtering
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_status ON sessions(status)",
            [],
        )
        .map_err(|e| Error::DatabaseError(format!("Failed to create status index: {e}")))?;

        // Create index on name for lookups
        conn.execute("CREATE INDEX IF NOT EXISTS idx_name ON sessions(name)", [])
            .map_err(|e| Error::DatabaseError(format!("Failed to create name index: {e}")))?;

        // Create trigger to auto-update updated_at timestamp
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS update_timestamp
             AFTER UPDATE ON sessions
             FOR EACH ROW
             BEGIN
                 UPDATE sessions SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
             END",
            [],
        )
        .map_err(|e| Error::DatabaseError(format!("Failed to create update trigger: {e}")))?;

        drop(conn);
        Ok(())
    }

    /// Create a new session
    ///
    /// Creates a session with status set to `Creating` and auto-generated timestamps.
    ///
    /// # Errors
    ///
    /// Returns `Error::DatabaseError` if:
    /// - A session with the same name already exists (UNIQUE constraint)
    /// - The database connection fails
    pub fn create(&self, name: &str, workspace_path: &str) -> Result<Session> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::DatabaseError(format!("Lock error: {e}")))?;

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| Error::Unknown(format!("System time error: {e}")))?
            .as_secs();

        let status = SessionStatus::Creating;

        conn.execute(
            "INSERT INTO sessions (name, status, workspace_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (name, status.to_string(), workspace_path, now, now),
        )
        .map_err(|e| {
            if e.to_string().to_lowercase().contains("unique") {
                Error::DatabaseError(format!("Session '{name}' already exists"))
            } else {
                Error::DatabaseError(format!("Failed to create session: {e}"))
            }
        })?;

        let id = conn.last_insert_rowid();
        drop(conn);

        Ok(Session {
            id: Some(id),
            name: name.to_string(),
            status,
            workspace_path: workspace_path.to_string(),
            zellij_tab: format!("jjz:{name}"),
            branch: None,
            created_at: now,
            updated_at: now,
            last_synced: None,
            metadata: None,
        })
    }

    /// Get a session by name
    ///
    /// # Errors
    ///
    /// Returns `Error::DatabaseError` if the database query fails.
    pub fn get(&self, name: &str) -> Result<Option<Session>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::DatabaseError(format!("Lock error: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata
                 FROM sessions WHERE name = ?1",
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to prepare query: {e}")))?;

        let mut rows = stmt
            .query([name])
            .map_err(|e| Error::DatabaseError(format!("Failed to execute query: {e}")))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| Error::DatabaseError(format!("Failed to read row: {e}")))?
        {
            let id: i64 = row
                .get(0)
                .map_err(|e| Error::DatabaseError(format!("Failed to read id: {e}")))?;
            let name: String = row
                .get(1)
                .map_err(|e| Error::DatabaseError(format!("Failed to read name: {e}")))?;
            let status_str: String = row
                .get(2)
                .map_err(|e| Error::DatabaseError(format!("Failed to read status: {e}")))?;
            let status = SessionStatus::from_str(&status_str)?;
            let workspace_path: String = row
                .get(3)
                .map_err(|e| Error::DatabaseError(format!("Failed to read workspace_path: {e}")))?;
            let branch: Option<String> = row
                .get(4)
                .map_err(|e| Error::DatabaseError(format!("Failed to read branch: {e}")))?;
            let created_at: u64 = row
                .get(5)
                .map_err(|e| Error::DatabaseError(format!("Failed to read created_at: {e}")))?;
            let updated_at: u64 = row
                .get(6)
                .map_err(|e| Error::DatabaseError(format!("Failed to read updated_at: {e}")))?;
            let last_synced: Option<u64> = row
                .get(7)
                .map_err(|e| Error::DatabaseError(format!("Failed to read last_synced: {e}")))?;
            let metadata_str: Option<String> = row
                .get(8)
                .map_err(|e| Error::DatabaseError(format!("Failed to read metadata: {e}")))?;

            let metadata = match metadata_str {
                Some(s) => Some(
                    serde_json::from_str(&s)
                        .map_err(|e| Error::ParseError(format!("Invalid metadata JSON: {e}")))?,
                ),
                None => None,
            };

            let session = Session {
                id: Some(id),
                name: name.clone(),
                status,
                workspace_path,
                zellij_tab: format!("jjz:{name}"),
                branch,
                created_at,
                updated_at,
                last_synced,
                metadata,
            };
            drop(rows);
            drop(stmt);
            drop(conn);
            Ok(Some(session))
        } else {
            drop(rows);
            drop(stmt);
            drop(conn);
            Ok(None)
        }
    }

    /// Update an existing session
    ///
    /// Updates the specified fields and automatically sets `updated_at` timestamp.
    ///
    /// # Errors
    ///
    /// Returns `Error::DatabaseError` if the database update fails.
    pub fn update(&self, name: &str, update: SessionUpdate) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::DatabaseError(format!("Lock error: {e}")))?;

        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(status) = update.status {
            updates.push("status = ?");
            params.push(Box::new(status.to_string()));
        }

        if let Some(branch) = update.branch {
            updates.push("branch = ?");
            params.push(Box::new(branch));
        }

        if let Some(last_synced) = update.last_synced {
            updates.push("last_synced = ?");
            params.push(Box::new(last_synced));
        }

        if let Some(metadata) = update.metadata {
            updates.push("metadata = ?");
            let json_str = serde_json::to_string(&metadata)
                .map_err(|e| Error::ParseError(format!("Failed to serialize metadata: {e}")))?;
            params.push(Box::new(json_str));
        }

        if updates.is_empty() {
            return Ok(());
        }

        let sql = format!("UPDATE sessions SET {} WHERE name = ?", updates.join(", "));
        params.push(Box::new(name.to_string()));

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(std::convert::AsRef::as_ref).collect();

        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| Error::DatabaseError(format!("Failed to update session: {e}")))?;

        drop(conn);
        Ok(())
    }

    /// Delete a session by name
    ///
    /// Returns `true` if the session was deleted, `false` if it didn't exist.
    ///
    /// # Errors
    ///
    /// Returns `Error::DatabaseError` if the database delete fails.
    pub fn delete(&self, name: &str) -> Result<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::DatabaseError(format!("Lock error: {e}")))?;

        let changes = conn
            .execute("DELETE FROM sessions WHERE name = ?1", [name])
            .map_err(|e| Error::DatabaseError(format!("Failed to delete session: {e}")))?;

        drop(conn);
        Ok(changes > 0)
    }

    /// List all sessions, optionally filtered by status
    ///
    /// Sessions are ordered by `created_at` timestamp.
    ///
    /// # Errors
    ///
    /// Returns `Error::DatabaseError` if the database query fails.
    pub fn list(&self, status_filter: Option<SessionStatus>) -> Result<Vec<Session>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::DatabaseError(format!("Lock error: {e}")))?;

        let (sql, params): (String, Vec<String>) = status_filter.map_or_else(
            || {
                (
                    "SELECT id, name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata
                 FROM sessions ORDER BY created_at"
                        .to_string(),
                    vec![],
                )
            },
            |status| {
                (
                    "SELECT id, name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata
                 FROM sessions WHERE status = ?1 ORDER BY created_at"
                        .to_string(),
                    vec![status.to_string()],
                )
            },
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| Error::DatabaseError(format!("Failed to prepare query: {e}")))?;

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                let id: i64 = row.get(0)?;
                let name: String = row.get(1)?;
                let status_str: String = row.get(2)?;
                let workspace_path: String = row.get(3)?;
                let branch: Option<String> = row.get(4)?;
                let created_at: u64 = row.get(5)?;
                let updated_at: u64 = row.get(6)?;
                let last_synced: Option<u64> = row.get(7)?;
                let metadata_str: Option<String> = row.get(8)?;

                Ok((
                    id,
                    name,
                    status_str,
                    workspace_path,
                    branch,
                    created_at,
                    updated_at,
                    last_synced,
                    metadata_str,
                ))
            })
            .map_err(|e| Error::DatabaseError(format!("Failed to execute query: {e}")))?;

        let mut sessions = Vec::new();
        for row_result in rows {
            let (
                id,
                name,
                status_str,
                workspace_path,
                branch,
                created_at,
                updated_at,
                last_synced,
                metadata_str,
            ) = row_result.map_err(|e| Error::DatabaseError(format!("Failed to read row: {e}")))?;

            let status = SessionStatus::from_str(&status_str)?;

            let metadata = match metadata_str {
                Some(s) => Some(
                    serde_json::from_str(&s)
                        .map_err(|e| Error::ParseError(format!("Invalid metadata JSON: {e}")))?,
                ),
                None => None,
            };

            sessions.push(Session {
                id: Some(id),
                name: name.clone(),
                status,
                workspace_path,
                zellij_tab: format!("jjz:{name}"),
                branch,
                created_at,
                updated_at,
                last_synced,
                metadata,
            });
        }

        drop(stmt);
        drop(conn);
        Ok(sessions)
    }

    /// Rebuild database from a list of discovered sessions
    ///
    /// Drops existing data and recreates the schema, then inserts all provided sessions.
    /// Used for recovery from database corruption.
    ///
    /// NOTE: Currently only used in tests. When implementing corruption recovery
    /// features, this will be promoted to production.
    ///
    /// # Errors
    ///
    /// Returns `Error::DatabaseError` if the rebuild process fails.
    #[cfg(test)]
    pub fn rebuild_from_sessions(&self, sessions: Vec<Session>) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::DatabaseError(format!("Lock error: {e}")))?;

        // Drop existing table and trigger
        conn.execute("DROP TABLE IF EXISTS sessions", [])
            .map_err(|e| Error::DatabaseError(format!("Failed to drop sessions table: {e}")))?;

        conn.execute("DROP TRIGGER IF EXISTS update_timestamp", [])
            .map_err(|e| Error::DatabaseError(format!("Failed to drop update trigger: {e}")))?;

        // Release lock before calling init_schema which needs the lock
        drop(conn);

        // Recreate schema
        self.init_schema()?;

        // Insert all sessions
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::DatabaseError(format!("Lock error: {e}")))?;

        for session in sessions {
            let metadata_json = match session.metadata {
                Some(ref m) => Some(serde_json::to_string(m).map_err(|e| {
                    Error::ParseError(format!("Failed to serialize metadata: {e}"))
                })?),
                None => None,
            };

            conn.execute(
                "INSERT INTO sessions (name, status, workspace_path, branch, created_at, updated_at, last_synced, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    &session.name,
                    session.status.to_string(),
                    &session.workspace_path,
                    &session.branch,
                    session.created_at,
                    session.updated_at,
                    session.last_synced,
                    metadata_json,
                ),
            )
            .map_err(|e| Error::DatabaseError(format!("Failed to insert session during rebuild: {e}")))?;
        }

        drop(conn);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    // Helper to create a temporary database for testing
    fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new().map_err(|e| Error::IoError(e.to_string()))?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::open(&db_path)?;
        Ok((db, dir))
    }

    // ===== Schema Tests =====

    #[test]
    fn test_schema_has_all_columns() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let conn = db
            .conn
            .lock()
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        let mut stmt = conn
            .prepare("PRAGMA table_info(sessions)")
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let columns: std::result::Result<Vec<String>, _> = stmt
            .query_map([], |row| row.get(1))
            .map_err(|e| Error::DatabaseError(e.to_string()))?
            .collect();

        let columns = columns.map_err(|e| Error::DatabaseError(e.to_string()))?;
        drop(stmt);
        drop(conn);

        assert!(columns.contains(&"id".to_string()));
        assert!(columns.contains(&"name".to_string()));
        assert!(columns.contains(&"status".to_string()));
        assert!(columns.contains(&"workspace_path".to_string()));
        assert!(columns.contains(&"branch".to_string()));
        assert!(columns.contains(&"created_at".to_string()));
        assert!(columns.contains(&"updated_at".to_string()));
        assert!(columns.contains(&"last_synced".to_string()));
        assert!(columns.contains(&"metadata".to_string()));
        Ok(())
    }

    #[test]
    fn test_schema_has_indexes() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let conn = db
            .conn
            .lock()
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='sessions'")
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let indexes: std::result::Result<Vec<String>, _> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| Error::DatabaseError(e.to_string()))?
            .collect();

        let indexes = indexes.map_err(|e| Error::DatabaseError(e.to_string()))?;
        drop(stmt);
        drop(conn);

        // Should have idx_status, idx_name, and the auto-generated UNIQUE index
        assert!(indexes.iter().any(|name| name.contains("idx_status")));
        assert!(indexes.iter().any(|name| name.contains("idx_name")));
        Ok(())
    }

    #[test]
    fn test_schema_has_unique_name_constraint() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create first session
        let _session1 = db.create("test", "/path1")?;

        // Try to create duplicate
        let result = db.create("test", "/path2");
        assert!(result.is_err());

        // Verify it's a database error about unique constraint
        if let Err(Error::DatabaseError(msg)) = result {
            assert!(msg.to_lowercase().contains("unique") || msg.contains("already exists"));
        } else {
            return Err(Error::Unknown(
                "Expected DatabaseError with UNIQUE constraint violation".to_string(),
            ));
        }
        Ok(())
    }

    // ===== CRUD Operation Tests =====

    #[test]
    fn test_create_session_sets_status_creating() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let session = db.create("test-session", "/workspace")?;

        assert_eq!(session.name, "test-session");
        assert_eq!(session.status, SessionStatus::Creating);
        assert_eq!(session.workspace_path, "/workspace");
        Ok(())
    }

    #[test]
    fn test_create_session_generates_id() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let session = db.create("test", "/path")?;

        assert!(session.id.is_some());
        assert!(session.id.is_some_and(|id| id > 0));
        Ok(())
    }

    #[test]
    fn test_create_session_sets_timestamps() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let session = db.create("test", "/path")?;

        assert!(session.created_at > 0);
        assert!(session.updated_at > 0);
        assert_eq!(session.created_at, session.updated_at);
        Ok(())
    }

    #[test]
    fn test_get_session_by_name_exists() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let created = db.create("test", "/path")?;

        let retrieved = db.get("test")?;
        assert!(retrieved.is_some());

        let session = retrieved.ok_or_else(|| Error::NotFound("session".into()))?;
        assert_eq!(session.name, created.name);
        assert_eq!(session.status, created.status);
        Ok(())
    }

    #[test]
    fn test_get_session_by_name_missing() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let result = db.get("nonexistent")?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn test_update_session_status() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let session = db.create("test", "/path")?;

        let update = SessionUpdate {
            status: Some(SessionStatus::Active),
            ..Default::default()
        };
        db.update("test", update)?;

        let updated = db
            .get("test")?
            .ok_or_else(|| Error::NotFound("session".into()))?;
        assert_eq!(updated.status, SessionStatus::Active);
        assert!(updated.updated_at >= session.updated_at);
        Ok(())
    }

    #[test]
    fn test_update_session_branch() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        db.create("test", "/path")?;

        let update = SessionUpdate {
            branch: Some("feature-branch".to_string()),
            ..Default::default()
        };
        db.update("test", update)?;

        let updated = db
            .get("test")?
            .ok_or_else(|| Error::NotFound("session".into()))?;
        assert_eq!(updated.branch, Some("feature-branch".to_string()));
        Ok(())
    }

    #[test]
    fn test_update_session_metadata() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        db.create("test", "/path")?;

        let metadata = serde_json::json!({"key": "value"});
        let update = SessionUpdate {
            metadata: Some(metadata.clone()),
            ..Default::default()
        };
        db.update("test", update)?;

        let updated = db
            .get("test")?
            .ok_or_else(|| Error::NotFound("session".into()))?;
        assert_eq!(updated.metadata, Some(metadata));
        Ok(())
    }

    #[test]
    fn test_delete_session_exists() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        db.create("test", "/path")?;

        let deleted = db.delete("test")?;
        assert!(deleted);

        let result = db.get("test")?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn test_delete_session_missing() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let deleted = db.delete("nonexistent")?;
        assert!(!deleted);
        Ok(())
    }

    // ===== List Operation Tests =====

    #[test]
    fn test_list_all_sessions() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        db.create("session1", "/path1")?;
        db.create("session2", "/path2")?;
        db.create("session3", "/path3")?;

        let sessions = db.list(None)?;
        assert_eq!(sessions.len(), 3);
        Ok(())
    }

    #[test]
    fn test_list_sessions_by_status_active() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        db.create("session1", "/path1")?;
        db.create("session2", "/path2")?;

        // Update one to active
        let update = SessionUpdate {
            status: Some(SessionStatus::Active),
            ..Default::default()
        };
        db.update("session1", update)?;

        let sessions = db.list(Some(SessionStatus::Active))?;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "session1");
        Ok(())
    }

    #[test]
    fn test_list_sessions_by_status_empty() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        db.create("session1", "/path1")?;

        let sessions = db.list(Some(SessionStatus::Completed))?;
        assert_eq!(sessions.len(), 0);
        Ok(())
    }

    #[test]
    fn test_list_sessions_ordered_by_created() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create sessions with small delay to ensure different timestamps
        let s1 = db.create("session1", "/path1")?;
        std::thread::sleep(std::time::Duration::from_millis(10));
        let s2 = db.create("session2", "/path2")?;

        let sessions = db.list(None)?;
        assert_eq!(sessions.len(), 2);
        assert!(sessions[0].created_at <= sessions[1].created_at);
        assert_eq!(sessions[0].name, s1.name);
        assert_eq!(sessions[1].name, s2.name);
        Ok(())
    }

    // ===== Timestamp Tests =====

    #[test]
    fn test_updated_at_changes_on_update() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let session = db.create("test", "/path")?;
        let original_updated = session.updated_at;

        // SQLite strftime('%s', 'now') has second-level precision
        // Sleep for 1 full second to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_secs(1));

        let update = SessionUpdate {
            status: Some(SessionStatus::Active),
            ..Default::default()
        };
        db.update("test", update)?;

        let updated = db
            .get("test")?
            .ok_or_else(|| Error::NotFound("session".into()))?;
        assert!(updated.updated_at > original_updated);
        Ok(())
    }

    #[test]
    fn test_created_at_immutable() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let session = db.create("test", "/path")?;
        let original_created = session.created_at;

        let update = SessionUpdate {
            status: Some(SessionStatus::Active),
            ..Default::default()
        };
        db.update("test", update)?;

        let updated = db
            .get("test")?
            .ok_or_else(|| Error::NotFound("session".into()))?;
        assert_eq!(updated.created_at, original_created);
        Ok(())
    }

    #[test]
    fn test_timestamps_are_unix_epoch() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let session = db.create("test", "/path")?;

        // Unix epoch timestamps should be reasonable (after 2020, before 2100)
        let year_2020 = 1_577_836_800u64; // 2020-01-01
        let year_2100 = 4_102_444_800u64; // 2100-01-01

        assert!(session.created_at > year_2020);
        assert!(session.created_at < year_2100);
        assert!(session.updated_at > year_2020);
        assert!(session.updated_at < year_2100);
        Ok(())
    }

    // ===== Recovery Tests =====

    #[test]
    fn test_rebuild_from_sessions_drops_old_data() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        db.create("old-session", "/old")?;

        let new_sessions = vec![Session::new("new-session", "/new")?];
        db.rebuild_from_sessions(new_sessions)?;

        let all = db.list(None)?;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "new-session");

        // Old session should be gone
        let old = db.get("old-session")?;
        assert!(old.is_none());
        Ok(())
    }

    #[test]
    fn test_rebuild_from_sessions_inserts_new() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        let sessions = vec![
            Session::new("session1", "/path1")?,
            Session::new("session2", "/path2")?,
        ];
        db.rebuild_from_sessions(sessions)?;

        let all = db.list(None)?;
        assert_eq!(all.len(), 2);
        Ok(())
    }

    // ===== Concurrency Tests =====

    #[test]
    fn test_concurrent_creates() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let db = Arc::new(db);

        let mut handles = vec![];
        for i in 0..5 {
            let db_clone = Arc::clone(&db);
            let handle = std::thread::spawn(move || {
                let name = format!("session{i}");
                db_clone.create(&name, "/path")
            });
            handles.push(handle);
        }

        let mut created = 0;
        for handle in handles {
            if handle.join().is_ok_and(|r| r.is_ok()) {
                created += 1;
            }
        }

        assert_eq!(created, 5);
        let all = db.list(None)?;
        assert_eq!(all.len(), 5);
        Ok(())
    }

    #[test]
    fn test_concurrent_reads() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        db.create("test", "/path")?;
        let db = Arc::new(db);

        let mut handles = vec![];
        for _ in 0..10 {
            let db_clone = Arc::clone(&db);
            let handle = std::thread::spawn(move || db_clone.get("test"));
            handles.push(handle);
        }

        let mut successful_reads = 0;
        for handle in handles {
            if handle.join().is_ok_and(|r| r.is_ok_and(|s| s.is_some())) {
                successful_reads += 1;
            }
        }

        assert_eq!(successful_reads, 10);
        Ok(())
    }

    #[test]
    fn test_concurrent_create_same_name() -> Result<()> {
        let (db, _dir) = setup_test_db()?;
        let db = Arc::new(db);

        let db1 = Arc::clone(&db);
        let db2 = Arc::clone(&db);

        let h1 = std::thread::spawn(move || db1.create("duplicate", "/path1"));
        let h2 = std::thread::spawn(move || db2.create("duplicate", "/path2"));

        let r1 = h1.join();
        let r2 = h2.join();

        // One should succeed, one should fail with UNIQUE constraint
        let success_count = [r1, r2]
            .iter()
            .filter(|r| r.as_ref().is_ok_and(std::result::Result::is_ok))
            .count();

        assert_eq!(success_count, 1);
        Ok(())
    }
}

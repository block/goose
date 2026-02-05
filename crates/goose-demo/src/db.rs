//! SQLite database for session persistence
//!
//! Design principles:
//! - Single app.db file for all data
//! - Minimal lock holding - acquire, read/write, release immediately
//! - Sessions run fully in parallel with no contention
//! - Message history stored as JSON blobs for simplicity

use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{Error, Result};

/// Database handle with internal synchronization
///
/// SQLite in WAL mode allows concurrent reads, but writes are serialized.
/// We use a Mutex (not async) because SQLite operations are fast and
/// we want to minimize lock scope.
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open or create the database at the given path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| Error::Database(format!("Failed to open database: {}", e)))?;

        // Enable WAL mode for better concurrency
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| Error::Database(format!("Failed to set WAL mode: {}", e)))?;

        // Reasonable busy timeout for concurrent access
        conn.pragma_update(None, "busy_timeout", 5000)
            .map_err(|e| Error::Database(format!("Failed to set busy timeout: {}", e)))?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.migrate()?;
        Ok(db)
    }

    /// Run database migrations
    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();

        // Initial schema
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                preamble TEXT,
                -- Messages stored as JSON array for simplicity
                -- Each message is a rig::message::Message serialized
                messages TEXT NOT NULL DEFAULT '[]',
                -- Enabled extensions as JSON array of names
                enabled_extensions TEXT NOT NULL DEFAULT '[]'
            );
            
            CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);
            "#,
        )
        .map_err(|e| Error::Database(format!("Migration failed: {}", e)))?;

        // Migration: rename mcp_servers to enabled_extensions if needed
        // Check if old column exists
        let has_mcp_servers: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'mcp_servers'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0)
            > 0;

        if has_mcp_servers {
            // Migrate old mcp_servers column to enabled_extensions
            conn.execute_batch(
                r#"
                ALTER TABLE sessions ADD COLUMN enabled_extensions TEXT NOT NULL DEFAULT '[]';
                -- Note: We can't easily migrate mcp_servers data to extension names,
                -- so we just start fresh with empty enabled_extensions
                "#,
            )
            .ok(); // Ignore error if column already exists
        }

        Ok(())
    }

    /// Create a new session
    pub fn create_session(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock();

        conn.execute(
            "INSERT INTO sessions (id, created_at, updated_at, messages, enabled_extensions) VALUES (?1, ?2, ?2, '[]', '[]')",
            params![id, now],
        )
        .map_err(|e| Error::Database(format!("Failed to create session: {}", e)))?;

        Ok(())
    }

    /// Load a session's data
    pub fn load_session(&self, id: &str) -> Result<Option<SessionData>> {
        let conn = self.conn.lock();

        conn.query_row(
            "SELECT id, created_at, updated_at, preamble, messages, enabled_extensions FROM sessions WHERE id = ?1",
            params![id],
            |row| {
                Ok(SessionData {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    updated_at: row.get(2)?,
                    preamble: row.get(3)?,
                    messages_json: row.get(4)?,
                    enabled_extensions_json: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(|e| Error::Database(format!("Failed to load session: {}", e)))
    }

    /// Update session messages (append-optimized pattern)
    pub fn update_messages(&self, id: &str, messages_json: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock();

        let rows = conn
            .execute(
                "UPDATE sessions SET messages = ?1, updated_at = ?2 WHERE id = ?3",
                params![messages_json, now, id],
            )
            .map_err(|e| Error::Database(format!("Failed to update messages: {}", e)))?;

        if rows == 0 {
            return Err(Error::SessionNotFound(id.to_string()));
        }

        Ok(())
    }

    /// Update session preamble
    pub fn update_preamble(&self, id: &str, preamble: Option<&str>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock();

        let rows = conn
            .execute(
                "UPDATE sessions SET preamble = ?1, updated_at = ?2 WHERE id = ?3",
                params![preamble, now, id],
            )
            .map_err(|e| Error::Database(format!("Failed to update preamble: {}", e)))?;

        if rows == 0 {
            return Err(Error::SessionNotFound(id.to_string()));
        }

        Ok(())
    }

    /// Update enabled extensions
    pub fn update_enabled_extensions(&self, id: &str, enabled_extensions_json: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock();

        let rows = conn
            .execute(
                "UPDATE sessions SET enabled_extensions = ?1, updated_at = ?2 WHERE id = ?3",
                params![enabled_extensions_json, now, id],
            )
            .map_err(|e| Error::Database(format!("Failed to update enabled extensions: {}", e)))?;

        if rows == 0 {
            return Err(Error::SessionNotFound(id.to_string()));
        }

        Ok(())
    }

    /// Delete a session
    pub fn delete_session(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock();

        let rows = conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![id])
            .map_err(|e| Error::Database(format!("Failed to delete session: {}", e)))?;

        Ok(rows > 0)
    }

    /// List all session IDs (for debugging/admin)
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock();

        let mut stmt = conn
            .prepare("SELECT id FROM sessions ORDER BY updated_at DESC")
            .map_err(|e| Error::Database(format!("Failed to prepare statement: {}", e)))?;

        let ids = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| Error::Database(format!("Failed to list sessions: {}", e)))?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(|e| Error::Database(format!("Failed to collect sessions: {}", e)))?;

        Ok(ids)
    }
}

/// Raw session data from database
#[derive(Debug, Clone)]
pub struct SessionData {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub preamble: Option<String>,
    pub messages_json: String,
    pub enabled_extensions_json: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_db() -> Database {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("goose2_test_{}.db", n));
        // Clean up any existing file
        let _ = std::fs::remove_file(&path);
        Database::open(&path).unwrap()
    }

    #[test]
    fn test_create_and_load_session() {
        let db = temp_db();

        db.create_session("test-123").unwrap();

        let session = db.load_session("test-123").unwrap().unwrap();
        assert_eq!(session.id, "test-123");
        assert_eq!(session.messages_json, "[]");
        assert_eq!(session.enabled_extensions_json, "[]");
    }

    #[test]
    fn test_update_messages() {
        let db = temp_db();

        db.create_session("test-123").unwrap();
        db.update_messages("test-123", r#"[{"role":"user","content":"hello"}]"#)
            .unwrap();

        let session = db.load_session("test-123").unwrap().unwrap();
        assert!(session.messages_json.contains("hello"));
    }

    #[test]
    fn test_update_enabled_extensions() {
        let db = temp_db();

        db.create_session("test-123").unwrap();
        db.update_enabled_extensions("test-123", r#"["develop", "browser"]"#)
            .unwrap();

        let session = db.load_session("test-123").unwrap().unwrap();
        assert!(session.enabled_extensions_json.contains("develop"));
        assert!(session.enabled_extensions_json.contains("browser"));
    }

    #[test]
    fn test_session_not_found() {
        let db = temp_db();

        let result = db.update_messages("nonexistent", "[]");
        assert!(matches!(result, Err(Error::SessionNotFound(_))));
    }
}

//! Database abstraction for the cloud server.
//!
//! Supports two backends determined by environment variables:
//!
//! - **SQLite** (default): `OZ_DB_PATH` env var (defaults to `oz-pos.db`)
//! - **PostgreSQL**: `DATABASE_URL` env var (must start with `postgres://`)
//!
//! # Usage
//!
//! ```no_run
//! let pool = DbPool::from_env().await?;
//! let conn = pool.get().await?;
//! ```

use std::str::FromStr;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::info;

/// A pooled database connection, either SQLite (behind a Mutex) or
/// PostgreSQL (via deadpool).
#[derive(Clone, Debug)]
pub enum DbPool {
    /// SQLite connection wrapped in `Arc<Mutex<>>` (compatible with
    /// `CloudServerState` and existing handlers).
    Sqlite(Arc<Mutex<rusqlite::Connection>>),
    /// PostgreSQL connection pool from deadpool-postgres.
    Postgres(deadpool_postgres::Pool),
}

impl DbPool {
    /// Create a new `DbPool` from the environment.
    ///
    /// Resolution order:
    /// 1. If `DATABASE_URL` starts with `postgres://` or `postgresql://`,
    ///    connect to PostgreSQL.
    /// 2. Otherwise, open SQLite from `OZ_DB_PATH` (default `oz-pos.db`).
    pub async fn from_env() -> Result<Self, DbError> {
        match std::env::var("DATABASE_URL") {
            Ok(url) if url.starts_with("postgres://") || url.starts_with("postgresql://") => {
                Self::connect_postgres(&url).await
            }
            _ => {
                let path = std::env::var("OZ_DB_PATH").unwrap_or_else(|_| "oz-pos.db".into());
                Self::connect_sqlite(&path)
            }
        }
    }

    /// Connect to a SQLite database at the given path.
    pub fn connect_sqlite(path: &str) -> Result<Self, DbError> {
        let mut conn = rusqlite::Connection::open(path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        oz_core::migrations::run(&mut conn)?;
        info!(db = %path, "SQLite database opened and migrations applied");
        Ok(Self::Sqlite(Arc::new(Mutex::new(conn))))
    }

    /// Create an in-memory SQLite database (for tests).
    pub fn connect_sqlite_in_memory() -> Result<Self, DbError> {
        let mut conn = rusqlite::Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        oz_core::migrations::run(&mut conn)?;
        info!("In-memory SQLite database initialized");
        Ok(Self::Sqlite(Arc::new(Mutex::new(conn))))
    }

    /// Connect to a PostgreSQL database via connection URL.
    pub async fn connect_postgres(url: &str) -> Result<Self, DbError> {
        use deadpool_postgres::{Manager, ManagerConfig, RecyclingMethod};

        let config = tokio_postgres::Config::from_str(url)
            .map_err(|e| DbError::Config(format!("invalid DATABASE_URL: {e}")))?;

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let manager = Manager::from_config(config, tokio_postgres::NoTls, mgr_config);

        let pool = deadpool_postgres::Pool::builder(manager)
            .max_size(8)
            .build()
            .map_err(|e| DbError::Pool(e.to_string()))?;

        // Verify connectivity by running a test query
        let client = pool
            .get()
            .await
            .map_err(|e| DbError::Connection(e.to_string()))?;

        client
            .execute("SELECT 1", &[])
            .await
            .map_err(|e| DbError::Connection(e.to_string()))?;

        // Create the offline_queue table if not exists (PG-compatible DDL)
        client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS offline_queue (
                    id TEXT PRIMARY KEY,
                    action TEXT NOT NULL,
                    payload TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    retry_count INTEGER NOT NULL DEFAULT 0,
                    last_error TEXT,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    synced_at TIMESTAMPTZ
                );",
            )
            .await
            .map_err(|e| DbError::Migration(e.to_string()))?;

        info!("PostgreSQL database connected and tables initialised");
        Ok(Self::Postgres(pool))
    }

    /// Get a raw SQLite connection reference, if this is a SQLite pool.
    /// Panics if called on a PostgreSQL pool.
    pub fn sqlite_conn(&self) -> Arc<Mutex<rusqlite::Connection>> {
        match self {
            Self::Sqlite(conn) => conn.clone(),
            Self::Postgres(_) => {
                panic!("called sqlite_conn() on a PostgreSQL pool")
            }
        }
    }

    /// Get a PostgreSQL client from the pool, if this is a PG pool.
    /// Panics if called on a SQLite pool.
    pub async fn pg_client(&self) -> Result<deadpool_postgres::Client, DbError> {
        match self {
            Self::Postgres(pool) => pool
                .get()
                .await
                .map_err(|e| DbError::Connection(e.to_string())),
            Self::Sqlite(_) => {
                panic!("called pg_client() on a SQLite pool")
            }
        }
    }

    /// Returns `true` if this is a PostgreSQL pool.
    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    /// Returns `true` if this is a SQLite pool.
    pub fn is_sqlite(&self) -> bool {
        matches!(self, Self::Sqlite(_))
    }
}

/// Errors that can occur during database setup.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Core error: {0}")]
    Core(#[from] oz_core::error::CoreError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Pool creation error: {0}")]
    Pool(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Migration error: {0}")]
    Migration(String),
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_in_memory_creates_db() {
        let pool = DbPool::connect_sqlite_in_memory().unwrap();
        assert!(pool.is_sqlite());
        assert!(!pool.is_postgres());
    }

    #[test]
    fn sqlite_conn_returns_connection() {
        let pool = DbPool::connect_sqlite_in_memory().unwrap();
        let conn = pool.sqlite_conn();
        let guard = conn.blocking_lock();
        let result: i64 = guard.query_row("SELECT 1", [], |row| row.get(0)).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn sqlite_migrations_run() {
        let pool = DbPool::connect_sqlite_in_memory().unwrap();
        let conn = pool.sqlite_conn();
        let guard = conn.blocking_lock();
        // Verify a core table exists after migrations
        let count: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='settings'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "settings table should exist after migrations");
    }

    #[test]
    fn sqlite_from_path_creates_db() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let path_str = path.to_str().unwrap();
        let pool = DbPool::connect_sqlite(path_str).unwrap();
        assert!(pool.is_sqlite());
        assert!(path.exists(), "database file should exist");
    }

    #[tokio::test]
    async fn postgres_url_parsing_rejects_bad_url() {
        let result = DbPool::connect_postgres("not-a-url").await;
        // This should fail because Config::from_str will reject invalid URLs
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn postgres_url_parsing_accepts_valid_url() {
        // This won't connect, but the URL parsing should succeed
        let result = DbPool::connect_postgres("postgresql://localhost:5432/test").await;
        // Will fail at connection, not parsing
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Connection") || msg.contains("connection"),
            "expected connection error, got: {msg}"
        );
    }

    #[tokio::test]
    async fn from_env_defaults_to_sqlite() {
        // Explicitly set DATABASE_URL to empty so a parallel test's
        // postgres:// value doesn't race in.
        unsafe { std::env::set_var("DATABASE_URL", "") };
        unsafe { std::env::set_var("OZ_DB_PATH", ":memory:") };
        let pool = DbPool::from_env().await.unwrap();
        assert!(pool.is_sqlite());
        unsafe { std::env::remove_var("DATABASE_URL") };
        unsafe { std::env::remove_var("OZ_DB_PATH") };
    }

    #[tokio::test]
    async fn from_env_detects_postgres_url() {
        unsafe { std::env::set_var("DATABASE_URL", "postgresql://localhost:5432/test") };
        let pool = DbPool::from_env().await;
        unsafe { std::env::remove_var("DATABASE_URL") };
        // Should attempt connection but fail
        assert!(pool.is_err());
        let msg = pool.unwrap_err().to_string();
        assert!(
            msg.contains("Connection") || msg.contains("connection"),
            "expected connection error, got: {msg}"
        );
    }

    #[test]
    fn db_error_sqlite_display() {
        let err = DbError::Sqlite(rusqlite::Error::InvalidColumnName("x".into()));
        let msg = err.to_string();
        assert!(msg.contains("SQLite error"));
    }

    #[test]
    fn db_error_config_display() {
        let err = DbError::Config("missing host".into());
        assert_eq!(err.to_string(), "Configuration error: missing host");
    }

    #[test]
    fn db_error_connection_display() {
        let err = DbError::Connection("refused".into());
        assert_eq!(err.to_string(), "Connection error: refused");
    }

    #[test]
    fn db_error_pool_display() {
        let err = DbError::Pool("no connections available".into());
        assert_eq!(
            err.to_string(),
            "Pool creation error: no connections available"
        );
    }

    #[test]
    fn db_error_migration_display() {
        let err = DbError::Migration("syntax error".into());
        assert_eq!(err.to_string(), "Migration error: syntax error");
    }

    #[test]
    fn db_error_debug() {
        let err = DbError::Config("test".into());
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn db_error_from_core_error() {
        let core_err = oz_core::CoreError::NotFound {
            entity: "table",
            id: "x".into(),
        };
        let db_err: DbError = core_err.into();
        assert!(db_err.to_string().contains("not found"));
    }
}

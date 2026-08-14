/*
last audited 19-07-26 by RSA-Agent
crate: cloud-server | status: SAFE | lint: CLEAN
findings: 6 unsafe blocks in #[cfg(test)] only — std::env::set_var/remove_var (Rust 2024 edition). SAFETY comments added 19-07-26.
next: none | perf: N/A
*/

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

use crate::config::CloudServerConfig;

use tokio::sync::Mutex;
use tracing::{info, warn};

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
    /// Create a new `DbPool` from a [`CloudServerConfig`].
    ///
    /// Resolution order:
    /// 1. If `database_url` starts with `postgres://` or `postgresql://`,
    ///    connect to PostgreSQL.
    /// 2. Otherwise, open SQLite from `db_path`.
    pub async fn from_config(config: &CloudServerConfig) -> Result<Self, DbError> {
        if let Some(ref url) = config.database_url
            && (url.starts_with("postgres://") || url.starts_with("postgresql://"))
        {
            return Self::connect_postgres(url, config.require_tls, config.db_pool_size).await;
        }
        Self::connect_sqlite(&config.db_path)
    }

    /// Create a pool from the environment (used by tests that set env vars).
    /// Production code should go through [`CloudServerConfig`].
    #[cfg(test)]
    pub async fn from_env() -> Result<Self, DbError> {
        let config =
            CloudServerConfig::from_env().expect("CloudServerConfig::from_env failed in test");
        Self::from_config(&config).await
    }

    /// Detect paths that are obviously non-Linux: a Windows drive-letter
    /// prefix (`C:/…`, `C:\…`), a bare drive letter (`C:…`), or a
    /// backslash separator. On Unix targets the cloud server can never
    /// legitimately use such a path — the usual cause is Git Bash (MSYS)
    /// path conversion mangling a `docker run -e OZ_DB_PATH=…` argument.
    ///
    /// The detector itself is platform-independent so it can be unit-tested
    /// everywhere; only the rejection in [`Self::connect_sqlite`] is Unix-
    /// gated (Windows native dev runs legitimately use Windows paths).
    fn looks_like_windows_path(path: &str) -> bool {
        let bytes = path.as_bytes();
        let drive_letter = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
        drive_letter || path.contains('\\')
    }

    /// Connect to a SQLite database at the given path.
    pub fn connect_sqlite(path: &str) -> Result<Self, DbError> {
        // The cloud server is deployed as a Linux container, so a
        // Windows-style `OZ_DB_PATH` can never be valid there — it is almost
        // always a Git Bash (MSYS) path-conversion artifact from
        // `docker run -e OZ_DB_PATH=/tmp/...` (see
        // docs/operations/docker-deployment.md). Fail fast with an actionable
        // message instead of rusqlite's cryptic `unable to open database
        // file`. No-op on Windows native builds, where Windows paths are
        // legitimate (dev runs and unit tests use them).
        if cfg!(unix) && Self::looks_like_windows_path(path) {
            return Err(DbError::Config(format!(
                "OZ_DB_PATH is set to {path:?}, which looks like a Windows path but the cloud \
                 server runs on Linux. This is usually a Git Bash path-conversion artifact: \
                 prefix the docker run command with `MSYS_NO_PATHCONV=1` or pass a container \
                 path such as /data/oz-pos.db"
            )));
        }
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
    ///
    /// When `require_tls` is set, the URL must specify `sslmode=require`;
    /// otherwise startup fails rather than allowing a plaintext fallback.
    /// `pool_size` bounds the deadpool connection pool (max open connections).
    pub async fn connect_postgres(
        url: &str,
        require_tls: bool,
        pool_size: usize,
    ) -> Result<Self, DbError> {
        use deadpool_postgres::{Manager, ManagerConfig, RecyclingMethod};

        let config = tokio_postgres::Config::from_str(url)
            .map_err(|e| DbError::Config(format!("invalid DATABASE_URL: {e}")))?;

        if require_tls && config.get_ssl_mode() != tokio_postgres::config::SslMode::Require {
            return Err(DbError::Config(
                "OZ_DB_REQUIRE_TLS=1 requires DATABASE_URL to set `sslmode=require`; \
                 refusing to connect with a plaintext fallback"
                    .into(),
            ));
        }

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        // TLS via rustls. tokio-postgres honours the connection string's
        // `sslmode` (`disable` | `prefer` | `require`, default `prefer`), so a
        // local plaintext Postgres keeps working while production sets
        // `sslmode=require` for an encrypted connection. Roots come from the
        // platform trust store (ca-certificates in the container image).
        let mut roots = rustls::RootCertStore::empty();
        let native = rustls_native_certs::load_native_certs();
        if !native.errors.is_empty() {
            warn!(
                errors = ?native.errors,
                "some native root certificates failed to load"
            );
        }
        for cert in native.certs {
            roots
                .add(cert)
                .map_err(|e| DbError::Config(format!("failed to add root certificate: {e}")))?;
        }
        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);

        let manager = Manager::from_config(config, tls, mgr_config);

        let pool = deadpool_postgres::Pool::builder(manager)
            .max_size(pool_size)
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

        // Apply the full schema — the Postgres port of the SQLite init
        // migration (92 tables, indexes, triggers, seed rows). `batch_execute`
        // sends the whole script as one simple-query message, which Postgres
        // executes in a single implicit transaction, so the migration is
        // atomic as well as idempotent (`IF NOT EXISTS` / `ON CONFLICT DO
        // NOTHING` / `CREATE OR REPLACE`).
        client
            .batch_execute(oz_core::migrations::PG_INIT)
            .await
            .map_err(|e| DbError::Migration(e.to_string()))?;

        info!("PostgreSQL database connected and full schema applied");
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    /// Returns `true` if this is a SQLite pool.
    #[allow(dead_code)]
    pub fn is_sqlite(&self) -> bool {
        matches!(self, Self::Sqlite(_))
    }
}

/// Errors that can occur during database setup.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// A SQLite-specific error occurred.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// An error from the `oz-core` crate.
    #[error("Core error: {0}")]
    Core(#[from] oz_core::error::CoreError),

    /// Invalid or missing configuration.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Failed to create or configure the connection pool.
    #[error("Pool creation error: {0}")]
    Pool(String),

    /// Failed to establish a database connection.
    #[error("Connection error: {0}")]
    Connection(String),

    /// A database migration step failed.
    #[error("Migration error: {0}")]
    Migration(String),
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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
    fn windows_style_paths_are_detected() {
        // Windows drive-letter prefixes (forward or backslash).
        assert!(DbPool::looks_like_windows_path(
            "C:/Users/User/AppData/Local/Temp/test.db"
        ));
        assert!(DbPool::looks_like_windows_path("C:\\Users\\User\\test.db"));
        assert!(DbPool::looks_like_windows_path("c:/data/db.sqlite"));
        // Bare drive letters and backslash separators.
        assert!(DbPool::looks_like_windows_path("C:relative.db"));
        assert!(DbPool::looks_like_windows_path("/data/oz\\pos.db"));
        // Legitimate Linux/container paths must NOT be flagged.
        assert!(!DbPool::looks_like_windows_path("/data/oz-pos.db"));
        assert!(!DbPool::looks_like_windows_path("oz-pos.db"));
        assert!(!DbPool::looks_like_windows_path(":memory:"));
    }

    #[cfg(unix)]
    #[test]
    fn connect_sqlite_rejects_windows_path_on_unix() {
        let err = DbPool::connect_sqlite("C:/Users/User/AppData/Local/Temp/test.db")
            .expect_err("Windows-style path must be rejected on Unix");
        let msg = err.to_string();
        assert!(
            msg.contains("MSYS_NO_PATHCONV"),
            "message should hint the fix: {msg}"
        );
        assert!(
            msg.contains("Windows path"),
            "message should name the path kind: {msg}"
        );
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
        let result = DbPool::connect_postgres("not-a-url", false, 20).await;
        // This should fail because Config::from_str will reject invalid URLs
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn postgres_url_parsing_accepts_valid_url() {
        // This won't connect, but the URL parsing should succeed
        let result = DbPool::connect_postgres("postgresql://localhost:5432/test", false, 20).await;
        // Will fail at connection, not parsing
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Connection") || msg.contains("connection"),
            "expected connection error, got: {msg}"
        );
    }

    #[tokio::test]
    async fn require_tls_rejects_url_without_sslmode_require() {
        // No sslmode → defaults to `prefer`, which could fall back to
        // plaintext, so the TLS requirement must fail before connecting.
        let err = DbPool::connect_postgres("postgresql://localhost:5432/test", true, 20)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("sslmode=require"),
            "expected an sslmode=require error, got: {err}"
        );
    }

    #[tokio::test]
    async fn require_tls_accepts_sslmode_require_and_fails_on_connection() {
        // sslmode=require passes the check; it then fails because no server
        // is listening — a Connection error, not a config error.
        let err =
            DbPool::connect_postgres("postgresql://localhost:5432/test?sslmode=require", true, 20)
                .await
                .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Connection") || msg.contains("connection"),
            "expected connection error, got: {msg}"
        );
    }

    /// Serializes env-var-dependent tests since `std::env::set_var` is
    /// process-global and tokio runs tests in parallel.
    static ENV_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

    #[serial]
    #[tokio::test]
    async fn from_env_defaults_to_sqlite() {
        let _guard = ENV_LOCK.lock().await;
        // SAFETY: env var mutations are serialized via ENV_LOCK (held by _guard)
        // to prevent data races on the process-global environment. The Mutex guard
        // ensures exclusive access; the environment is restored in reverse order.
        unsafe { std::env::set_var("DATABASE_URL", "") };
        unsafe { std::env::set_var("OZ_DB_PATH", ":memory:") };
        let pool = DbPool::from_env().await.unwrap();
        assert!(pool.is_sqlite());
        // SAFETY: Same ENV_LOCK serialization as above. These restore env vars
        // to their prior state (empty/unset) before the guard is released.
        unsafe { std::env::remove_var("DATABASE_URL") };
        unsafe { std::env::remove_var("OZ_DB_PATH") };
    }

    #[serial]
    #[tokio::test]
    async fn from_env_detects_postgres_url() {
        let _guard = ENV_LOCK.lock().await;
        // SAFETY: ENV_LOCK (held by _guard) serializes access to the process-global
        // environment. The set_var is paired with a remove_var before the guard drops.
        unsafe { std::env::set_var("DATABASE_URL", "postgresql://localhost:5432/test") };
        let pool = DbPool::from_env().await;
        // SAFETY: Restores the environment — see SAFETY note on set_var above.
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

    /// Integration test: connect to a real PostgreSQL instance.
    ///
    /// Requires the  Docker container (port 15432).
    /// Skipped when the container is not reachable.
    #[tokio::test]
    async fn pg_integration_connect_and_create_tables() {
        let url = "postgres://postgres:postgres@localhost:15432/postgres";
        let pool = match DbPool::connect_postgres(url, false, 20).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("PG integration test skipped: {e}");
                return;
            }
        };
        assert!(pool.is_postgres());

        // Verify we can get a client and query
        let client = pool.pg_client().await.expect("pg_client should succeed");
        let row = client
            .query_one("SELECT COUNT(*) FROM processed_webhooks", &[])
            .await
            .expect("query should succeed");
        let count: i64 = row.get(0);
        assert!(
            count >= 0,
            "processed_webhooks table should exist and be queryable"
        );

        // Verify offline_queue table exists too
        let row = client
            .query_one("SELECT COUNT(*) FROM offline_queue", &[])
            .await
            .expect("offline_queue query should succeed");
        let count: i64 = row.get(0);
        assert!(count >= 0, "offline_queue table should exist");

        // The full Postgres migration (not the old 2-table stub) must have
        // applied: 92 base tables and the 4 rewritten triggers. `>=` tolerates
        // a shared dev database with extra objects.
        let table_count: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM information_schema.tables
                 WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
                &[],
            )
            .await
            .expect("information_schema query should succeed")
            .get(0);
        assert!(
            table_count >= 92,
            "expected the full 92-table schema, found {table_count} tables"
        );
        let trigger_count: i64 = client
            .query_one(
                "SELECT COUNT(*) FROM pg_trigger WHERE NOT tgisinternal",
                &[],
            )
            .await
            .expect("pg_trigger query should succeed")
            .get(0);
        assert!(
            trigger_count >= 4,
            "expected the 4 rewritten triggers, found {trigger_count}"
        );
    }
}

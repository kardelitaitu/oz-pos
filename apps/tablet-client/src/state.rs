//! `AppState` — the long-lived state managed by Tauri and reached via
//! `State<'_, AppState>` in every command.
//!
//! Holds:
//! - A `rusqlite::Connection` (behind a `tokio::sync::Mutex`) for DB access.
//! - A `DriverRegistry` from `oz_hal` for hardware access.
//! - The Tauri `AppHandle` for emitting events back to the front-end.
//!
//! `AppState::new` opens the local SQLite database, runs migrations, and
//! creates an empty `DriverRegistry`. Hardware is registered at runtime
//! via the setup wizard (or a future `init_hardware` command); the front
//! end never assumes a particular device is plugged in at startup.
//!
//! # Connection pooling
//!
//! The `Mutex<Connection>` here is a single-connection placeholder. A
//! real deployment will switch to `r2d2_sqlite` or `deadpool-sqlite`
//! so that Tauri commands can issue concurrent reads (the `rust-backend`
//! skill prescribes this; switching is mechanical).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use rusqlite::Connection;
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::{Mutex, oneshot};

use oz_core::migrations;
use oz_core::session::SessionContext;
use oz_hal::DriverRegistry;
use platform_core::StoreDatabaseManager;
use platform_kernel::Kernel;

use crate::error::AppError;

/// Shared application state.
pub struct AppState {
    /// SQLite connection for the local store. Wrapped in a `Mutex` so
    /// commands can borrow it across `.await` points safely.
    pub db: Mutex<Connection>,

    /// HAL driver registry. Use `state.registry.scanner(id)` etc.
    pub registry: Arc<DriverRegistry>,

    /// Tauri app handle, used for emitting events to the front-end.
    /// `None` in test or headless contexts where no UI is attached.
    pub app: Option<AppHandle>,

    /// Path to the SQLite database file (for diagnostics + `oz-cli` reuse).
    pub db_path: PathBuf,

    /// Cancel-sender for the active barcode scanner background task.
    /// When `Some`, the scanner polling loop is running; dropping
    /// or signalling it stops the loop gracefully.
    pub scanner_cancel: Mutex<Option<oneshot::Sender<()>>>,

    /// Module system kernel. Manages module lifecycle (load → start → stop).
    /// Modules are registered in `lib.rs::run()` during setup.
    pub kernel: Mutex<Kernel>,

    /// In-memory session store mapping opaque session tokens to resolved
    /// [`SessionContext`] values. ADR #4 / ADR #7.
    pub session_store: Arc<RwLock<HashMap<String, SessionContext>>>,

    /// Session TTL in seconds. Read from `session.ttl_seconds` setting
    /// at startup; defaults to 86400 (24 hours). Set to 0 to disable
    /// session expiry (development mode).
    pub session_ttl_seconds: i64,

    /// Terminal identifier for multi-terminal deployments.
    ///
    /// Set once at startup or via set_feature(MultiTerminal, true).
    /// Consumers (Redis pub/sub subscriber, inventory change publisher)
    /// read this field instead of calling std::env::var().
    pub terminal_id: tokio::sync::Mutex<Option<String>>,

    /// Store-scoped database manager (ADR #4 Phase 2 / ADR #7).
    /// Each resolved store is opened in its own migrated SQLite database.
    pub db_manager: StoreDatabaseManager,

    /// Per-process secret for the pre-session picker ticket HMAC.
    ///
    /// Parity with the desktop client (audit/06 residual). Generated
    /// once at startup. Tickets are short-lived (5 min) and die with
    /// the process, so the secret is never persisted — a restart
    /// simply invalidates outstanding tickets.
    pub picker_ticket_secret: Vec<u8>,
}

impl AppState {
    /// Open the DB at `<app_data_dir>/oz-pos.db`, run migrations, and
    /// create the empty driver registry.
    pub fn new(app: &AppHandle) -> Result<Self, AppError> {
        let db_path = resolve_db_path(app)?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Internal(format!("creating db dir {parent:?}: {e}")))?;
        }

        let mut conn = Connection::open(&db_path)
            .map_err(|e| AppError::Internal(format!("opening {db_path:?}: {e}")))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| AppError::Internal(format!("enabling foreign_keys: {e}")))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| AppError::Internal(format!("enabling WAL: {e}")))?;

        migrations::run(&mut conn)
            .map_err(|e| AppError::Internal(format!("running migrations: {e}")))?;

        // ── Popularity full pass (ADR #37) ────────────────────────────
        // Materialize popularity scores right after migrations so product
        // lookups rank recently-managed items from the first launch (sales
        // come from sale_lines, edit events were seeded by migration 134,
        // search events accumulate from launch). Local-only analytics — a
        // failure must not block startup.
        if let Err(e) = oz_core::db::Store::new(&conn).recompute_all_popularity() {
            tracing::warn!(
                error = %e,
                "popularity full pass failed; product popularity sort falls back"
            );
        }

        // ── Session TTL ──────────────────────────────────────────────
        // Read from settings; default 24h. 0 or missing = no expiry.
        let session_ttl_seconds: i64 = oz_core::Settings::get(&conn, "session.ttl_seconds")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(86400);

        let data_dir = db_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let db_manager = StoreDatabaseManager::new(data_dir, oz_core::migrations::ALL);
        let registry = Arc::new(DriverRegistry::default());

        tracing::info!(?db_path, "AppState initialised");

        Ok(Self {
            db: Mutex::new(conn),
            registry,
            app: Some(app.clone()),
            db_path,
            scanner_cancel: Mutex::new(None),
            kernel: Mutex::new(Kernel::new()),
            session_store: Arc::new(RwLock::new(HashMap::new())),
            session_ttl_seconds,
            terminal_id: Mutex::new(None),
            db_manager,
            picker_ticket_secret: uuid::Uuid::new_v4().as_bytes().to_vec(),
        })
    }

    /// Resolve an opaque session token to its [`SessionContext`].
    ///
    /// ADR #4 / ADR #7: Commands call this to look up the caller's
    /// resolved scope. Returns `AppError::InvalidSession` if the
    /// token is unknown OR if the session has expired (TTL check).
    ///
    /// Expired sessions are atomically removed during resolution.
    ///
    /// Uses a double-check lock pattern: the fast path (valid session)
    /// only acquires a shared read lock. The exclusive write lock is
    /// only acquired when a session is actually expired, which is rare.
    pub fn resolve_session(&self, token: &str) -> Result<SessionContext, AppError> {
        // Fast path: read-only check.
        {
            let store = self
                .session_store
                .read()
                .map_err(|e| AppError::Internal(format!("session store lock poisoned: {e}")))?;

            match store.get(token) {
                Some(ctx) if !ctx.is_expired() => return Ok(ctx.clone()),
                Some(_) => {} // expired — fall through to write-lock path
                None => return Err(AppError::InvalidSession),
            }
        }

        // Slow path: expired session — acquire write lock to remove it.
        let mut store = self
            .session_store
            .write()
            .map_err(|e| AppError::Internal(format!("session store lock poisoned: {e}")))?;

        // Double-check: another thread may have already removed or refreshed it.
        if let Some(ctx) = store.get(token)
            && ctx.is_expired()
        {
            store.remove(token);
            tracing::info!(token = %token, "session expired — removed from store");
        }

        Err(AppError::InvalidSession)
    }

    /// Resolve a session token and return its context and store-scoped database.
    ///
    /// ADR #7: The session determines the store; callers never supply a
    /// store identifier directly for scoped commands.
    pub fn resolve_scope(
        &self,
        token: &str,
    ) -> Result<(SessionContext, std::sync::Arc<std::sync::Mutex<Connection>>), AppError> {
        let session = self.resolve_session(token)?;
        let conn = self
            .db_manager
            .open_store(&session.store_id)
            .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
        Ok((session, conn))
    }

    /// Resolve a session token and return only its store-scoped database.
    pub fn resolve_store(
        &self,
        token: &str,
    ) -> Result<std::sync::Arc<std::sync::Mutex<Connection>>, AppError> {
        self.resolve_scope(token).map(|(_, conn)| conn)
    }

    /// Remove every session bound to `user_id` except the token in
    /// `keep_token` (STAFF-03 PIN rotation).
    ///
    /// The caller's own session is preserved — they authenticated moments
    /// ago and the UI follows up with a reload using the same token — while
    /// stale terminal sessions issued under the old PIN are invalidated.
    pub fn invalidate_user_sessions_except(&self, user_id: &str, keep_token: &str) -> usize {
        let mut store = match self.session_store.write() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("session store lock poisoned during invalidation: {e}");
                return 0;
            }
        };
        let before = store.len();
        store.retain(|token, ctx| {
            ctx.user_id != user_id || (!keep_token.is_empty() && token == keep_token)
        });
        let removed = before - store.len();
        if removed > 0 {
            tracing::info!(
                user_id = %user_id,
                removed = %removed,
                keep_token = %keep_token,
                "sessions invalidated after PIN rotation"
            );
        }
        removed
    }

    /// Remove all expired sessions from the store in a single sweep.
    /// Called periodically by the background session-cleanup daemon.
    pub fn prune_expired_sessions(&self) -> usize {
        let mut store = match self.session_store.write() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("session store lock poisoned during prune: {e}");
                return 0;
            }
        };
        let before = store.len();
        store.retain(|_, ctx| !ctx.is_expired());
        let pruned = before - store.len();
        if pruned > 0 {
            tracing::info!(
                "pruned {pruned} expired session(s), {remaining} remain",
                remaining = store.len()
            );
        }
        pruned
    }
}

fn resolve_db_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("resolving app data dir: {e}")))?;
    Ok(dir.join("oz-pos.db"))
}

impl Drop for AppState {
    fn drop(&mut self) {
        tracing::info!("stopping kernel modules");
        // Retry the lock for up to 500ms before giving up. This addresses
        // a Windows window-lifecycle bottleneck where `try_lock()` would
        // silently skip `stop_all()` if a Tauri command was mid-execution.
        // A single `try_lock()` is too aggressive during shutdown because
        // commands may still be draining. But `blocking_lock()` risks a
        // deadlock if a command holding the lock is waiting for the runtime
        // to shut down (circular dependency). The bounded retry loop
        // gives commands time to finish while guaranteeing the Drop
        // doesn't hang indefinitely.
        const DROP_LOCK_RETRIES: usize = 50;
        let mut stopped = false;
        for _ in 0..DROP_LOCK_RETRIES {
            if let Ok(mut kernel) = self.kernel.try_lock() {
                let _ = kernel.stop_all();
                stopped = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if !stopped {
            tracing::warn!(
                "kernel lock contended after 500ms, skipping stop_all — \
                 modules may not have been stopped cleanly"
            );
        }
    }
}

#[cfg(test)]
impl AppState {
    /// Construct an `AppState` suitable for unit tests.
    /// Creates a lightweight Tauri app handle via `tauri::test::mock_builder`.
    pub fn for_test() -> Self {
        Self {
            db: Mutex::new(Connection::open_in_memory().unwrap()),
            registry: Arc::new(DriverRegistry::default()),
            app: None,
            db_path: ":memory:".into(),
            scanner_cancel: Mutex::new(None),
            kernel: Mutex::new(Kernel::new()),
            session_store: Arc::new(RwLock::new(HashMap::new())),
            session_ttl_seconds: 86400,
            terminal_id: Mutex::new(None),
            db_manager: StoreDatabaseManager::new(std::env::temp_dir(), oz_core::migrations::ALL),
            picker_ticket_secret: b"test-picker-ticket-secret".to_vec(),
        }
    }

    /// Construct an `AppState` with a pre-configured connection (migrations
    /// already run). Used by integration tests that need a seeded database.
    pub fn for_test_with_conn(conn: Connection) -> Self {
        Self {
            db: Mutex::new(conn),
            registry: Arc::new(DriverRegistry::default()),
            app: None,
            db_path: ":memory:".into(),
            scanner_cancel: Mutex::new(None),
            kernel: Mutex::new(Kernel::new()),
            session_store: Arc::new(RwLock::new(HashMap::new())),
            session_ttl_seconds: 86400,
            terminal_id: Mutex::new(None),
            db_manager: StoreDatabaseManager::new(std::env::temp_dir(), oz_core::migrations::ALL),
            picker_ticket_secret: b"test-picker-ticket-secret".to_vec(),
        }
    }

    /// Construct a test state with an isolated store database directory.
    ///
    /// Injecting the manager lets scope tests prove that a session cannot
    /// observe another store's database file.
    pub fn for_test_with_db_manager(db_manager: StoreDatabaseManager) -> Self {
        let mut state = Self::for_test();
        state.db_manager = db_manager;
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::session::SessionContext;

    #[test]
    fn for_test_creates_valid_state() {
        let state = AppState::for_test();
        assert_eq!(state.db_path, std::path::PathBuf::from(":memory:"));
        assert!(state.app.is_none());
        assert!(
            state.db.try_lock().is_ok(),
            "in-memory DB should be accessible"
        );
    }

    #[test]
    fn for_test_with_conn_preserves_connection() {
        let conn = Connection::open_in_memory().unwrap();
        let state = AppState::for_test_with_conn(conn);
        let guard = state.db.try_lock().expect("db mutex should be available");
        // Verify it's a live SQLite connection.
        guard
            .execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(1);")
            .unwrap();
        let count: i32 = guard
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn resolve_session_empty_token_returns_invalid() {
        let state = AppState::for_test();
        let result = state.resolve_session("");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn resolve_session_missing_token_returns_invalid() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn resolve_session_expired_token_is_rejected_and_removed() {
        let state = AppState::for_test();
        let ctx = SessionContext {
            user_id: "expired-user".into(),
            store_id: "store-expired".into(),
            role_id: "role-owner".into(),
            terminal_id: "term-1".into(),
            instance_id: "inst-1".into(),
            type_key: "pos".into(),
            expires_at: Some(1),
            created_at: 0,
        };
        state
            .session_store
            .write()
            .unwrap()
            .insert("expired-token".into(), ctx);

        assert!(matches!(
            state.resolve_session("expired-token"),
            Err(AppError::InvalidSession)
        ));
        assert!(
            !state
                .session_store
                .read()
                .unwrap()
                .contains_key("expired-token")
        );
    }

    #[test]
    fn resolve_scope_isolates_store_databases() {
        let test_dir =
            std::env::temp_dir().join(format!("oz-pos-tablet-scope-test-{}", uuid::Uuid::now_v7()));
        let manager = StoreDatabaseManager::new(test_dir.clone(), oz_core::migrations::ALL);
        let state = AppState::for_test_with_db_manager(manager);
        for (token, store_id) in [("token-a", "store-a"), ("token-b", "store-b")] {
            state.session_store.write().unwrap().insert(
                token.into(),
                SessionContext {
                    user_id: "user-1".into(),
                    store_id: store_id.into(),
                    role_id: "role-owner".into(),
                    terminal_id: "term-1".into(),
                    instance_id: "inst-1".into(),
                    type_key: "pos".into(),
                    expires_at: None,
                    created_at: 0,
                },
            );
        }

        let (_, store_a) = state.resolve_scope("token-a").unwrap();
        let conn_a = store_a.lock().unwrap();
        conn_a
            .execute_batch(
                "CREATE TABLE scope_probe (value TEXT NOT NULL); INSERT INTO scope_probe VALUES ('A');",
            )
            .unwrap();
        drop(conn_a);

        let (_, store_b) = state.resolve_scope("token-b").unwrap();
        let conn_b = store_b.lock().unwrap();
        let table_count: i64 = conn_b
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'scope_probe'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 0, "store B must not see store A data");
        drop(conn_b);
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn resolve_session_valid_token_returns_context() {
        let state = AppState::for_test();
        let ctx = SessionContext {
            user_id: "user-1".into(),
            store_id: "store-1".into(),
            role_id: "role-1".into(),
            terminal_id: "term-1".into(),
            instance_id: "inst-1".into(),
            type_key: "pos".into(),
            expires_at: None,
            created_at: 0,
        };
        {
            let mut store = state.session_store.write().unwrap();
            store.insert("valid-token".into(), ctx.clone());
        }
        let result = state.resolve_session("valid-token");
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.user_id, "user-1");
        assert_eq!(resolved.store_id, "store-1");
    }

    #[test]
    fn resolve_session_returns_clone_not_reference() {
        let state = AppState::for_test();
        let original = SessionContext {
            user_id: "u1".into(),
            store_id: "s1".into(),
            role_id: "r1".into(),
            terminal_id: "t1".into(),
            instance_id: "i1".into(),
            type_key: "pos".into(),
            expires_at: None,
            created_at: 0,
        };
        {
            let mut store = state.session_store.write().unwrap();
            store.insert("tok".into(), original.clone());
        }
        let resolved = state.resolve_session("tok").unwrap();
        // Mutating the original in the store should not affect the resolved clone.
        {
            let mut store = state.session_store.write().unwrap();
            if let Some(ctx) = store.get_mut("tok") {
                ctx.user_id = "changed".into();
            }
        }
        assert_eq!(resolved.user_id, "u1");
    }
}

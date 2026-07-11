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
        })
    }

    /// Resolve an opaque session token to its [`SessionContext`].
    ///
    /// ADR #4 / ADR #7: Commands call this to look up the caller's
    /// resolved scope.
    /// Returns `AppError::InvalidSession` if the token is unknown.
    pub fn resolve_session(&self, token: &str) -> Result<SessionContext, AppError> {
        let store = self
            .session_store
            .read()
            .map_err(|e| AppError::Internal(format!("session store lock poisoned: {e}")))?;
        store.get(token).cloned().ok_or(AppError::InvalidSession)
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
        if let Ok(mut kernel) = self.kernel.try_lock() {
            let _ = kernel.stop_all();
        } else {
            tracing::warn!("kernel lock contended, skipping stop_all");
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
        }
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
    fn resolve_session_valid_token_returns_context() {
        let state = AppState::for_test();
        let ctx = SessionContext {
            user_id: "user-1".into(),
            store_id: "store-1".into(),
            role_id: "role-1".into(),
            terminal_id: "term-1".into(),
            instance_id: "inst-1".into(),
            type_key: "pos".into(),
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

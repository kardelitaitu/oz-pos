/*
last audited 12-07-27 by C-2 env-var fix
crate: oz-pos-app | status: SAFE (C-2 resolved) | lint: CLEAN
findings: unsafe env::set_var removed; terminal_id typed field added; Drop bounded retry applied | next: consolidate lock types; shutdown channels; SQLCipher | perf: Arc-clones on checkout hot path
*/

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
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use notify::Watcher as _;
use oz_core::cache::Cache;
use oz_plugin::PluginManager;

use rusqlite::Connection;
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::{Mutex, oneshot};

use oz_core::migrations;
use oz_core::session::SessionContext;
use oz_hal::DriverRegistry;
use platform_core::StoreDatabaseManager;
use platform_kernel::Kernel;
use platform_sync::daemon::SyncDaemon;

use crate::error::AppError;

/// Shared application state.
pub struct AppState {
    /// SQLite connection for the local store. Wrapped in `Arc<Mutex<..>>` so
    /// the background sync daemon can hold a reference.
    pub db: Arc<Mutex<Connection>>,

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

    /// Optional plugin manager for custom Lua business rules.
    /// `None` when no `plugins/` directory exists or loading failed.
    /// Wrapped in an `Arc<Mutex>` to share with background hot-reload task.
    pub plugins: Arc<Mutex<Option<PluginManager>>>,

    /// Plugin file watcher (kept alive to prevent dropping).
    pub plugin_watcher: Option<notify::RecommendedWatcher>,

    /// Background sync daemon. Started during app setup via
    /// [`SyncDaemon::start`](platform_sync::daemon::SyncDaemon::start).
    pub sync_daemon: SyncDaemon,

    /// Caching layer (Redis-backed when configured, no-op otherwise).
    /// Shared across all `Store` instances via `Arc`.
    pub cache: Arc<dyn Cache>,

    /// Store-scoped database manager (ADR #4 Phase 2).
    ///
    /// Manages per-store SQLite files created when additional stores
    /// are added. The global database (store_profiles, users, terminals)
    /// is accessed via `db_manager.global()`.
    pub db_manager: StoreDatabaseManager,

    /// Shutdown sender for the inventory pub/sub background listener.
    /// Dropped on app shutdown to stop the listener thread gracefully.
    pub inventory_pubsub_shutdown: Option<std::sync::mpsc::Sender<()>>,

    /// In-memory session store mapping opaque session tokens to resolved
    /// [`SessionContext`] values. ADR #4 / ADR #7.
    ///
    /// Tokens are randomly-generated UUIDs created during login/session
    /// resolution. Commands look up their context via [`AppState::resolve_session`].
    pub session_store: Arc<RwLock<HashMap<String, SessionContext>>>,

    /// Terminal identifier for multi-terminal deployments.
    ///
    /// Set once at startup from the registered terminal matching this
    /// device's hostname. Commands that auto-register a terminal via
    /// `set_feature(MultiTerminal, true)` update this field directly
    /// instead of mutating the process env (which is UB from async
    /// tokio workers). Consumers (Redis pub/sub subscriber, inventory
    /// change publisher) read this field.
    pub terminal_id: Arc<std::sync::Mutex<Option<String>>>,
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

        // Seed the primary store profile if none exists.
        seed_primary_store(&conn)
            .map_err(|e| AppError::Internal(format!("seeding primary store: {e}")))?;

        // ── Cache layer initialisation (read settings BEFORE moving conn) ──
        let redis_url =
            oz_core::Settings::get_redis_url(&conn).unwrap_or_else(|_| "redis://127.0.0.1/".into());
        let cache_ttl = oz_core::Settings::get_redis_cache_ttl(&conn).unwrap_or(300);
        let cache = platform_startup::init_cache(&redis_url, cache_ttl);

        // ── OZ_TERMINAL_ID for multi-terminal support ───────────────
        // On subsequent launches where MultiTerminal is already enabled,
        // look up the registered terminal by hostname. The terminal_id is
        // stored in AppState (typed field) instead of the process env var.
        // The Redis pub/sub subscriber and inventory change publisher read
        // it from this field — they no longer call std::env::var().
        let terminal_id: Arc<std::sync::Mutex<Option<String>>> =
            Arc::new(std::sync::Mutex::new(None));
        let reg = oz_core::Settings::load_features(&conn).unwrap_or_default();
        if reg.is_enabled(oz_core::Feature::MultiTerminal) {
            let device_id = std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_default();
            if !device_id.is_empty() {
                let store = oz_core::db::Store::new(&conn);
                if let Ok(Some(terminal)) = store.get_terminal_by_device_id(&device_id) {
                    *terminal_id.lock().unwrap() = Some(terminal.id.clone());
                    tracing::info!(
                        terminal_id = %terminal.id,
                        device_id = %device_id,
                        "terminal_id set at startup for multi-terminal"
                    );
                }
            }
        }

        // ── Start inventory pub/sub listener (Redis only) ────────────
        let pubsub_terminal_id = terminal_id.lock().unwrap().clone();
        let inventory_pubsub_shutdown =
            cache.start_inventory_pubsub(cache.clone(), pubsub_terminal_id);
        if inventory_pubsub_shutdown.is_some() {
            tracing::info!("inventory pub/sub listener started");
        }

        let db = Arc::new(Mutex::new(conn));

        // ── Store-scoped database manager (ADR #4 Phase 2) ────────
        let db_dir = db_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let db_manager = StoreDatabaseManager::new(db_dir, oz_core::migrations::ALL);

        let registry = Arc::new(DriverRegistry::default());

        // Load plugins from <app_data_dir>/plugins/.
        let plugins_dir = app.path().app_data_dir().ok().map(|d| d.join("plugins"));
        let plugins: Arc<Mutex<Option<PluginManager>>> =
            Arc::new(Mutex::new(plugins_dir.as_ref().and_then(
                |dir| match PluginManager::new(dir) {
                    Ok(pm) => Some(pm),
                    Err(e) => {
                        tracing::warn!(error = %e, "initialising plugin manager");
                        None
                    }
                },
            )));

        // Start plugin hot-reload file watcher.
        let plugin_watcher = plugins_dir.as_ref().and_then(|dir| {
            if !dir.exists() {
                return None;
            }
            start_plugin_watcher(plugins.clone(), dir.clone())
        });

        tracing::info!(
            cache_healthy = cache.is_healthy(),
            ?db_path,
            plugins_loaded = plugins.try_lock().map(|g| g.is_some()).unwrap_or(false),
            "AppState initialised"
        );

        Ok(Self {
            db,
            db_manager,
            registry,
            app: Some(app.clone()),
            db_path,
            scanner_cancel: Mutex::new(None),
            kernel: Mutex::new(Kernel::new()),
            plugins,
            plugin_watcher,
            sync_daemon: SyncDaemon::new(),
            cache,
            inventory_pubsub_shutdown,
            session_store: Arc::new(RwLock::new(HashMap::new())),
            terminal_id,
        })
    }
}

/// Seed the default primary store profile if the table is empty.
///
/// Called once on first startup after migrations run. Subsequent
/// launches find the existing row and skip the insert.
fn seed_primary_store(conn: &Connection) -> Result<(), rusqlite::Error> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM store_profiles", [], |r| r.get(0))?;
    if count == 0 {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute(
            "INSERT INTO store_profiles (id, name, address, tax_id, currency, timezone, is_primary, created_at, updated_at)
             VALUES ('default', 'Main Store', '', '', 'USD', 'UTC', 1, ?1, ?1)",
            rusqlite::params![now],
        )?;
        tracing::info!("seeded default primary store profile");
    }
    Ok(())
}

impl AppState {
    /// Create a [`Store`] with the shared cache layer and terminal
    /// identity for pub/sub message tagging.
    ///
    /// Command handlers should use this instead of `Store::new(&conn)`
    /// to benefit from Redis caching (when configured) and to ensure
    /// inventory-change pub/sub messages are correctly tagged with the
    /// terminal's identity.
    pub fn store<'a>(&self, conn: &'a Connection) -> oz_core::db::Store<'a> {
        let tid = self.terminal_id.lock().unwrap().clone();
        oz_core::db::Store::with_cache(conn, self.cache.clone()).with_terminal_id(tid)
    }

    /// Resolve an opaque session token to its [`SessionContext`].
    ///
    /// ADR #4 / ADR #7: Commands call this to look up the caller's
    /// resolved scope (store, instance, type, user, role, terminal).
    /// Returns `AppError::InvalidSession` if the token is unknown.
    pub fn resolve_session(&self, token: &str) -> Result<SessionContext, AppError> {
        let store = self
            .session_store
            .read()
            .map_err(|e| AppError::Internal(format!("session store lock poisoned: {e}")))?;
        store.get(token).cloned().ok_or(AppError::InvalidSession)
    }

    /// Resolve a session token and open the store-scoped database.
    ///
    /// ADR #7: Convenience method combining `resolve_session` +
    /// `db_manager.open_store` in one call. Most domain commands
    /// should use this instead of the global `db` field.
    ///
    /// Returns the resolved [`SessionContext`] and an [`Arc`]`<Mutex<Connection>>`
    /// for the store-scoped SQLite database. The caller must call `.lock()` on
    /// the returned connection before querying.
    pub fn resolve_scope(
        &self,
        token: &str,
    ) -> Result<(SessionContext, Arc<std::sync::Mutex<Connection>>), AppError> {
        let session = self.resolve_session(token)?;
        let conn = self
            .db_manager
            .open_store(&session.store_id)
            .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
        Ok((session, conn))
    }

    /// Resolve a session token and return only the store-scoped database
    /// connection. Convenience wrapper for commands that don't need the
    /// [`SessionContext`] (e.g., `adjust_stock_scoped`).
    pub fn resolve_store(
        &self,
        token: &str,
    ) -> Result<Arc<std::sync::Mutex<Connection>>, AppError> {
        self.resolve_scope(token).map(|(_, conn)| conn)
    }
}

/// Start a background file watcher that hot-reloads plugins when
/// `.lua` or `plugin.toml` files change in `plugins_dir`.
fn start_plugin_watcher(
    plugins: Arc<Mutex<Option<PluginManager>>>,
    plugins_dir: PathBuf,
) -> Option<notify::RecommendedWatcher> {
    let reload_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = reload_flag.clone();

    let mut watcher = notify::RecommendedWatcher::new(
        move |_res: Result<notify::Event, notify::Error>| {
            flag_clone.store(true, Ordering::SeqCst);
        },
        notify::Config::default(),
    )
    .map_err(|e| tracing::warn!(error = %e, "failed to create plugin file watcher"))
    .ok()?;

    watcher
        .watch(&plugins_dir, notify::RecursiveMode::Recursive)
        .map_err(|e| tracing::warn!(error = %e, "failed to watch plugins directory"))
        .ok()?;

    tracing::info!(dir = %plugins_dir.display(), "plugin hot-reload watcher started");

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if reload_flag.swap(false, Ordering::SeqCst) {
                tracing::info!("plugin change detected, hot-reloading…");
                let mut guard = plugins.lock().await;
                match PluginManager::new(&plugins_dir) {
                    Ok(pm) => {
                        *guard = Some(pm);
                        tracing::info!("plugins hot-reloaded successfully");
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "failed to hot-reload plugins, keeping old runtime"
                        );
                    }
                }
            }
        }
    });

    Some(watcher)
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
        let db = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        Self {
            db,
            db_manager: StoreDatabaseManager::new(std::env::temp_dir(), oz_core::migrations::ALL),
            registry: Arc::new(DriverRegistry::default()),
            app: None,
            db_path: ":memory:".into(),
            scanner_cancel: Mutex::new(None),
            kernel: Mutex::new(Kernel::new()),
            plugins: Arc::new(Mutex::new(None)),
            plugin_watcher: None,
            sync_daemon: SyncDaemon::new(),
            cache: oz_core::cache::create_cache("redis://127.0.0.1/", 300),
            inventory_pubsub_shutdown: None,
            session_store: Arc::new(RwLock::new(HashMap::new())),
            terminal_id: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_session_returns_context_for_valid_token() {
        let state = AppState::for_test();
        let ctx = SessionContext::new(
            "u1".into(),
            "r1".into(),
            "t1".into(),
            "s1".into(),
            "i1".into(),
            "type1".into(),
        );
        state
            .session_store
            .write()
            .unwrap()
            .insert("tok-abc".into(), ctx.clone());

        let resolved = state.resolve_session("tok-abc").unwrap();
        assert_eq!(resolved.store_id, "s1");
        assert_eq!(resolved.user_id, "u1");
    }

    #[test]
    fn resolve_session_returns_error_for_unknown_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn resolve_session_with_empty_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn resolve_session_returns_full_context() {
        let state = AppState::for_test();
        let ctx = SessionContext::new(
            "user-full".into(),
            "role-manager".into(),
            "term-kitchen".into(),
            "store-main".into(),
            "instance-1".into(),
            "kds".into(),
        );
        state
            .session_store
            .write()
            .unwrap()
            .insert("tok-full".into(), ctx);

        let resolved = state.resolve_session("tok-full").unwrap();
        assert_eq!(resolved.user_id, "user-full");
        assert_eq!(resolved.role_id, "role-manager");
        assert_eq!(resolved.terminal_id, "term-kitchen");
        assert_eq!(resolved.store_id, "store-main");
        assert_eq!(resolved.instance_id, "instance-1");
        assert_eq!(resolved.type_key, "kds");
    }

    #[test]
    fn resolve_session_clone_preserves_all_fields() {
        let state = AppState::for_test();
        let ctx = SessionContext::new(
            "u1".into(),
            "r1".into(),
            "t1".into(),
            "s1".into(),
            "i1".into(),
            "type1".into(),
        );
        state
            .session_store
            .write()
            .unwrap()
            .insert("tok".into(), ctx.clone());

        let resolved = state.resolve_session("tok").unwrap();
        // Clone should produce identical values
        let cloned = resolved.clone();
        assert_eq!(cloned.store_id, "s1");
        assert_eq!(cloned.user_id, "u1");
        assert_eq!(cloned.type_key, "type1");
    }

    #[tokio::test]
    async fn store_method_creates_store_with_cache() {
        let state = AppState::for_test();
        let conn = state.db.lock().await;
        let store = state.store(&conn);
        let _ = store;
    }

    #[test]
    fn for_test_creates_valid_state() {
        let state = AppState::for_test();
        assert_eq!(state.db_path.to_str(), Some(":memory:"));
        assert!(state.app.is_none());
        assert!(state.plugin_watcher.is_none());
    }
}

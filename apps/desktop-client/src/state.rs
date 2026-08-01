/*
last audited 12-07-27 by C-2 env-var fix
crate: oz-pos-app | status: SAFE (C-2 resolved; M-4, M-5 fixed) | lint: CLEAN
findings: unsafe env::set_var removed; terminal_id typed field added; Drop bounded retry applied; M-4 logging; M-5 plugin task handle | next: SQLCipher | perf: Arc-clones on checkout hot path
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
//!
//! # Sync primitive convention (M-1)
//!
//! | Primitive | Where | Why |
//! |-----------|-------|-----|
//! | `tokio::sync::Mutex` | Every async-accessible field (`db`, `kernel`, `plugins`, `scanner_cancel`, `terminal_id`) | `.lock().await` is required in Tauri command handlers; calling `.lock()` on `std::sync::Mutex` from async code blocks the tokio worker thread. |
//! | `std::sync::RwLock` | `session_store` only | Accessed from both sync (`resolve_session`, `create_session`) and async (`session cleanup daemon`) code. `tokio::sync::RwLock::read()` would panic if called from sync context without a blocking wrapper. Keep `std::sync::RwLock` and wrap async access with `tokio::task::spawn_blocking` when necessary. |
//! | `std::sync::mpsc` | `inventory_pubsub_shutdown` only | Used from `Drop` which is sync-only. Tokio channels don't implement `Sync` and would require an async `Drop` bound. |
//! | `Arc<AtomicBool>` | Plugin reload flag | Lock-free flag set by the `notify` callback (sync) and consumed by the tokio loop (async). Correct by design — no `.lock()` at all. |
//!
//! **Rule of thumb:** When adding a new field to `AppState`, default to
//! `tokio::sync::Mutex` unless the field is accessed exclusively from
//! sync code (e.g. `Drop`, `notify` callbacks). If you must use
//! `std::sync::Mutex`, document why in the field's doc comment.

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

    /// Join handle for the plugin hot-reload background task. Aborted on
    /// [`AppState::drop`] to stop the loop gracefully (M-5).
    pub plugin_hot_reload_task: Option<tokio::task::JoinHandle<()>>,

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

    /// Kernel shutdown signal. Send `()` in [`Drop`] before attempting the
    /// kernel lock retry loop (M-2). Long-running kernel operations can
    /// listen on the corresponding receiver to abort early on shutdown.
    pub kernel_shutdown: Option<tokio::sync::oneshot::Sender<()>>,

    /// In-memory session store mapping opaque session tokens to resolved
    /// [`SessionContext`] values. ADR #4 / ADR #7.
    ///
    /// Tokens are randomly-generated UUIDs created during login/session
    /// resolution. Commands look up their context via [`AppState::resolve_session`].
    pub session_store: Arc<RwLock<HashMap<String, SessionContext>>>,

    /// Session TTL in seconds. Read from `session.ttl_seconds` setting
    /// at startup; defaults to 86400 (24 hours). Set to 0 to disable
    /// session expiry (development mode).
    ///
    /// Used by `create_session` to stamp `expires_at` on new sessions,
    /// and by `resolve_session` / `prune_expired_sessions` to reject
    /// stale tokens.
    pub session_ttl_seconds: i64,

    /// Terminal identifier for multi-terminal deployments.
    ///
    /// Set once at startup from the registered terminal matching this
    /// device's hostname. Commands that auto-register a terminal via
    /// `set_feature(MultiTerminal, true)` update this field directly
    /// instead of mutating the process env (which is UB from async
    /// tokio workers). Consumers (Redis pub/sub subscriber, inventory
    /// change publisher) read this field.
    pub terminal_id: Arc<Mutex<Option<String>>>,
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
        let redis_url = oz_core::Settings::get_redis_url(&conn).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "failed to read redis_url setting, falling back to localhost");
            "redis://127.0.0.1/".into()
        });
        let cache_ttl = oz_core::Settings::get_redis_cache_ttl(&conn).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "failed to read cache_ttl setting, falling back to 300s");
            300u64
        });
        let cache = platform_startup::init_cache(&redis_url, cache_ttl);

        // ── Session TTL ──────────────────────────────────────────────
        // Read from settings; default 24h. 0 or missing = no expiry.
        // MUST be read before `conn` is moved into `Arc::new(Mutex::new(conn))`.
        let session_ttl_seconds: i64 = oz_core::Settings::get(&conn, "session.ttl_seconds")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(86400);

        // ── OZ_TERMINAL_ID for multi-terminal support ───────────────
        // On subsequent launches where MultiTerminal is already enabled,
        // look up the registered terminal by hostname. The terminal_id is
        // stored in AppState (typed field) instead of the process env var.
        // The Redis pub/sub subscriber and inventory change publisher read
        // it from this field — they no longer call std::env::var().
        let terminal_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let reg = oz_core::Settings::load_features(&conn).unwrap_or_default();
        if reg.is_enabled(oz_core::Feature::MultiTerminal) {
            let device_id = std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_default();
            if !device_id.is_empty() {
                let store = oz_core::db::Store::new(&conn);
                if let Ok(Some(terminal)) = store.get_terminal_by_device_id(&device_id) {
                    *terminal_id.blocking_lock() = Some(terminal.id.clone());
                    tracing::info!(
                        terminal_id = %terminal.id,
                        device_id = %device_id,
                        "terminal_id set at startup for multi-terminal"
                    );
                }
            }
        }

        // ── Start inventory pub/sub listener (Redis only) ────────────
        let pubsub_terminal_id = terminal_id.blocking_lock().clone();
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

        // Start plugin hot-reload file watcher (M-5).
        let (plugin_watcher, plugin_hot_reload_task) = if let Some(dir) = plugins_dir.as_ref() {
            if dir.exists() {
                start_plugin_watcher(plugins.clone(), dir.clone())
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // ── Kernel shutdown channel (M-2) ────────────────────────────
        // The receiver is dropped here — the infrastructure is in place for
        // future long-running kernel operations to listen for shutdown.
        let (kernel_shutdown_tx, _kernel_shutdown_rx) = tokio::sync::oneshot::channel::<()>();

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
            plugin_hot_reload_task,
            sync_daemon: SyncDaemon::new(),
            cache,
            inventory_pubsub_shutdown,
            kernel_shutdown: Some(kernel_shutdown_tx),
            session_store: Arc::new(RwLock::new(HashMap::new())),
            session_ttl_seconds,
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
    /// Create a `Store` with the shared cache layer and terminal
    /// identity for pub/sub message tagging.
    ///
    /// Command handlers should use this instead of `Store::new(&conn)`
    /// to benefit from Redis caching (when configured) and to ensure
    /// inventory-change pub/sub messages are correctly tagged with the
    /// terminal's identity.
    /// Create a `Store` with the shared cache layer and a pre-acquired
    /// terminal identity for pub/sub message tagging.
    ///
    /// Callers should acquire the terminal_id via
    /// `state.terminal_id.lock().await.clone()` BEFORE locking the
    /// database, so the db guard never crosses an await point.
    ///
    /// Command handlers should use this instead of `Store::new(&conn)`
    /// to benefit from Redis caching (when configured) and to ensure
    /// inventory-change pub/sub messages are correctly tagged with the
    /// terminal's identity.
    pub fn store_with_tid<'a>(
        &self,
        conn: &'a Connection,
        tid: Option<String>,
    ) -> oz_core::db::Store<'a> {
        oz_core::db::Store::with_cache(conn, self.cache.clone()).with_terminal_id(tid)
    }

    /// Resolve an opaque session token to its [`SessionContext`].
    ///
    /// ADR #4 / ADR #7: Commands call this to look up the caller's
    /// resolved scope (store, instance, type, user, role, terminal).
    /// Returns `AppError::InvalidSession` if the token is unknown
    /// OR if the session has expired (TTL check).
    ///
    /// Expired sessions are atomically removed from the store during
    /// resolution, so subsequent lookups also get `InvalidSession`.
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
    ///
    /// Called periodically by the background session-cleanup daemon
    /// (every 5 minutes). Also called lazily during `create_session`
    /// when the store is approaching capacity.
    pub fn prune_expired_sessions(&self) -> usize {
        let mut store = match self.session_store.write() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("session store lock poisoned during prune: {e}");
                return 0;
            }
        };
        let before = store.len();
        store.retain(|token, ctx| {
            if ctx.is_expired() {
                tracing::trace!(token = %token, "pruning expired session");
                false
            } else {
                true
            }
        });
        let pruned = before - store.len();
        if pruned > 0 {
            tracing::info!(
                "pruned {pruned} expired session(s), {remaining} remain",
                remaining = store.len()
            );
        }
        pruned
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
///
/// Returns a tuple of (file watcher, task join handle). The join handle
/// should be stored and aborted during [`Drop`] to stop the loop
/// gracefully (M-5).
fn start_plugin_watcher(
    plugins: Arc<Mutex<Option<PluginManager>>>,
    plugins_dir: PathBuf,
) -> (
    Option<notify::RecommendedWatcher>,
    Option<tokio::task::JoinHandle<()>>,
) {
    let reload_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = reload_flag.clone();

    let mut watcher = match notify::RecommendedWatcher::new(
        move |_res: Result<notify::Event, notify::Error>| {
            flag_clone.store(true, Ordering::SeqCst);
        },
        notify::Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!(error = %e, "failed to create plugin file watcher");
            return (None, None);
        }
    };

    if let Err(e) = watcher.watch(&plugins_dir, notify::RecursiveMode::Recursive) {
        tracing::warn!(error = %e, "failed to watch plugins directory");
        return (None, None);
    }

    tracing::info!(dir = %plugins_dir.display(), "plugin hot-reload watcher started");

    let handle = tokio::spawn(async move {
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

    (Some(watcher), Some(handle))
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
        // Abort the plugin hot-reload background task (M-5).
        if let Some(handle) = self.plugin_hot_reload_task.take() {
            handle.abort();
            tracing::info!("plugin hot-reload task cancelled");
        }

        // Signal kernel shutdown (M-2). This tells any kernel command
        // holding the lock to abort early, reducing contention during the
        // retry loop below. The receiver is a no-op for now — infrastructure
        // for future long-running kernel operations.
        if let Some(tx) = self.kernel_shutdown.take() {
            let _ = tx.send(());
            tracing::info!("kernel shutdown signal sent");
        }

        tracing::info!("stopping kernel modules");
        // Retry the lock for up to 2000ms (200 × 10ms) before giving up. This addresses
        // a Windows window-lifecycle bottleneck where `try_lock()` would
        // silently skip `stop_all()` if a Tauri command was mid-execution.
        // A single `try_lock()` is too aggressive during shutdown because
        // commands may still be draining. But `blocking_lock()` risks a
        // deadlock if a command holding the lock is waiting for the runtime
        // to shut down (circular dependency). The bounded retry loop
        // gives commands time to finish while guaranteeing the Drop
        // doesn't hang indefinitely. Increased from 50→200 for a more
        // generous shutdown window (M-2).
        const DROP_LOCK_RETRIES: usize = 200;
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
                "kernel lock contended after {}ms, skipping stop_all — \
                 modules may not have been stopped cleanly",
                DROP_LOCK_RETRIES * 10,
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
            plugin_hot_reload_task: None,
            sync_daemon: SyncDaemon::new(),
            cache: oz_core::cache::create_cache("redis://127.0.0.1/", 300),
            inventory_pubsub_shutdown: None,
            kernel_shutdown: None,
            session_store: Arc::new(RwLock::new(HashMap::new())),
            session_ttl_seconds: 86400,
            terminal_id: Arc::new(Mutex::new(None)),
        }
    }

    /// Construct a test state with a caller-provided global database connection.
    ///
    /// This keeps authorization tests independent from the production app
    /// bootstrap while preserving the same global-identity lookup path.
    pub fn for_test_with_conn(conn: Connection) -> Self {
        let mut state = Self::for_test();
        state.db = Arc::new(Mutex::new(conn));
        state
    }

    /// Construct a test state with an isolated store database directory.
    ///
    /// The production state owns this manager, so injecting it here lets
    /// scope tests prove that a session cannot observe another store's file.
    pub fn for_test_with_db_manager(db_manager: StoreDatabaseManager) -> Self {
        let mut state = Self::for_test();
        state.db_manager = db_manager;
        state
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
            None,
            0,
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
    fn resolve_session_rejects_and_removes_expired_token() {
        let state = AppState::for_test();
        let expired = SessionContext::new(
            "u-expired".into(),
            "r1".into(),
            "t1".into(),
            "store-expired".into(),
            "i1".into(),
            "pos".into(),
            Some(1),
            0,
        );
        state
            .session_store
            .write()
            .unwrap()
            .insert("expired-token".into(), expired);

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
        let temp_dir = tempfile::tempdir().unwrap();
        let manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
        let state = AppState::for_test_with_db_manager(manager);
        for (token, store_id) in [("token-a", "store-a"), ("token-b", "store-b")] {
            state.session_store.write().unwrap().insert(
                token.into(),
                SessionContext::new(
                    "user-1".into(),
                    "role-owner".into(),
                    "terminal-1".into(),
                    store_id.into(),
                    "instance-1".into(),
                    "pos".into(),
                    None,
                    0,
                ),
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
            None,
            0,
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
            None,
            0,
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
    async fn store_with_tid_creates_store_with_cache() {
        let state = AppState::for_test();
        let tid = state.terminal_id.lock().await.clone();
        let conn = state.db.lock().await;
        let store = state.store_with_tid(&conn, tid);
        let _ = store;
    }

    #[test]
    fn for_test_creates_valid_state() {
        let state = AppState::for_test();
        assert_eq!(state.db_path.to_str(), Some(":memory:"));
        assert!(state.app.is_none());
        assert!(state.plugin_watcher.is_none());
        assert!(state.plugin_hot_reload_task.is_none());
    }
}

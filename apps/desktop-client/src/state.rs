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

use oz_core::cache::Cache;

use rusqlite::Connection;
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::{Mutex, oneshot};

use oz_core::{Cart, CartId, migrations};
use oz_hal::DriverRegistry;
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

    /// In-memory cart store shared across sales commands.
    /// TODO(oz-core): replace with a SQLite-backed `CartStore` so
    /// carts survive a restart.
    pub carts: Mutex<HashMap<CartId, Cart>>,

    /// Cancel-sender for the active barcode scanner background task.
    /// When `Some`, the scanner polling loop is running; dropping
    /// or signalling it stops the loop gracefully.
    pub scanner_cancel: Mutex<Option<oneshot::Sender<()>>>,

    /// Module system kernel. Manages module lifecycle (load → start → stop).
    /// Modules are registered in `lib.rs::run()` during setup.
    pub kernel: Mutex<Kernel>,

    /// Optional Lua scripting runtime for custom business rules.
    /// `None` when no `scripts/` directory exists or loading failed.
    /// Wrapped in a `Mutex` because `rlua::Lua` uses interior mutability
    /// and is not safe for concurrent access from multiple Tauri commands.
    pub lua: Mutex<Option<oz_lua::LuaRuntime>>,

    /// Background sync daemon. Started during app setup via
    /// [`SyncDaemon::start`](platform_sync::daemon::SyncDaemon::start).
    pub sync_daemon: SyncDaemon,

    /// Caching layer (Redis-backed when configured, no-op otherwise).
    /// Shared across all `Store` instances via `Arc`.
    pub cache: Arc<dyn Cache>,

    /// Shutdown sender for the inventory pub/sub background listener.
    /// Dropped on app shutdown to stop the listener thread gracefully.
    pub inventory_pubsub_shutdown: Option<std::sync::mpsc::Sender<()>>,
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
        // look up the registered terminal by hostname and set the env var
        // so the Redis pub/sub subscriber can filter its own messages.
        let reg = oz_core::Settings::load_features(&conn).unwrap_or_default();
        if reg.is_enabled(oz_core::Feature::MultiTerminal) {
            let device_id = std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_default();
            if !device_id.is_empty() {
                let store = oz_core::db::Store::new(&conn);
                if let Ok(Some(terminal)) = store.get_terminal_by_device_id(&device_id) {
                    // SAFETY: single-threaded startup, called once per process.
                    unsafe {
                        std::env::set_var("OZ_TERMINAL_ID", &terminal.id);
                    }
                    tracing::info!(
                        terminal_id = %terminal.id,
                        device_id = %device_id,
                        "OZ_TERMINAL_ID set at startup for multi-terminal"
                    );
                }
            }
        }

        // ── Start inventory pub/sub listener (Redis only) ────────────
        let inventory_pubsub_shutdown = cache.start_inventory_pubsub(cache.clone());
        if inventory_pubsub_shutdown.is_some() {
            tracing::info!("inventory pub/sub listener started");
        }

        let db = Arc::new(Mutex::new(conn));
        let registry = Arc::new(DriverRegistry::default());

        // Load Lua business rule scripts from <app_data_dir>/scripts/.
        let lua = (|| -> Option<oz_lua::LuaRuntime> {
            let scripts_dir = app.path().app_data_dir().ok()?.join("scripts");
            if !scripts_dir.exists() {
                return None;
            }
            match oz_lua::LuaRuntime::new() {
                Ok(runtime) => {
                    if let Err(e) = runtime.load_dir(&scripts_dir) {
                        tracing::warn!(error = %e, "loading Lua scripts");
                    }
                    Some(runtime)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "initialising Lua runtime");
                    None
                }
            }
        })();

        tracing::info!(
            cache_healthy = cache.is_healthy(),
            ?db_path,
            lua_loaded = lua.is_some(),
            "AppState initialised"
        );

        Ok(Self {
            db,
            registry,
            app: Some(app.clone()),
            db_path,
            carts: Mutex::new(HashMap::new()),
            scanner_cancel: Mutex::new(None),
            kernel: Mutex::new(Kernel::new()),
            lua: Mutex::new(lua),
            sync_daemon: SyncDaemon::new(),
            cache,
            inventory_pubsub_shutdown,
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
    /// Create a [`Store`] with the shared cache layer.
    ///
    /// Command handlers should use this instead of `Store::new(&conn)`
    /// to benefit from Redis caching (when configured).
    pub fn store<'a>(&self, conn: &'a Connection) -> oz_core::db::Store<'a> {
        oz_core::db::Store::with_cache(conn, self.cache.clone())
    }
}

fn resolve_db_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("resolving app data dir: {e}")))?;
    Ok(dir.join("oz-pos.db"))
}

#[cfg(test)]
impl AppState {
    /// Construct an `AppState` suitable for unit tests.
    /// Creates a lightweight Tauri app handle via `tauri::test::mock_builder`.
    pub fn for_test() -> Self {
        Self {
            db: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            registry: Arc::new(DriverRegistry::default()),
            app: None,
            db_path: ":memory:".into(),
            carts: Mutex::new(HashMap::new()),
            scanner_cancel: Mutex::new(None),
            kernel: Mutex::new(Kernel::new()),
            lua: Mutex::new(None),
            sync_daemon: SyncDaemon::new(),
            cache: oz_core::cache::create_cache("redis://127.0.0.1/", 300),
            inventory_pubsub_shutdown: None,
        }
    }

    /// Construct an `AppState` with a pre-configured connection (migrations
    /// already run). Used by integration tests that need a seeded database.
    pub fn for_test_with_conn(conn: Connection) -> Self {
        Self {
            db: Arc::new(Mutex::new(conn)),
            registry: Arc::new(DriverRegistry::default()),
            app: None,
            db_path: ":memory:".into(),
            carts: Mutex::new(HashMap::new()),
            scanner_cancel: Mutex::new(None),
            kernel: Mutex::new(Kernel::new()),
            lua: Mutex::new(None),
            sync_daemon: SyncDaemon::new(),
            cache: oz_core::cache::create_cache("redis://127.0.0.1/", 300),
            inventory_pubsub_shutdown: None,
        }
    }
}

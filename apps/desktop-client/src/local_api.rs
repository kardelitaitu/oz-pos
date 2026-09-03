//! Local REST API server — embeds the `oz-api` router on loopback so
//! merchants can run their own scripts against the register.
//!
//! Mirrors the `lan_server` precedent: settings-driven, default-off,
//! loopback-only. Settings keys (global DB, device-level):
//! `local_api.enabled` ("1"/"0"), `local_api.port` (default 3099),
//! `local_api.secret` (per-install random hex, generated on first
//! enable — signs API tokens and doubles as the operator admin key, so
//! it is on the settings secret deny-list and never leaves the backend
//! except via the dedicated mint/status commands).
//!
//! The server serves the PRIMARY STORE's database — resolved at start
//! via `store_profiles.is_primary` and opened as a dedicated WAL
//! connection to `store-{id}.sqlite` (the same file the scoped Tauri
//! commands read through `state.resolve_scope()`), so scripts see
//! exactly what the register UI shows. The `local_api.*` settings
//! themselves live on the GLOBAL DB (device-level). CORS is
//! fail-closed (empty allowlist): local scripts are curl/Python/Node,
//! not browser pages. `GET /api/openapi.json` serves
//! `oz_api::spec::local_spec()` — the shared contract with every
//! operation tagged `x-oz-scope: "both"`.

use std::path::PathBuf;
use std::sync::Arc;

use platform_core::StoreDatabaseManager;
use rusqlite::Connection;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};

/// Settings key: "1" enables the local API (default off).
pub const SETTINGS_ENABLED: &str = "local_api.enabled";
/// Settings key: TCP port (default [`DEFAULT_PORT`]).
pub const SETTINGS_PORT: &str = "local_api.port";
/// Settings key: per-install signing secret (hex, 32 bytes).
pub const SETTINGS_SECRET: &str = "local_api.secret";
/// Default listen port — matches `OZ_API_PORT` in the standalone crate.
pub const DEFAULT_PORT: u16 = 3099;
/// Default token lifetime for UI-minted keys (30 days).
pub const DEFAULT_TOKEN_HOURS: i64 = 720;
/// Upper bound for UI-minted token lifetime (1 year).
pub const MAX_TOKEN_HOURS: i64 = 8760;

/// A running local API server and the means to stop it.
pub struct LocalApiHandle {
    /// The port actually bound (the OS picks for port 0 in tests).
    pub port: u16,
    /// Base URL for scripts, e.g. `http://127.0.0.1:3099/api/v1`.
    pub base_url: String,
    shutdown: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

impl LocalApiHandle {
    /// Signal graceful shutdown and abort the task without waiting.
    /// Used by `AppState::drop` (sync context); prefer [`Self::stop_async`]
    /// when an await point is available — it guarantees the listener
    /// socket is released before returning, so an immediate re-bind of
    /// the same port cannot race the OS teardown.
    pub fn stop(self) {
        let _ = self.shutdown.send(());
        self.task.abort();
        tracing::info!(port = self.port, "local API server stopped");
    }

    /// Graceful shutdown awaited up to 2 s (abort fallback). Returns
    /// only after the serve task — and with it the listener — is gone.
    pub async fn stop_async(self) {
        let LocalApiHandle {
            port,
            shutdown,
            mut task,
            ..
        } = self;
        let _ = shutdown.send(());
        if tokio::time::timeout(std::time::Duration::from_secs(2), &mut task)
            .await
            .is_err()
        {
            task.abort();
            tracing::warn!(port, "local API graceful shutdown timed out, aborted");
        }
        tracing::info!(port, "local API server stopped");
    }
}

/// The primary store's id from the GLOBAL DB (`store_profiles`),
/// falling back to `'default'` when no row is flagged (fresh install
/// before `seed_primary_store` promotion, or a corrupted profile).
pub fn primary_store_id(global: &Connection) -> String {
    global
        .query_row(
            "SELECT id FROM store_profiles WHERE is_primary = 1 LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "default".to_string())
}

/// Open the dedicated API connection to a store's database.
///
/// Goes through `db_manager.open_store` first so the file exists and is
/// migrated (the manager caches its own std-Mutex connection — the UI's
/// path), then opens a SECOND connection to the same file wrapped in a
/// tokio Mutex, which is what `oz_api::AppState` requires. WAL mode
/// makes the two connections safe to share the file; `busy_timeout`
/// absorbs write contention between API and UI instead of surfacing
/// `SQLITE_BUSY` to scripts.
pub fn open_api_store_connection(
    db_manager: &StoreDatabaseManager,
    store_id: &str,
) -> Result<(Arc<Mutex<Connection>>, PathBuf), String> {
    db_manager
        .open_store(store_id)
        .map_err(|e| format!("preparing store db {store_id}: {e}"))?;
    let path = db_manager.store_db_path(store_id);
    let conn = Connection::open(&path).map_err(|e| format!("opening {}: {e}", path.display()))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("enabling FK on {}: {e}", path.display()))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("enabling WAL on {}: {e}", path.display()))?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| format!("setting busy_timeout on {}: {e}", path.display()))?;
    Ok((Arc::new(Mutex::new(conn)), path))
}

/// Read `local_api.enabled` from the settings table.
pub fn is_enabled(conn: &Connection) -> bool {
    oz_core::Settings::get(conn, SETTINGS_ENABLED)
        .unwrap_or(None)
        .as_deref()
        == Some("1")
}

/// Read and validate `local_api.port`; falls back to [`DEFAULT_PORT`].
pub fn resolve_port(conn: &Connection) -> u16 {
    oz_core::Settings::get(conn, SETTINGS_PORT)
        .unwrap_or(None)
        .and_then(|s| s.trim().parse::<u16>().ok())
        .filter(|p| (1024..=65535).contains(p))
        .unwrap_or(DEFAULT_PORT)
}

/// Load the per-install secret, generating and persisting one on first
/// use. Two UUID v7s (simple form) give 64 hex chars — ~148 random bits
/// plus timestamps; adequate for a loopback-only signing key, and a
/// pure-`rand` upgrade is tracked as a review follow-up.
pub fn load_or_create_secret(conn: &Connection) -> Result<String, String> {
    if let Some(existing) = oz_core::Settings::get(conn, SETTINGS_SECRET)
        .map_err(|e| format!("reading {SETTINGS_SECRET}: {e}"))?
        .filter(|s| !s.trim().is_empty())
    {
        return Ok(existing);
    }
    let secret = format!(
        "{}{}",
        uuid::Uuid::now_v7().simple(),
        uuid::Uuid::now_v7().simple()
    );
    oz_core::Settings::set(conn, SETTINGS_SECRET, &secret)
        .map_err(|e| format!("persisting {SETTINGS_SECRET}: {e}"))?;
    tracing::info!("local API: generated new per-install signing secret");
    Ok(secret)
}

/// Mint an API token signed with the local secret.
///
/// `expiry_hours` is clamped to `1..=MAX_TOKEN_HOURS`. The token carries
/// no `permissions` claim (legacy full-read) — on a loopback-only bind
/// the operator is the tenant. Master-data writes still additionally
/// require `X-Admin-Key: <secret>` (the operator tier, D1).
pub fn mint_token(
    secret: &str,
    label: &str,
    expiry_hours: Option<i64>,
) -> Result<oz_api::auth::TokenResponse, String> {
    let hours = expiry_hours
        .unwrap_or(DEFAULT_TOKEN_HOURS)
        .clamp(1, MAX_TOKEN_HOURS);
    let label = if label.trim().is_empty() {
        "local-script"
    } else {
        label.trim()
    };
    oz_api::auth::create_token_full(label, Some(hours), None, None, None, Some(secret))
        .map_err(|e| format!("minting local API token: {e}"))
}

/// Bind `127.0.0.1:port` and serve the `oz-api` router until the
/// returned handle is stopped.
///
/// `port` 0 lets the OS choose (tests); the actual port is reported on
/// the handle. Binding is done BEFORE spawning so a port conflict
/// returns `Err` to the caller instead of dying in a background task.
pub async fn start(
    db: Arc<Mutex<Connection>>,
    db_path: PathBuf,
    image_dir: PathBuf,
    secret: String,
    port: u16,
) -> Result<LocalApiHandle, String> {
    // Bind BEFORE building the router so the self-documenting
    // /api/openapi.json handler can advertise the actual port
    // (OS-chosen when 0).
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| format!("binding 127.0.0.1:{port}: {e}"))?;
    let bound_port = listener
        .local_addr()
        .map_err(|e| format!("reading bound address: {e}"))?
        .port();

    let api_state = oz_api::AppState {
        db,
        pg: None,
        // The per-install secret doubles as the operator admin key: token
        // minting over HTTP and master-data writes require X-Admin-Key.
        admin_key: Some(secret.clone()),
        api_secret: secret,
        db_path: db_path.display().to_string(),
        port: bound_port,
        // Fail-closed: no browser origin may call the local API. Local
        // scripts (curl/Python/Node) do not send CORS preflights.
        cors_origins: Vec::new(),
        image_dir,
    };
    // Mirror serve()'s contract: the content-addressed image store must
    // exist before the image routes can write to it.
    std::fs::create_dir_all(&api_state.image_dir)
        .map_err(|e| format!("creating image dir {}: {e}", api_state.image_dir.display()))?;
    // The shared OpenAPI document (every operation `x-oz-scope: "both"`)
    // served locally — scripts can discover the contract from the
    // running server instead of the repo docs.
    let app = oz_api::router(api_state).route(
        "/api/openapi.json",
        axum::routing::get(move || async move { axum::Json(oz_api::spec::local_spec(bound_port)) }),
    );

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let serve = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = serve.await {
            tracing::error!(error = %e, "local API server exited with error");
        }
    });

    let base_url = format!("http://127.0.0.1:{bound_port}/api/v1");
    tracing::info!(port = bound_port, "local API server listening on loopback");
    Ok(LocalApiHandle {
        port: bound_port,
        base_url,
        shutdown: shutdown_tx,
        task,
    })
}

/// Wire status for the Settings UI — never carries the secret itself.
/// IPC DTO convention: camelCase on the wire (see `OfflineQueueSummaryDto`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalApiStatus {
    /// Whether the setting is on (persisted intent).
    pub enabled: bool,
    /// Whether a server is currently listening.
    pub running: bool,
    /// The configured port (from settings, even when not running).
    pub port: u16,
    /// Base URL when running.
    pub base_url: Option<String>,
}

#[cfg(test)]
#[path = "local_api_tests.rs"]
mod tests;

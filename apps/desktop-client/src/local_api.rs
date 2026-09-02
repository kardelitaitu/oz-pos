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
//! The server shares `AppState::db` (the primary-store connection) with
//! the Tauri commands — same `Arc<tokio::sync::Mutex<Connection>>` type
//! `oz_api::AppState` expects. CORS is fail-closed (empty allowlist):
//! local scripts are curl/Python/Node, not browser pages.

use std::path::PathBuf;
use std::sync::Arc;

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
    /// Signal graceful shutdown and (belt-and-braces) abort the task.
    pub fn stop(self) {
        let _ = self.shutdown.send(());
        self.task.abort();
        tracing::info!(port = self.port, "local API server stopped");
    }
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
/// use. Two UUID v7s (simple form) give 32 bytes of randomness without
/// pulling another RNG dependency into this path.
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
    let api_state = oz_api::AppState {
        db,
        pg: None,
        // The per-install secret doubles as the operator admin key: token
        // minting over HTTP and master-data writes require X-Admin-Key.
        admin_key: Some(secret.clone()),
        api_secret: secret,
        db_path: db_path.display().to_string(),
        port,
        // Fail-closed: no browser origin may call the local API. Local
        // scripts (curl/Python/Node) do not send CORS preflights.
        cors_origins: Vec::new(),
        image_dir,
    };
    // Mirror serve()'s contract: the content-addressed image store must
    // exist before the image routes can write to it.
    std::fs::create_dir_all(&api_state.image_dir)
        .map_err(|e| format!("creating image dir {}: {e}", api_state.image_dir.display()))?;
    let app = oz_api::router(api_state);

    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| format!("binding 127.0.0.1:{port}: {e}"))?;
    let bound_port = listener
        .local_addr()
        .map_err(|e| format!("reading bound address: {e}"))?
        .port();

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

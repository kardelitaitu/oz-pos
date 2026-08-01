//! Cloud sync client — pushes pending offline queue items to a remote server.
//!
//! The sync client reads from the local offline queue, sends items as a batch
//! to the configured remote server via `POST /api/sync/push`, and marks each
//! item as synced or failed based on the server's per-item outcomes.
//!
//! Pull (`GET /api/sync/snapshot`) fetches the server's authoritative
//! reference data (products, tax rates, users) and upserts it locally.
//!
//! ## Runtime safety
//!
//! The public API (`ping_server`, `request_token`, `send_items_to_server`,
//! `fetch_snapshot_from_server`) is **async** using `reqwest::Client` so
//! Tauri v2 command handlers can call them with `.await` without nesting
//! Tokio runtimes. The legacy blocking helpers (`sync_pending`,
//! `send_items_to_server_blocking`) remain available only for
//! `tokio::task::spawn_blocking` or non-async contexts.

use serde::{Deserialize, Serialize};

use crate::db::Store;
use crate::error::CoreError;
use crate::offline::OfflineQueueItem;

/// Per-item outcome returned by the server's `POST /api/sync/push`.
///
/// Mirrors `platform_sync::transport::PushOutcome` without depending on that
/// crate (oz-core is a foundational crate).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PushOutcome {
    /// Item was accepted and applied by the server.
    Accepted,
    /// Item conflicted with the server version.
    Conflict(OfflineQueueItem),
    /// Item was rejected with a reason.
    Rejected {
        /// Human-readable rejection reason from the server.
        reason: String,
    },
}

/// Server response envelope for push.
#[derive(Debug, Clone, Deserialize)]
struct PushResponse {
    results: Vec<PushOutcome>,
}

/// Result of a single sync attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncAttemptResult {
    /// Number of items successfully synced.
    pub synced: usize,
    /// Number of items that failed to sync.
    pub failed: usize,
    /// Error message if the entire sync failed (e.g. network error).
    pub error: Option<String>,
}

/// Result of a `pull_snapshot` round-trip.
///
/// The three counts tell the UI how many rows landed in the local
/// cache for each domain (products, tax rates, users). `error` is
/// populated when the entire pull failed at the network or decode
/// stage — partial successes are surfaced as `Ok` with the per-domain
/// counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullResult {
    /// Number of products upserted from the server snapshot.
    pub products_pulled: usize,
    /// Number of tax rates upserted from the server snapshot.
    pub tax_rates_pulled: usize,
    /// Number of users upserted from the server snapshot.
    pub users_pulled: usize,
    /// Error message if the entire pull failed (e.g. network error).
    pub error: Option<String>,
}

/// Result of a health-check ping to the cloud server.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingResult {
    /// Whether the server responded successfully.
    pub ok: bool,
    /// Status text (e.g. "Connected", "Connection refused", etc.).
    pub status: String,
    /// Round-trip latency in milliseconds, if the ping succeeded.
    pub latency_ms: Option<u64>,
}

/// Format an ISO-8601 expiry timestamp as a human-readable relative duration.
///
/// Returns strings like "in 2 hours", "in 3 days", "in 5 minutes", or
/// the raw timestamp if parsing fails.
#[cfg(feature = "sync-http")]
fn format_expiry(iso: &str) -> String {
    // Try RFC 3339 first (the most common ISO-8601 variant from APIs).
    let expiry = match chrono::DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => dt,
        Err(_) => return format!("expires {iso}"),
    };
    let now = chrono::Utc::now();
    let dur = expiry.signed_duration_since(now);

    if dur.num_seconds() <= 0 {
        return "expired".into();
    }

    let mins = dur.num_minutes();
    let hours = dur.num_hours();
    let days = dur.num_days();

    if days >= 2 {
        format!("expires in {days} days")
    } else if days == 1 {
        "expires in 1 day".into()
    } else if hours >= 2 {
        format!("expires in {hours} hours")
    } else if hours == 1 {
        "expires in 1 hour".into()
    } else if mins >= 2 {
        format!("expires in {mins} minutes")
    } else if mins == 1 {
        "expires in 1 minute".into()
    } else {
        "expires in less than a minute".into()
    }
}

/// Result of requesting a new API token from the cloud server.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResult {
    /// Whether the token was successfully obtained.
    pub ok: bool,
    /// The JWT token string (only present on success).
    pub token: Option<String>,
    /// Human-readable status or error message.
    pub status: String,
    /// Token expiry in ISO-8601 format, if the server returned one.
    pub expires_at: Option<String>,
}

/// Request a new JWT API token from the cloud server's
/// `POST /api/v1/tokens` endpoint (async — safe to call from
/// Tauri async command handlers).
#[cfg(feature = "sync-http")]
pub async fn request_token(url: &str) -> TokenResult {
    let token_url = format!("{}/api/v1/tokens", url.trim_end_matches('/'));
    let body = serde_json::json!({"label": "pos-terminal"});

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return TokenResult {
                ok: false,
                token: None,
                status: format!("Failed to build HTTP client: {e}"),
                expires_at: None,
            };
        }
    };

    match client
        .post(&token_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                #[derive(Deserialize)]
                struct TokenPayload {
                    token: String,
                    #[serde(default)]
                    expires_at: Option<String>,
                }
                #[derive(Deserialize)]
                struct TokenResponse {
                    token: TokenPayload,
                }
                match resp.json::<TokenResponse>().await {
                    Ok(tr) => {
                        let expires = tr.token.expires_at.clone();
                        TokenResult {
                            ok: true,
                            status: expires
                                .as_ref()
                                .map(|e| format!("Token obtained — {}", format_expiry(e)))
                                .unwrap_or_else(|| "Token obtained".into()),
                            token: Some(tr.token.token),
                            expires_at: tr.token.expires_at,
                        }
                    }
                    Err(e) => TokenResult {
                        ok: false,
                        token: None,
                        status: format!("Failed to parse token response: {e}"),
                        expires_at: None,
                    },
                }
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                TokenResult {
                    ok: false,
                    token: None,
                    status: format!("Server returned {status}: {body}"),
                    expires_at: None,
                }
            }
        }
        Err(e) => TokenResult {
            ok: false,
            token: None,
            status: format!("Request failed: {e}"),
            expires_at: None,
        },
    }
}

/// Stub when sync-http is disabled.
#[cfg(not(feature = "sync-http"))]
pub async fn request_token(_url: &str) -> TokenResult {
    TokenResult {
        ok: false,
        token: None,
        status: "sync-http feature is disabled".into(),
        expires_at: None,
    }
}

/// Ping the cloud server's `/health` endpoint to verify connectivity
/// (async — safe to call from Tauri async command handlers).
#[cfg(feature = "sync-http")]
pub async fn ping_server(url: &str) -> PingResult {
    let health_url = format!("{}/health", url.trim_end_matches('/'));
    let start = std::time::Instant::now();
    match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(client) => match client.get(&health_url).send().await {
            Ok(resp) => {
                let latency = start.elapsed().as_millis() as u64;
                if resp.status().is_success() {
                    PingResult {
                        ok: true,
                        status: format!("Connected ({latency}ms)"),
                        latency_ms: Some(latency),
                    }
                } else {
                    let status = resp.status();
                    PingResult {
                        ok: false,
                        status: format!("Server returned {status}"),
                        latency_ms: Some(latency),
                    }
                }
            }
            Err(e) => PingResult {
                ok: false,
                status: format!("Connection failed: {e}"),
                latency_ms: None,
            },
        },
        Err(e) => PingResult {
            ok: false,
            status: format!("Failed to build HTTP client: {e}"),
            latency_ms: None,
        },
    }
}

/// Stub when sync-http is disabled.
#[cfg(not(feature = "sync-http"))]
pub async fn ping_server(_url: &str) -> PingResult {
    PingResult {
        ok: false,
        status: "sync-http feature is disabled".into(),
        latency_ms: None,
    }
}

/// Sync client configuration.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Remote server base URL (e.g. "http://localhost:3099").
    pub server_url: String,
    /// API key for authentication (sent as `Authorization: Bearer {key}`).
    /// This should be a JWT token generated by the cloud server's
    /// `POST /api/v1/tokens` endpoint.
    pub api_key: Option<String>,
}

impl SyncConfig {
    /// Load sync configuration from settings.
    pub fn from_settings(store: &Store) -> Result<Option<Self>, CoreError> {
        let enabled = crate::settings::Settings::is_sync_enabled(store.conn())?;
        if !enabled {
            return Ok(None);
        }
        let server_url = crate::settings::Settings::get_sync_server_url(store.conn())?;
        let server_url = match server_url {
            Some(u) if !u.is_empty() => u,
            _ => return Ok(None),
        };
        let api_key =
            crate::settings::Settings::get_sync_api_key(store.conn())?.filter(|k| !k.is_empty());
        Ok(Some(Self {
            server_url,
            api_key,
        }))
    }
}

/// Apply per-item sync outcomes to the offline queue (mark items as
/// synced or failed). This is the DB-only post-processing phase that
/// runs after the async HTTP call completes, so no Store reference
/// is held during the network round-trip.
pub fn apply_sync_outcomes(
    store: &Store,
    pending: &[OfflineQueueItem],
    outcomes: &[PushOutcome],
) -> Result<SyncAttemptResult, CoreError> {
    let mut synced = 0usize;
    let mut failed = 0usize;
    let mut global_error: Option<String> = None;

    for (item, outcome) in pending.iter().zip(outcomes.iter()) {
        match outcome {
            PushOutcome::Accepted => {
                store.mark_offline_synced(&item.id)?;
                synced += 1;
            }
            PushOutcome::Rejected { reason } => {
                store.mark_offline_failed(&item.id, reason)?;
                failed += 1;
                global_error = Some(reason.clone());
            }
            PushOutcome::Conflict(server_item) => {
                tracing::warn!(
                    item_id = %item.id,
                    server_action = %server_item.action,
                    "sync conflict: item already exists on server with different data"
                );
                let msg = "server conflict: item already exists with different data";
                store.mark_offline_failed(&item.id, msg)?;
                failed += 1;
                global_error = Some(msg.into());
            }
        }
    }

    Ok(SyncAttemptResult {
        synced,
        failed,
        error: global_error,
    })
}

/// Mark all pending items as failed with the given error message.
pub fn mark_all_failed(
    store: &Store,
    pending: &[OfflineQueueItem],
    err_msg: &str,
) -> Result<SyncAttemptResult, CoreError> {
    for item in pending {
        store.mark_offline_failed(&item.id, err_msg)?;
    }
    Ok(SyncAttemptResult {
        synced: 0,
        failed: pending.len(),
        error: Some(err_msg.into()),
    })
}

/// Attempt to sync all pending offline items to the remote server.
///
/// Uses blocking HTTP — only safe when called from a non-async context
/// or inside `tokio::task::spawn_blocking`. For async Tauri commands,
/// prefer the split read/HTTP/write pattern using `send_items_to_server`
/// (async) + `apply_sync_outcomes`.
pub fn sync_pending(store: &Store, config: &SyncConfig) -> Result<SyncAttemptResult, CoreError> {
    let pending = store.list_pending_offline()?;
    if pending.is_empty() {
        return Ok(SyncAttemptResult {
            synced: 0,
            failed: 0,
            error: None,
        });
    }

    // This still uses reqwest::blocking — only safe from spawn_blocking or
    // non-async contexts. The Tauri commands use the split async path instead.
    match send_items_to_server_blocking(config, &pending) {
        Ok(outcomes) => apply_sync_outcomes(store, &pending, &outcomes),
        Err(e) => mark_all_failed(store, &pending, &e.to_string()),
    }
}

/// Blocking variant of send_items_to_server — only for spawn_blocking contexts.
#[cfg(feature = "sync-http")]
fn send_items_to_server_blocking(
    config: &SyncConfig,
    items: &[OfflineQueueItem],
) -> Result<Vec<PushOutcome>, Box<dyn std::error::Error>> {
    let url = format!("{}/api/sync/push", config.server_url.trim_end_matches('/'));

    let mut request = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?
        .post(&url)
        .header("Content-Type", "application/json");

    if let Some(ref key) = config.api_key {
        request = request.header("Authorization", &format!("Bearer {key}"));
    }

    let resp = request
        .json(items)
        .send()
        .map_err(|e| format!("sync HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("sync server returned {status}: {body}").into());
    }

    let push_resp: PushResponse = resp
        .json()
        .map_err(|e| format!("sync response parse failed: {e}"))?;

    tracing::info!(
        item_count = items.len(),
        server = %config.server_url,
        "synced batch to server"
    );
    Ok(push_resp.results)
}

#[cfg(not(feature = "sync-http"))]
fn send_items_to_server_blocking(
    config: &SyncConfig,
    items: &[OfflineQueueItem],
) -> Result<Vec<PushOutcome>, Box<dyn std::error::Error>> {
    tracing::info!(
        item_count = items.len(),
        server = %config.server_url,
        "sync-http feature disabled; would sync batch to server"
    );
    Ok(vec![PushOutcome::Accepted; items.len()])
}

/// Send a batch of offline queue items to the remote server via
/// `POST /api/sync/push` and return per-item outcomes (async).
#[cfg(feature = "sync-http")]
pub async fn send_items_to_server(
    config: &SyncConfig,
    items: &[OfflineQueueItem],
) -> Result<Vec<PushOutcome>, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/api/sync/push", config.server_url.trim_end_matches('/'));

    let mut request = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?
        .post(&url)
        .header("Content-Type", "application/json");

    if let Some(ref key) = config.api_key {
        request = request.header("Authorization", &format!("Bearer {key}"));
    }

    let resp = request
        .json(items)
        .send()
        .await
        .map_err(|e| format!("sync HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("sync server returned {status}: {body}").into());
    }

    let push_resp: PushResponse = resp
        .json()
        .await
        .map_err(|e| format!("sync response parse failed: {e}"))?;

    tracing::info!(
        item_count = items.len(),
        server = %config.server_url,
        "synced batch to server"
    );
    Ok(push_resp.results)
}

/// Stub used when `sync-http` feature is disabled — just logs the intent.
#[cfg(not(feature = "sync-http"))]
pub async fn send_items_to_server(
    config: &SyncConfig,
    items: &[OfflineQueueItem],
) -> Result<Vec<PushOutcome>, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(
        item_count = items.len(),
        server = %config.server_url,
        "sync-http feature disabled; would sync batch to server"
    );
    // Pretend all items were accepted when HTTP is compiled out.
    Ok(vec![PushOutcome::Accepted; items.len()])
}

// ── Pull (snapshot import) ───────────────────────────────────────────
//
// `pull_snapshot` fetches the server's authoritative copy of the
// reference data (products, tax rates, users) and upserts it into the
// local DB inside a single transaction. Used by the `sync_pull`
// Tauri command when the user clicks "Pull from server" in the Sync
// tab — they want the server to be the new source of truth, and the
// local cache to match.

/// Server snapshot envelope. The server is expected to return the
/// flat column-shape for each row (matching the `products` / `tax_rates`
/// / `users` tables in the migrations) so the client can upsert
/// directly without remapping.
#[derive(Debug, Default, Deserialize)]
pub struct Snapshot {
    /// Products to upsert, keyed by `sku`.
    #[serde(default)]
    products: Vec<SnapshotProduct>,
    /// Tax rates to upsert, keyed by `id`.
    #[serde(default)]
    tax_rates: Vec<SnapshotTaxRate>,
    /// Users to upsert, keyed by `username`.
    #[serde(default)]
    users: Vec<SnapshotUser>,
}

/// Flat product row matching the `products` table columns.
#[derive(Debug, Deserialize)]
struct SnapshotProduct {
    /// Internal row id (UUID v4). If absent, a fresh UUID is generated.
    id: Option<String>,
    /// Stock-keeping unit — UNIQUE column used for the upsert conflict target.
    sku: String,
    /// Display name.
    name: String,
    /// Price in minor units (e.g. cents).
    price_minor: i64,
    /// ISO-4217 currency code.
    currency: String,
    /// Optional category FK.
    category_id: Option<String>,
    /// Optional machine-readable barcode.
    barcode: Option<String>,
    /// ISO-8601 creation timestamp; `None` lets the DB default fill it.
    created_at: Option<String>,
    /// ISO-8601 last-update timestamp; defaults to `now()` on insert.
    updated_at: Option<String>,
    /// ISO-8601 last price-change timestamp; defaults to `now()`.
    price_updated_at: Option<String>,
    /// Whether the product requires serial-number capture at checkout.
    #[serde(default)]
    track_serial: bool,
}

/// Flat tax-rate row matching the `tax_rates` table columns.
#[derive(Debug, Deserialize)]
struct SnapshotTaxRate {
    /// Internal row id (UUID v4) — used as the upsert conflict target.
    id: String,
    /// Display name.
    name: String,
    /// Rate in basis points (1 bps = 0.01 %).
    rate_bps: i64,
    /// Whether this is the default tax rate for the store.
    #[serde(default)]
    is_default: bool,
    /// Whether tax is included in the displayed price.
    #[serde(default)]
    is_inclusive: bool,
    /// ISO-8601 creation timestamp.
    created_at: Option<String>,
    /// ISO-8601 last-update timestamp.
    updated_at: Option<String>,
}

/// Placeholder written into `users.pin_hash` for snapshot-imported users.
///
/// SYNC-06: the snapshot contract deliberately carries NO credential
/// material, so `upsert_users` cannot write a real verifier. This sentinel
/// can never match a bcrypt/argon2 verification, so a snapshot-imported
/// user cannot authenticate until a local administrator provisions their
/// PIN through the normal identity-management flow.
///
/// Shared with `platform-sync`'s `import_snapshot` so the sentinel lives
/// in exactly one place.
pub const SNAPSHOT_PIN_HASH_PLACEHOLDER: &str = "!snapshot-no-credential!";

/// Flat user row matching the `users` table columns (minus secrets).
///
/// SYNC-06: `pin_hash` is intentionally absent from the snapshot
/// contract — a sync token with snapshot access must never receive
/// credential-verifier material for tenant users. `deny_unknown_fields`
/// makes the client fail loudly if a (buggy/older) server ever sends a
/// `pin_hash` field instead of silently importing it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotUser {
    /// Internal row id (UUID v4).
    id: Option<String>,
    /// Login username — UNIQUE column used for the upsert conflict target.
    username: String,
    /// Display name shown on the POS UI.
    display_name: String,
    /// FK to `roles.id`.
    role_id: String,
    /// Whether this user can log in.
    #[serde(default = "default_true")]
    is_active: bool,
    /// ISO-8601 creation timestamp.
    created_at: Option<String>,
    /// ISO-8601 last-update timestamp.
    updated_at: Option<String>,
}

/// Default `true` for `is_active` so a missing field means "user is active".
fn default_true() -> bool {
    true
}

/// Fetch a snapshot from the server via `GET /api/sync/snapshot` (async).
#[cfg(feature = "sync-http")]
pub async fn fetch_snapshot_from_server(
    config: &SyncConfig,
) -> Result<Snapshot, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "{}/api/sync/snapshot",
        config.server_url.trim_end_matches('/')
    );
    let mut request = reqwest::Client::new()
        .get(&url)
        .header("Accept", "application/json");

    if let Some(ref key) = config.api_key {
        request = request.header("Authorization", &format!("Bearer {key}"));
    }

    let resp = request
        .send()
        .await
        .map_err(|e| format!("snapshot HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("snapshot server returned {status}: {body}").into());
    }

    let snapshot: Snapshot = resp
        .json()
        .await
        .map_err(|e| format!("snapshot JSON decode failed: {e}"))?;

    Ok(snapshot)
}

/// Stub used when `sync-http` feature is disabled.
#[cfg(not(feature = "sync-http"))]
pub async fn fetch_snapshot_from_server(
    _config: &SyncConfig,
) -> Result<Snapshot, Box<dyn std::error::Error + Send + Sync>> {
    Err("sync-http feature is disabled; cannot pull snapshot from server".into())
}

/// Apply a fetched snapshot to the local database inside a single
/// transaction. This is the DB-only phase that runs after the async
/// `fetch_snapshot_from_server` call completes.
pub fn apply_snapshot(store: &Store, snapshot: &Snapshot) -> Result<PullResult, CoreError> {
    let tx = store.conn.unchecked_transaction()?;

    let products_pulled = upsert_products(&tx, &snapshot.products)?;
    let tax_rates_pulled = upsert_tax_rates(&tx, &snapshot.tax_rates)?;
    let users_pulled = upsert_users(&tx, &snapshot.users)?;

    tx.commit()?;

    tracing::info!(
        products = products_pulled,
        tax_rates = tax_rates_pulled,
        users = users_pulled,
        "applied server snapshot to local db"
    );

    Ok(PullResult {
        products_pulled,
        tax_rates_pulled,
        users_pulled,
        error: None,
    })
}

fn upsert_products(
    tx: &rusqlite::Transaction<'_>,
    rows: &[SnapshotProduct],
) -> Result<usize, CoreError> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut count = 0usize;
    let mut stmt = tx.prepare(
        "INSERT INTO products (id, sku, name, price_minor, currency,
                               category_id, barcode, created_at, updated_at,
                               price_updated_at, track_serial)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                 COALESCE(?8, ?11), COALESCE(?9, ?11), COALESCE(?10, ?11), ?12)
         ON CONFLICT(sku) DO UPDATE SET
             name            = excluded.name,
             price_minor     = excluded.price_minor,
             currency        = excluded.currency,
             category_id     = excluded.category_id,
             barcode         = excluded.barcode,
             updated_at      = COALESCE(excluded.updated_at, ?11),
             price_updated_at = COALESCE(excluded.price_updated_at, ?11),
             track_serial    = excluded.track_serial",
    )?;
    for p in rows {
        let id =
            p.id.clone()
                .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        stmt.execute(rusqlite::params![
            id,
            p.sku,
            p.name,
            p.price_minor,
            p.currency,
            p.category_id,
            p.barcode,
            p.created_at,
            p.updated_at,
            p.price_updated_at,
            now,
            p.track_serial as i64,
        ])?;
        count += 1;
    }
    stmt.finalize()?;
    Ok(count)
}

fn upsert_tax_rates(
    tx: &rusqlite::Transaction<'_>,
    rows: &[SnapshotTaxRate],
) -> Result<usize, CoreError> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut count = 0usize;
    let mut stmt = tx.prepare(
        "INSERT INTO tax_rates (id, name, rate_bps, is_default, is_inclusive,
                                created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5,
                 COALESCE(?6, ?8), COALESCE(?7, ?8))
         ON CONFLICT(id) DO UPDATE SET
             name         = excluded.name,
             rate_bps     = excluded.rate_bps,
             is_default   = excluded.is_default,
             is_inclusive = excluded.is_inclusive,
             updated_at   = COALESCE(excluded.updated_at, ?8)",
    )?;
    for r in rows {
        stmt.execute(rusqlite::params![
            r.id,
            r.name,
            r.rate_bps,
            r.is_default as i64,
            r.is_inclusive as i64,
            r.created_at,
            r.updated_at,
            now,
        ])?;
        count += 1;
    }
    stmt.finalize()?;
    Ok(count)
}

fn upsert_users(tx: &rusqlite::Transaction<'_>, rows: &[SnapshotUser]) -> Result<usize, CoreError> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut count = 0usize;
    // SYNC-06: `pin_hash` is never taken from the snapshot. New rows get a
    // non-verifiable placeholder, and on conflict the EXISTING local hash
    // is preserved (the UPDATE clause deliberately omits `pin_hash`) — a
    // snapshot pull can neither replicate credentials nor lock out an
    // operator who already has a working PIN.
    let mut stmt = tx.prepare(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id,
                            is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6,
                 COALESCE(?7, ?9), COALESCE(?8, ?9))
         ON CONFLICT(username) DO UPDATE SET
             display_name = excluded.display_name,
             role_id      = excluded.role_id,
             is_active    = excluded.is_active,
             updated_at   = COALESCE(excluded.updated_at, ?9)",
    )?;
    for u in rows {
        let id =
            u.id.clone()
                .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        stmt.execute(rusqlite::params![
            id,                            // ?1
            u.username,                    // ?2
            SNAPSHOT_PIN_HASH_PLACEHOLDER, // ?3 — never a real verifier
            u.display_name,                // ?4
            u.role_id,                     // ?5
            u.is_active as i64,            // ?6
            u.created_at,                  // ?7
            u.updated_at,                  // ?8
            now,                           // ?9 — default for created_at / updated_at
        ])?;
        count += 1;
    }
    stmt.finalize()?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use crate::settings::Settings;
    use rusqlite::Connection;

    fn setup() -> Store<'static> {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        let conn: &'static Connection = Box::leak(Box::new(conn));
        Store::new(conn)
    }

    #[test]
    fn sync_pending_empty_queue() {
        let store = setup();
        let config = SyncConfig {
            server_url: "http://localhost:3099".into(),
            api_key: None,
        };
        let result = sync_pending(&store, &config).unwrap();
        assert_eq!(result.synced, 0);
        assert_eq!(result.failed, 0);
        assert!(result.error.is_none());
    }

    #[test]
    fn sync_config_from_settings_disabled() {
        let store = setup();
        let config = SyncConfig::from_settings(&store).unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn sync_pending_marks_items_synced() {
        let store = setup();
        let _item = store
            .enqueue_offline("complete_sale", r#"{"test": true}"#)
            .unwrap();

        let config = SyncConfig {
            server_url: "http://localhost:3099".into(),
            api_key: None,
        };
        // No server running locally — sync should fail with a transport error.
        let result = sync_pending(&store, &config).unwrap();
        assert_eq!(result.synced, 0);
        assert_eq!(result.failed, 1);
        assert!(result.error.is_some(), "should report a network error");

        // Item should be marked as failed (no longer pending).
        let pending = store.list_pending_offline().unwrap();
        assert!(pending.is_empty(), "failed item is no longer pending");
        let all = store.list_all_offline().unwrap();
        assert_eq!(all.len(), 1, "item still in queue with failed status");
        assert_eq!(all[0].status, crate::offline::OfflineQueueStatus::Failed);
    }

    #[test]
    fn sync_pending_multiple_items() {
        let store = setup();
        store
            .enqueue_offline("complete_sale", r#"{"id":1}"#)
            .unwrap();
        store
            .enqueue_offline("complete_sale", r#"{"id":2}"#)
            .unwrap();

        let config = SyncConfig {
            server_url: "http://localhost:3099".into(),
            api_key: None,
        };
        let result = sync_pending(&store, &config).unwrap();
        // No server running — all items fail.
        assert_eq!(result.synced, 0);
        assert_eq!(result.failed, 2);
        assert!(result.error.is_some(), "should report a network error");
    }

    #[test]
    fn sync_config_from_settings_enabled_with_url() {
        let store = setup();
        let conn = store.conn();
        Settings::set_sync_enabled(conn, true).unwrap();
        Settings::set_sync_server_url(conn, "http://sync.example.com").unwrap();

        let config = SyncConfig::from_settings(&store).unwrap();
        assert!(config.is_some());
        assert_eq!(config.unwrap().server_url, "http://sync.example.com");
    }

    #[test]
    fn sync_config_from_settings_enabled_no_url() {
        let store = setup();
        let conn = store.conn();
        Settings::set_sync_enabled(conn, true).unwrap();
        // Don't set a URL
        let config = SyncConfig::from_settings(&store).unwrap();
        assert!(config.is_none(), "should be None when no URL is set");
    }

    #[test]
    fn sync_config_from_settings_enabled_empty_url() {
        let store = setup();
        let conn = store.conn();
        Settings::set_sync_enabled(conn, true).unwrap();
        Settings::set_sync_server_url(conn, "").unwrap();

        let config = SyncConfig::from_settings(&store).unwrap();
        assert!(config.is_none(), "should be None when URL is empty");
    }

    #[test]
    fn sync_config_from_settings_with_api_key() {
        let store = setup();
        let conn = store.conn();
        Settings::set_sync_enabled(conn, true).unwrap();
        Settings::set_sync_server_url(conn, "http://sync.example.com").unwrap();
        Settings::set_sync_api_key(conn, "sk-test-key").unwrap();

        let config = SyncConfig::from_settings(&store).unwrap().unwrap();
        assert_eq!(config.server_url, "http://sync.example.com");
        assert_eq!(config.api_key, Some("sk-test-key".into()));
    }

    // ── SYNC-04: per-outcome application contract ───────────────
    //
    // `retry_offline_sync` and `sync_run` both delegate here. These tests
    // pin that an item is marked synced ONLY on an accepted outcome, and
    // marked failed (never falsely synced) on rejection or conflict.

    #[test]
    fn apply_sync_outcomes_accepted_marks_synced() {
        let store = setup();
        let items = [
            store
                .enqueue_offline("complete_sale", r#"{"id":1}"#)
                .unwrap(),
            store.enqueue_offline("void_sale", r#"{"id":2}"#).unwrap(),
        ];

        let outcomes = vec![PushOutcome::Accepted, PushOutcome::Accepted];
        let result = apply_sync_outcomes(&store, &items, &outcomes).unwrap();
        assert_eq!(result.synced, 2);
        assert_eq!(result.failed, 0);
        assert!(result.error.is_none());

        let all = store.list_all_offline().unwrap();
        assert!(
            all.iter()
                .all(|i| i.status == crate::offline::OfflineQueueStatus::Synced)
        );
    }

    #[test]
    fn apply_sync_outcomes_rejected_marks_failed() {
        let store = setup();
        let items = [store
            .enqueue_offline("complete_sale", r#"{"id":1}"#)
            .unwrap()];

        let outcomes = vec![PushOutcome::Rejected {
            reason: "invalid action".into(),
        }];
        let result = apply_sync_outcomes(&store, &items, &outcomes).unwrap();
        assert_eq!(result.synced, 0);
        assert_eq!(result.failed, 1);
        assert_eq!(result.error.as_deref(), Some("invalid action"));

        let all = store.list_all_offline().unwrap();
        assert_eq!(all[0].status, crate::offline::OfflineQueueStatus::Failed);
        assert_eq!(all[0].last_error.as_deref(), Some("invalid action"));
    }

    #[test]
    fn apply_sync_outcomes_conflict_marks_failed_never_synced() {
        let store = setup();
        let local = store
            .enqueue_offline("complete_sale", r#"{"id":1}"#)
            .unwrap();
        let items = [local.clone()];

        // A conflict outcome carries the server's copy of the item.
        let server_item = OfflineQueueItem {
            id: local.id.clone(),
            action: local.action.clone(),
            payload: r#"{"id":1,"remote":true}"#.into(),
            status: local.status,
            retry_count: local.retry_count,
            last_error: None,
            tenant_id: local.tenant_id.clone(),
            created_at: local.created_at.clone(),
            synced_at: None,
            priority: local.priority,
        };
        let outcomes = vec![PushOutcome::Conflict(server_item)];
        let result = apply_sync_outcomes(&store, &items, &outcomes).unwrap();
        assert_eq!(result.synced, 0);
        assert_eq!(result.failed, 1);

        // The local item must be marked failed — never silently synced.
        let all = store.list_all_offline().unwrap();
        assert_eq!(all[0].status, crate::offline::OfflineQueueStatus::Failed);
        assert!(
            all[0]
                .last_error
                .as_deref()
                .unwrap_or_default()
                .contains("conflict"),
            "conflict failure message must explain the conflict"
        );
    }

    #[test]
    fn apply_sync_outcomes_truncates_on_outcome_len_mismatch() {
        // Documented behaviour: if the server returns fewer outcomes than
        // pending items, `zip` silently truncates. The unpaired items are
        // neither marked synced nor failed (they stay pending) — the
        // retry caller must re-list them next cycle. This pins the
        // current contract so a future refactor can't silently mark them
        // synced without an outcome.
        let store = setup();
        let items = [
            store
                .enqueue_offline("complete_sale", r#"{"id":1}"#)
                .unwrap(),
            store
                .enqueue_offline("complete_sale", r#"{"id":2}"#)
                .unwrap(),
        ];
        let outcomes = vec![PushOutcome::Accepted]; // one outcome for two items
        let result = apply_sync_outcomes(&store, &items, &outcomes).unwrap();
        assert_eq!(result.synced, 1);
        assert_eq!(result.failed, 0);

        let all = store.list_all_offline().unwrap();
        // One synced, one still pending — never falsely synced.
        assert_eq!(
            all.iter()
                .filter(|i| i.status == crate::offline::OfflineQueueStatus::Synced)
                .count(),
            1
        );
        assert_eq!(
            all.iter()
                .filter(|i| i.status == crate::offline::OfflineQueueStatus::Pending)
                .count(),
            1
        );
    }

    // ── SYNC-06: snapshot credential-exposure contract ──────────
    //
    // The snapshot must NEVER carry `pin_hash`. These tests pin both
    // directions: (1) the client rejects a snapshot that (incorrectly)
    // includes the field, and (2) applying a valid snapshot writes a
    // non-verifiable placeholder for new users while preserving any
    // existing local credential hash on conflict.

    #[test]
    fn snapshot_user_without_pin_hash_deserializes() {
        // A snapshot user row with NO pin_hash field is the normal
        // contract and must deserialize cleanly.
        let json = r#"{
            "users": [{
                "id": "u1",
                "username": "alice",
                "display_name": "Alice",
                "role_id": "r-owner",
                "is_active": true
            }]
        }"#;
        let snap: Snapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snap.users.len(), 1);
        assert_eq!(snap.users[0].username, "alice");
    }

    #[test]
    fn snapshot_user_with_pin_hash_is_rejected() {
        // Defense in depth: a snapshot that (incorrectly) carries pin_hash
        // must fail loudly instead of silently importing credential
        // material into the local users table.
        let json = r#"{
            "users": [{
                "id": "u1",
                "username": "alice",
                "pin_hash": "SENSITIVE-HASH",
                "display_name": "Alice",
                "role_id": "r-owner",
                "is_active": true
            }]
        }"#;
        assert!(
            serde_json::from_str::<Snapshot>(json).is_err(),
            "snapshot with pin_hash must be rejected"
        );
    }

    #[test]
    fn apply_snapshot_writes_placeholder_pin_hash_for_new_users() {
        let store = setup();
        // Seed a role so the users FK is satisfied.
        store
            .conn()
            .execute(
                "INSERT INTO roles (id, name, permissions) VALUES ('r-owner', 'Owner', '[]')",
                [],
            )
            .unwrap();

        let snap = Snapshot {
            products: vec![],
            tax_rates: vec![],
            users: vec![SnapshotUser {
                id: Some("u1".into()),
                username: "alice".into(),
                display_name: "Alice".into(),
                role_id: "r-owner".into(),
                is_active: true,
                created_at: None,
                updated_at: None,
            }],
        };
        let result = apply_snapshot(&store, &snap).unwrap();
        assert_eq!(result.users_pulled, 1);

        let hash: String = store
            .conn()
            .query_row(
                "SELECT pin_hash FROM users WHERE username = 'alice'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hash, SNAPSHOT_PIN_HASH_PLACEHOLDER);
        assert_ne!(hash, "SENSITIVE-HASH", "never a real verifier");
    }

    #[test]
    fn apply_snapshot_preserves_existing_local_pin_hash_on_conflict() {
        let store = setup();
        store
            .conn()
            .execute(
                "INSERT INTO roles (id, name, permissions) VALUES ('r-owner', 'Owner', '[]')",
                [],
            )
            .unwrap();
        // Pre-existing local user with a REAL credential hash.
        store
            .conn()
            .execute(
                "INSERT INTO users (id, username, pin_hash, display_name, role_id)
                 VALUES ('u-local', 'bob', 'REAL-LOCAL-HASH', 'Bob', 'r-owner')",
                [],
            )
            .unwrap();

        // Snapshot upserts the same username with a fresh remote id.
        let snap = Snapshot {
            products: vec![],
            tax_rates: vec![],
            users: vec![SnapshotUser {
                id: Some("u-remote".into()),
                username: "bob".into(),
                display_name: "Bob Updated".into(),
                role_id: "r-owner".into(),
                is_active: true,
                created_at: None,
                updated_at: None,
            }],
        };
        apply_snapshot(&store, &snap).unwrap();

        // The conflict-update must NOT clobber the local credential hash.
        let hash: String = store
            .conn()
            .query_row(
                "SELECT pin_hash FROM users WHERE username = 'bob'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hash, "REAL-LOCAL-HASH");

        // ...but the non-secret metadata from the snapshot still lands.
        let name: String = store
            .conn()
            .query_row(
                "SELECT display_name FROM users WHERE username = 'bob'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "Bob Updated");
    }

    #[test]
    fn sync_attempt_result_debug() {
        let result = SyncAttemptResult {
            synced: 5,
            failed: 1,
            error: Some("network error".into()),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("synced: 5"));
        assert!(debug.contains("failed: 1"));
    }

    #[test]
    fn sync_attempt_result_serde_roundtrip() {
        let result = SyncAttemptResult {
            synced: 10,
            failed: 2,
            error: Some("timeout".into()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: SyncAttemptResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.synced, 10);
        assert_eq!(back.failed, 2);
        assert_eq!(back.error, Some("timeout".into()));
    }

    #[test]
    fn sync_attempt_result_no_error() {
        let result = SyncAttemptResult {
            synced: 0,
            failed: 0,
            error: None,
        };
        assert!(result.error.is_none());
    }

    // ── format_expiry tests ────────────────────────────────────

    #[cfg(feature = "sync-http")]
    #[test]
    fn format_expiry_exactly_one_hour() {
        // Small buffer (+5s) accounts for sub-second drift between the
        // timestamp construction and format_expiry's internal now() call.
        let ts = (chrono::Utc::now() + chrono::Duration::hours(1) + chrono::Duration::seconds(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        assert_eq!(format_expiry(&ts), "expires in 1 hour");
    }

    #[cfg(feature = "sync-http")]
    #[test]
    fn format_expiry_exactly_one_day() {
        let ts = (chrono::Utc::now() + chrono::Duration::days(1) + chrono::Duration::seconds(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        assert_eq!(format_expiry(&ts), "expires in 1 day");
    }

    #[cfg(feature = "sync-http")]
    #[test]
    fn format_expiry_already_expired() {
        let ts = (chrono::Utc::now() - chrono::Duration::hours(1))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        assert_eq!(format_expiry(&ts), "expired");
    }

    #[cfg(feature = "sync-http")]
    #[test]
    fn format_expiry_less_than_a_minute() {
        let ts = (chrono::Utc::now() + chrono::Duration::seconds(30))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        assert_eq!(format_expiry(&ts), "expires in less than a minute");
    }

    #[cfg(feature = "sync-http")]
    #[test]
    fn format_expiry_ninety_minutes() {
        // Small buffer (+5s) prevents sub-second drift from pushing the
        // duration below 60 minutes (which would display as "59 minutes").
        let ts =
            (chrono::Utc::now() + chrono::Duration::minutes(90) + chrono::Duration::seconds(5))
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        assert_eq!(format_expiry(&ts), "expires in 1 hour");
    }

    #[cfg(feature = "sync-http")]
    #[test]
    fn format_expiry_twenty_five_hours() {
        let ts = (chrono::Utc::now() + chrono::Duration::hours(25) + chrono::Duration::seconds(5))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        assert_eq!(format_expiry(&ts), "expires in 1 day");
    }

    #[cfg(feature = "sync-http")]
    #[test]
    fn format_expiry_unparseable_fallback() {
        assert_eq!(format_expiry("not-a-timestamp"), "expires not-a-timestamp");
    }
}

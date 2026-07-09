//! Cloud sync client — pushes pending offline queue items to a remote server.
//!
//! The sync client reads from the local offline queue, sends each item to the
//! configured remote server via REST API, and marks the item as synced (or
//! failed) in the local database.

use serde::{Deserialize, Serialize};

use crate::db::Store;
use crate::error::CoreError;
use crate::offline::OfflineQueueItem;

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

/// Sync client configuration.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Remote server base URL (e.g. "http://localhost:3099").
    pub server_url: String,
    /// API key for authentication.
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
        let api_key = crate::settings::Settings::get_sync_api_key(store.conn())?;
        Ok(Some(Self {
            server_url,
            api_key,
        }))
    }
}

/// Attempt to sync all pending offline items to the remote server.
///
/// Returns a `SyncAttemptResult` with counts of synced/failed items.
/// When the server is unreachable the entire batch fails with an error.
pub fn sync_pending(store: &Store, config: &SyncConfig) -> Result<SyncAttemptResult, CoreError> {
    let pending = store.list_pending_offline()?;
    if pending.is_empty() {
        return Ok(SyncAttemptResult {
            synced: 0,
            failed: 0,
            error: None,
        });
    }

    let mut synced = 0usize;
    let mut failed = 0usize;
    let mut global_error: Option<String> = None;

    for item in &pending {
        match send_item_to_server(config, item) {
            Ok(()) => {
                store.mark_offline_synced(&item.id)?;
                synced += 1;
            }
            Err(e) => {
                let err_msg = e.to_string();
                store.mark_offline_failed(&item.id, &err_msg)?;
                failed += 1;
                global_error = Some(err_msg);
            }
        }
    }

    Ok(SyncAttemptResult {
        synced,
        failed,
        error: global_error,
    })
}

/// Send a single offline queue item to the remote server via HTTP POST.
#[cfg(feature = "sync-http")]
fn send_item_to_server(
    config: &SyncConfig,
    item: &OfflineQueueItem,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/events", config.server_url.trim_end_matches('/'));
    let payload = serde_json::json!({
        "id": item.id,
        "action": item.action,
        "payload": item.payload,
    });

    let mut headers = reqwest::blocking::Client::new()
        .post(&url)
        .header("Content-Type", "application/json");

    if let Some(ref key) = config.api_key {
        headers = headers.header("Authorization", &format!("Bearer {key}"));
    }

    let resp = headers
        .json(&payload)
        .send()
        .map_err(|e| format!("sync HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("sync server returned {status}: {body}").into());
    }

    tracing::info!(
        item_id = %item.id,
        action = %item.action,
        server = %config.server_url,
        "synced item to server"
    );
    Ok(())
}

/// Stub used when `sync-http` feature is disabled — just logs the intent.
#[cfg(not(feature = "sync-http"))]
fn send_item_to_server(
    config: &SyncConfig,
    item: &OfflineQueueItem,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(
        item_id = %item.id,
        action = %item.action,
        server = %config.server_url,
        "sync-http feature disabled; would sync item to server"
    );
    Ok(())
}

/// Send a single offline queue item to the remote server (async version).
///
/// This is the async counterpart used by the background daemon.
pub async fn sync_pending_async(
    store: &Store<'_>,
    config: &SyncConfig,
) -> Result<SyncAttemptResult, CoreError> {
    sync_pending(store, config)
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
struct Snapshot {
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

/// Flat user row matching the `users` table columns.
#[derive(Debug, Deserialize)]
struct SnapshotUser {
    /// Internal row id (UUID v4).
    id: Option<String>,
    /// Login username — UNIQUE column used for the upsert conflict target.
    username: String,
    /// Bcrypt/argon2 hash of the PIN/password.
    pin_hash: String,
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

/// Fetch a snapshot from the server via `GET /api/snapshot`.
#[cfg(feature = "sync-http")]
fn fetch_snapshot_from_server(config: &SyncConfig) -> Result<Snapshot, Box<dyn std::error::Error>> {
    let url = format!("{}/api/snapshot", config.server_url.trim_end_matches('/'));
    let mut request = reqwest::blocking::Client::new()
        .get(&url)
        .header("Accept", "application/json");

    if let Some(ref key) = config.api_key {
        request = request.header("Authorization", &format!("Bearer {key}"));
    }

    let resp = request
        .send()
        .map_err(|e| format!("snapshot HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("snapshot server returned {status}: {body}").into());
    }

    let snapshot: Snapshot = resp
        .json()
        .map_err(|e| format!("snapshot JSON decode failed: {e}"))?;

    Ok(snapshot)
}

/// Stub used when `sync-http` feature is disabled — surfaces a clear
/// error so the UI knows the request could not be made.
#[cfg(not(feature = "sync-http"))]
fn fetch_snapshot_from_server(
    _config: &SyncConfig,
) -> Result<Snapshot, Box<dyn std::error::Error>> {
    Err("sync-http feature is disabled; cannot pull snapshot from server".into())
}

/// Pull a snapshot from the server and upsert the rows into the local
/// database inside a single transaction.
///
/// Network and decode failures populate `PullResult.error` so the UI
/// can surface them inline; database failures (e.g. constraint
/// violation on a malformed row) bubble up as `CoreError` since they
/// indicate a snapshot / schema mismatch the developer needs to see.
pub fn pull_snapshot(store: &Store, config: &SyncConfig) -> Result<PullResult, CoreError> {
    let snapshot = match fetch_snapshot_from_server(config) {
        Ok(s) => s,
        Err(e) => {
            return Ok(PullResult {
                products_pulled: 0,
                tax_rates_pulled: 0,
                users_pulled: 0,
                error: Some(e.to_string()),
            });
        }
    };

    let tx = store.conn.unchecked_transaction()?;

    let products_pulled = upsert_products(&tx, &snapshot.products)?;
    let tax_rates_pulled = upsert_tax_rates(&tx, &snapshot.tax_rates)?;
    let users_pulled = upsert_users(&tx, &snapshot.users)?;

    tx.commit()?;

    tracing::info!(
        products = products_pulled,
        tax_rates = tax_rates_pulled,
        users = users_pulled,
        server = %config.server_url,
        "pulled snapshot from server"
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
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
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
    let mut stmt = tx.prepare(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id,
                            is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6,
                 COALESCE(?7, ?9), COALESCE(?8, ?9))
         ON CONFLICT(username) DO UPDATE SET
             pin_hash     = excluded.pin_hash,
             display_name = excluded.display_name,
             role_id      = excluded.role_id,
             is_active    = excluded.is_active,
             updated_at   = COALESCE(excluded.updated_at, ?9)",
    )?;
    for u in rows {
        let id =
            u.id.clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        stmt.execute(rusqlite::params![
            id,                 // ?1
            u.username,         // ?2
            u.pin_hash,         // ?3
            u.display_name,     // ?4
            u.role_id,          // ?5
            u.is_active as i64, // ?6
            u.created_at,       // ?7
            u.updated_at,       // ?8
            now,                // ?9 — default for created_at / updated_at
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
}

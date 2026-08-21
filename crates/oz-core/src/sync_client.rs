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

use rusqlite::Connection;
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
    /// The server rejected the attempt because this tenant is on the
    /// `free` plan (ADR sync-plan-gating). The UI shows an upgrade prompt
    /// and queued items stay `pending` — they are valid, just gated.
    #[serde(default)]
    pub plan_required: bool,
}

/// Typed HTTP error from the sync client (ADR sync-auth-hardening P1/P4).
///
/// 401 responses are split so callers can refresh the stored token and retry
/// exactly once when it EXPIRED, while treating a genuinely invalid key as a
/// configuration problem that must not be masked by a refresh.
#[derive(Debug, thiserror::Error)]
pub enum SyncHttpError {
    /// The server said the token expired (HTTP 401 + `token_expired`, or a
    /// bare 401 from an older server). Safe to refresh the API key and
    /// retry once.
    #[error("sync server rejected authentication: token expired (HTTP 401)")]
    AuthExpired,

    /// The server said the token is invalid or missing (HTTP 401 +
    /// `invalid_token` / `missing_token`). A configuration problem — do NOT
    /// refresh; surface the error.
    #[error("sync server rejected authentication: invalid token (HTTP 401)")]
    AuthInvalid,

    /// The tenant is on the `free` plan and cloud sync is gated
    /// (HTTP 403 + `plan_required`, ADR sync-plan-gating). Terminal: do
    /// NOT refresh, retry, or quarantine — surface the upgrade prompt.
    #[error("cloud sync requires a paid plan (HTTP 403 plan_required)")]
    PlanRequired,

    /// The server returned a non-2xx status other than 401.
    #[error("sync server returned {status}: {body}")]
    Server {
        /// HTTP status code.
        status: u16,
        /// Response body for diagnostics.
        body: String,
    },

    /// The request failed at the network layer (connect, timeout, DNS).
    #[error("sync request failed: {0}")]
    Network(String),

    /// The response could not be parsed.
    #[error("sync response parse failed: {0}")]
    Parse(String),

    /// The HTTP client could not be constructed.
    #[error("failed to build HTTP client: {0}")]
    Client(String),
}

/// Classify a 401 response body (ADR sync-auth-hardening P4).
///
/// Servers with structured errors say `token_expired` / `invalid_token` /
/// `missing_token`. A bare 401 (older server) is treated as stale auth so
/// the refresh-and-retry behaviour from P1 keeps working.
fn classify_401(body: &str) -> SyncHttpError {
    if body.contains("token_expired") {
        SyncHttpError::AuthExpired
    } else if body.contains("invalid_token") || body.contains("missing_token") {
        SyncHttpError::AuthInvalid
    } else {
        SyncHttpError::AuthExpired
    }
}

/// Classify a non-2xx HTTP status into a typed [`SyncHttpError`]
/// (ADR sync-auth-hardening P4 + ADR sync-plan-gating).
///
/// Used by both `send_items_to_server` and `fetch_snapshot_from_server` so
/// the push and pull paths agree on 401/403 semantics:
///
/// - `401` → `AuthExpired` / `AuthInvalid` (refresh only on expiry).
/// - `403` + `plan_required` → `PlanRequired` (terminal — no refresh,
///   no retry, no quarantine).
/// - anything else → `Server { status, body }`.
fn classify_http_status(status: u16, body: &str) -> SyncHttpError {
    if status == reqwest::StatusCode::UNAUTHORIZED.as_u16() {
        classify_401(body)
    } else if status == reqwest::StatusCode::FORBIDDEN.as_u16() && body.contains("plan_required") {
        SyncHttpError::PlanRequired
    } else {
        SyncHttpError::Server {
            status,
            body: body.to_owned(),
        }
    }
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

/// Result of reading the caller's own sync plan (ADR sync-plan-gating).
///
/// The plan string is `free` | `pro`, or `None` when the read failed or the
/// server is unreachable — the UI falls back to showing nothing rather than
/// guessing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantPlanResult {
    /// Whether the server responded successfully.
    pub ok: bool,
    /// Effective plan (`free` | `pro`), when the read succeeded.
    pub plan: Option<String>,
    /// Human-readable status or error message.
    pub status: String,
}

/// Read the caller's own sync plan from `GET /api/v1/tenants/me/plan`.
///
/// Uses the stored API key (JWT) so the server resolves the tenant from the
/// token claims. Unlike the sync endpoints this route is NOT plan-gated, so a
/// free tenant can read its own plan to render the upgrade prompt.
#[cfg(feature = "sync-http")]
pub async fn fetch_tenant_plan(url: &str, api_key: &str) -> TenantPlanResult {
    let plan_url = format!("{}/api/v1/tenants/me/plan", url.trim_end_matches('/'));

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return TenantPlanResult {
                ok: false,
                plan: None,
                status: format!("Failed to build HTTP client: {e}"),
            };
        }
    };

    match client
        .get(&plan_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                #[derive(Deserialize)]
                struct PlanPayload {
                    plan: String,
                }
                match resp.json::<PlanPayload>().await {
                    Ok(payload) => TenantPlanResult {
                        ok: true,
                        plan: Some(payload.plan),
                        status: "ok".into(),
                    },
                    Err(e) => TenantPlanResult {
                        ok: false,
                        plan: None,
                        status: format!("Failed to parse plan response: {e}"),
                    },
                }
            } else {
                TenantPlanResult {
                    ok: false,
                    plan: None,
                    status: format!("Server returned {}", resp.status()),
                }
            }
        }
        Err(e) => TenantPlanResult {
            ok: false,
            plan: None,
            status: format!("Connection failed: {e}"),
        },
    }
}

/// Stub when sync-http is disabled.
#[cfg(not(feature = "sync-http"))]
pub async fn fetch_tenant_plan(_url: &str, _api_key: &str) -> TenantPlanResult {
    TenantPlanResult {
        ok: false,
        plan: None,
        status: "sync-http feature is disabled".into(),
    }
}

/// Read the admin key that gates token minting (ADR sync-auth-hardening P2).
///
/// Comes from the `OZ_ADMIN_KEY` environment variable; the client sends it as
/// `X-Admin-Key` so auto-provisioning and refresh keep working against a
/// gated server. Returns `None` on dev machines without the variable.
pub fn admin_key_from_env() -> Option<String> {
    std::env::var("OZ_ADMIN_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty())
}

/// Result of registering a sync terminal (ADR sync-auth-hardening P3).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRegistrationResult {
    /// Whether registration succeeded.
    pub ok: bool,
    /// Terminal identifier (present on success).
    pub terminal_id: Option<String>,
    /// Plaintext device secret (present on success — shown once).
    pub device_secret: Option<String>,
    /// Human-readable status or error message.
    pub status: String,
}

/// Register this terminal with the sync server (ADR sync-auth-hardening P3).
///
/// Posts to `POST /api/v1/terminals` with the optional `X-Admin-Key` header.
/// Returns the plaintext device secret exactly once; the server only keeps
/// its SHA-256 hash.
#[cfg(feature = "sync-http")]
pub async fn register_terminal(
    url: &str,
    admin_key: Option<&str>,
    terminal_id: &str,
    label: &str,
) -> TerminalRegistrationResult {
    let register_url = format!("{}/api/v1/terminals", url.trim_end_matches('/'));
    let body = serde_json::json!({
        "terminal_id": terminal_id,
        "label": label,
    });

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return TerminalRegistrationResult {
                ok: false,
                terminal_id: None,
                device_secret: None,
                status: format!("Failed to build HTTP client: {e}"),
            };
        }
    };

    let mut request = client
        .post(&register_url)
        .header("Content-Type", "application/json");
    if let Some(key) = admin_key {
        request = request.header("X-Admin-Key", key);
    }

    match request.json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            #[derive(Deserialize)]
            struct RegisterPayload {
                terminal_id: String,
                device_secret: String,
            }
            match resp.json::<RegisterPayload>().await {
                Ok(payload) => TerminalRegistrationResult {
                    ok: true,
                    terminal_id: Some(payload.terminal_id),
                    device_secret: Some(payload.device_secret),
                    status: "registered".into(),
                },
                Err(e) => TerminalRegistrationResult {
                    ok: false,
                    terminal_id: None,
                    device_secret: None,
                    status: format!("Failed to parse registration response: {e}"),
                },
            }
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            TerminalRegistrationResult {
                ok: false,
                terminal_id: None,
                device_secret: None,
                status: format!("Server returned {status}: {body}"),
            }
        }
        Err(e) => TerminalRegistrationResult {
            ok: false,
            terminal_id: None,
            device_secret: None,
            status: format!("Request failed: {e}"),
        },
    }
}

/// Stub when sync-http is disabled.
#[cfg(not(feature = "sync-http"))]
pub async fn register_terminal(
    _url: &str,
    _admin_key: Option<&str>,
    _terminal_id: &str,
    _label: &str,
) -> TerminalRegistrationResult {
    TerminalRegistrationResult {
        ok: false,
        terminal_id: None,
        device_secret: None,
        status: "sync-http feature is disabled".into(),
    }
}

/// Request a token using terminal client credentials (ADR sync-auth-hardening
/// P3) — the client-credentials path, no admin key required.
#[cfg(feature = "sync-http")]
pub async fn request_token_client_credentials(
    url: &str,
    client_id: &str,
    client_secret: &str,
) -> TokenResult {
    let token_url = format!("{}/api/v1/tokens", url.trim_end_matches('/'));
    let body = serde_json::json!({
        "label": "pos-terminal",
        "client_id": client_id,
        "client_secret": client_secret,
    });

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
        Ok(resp) if resp.status().is_success() => {
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
                Ok(tr) => TokenResult {
                    ok: true,
                    status: "Token obtained".into(),
                    token: Some(tr.token.token),
                    expires_at: tr.token.expires_at,
                },
                Err(e) => TokenResult {
                    ok: false,
                    token: None,
                    status: format!("Failed to parse token response: {e}"),
                    expires_at: None,
                },
            }
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            TokenResult {
                ok: false,
                token: None,
                status: format!("Server returned {status}: {body}"),
                expires_at: None,
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
pub async fn request_token_client_credentials(
    _url: &str,
    _client_id: &str,
    _client_secret: &str,
) -> TokenResult {
    TokenResult {
        ok: false,
        token: None,
        status: "sync-http feature is disabled".into(),
        expires_at: None,
    }
}

/// Mint a token using the strongest available authentication:
/// terminal client credentials first, then the admin key env var, then an
/// open (label-only) mint for dev servers.
pub async fn mint_token(server_url: &str, client_credentials: Option<(&str, &str)>) -> TokenResult {
    if let Some((client_id, client_secret)) = client_credentials {
        return request_token_client_credentials(server_url, client_id, client_secret).await;
    }
    let admin_key = admin_key_from_env();
    request_token(server_url, admin_key.as_deref()).await
}

/// Request a new JWT API token from the cloud server's
/// `POST /api/v1/tokens` endpoint (async — safe to call from
/// Tauri async command handlers).
///
/// `admin_key` is sent as `X-Admin-Key` when present (ADR sync-auth-hardening
/// P2); servers configured with `OZ_ADMIN_KEY` reject minting without it.
#[cfg(feature = "sync-http")]
pub async fn request_token(url: &str, admin_key: Option<&str>) -> TokenResult {
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

    let mut request = client
        .post(&token_url)
        .header("Content-Type", "application/json");
    if let Some(key) = admin_key {
        request = request.header("X-Admin-Key", key);
    }

    match request.json(&body).send().await {
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
pub async fn request_token(_url: &str, _admin_key: Option<&str>) -> TokenResult {
    TokenResult {
        ok: false,
        token: None,
        status: "sync-http feature is disabled".into(),
        expires_at: None,
    }
}

/// Request a fresh token from the server (ADR sync-auth-hardening P1).
///
/// Async-only — performs no DB work, so callers can run it before taking
/// the DB lock (the same three-phase split the sync commands use). Returns
/// the new key on success, or `None` when the server refused to mint one.
pub async fn request_refresh_token(
    server_url: &str,
    client_credentials: Option<(&str, &str)>,
) -> Option<String> {
    let token = mint_token(server_url, client_credentials).await;
    if !token.ok {
        tracing::warn!(
            status = %token.status,
            "token refresh failed — sync stays on the stored key"
        );
        return None;
    }
    token.token
}

/// Persist a freshly requested API key (ADR sync-auth-hardening P1).
///
/// Synchronous write; callers hold the DB lock only for this call so the
/// guard never crosses an await point and Tauri command futures stay `Send`.
pub fn persist_refreshed_api_key(conn: &Connection, key: &str) -> Result<(), CoreError> {
    crate::settings::Settings::set_sync_api_key(conn, key)?;
    tracing::info!("refreshed sync API key after auth rejection");
    Ok(())
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
                // OFF-11: the server already holds this queued action, so the
                // server's copy wins. Record it as a *resolved* conflict (via
                // `mark_offline_resolved`) rather than a bare failure — this is
                // the marker the `offline_queue_status_summary` conflict_count
                // query counts (`last_error LIKE 'resolved: conflict%'`), so the
                // UI's conflict observability reflects real command-boundary
                // conflicts instead of always reading zero.
                tracing::warn!(
                    item_id = %item.id,
                    server_action = %server_item.action,
                    "sync conflict: item already exists on server with different data; server copy wins"
                );
                let resolution = format!(
                    "server item wins (action={} already on server)",
                    server_item.action
                );
                store.mark_offline_resolved(&item.id, &resolution)?;
                synced += 1;
            }
        }
    }

    Ok(SyncAttemptResult {
        synced,
        failed,
        error: global_error,
        plan_required: false,
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
        plan_required: false,
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
            plan_required: false,
        });
    }

    // This still uses reqwest::blocking — only safe from spawn_blocking or
    // non-async contexts. The Tauri commands use the split async path instead.
    match send_items_to_server_blocking(config, &pending) {
        Ok(outcomes) => apply_sync_outcomes(store, &pending, &outcomes),
        // ADR sync-plan-gating: a free tenant is gated, not broken. Do NOT
        // mark the items failed — they stay `pending` and sync automatically
        // once the tenant upgrades.
        Err(SyncHttpError::PlanRequired) => Ok(SyncAttemptResult {
            synced: 0,
            failed: 0,
            error: Some("cloud sync requires a paid plan".into()),
            plan_required: true,
        }),
        Err(e) => mark_all_failed(store, &pending, &e.to_string()),
    }
}

/// Blocking variant of send_items_to_server — only for spawn_blocking contexts.
#[cfg(feature = "sync-http")]
fn send_items_to_server_blocking(
    config: &SyncConfig,
    items: &[OfflineQueueItem],
) -> Result<Vec<PushOutcome>, SyncHttpError> {
    let url = format!("{}/api/sync/push", config.server_url.trim_end_matches('/'));

    let mut request = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| SyncHttpError::Client(format!("failed to build HTTP client: {e}")))?
        .post(&url)
        .header("Content-Type", "application/json");

    if let Some(ref key) = config.api_key {
        request = request.header("Authorization", &format!("Bearer {key}"));
    }

    let resp = request
        .json(items)
        .send()
        .map_err(|e| SyncHttpError::Network(format!("sync HTTP request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(classify_http_status(status.as_u16(), &body));
    }

    let push_resp: PushResponse = resp
        .json()
        .map_err(|e| SyncHttpError::Parse(format!("sync response parse failed: {e}")))?;

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
) -> Result<Vec<PushOutcome>, SyncHttpError> {
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
) -> Result<Vec<PushOutcome>, SyncHttpError> {
    let url = format!("{}/api/sync/push", config.server_url.trim_end_matches('/'));

    let mut request = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| SyncHttpError::Client(e.to_string()))?
        .post(&url)
        .header("Content-Type", "application/json");

    if let Some(ref key) = config.api_key {
        request = request.header("Authorization", &format!("Bearer {key}"));
    }

    let resp = request
        .json(items)
        .send()
        .await
        .map_err(|e| SyncHttpError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        // Read the body once; `text()` consumes the response.
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(classify_http_status(status.as_u16(), &body));
    }

    let push_resp: PushResponse = resp
        .json()
        .await
        .map_err(|e| SyncHttpError::Parse(e.to_string()))?;

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
) -> Result<Vec<PushOutcome>, SyncHttpError> {
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
    /// Store scoping for the soft-scoping layer (migration 069/117).
    ///
    /// `None`/absent means the shared global catalog; `Some(id)` means the
    /// row is visible only to that store. Backward compatible: servers that
    /// omit the field deserialize as `None`, so every pulled row lands in
    /// the global catalog exactly as before.
    #[serde(default)]
    store_id: Option<String>,
    /// Product brand (free text, synced — ADR #36 D2).
    #[serde(default)]
    brand: Option<String>,
    /// Rack position code (synced).
    #[serde(default)]
    rack_location: Option<String>,
    /// Free-text notes (synced).
    #[serde(default)]
    notes: Option<String>,
    /// Unit of measure (synced).
    #[serde(default)]
    unit: Option<String>,
    /// Active/sellable status — synced so retirement propagates to every
    /// store. `cost_minor`, `default_supplier_id`, and `popularity_score` are
    /// deliberately absent (local-only, ADR #36 D2 / ADR #37 D4).
    #[serde(default = "default_true")]
    is_active: bool,
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
pub async fn fetch_snapshot_from_server(config: &SyncConfig) -> Result<Snapshot, SyncHttpError> {
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
        .map_err(|e| SyncHttpError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(classify_http_status(status.as_u16(), &body));
    }

    let snapshot: Snapshot = resp
        .json()
        .await
        .map_err(|e| SyncHttpError::Parse(e.to_string()))?;

    Ok(snapshot)
}

/// Stub used when `sync-http` feature is disabled.
#[cfg(not(feature = "sync-http"))]
pub async fn fetch_snapshot_from_server(_config: &SyncConfig) -> Result<Snapshot, SyncHttpError> {
    Err(SyncHttpError::Network(
        "sync-http feature is disabled; cannot pull snapshot from server".into(),
    ))
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
                               price_updated_at, track_serial, store_id,
                               brand, rack_location, notes, unit, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                 COALESCE(?8, ?11), COALESCE(?9, ?11), COALESCE(?10, ?11), ?12, ?13,
                 ?14, ?15, ?16, ?17, ?18)
         ON CONFLICT (tenant_id, sku) DO UPDATE SET
             name            = excluded.name,
             price_minor     = excluded.price_minor,
             currency        = excluded.currency,
             category_id     = excluded.category_id,
             barcode         = excluded.barcode,
             updated_at      = COALESCE(excluded.updated_at, ?11),
             price_updated_at = COALESCE(excluded.price_updated_at, ?11),
             track_serial    = excluded.track_serial,
             store_id        = excluded.store_id,
             brand           = excluded.brand,
             rack_location   = excluded.rack_location,
             notes           = excluded.notes,
             unit            = excluded.unit,
             is_active       = excluded.is_active",
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
            p.store_id,
            p.brand,
            p.rack_location,
            p.notes,
            p.unit,
            p.is_active as i64,
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
         ON CONFLICT (tenant_id, username) DO UPDATE SET
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
#[path = "sync_client_tests.rs"]
mod tests;

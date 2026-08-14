//! Cloud settings administration — per-tenant SMTP / report-schedule
//! provisioning.
//!
//! - `GET /api/v1/settings?tenant=<id>` — read a tenant's **effective**
//!   cloud settings (scoped key `{base}:{tenant}` first, bare-key fallback),
//!   exactly as the report-sender loop in `apps/cloud-server` resolves them.
//! - `PUT /api/v1/settings` — write a tenant's scoped settings (SMTP
//!   config, report schedule, store name). Absent fields are left
//!   unchanged; an explicit `null` deletes the tenant's scoped override so
//!   it falls back to the bare key again.
//!
//! Both are gated by the same `OZ_ADMIN_KEY` as token minting and plan
//! assignment (ADR sync-auth-hardening P2): when the admin key is
//! configured, the `X-Admin-Key` header must match; in dev mode (no admin
//! key) the endpoints are open.
//!
//! Keys are always written in scoped suffix form (`smtp_config:{tenant}`,
//! `report_schedule:{tenant}`, `store.name:{tenant}`), which is exactly
//! what the cloud report loop reads — a second tenant is enabled purely by
//! provisioning its scoped keys, with no data migration. SMTP passwords
//! are encrypted at rest with `oz_core::crypto::encrypt_smtp_at_rest`
//! (matching what the report loop's `decrypt_smtp_at_rest` expects) and
//! decrypted in the GET response so admin round-trips are lossless.

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use oz_core::db::Store;
use oz_core::export::email_report::{SMTP_CONFIG_SETTINGS_KEY, SmtpConfig};
use oz_core::export::email_sender::LAST_SENT_KEY;
use oz_core::export::{REPORT_SCHEDULE_SETTINGS_KEY, ReportScheduleConfig};

use crate::AppState;
use crate::routes::tokens::admin_key_authorised;

/// Store-name settings key (bare form; scoped as `store.name:{tenant}`).
const STORE_NAME_SETTINGS_KEY: &str = "store.name";

/// One field-level write operation, resolved before any write happens so a
/// bad request never leaves a half-applied config.
enum Op {
    /// Field absent from the request — leave the stored value untouched.
    Leave,
    /// Explicit `null` — delete the tenant's scoped override (falls back
    /// to the bare key).
    Delete,
    /// Write this canonical serialized value to `{base}:{tenant}`.
    Write(String),
}

/// Query params for `GET /api/v1/settings`.
#[derive(Deserialize)]
pub struct GetSettingsParams {
    /// Tenant to read (defaults to `default`).
    pub tenant: Option<String>,
}

/// A PUT field that distinguishes absent from explicit `null`: a plain
/// `Option<Field<T>>` would have serde swallow the `null` into the outer
/// `None`, so the fields use `deserialize_field` (below):
/// absent → `None`, `null` → `Some(Field::Null)`, value → `Some(Field::Value(v))`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Field<T> {
    /// Explicit JSON `null` — delete the tenant's scoped override.
    Null,
    /// A concrete value to validate and store.
    Value(T),
}

/// Deserialize an optional PUT field, preserving explicit `null` (which a
/// plain `Option` would collapse into `None`). Used via `#[serde(default,
/// deserialize_with = "deserialize_field")]` on every optional field.
fn deserialize_field<'de, D, T>(d: D) -> Result<Option<Field<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = Value::deserialize(d)?;
    match value {
        Value::Null => Ok(Some(Field::Null)),
        other => match serde_json::from_value(other) {
            Ok(v) => Ok(Some(Field::Value(v))),
            Err(e) => Err(serde::de::Error::custom(e)),
        },
    }
}

/// Request body for `PUT /api/v1/settings` — every field is optional;
/// absent fields are left untouched, `null` deletes the scoped override.
#[derive(Deserialize)]
pub struct PutSettingsRequest {
    /// Tenant to write (defaults to `default`).
    pub tenant: Option<String>,
    /// Store display name override.
    #[serde(default, deserialize_with = "deserialize_field")]
    pub store_name: Option<Field<String>>,
    /// SMTP config override (validated against the loop's `SmtpConfig`).
    #[serde(default, deserialize_with = "deserialize_field")]
    pub smtp_config: Option<Field<Value>>,
    /// Report schedule override (validated against `ReportScheduleConfig`).
    #[serde(default, deserialize_with = "deserialize_field")]
    pub report_schedule: Option<Field<Value>>,
}

/// Effective per-tenant settings view returned by GET and after PUT.
#[derive(Serialize)]
pub struct SettingsView {
    /// The tenant these settings belong to.
    pub tenant: String,
    /// Effective store name (scoped first, then bare, then `null`).
    pub store_name: Option<String>,
    /// Effective SMTP config (password decrypted) or `null`.
    pub smtp_config: Option<SmtpConfig>,
    /// Effective report schedule or `null`.
    pub report_schedule: Option<ReportScheduleConfig>,
    /// Effective last-sent dedup timestamp or `null`.
    pub last_report_sent_at: Option<String>,
}

/// A tenant id must be a non-empty string of `[a-zA-Z0-9_-]` (max 64) so
/// scoped keys stay sane and unambiguous (`{base}:{tenant}`).
fn valid_tenant(tenant: &str) -> bool {
    !tenant.is_empty()
        && tenant.len() <= 64
        && tenant
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn scoped_key(base: &str, tenant: &str) -> String {
    crate::pg::scoped_setting_key(base, tenant)
}

/// Deserialize a stored SMTP config, decrypting the password (legacy
/// plaintext passes through unchanged). `None` on malformed storage.
fn parse_smtp_config(raw: &str) -> Option<SmtpConfig> {
    let mut config: SmtpConfig = serde_json::from_str(raw).ok()?;
    if let Some(ref pwd) = config.password
        && !pwd.is_empty()
    {
        config.password = Some(oz_core::crypto::decrypt_smtp_at_rest(pwd));
    }
    Some(config)
}

/// `GET /api/v1/settings` — read a tenant's effective cloud settings.
pub async fn get_settings_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<GetSettingsParams>,
) -> Response {
    if !admin_key_authorised(&headers, state.admin_key.as_deref()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_admin_key"})),
        )
            .into_response();
    }
    let tenant = params.tenant.as_deref().unwrap_or("default");
    if !valid_tenant(tenant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_tenant", "tenant": tenant})),
        )
            .into_response();
    }
    match read_settings(&state, tenant).await {
        Ok(view) => (StatusCode::OK, Json(view)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, tenant, "reading settings failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "settings_read_failed"})),
            )
                .into_response()
        }
    }
}

/// `PUT /api/v1/settings` — write a tenant's scoped settings.
///
/// Returns 401 without a matching admin key, 400 on validation failure
/// (unknown tenant charset, empty store name, malformed SMTP/schedule
/// JSON), 200 with the effective settings after the write.
pub async fn put_settings_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PutSettingsRequest>,
) -> Response {
    if !admin_key_authorised(&headers, state.admin_key.as_deref()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_admin_key"})),
        )
            .into_response();
    }
    let tenant = body.tenant.as_deref().unwrap_or("default");
    if !valid_tenant(tenant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_tenant", "tenant": tenant})),
        )
            .into_response();
    }

    // Validate + canonicalize every provided field BEFORE writing anything,
    // so a bad request never leaves a half-applied config.
    if let Some(Field::Value(name)) = &body.store_name
        && name.trim().is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_store_name"})),
        )
            .into_response();
    }
    let smtp_op = match &body.smtp_config {
        Some(Field::Value(value)) => match serde_json::from_value::<SmtpConfig>(value.clone()) {
            Ok(mut config) => {
                // Encrypt the password at rest like the rest of the cloud
                // path expects (decrypt on read is lossless).
                if let Some(ref pwd) = config.password
                    && !pwd.is_empty()
                {
                    config.password = Some(oz_core::crypto::encrypt_smtp_at_rest(pwd));
                }
                match serde_json::to_string(&config) {
                    Ok(json) => Op::Write(json),
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({"error": "invalid_smtp_config"})),
                        )
                            .into_response();
                    }
                }
            }
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "invalid_smtp_config"})),
                )
                    .into_response();
            }
        },
        Some(Field::Null) => Op::Delete,
        None => Op::Leave,
    };
    let schedule_op = match &body.report_schedule {
        Some(Field::Value(value)) => {
            match serde_json::from_value::<ReportScheduleConfig>(value.clone()) {
                Ok(config) => match serde_json::to_string(&config) {
                    Ok(json) => Op::Write(json),
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({"error": "invalid_report_schedule"})),
                        )
                            .into_response();
                    }
                },
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": "invalid_report_schedule"})),
                    )
                        .into_response();
                }
            }
        }
        Some(Field::Null) => Op::Delete,
        None => Op::Leave,
    };
    let store_op = match &body.store_name {
        Some(Field::Value(name)) => Op::Write(name.trim().to_string()),
        Some(Field::Null) => Op::Delete,
        None => Op::Leave,
    };

    if let Err(e) = write_settings(&state, tenant, store_op, smtp_op, schedule_op).await {
        tracing::error!(error = %e, tenant, "writing settings failed");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "settings_write_failed"})),
        )
            .into_response();
    }

    match read_settings(&state, tenant).await {
        Ok(view) => (StatusCode::OK, Json(view)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, tenant, "re-reading settings failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "settings_read_failed"})),
            )
                .into_response()
        }
    }
}

/// Apply the per-field write operations for one tenant.
async fn write_settings(
    state: &AppState,
    tenant: &str,
    store_op: Op,
    smtp_op: Op,
    schedule_op: Op,
) -> Result<(), String> {
    if let Some(pool) = &state.pg {
        apply_ops_pg(pool, tenant, store_op, smtp_op, schedule_op).await
    } else {
        let db = state.db.lock().await;
        apply_ops_sqlite(&db, tenant, store_op, smtp_op, schedule_op)
    }
}

/// Apply one field-level operation for a tenant on Postgres.
async fn apply_op_pg(
    pool: &deadpool_postgres::Pool,
    tenant: &str,
    base: &str,
    op: &Op,
) -> Result<(), String> {
    use crate::pg::set_setting_pg;
    match op {
        Op::Leave => Ok(()),
        Op::Delete => {
            let client = pool.get().await.map_err(|e| e.to_string())?;
            client
                .execute(
                    "DELETE FROM settings WHERE key = $1",
                    &[&scoped_key(base, tenant)],
                )
                .await
                .map_err(|e| format!("DB error: {e}"))?;
            Ok(())
        }
        Op::Write(value) => set_setting_pg(pool, &scoped_key(base, tenant), value).await,
    }
}

async fn apply_ops_pg(
    pool: &deadpool_postgres::Pool,
    tenant: &str,
    store_op: Op,
    smtp_op: Op,
    schedule_op: Op,
) -> Result<(), String> {
    apply_op_pg(pool, tenant, STORE_NAME_SETTINGS_KEY, &store_op).await?;
    apply_op_pg(pool, tenant, SMTP_CONFIG_SETTINGS_KEY, &smtp_op).await?;
    apply_op_pg(pool, tenant, REPORT_SCHEDULE_SETTINGS_KEY, &schedule_op).await?;
    Ok(())
}

fn apply_ops_sqlite(
    conn: &rusqlite::Connection,
    tenant: &str,
    store_op: Op,
    smtp_op: Op,
    schedule_op: Op,
) -> Result<(), String> {
    let store = Store::new(conn);
    let apply = |op: &Op, base: &str| -> Result<(), String> {
        let key = scoped_key(base, tenant);
        match op {
            Op::Leave => Ok(()),
            Op::Delete => conn
                .execute(
                    "DELETE FROM settings WHERE key = ?1",
                    rusqlite::params![key],
                )
                .map(|_| ())
                .map_err(|e| format!("DB error: {e}")),
            Op::Write(value) => store.set_setting(&key, value).map_err(|e| e.to_string()),
        }
    };
    apply(&store_op, STORE_NAME_SETTINGS_KEY)?;
    apply(&smtp_op, SMTP_CONFIG_SETTINGS_KEY)?;
    apply(&schedule_op, REPORT_SCHEDULE_SETTINGS_KEY)?;
    Ok(())
}

/// Read a scoped settings value from Postgres with bare-key fallback
/// (mirrors the report loop's resolution order).
async fn get_setting_scoped_pg(
    pool: &deadpool_postgres::Pool,
    base: &str,
    tenant: &str,
) -> Result<Option<String>, String> {
    let scoped = scoped_key(base, tenant);
    if let Some(v) = crate::pg::get_setting_pg(pool, &scoped).await? {
        return Ok(Some(v));
    }
    crate::pg::get_setting_pg(pool, base).await
}

/// Read a tenant's effective settings (scoped key first, bare fallback).
async fn read_settings(state: &AppState, tenant: &str) -> Result<SettingsView, String> {
    let (store_name, smtp_raw, schedule_raw, last_sent) = if let Some(pool) = &state.pg {
        (
            get_setting_scoped_pg(pool, STORE_NAME_SETTINGS_KEY, tenant).await?,
            get_setting_scoped_pg(pool, SMTP_CONFIG_SETTINGS_KEY, tenant).await?,
            get_setting_scoped_pg(pool, REPORT_SCHEDULE_SETTINGS_KEY, tenant).await?,
            get_setting_scoped_pg(pool, LAST_SENT_KEY, tenant).await?,
        )
    } else {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        let get_scoped = |base: &str| -> Result<Option<String>, String> {
            if let Some(v) = store
                .get_setting(&scoped_key(base, tenant))
                .map_err(|e| e.to_string())?
            {
                return Ok(Some(v));
            }
            store.get_setting(base).map_err(|e| e.to_string())
        };
        (
            get_scoped(STORE_NAME_SETTINGS_KEY)?,
            get_scoped(SMTP_CONFIG_SETTINGS_KEY)?,
            get_scoped(REPORT_SCHEDULE_SETTINGS_KEY)?,
            get_scoped(LAST_SENT_KEY)?,
        )
    };

    // Malformed stored values surface as `null` rather than failing the
    // whole read (the loop tolerates per-tenant parse failures too).
    let smtp_config = smtp_raw.as_deref().and_then(parse_smtp_config).or_else(|| {
        tracing::warn!(
            tenant,
            "stored smtp_config failed to parse; treating as unset"
        );
        None
    });
    let report_schedule = schedule_raw
        .as_deref()
        .and_then(|raw| serde_json::from_str::<ReportScheduleConfig>(raw).ok())
        .or_else(|| {
            tracing::warn!(
                tenant,
                "stored report_schedule failed to parse; treating as unset"
            );
            None
        });

    Ok(SettingsView {
        tenant: tenant.to_string(),
        store_name,
        smtp_config,
        report_schedule,
        last_report_sent_at: last_sent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    fn state_with(admin_key: Option<&str>) -> AppState {
        AppState {
            db: Arc::new(Mutex::new(oz_core::migrations::fresh_db())),
            pg: None,
            admin_key: admin_key.map(|s| s.to_owned()),
            api_secret: String::new(),
            db_path: ":memory:".into(),
            port: 3099,
        }
    }

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = resp
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        serde_json::from_slice(&bytes).expect("parse JSON body")
    }

    fn get_settings(tenant: Option<&str>, admin_key: Option<&str>) -> Request<Body> {
        let uri = match tenant {
            Some(t) => format!("/api/v1/settings?tenant={t}"),
            None => "/api/v1/settings".into(),
        };
        let mut builder = Request::builder().method("GET").uri(uri);
        if let Some(key) = admin_key {
            builder = builder.header("X-Admin-Key", key);
        }
        builder.body(Body::empty()).unwrap()
    }

    fn put_settings(body: &str, admin_key: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder()
            .method("PUT")
            .uri("/api/v1/settings")
            .header("Content-Type", "application/json");
        if let Some(key) = admin_key {
            builder = builder.header("X-Admin-Key", key);
        }
        builder.body(Body::from(body.to_owned())).unwrap()
    }

    fn smtp_json() -> String {
        r#"{"host":"smtp.example.com","port":587,"username":"u","password":"secret","from":"r@example.com","use_tls":true}"#.into()
    }

    fn schedule_json() -> String {
        r#"{"enabled":true,"cadence":"daily","report_types":["daily_revenue"],"recipients":["a@b.c"],"send_at_time":"08:00","timezone":"UTC","lookback_days":7}"#.into()
    }

    // ── Deserialization semantics ─────────────────────────────────

    #[test]
    fn field_null_semantics() {
        #[derive(Deserialize)]
        struct S {
            #[serde(default, deserialize_with = "deserialize_field")]
            a: Option<Field<String>>,
        }
        let s: S = serde_json::from_str(r#"{"a":null}"#).unwrap();
        assert!(
            matches!(s.a, Some(Field::Null)),
            "explicit null must be Some(Field::Null), got: {:?}",
            s.a
        );
        let s: S = serde_json::from_str(r#"{}"#).unwrap();
        assert!(s.a.is_none(), "missing field must deserialize as None");
        let s: S = serde_json::from_str(r#"{"a":"x"}"#).unwrap();
        assert!(matches!(s.a, Some(Field::Value(_))));
    }

    // ── Admin gating ──────────────────────────────────────────────

    #[tokio::test]
    async fn settings_require_admin_key_when_configured() {
        let app = router(state_with(Some("sekret")));
        let resp = app.clone().oneshot(get_settings(None, None)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let resp = app
            .clone()
            .oneshot(get_settings(None, Some("wrong")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let resp = app
            .oneshot(get_settings(None, Some("sekret")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn settings_open_in_dev_mode() {
        let app = router(state_with(None));
        let resp = app.oneshot(get_settings(None, None)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // ── GET default state ─────────────────────────────────────────

    #[tokio::test]
    async fn get_settings_returns_empty_effective_view() {
        let app = router(state_with(None));
        let resp = app.oneshot(get_settings(None, None)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["tenant"], "default");
        assert!(json["store_name"].is_null());
        assert!(json["smtp_config"].is_null());
        assert!(json["report_schedule"].is_null());
        assert!(json["last_report_sent_at"].is_null());
    }

    #[tokio::test]
    async fn get_settings_invalid_tenant_returns_400() {
        let app = router(state_with(None));
        let resp = app
            .oneshot(get_settings(Some("bad%20tenant!"), None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp).await;
        assert_eq!(json["error"], "invalid_tenant");
    }

    // ── PUT round-trip + scoping ──────────────────────────────────

    #[tokio::test]
    async fn put_then_get_round_trips_typed_config() {
        let app = router(state_with(None));
        let body = format!(
            r#"{{"smtp_config":{},"report_schedule":{},"store_name":"Cloud Store"}}"#,
            smtp_json(),
            schedule_json()
        );
        let resp = app
            .clone()
            .oneshot(put_settings(&body, None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["tenant"], "default");
        assert_eq!(json["store_name"], "Cloud Store");
        assert_eq!(json["smtp_config"]["host"], "smtp.example.com");
        assert_eq!(json["smtp_config"]["password"], "secret");
        assert_eq!(json["report_schedule"]["cadence"], "daily");

        let resp = app.oneshot(get_settings(None, None)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["store_name"], "Cloud Store");
        assert_eq!(json["smtp_config"]["host"], "smtp.example.com");
        // Password is decrypted on read, so the round-trip is lossless.
        assert_eq!(json["smtp_config"]["password"], "secret");
        assert_eq!(json["report_schedule"]["cadence"], "daily");
    }

    #[tokio::test]
    async fn password_is_encrypted_at_rest() {
        // Call the handler directly so we can inspect the raw settings row.
        let state = state_with(None);
        let req = PutSettingsRequest {
            tenant: None,
            store_name: None,
            smtp_config: Some(Field::Value(serde_json::from_str(&smtp_json()).unwrap())),
            report_schedule: None,
        };
        let resp = put_settings_handler(State(state.clone()), HeaderMap::new(), Json(req))
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);

        let db = state.db.lock().await;
        let raw: String = db
            .query_row(
                "SELECT value FROM settings WHERE key = 'smtp_config:default'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            !raw.contains("secret"),
            "password must be encrypted at rest, got: {raw}"
        );
        let stored: SmtpConfig = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            oz_core::crypto::decrypt_smtp_at_rest(stored.password.as_deref().unwrap()),
            "secret"
        );
    }

    #[tokio::test]
    async fn scoped_keys_are_written_suffix_form() {
        let app = router(state_with(None));
        let body = r#"{"tenant":"tenant-b","store_name":"B Store"}"#;
        let resp = app
            .clone()
            .oneshot(put_settings(&body, None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["tenant"], "tenant-b");
        assert_eq!(json["store_name"], "B Store");

        // tenant-b's override must not leak into default's effective view.
        let resp = app
            .oneshot(get_settings(Some("default"), None))
            .await
            .unwrap();
        let json = body_json(resp).await;
        assert!(json["store_name"].is_null(), "scoped key must not leak");
    }

    #[tokio::test]
    async fn scoped_key_falls_back_to_bare() {
        // The endpoint always writes scoped keys, so the bare-key fallback
        // matters for legacy deployments — seed a bare row directly (as
        // pre-endpoint provisioning would have) and read a tenant with no
        // scoped override.
        let conn = oz_core::migrations::fresh_db();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('store.name', 'Legacy Store')",
            [],
        )
        .unwrap();
        let state = AppState {
            db: Arc::new(Mutex::new(conn)),
            pg: None,
            admin_key: None,
            api_secret: String::new(),
            db_path: ":memory:".into(),
            port: 3099,
        };
        let app = router(state);

        let resp = app
            .oneshot(get_settings(Some("tenant-x"), None))
            .await
            .unwrap();
        let json = body_json(resp).await;
        assert_eq!(
            json["store_name"], "Legacy Store",
            "missing scoped key must fall back to the bare key"
        );
    }

    #[tokio::test]
    async fn null_deletes_scoped_override() {
        let app = router(state_with(None));
        let resp = app
            .clone()
            .oneshot(put_settings(
                r#"{"tenant":"tenant-b","store_name":"B Store"}"#,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // null → delete the scoped override; falls back to bare (absent).
        let resp = app
            .clone()
            .oneshot(put_settings(
                r#"{"tenant":"tenant-b","store_name":null}"#,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(
            json["store_name"].is_null(),
            "deleted override must read null"
        );
    }

    // ── Validation ────────────────────────────────────────────────

    #[tokio::test]
    async fn put_rejects_malformed_smtp_config() {
        let app = router(state_with(None));
        let resp = app
            .oneshot(put_settings(r#"{"smtp_config":{"host":123}}"#, None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp).await;
        assert_eq!(json["error"], "invalid_smtp_config");
    }

    #[tokio::test]
    async fn put_rejects_malformed_schedule() {
        let app = router(state_with(None));
        let resp = app
            .oneshot(put_settings(r#"{"report_schedule":{"cadence":42}}"#, None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp).await;
        assert_eq!(json["error"], "invalid_report_schedule");
    }

    #[tokio::test]
    async fn put_rejects_empty_store_name() {
        let app = router(state_with(None));
        let resp = app
            .oneshot(put_settings(r#"{"store_name":"   "}"#, None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp).await;
        assert_eq!(json["error"], "invalid_store_name");
    }

    #[tokio::test]
    async fn put_rejects_invalid_tenant() {
        let app = router(state_with(None));
        let resp = app
            .oneshot(put_settings(
                r#"{"tenant":"bad tenant!","store_name":"X"}"#,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp).await;
        assert_eq!(json["error"], "invalid_tenant");
    }

    // ── Postgres integration ──────────────────────────────────────

    /// Build a deadpool pool from `OZ_TEST_PG_URL` (falling back to the
    /// local dev container) and apply the schema. `None` when unreachable.
    async fn test_pool() -> Option<deadpool_postgres::Pool> {
        use deadpool_postgres::Manager;
        use std::str::FromStr;
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
        let config = tokio_postgres::Config::from_str(&url).expect("valid postgres URL");
        let manager = Manager::new(config, tokio_postgres::NoTls);
        let pool = deadpool_postgres::Pool::builder(manager)
            .max_size(5)
            .build()
            .expect("pool build");
        match pool.get().await {
            Ok(client) => {
                if let Err(e) = client.batch_execute(oz_core::migrations::PG_INIT).await {
                    eprintln!("PG settings integration: schema apply failed: {e:?}");
                    return None;
                }
                Some(pool)
            }
            Err(e) => {
                eprintln!("PG settings integration: pool get failed: {e}");
                None
            }
        }
    }

    /// PG round-trip: PUT writes `{base}:{tenant}` keys that the cloud
    /// report loop's scoped reads resolve, per tenant, with the SMTP
    /// password encrypted at rest.
    #[tokio::test]
    async fn pg_integration_settings_provision_per_tenant() {
        let Some(pool) = test_pool().await else {
            eprintln!("PG settings integration test skipped: no Postgres");
            return;
        };
        let ns = format!("pg-settings-test-{}", uuid::Uuid::now_v7());
        // Clean any leftovers from a crashed previous run (namespaced).
        {
            let client = pool.get().await.unwrap();
            client
                .batch_execute(&format!(
                    "DELETE FROM settings WHERE key LIKE 'smtp_config:%{ns}%'
                      OR key LIKE 'report_schedule:%{ns}%'
                      OR key LIKE 'store.name:%{ns}%'"
                ))
                .await
                .unwrap();
        }
        let query_pool = pool.clone();
        let state = AppState {
            db: Arc::new(Mutex::new(oz_core::migrations::fresh_db())),
            pg: Some(pool),
            admin_key: Some("sekret".into()),
            api_secret: String::new(),
            db_path: ":memory:".into(),
            port: 3099,
        };
        let app = router(state);
        let tenant_b = format!("{ns}-b");

        // Write tenant-b's SMTP + schedule via the admin endpoint.
        let body = format!(
            r#"{{"tenant":"{tenant_b}","store_name":"B Cloud Store","smtp_config":{},"report_schedule":{}}}"#,
            smtp_json(),
            schedule_json()
        );
        let resp = app
            .clone()
            .oneshot(put_settings(&body, Some("sekret")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "PUT must succeed on PG");
        let json = body_json(resp).await;
        assert_eq!(json["tenant"], tenant_b);
        assert_eq!(json["store_name"], "B Cloud Store");
        assert_eq!(json["smtp_config"]["host"], "smtp.example.com");
        assert_eq!(json["smtp_config"]["password"], "secret");

        // The scoped key is stored in suffix form and the password is
        // encrypted at rest.
        let client = query_pool.get().await.unwrap();
        let row = client
            .query_one(
                "SELECT value FROM settings WHERE key = $1",
                &[&format!("smtp_config:{tenant_b}")],
            )
            .await
            .expect("scoped smtp_config row must exist");
        let stored: String = row.get(0);
        assert!(
            !stored.contains("secret"),
            "password must be encrypted at rest, got: {stored}"
        );

        // default's view must not include tenant-b's override.
        let resp = app
            .clone()
            .oneshot(get_settings(Some("default"), Some("sekret")))
            .await
            .unwrap();
        let json = body_json(resp).await;
        assert!(
            json["smtp_config"].is_null(),
            "tenant-b's scoped config must not leak into default"
        );

        // And tenant-b reads its own effective config back.
        let resp = app
            .oneshot(get_settings(Some(&tenant_b), Some("sekret")))
            .await
            .unwrap();
        let json = body_json(resp).await;
        assert_eq!(json["store_name"], "B Cloud Store");
        assert_eq!(json["smtp_config"]["host"], "smtp.example.com");
    }
}

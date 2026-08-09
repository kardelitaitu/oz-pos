//! Tenant plan administration (ADR sync-plan-gating).
//!
//! `PUT /api/v1/tenants/{tenant_id}/plan` — set a tenant's cloud sync plan.
//! Gated by the same `OZ_ADMIN_KEY` as token minting (P2): when the admin
//! key is configured, the `X-Admin-Key` header must match; in dev mode
//! (no admin key) the endpoint is open so local setups can assign plans.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use oz_core::TenantPlan;
use oz_core::db::Store;

use crate::AppState;
use crate::routes::tokens::admin_key_authorised;

/// Request body for setting a tenant's plan.
#[derive(Deserialize)]
pub struct SetPlanRequest {
    /// `free` or `pro`. Unknown values are rejected (fail closed).
    pub plan: String,
}

/// `PUT /api/v1/tenants/{tenant_id}/plan` — assign or change a tenant's plan.
///
/// Returns 401 when an admin key is configured but missing/mismatched, 400
/// for an unknown plan name, 200 with the stored plan on success.
pub async fn set_tenant_plan_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<String>,
    Json(body): Json<SetPlanRequest>,
) -> Response {
    if !admin_key_authorised(&headers, state.admin_key.as_deref()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_admin_key"})),
        )
            .into_response();
    }

    let plan = match body.plan.as_str() {
        "free" => TenantPlan::Free,
        "pro" => TenantPlan::Pro,
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "unknown_plan",
                    "plan": other,
                    "supported": ["free", "pro"],
                })),
            )
                .into_response();
        }
    };

    let db = state.db.lock().await;
    let store = Store::new(&db);
    match store.set_tenant_plan(&tenant_id, plan) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "tenant_id": tenant_id,
                "plan": plan.as_db_str(),
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, tenant_id = %tenant_id, "setting tenant plan failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "plan_update_failed"})),
            )
                .into_response()
        }
    }
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

    fn test_app() -> axum::Router {
        let conn = oz_core::migrations::fresh_db();
        let state = AppState {
            db: Arc::new(Mutex::new(conn)),
            admin_key: None,
        };
        router(state)
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

    fn put_plan(uri: &str, body: &str, admin_key: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder()
            .method("PUT")
            .uri(uri)
            .header("Content-Type", "application/json");
        if let Some(key) = admin_key {
            builder = builder.header("X-Admin-Key", key);
        }
        builder.body(Body::from(body.to_owned())).unwrap()
    }

    #[tokio::test]
    async fn set_plan_pro_returns_stored_plan() {
        let app = test_app();
        let resp = app
            .oneshot(put_plan(
                "/api/v1/tenants/tenant-a/plan",
                r#"{"plan":"pro"}"#,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["tenant_id"], "tenant-a");
        assert_eq!(json["plan"], "pro");
    }

    #[tokio::test]
    async fn set_plan_free_accepted() {
        let app = test_app();
        let resp = app
            .oneshot(put_plan(
                "/api/v1/tenants/tenant-a/plan",
                r#"{"plan":"free"}"#,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["plan"], "free");
    }

    #[tokio::test]
    async fn set_plan_unknown_plan_rejected() {
        let app = test_app();
        let resp = app
            .oneshot(put_plan(
                "/api/v1/tenants/tenant-a/plan",
                r#"{"plan":"enterprise"}"#,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp).await;
        assert_eq!(json["error"], "unknown_plan");
    }

    #[tokio::test]
    async fn set_plan_requires_admin_key_when_configured() {
        let conn = oz_core::migrations::fresh_db();
        let state = AppState {
            db: Arc::new(Mutex::new(conn)),
            admin_key: Some("super-secret".to_string()),
        };
        let app = router(state);

        // No key → 401.
        let resp = app
            .clone()
            .oneshot(put_plan(
                "/api/v1/tenants/tenant-a/plan",
                r#"{"plan":"pro"}"#,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // Wrong key → 401.
        let resp = app
            .clone()
            .oneshot(put_plan(
                "/api/v1/tenants/tenant-a/plan",
                r#"{"plan":"pro"}"#,
                Some("wrong-key"),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // Correct key → 200.
        let resp = app
            .oneshot(put_plan(
                "/api/v1/tenants/tenant-a/plan",
                r#"{"plan":"pro"}"#,
                Some("super-secret"),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

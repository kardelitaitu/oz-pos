use super::*;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::get,
};
use tower::ServiceExt;

// ── Preset resolution ───────────────────────────────────────────────

#[test]
fn resolve_preset_terminal_binds_read_keys() {
    let keys = resolve_preset("terminal").unwrap();
    assert!(keys.contains(&"products:read"));
    assert!(keys.contains(&"categories:read"));
    assert!(keys.contains(&"reference:read"));
    assert!(keys.contains(&"plan:read"));
    // Terminal preset must never carry PII-scoped keys.
    assert!(!keys.contains(&"sales:view"));
    assert!(!keys.contains(&"staff:read"));
}

#[test]
fn resolve_preset_dashboard_excludes_pii_routes() {
    let keys = resolve_preset("dashboard").unwrap();
    assert!(keys.contains(&"products:read"));
    assert!(keys.contains(&"reports:view"));
    assert!(keys.contains(&"analytics:view"));
    // Dashboard is derived by excluding pii-marked routes (decision 3):
    // sales:view gates the pii-flagged /api/v1/sales/{id} route.
    assert!(!keys.contains(&"sales:view"));
}

#[test]
fn resolve_preset_audit_has_audit_and_reports() {
    let keys = resolve_preset("audit").unwrap();
    assert!(keys.contains(&"audit:view"));
    assert!(keys.contains(&"reports:view"));
}

#[test]
fn resolve_preset_unknown_returns_none() {
    assert!(resolve_preset("superuser").is_none());
    assert!(resolve_preset("").is_none());
}

// ── Key validation ──────────────────────────────────────────────────

#[test]
fn validate_keys_accepts_registered_keys() {
    let keys = vec!["products:read".to_string(), "sales:view".to_string()];
    assert!(validate_keys(&keys).is_ok());
}

#[test]
fn validate_keys_rejects_unknown_keys() {
    let keys = vec!["products:read".to_string(), "not:a_key".to_string()];
    let err = validate_keys(&keys).unwrap_err();
    assert_eq!(err, vec!["not:a_key".to_string()]);
}

#[test]
fn validate_keys_accepts_new_read_tier_keys() {
    // reference:read / plan:read / categories:read were added for 0047.
    let keys = vec![
        "reference:read".to_string(),
        "plan:read".to_string(),
        "categories:read".to_string(),
    ];
    assert!(validate_keys(&keys).is_ok());
}

// ── Path matching ───────────────────────────────────────────────────

#[test]
fn path_matches_concrete_route() {
    assert!(path_matches("/api/v1/products", "/api/v1/products"));
    assert!(path_matches(
        "/api/v1/products/{sku}",
        "/api/v1/products/BEV-01"
    ));
    assert!(path_matches(
        "/api/v1/exchange-rates/latest/{from}/{to}",
        "/api/v1/exchange-rates/latest/USD/IDR"
    ));
    assert!(path_matches("/api/v1/images:pack", "/api/v1/images:pack"));
    assert!(path_matches(
        "/api/v1/images/{hash16}",
        "/api/v1/images/aaaaaaaaaaaaaaaa"
    ));
}

#[test]
fn path_matches_rejects_wrong_segments() {
    assert!(!path_matches("/api/v1/products", "/api/v1/categories"));
    assert!(!path_matches(
        "/api/v1/products/{sku}",
        "/api/v1/products/BEV-01/extra"
    ));
    assert!(!path_matches(
        "/api/v1/products/{sku}",
        "/api/v1/categories/BEV-01"
    ));
}

// ── Read-gate middleware ────────────────────────────────────────────

/// Build a tiny router exercising the read gate directly.
fn gate_app() -> Router {
    async fn ok_handler() -> StatusCode {
        StatusCode::OK
    }
    let handler = get(ok_handler);
    Router::new()
        .route("/api/v1/products", handler.clone())
        .route("/api/v1/products/{sku}", handler.clone())
        .route("/api/v1/sales/{id}", handler.clone())
        .route("/api/sync/status", handler) // not in map → always passes
        .layer(middleware::from_fn(read_gate_middleware))
}

fn claims(permissions: Option<Vec<String>>) -> ApiTokenClaims {
    ApiTokenClaims {
        sub: "test".into(),
        jti: "jti-1".into(),
        exp: 9_999_999_999,
        iat: 1_700_000_000,
        tenant_id: Some("tenant-a".into()),
        terminal_id: None,
        permissions,
    }
}

fn req_with_claims(uri: &str, claims: ApiTokenClaims) -> Request<Body> {
    let mut req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    req.extensions_mut().insert(claims);
    req
}

#[tokio::test]
async fn gate_passes_legacy_token_without_permissions() {
    let app = gate_app();
    let resp = app
        .clone()
        .oneshot(req_with_claims("/api/v1/products", claims(None)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn gate_passes_with_matching_permission() {
    let app = gate_app();
    let resp = app
        .clone()
        .oneshot(req_with_claims(
            "/api/v1/products",
            claims(Some(vec!["products:read".into()])),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn gate_denies_without_matching_permission() {
    let app = gate_app();
    let resp = app
        .clone()
        .oneshot(req_with_claims(
            "/api/v1/sales/00000000-0000-0000-0000-000000000000",
            claims(Some(vec!["products:read".into()])),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "insufficient_scope");
}

#[tokio::test]
async fn gate_passes_route_not_in_map() {
    let app = gate_app();
    let resp = app
        .clone()
        .oneshot(req_with_claims(
            "/api/sync/status",
            claims(Some(vec!["products:read".into()])),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn gate_uses_has_permission_wildcards() {
    // products:* grants products:read through the registry resolver.
    let app = gate_app();
    let resp = app
        .clone()
        .oneshot(req_with_claims(
            "/api/v1/products",
            claims(Some(vec!["products:*".into()])),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ── PII invariant (spec 0047 decision 3) ────────────────────────────

#[test]
fn dashboard_preset_disjoint_from_pii_routes() {
    // Every key the dashboard preset carries must gate only non-PII routes.
    let dashboard: Vec<&str> = DASHBOARD_PRESET.to_vec();
    for entry in READ_KEY_MAP {
        if entry.pii {
            assert!(
                !dashboard.contains(&entry.key),
                "dashboard preset leaks into PII route {} ({})",
                entry.path,
                entry.key
            );
        }
    }
}

#[test]
fn read_key_map_covers_all_protected_get_routes() {
    // Every image + product + reference GET route in the router must have
    // a read-key entry. This mirrors the OpenAPI drift-guard assertion.
    let expected_paths = [
        "/api/v1/products",
        "/api/v1/products/{sku}",
        "/api/v1/categories",
        "/api/v1/exchange-rates",
        "/api/v1/exchange-rates/latest",
        "/api/v1/exchange-rates/latest/{from}/{to}",
        "/api/v1/tenants/me/plan",
        "/api/v1/sales/{id}",
        "/api/v1/images:pack",
        "/api/v1/images:missing",
        "/api/v1/images/{hash16}",
    ];
    for p in &expected_paths {
        assert!(
            READ_KEY_MAP.iter().any(|e| e.path == *p),
            "READ_KEY_MAP missing route {}",
            p
        );
    }
}

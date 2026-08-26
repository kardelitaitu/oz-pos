use super::*;
use crate::DEFAULT_CORS_ORIGINS;
use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;
use tokio::sync::Mutex;

fn state() -> AppState {
    AppState {
        db: Arc::new(Mutex::new(oz_core::migrations::fresh_db())),
        pg: None,
        admin_key: None,
        api_secret: String::new(),
        db_path: ":memory:".into(),
        port: 3099,
        cors_origins: DEFAULT_CORS_ORIGINS.iter().map(|s| s.to_string()).collect(),
    }
}

fn claims(tenant_id: Option<&str>) -> ApiTokenClaims {
    ApiTokenClaims {
        sub: "test-token".into(),
        jti: "jti-1".into(),
        exp: 9999999999,
        iat: 1000000000,
        tenant_id: tenant_id.map(|s| s.to_owned()),
        terminal_id: None,
    }
}

fn body() -> CreateTaxRateRequest {
    CreateTaxRateRequest {
        name: "VAT 10%".into(),
        rate_bps: 1000,
        is_default: true,
        is_inclusive: false,
    }
}

// ── store_error_response mapping ───────────────────────────────

#[test]
fn store_error_response_maps_validation_to_400() {
    let resp = store_error_response(CoreError::Validation {
        message: "bad input".into(),
        field: "name",
    })
    .into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn store_error_response_maps_conflict_to_409() {
    let resp = store_error_response(CoreError::Conflict {
        entity: "tax_rate",
        field: "name",
    })
    .into_response();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[test]
fn store_error_response_maps_not_found_to_404() {
    let resp = store_error_response(CoreError::NotFound {
        entity: "tax_rate",
        id: "x".into(),
    })
    .into_response();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[test]
fn store_error_response_maps_unknown_to_500() {
    let resp = store_error_response(CoreError::Internal("boom".into())).into_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// ── create_tax_rate handler ────────────────────────────────────

#[tokio::test]
async fn create_tax_rate_returns_201_with_default_tenant() {
    let response = create_tax_rate(State(state()), Extension(claims(None)), Json(body()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["name"], "VAT 10%");
    assert_eq!(json["rate_bps"], 1000);
    assert!(json["is_default"].as_bool().unwrap());
}

#[tokio::test]
async fn create_tax_rate_stamps_tenant_from_claims() {
    let app_state = state();
    let response = create_tax_rate(
        State(app_state.clone()),
        Extension(claims(Some("tenant-42"))),
        Json(body()),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::CREATED);

    let db = app_state.db.lock().await;
    let tenant: String = db
        .query_row(
            "SELECT tenant_id FROM tax_rates WHERE name = 'VAT 10%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        tenant, "tenant-42",
        "tenant_id must be stamped from JWT claims"
    );
}

#[tokio::test]
async fn create_tax_rate_returns_400_on_validation_error() {
    let bad = CreateTaxRateRequest {
        name: "".into(), // empty name → store validation error
        rate_bps: 1000,
        is_default: false,
        is_inclusive: false,
    };
    let response = create_tax_rate(State(state()), Extension(claims(None)), Json(bad))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_tax_rate_allows_duplicate_name() {
    // The store has NO uniqueness constraint on tax_rates.name (verified:
    // no UNIQUE index and no CoreError::Conflict in the tax store), so a
    // duplicate-name create is legal and must return 201. This pins the
    // current contract — if name uniqueness is added later, this test
    // will fail and force the handler's 409 path to be exercised.
    let app_state = state();
    {
        let db = app_state.db.lock().await;
        let store = Store::new(&db);
        store
            .create_tax_rate("VAT 10%", 1000, false, false)
            .unwrap();
    }
    let response = create_tax_rate(
        State(app_state.clone()),
        Extension(claims(None)),
        Json(body()),
    )
    .await
    .into_response();
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "duplicate tax-rate names are currently legal (no unique constraint)"
    );
}

// ── CreateTaxRateRequest deserialization ────────────────────

#[test]
fn create_tax_rate_request_minimal() {
    let json = r#"{"name":"VAT 10%","rate_bps":1000,"is_default":true,"is_inclusive":false}"#;
    let req: CreateTaxRateRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.name, "VAT 10%");
    assert_eq!(req.rate_bps, 1000);
    assert!(req.is_default);
    assert!(!req.is_inclusive);
}

#[test]
fn create_tax_rate_request_inclusive() {
    let json = r#"{"name":"GST 5%","rate_bps":500,"is_default":false,"is_inclusive":true}"#;
    let req: CreateTaxRateRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.name, "GST 5%");
    assert_eq!(req.rate_bps, 500);
    assert!(!req.is_default);
    assert!(req.is_inclusive);
}

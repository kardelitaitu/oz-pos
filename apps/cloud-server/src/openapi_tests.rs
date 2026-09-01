use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn test_app() -> axum::Router {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/openapi.json", get(openapi_json_handler))
        .route("/api/docs", get(swagger_ui_handler))
        .route("/api/docs/scalar", get(scalar_ui_handler))
}

#[tokio::test]
async fn openapi_json_returns_200() {
    let app = test_app();
    let req = Request::builder()
        .uri("/api/openapi.json")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn openapi_json_has_required_fields() {
    let app = test_app();
    let req = Request::builder()
        .uri("/api/openapi.json")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["openapi"], "3.1.0");
    assert!(json["info"]["title"].is_string());
    assert!(json["info"]["version"].is_string());
    assert!(json["paths"].is_object());
    assert!(json["components"]["schemas"].is_object());
    assert!(json["components"]["securitySchemes"]["bearerAuth"].is_object());
}

#[tokio::test]
async fn openapi_json_documents_all_tag_groups() {
    let app = test_app();
    let req = Request::builder()
        .uri("/api/openapi.json")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let tags: Vec<&str> = json["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(tags.contains(&"Health"));
    assert!(tags.contains(&"Auth"));
    assert!(tags.contains(&"Products"));
    assert!(tags.contains(&"Categories"));
    assert!(tags.contains(&"Tax Rates"));
    assert!(tags.contains(&"Users"));
    assert!(tags.contains(&"Sales"));
    assert!(tags.contains(&"Sync"));
    assert!(tags.contains(&"Plans"));
    assert!(tags.contains(&"Terminals"));
    assert!(tags.contains(&"Webhooks"));
    // Future tag groups — reserved for planned features.
    assert!(tags.contains(&"Inventory"));
    assert!(tags.contains(&"Orders"));
    assert!(tags.contains(&"Reports"));
    assert!(tags.contains(&"Customers"));
    assert!(tags.contains(&"Notifications"));
    assert!(tags.contains(&"Analytics"));
    assert!(tags.contains(&"Images"));
}

#[tokio::test]
async fn openapi_json_documents_all_paths() {
    let app = test_app();
    let req = Request::builder()
        .uri("/api/openapi.json")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let paths = json["paths"].as_object().unwrap();
    assert!(paths.contains_key("/health"), "missing /health");
    assert!(paths.contains_key("/api/health"), "missing /api/health");
    assert!(
        paths.contains_key("/api/v1/health"),
        "missing /api/v1/health"
    );
    assert!(paths.contains_key("/metrics"), "missing /metrics");
    assert!(paths.contains_key("/api/v1/tokens"), "missing tokens");
    assert!(paths.contains_key("/api/v1/products"), "missing products");
    assert!(
        paths.contains_key("/api/v1/products/{sku}"),
        "missing product by SKU"
    );
    assert!(
        paths.contains_key("/api/v1/products/{sku}/stock"),
        "missing stock adjustment"
    );
    assert!(
        paths.contains_key("/api/v1/tenants/me/plan"),
        "missing self plan read"
    );
    assert!(
        paths.contains_key("/api/v1/tenants/{tenant_id}/plan"),
        "missing plan set"
    );
    assert!(
        paths.contains_key("/api/v1/categories"),
        "missing categories"
    );
    assert!(paths.contains_key("/api/v1/tax-rates"), "missing tax rates");
    assert!(
        paths.contains_key("/api/v1/exchange-rates"),
        "missing exchange rates"
    );
    assert!(
        paths.contains_key("/api/v1/exchange-rates/latest"),
        "missing exchange rates latest listing"
    );
    assert!(
        paths.contains_key("/api/v1/exchange-rates/latest/{from}/{to}"),
        "missing exchange rates pair lookup"
    );
    assert!(
        paths.contains_key("/api/v1/exchange-rates/{id}"),
        "missing exchange rate delete"
    );
    assert!(paths.contains_key("/api/v1/users"), "missing users");
    assert!(paths.contains_key("/api/v1/sales"), "missing sales");
    assert!(
        paths.contains_key("/api/v1/sales/{id}"),
        "missing sale by ID"
    );
    assert!(
        paths.contains_key("/api/v1/sales/{id}/status"),
        "missing sale status update"
    );
    assert!(
        paths.contains_key("/api/v1/terminals"),
        "missing terminal registration"
    );
    assert!(
        paths.contains_key("/api/sync/status"),
        "missing sync status"
    );
    assert!(paths.contains_key("/api/sync/push"), "missing sync push");
    assert!(paths.contains_key("/api/sync/pull"), "missing sync pull");
    assert!(
        paths.contains_key("/api/webhooks/stripe"),
        "missing stripe webhook"
    );
    assert!(
        paths.contains_key("/api/webhooks/square"),
        "missing square webhook"
    );
    assert!(paths.contains_key("/api/v1/images"), "missing images");
    assert!(
        paths.contains_key("/api/v1/images:pack"),
        "missing images pack"
    );
    assert!(
        paths.contains_key("/api/v1/images:missing"),
        "missing images missing-set"
    );
    assert!(
        paths.contains_key("/api/v1/images/{hash16}"),
        "missing images by hash"
    );
}

#[tokio::test]
async fn swagger_ui_returns_html() {
    let app = test_app();
    let req = Request::builder()
        .uri("/api/docs")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("swagger-ui"));
    assert!(html.contains("/api/openapi.json"));
}

#[tokio::test]
async fn scalar_ui_returns_html() {
    let app = test_app();
    let req = Request::builder()
        .uri("/api/docs/scalar")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("api-reference"));
    assert!(html.contains("/api/openapi.json"));
}

#[test]
fn openapi_spec_is_valid_json() {
    let spec = openapi_spec();
    // Verify it serializes without panicking
    let _json_str = serde_json::to_string_pretty(&spec).unwrap();
}

#[test]
fn security_scheme_has_bearer_auth() {
    let spec = openapi_spec();
    let security = &spec["components"]["securitySchemes"]["bearerAuth"];
    assert_eq!(security["type"], "http");
    assert_eq!(security["scheme"], "bearer");
    assert_eq!(security["bearerFormat"], "JWT");
}

#[test]
fn all_protected_routes_have_security() {
    let spec = openapi_spec();
    let paths = spec["paths"].as_object().unwrap();

    // Routes that should have bearerAuth security
    let protected_routes = [
        ("/api/v1/products", "get"),
        ("/api/v1/products", "post"),
        ("/api/v1/products/{sku}", "get"),
        ("/api/v1/products/{sku}/stock", "patch"),
        ("/api/v1/categories", "get"),
        ("/api/v1/tax-rates", "post"),
        ("/api/v1/exchange-rates", "get"),
        ("/api/v1/exchange-rates", "post"),
        ("/api/v1/exchange-rates/latest", "get"),
        ("/api/v1/exchange-rates/latest/{from}/{to}", "get"),
        ("/api/v1/exchange-rates/{id}", "delete"),
        ("/api/v1/users", "post"),
        ("/api/v1/sales", "post"),
        ("/api/v1/sales/{id}", "get"),
        ("/api/v1/sales/{id}/status", "patch"),
        ("/api/sync/status", "get"),
        ("/api/sync/push", "post"),
        ("/api/sync/pull", "post"),
        // Images (spec 0046b) — all operations require bearerAuth.
        ("/api/v1/images", "put"),
        ("/api/v1/images", "post"),
        ("/api/v1/images:pack", "get"),
        ("/api/v1/images:missing", "get"),
        ("/api/v1/images/{hash16}", "get"),
    ];

    for (path, method) in &protected_routes {
        let operation = &paths[*path][*method];
        let security = operation
            .get("security")
            .unwrap_or_else(|| panic!("{method} {path} must have security defined"));
        let has_bearer = security
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s.as_object().unwrap().contains_key("bearerAuth"));
        assert!(has_bearer, "{method} {path} must have bearerAuth security");
    }
}

#[test]
fn health_endpoints_have_no_security() {
    let spec = openapi_spec();
    let paths = spec["paths"].as_object().unwrap();

    for path in &["/health", "/api/health", "/api/v1/health", "/metrics"] {
        let operation = &paths[*path]["get"];
        assert!(
            operation.get("security").is_none()
                || operation
                    .get("security")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .is_empty(),
            "{path} should not require security"
        );
    }
}

// ── Drift guard (spec 0047 §3) — liveness probe ──────────────────────

/// Build a full router backed by an in-memory SQLite database, used by
/// the drift-guard liveness probe to confirm every documented path+method
/// resolves to a real route (≠ 404 ≠ 405).
fn test_full_router() -> axum::Router {
    let state = crate::CloudServerState {
        db: std::sync::Arc::new(tokio::sync::Mutex::new(oz_core::migrations::fresh_db())),
        pg: None,
        started_at: std::time::Instant::now(),
        health_depth_cache: crate::HealthDepthCache::default(),
        stripe_webhook_secret: None,
        square_webhook_signature_key: None,
        square_webhook_url: None,
    };
    let config = crate::config::CloudServerConfig {
        db_path: ":memory:".into(),
        database_url: None,
        require_tls: false,
        db_pool_size: 20,
        apply_schema: true,
        port: 3099,
        admin_key: None,
        enforce_plans: false,
        production: false,
        log_format: crate::config::LogFormat::Plain,
        redirect_only: false,
        sync_redirect_url: None,
        stripe_webhook_secret: None,
        square_webhook_signature_key: None,
        square_webhook_url: None,
        api_secret: Some("test-secret".into()),
        redis_url: None,
    };
    crate::build_router(
        state,
        crate::rate_limit::RateLimiterState::new(),
        &config,
        None,
    )
}

/// Every (path, method) declared in the OpenAPI spec must resolve to a
/// real route in the merged router.  We fire a `tower::ServiceExt::oneshot`
/// request for each and assert the status is **not 404 and not 405**
/// (401 is a pass — it proves the route + method exist; auth is not this
/// test's job).  Catches renamed/removed/retyped endpoints.
#[tokio::test]
async fn every_spec_path_method_is_alive() {
    let spec = openapi_spec();
    let paths = spec["paths"].as_object().unwrap().clone();
    let router = test_full_router();
    let mut results = Vec::new();

    for (path_template, methods) in &paths {
        let methods_obj = methods.as_object().unwrap();
        for (method, _operation) in methods_obj {
            // Build a concrete URL from the path template.
            let url = concrete_url(path_template);
            // OpenAPI method keys are lowercase ("get"); HTTP methods are
            // case-sensitive and must be uppercase to match the router.
            let uppercase = method.to_uppercase();
            let http_method = axum::http::Method::from_bytes(uppercase.as_bytes())
                .expect("OpenAPI methods are standard HTTP verbs");
            let req = Request::builder()
                .method(http_method)
                .uri(&url)
                .body(Body::empty())
                .unwrap();
            let resp = router.clone().oneshot(req).await.unwrap();
            let status = resp.status();
            if status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED {
                results.push(format!(
                    "[{method} {path_template}] ({url}) → {status} — route missing or method mismatch"
                ));
            }
        }
    }

    if !results.is_empty() {
        panic!(
            "Drift detected — {} spec-declared route(s) are not alive:\n{}",
            results.len(),
            results.join("\n")
        );
    }
}

/// Replace OpenAPI path parameters (`{param}`) with test values.
fn concrete_url(template: &str) -> String {
    template
        .replace("{sku}", "TEST-SKU")
        .replace("{id}", "00000000-0000-0000-0000-000000000000")
        .replace("{from}", "USD")
        .replace("{to}", "IDR")
        .replace("{tenant_id}", "tenant-a")
        .replace("{hash16}", "aaaaaaaaaaaaaaaa")
}

/// Every operation in the spec must declare `security: [{bearerAuth: []}]`
/// unless the path is on the explicit public allowlist.  A new endpoint
/// silently missing the security block becomes a red test (spec 0047 §3
/// assertion 3 — the mechanical audit-stamp guard).
#[test]
fn security_coverage_walk_every_operation() {
    let spec = openapi_spec();
    let paths = spec["paths"].as_object().unwrap();

    /// Paths that are intentionally public (no bearerAuth).  Everything
    /// else must carry bearerAuth.
    fn is_public(path: &str) -> bool {
        path == "/health"
            || path == "/api/health"
            || path == "/api/v1/health"
            || path == "/metrics"
            || path == "/api/openapi.json"
            || path == "/api/docs"
            || path == "/api/docs/scalar"
            || path == "/api/v1/tokens" // mints tokens — cannot require one
            // Admin-key gated (X-Admin-Key header), not JWT — so no
            // bearerAuth; they are public in the JWT sense.
            || path == "/api/v1/terminals"
            || path == "/api/v1/tenants/{tenant_id}/plan"
            || path.starts_with("/api/webhooks/")
    }

    let mut violations = Vec::new();
    for (path, methods) in paths {
        for (method, operation) in methods.as_object().unwrap() {
            let has_bearer = operation
                .get("security")
                .and_then(|s| s.as_array())
                .is_some_and(|arr| {
                    arr.iter().any(|entry| {
                        entry
                            .as_object()
                            .is_some_and(|m| m.contains_key("bearerAuth"))
                    })
                });
            if is_public(path) {
                // Public endpoints must NOT require auth.
                if has_bearer {
                    violations.push(format!("{method} {path} is public but declares bearerAuth"));
                }
            } else if !has_bearer {
                violations.push(format!("{method} {path} is missing bearerAuth security"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Security-coverage drift — {} violation(s):\n{}",
        violations.len(),
        violations.join("\n")
    );
}

// ── Drift-guard assertion 4 (spec 0047 §3) — READ_KEY_MAP coverage ──

/// Every protected GET operation in the spec must have a corresponding
/// entry in `oz_api::read_tiers::READ_KEY_MAP`.  Public/health/docs and
/// sync routes are excluded (they keep their own gating).
#[test]
fn every_spec_get_operation_has_read_key_entry() {
    let spec = openapi_spec();
    let paths = spec["paths"].as_object().unwrap();

    let mut missing = Vec::new();

    for (path, methods) in paths {
        for (method, operation) in methods.as_object().unwrap() {
            // Non-GET methods are not in the read-key map.
            if *method != "get" {
                continue;
            }
            // Public routes (no bearerAuth) are excluded from the map.
            let has_bearer = operation
                .get("security")
                .and_then(|s| s.as_array())
                .is_some_and(|arr| {
                    arr.iter().any(|entry| {
                        entry
                            .as_object()
                            .is_some_and(|m| m.contains_key("bearerAuth"))
                    })
                });
            if !has_bearer {
                continue;
            }
            // Sync routes keep their existing gating (spec 0047 §4 F3) —
            // they are excluded from the read-key map.
            if path.starts_with("/api/sync/") {
                continue;
            }

            // Check if READ_KEY_MAP has an entry for this (method, path).
            let in_map = oz_api::read_tiers::READ_KEY_MAP
                .iter()
                .any(|e| e.method == "GET" && e.path == path.as_str());
            if !in_map {
                missing.push(format!("GET {path}"));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Drift detected — {} GET operation(s) in the spec have no READ_KEY_MAP entry:\n{}",
        missing.len(),
        missing.join("\n")
    );
}

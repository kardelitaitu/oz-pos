
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
        ("/api/v1/users", "post"),
        ("/api/v1/sales", "post"),
        ("/api/v1/sales/{id}", "get"),
        ("/api/v1/sales/{id}/status", "patch"),
        ("/api/sync/status", "get"),
        ("/api/sync/push", "post"),
        ("/api/sync/pull", "post"),
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

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

fn body() -> CreateUserRequest {
    CreateUserRequest {
        username: "alice".into(),
        pin_hash: "hash123".into(),
        display_name: "Alice".into(),
        role_id: "role-staff".into(),
    }
}

/// Seed the role FK target so user creation can succeed.
fn seed_role(conn: &rusqlite::Connection, role_id: &str) {
    conn.execute(
        "INSERT OR IGNORE INTO roles (id, name, permissions) VALUES (?1, ?2, '[]')",
        rusqlite::params![role_id, role_id],
    )
    .unwrap();
}

// ── create_user handler ─────────────────────────────────────────

#[tokio::test]
async fn create_user_returns_201_with_default_tenant() {
    let app_state = state();
    {
        let db = app_state.db.lock().await;
        seed_role(&db, "role-staff");
    }
    let response = create_user(State(app_state), Extension(claims(None)), Json(body()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["username"], "alice");
    assert_eq!(json["display_name"], "Alice");
}

#[tokio::test]
async fn create_user_stamps_tenant_from_claims() {
    let app_state = state();
    {
        let db = app_state.db.lock().await;
        seed_role(&db, "role-staff");
    }
    let response = create_user(
        State(app_state.clone()),
        Extension(claims(Some("tenant-9"))),
        Json(body()),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::CREATED);

    let db = app_state.db.lock().await;
    let tenant: String = db
        .query_row(
            "SELECT tenant_id FROM users WHERE username = 'alice'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        tenant, "tenant-9",
        "tenant_id must be stamped from JWT claims"
    );
}

#[tokio::test]
async fn create_user_normalizes_username_to_lowercase() {
    let app_state = state();
    {
        let db = app_state.db.lock().await;
        seed_role(&db, "role-staff");
    }
    let body = CreateUserRequest {
        username: "  ALICE  ".into(), // trimmed + lowercased by the store
        pin_hash: "hash123".into(),
        display_name: "Alice".into(),
        role_id: "role-staff".into(),
    };
    let response = create_user(
        State(app_state.clone()),
        Extension(claims(None)),
        Json(body),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::CREATED);

    let db = app_state.db.lock().await;
    let username: String = db
        .query_row(
            "SELECT username FROM users WHERE id = (SELECT MAX(id) FROM users)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(username, "alice", "username must be trimmed and lowercased");
}

#[tokio::test]
async fn create_user_returns_400_on_empty_username() {
    let app_state = state();
    {
        let db = app_state.db.lock().await;
        seed_role(&db, "role-staff");
    }
    let bad = CreateUserRequest {
        username: "   ".into(),
        pin_hash: "hash123".into(),
        display_name: "Alice".into(),
        role_id: "role-staff".into(),
    };
    let response = create_user(State(state()), Extension(claims(None)), Json(bad))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_user_returns_409_on_duplicate_username() {
    // users.username has a UNIQUE constraint and the store maps the
    // violation to CoreError::Conflict → the handler must return 409.
    let app_state = state();
    {
        let db = app_state.db.lock().await;
        seed_role(&db, "role-staff");
        let store = Store::new(&db);
        store
            .create_user("alice", "hash123", "Alice", "role-staff")
            .unwrap();
    }
    let response = create_user(
        State(app_state.clone()),
        Extension(claims(None)),
        Json(body()),
    )
    .await
    .into_response();
    assert_eq!(
        response.status(),
        StatusCode::CONFLICT,
        "duplicate username must surface as 409"
    );
}

// ── CreateUserRequest deserialization ───────────────────────

#[test]
fn create_user_request_minimal() {
    let json = r#"{"username":"alice","pin_hash":"hash123","display_name":"Alice","role_id":"role-staff"}"#;
    let req: CreateUserRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.username, "alice");
    assert_eq!(req.pin_hash, "hash123");
    assert_eq!(req.display_name, "Alice");
    assert_eq!(req.role_id, "role-staff");
}

#[test]
fn create_user_request_owner_role() {
    let json =
        r#"{"username":"owner","pin_hash":"abc","display_name":"Owner","role_id":"role-owner"}"#;
    let req: CreateUserRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.username, "owner");
    assert_eq!(req.role_id, "role-owner");
}

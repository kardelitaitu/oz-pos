use super::*;

#[test]
fn staff_login_args_deserialize() {
    let json = r#"{"username":"cashier1","pin":"1234"}"#;
    let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.username, "cashier1");
    assert_eq!(args.pin, "1234");
}

#[test]
fn staff_login_args_debug() {
    let args = StaffLoginArgs {
        username: "admin".into(),
        pin: "9999".into(),
        device_id: Some("term-1".into()),
    };
    let debug = format!("{:?}", args);
    assert!(debug.contains("admin"));
}

#[test]
fn staff_login_args_device_id_defaults_none() {
    // `device_id` is optional — legacy JSON without it must deserialize.
    let json = r#"{"username":"cashier1","pin":"1234"}"#;
    let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.device_id, None);
}

#[test]
fn staff_login_result_serialize() {
    let result = StaffLoginResult {
        session: LoginSession {
            user_id: "u1".into(),
            display_name: "Alice".into(),
            role_name: "Manager".into(),
            role_id: "r1".into(),
            permissions: vec!["analytics:view".into()],
        },
        picker_ticket: String::new(),
    };
    let json = serde_json::to_value(&result).unwrap();
    let session = &json["session"];
    assert_eq!(session["user_id"], "u1");
    assert_eq!(session["display_name"], "Alice");
    assert_eq!(session["role_name"], "Manager");
}

#[test]
fn staff_login_result_debug() {
    let result = StaffLoginResult {
        session: LoginSession {
            user_id: "u1".into(),
            display_name: "Bob".into(),
            role_name: "Cashier".into(),
            role_id: "r2".into(),
            permissions: vec![],
        },
        picker_ticket: String::new(),
    };
    let debug = format!("{:?}", result);
    assert!(debug.contains("Bob"));
}

// ── Session-mint authorization gate (audit-open-findings residual) ───────────
//
// Parity with the desktop client: `create_session` must fail closed
// when the caller claims an identity it has not authenticated. The
// gate itself is `oz_core::Store::verify_instance_access` (shared with
// the desktop client); these tests pin the command-level behavior on
// the tablet too.

use oz_core::migrations;
use tauri::Manager as _;

/// Seed the built-in roles plus one owner user in the GLOBAL identity DB.
fn seed_owner(conn: &rusqlite::Connection) {
    let store = Store::new(conn);
    store.seed_default_roles().unwrap();
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
    )
    .unwrap();
}

#[tokio::test]
async fn staff_login_mints_verifiable_picker_ticket() {
    // audit-open-findings (parity with the desktop client): the picker ticket
    // returned by a successful login must verify against the process
    // secret and bind the authenticated user.
    let conn = migrations::fresh_db();
    let store = Store::new(&conn);
    store.seed_default_roles().unwrap();
    let hash = oz_core::auth::hash_pin("1234").unwrap();
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-owner', 'owner', ?1, 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [hash],
    )
    .unwrap();
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = staff_login(
        StaffLoginArgs {
            username: "owner".into(),
            pin: "1234".into(),
            device_id: None,
        },
        app.state(),
    )
    .await
    .unwrap();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let state = app.state::<AppState>();
    assert_eq!(
        picker_ticket::verify_picker_ticket(
            &state.picker_ticket_secret,
            &result.picker_ticket,
            now
        )
        .as_deref(),
        Some("user-owner"),
        "login must mint a ticket bound to the authenticated user"
    );
}

#[tokio::test]
async fn staff_login_returns_granted_permission_keys() {
    // Parity with the desktop client: the session carries the role's
    // granted keys verbatim (Owner's preset grants the global `"*"`
    // wildcard) so UI gates can mirror the backend registry.
    let conn = migrations::fresh_db();
    let store = Store::new(&conn);
    store.seed_default_roles().unwrap();
    let hash = oz_core::auth::hash_pin("1234").unwrap();
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-owner', 'owner', ?1, 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [hash],
    )
    .unwrap();
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = staff_login(
        StaffLoginArgs {
            username: "owner".into(),
            pin: "1234".into(),
            device_id: None,
        },
        app.state(),
    )
    .await
    .unwrap();

    assert_eq!(
        result.session.permissions,
        vec!["*".to_string()],
        "owner login must carry the role's granted keys (global wildcard)"
    );
}

#[tokio::test]
async fn create_session_rejects_forged_role_id() {
    // A staff user whose REAL role is role-staff claims role-owner.
    let conn = migrations::fresh_db();
    let store = Store::new(&conn);
    store.seed_default_roles().unwrap();
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-staff', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
    )
    .unwrap();
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = create_session(
        CreateSessionArgs {
            user_id: "user-cashier".into(),
            role_id: "role-owner".into(), // forged
            store_id: "default".into(),
            instance_id: "default-restaurant-pos".into(),
            type_key: "restaurant-pos".into(),
            terminal_id: "terminal-1".into(),
        },
        app.state(),
    )
    .await;
    assert!(
        matches!(result, Err(AppError::Invalid(_))),
        "forged role must not mint a session"
    );
    let state = app.state::<AppState>();
    assert_eq!(
        state.session_store.read().unwrap().len(),
        0,
        "no session token may be created for a forged role"
    );
}

#[tokio::test]
async fn create_session_rejects_unknown_user() {
    let conn = migrations::fresh_db();
    seed_owner(&conn);
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = create_session(
        CreateSessionArgs {
            user_id: "ghost-user".into(),
            role_id: "role-owner".into(),
            store_id: "default".into(),
            instance_id: "default-restaurant-pos".into(),
            type_key: "restaurant-pos".into(),
            terminal_id: "terminal-1".into(),
        },
        app.state(),
    )
    .await;
    assert!(
        matches!(result, Err(AppError::Invalid(_))),
        "unknown user must not be able to open a session"
    );
    let state = app.state::<AppState>();
    assert_eq!(state.session_store.read().unwrap().len(), 0);
}

#[tokio::test]
async fn create_session_allows_real_owner() {
    let conn = migrations::fresh_db();
    seed_owner(&conn);
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = create_session(
        CreateSessionArgs {
            user_id: "user-owner".into(),
            role_id: "role-owner".into(),
            store_id: "default".into(),
            instance_id: "default-restaurant-pos".into(),
            type_key: "restaurant-pos".into(),
            terminal_id: "terminal-1".into(),
        },
        app.state(),
    )
    .await
    .unwrap();
    assert_eq!(result.context.role_id, "role-owner");
    assert_eq!(result.context.user_id, "user-owner");
    let state = app.state::<AppState>();
    assert_eq!(state.session_store.read().unwrap().len(), 1);
}

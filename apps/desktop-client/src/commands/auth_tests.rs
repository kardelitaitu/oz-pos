use super::*;

/// Test helper: sign a picker ticket for the given user using the test secret.
fn test_picker_ticket(user_id: &str) -> String {
    let secret = b"test-picker-ticket-secret";
    let expiry = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + 300; // 5 min from now
    picker_ticket::sign_picker_ticket(secret, user_id, expiry)
}

// ── StaffLoginArgs ──────────────────────────────────────────────────

#[test]
fn staff_login_args_deserialize() {
    let json = r##"{"username":"jdoe","pin":"1234"}"##;
    let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.username, "jdoe");
    assert_eq!(args.pin, "1234");
}

#[test]
fn staff_login_args_debug() {
    let args = StaffLoginArgs {
        username: "u".into(),
        pin: "0000".into(),
        device_id: Some("term-1".into()),
    };
    let d = format!("{args:?}");
    assert!(d.contains("u"));
}

#[test]
fn staff_login_args_device_id_defaults_none() {
    // `device_id` is optional — legacy JSON without it must deserialize.
    let json = r##"{"username":"jdoe","pin":"1234"}"##;
    let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.device_id, None);
}

#[test]
fn staff_login_args_device_id_deserializes() {
    let json = r##"{"username":"jdoe","pin":"1234","device_id":"term-7"}"##;
    let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.device_id.as_deref(), Some("term-7"));
}

// ── StaffLoginArgs edge cases ────────────────────────────────────────

#[test]
fn staff_login_args_whitespace_username() {
    let json = r##"{"username":"   ","pin":"1234"}"##;
    let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
    // After trimming in staff_login, this becomes empty
    assert_eq!(args.username, "   ");
    assert_eq!(args.pin, "1234");
}

#[test]
fn staff_login_args_empty_pin() {
    let json = r##"{"username":"jdoe","pin":""}"##;
    let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.username, "jdoe");
    assert_eq!(args.pin, "");
}

#[test]
fn staff_login_args_long_pin() {
    let json = r##"{"username":"jdoe","pin":"12345678901234567890"}"##;
    let args: StaffLoginArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.pin.len(), 20);
}

// ── StaffLoginResult ────────────────────────────────────────────────

#[test]
fn staff_login_result_serialize() {
    let session = LoginSession {
        user_id: "u1".into(),
        display_name: "John".into(),
        role_name: "Manager".into(),
        role_id: "r1".into(),
        permissions: vec!["analytics:view".into()],
    };
    let result = StaffLoginResult {
        session,
        picker_ticket: String::new(),
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["session"]["user_id"], "u1");
    assert_eq!(json["session"]["role_name"], "Manager");
}

#[test]
fn staff_login_result_debug() {
    let session = LoginSession {
        user_id: "u2".into(),
        display_name: "Alice".into(),
        role_name: "Cashier".into(),
        role_id: "r2".into(),
        permissions: vec![],
    };
    let result = StaffLoginResult {
        session,
        picker_ticket: String::new(),
    };
    let d = format!("{result:?}");
    assert!(d.contains("Alice"));
}

// ── Error mapping edge cases ────────────────────────────────────────

#[test]
fn staff_login_result_empty_display_name() {
    let session = LoginSession {
        user_id: "u3".into(),
        display_name: "".into(),
        role_name: "Cashier".into(),
        role_id: "r3".into(),
        permissions: vec![],
    };
    let result = StaffLoginResult {
        session,
        picker_ticket: String::new(),
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["session"]["display_name"], "");
}

#[test]
fn staff_login_result_null_role_id() {
    let session = LoginSession {
        user_id: "u4".into(),
        display_name: "Bob".into(),
        role_name: "".into(),
        role_id: "".into(),
        permissions: vec![],
    };
    let result = StaffLoginResult {
        session,
        picker_ticket: String::new(),
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["session"]["role_name"], "");
    assert_eq!(json["session"]["role_id"], "");
}

// ── Session-mint authorization gate (audit-open-findings residual) ───────────
//
// TDD red: `create_session` must fail closed when the caller claims an
// identity it has not authenticated — unknown user, or a role_id that
// does not match the user's actual database role. Previously the gate
// (oz_core `Store::verify_instance_access`) trusted the claimed role
// and never resolved the user, so a caller who knew an owner's user id
// could mint a session as that owner and inherit every permission.

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
    // audit-open-findings: the picker ticket returned by a successful login must
    // verify against the process secret and bind the authenticated user.
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
    // The session carries the role's granted keys verbatim so UI gates
    // can mirror the backend registry. Owner's preset grants the global
    // `"*"` wildcard — the DTO must surface it as-is.
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
            picker_ticket: test_picker_ticket("user-cashier"),
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
            picker_ticket: test_picker_ticket("ghost-user"),
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
            picker_ticket: test_picker_ticket("user-owner"),
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

// ── refresh_picker_ticket ───────────────────────────────────────────

#[tokio::test]
async fn refresh_picker_ticket_returns_fresh_ticket() {
    let conn = migrations::fresh_db();
    seed_owner(&conn);
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    // Create a session first.
    let session_token = create_session(
        CreateSessionArgs {
            user_id: "user-owner".into(),
            role_id: "role-owner".into(),
            store_id: "default".into(),
            instance_id: "default-restaurant-pos".into(),
            type_key: "restaurant-pos".into(),
            terminal_id: "terminal-1".into(),
            picker_ticket: test_picker_ticket("user-owner"),
        },
        app.state(),
    )
    .await
    .unwrap()
    .session_token;

    let result = refresh_picker_ticket(session_token, app.state())
        .await
        .unwrap();

    // The fresh ticket must be a valid, non-empty HMAC ticket.
    assert!(!result.picker_ticket.is_empty());

    // Verify it against the process secret — must bind the same user.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let state = app.state::<AppState>();
    assert_eq!(
        picker_ticket::verify_picker_ticket(
            &state.picker_ticket_secret,
            &result.picker_ticket,
            now,
        )
        .as_deref(),
        Some("user-owner"),
        "refreshed ticket must bind the session user"
    );
}

#[tokio::test]
async fn refresh_picker_ticket_rejects_invalid_session() {
    let conn = migrations::fresh_db();
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = refresh_picker_ticket("nonexistent-token".into(), app.state()).await;
    assert!(
        matches!(result, Err(AppError::InvalidSession)),
        "invalid session must be rejected"
    );
}

#[tokio::test]
async fn refresh_picker_ticket_rejects_expired_session() {
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

    // Manually insert an expired session.
    {
        let state = app.state::<AppState>();
        let mut session_store = state.session_store.write().unwrap();
        let ctx = oz_core::session::SessionContext::new(
            "user-owner".into(),
            "role-owner".into(),
            "terminal-1".into(),
            "default".into(),
            "default-restaurant-pos".into(),
            "restaurant-pos".into(),
            Some(-1), // already expired
            0,
        );
        session_store.insert("expired-session-token".into(), ctx);
    }

    let result = refresh_picker_ticket("expired-session-token".into(), app.state()).await;
    assert!(
        matches!(result, Err(AppError::InvalidSession)),
        "expired session must be rejected"
    );
}

#[tokio::test]
async fn refreshed_picker_ticket_can_be_used_for_create_session() {
    // End-to-end: login → refresh ticket → create_session with refreshed ticket.
    let conn = migrations::fresh_db();
    seed_owner(&conn);
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    // Step 1: Create a session.
    let session_token = create_session(
        CreateSessionArgs {
            user_id: "user-owner".into(),
            role_id: "role-owner".into(),
            store_id: "default".into(),
            instance_id: "default-restaurant-pos".into(),
            type_key: "restaurant-pos".into(),
            terminal_id: "terminal-1".into(),
            picker_ticket: test_picker_ticket("user-owner"),
        },
        app.state(),
    )
    .await
    .unwrap()
    .session_token;

    // Step 2: Refresh the picker ticket.
    let refresh_result = refresh_picker_ticket(session_token, app.state())
        .await
        .unwrap();

    // Step 3: Use the refreshed ticket to create ANOTHER session.
    let result = create_session(
        CreateSessionArgs {
            user_id: "user-owner".into(),
            role_id: "role-owner".into(),
            store_id: "default".into(),
            instance_id: "default-restaurant-pos".into(),
            type_key: "restaurant-pos".into(),
            terminal_id: "terminal-1".into(),
            picker_ticket: refresh_result.picker_ticket,
        },
        app.state(),
    )
    .await
    .unwrap();

    assert_eq!(result.context.user_id, "user-owner");
    assert_eq!(result.context.role_id, "role-owner");
    assert!(!result.session_token.is_empty());
}

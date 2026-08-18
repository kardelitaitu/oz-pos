
use super::*;

// ── StaffMemberDto ──────────────────────────────────────────────────

#[test]
fn staff_member_dto_debug() {
    let dto = StaffMemberDto {
        id: "u1".into(),
        username: "jdoe".into(),
        display_name: "John Doe".into(),
        role_id: "r1".into(),
        role_name: "Manager".into(),
        is_active: true,
        national_id_masked: "*****6789".into(),
        is_profile_complete: true,
        assignment: assignment_dto(None),
    };
    let d = format!("{dto:?}");
    assert!(d.contains("jdoe"));
    assert!(d.contains("John Doe"));
    assert!(d.contains("Manager"));
}

#[test]
fn staff_member_dto_serialize() {
    let dto = StaffMemberDto {
        id: "u2".into(),
        username: "asmith".into(),
        display_name: "Alice Smith".into(),
        role_id: "r2".into(),
        role_name: "Cashier".into(),
        is_active: false,
        national_id_masked: "****".into(),
        is_profile_complete: false,
        assignment: assignment_dto(None),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["username"], "asmith");
    assert_eq!(json["role_name"], "Cashier");
    assert_eq!(json["is_active"], false);
}

// ── RoleDto ─────────────────────────────────────────────────────────

#[test]
fn role_dto_debug() {
    let dto = RoleDto {
        id: "r1".into(),
        name: "Admin".into(),
        description: "Full access".into(),
        permissions: vec![],
    };
    let d = format!("{dto:?}");
    assert!(d.contains("Admin"));
    assert!(d.contains("Full access"));
}

#[test]
fn role_dto_serialize() {
    let dto = RoleDto {
        id: "r2".into(),
        name: "Viewer".into(),
        description: String::new(),
        permissions: vec![],
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["name"], "Viewer");
    assert_eq!(json["description"], "");
}

// ── CreateStaffArgs ─────────────────────────────────────────────────

#[test]
fn create_staff_args_deserialize() {
    let json = r##"{"username":"jdoe","pin":"1234","display_name":"John Doe","role_id":"r1","caller_user_id":"admin1"}"##;
    let args: CreateStaffArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.username, "jdoe");
    assert_eq!(args.pin, "1234");
    assert_eq!(args.display_name, "John Doe");
    assert_eq!(args.role_id, "r1");
    assert_eq!(args.caller_user_id, "admin1");
}

#[test]
fn create_staff_args_debug() {
    let args = CreateStaffArgs {
        username: "u".into(),
        pin: "0000".into(),
        display_name: "D".into(),
        role_id: "r".into(),
        caller_user_id: "c".into(),
    };
    let d = format!("{args:?}");
    assert!(d.contains("u"));
    assert!(d.contains("r"));
}

// ── UpdateStaffArgs ─────────────────────────────────────────────────

#[test]
fn update_staff_args_deserialize() {
    let json = r##"{"id":"u1","username":"jdoe2","display_name":"John D","role_id":"r2","is_active":false,"caller_user_id":"admin1"}"##;
    let args: UpdateStaffArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.id, "u1");
    assert!(!args.is_active);
    assert_eq!(args.caller_user_id, "admin1");
}

#[test]
fn update_staff_args_debug() {
    let args = UpdateStaffArgs {
        id: "x".into(),
        username: "y".into(),
        display_name: "z".into(),
        role_id: "r".into(),
        is_active: true,
        caller_user_id: "c".into(),
    };
    let d = format!("{args:?}");
    assert!(d.contains("z"));
}

// ── BootstrapOwnerArgs / BootstrapOwnerResult ────────────────────────

#[test]
fn bootstrap_owner_args_deserialize() {
    let json = r##"{"username":"owner","pin":"1234","display_name":"Store Owner"}"##;
    let args: BootstrapOwnerArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.username, "owner");
    assert_eq!(args.pin, "1234");
    assert_eq!(args.display_name, "Store Owner");
}

#[test]
fn bootstrap_owner_args_debug() {
    let args = BootstrapOwnerArgs {
        username: "owner".into(),
        pin: "0000".into(),
        display_name: "Owner".into(),
    };
    let d = format!("{args:?}");
    assert!(d.contains("owner"));
    assert!(d.contains("Owner"));
}

#[test]
fn bootstrap_owner_result_serialize() {
    let result = BootstrapOwnerResult {
        session: oz_core::auth::LoginSession {
            user_id: "u1".into(),
            display_name: "Owner".into(),
            role_name: "Owner".into(),
            role_id: "role-owner".into(),
            permissions: vec!["*".into()],
        },
        picker_ticket: "ticket".into(),
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["session"]["role_id"], "role-owner");
    assert_eq!(json["picker_ticket"], "ticket");
}

#[test]
fn bootstrap_owner_result_debug() {
    let result = BootstrapOwnerResult {
        session: oz_core::auth::LoginSession {
            user_id: "u2".into(),
            display_name: "Boss".into(),
            role_name: "Owner".into(),
            role_id: "role-owner".into(),
            permissions: vec![],
        },
        picker_ticket: String::new(),
    };
    let d = format!("{result:?}");
    assert!(d.contains("Boss"));
}

// ── run_bootstrap_owner logic ───────────────────────────────────────

use oz_core::migrations;
use tauri::Manager as _;

#[test]
fn run_bootstrap_owner_creates_owner_role_user() {
    let conn = migrations::fresh_db();
    let result = run_bootstrap_owner(
        &conn,
        &BootstrapOwnerArgs {
            username: "owner".into(),
            pin: "1234".into(),
            display_name: "Store Owner".into(),
        },
    )
    .unwrap();

    assert_eq!(result.session.role_id, "role-owner");
    assert_eq!(result.session.role_name, "Owner");
    assert_eq!(result.session.display_name, "Store Owner");

    let store = Store::new(&conn);
    let user = store.get_user(&result.session.user_id).unwrap().unwrap();
    assert_eq!(user.role_id, oz_core::builtin_roles::OWNER);
    assert!(user.is_active);
}

#[test]
fn run_bootstrap_owner_lowercases_username() {
    let conn = migrations::fresh_db();
    let result = run_bootstrap_owner(
        &conn,
        &BootstrapOwnerArgs {
            username: "  OWNER  ".into(),
            pin: "1234".into(),
            display_name: "  Store Owner  ".into(),
        },
    )
    .unwrap();
    // `create_user` assigns a UUID id; the USERNAME must be normalized.
    assert_eq!(result.session.display_name, "Store Owner");
    let user = Store::new(&conn)
        .get_user_by_username("owner")
        .unwrap()
        .unwrap();
    assert_eq!(user.id, result.session.user_id);
}

#[test]
fn run_bootstrap_owner_rejects_when_users_exist() {
    let conn = migrations::fresh_db();
    // Bootstrap once, then try again — the second call must fail closed.
    run_bootstrap_owner(
        &conn,
        &BootstrapOwnerArgs {
            username: "owner".into(),
            pin: "1234".into(),
            display_name: "Owner".into(),
        },
    )
    .unwrap();
    let err = run_bootstrap_owner(
        &conn,
        &BootstrapOwnerArgs {
            username: "owner2".into(),
            pin: "1234".into(),
            display_name: "Owner 2".into(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, AppError::Invalid(_)));
    // No second user may exist.
    assert_eq!(Store::new(&conn).list_users().unwrap().len(), 1);
}

#[test]
fn run_bootstrap_owner_rejects_empty_username() {
    let conn = migrations::fresh_db();
    let err = run_bootstrap_owner(
        &conn,
        &BootstrapOwnerArgs {
            username: "  ".into(),
            pin: "1234".into(),
            display_name: "Owner".into(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, AppError::Invalid(_)));
}

#[test]
fn run_bootstrap_owner_rejects_empty_display_name() {
    let conn = migrations::fresh_db();
    let err = run_bootstrap_owner(
        &conn,
        &BootstrapOwnerArgs {
            username: "owner".into(),
            pin: "1234".into(),
            display_name: "  ".into(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, AppError::Invalid(_)));
}

#[test]
fn run_bootstrap_owner_rejects_short_pin() {
    let conn = migrations::fresh_db();
    let err = run_bootstrap_owner(
        &conn,
        &BootstrapOwnerArgs {
            username: "owner".into(),
            pin: "123".into(),
            display_name: "Owner".into(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, AppError::Invalid(_)));
}

#[tokio::test]
async fn bootstrap_owner_mints_verifiable_picker_ticket() {
    // audit/06 (parity with the desktop client): the command-level
    // bootstrap must mint a ticket bound to the NEW owner so the
    // pre-session workspace picker works immediately after setup.
    let conn = migrations::fresh_db();
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = bootstrap_owner(
        BootstrapOwnerArgs {
            username: "owner".into(),
            pin: "1234".into(),
            display_name: "Store Owner".into(),
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
        Some(result.session.user_id.as_str()),
        "bootstrap must mint a ticket bound to the new owner"
    );
    assert!(!result.picker_ticket.is_empty());
    assert_eq!(result.session.role_id, "role-owner");
}


use super::*;

// ── Token Rejection ─────────────────────────────────────────────────

#[test]
fn workspaces_scoped_rejects_invalid_token() {
    let state = AppState::for_test();
    let result = state.resolve_session("nonexistent-token");
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

// ── WorkspaceTypeDto ─────────────────────────────────────────────────

#[test]
fn workspace_type_dto_debug() {
    let dto = WorkspaceTypeDto {
        key: "retail".into(),
        name: "Retail".into(),
        description: "Retail POS".into(),
        icon: "store".into(),
    };
    let d = format!("{dto:?}");
    assert!(d.contains("retail"));
    assert!(d.contains("Retail POS"));
}

#[test]
fn workspace_type_dto_serialize() {
    let dto = WorkspaceTypeDto {
        key: "restaurant".into(),
        name: "Restaurant".into(),
        description: String::new(),
        icon: "utensils".into(),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["key"], "restaurant");
    assert_eq!(json["description"], "");
}

// ── WorkspaceScreenDto ──────────────────────────────────────────────

#[test]
fn workspace_screen_dto_debug() {
    let dto = WorkspaceScreenDto {
        screen_key: "pos".into(),
        sort_order: 1,
    };
    let d = format!("{dto:?}");
    assert!(d.contains("pos"));
    assert!(d.contains("1"));
}

#[test]
fn workspace_screen_dto_serialize() {
    let dto = WorkspaceScreenDto {
        screen_key: "history".into(),
        sort_order: 5,
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["screen_key"], "history");
    assert_eq!(json["sort_order"], 5);
}

// ── CreateInstanceRequest ───────────────────────────────────────────

#[test]
fn create_instance_request_deserializes() {
    let json = r#"{
        "id": "ws-dt-1",
        "type_key": "restaurant-pos",
        "store_id": "store-downtown",
        "name": "Downtown - Cashier 1"
    }"#;
    let req: CreateInstanceRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.id, "ws-dt-1");
    assert_eq!(req.type_key, "restaurant-pos");
    assert_eq!(req.name, "Downtown - Cashier 1");
    assert!(req.description.is_none());
    assert!(req.colour.is_none());
    assert!(req.purpose_key.is_none());
}

// ── BootResolution (ADR #4 Phase 3) ─────────────────────────────────

#[test]
fn boot_resolution_dto_serialize_bound() {
    let res = BootResolution {
        is_bound: true,
        store_id: "store-downtown".into(),
        instance_id: Some("ws-dt-cashier-1".into()),
    };
    let json = serde_json::to_value(&res).unwrap();
    assert_eq!(json["isBound"], true);
    assert_eq!(json["storeId"], "store-downtown");
    assert_eq!(json["instanceId"], "ws-dt-cashier-1");
}

#[test]
fn boot_resolution_dto_serialize_unbound() {
    let res = BootResolution {
        is_bound: false,
        store_id: "default".into(),
        instance_id: None,
    };
    let json = serde_json::to_value(&res).unwrap();
    assert_eq!(json["isBound"], false);
    assert_eq!(json["storeId"], "default");
    assert!(json["instanceId"].is_null());
}

#[test]
fn boot_resolution_dto_debug() {
    let res = BootResolution {
        is_bound: false,
        store_id: "default".into(),
        instance_id: None,
    };
    let d = format!("{res:?}");
    assert!(d.contains("default"));
    assert!(d.contains("false"));
}

// ── Pre-session picker ticket binding (audit/06 residual) ──────────
//
// TDD red: `list_workspaces` / `list_workspace_screens` must bind the
// listing to the authenticated user server-side. Previously the commands
// trusted the caller-supplied `role_id` / `user_id`, so any caller who
// knew an owner's id could enumerate instances in any store as
// `role-owner`. Now the caller presents a short-lived HMAC ticket and the
// REAL role is resolved from the global identity DB.

use oz_core::StoreProfile;
use oz_core::db::assignments::{AssignmentSpec, ScopeMode};
use oz_core::migrations;
use platform_core::StoreDatabaseManager;
use tauri::Manager as _;

/// Seed the GLOBAL identity DB with an owner and a limited user whose
/// role has no workspace-type grants (so it sees no instances — the
/// retired cashier role behaved the same way; 0048 sweep).
fn seed_global_users(conn: &rusqlite::Connection) {
    let store = Store::new(conn);
    store.seed_default_roles().unwrap();
    conn.execute_batch(
        "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
            ('role-lite', 'Lite', 'Limited', '[\"sales:view\"]', '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');
         INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
            ('user-owner',   'owner',   'hash', 'Owner',   'role-owner',   1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
            ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-lite',    1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
    )
    .unwrap();
}

fn make_profile(id: &str, name: &str) -> StoreProfile {
    StoreProfile {
        id: id.to_owned(),
        name: name.to_owned(),
        address: String::new(),
        tax_id: String::new(),
        currency: "USD".to_owned(),
        timezone: "UTC".to_owned(),
        is_primary: false,
        created_at: "2026-07-01T10:00:00Z".to_owned(),
        updated_at: "2026-07-01T10:00:00Z".to_owned(),
    }
}

/// Build an AppState with a global identity DB (users seeded) and a
/// temp-dir store manager with store-a (1 instance) and store-b (1
/// instance) so cross-store isolation can be exercised.
fn picker_state() -> (AppState, tempfile::TempDir) {
    let conn = migrations::fresh_db();
    seed_global_users(&conn);
    let temp_dir = tempfile::tempdir().unwrap();
    let mut state = AppState::for_test_with_conn(conn);
    state.db_manager =
        StoreDatabaseManager::new(temp_dir.path().to_path_buf(), migrations::ALL);

    for (store_id, instance_id) in [("store-a", "ws-a-1"), ("store-b", "ws-b-1")] {
        let conn = state.db_manager.open_store(store_id).unwrap();
        let db = conn.lock().unwrap();
        let store = Store::new(&db);
        store
            .create_store_profile(&make_profile(store_id, store_id))
            .unwrap();
        store
            .create_workspace_instance(instance_id, "store-pos", store_id, "POS", "", None)
            .unwrap();
    }
    (state, temp_dir)
}

fn sign_ticket_for(state: &AppState, user_id: &str, ttl_offset: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    picker_ticket::sign_picker_ticket(&state.picker_ticket_secret, user_id, now + ttl_offset)
}

#[tokio::test]
async fn list_workspaces_rejects_forged_or_missing_ticket() {
    let (state, _dir) = picker_state();
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // Empty ticket, garbage ticket, and a ticket forged with the wrong
    // secret must all be denied uniformly (no enumeration oracle).
    for ticket in [
        String::new(),
        "garbage-not-a-ticket".into(),
        picker_ticket::sign_picker_ticket(
            b"attacker-secret",
            "user-owner",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64
                + 300,
        ),
    ] {
        let result = list_workspaces(app.state(), ticket.clone(), "store-a".into()).await;
        assert!(
            matches!(result, Err(AppError::PermissionDenied(_))),
            "ticket {ticket:?} must be denied"
        );
    }
}

#[tokio::test]
async fn list_workspaces_rejects_expired_ticket() {
    let (state, _dir) = picker_state();
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let expired = sign_ticket_for(&app.state(), "user-owner", -1);
    let result = list_workspaces(app.state(), expired, "store-a".into()).await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

#[tokio::test]
async fn list_workspaces_rejects_unknown_user_even_with_valid_signature() {
    let (state, _dir) = picker_state();
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // A correctly-signed ticket for a user that no longer exists must
    // still be denied — identity must resolve, not just the HMAC.
    let ghost = sign_ticket_for(&app.state(), "user-deleted", 300);
    let result = list_workspaces(app.state(), ghost, "store-a".into()).await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

#[tokio::test]
async fn list_workspaces_uses_real_role_not_claimed_role() {
    let (state, _dir) = picker_state();
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // Owner's ticket → owner bypass lists the instance.
    let owner_ticket = sign_ticket_for(&app.state(), "user-owner", 300);
    let owner_rows = list_workspaces(app.state(), owner_ticket, "store-a".into())
        .await
        .unwrap();
    assert!(owner_rows.iter().any(|d| d.instance_id == "ws-a-1"));

    // Cashier's ticket → cashier role has no role_workspace_types in the
    // fresh store DB, so the listing is empty. The old command let a
    // caller CLAIM role-owner to see the instance; the new one derives
    // the role from the DB, so the cashier cannot escalate.
    let cashier_ticket = sign_ticket_for(&app.state(), "user-cashier", 300);
    let cashier_rows = list_workspaces(app.state(), cashier_ticket, "store-a".into())
        .await
        .unwrap();
    assert!(
        cashier_rows.is_empty(),
        "limited role must not see owner-level instances, got {cashier_rows:?}"
    );
}

#[tokio::test]
async fn scoped_assignment_filters_picker_workspace_list() {
    let (state, _dir) = picker_state();
    // Add a second instance of a different workspace type to store-a so
    // the workspace dimension of a scoped assignment has something to
    // filter (the owner bypass would otherwise list both).
    {
        let conn = state.db_manager.open_store("store-a").unwrap();
        let db = conn.lock().unwrap();
        Store::new(&db)
            .create_workspace_instance("ws-a-2", "kds", "store-a", "Kitchen", "", None)
            .unwrap();
    }
    // The owner keeps the legacy owner bypass (both instances would
    // list) but the assignment scope now limits the picker to the
    // store-pos workspace type (ADR #35 D5 / spec 0048).
    {
        let db = state.db.lock().await;
        Store::new(&db)
            .set_assignment(
                "user-owner",
                "role-owner",
                &AssignmentSpec {
                    scope_mode: ScopeMode::Scoped,
                    branches_all: true,
                    branches: vec![],
                    workspaces_all: false,
                    workspaces: vec!["store-pos".into()],
                },
            )
            .unwrap();
    }
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();
    let ticket = sign_ticket_for(&app.state(), "user-owner", 300);
    let rows = list_workspaces(app.state(), ticket, "store-a".into())
        .await
        .unwrap();
    assert!(
        rows.iter().any(|d| d.type_key == "store-pos"),
        "in-scope workspace type must list, got {rows:?}"
    );
    assert!(
        rows.iter().all(|d| d.type_key != "kds"),
        "out-of-scope workspace type must be hidden, got {rows:?}"
    );
}

#[tokio::test]
async fn scoped_assignment_branch_dimension_denies_out_of_scope_store() {
    let (state, _dir) = picker_state();
    // Owner scoped to branch store-a only — store-b is out of scope, so
    // listing it must yield nothing (fail closed per ADR #35 D5).
    {
        let db = state.db.lock().await;
        Store::new(&db)
            .set_assignment(
                "user-owner",
                "role-owner",
                &AssignmentSpec {
                    scope_mode: ScopeMode::Scoped,
                    branches_all: false,
                    branches: vec!["store-a".into()],
                    workspaces_all: true,
                    workspaces: vec![],
                },
            )
            .unwrap();
    }
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();
    let ticket = sign_ticket_for(&app.state(), "user-owner", 300);
    let in_scope = list_workspaces(app.state(), ticket.clone(), "store-a".into())
        .await
        .unwrap();
    assert!(in_scope.iter().any(|d| d.instance_id == "ws-a-1"));
    let out_of_scope = list_workspaces(app.state(), ticket, "store-b".into())
        .await
        .unwrap();
    assert!(
        out_of_scope.is_empty(),
        "branch out of scope must deny the whole store, got {out_of_scope:?}"
    );
}

#[tokio::test]
async fn list_workspaces_inactive_user_is_denied() {
    let (state, _dir) = picker_state();
    {
        let db = state.db.lock().await;
        db.execute("UPDATE users SET is_active = 0 WHERE id = 'user-owner'", [])
            .unwrap();
    }
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let ticket = sign_ticket_for(&app.state(), "user-owner", 300);
    let result = list_workspaces(app.state(), ticket, "store-a".into()).await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

#[tokio::test]
async fn list_workspace_screens_requires_valid_ticket() {
    let (state, _dir) = picker_state();
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let denied = list_workspace_screens(
        app.state(),
        "bogus".into(),
        "store-pos".into(),
        "store-a".into(),
    )
    .await;
    assert!(matches!(denied, Err(AppError::PermissionDenied(_))));

    let ticket = sign_ticket_for(&app.state(), "user-owner", 300);
    let allowed =
        list_workspace_screens(app.state(), ticket, "store-pos".into(), "store-a".into()).await;
    assert!(allowed.is_ok());
}

#[tokio::test]
async fn list_workspaces_for_store_scoped_rejects_invalid_session() {
    let (state, _dir) = picker_state();
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result =
        list_workspaces_for_store_scoped("missing-token".into(), "store-a".into(), app.state())
            .await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[tokio::test]
async fn list_workspaces_for_store_scoped_uses_session_role() {
    let (state, _dir) = picker_state();
    state.session_store.write().unwrap().insert(
        "cashier-token".into(),
        oz_core::session::SessionContext::new(
            "user-cashier".into(),
            "role-lite".into(),
            "terminal-1".into(),
            "store-a".into(),
            "ws-a-1".into(),
            "store-pos".into(),
            None,
            0,
        ),
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // The session token binds the real role — a limited session listing
    // store-a must not see owner-level instances (same as the ticket path).
    let rows =
        list_workspaces_for_store_scoped("cashier-token".into(), "store-a".into(), app.state())
            .await
            .unwrap();
    assert!(
        rows.is_empty(),
        "cashier session must not enumerate store-a instances, got {rows:?}"
    );
}

// ── Scoped-sessions follow-up: post-login listings ────────────────────
//
// TDD red: a scoped member must not be able to switch into an
// out-of-scope workspace type or store AFTER login. `list_workspaces_scoped`
// and `list_workspaces_for_store_scoped` must scope-filter through the
// user's assignment (ADR #35 D5 / spec 0048), mirroring the picker.
// `restaurant-pos` is used as the out-of-scope type because the Free tier
// allows it (so tier entitlement filtering cannot hide it — only the
// assignment can).

fn mint_session(state: &mut AppState, token: &str, user: &str, role: &str, store: &str) {
    state.session_store.write().unwrap().insert(
        token.into(),
        oz_core::session::SessionContext::new(
            user.into(),
            role.into(),
            "terminal-1".into(),
            store.into(),
            "ws-a-1".into(),
            "store-pos".into(),
            None,
            0,
        ),
    );
}

#[tokio::test]
async fn scoped_assignment_filters_session_workspace_listing() {
    let (mut state, _dir) = picker_state();
    // A second store-a instance of a type the Free tier ALLOWS
    // (restaurant-pos) so only the assignment scope can hide it.
    {
        let conn = state.db_manager.open_store("store-a").unwrap();
        let db = conn.lock().unwrap();
        Store::new(&db)
            .create_workspace_instance(
                "ws-a-rest",
                "restaurant-pos",
                "store-a",
                "Restaurant",
                "",
                None,
            )
            .unwrap();
    }
    // Owner scoped to workspace type `store-pos` only.
    {
        let db = state.db.lock().await;
        Store::new(&db)
            .set_assignment(
                "user-owner",
                "role-owner",
                &AssignmentSpec {
                    scope_mode: ScopeMode::Scoped,
                    branches_all: true,
                    branches: vec![],
                    workspaces_all: false,
                    workspaces: vec!["store-pos".into()],
                },
            )
            .unwrap();
    }
    mint_session(
        &mut state,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-a",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let rows = list_workspaces_scoped("owner-token".into(), app.state())
        .await
        .unwrap();
    assert!(
        rows.iter().any(|d| d.type_key == "store-pos"),
        "in-scope workspace type must list, got {rows:?}"
    );
    assert!(
        rows.iter().all(|d| d.type_key != "restaurant-pos"),
        "out-of-scope workspace type must be hidden after login, got {rows:?}"
    );
}

#[tokio::test]
async fn scoped_assignment_branch_dimension_denies_out_of_scope_store_for_session() {
    let (mut state, _dir) = picker_state();
    // Owner scoped to branch store-a only — store-b is out of scope, so
    // the terminal-management listing of store-b must yield nothing
    // (fail closed, same as the picker).
    {
        let db = state.db.lock().await;
        Store::new(&db)
            .set_assignment(
                "user-owner",
                "role-owner",
                &AssignmentSpec {
                    scope_mode: ScopeMode::Scoped,
                    branches_all: false,
                    branches: vec!["store-a".into()],
                    workspaces_all: true,
                    workspaces: vec![],
                },
            )
            .unwrap();
    }
    mint_session(
        &mut state,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-a",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let in_scope =
        list_workspaces_for_store_scoped("owner-token".into(), "store-a".into(), app.state())
            .await
            .unwrap();
    assert!(in_scope.iter().any(|d| d.instance_id == "ws-a-1"));

    let out_of_scope =
        list_workspaces_for_store_scoped("owner-token".into(), "store-b".into(), app.state())
            .await
            .unwrap();
    assert!(
        out_of_scope.is_empty(),
        "branch out of scope must deny the whole store listing, got {out_of_scope:?}"
    );
}

#[tokio::test]
async fn scoped_assignment_workspace_dimension_filters_for_store_listing() {
    let (mut state, _dir) = picker_state();
    // Same Free-tier-allowed out-of-scope type as the session listing test.
    {
        let conn = state.db_manager.open_store("store-a").unwrap();
        let db = conn.lock().unwrap();
        Store::new(&db)
            .create_workspace_instance(
                "ws-a-rest",
                "restaurant-pos",
                "store-a",
                "Restaurant",
                "",
                None,
            )
            .unwrap();
    }
    // Owner scoped to workspace type `store-pos` only.
    {
        let db = state.db.lock().await;
        Store::new(&db)
            .set_assignment(
                "user-owner",
                "role-owner",
                &AssignmentSpec {
                    scope_mode: ScopeMode::Scoped,
                    branches_all: true,
                    branches: vec![],
                    workspaces_all: false,
                    workspaces: vec!["store-pos".into()],
                },
            )
            .unwrap();
    }
    mint_session(
        &mut state,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-a",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let rows =
        list_workspaces_for_store_scoped("owner-token".into(), "store-a".into(), app.state())
            .await
            .unwrap();
    assert!(
        rows.iter().any(|d| d.type_key == "store-pos"),
        "in-scope workspace type must list, got {rows:?}"
    );
    assert!(
        rows.iter().all(|d| d.type_key != "restaurant-pos"),
        "out-of-scope workspace type must be hidden, got {rows:?}"
    );
}

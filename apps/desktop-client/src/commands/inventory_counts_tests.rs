use super::*;
use oz_core::session::SessionContext;
use platform_core::StoreDatabaseManager;
use tauri::Manager as _;

// ── Existing tests (preserved) ────────────────────────────────────

#[test]
fn create_args_reject_legacy_actor_field() {
    let args: CreateStockCountArgs =
        serde_json::from_str(r#"{"countType":"full","notes":"cycle","countedBy":"forged"}"#)
            .unwrap();
    assert_eq!(args.count_type, "full");
    assert_eq!(args.notes, "cycle");
}

#[test]
fn complete_args_use_camel_case() {
    let args: CompleteStockCountArgs =
        serde_json::from_str(r#"{"countId":"count-1"}"#).unwrap();
    assert_eq!(args.count_id, "count-1");
}

#[test]
fn quantity_validation_rejects_negative_values() {
    assert!(validate_quantity("counted_qty", -1).is_err());
    assert!(validate_quantity("counted_qty", 0).is_ok());
}

// ── Helpers ───────────────────────────────────────────────────────

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

fn seed_staff(conn: &rusqlite::Connection) {
    let store = Store::new(conn);
    store.seed_default_roles().unwrap();
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-staff', 'staff', 'hash', 'Staff', 'role-staff', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
    )
    .unwrap();
}

fn scoped_state(
    conn: rusqlite::Connection,
    token: &str,
    user_id: &str,
    role_id: &str,
    store_id: &str,
) -> AppState {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut state = AppState::for_test_with_conn(conn);
    state.db_manager =
        StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
    state.session_store.write().unwrap().insert(
        token.into(),
        SessionContext::new(
            user_id.into(),
            role_id.into(),
            "terminal-1".into(),
            store_id.into(),
            "instance-1".into(),
            "pos".into(),
            None,
            0,
        ),
    );
    state
}

fn make_count_args(count_type: &str) -> CreateStockCountArgs {
    CreateStockCountArgs {
        count_type: count_type.into(),
        notes: format!("Test count: {count_type}"),
    }
}

fn create_product_in_store(state: &AppState, sku: &str, name: &str) {
    let store_db = state.db_manager.open_store("s1").unwrap();
    let db = store_db.lock().unwrap();
    let s = Store::new(&db);
    s.create_product(
        sku,
        name,
        oz_core::Money {
            minor_units: 1000,
            currency: "USD".parse().unwrap(),
        },
        None,
        None,
        10,
        None,
    )
    .unwrap();
}

// ── Session validation ────────────────────────────────────────────

#[tokio::test]
async fn scoped_list_stock_counts_rejects_invalid_token() {
    let conn = oz_core::migrations::fresh_db();
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = list_stock_counts_scoped("bad-token".into(), app.state()).await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[tokio::test]
async fn scoped_get_stock_count_rejects_invalid_token() {
    let conn = oz_core::migrations::fresh_db();
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = get_stock_count_scoped("bad-token".into(), "count-1".into(), app.state()).await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

// ── Permission matrix: owner (has INVENTORY_COUNT) ────────────────

#[tokio::test]
async fn owner_can_create_stock_count() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result =
        create_stock_count_scoped("tok".into(), make_count_args("full"), app.state()).await;
    assert!(result.is_ok(), "owner should create a stock count");
    let c = result.unwrap();
    assert_eq!(c.status, "draft");
    assert_eq!(c.count_type, "full");
}

#[tokio::test]
async fn owner_can_get_stock_count_by_id() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let created =
        create_stock_count_scoped("tok".into(), make_count_args("cyclic"), app.state())
            .await
            .unwrap();
    let fetched = get_stock_count_scoped("tok".into(), created.id.clone(), app.state()).await;
    assert!(fetched.is_ok());
    assert!(fetched.unwrap().is_some());
}

#[tokio::test]
async fn owner_can_list_stock_counts() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    create_stock_count_scoped("tok".into(), make_count_args("full"), app.state())
        .await
        .unwrap();
    create_stock_count_scoped("tok".into(), make_count_args("cyclic"), app.state())
        .await
        .unwrap();

    let counts = list_stock_counts_scoped("tok".into(), app.state()).await.unwrap();
    assert_eq!(counts.len(), 2);
}

#[tokio::test]
async fn owner_can_add_count_line() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    create_product_in_store(&state, "WG-001", "Widget");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let count =
        create_stock_count_scoped("tok".into(), make_count_args("full"), app.state())
            .await
            .unwrap();

    let args = AddCountLineArgs {
        count_id: count.id.clone(),
        sku: "WG-001".into(),
        product_name: "Widget".into(),
        expected_qty: 10,
    };
    let result = add_count_line_scoped("tok".into(), args, app.state()).await;
    assert!(result.is_ok(), "owner should add a count line");
}

#[tokio::test]
async fn owner_can_get_count_lines() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    create_product_in_store(&state, "WG-001", "Widget");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let count =
        create_stock_count_scoped("tok".into(), make_count_args("full"), app.state())
            .await
            .unwrap();

    add_count_line_scoped(
        "tok".into(),
        AddCountLineArgs {
            count_id: count.id.clone(),
            sku: "WG-001".into(),
            product_name: "Widget".into(),
            expected_qty: 10,
        },
        app.state(),
    )
    .await
    .unwrap();

    let lines = get_count_lines_scoped("tok".into(), count.id, app.state()).await;
    assert!(lines.is_ok());
    assert_eq!(lines.unwrap().len(), 1);
}

#[tokio::test]
async fn owner_can_update_count_line() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    create_product_in_store(&state, "WG-001", "Widget");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let count =
        create_stock_count_scoped("tok".into(), make_count_args("full"), app.state())
            .await
            .unwrap();

    let line = add_count_line_scoped(
        "tok".into(),
        AddCountLineArgs {
            count_id: count.id.clone(),
            sku: "WG-001".into(),
            product_name: "Widget".into(),
            expected_qty: 10,
        },
        app.state(),
    )
    .await
    .unwrap();

    let result = update_count_line_scoped(
        "tok".into(),
        UpdateCountLineArgs {
            line_id: line.id.clone(),
            counted_qty: Some(8),
            notes: "2 missing".into(),
        },
        app.state(),
    )
    .await;
    assert!(result.is_ok(), "owner should update a count line");
}

#[tokio::test]
async fn owner_can_remove_count_line() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    create_product_in_store(&state, "WG-001", "Widget");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let count =
        create_stock_count_scoped("tok".into(), make_count_args("full"), app.state())
            .await
            .unwrap();

    let line = add_count_line_scoped(
        "tok".into(),
        AddCountLineArgs {
            count_id: count.id.clone(),
            sku: "WG-001".into(),
            product_name: "Widget".into(),
            expected_qty: 10,
        },
        app.state(),
    )
    .await
    .unwrap();

    let result =
        remove_count_line_scoped("tok".into(), RemoveCountLineArgs { line_id: line.id }, app.state()).await;
    assert!(result.is_ok(), "owner should remove a count line");

    let lines = get_count_lines_scoped("tok".into(), count.id, app.state())
        .await
        .unwrap();
    assert!(lines.is_empty(), "removed line should not exist");
}

#[tokio::test]
async fn owner_can_complete_stock_count() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let count =
        create_stock_count_scoped("tok".into(), make_count_args("full"), app.state())
            .await
            .unwrap();

    let result = complete_stock_count_scoped(
        "tok".into(),
        CompleteStockCountArgs {
            count_id: count.id.clone(),
        },
        app.state(),
    )
    .await;
    assert!(result.is_ok(), "owner should complete a stock count");

    let completed = get_stock_count_scoped("tok".into(), count.id, app.state())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(completed.status, "completed");
}

#[tokio::test]
async fn owner_can_list_stock_adjustments() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // No adjustments yet — should return empty list.
    let result = list_stock_adjustments_scoped("tok".into(), app.state()).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

// ── Permission matrix: staff (no INVENTORY_COUNT) ─────────────────

#[tokio::test]
async fn staff_denied_create_stock_count() {
    let conn = oz_core::migrations::fresh_db();
    seed_staff(&conn);
    let state = scoped_state(conn, "tok", "user-staff", "role-staff", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result =
        create_stock_count_scoped("tok".into(), make_count_args("full"), app.state()).await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

#[tokio::test]
async fn staff_denied_add_count_line() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    seed_staff(&conn);
    let state = scoped_state(conn, "tok", "user-staff", "role-staff", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = add_count_line_scoped(
        "tok".into(),
        AddCountLineArgs {
            count_id: "nonexistent".into(),
            sku: "WG-001".into(),
            product_name: "Widget".into(),
            expected_qty: 10,
        },
        app.state(),
    )
    .await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

#[tokio::test]
async fn staff_denied_complete_stock_count() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    seed_staff(&conn);
    let state = scoped_state(conn, "tok", "user-staff", "role-staff", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = complete_stock_count_scoped(
        "tok".into(),
        CompleteStockCountArgs {
            count_id: "nonexistent".into(),
        },
        app.state(),
    )
    .await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

// ── Edge cases ────────────────────────────────────────────────────

#[tokio::test]
async fn list_stock_counts_empty_when_none() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let counts = list_stock_counts_scoped("tok".into(), app.state()).await.unwrap();
    assert!(counts.is_empty());
}

#[tokio::test]
async fn get_stock_count_returns_none_for_unknown() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result =
        get_stock_count_scoped("tok".into(), "nonexistent-id".into(), app.state()).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn create_stock_count_validates_count_type() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner(&conn);
    let state = scoped_state(conn, "tok", "user-owner", "role-owner", "s1");
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result =
        create_stock_count_scoped("tok".into(), make_count_args("invalid_type"), app.state()).await;
    assert!(result.is_err(), "invalid count type should be rejected");
}

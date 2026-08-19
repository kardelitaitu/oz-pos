use super::*;
use oz_core::session::SessionContext;
use platform_core::StoreDatabaseManager;
use tauri::Manager as _;

fn price(minor: i64) -> oz_core::Money {
    oz_core::Money {
        minor_units: minor,
        currency: "USD".parse().unwrap(),
    }
}

/// Seed a user with inventory:view but NOT inventory:locations_manage.
/// The new role-staff preset grants both, so a limited user must use a
/// custom role (0048 retirement sweep).
fn seed_cashier_user(conn: &rusqlite::Connection) {
    let store = Store::new(conn);
    store.seed_default_roles().unwrap();
    conn.execute_batch(
        "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
            ('role-lite', 'Lite', 'Limited inventory view', '[\"inventory:view\"]', '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');
         INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-lite', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
    )
    .unwrap();
}

fn seed_owner_user(conn: &rusqlite::Connection) {
    let store = Store::new(conn);
    store.seed_default_roles().unwrap();
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
    )
    .unwrap();
}

fn scoped_state_with_token(
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

// ── LOC-06: least-privilege permission matrix ──────────────────────

#[tokio::test]
async fn cashier_can_list_locations_but_cannot_create_them() {
    // The limited role has INVENTORY_VIEW (list is allowed) but must NOT
    // hold INVENTORY_LOCATIONS_MANAGE (create/rename/deactivate/rebind
    // are management capabilities, not sales side-effects).
    let conn = oz_core::migrations::fresh_db();
    seed_cashier_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "cashier-token",
        "user-cashier",
        "role-lite",
        "store-cashier",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // Read: cashier is allowed. Migrations seed two default locations,
    // so the list is non-empty — the point is the read path works.
    let listed = list_inventory_locations("cashier-token".into(), app.state())
        .await
        .unwrap();
    assert!(
        listed.iter().any(|l| l.name == "Default Inventory"),
        "cashier should be able to list seeded locations"
    );

    // Mutation: cashier is denied with PermissionDenied.
    let created = create_inventory_location(
        "cashier-token".into(),
        "Rogue Loc".into(),
        "store".into(),
        String::new(),
        app.state(),
    )
    .await;
    assert!(matches!(created, Err(AppError::PermissionDenied(_))));

    // And the denied create must not have leaked a row.
    let after = list_inventory_locations("cashier-token".into(), app.state())
        .await
        .unwrap();
    assert!(
        !after.iter().any(|l| l.name == "Rogue Loc"),
        "denied create must not insert a location"
    );
}

#[tokio::test]
async fn owner_can_create_and_deactivate_locations() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let id = create_inventory_location(
        "owner-token".into(),
        "Backroom".into(),
        "warehouse".into(),
        "Secondary storage".into(),
        app.state(),
    )
    .await
    .unwrap();
    assert!(!id.is_empty());

    let deactivated = deactivate_inventory_location("owner-token".into(), id, app.state()).await;
    assert!(deactivated.is_ok());
}

#[tokio::test]
async fn sales_process_gated_inventory_commands_authorise_via_global_db() {
    // The SALES_PROCESS-gated inventory commands (shifts/transactions/
    // thresholds/alerts/pending-sale) must also authorise against the
    // GLOBAL identity DB — the store DB has no users, so a store-scoped
    // check would deny every caller with "user not found".
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let shifts = list_inventory_shifts("owner-token".into(), app.state())
        .await
        .unwrap();
    assert!(shifts.is_empty());
}

#[tokio::test]
async fn location_read_is_scoped_to_session_store() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let temp_dir = tempfile::tempdir().unwrap();
    let mut state = AppState::for_test_with_conn(conn);
    state.db_manager =
        StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
    for (token, store_id) in [("store-a-token", "store-a"), ("store-b-token", "store-b")] {
        state.session_store.write().unwrap().insert(
            token.into(),
            SessionContext::new(
                "user-owner".into(),
                "role-owner".into(),
                "terminal-1".into(),
                store_id.into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );
    }

    // Seed a location ONLY into store A's database. The guard is scoped
    // to a block so it drops before the async commands below.
    {
        let store_a_conn = state.db_manager.open_store("store-a").unwrap();
        let store_a_db = store_a_conn.lock().unwrap();
        Store::new(&store_a_db)
            .create_inventory_location("Store A Only", "warehouse", "")
            .unwrap();
    }

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let store_a = list_inventory_locations("store-a-token".into(), app.state())
        .await
        .unwrap();
    let store_b = list_inventory_locations("store-b-token".into(), app.state())
        .await
        .unwrap();
    assert!(
        store_a.iter().any(|l| l.name == "Store A Only"),
        "store A must see its own location"
    );
    assert!(
        !store_b.iter().any(|l| l.name == "Store A Only"),
        "store B must not see store A locations"
    );
}

// ── LOC-07: update inventory location ──────────────────────────────

#[tokio::test]
async fn owner_can_update_location_name_and_type() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let id = create_inventory_location(
        "owner-token".into(),
        "Original".into(),
        "warehouse".into(),
        String::new(),
        app.state(),
    )
    .await
    .unwrap();

    update_inventory_location(
        "owner-token".into(),
        id.clone(),
        "Renamed".into(),
        "shelf".into(),
        "".into(),
        app.state(),
    )
    .await
    .unwrap();

    let listed = list_inventory_locations("owner-token".into(), app.state())
        .await
        .unwrap();
    let loc = listed.iter().find(|l| l.id == id).unwrap();
    assert_eq!(loc.name, "Renamed");
    assert_eq!(loc.location_type, "shelf");
}

#[tokio::test]
async fn cashier_cannot_update_location() {
    let conn = oz_core::migrations::fresh_db();
    seed_cashier_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "cashier-token",
        "user-cashier",
        "role-lite",
        "store-cashier",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // First create a location as owner to have something to update.
    let owner_conn = oz_core::migrations::fresh_db();
    seed_owner_user(&owner_conn);
    let owner_state = scoped_state_with_token(
        owner_conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    let owner_app = tauri::test::mock_builder()
        .manage(owner_state)
        .build(tauri::generate_context!())
        .unwrap();
    let id = create_inventory_location(
        "owner-token".into(),
        "Target".into(),
        "warehouse".into(),
        String::new(),
        owner_app.state(),
    )
    .await
    .unwrap();

    let result = update_inventory_location(
        "cashier-token".into(),
        id,
        "Hacked".into(),
        "warehouse".into(),
        "".into(),
        app.state(),
    )
    .await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

// ── SHIFT-01: inventory shift lifecycle ────────────────────────────

#[tokio::test]
async fn owner_can_start_and_end_inventory_shift() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // Create a location first.
    let loc_id = create_inventory_location(
        "owner-token".into(),
        "Warehouse".into(),
        "warehouse".into(),
        String::new(),
        app.state(),
    )
    .await
    .unwrap();

    // Start shift.
    let shift = start_inventory_shift(
        "owner-token".into(),
        loc_id,
        "Morning count".into(),
        app.state(),
    )
    .await
    .unwrap();
    assert!(!shift.id.is_empty());

    // Active shift should exist.
    let active = get_active_inventory_shift("owner-token".into(), app.state())
        .await
        .unwrap();
    assert!(active.is_some());
    assert_eq!(active.unwrap().id, shift.id);

    // End shift.
    end_inventory_shift("owner-token".into(), shift.id, app.state())
        .await
        .unwrap();

    // No active shift after ending.
    let after = get_active_inventory_shift("owner-token".into(), app.state())
        .await
        .unwrap();
    assert!(after.is_none());
}

// ── THRESHOLD-01: stock threshold management ───────────────────────

#[tokio::test]
async fn owner_can_set_and_list_stock_thresholds() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);

    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );

    // Create a product in the store-DB managed by db_manager (not the
    // raw conn) because set_stock_threshold opens the store via
    // db_manager.open_store("store-owner").
    let product_id = {
        let store_db_arc = state.db_manager.open_store("store-owner").unwrap();
        let db = store_db_arc.lock().unwrap();
        let store = Store::new(&db);
        let p = store
            .create_product("WG-001", "Widget", price(1000), None, None, 0, None)
            .unwrap();
        p.id.clone()
    };

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // stock_thresholds.product_id is a UUID FK — pass the real id, not SKU.
    set_stock_threshold(
        "owner-token".into(),
        product_id.clone(),
        None,
        5,
        true,
        app.state(),
    )
    .await
    .unwrap();

    let thresholds = get_stock_thresholds("owner-token".into(), None, app.state())
        .await
        .unwrap();
    assert!(!thresholds.is_empty());
}

// ── list_inventory_shifts ──────────────────────────────────────────

#[tokio::test]
async fn owner_can_list_inventory_shifts() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let shifts = list_inventory_shifts("owner-token".into(), app.state())
        .await
        .unwrap();
    assert!(shifts.is_empty(), "no shifts yet");
}

// ── list_inventory_transactions ────────────────────────────────────

#[tokio::test]
async fn owner_can_list_inventory_transactions() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let txns = list_inventory_transactions("owner-token".into(), app.state())
        .await
        .unwrap();
    assert!(txns.is_empty(), "no transactions yet");
}

// ── delete_stock_threshold ─────────────────────────────────────────

#[tokio::test]
async fn owner_can_delete_stock_threshold() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    // Create a product in the store-DB before moving state into the app.
    let product_id = {
        let store_db_arc = state.db_manager.open_store("store-owner").unwrap();
        let db = store_db_arc.lock().unwrap();
        let store = Store::new(&db);
        let p = store
            .create_product("WG-001", "Widget", price(1000), None, None, 10, None)
            .unwrap();
        p.id
    };
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // Create a threshold using the product's UUID.
    set_stock_threshold(
        "owner-token".into(),
        product_id.clone(),
        None,
        5,
        true,
        app.state(),
    )
    .await
    .unwrap();

    let thresholds = get_stock_thresholds("owner-token".into(), None, app.state())
        .await
        .unwrap();
    let threshold_id = thresholds[0].id.clone();

    let result = delete_stock_threshold("owner-token".into(), threshold_id, app.state()).await;
    assert!(result.is_ok(), "owner should delete a threshold");

    let after = get_stock_thresholds("owner-token".into(), None, app.state())
        .await
        .unwrap();
    assert!(after.is_empty(), "threshold should be gone");
}

// ── get_low_stock_alerts_at_location_scoped ────────────────────────

#[tokio::test]
async fn owner_can_get_low_stock_alerts() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = get_low_stock_alerts_at_location_scoped(
        "owner-token".into(),
        "loc-default".into(),
        10,
        app.state(),
    )
    .await;
    assert!(result.is_ok(), "owner should get low stock alerts");
}

// ── active_stock_alerts_scoped ─────────────────────────────────────

#[tokio::test]
async fn owner_can_get_active_stock_alerts() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result =
        active_stock_alerts_scoped("owner-token".into(), "loc-default".into(), app.state()).await;
    assert!(result.is_ok(), "owner should get active stock alerts");
    assert!(result.unwrap().is_empty());
}

// ── acknowledge_stock_alert_scoped ─────────────────────────────────

#[tokio::test]
async fn acknowledge_nonexistent_alert_returns_error() {
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "owner-token",
        "user-owner",
        "role-owner",
        "store-owner",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = acknowledge_stock_alert_scoped(
        "owner-token".into(),
        "nonexistent-alert".into(),
        app.state(),
    )
    .await;
    assert!(result.is_err(), "nonexistent alert should fail");
}

// ── Staff permission matrix ────────────────────────────────────────

#[tokio::test]
async fn cashier_denied_inventory_shifts() {
    let conn = oz_core::migrations::fresh_db();
    seed_cashier_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "cashier-token",
        "user-cashier",
        "role-lite",
        "store-cashier",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // Cashier has inventory:view but NOT SALES_PROCESS.
    let result = list_inventory_shifts("cashier-token".into(), app.state()).await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

#[tokio::test]
async fn cashier_denied_inventory_transactions() {
    let conn = oz_core::migrations::fresh_db();
    seed_cashier_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "cashier-token",
        "user-cashier",
        "role-lite",
        "store-cashier",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = list_inventory_transactions("cashier-token".into(), app.state()).await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

#[tokio::test]
async fn cashier_denied_delete_stock_threshold() {
    let conn = oz_core::migrations::fresh_db();
    seed_cashier_user(&conn);
    let state = scoped_state_with_token(
        conn,
        "cashier-token",
        "user-cashier",
        "role-lite",
        "store-cashier",
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result = delete_stock_threshold("cashier-token".into(), "any-id".into(), app.state()).await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

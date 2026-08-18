
use super::*;
use oz_core::session::SessionContext;
use platform_core::StoreDatabaseManager;
use tauri::Manager as _;

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

    let deactivated =
        deactivate_inventory_location("owner-token".into(), id, app.state()).await;
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

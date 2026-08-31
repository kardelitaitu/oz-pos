use super::*;
use oz_core::session::SessionContext;
use platform_core::StoreDatabaseManager;
use tauri::Manager as _;

// ── CategoryDto ─────────────────────────────────────────────────────

#[test]
fn category_dto_debug() {
    let dto = CategoryDto {
        id: "cat1".into(),
        name: "Drinks".into(),
        colour: "#06b6d4".into(),
        icon: "coffee".into(),
    };
    let d = format!("{dto:?}");
    assert!(d.contains("Drinks"));
    assert!(d.contains("#06b6d4"));
}

#[test]
fn category_dto_serialize() {
    let dto = CategoryDto {
        id: "cat2".into(),
        name: "Bakery".into(),
        colour: "#f59e0b".into(),
        icon: String::new(),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["name"], "Bakery");
    assert_eq!(json["icon"], "");
}

// ── CreateCategoryArgs ──────────────────────────────────────────────

#[test]
fn create_category_args_deserialize() {
    let json = r##"{"id":"cat-drinks","name":"Drinks","colour":"#06b6d4","icon":"coffee"}"##;
    let args: CreateCategoryArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.id, "cat-drinks");
    assert_eq!(args.name, "Drinks");
}

#[test]
fn create_category_args_debug() {
    let args = CreateCategoryArgs {
        id: "c".into(),
        name: "N".into(),
        colour: "#fff".into(),
        icon: "".into(),
    };
    let d = format!("{args:?}");
    assert!(d.contains("N"));
}

// ── CreateCategoryResult ────────────────────────────────────────────

#[test]
fn create_category_result_debug() {
    let result = CreateCategoryResult { id: "cat-1".into() };
    let d = format!("{result:?}");
    assert!(d.contains("cat-1"));
}

#[test]
fn create_category_result_serialize() {
    let result = CreateCategoryResult { id: "cat-2".into() };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["id"], "cat-2");
}

// ── UpdateCategoryArgs ──────────────────────────────────────────────

#[test]
fn update_category_args_deserialize() {
    let json = r##"{"id":"cat-1","name":"Updated","colour":"#111","icon":"cup"}"##;
    let args: UpdateCategoryArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.name, "Updated");
    assert_eq!(args.icon, "cup");
}

#[test]
fn update_category_args_debug() {
    let args = UpdateCategoryArgs {
        id: "x".into(),
        name: "Y".into(),
        colour: "#000".into(),
        icon: "".into(),
    };
    let d = format!("{args:?}");
    assert!(d.contains("Y"));
}

// ── UpdateCategoryResult ────────────────────────────────────────────

#[test]
fn update_category_result_serialize() {
    let result = UpdateCategoryResult { id: "cat-3".into() };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["id"], "cat-3");
}

// ── DeleteCategoryArgs ──────────────────────────────────────────────

#[test]
fn delete_category_args_deserialize() {
    let json = r#"{"id":"cat-del"}"#;
    let args: DeleteCategoryArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.id, "cat-del");
}

#[test]
fn delete_category_args_debug() {
    let args = DeleteCategoryArgs { id: "x".into() };
    let d = format!("{args:?}");
    assert!(d.contains("x"));
}

// ── Scoped-command permission + isolation (CAT-01) ─────────────────

/// Seed the GLOBAL identity DB with an owner user (all permissions).
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

fn create_args(id: &str) -> CreateCategoryArgs {
    CreateCategoryArgs {
        id: id.into(),
        name: format!("Category {id}"),
        colour: "#06b6d4".into(),
        icon: "coffee".into(),
    }
}

#[tokio::test]
async fn scoped_category_command_rejects_invalid_session() {
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test())
        .build(tauri::generate_context!())
        .unwrap();

    let result =
        create_category_scoped("missing-token".into(), create_args("c"), app.state()).await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[tokio::test]
async fn scoped_category_command_denies_user_without_permission() {
    // A narrow custom role (no products:* grants) — the new role-staff
    // preset includes products:create/update/delete, so a limited user
    // must use a custom role instead (0048 retirement sweep).
    let conn = oz_core::migrations::fresh_db();
    let store = Store::new(&conn);
    store.seed_default_roles().unwrap();
    conn.execute_batch(
        "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
            ('role-lite', 'Lite', 'Limited', '[\"sales:view\"]', '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');
         INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-lite', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
    )
    .unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let mut state = AppState::for_test_with_conn(conn);
    state.db_manager =
        StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
    state.session_store.write().unwrap().insert(
        "cashier-token".into(),
        SessionContext::new(
            "user-cashier".into(),
            "role-lite".into(),
            "terminal-1".into(),
            "store-cashier".into(),
            "instance-1".into(),
            "pos".into(),
            None,
            0,
        ),
    );
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let result =
        create_category_scoped("cashier-token".into(), create_args("c"), app.state()).await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

#[tokio::test]
async fn scoped_category_write_command_targets_only_the_session_store() {
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

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    // Create a category ONLY in store A's database.
    create_category_scoped("store-a-token".into(), create_args("cat-a"), app.state())
        .await
        .unwrap();

    let store_a = list_categories_scoped("store-a-token".into(), app.state())
        .await
        .unwrap();
    let store_b = list_categories_scoped("store-b-token".into(), app.state())
        .await
        .unwrap();
    assert_eq!(store_a.len(), 1);
    assert_eq!(store_a[0].id, "cat-a");
    assert!(
        store_b.is_empty(),
        "store B must not see store A category data"
    );
}

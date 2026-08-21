use super::*;
use oz_core::session::SessionContext;
use platform_core::StoreDatabaseManager;
use tauri::Manager as _;

// ── TaxRateDto ──────────────────────────────────────────────────────

#[test]
fn tax_rate_dto_debug() {
    let dto = TaxRateDto {
        id: "t1".into(),
        name: "VAT".into(),
        rate_bps: 1100,
        is_default: true,
        is_inclusive: false,
        display_rate: "11.00%".into(),
        created_at: "2025-01-01".into(),
        updated_at: "2025-01-01".into(),
    };
    let d = format!("{dto:?}");
    assert!(d.contains("VAT"));
    assert!(d.contains("1100"));
}

#[test]
fn tax_rate_dto_serialize() {
    let dto = TaxRateDto {
        id: "t2".into(),
        name: "GST".into(),
        rate_bps: 1000,
        is_default: false,
        is_inclusive: true,
        display_rate: "10.00%".into(),
        created_at: "2025-02-01".into(),
        updated_at: "2025-02-01".into(),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["name"], "GST");
    assert_eq!(json["is_inclusive"], true);
}

// ── CreateTaxRateArgs ───────────────────────────────────────────────

#[test]
fn create_tax_rate_args_deserialize_camel_case() {
    // Wire contract is camelCase (frontend sends rateBps/isDefault/...).
    let json = r##"{"name":"VAT","rateBps":1100,"isDefault":true,"isInclusive":false}"##;
    let args: CreateTaxRateArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.name, "VAT");
    assert_eq!(args.rate_bps, 1100);
    assert!(args.is_default);
    assert!(!args.is_inclusive);
}

#[test]
fn create_tax_rate_args_debug() {
    let args = CreateTaxRateArgs {
        name: "T".into(),
        rate_bps: 500,
        is_default: false,
        is_inclusive: false,
    };
    let d = format!("{args:?}");
    assert!(d.contains("T"));
}

// ── UpdateTaxRateArgs ───────────────────────────────────────────────

#[test]
fn update_tax_rate_args_deserialize_camel_case() {
    let json =
        r##"{"id":"t1","name":"VAT Updated","rateBps":1200,"isDefault":false,"isInclusive":true}"##;
    let args: UpdateTaxRateArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.id, "t1");
    assert_eq!(args.rate_bps, 1200);
    assert!(args.is_inclusive);
}

#[test]
fn update_tax_rate_args_debug() {
    let args = UpdateTaxRateArgs {
        id: "x".into(),
        name: "N".into(),
        rate_bps: 0,
        is_default: true,
        is_inclusive: false,
    };
    let d = format!("{args:?}");
    assert!(d.contains("N"));
}

// ── SetCategoryTaxRatesArgs ─────────────────────────────────────────

#[test]
fn set_category_tax_rates_args_deserialize() {
    let json = r##"{"category_id":"cat1","tax_rate_ids":["t1","t2"]}"##;
    let args: SetCategoryTaxRatesArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.category_id, "cat1");
    assert_eq!(args.tax_rate_ids, vec!["t1", "t2"]);
}

#[test]
fn set_category_tax_rates_args_deserialize_empty() {
    let json = r##"{"category_id":"cat2","tax_rate_ids":[]}"##;
    let args: SetCategoryTaxRatesArgs = serde_json::from_str(json).unwrap();
    assert!(args.tax_rate_ids.is_empty());
}

#[test]
fn set_category_tax_rates_args_debug() {
    let args = SetCategoryTaxRatesArgs {
        category_id: "c".into(),
        tax_rate_ids: vec!["t1".into()],
    };
    let d = format!("{args:?}");
    assert!(d.contains("c"));
}

// ── CategoryTaxRateRow ──────────────────────────────────────────────

#[test]
fn category_tax_rate_row_debug() {
    let row = CategoryTaxRateRow {
        category_id: "cat1".into(),
        tax_rate_ids: vec!["t1".into()],
    };
    let d = format!("{row:?}");
    assert!(d.contains("cat1"));
}

#[test]
fn category_tax_rate_row_serialize() {
    let row = CategoryTaxRateRow {
        category_id: "cat2".into(),
        tax_rate_ids: vec![],
    };
    let json = serde_json::to_value(&row).unwrap();
    assert_eq!(json["category_id"], "cat2");
    assert!(json["tax_rate_ids"].as_array().unwrap().is_empty());
}

// ── Scoped-command permission + isolation (Phase 5) ─────────────────

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

#[tokio::test]
async fn require_tax_permission_uses_global_identity_db() {
    // Users/roles are GLOBAL authentication records (ADR #4/#7); the
    // store-scoped DBs have no users, so a permission check against the
    // global DB must succeed for an owner while the store DB alone
    // would report "user not found".
    let conn = oz_core::migrations::fresh_db();
    seed_owner_user(&conn);
    let state = AppState::for_test_with_conn(conn);

    assert!(
        require_tax_permission(&state, "user-owner", oz_core::permissions::SETTINGS_READ)
            .await
            .is_ok()
    );
    assert!(
        require_tax_permission(&state, "user-owner", oz_core::permissions::SETTINGS_EDIT)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn require_tax_permission_rejects_missing_user() {
    let conn = oz_core::migrations::fresh_db();
    let state = AppState::for_test_with_conn(conn);

    assert!(matches!(
        require_tax_permission(&state, "missing-user", oz_core::permissions::SETTINGS_READ).await,
        Err(AppError::PermissionDenied(_))
    ));
}

#[tokio::test]
async fn scoped_tax_command_rejects_invalid_session() {
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test())
        .build(tauri::generate_context!())
        .unwrap();

    let result = list_tax_rates_scoped("missing-token".into(), app.state()).await;
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[tokio::test]
async fn scoped_tax_command_denies_user_without_permission() {
    let conn = oz_core::migrations::fresh_db();
    let store = Store::new(&conn);
    store.seed_default_roles().unwrap();
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-staff', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
        [],
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
            "role-staff".into(),
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

    // Cashier role lacks SETTINGS_READ → PermissionDenied from the
    // GLOBAL identity DB (not "user not found" from an empty store DB).
    let result = list_tax_rates_scoped("cashier-token".into(), app.state()).await;
    assert!(matches!(result, Err(AppError::PermissionDenied(_))));
}

#[tokio::test]
async fn scoped_tax_command_reads_only_the_session_store() {
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

    // Seed a tax rate ONLY into store A's database. The guard is
    // scoped to a block so it drops before the async commands below.
    {
        let store_a_conn = state.db_manager.open_store("store-a").unwrap();
        let store_a_db = store_a_conn.lock().unwrap();
        Store::new(&store_a_db)
            .create_tax_rate("Store A VAT", 1000, true, false)
            .unwrap();
    }

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap();

    let store_a_rates = list_tax_rates_scoped("store-a-token".into(), app.state())
        .await
        .unwrap();
    let store_b_rates = list_tax_rates_scoped("store-b-token".into(), app.state())
        .await
        .unwrap();
    assert_eq!(store_a_rates.len(), 1);
    assert_eq!(store_a_rates[0].name, "Store A VAT");
    assert!(
        store_b_rates.is_empty(),
        "store B must not see store A tax data"
    );
}

#[tokio::test]
async fn scoped_tax_write_command_targets_only_the_session_store() {
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

    let created = create_tax_rate_scoped(
        "store-a-token".into(),
        CreateTaxRateArgs {
            name: "A-only".into(),
            rate_bps: 500,
            is_default: false,
            is_inclusive: false,
        },
        app.state(),
    )
    .await
    .unwrap();
    assert_eq!(created.name, "A-only");

    let store_b_rates = list_tax_rates_scoped("store-b-token".into(), app.state())
        .await
        .unwrap();
    assert!(
        store_b_rates.is_empty(),
        "writes scoped to store A must not leak into store B"
    );
}

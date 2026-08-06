//! Tax rate configuration commands.
//!
//! These commands provide CRUD access to the `tax_rates` table and
//! category-level tax rate assignments for the TaxConfigurationScreen
//! front-end.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

/// Verify a tax permission against the global identity database.
///
/// Users and roles are global authentication records (ADR #4 / ADR #7);
/// tax business data is read from the store-scoped connection after this
/// check succeeds. Mirror of `require_loyalty_permission` in loyalty.rs.
async fn require_tax_permission(
    state: &AppState,
    user_id: &str,
    permission: &str,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, user_id, permission)
}

// ── DTOs ──────────────────────────────────────────────────────────────

/// DTO for a tax rate sent to the front-end.
#[derive(Debug, Serialize)]
pub struct TaxRateDto {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Rate Bps.
    pub rate_bps: i64,
    /// Whether this is default.
    pub is_default: bool,
    /// Whether this is inclusive.
    pub is_inclusive: bool,
    /// Display Rate.
    pub display_rate: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

fn to_dto(r: oz_core::tax_rate::TaxRate) -> TaxRateDto {
    let display_rate = r.display_rate();
    TaxRateDto {
        id: r.id,
        name: r.name,
        rate_bps: r.rate_bps,
        is_default: r.is_default,
        is_inclusive: r.is_inclusive,
        display_rate,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Createtaxrateargs.
pub struct CreateTaxRateArgs {
    /// Display name.
    pub name: String,
    /// Rate Bps.
    pub rate_bps: i64,
    /// Whether this is default.
    pub is_default: bool,
    /// Whether this is inclusive.
    pub is_inclusive: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Updatetaxrateargs.
pub struct UpdateTaxRateArgs {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Rate Bps.
    pub rate_bps: i64,
    /// Whether this is default.
    pub is_default: bool,
    /// Whether this is inclusive.
    pub is_inclusive: bool,
}

#[derive(Debug, Deserialize)]
/// Setcategorytaxratesargs.
pub struct SetCategoryTaxRatesArgs {
    /// ID of the associated category.
    pub category_id: String,
    /// Tax Rate Ids.
    pub tax_rate_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
/// Categorytaxraterow.
pub struct CategoryTaxRateRow {
    /// ID of the associated category.
    pub category_id: String,
    /// Tax Rate Ids.
    pub tax_rate_ids: Vec<String>,
}

// ── Tax Rate CRUD ─────────────────────────────────────────────────────

#[command]
/// List tax rates for the store resolved from a session token. ADR #7.
///
/// TAX-01: session-scoped read with `SETTINGS_READ` on the backend.
pub async fn list_tax_rates_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<TaxRateDto>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_tax_permission(
        &state,
        &session.user_id,
        oz_core::permissions::SETTINGS_READ,
    )
    .await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rates = store.list_tax_rates()?;
    drop(db);
    Ok(rates.into_iter().map(to_dto).collect())
}

/// Create a tax rate in the store resolved from a session token. ADR #7.
///
/// TAX-01: resolves the store from the session and enforces
/// `SETTINGS_EDIT` on the backend.
#[command]
pub async fn create_tax_rate_scoped(
    session_token: String,
    args: CreateTaxRateArgs,
    state: State<'_, AppState>,
) -> Result<TaxRateDto, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_tax_permission(
        &state,
        &session.user_id,
        oz_core::permissions::SETTINGS_EDIT,
    )
    .await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    let rate = store.create_tax_rate(
        &args.name,
        args.rate_bps,
        args.is_default,
        args.is_inclusive,
    )?;
    drop(db);
    Ok(to_dto(rate))
}

/// Update a tax rate in the store resolved from a session token. ADR #7.
///
/// TAX-01: resolves the store from the session and enforces
/// `SETTINGS_EDIT` on the backend.
#[command]
pub async fn update_tax_rate_scoped(
    session_token: String,
    args: UpdateTaxRateArgs,
    state: State<'_, AppState>,
) -> Result<TaxRateDto, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_tax_permission(
        &state,
        &session.user_id,
        oz_core::permissions::SETTINGS_EDIT,
    )
    .await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    let rate = store.update_tax_rate(
        &args.id,
        &args.name,
        args.rate_bps,
        args.is_default,
        args.is_inclusive,
    )?;
    drop(db);
    Ok(to_dto(rate))
}

/// Delete a tax rate in the store resolved from a session token. ADR #7.
///
/// TAX-01: resolves the store from the session and enforces
/// `SETTINGS_EDIT` on the backend.
#[command]
pub async fn delete_tax_rate_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_tax_permission(
        &state,
        &session.user_id,
        oz_core::permissions::SETTINGS_EDIT,
    )
    .await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    store.delete_tax_rate(&id)?;
    drop(db);
    Ok(())
}

// ── Dependency Counts (TAX-03) ───────────────────────────────────────

/// DTO for tax-rate reference counts sent to the front-end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Taxratedependencycountsdto.
pub struct TaxRateDependencyCountsDto {
    /// Number of product assignments referencing this rate.
    pub products: i64,
    /// Number of category assignments referencing this rate.
    pub categories: i64,
    /// Number of historical sale lines referencing this rate.
    pub sale_lines: i64,
}

/// Get dependency (reference) counts for a tax rate in the store resolved
/// from a session token. ADR #7.
///
/// TAX-01: session-scoped read with `SETTINGS_READ` on the backend.
/// TAX-03: the delete-confirmation UI fetches these counts before showing
/// the confirm dialog, so the operator can see exactly what archiving the
/// rate will detach (product/category assignments) and what blocks it
/// (historical sale lines).
#[command]
pub async fn get_tax_rate_dependency_counts_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<TaxRateDependencyCountsDto, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_tax_permission(
        &state,
        &session.user_id,
        oz_core::permissions::SETTINGS_READ,
    )
    .await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let counts = store.tax_rate_dependency_counts(&id)?;
    drop(db);
    Ok(TaxRateDependencyCountsDto {
        products: counts.products,
        categories: counts.categories,
        sale_lines: counts.sale_lines,
    })
}

// ── Category Tax Rates ───────────────────────────────────────────────

/// List category-to-tax-rate assignments for the store resolved from a
/// session token. ADR #7. TAX-01: session-scoped with `SETTINGS_READ`.
#[command]
pub async fn list_category_tax_rates_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<CategoryTaxRateRow>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_tax_permission(
        &state,
        &session.user_id,
        oz_core::permissions::SETTINGS_READ,
    )
    .await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let rows = run_list_category_tax_rates(&db);
    drop(db);
    rows
}

/// Business logic for listing category tax rates (extracted for testing).
fn run_list_category_tax_rates(
    db: &rusqlite::Connection,
) -> Result<Vec<CategoryTaxRateRow>, AppError> {
    let store = Store::new(db);
    let categories = store.list_categories()?;

    let mut rows = Vec::new();
    for cat in &categories {
        let ids = store.get_category_tax_rates(&cat.id)?;
        if !ids.is_empty() {
            rows.push(CategoryTaxRateRow {
                category_id: cat.id.clone(),
                tax_rate_ids: ids,
            });
        }
    }
    Ok(rows)
}

/// Set (replace) the tax rates assigned to a category in the store resolved
/// from a session token. ADR #7. TAX-01: session-scoped with `SETTINGS_EDIT`.
#[command]
pub async fn set_category_tax_rates_scoped(
    session_token: String,
    args: SetCategoryTaxRatesArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_tax_permission(
        &state,
        &session.user_id,
        oz_core::permissions::SETTINGS_EDIT,
    )
    .await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    store.set_category_tax_rates(&args.category_id, &args.tax_rate_ids)?;
    drop(db);
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
        assert_eq!(json["rate_bps"], 1000);
        assert_eq!(json["is_default"], false);
        assert_eq!(json["is_inclusive"], true);
        assert_eq!(json["display_rate"], "10.00%");
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
        assert!(d.contains("500"));
    }

    // ── UpdateTaxRateArgs ───────────────────────────────────────────────

    #[test]
    fn update_tax_rate_args_deserialize_camel_case() {
        let json = r##"{"id":"t1","name":"VAT Updated","rateBps":1200,"isDefault":false,"isInclusive":true}"##;
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
    fn set_category_tax_rates_args_deserialize_empty_ids() {
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
            tax_rate_ids: vec!["t1".into(), "t2".into()],
        };
        let d = format!("{row:?}");
        assert!(d.contains("cat1"));
        assert!(d.contains("t1"));
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
            require_tax_permission(&state, "missing-user", oz_core::permissions::SETTINGS_READ)
                .await,
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
             VALUES ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-cashier', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
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
                "role-cashier".into(),
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
}

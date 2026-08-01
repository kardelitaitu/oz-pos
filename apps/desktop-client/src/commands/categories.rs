//! Category management Tauri commands.
//!
//! Exposes `list_categories`, `create_category`, `update_category`, and
//! `delete_category` to the front-end so the Category Management UI can
//! display and manipulate product categories.

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::Store;
use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

/// A category DTO for the front-end.
#[derive(Debug, Serialize)]
pub struct CategoryDto {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Colour.
    pub colour: String,
    /// Icon.
    pub icon: String,
}

/// Fetch all categories, ordered by name.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_categories_scoped`.
#[tauri::command]
pub async fn list_categories(state: State<'_, AppState>) -> Result<Vec<CategoryDto>, AppError> {
    let db = state.db.lock().await;
    run_list_categories(&db)
}

/// Fetch all categories for the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn list_categories_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<CategoryDto>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_list_categories(&db)
}

/// Business logic for listing categories (extracted for testing).
fn run_list_categories(conn: &rusqlite::Connection) -> Result<Vec<CategoryDto>, AppError> {
    let store = Store::new(conn);
    let categories = store.list_categories()?;

    let dtos: Vec<CategoryDto> = categories
        .into_iter()
        .map(|c| CategoryDto {
            id: c.id,
            name: c.name,
            colour: c.colour,
            icon: c.icon,
        })
        .collect();

    Ok(dtos)
}

// ── Create category ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Createcategoryargs.
pub struct CreateCategoryArgs {
    /// Unique category id (e.g. "cat-drinks", "cat-bakery").
    pub id: String,
    /// Display name (must be unique across all categories).
    pub name: String,
    /// Hex colour string (e.g. "#06b6d4").
    pub colour: String,
    /// Icon identifier (e.g. a lucide icon name or empty string).
    pub icon: String,
}

#[derive(Debug, Serialize)]
/// Createcategoryresult.
pub struct CreateCategoryResult {
    /// Unique identifier.
    pub id: String,
}

/// Create category.
///
/// **Deprecated for multi-store (ADR #7):** Use `create_category_scoped`,
/// which resolves the store from the session and enforces the manager
/// permission on the session user.
#[tauri::command]
pub async fn create_category(
    args: CreateCategoryArgs,
    state: State<'_, AppState>,
) -> Result<CreateCategoryResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    store.create_category(&args.id, &args.name, &args.colour, &args.icon)?;

    Ok(CreateCategoryResult { id: args.id })
}

/// Create category in the store resolved from a session token (CAT-01).
///
/// Resolves the store from the opaque session token and enforces
/// `products:create` on the session user — mirroring the scoped product
/// commands. ADR #7.
#[tauri::command]
pub async fn create_category_scoped(
    session_token: String,
    args: CreateCategoryArgs,
    state: State<'_, AppState>,
) -> Result<CreateCategoryResult, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    // Permission is checked against the GLOBAL identity DB (ADR #4/#7);
    // the store-scoped DB has no user rows.
    require_category_permission(&state, &session.user_id, permissions::PRODUCTS_CREATE).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    store.create_category(&args.id, &args.name, &args.colour, &args.icon)?;

    Ok(CreateCategoryResult { id: args.id })
}

// ── Update category ──────────────────────────────────────────────────

/// Arguments for updating an existing category.
#[derive(Debug, Deserialize)]
pub struct UpdateCategoryArgs {
    /// Existing category id (immutable).
    pub id: String,
    /// New display name.
    pub name: String,
    /// New hex colour string.
    pub colour: String,
    /// New icon identifier.
    pub icon: String,
}

#[derive(Debug, Serialize)]
/// Updatecategoryresult.
pub struct UpdateCategoryResult {
    /// Unique identifier.
    pub id: String,
}

/// Update an existing category's name, colour, and icon.
///
/// **Deprecated for multi-store (ADR #7):** Use `update_category_scoped`.
#[tauri::command]
pub async fn update_category(
    args: UpdateCategoryArgs,
    state: State<'_, AppState>,
) -> Result<UpdateCategoryResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.update_category(&args.id, &args.name, &args.colour, &args.icon)?;
    Ok(UpdateCategoryResult { id: args.id })
}

/// Update a category in the store resolved from a session token (CAT-01).
///
/// Enforces `products:update` on the session user. ADR #7.
#[tauri::command]
pub async fn update_category_scoped(
    session_token: String,
    args: UpdateCategoryArgs,
    state: State<'_, AppState>,
) -> Result<UpdateCategoryResult, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_category_permission(&state, &session.user_id, permissions::PRODUCTS_UPDATE).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    store.update_category(&args.id, &args.name, &args.colour, &args.icon)?;
    Ok(UpdateCategoryResult { id: args.id })
}

// ── Delete category ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Deletecategoryargs.
pub struct DeleteCategoryArgs {
    /// Unique identifier.
    pub id: String,
}

/// Result of deleting a category (CAT-02).
#[derive(Debug, Serialize)]
pub struct DeleteCategoryResult {
    /// Number of products unlinked from the deleted category.
    pub affected_products: i64,
}

/// Delete category.
///
/// **Deprecated for multi-store (ADR #7):** Use `delete_category_scoped`.
#[tauri::command]
pub async fn delete_category(
    args: DeleteCategoryArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_category(&args.id)?;
    Ok(())
}

/// Delete a category in the store resolved from a session token (CAT-01/02).
///
/// Enforces `products:delete` on the session user, then deletes the
/// category with the explicit unlink policy — products in the category are
/// set to `category_id = NULL` in the same transaction, and the number of
/// unlinked products is returned to the UI. ADR #7.
#[tauri::command]
pub async fn delete_category_scoped(
    session_token: String,
    args: DeleteCategoryArgs,
    state: State<'_, AppState>,
) -> Result<DeleteCategoryResult, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_category_permission(&state, &session.user_id, permissions::PRODUCTS_DELETE).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    let affected_products = store.delete_category_with_unlink(&args.id)?;
    Ok(DeleteCategoryResult { affected_products })
}

/// Verify a category permission against the global identity database.
///
/// Users and roles are global authentication records (ADR #4 / ADR #7);
/// category business data is read from the store-scoped connection after
/// this check succeeds. Mirror of `require_tax_permission` in tax.rs.
async fn require_category_permission(
    state: &AppState,
    user_id: &str,
    permission: &str,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, user_id, permission)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::session::SessionContext;
    use platform_core::StoreDatabaseManager;
    use tauri::Manager as _;

    fn usd() -> oz_core::Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> oz_core::Money {
        oz_core::Money {
            minor_units: minor,
            currency: usd(),
        }
    }

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
        // Cashier role lacks products:create/update/delete (ROLE_PRESETS).
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

    #[tokio::test]
    async fn delete_category_scoped_reports_unlinked_products() {
        // CAT-02 contract: the command returns how many products were
        // unlinked by the transactional delete (not just ok).
        let conn = oz_core::migrations::fresh_db();
        seed_owner_user(&conn);

        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
        state.session_store.write().unwrap().insert(
            "owner-token".into(),
            SessionContext::new(
                "user-owner".into(),
                "role-owner".into(),
                "terminal-1".into(),
                "store-owner".into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );

        // Seed one category with two products in the store DB.
        {
            let store_conn = state.db_manager.open_store("store-owner").unwrap();
            let store_db = store_conn.lock().unwrap();
            let store = Store::new(&store_db);
            store
                .create_category("cat-del", "Delete Me", "#f00", "trash")
                .unwrap();
            store
                .create_product("SKU-1", "One", price(100), Some("cat-del"), None, 0, None)
                .unwrap();
            store
                .create_product("SKU-2", "Two", price(200), Some("cat-del"), None, 0, None)
                .unwrap();
        }

        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = delete_category_scoped(
            "owner-token".into(),
            DeleteCategoryArgs {
                id: "cat-del".into(),
            },
            app.state(),
        )
        .await
        .unwrap();
        assert_eq!(result.affected_products, 2);

        let remaining = list_categories_scoped("owner-token".into(), app.state())
            .await
            .unwrap();
        assert!(remaining.is_empty(), "category must be deleted");
    }
}

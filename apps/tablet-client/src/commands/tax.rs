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

#[cfg(test)] #[path = "tax_tests.rs"] mod tests;

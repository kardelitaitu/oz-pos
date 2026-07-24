//! Tax rate configuration commands.
//!
//! These commands provide CRUD access to the `tax_rates` table and
//! category-level tax rate assignments for the TaxConfigurationScreen
//! front-end.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;

use crate::error::AppError;
use crate::state::AppState;

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

/// List tax rates.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_tax_rates_scoped`.
#[command]
pub async fn list_tax_rates(state: State<'_, AppState>) -> Result<Vec<TaxRateDto>, AppError> {
    let db = state.db.lock().await;
    run_list_tax_rates(&db)
}

/// List tax rates for the store resolved from a session token. ADR #7.
#[command]
pub async fn list_tax_rates_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<TaxRateDto>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_list_tax_rates(&db)
}

/// Business logic for listing tax rates (extracted for testing).
fn run_list_tax_rates(conn: &rusqlite::Connection) -> Result<Vec<TaxRateDto>, AppError> {
    let store = Store::new(conn);
    let rates = store.list_tax_rates()?;
    Ok(rates.into_iter().map(to_dto).collect())
}

#[command]
/// Create tax rate.
pub async fn create_tax_rate(
    args: CreateTaxRateArgs,
    state: State<'_, AppState>,
) -> Result<TaxRateDto, AppError> {
    let db = state.db.lock().await;
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

#[command]
/// Update tax rate.
pub async fn update_tax_rate(
    args: UpdateTaxRateArgs,
    state: State<'_, AppState>,
) -> Result<TaxRateDto, AppError> {
    let db = state.db.lock().await;
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

#[command]
/// Delete tax rate.
pub async fn delete_tax_rate(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_tax_rate(&id)?;
    drop(db);
    Ok(())
}

// ── Category Tax Rates ───────────────────────────────────────────────

/// Get all category-to-tax-rate assignments.
/// Returns an array of { category_id, tax_rate_ids } for every category
/// that has at least one tax rate assigned.
#[command]
pub async fn list_category_tax_rates(
    state: State<'_, AppState>,
) -> Result<Vec<CategoryTaxRateRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
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
    drop(db);
    Ok(rows)
}

/// Set (replace) the tax rates assigned to a category.
#[command]
pub async fn set_category_tax_rates(
    args: SetCategoryTaxRatesArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.set_category_tax_rates(&args.category_id, &args.tax_rate_ids)?;
    drop(db);
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
    fn create_tax_rate_args_deserialize() {
        let json = r##"{"name":"VAT","rate_bps":1100,"is_default":true,"is_inclusive":false}"##;
        let args: CreateTaxRateArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "VAT");
        assert!(args.is_default);
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
    fn update_tax_rate_args_deserialize() {
        let json = r##"{"id":"t1","name":"VAT Updated","rate_bps":1200,"is_default":false,"is_inclusive":true}"##;
        let args: UpdateTaxRateArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.id, "t1");
        assert_eq!(args.rate_bps, 1200);
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
}

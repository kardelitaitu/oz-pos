//! Tax rate configuration commands.
//!
//! These commands provide CRUD access to the `tax_rates` table for
//! the TaxConfigurationScreen front-end.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;

use crate::error::AppError;
use crate::state::AppState;

/// DTO for a tax rate sent to the front-end.
#[derive(Debug, Serialize)]
pub struct TaxRateDto {
    pub id: String,
    pub name: String,
    pub rate_bps: i64,
    pub is_default: bool,
    pub display_rate: String,
    pub created_at: String,
    pub updated_at: String,
}

fn to_dto(r: oz_core::tax_rate::TaxRate) -> TaxRateDto {
    let display_rate = r.display_rate();
    TaxRateDto {
        id: r.id,
        name: r.name,
        rate_bps: r.rate_bps,
        is_default: r.is_default,
        display_rate,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTaxRateArgs {
    pub name: String,
    pub rate_bps: i64,
    pub is_default: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaxRateArgs {
    pub id: String,
    pub name: String,
    pub rate_bps: i64,
    pub is_default: bool,
}

#[command]
pub async fn list_tax_rates(
    state: State<'_, AppState>,
) -> Result<Vec<TaxRateDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rates = store.list_tax_rates()?;
    drop(db);
    Ok(rates.into_iter().map(to_dto).collect())
}

#[command]
pub async fn create_tax_rate(
    args: CreateTaxRateArgs,
    state: State<'_, AppState>,
) -> Result<TaxRateDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rate = store.create_tax_rate(&args.name, args.rate_bps, args.is_default)?;
    drop(db);
    Ok(to_dto(rate))
}

#[command]
pub async fn update_tax_rate(
    args: UpdateTaxRateArgs,
    state: State<'_, AppState>,
) -> Result<TaxRateDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rate = store.update_tax_rate(&args.id, &args.name, args.rate_bps, args.is_default)?;
    drop(db);
    Ok(to_dto(rate))
}

#[command]
pub async fn delete_tax_rate(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_tax_rate(&id)?;
    drop(db);
    Ok(())
}

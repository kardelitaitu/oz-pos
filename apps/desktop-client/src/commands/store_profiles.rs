//! Tauri commands for store-profile CRUD — multi-store management.
//!
//! Each command talks to the `Store` facade via the shared `AppState`
//! database connection.

use oz_core::StoreProfile;
use serde::{Deserialize, Serialize};
use tauri::{command, State};

use crate::error::AppError;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────

/// JSON-safe representation of a store profile for the front-end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreProfileDto {
    pub id: String,
    pub name: String,
    pub address: String,
    pub tax_id: String,
    pub currency: String,
    pub timezone: String,
    pub is_primary: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<StoreProfile> for StoreProfileDto {
    fn from(p: StoreProfile) -> Self {
        Self {
            id: p.id,
            name: p.name,
            address: p.address,
            tax_id: p.tax_id,
            currency: p.currency,
            timezone: p.timezone,
            is_primary: p.is_primary,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateStoreProfileArgs {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub tax_id: Option<String>,
    pub currency: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStoreProfileArgs {
    pub id: String,
    pub name: String,
    pub address: String,
    pub tax_id: String,
    pub currency: String,
    pub timezone: String,
}

// ── Commands ───────────────────────────────────────────────────────

/// List all store profiles.
#[command]
pub async fn list_store_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<StoreProfileDto>, AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::Store::new(&conn);
    let profiles = store.list_store_profiles()?;
    Ok(profiles.into_iter().map(StoreProfileDto::from).collect())
}

/// Get a single store profile by id.
#[command]
pub async fn get_store_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<StoreProfileDto>, AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::Store::new(&conn);
    let profile = store.get_store_profile(&id)?;
    Ok(profile.map(StoreProfileDto::from))
}

/// Get the primary store profile.
#[command]
pub async fn get_primary_store(
    state: State<'_, AppState>,
) -> Result<Option<StoreProfileDto>, AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::Store::new(&conn);
    let profile = store.get_primary_store()?;
    Ok(profile.map(StoreProfileDto::from))
}

/// Create a new store profile (non-primary by default).
#[command]
pub async fn create_store_profile(
    args: CreateStoreProfileArgs,
    state: State<'_, AppState>,
) -> Result<StoreProfileDto, AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::Store::new(&conn);
    let now = chrono::Utc::now()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let profile = StoreProfile {
        id: args.id,
        name: args.name,
        address: args.address.unwrap_or_default(),
        tax_id: args.tax_id.unwrap_or_default(),
        currency: args.currency.unwrap_or_else(|| "USD".into()),
        timezone: args.timezone.unwrap_or_else(|| "UTC".into()),
        is_primary: false,
        created_at: now.clone(),
        updated_at: now,
    };
    let created = store.create_store_profile(&profile)?;
    Ok(StoreProfileDto::from(created))
}

/// Update a store profile's mutable fields.
#[command]
pub async fn update_store_profile(
    args: UpdateStoreProfileArgs,
    state: State<'_, AppState>,
) -> Result<StoreProfileDto, AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::Store::new(&conn);
    let updated = store.update_store_profile(
        &args.id,
        &args.name,
        &args.address,
        &args.tax_id,
        &args.currency,
        &args.timezone,
    )?;
    Ok(StoreProfileDto::from(updated))
}

/// Promote a store to primary (demoting the current primary).
#[command]
pub async fn set_primary_store(
    id: String,
    state: State<'_, AppState>,
) -> Result<StoreProfileDto, AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::Store::new(&conn);
    let profile = store.set_primary_store(&id)?;
    Ok(StoreProfileDto::from(profile))
}

/// Delete a non-primary store profile.
#[command]
pub async fn delete_store_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::Store::new(&conn);
    store.delete_store_profile(&id)?;
    Ok(())
}

// The core store profile CRUD logic is tested in oz-core's
// `db::store_profiles` module (13 tests). This module only
// provides Tauri command wrappers; the facade-level tests in
// `oz-core` already validate all error paths and edge cases.

//! Brand / white-label Tauri commands (tablet mirror).
//!
//! Exposes brand settings (primary colour, logo path, store name) to the
//! front-end. Logo file-picking is unavailable on tablet (no dialog plugin).

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::command;

use oz_core::Settings;

use crate::error::AppError;
use crate::state::AppState;

/// All brand settings in one shot.
#[derive(Debug, Serialize, Deserialize)]
pub struct BrandSettingsDto {
    /// Primary brand colour as a hex string (e.g. `"#10b981"`).
    pub primary_colour: String,
    /// Filesystem path to the store logo, if set.
    pub logo_path: Option<String>,
    /// Display name shown in the header.
    pub store_name: String,
}

/// Load all brand settings at once.
#[command]
pub async fn get_brand_settings(state: State<'_, AppState>) -> Result<BrandSettingsDto, AppError> {
    let conn = state.db.lock().await;
    Ok(BrandSettingsDto {
        primary_colour: Settings::get_brand_primary_colour(&conn)?,
        logo_path: Settings::get_brand_logo_path(&conn)?,
        store_name: Settings::get_brand_store_name(&conn)?,
    })
}

/// Set the primary brand colour.
#[command]
pub async fn set_brand_primary_colour(
    colour: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    Ok(Settings::set_brand_primary_colour(&conn, &colour)?)
}

/// Set the filesystem path to the store logo.
#[command]
pub async fn set_brand_logo_path(path: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    Ok(Settings::set_brand_logo_path(&conn, &path)?)
}

/// Set the brand store display name.
#[command]
pub async fn set_brand_store_name(
    name: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    Ok(Settings::set_brand_store_name(&conn, &name)?)
}

//! Brand / white-label Tauri commands.
//!
//! Exposes brand settings (primary colour, logo path, store name) to the
//! front-end and provides a file-picker for the logo image.

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::command;
use tauri_plugin_dialog::DialogExt;

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

/// Open a native file picker filtered to image files and return the
/// chosen path, or `None` if the user cancelled.
#[command]
pub async fn pick_logo_file(app_handle: tauri::AppHandle) -> Result<Option<String>, AppError> {
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();
    app_handle
        .dialog()
        .file()
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "svg", "webp"])
        .pick_file(move |file| {
            let _ = tx.send(file);
        });
    let file = rx.await.unwrap_or(None);
    Ok(file.map(|f| f.to_string()))
}

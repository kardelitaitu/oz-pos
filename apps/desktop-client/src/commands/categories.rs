//! Category management Tauri commands.
//!
//! Exposes `list_categories`, `create_category`, and `delete_category`
//! to the front-end so the Category Management UI can display and
//! manipulate product categories.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::Store;

use crate::error::AppError;
use crate::state::AppState;

/// A category DTO for the front-end.
#[derive(Debug, Serialize)]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    pub colour: String,
}

/// Fetch all categories, ordered by name.
#[command]
pub async fn list_categories(state: State<'_, AppState>) -> Result<Vec<CategoryDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let categories = store.list_categories()?;

    let dtos: Vec<CategoryDto> = categories
        .into_iter()
        .map(|c| CategoryDto {
            id: c.id,
            name: c.name,
            colour: c.colour,
        })
        .collect();

    Ok(dtos)
}

// ── Create category ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateCategoryArgs {
    /// Unique category id (e.g. "cat-drinks", "cat-bakery").
    pub id: String,
    /// Display name (must be unique across all categories).
    pub name: String,
    /// Hex colour string (e.g. "#06b6d4").
    pub colour: String,
}

#[derive(Debug, Serialize)]
pub struct CreateCategoryResult {
    pub id: String,
}

#[command]
pub async fn create_category(
    args: CreateCategoryArgs,
    state: State<'_, AppState>,
) -> Result<CreateCategoryResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    store.create_category(&args.id, &args.name, &args.colour)?;

    Ok(CreateCategoryResult { id: args.id })
}

// ── Delete category ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeleteCategoryArgs {
    pub id: String,
}

#[command]
pub async fn delete_category(
    args: DeleteCategoryArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_category(&args.id)?;
    Ok(())
}

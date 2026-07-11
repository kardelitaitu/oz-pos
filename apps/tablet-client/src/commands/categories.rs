//! Category management Tauri commands.
//!
//! Exposes `list_categories`, `create_category`, `update_category`, and
//! `delete_category` to the front-end so the Category Management UI can
//! display and manipulate product categories.

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
    pub icon: String,
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
            icon: c.icon,
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
    /// Icon identifier (e.g. a lucide icon name or empty string).
    pub icon: String,
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
pub struct UpdateCategoryResult {
    pub id: String,
}

/// Update an existing category's name, colour, and icon.
#[command]
pub async fn update_category(
    args: UpdateCategoryArgs,
    state: State<'_, AppState>,
) -> Result<UpdateCategoryResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.update_category(&args.id, &args.name, &args.colour, &args.icon)?;
    Ok(UpdateCategoryResult { id: args.id })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_dto_debug() {
        let dto = CategoryDto {
            id: "cat-1".into(),
            name: "Drinks".into(),
            colour: "#06b6d4".into(),
            icon: "coffee".into(),
        };
        let debug = format!("{:?}", dto);
        assert!(debug.contains("Drinks"));
    }

    #[test]
    fn category_dto_serialize() {
        let dto = CategoryDto {
            id: "cat-1".into(),
            name: "Food".into(),
            colour: "#ff0000".into(),
            icon: "utensils".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["id"], "cat-1");
        assert_eq!(json["name"], "Food");
        assert_eq!(json["colour"], "#ff0000");
    }

    #[test]
    fn create_category_args_deserialize() {
        let json = r##"{"id":"cat-bakery","name":"Bakery","colour":"#f59e0b","icon":"croissant"}"##;
        let args: CreateCategoryArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.id, "cat-bakery");
        assert_eq!(args.name, "Bakery");
    }

    #[test]
    fn create_category_args_debug() {
        let args = CreateCategoryArgs {
            id: "cat-test".into(),
            name: "Test".into(),
            colour: "#000".into(),
            icon: "test".into(),
        };
        let debug = format!("{:?}", args);
        assert!(debug.contains("cat-test"));
    }

    #[test]
    fn create_category_result_debug_and_serialize() {
        let result = CreateCategoryResult {
            id: "cat-99".into(),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("cat-99"));
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["id"], "cat-99");
    }
}

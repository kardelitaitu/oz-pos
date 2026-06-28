//! Category endpoints.
//!
//! `GET /api/v1/categories` — list all product categories.

use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
    pub colour: String,
}

/// List all categories, ordered by name.
pub async fn list_categories(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let db = state.db.lock().await;
    let mut stmt = db
        .prepare("SELECT id, name, colour FROM categories ORDER BY name")
        .expect("prepare list_categories query");

    let rows = stmt
        .query_map([], |row| {
            Ok(CategoryResponse {
                id: row.get("id")?,
                name: row.get("name")?,
                colour: row.get("colour")?,
            })
        })
        .expect("execute list_categories query");

    let categories: Vec<CategoryResponse> =
        rows.map(|r| r.expect("deserialize category row")).collect();
    Json(categories)
}

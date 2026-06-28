//! Category endpoints.
//!
//! `GET /api/v1/categories` — list all product categories.

use axum::{Json, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
    pub colour: String,
}

/// List all categories. Placeholder until the DB layer is wired up.
pub async fn list_categories() -> impl IntoResponse {
    Json(Vec::<CategoryResponse>::new())
}

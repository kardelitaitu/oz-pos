//! Customer management commands — list, get, create, update, delete.
//!
//! Delegates to `oz_core::db::Store` for all CRUD operations.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::Customer;

use crate::error::AppError;
use crate::state::AppState;

// ── DTO for the front-end ───────────────────────────────────────────

/// Customer as seen by the front-end.
#[derive(Debug, Serialize)]
pub struct CustomerDto {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Customer> for CustomerDto {
    fn from(c: Customer) -> Self {
        Self {
            id: c.id,
            name: c.name,
            email: c.email,
            phone: c.phone,
            notes: c.notes,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

// ── List customers ──────────────────────────────────────────────────

#[command]
pub async fn list_customers(
    state: State<'_, AppState>,
) -> Result<Vec<CustomerDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let customers = store.list_customers()?;
    drop(db);
    Ok(customers.into_iter().map(CustomerDto::from).collect())
}

// ── Get single customer ─────────────────────────────────────────────

#[command]
pub async fn get_customer(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<CustomerDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let customer = store.get_customer(&id)?;
    drop(db);
    Ok(customer.map(CustomerDto::from))
}

// ── Create customer ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateCustomerArgs {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
}

#[command]
pub async fn create_customer(
    args: CreateCustomerArgs,
    state: State<'_, AppState>,
) -> Result<CustomerDto, AppError> {
    let name = args.name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("customer name must not be empty".into()));
    }

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let customer = store.create_customer(name, args.email.as_deref(), args.phone.as_deref(), args.notes.as_deref())?;
    drop(db);
    Ok(CustomerDto::from(customer))
}

// ── Update customer ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateCustomerArgs {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
}

#[command]
pub async fn update_customer(
    args: UpdateCustomerArgs,
    state: State<'_, AppState>,
) -> Result<CustomerDto, AppError> {
    let name = args.name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("customer name must not be empty".into()));
    }

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let customer = store.update_customer(&args.id, name, args.email.as_deref(), args.phone.as_deref(), args.notes.as_deref())?;
    drop(db);
    Ok(CustomerDto::from(customer))
}

// ── Delete customer ─────────────────────────────────────────────────

#[command]
pub async fn delete_customer(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_customer(&id)?;
    drop(db);
    Ok(())
}

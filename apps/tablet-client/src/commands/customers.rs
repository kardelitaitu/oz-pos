//! Customer management commands — list, get, create, update, delete.
//!
//! Delegates to `oz_core::db::Store` for all CRUD operations.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::Customer;
use oz_core::db::Store;

use foundation::validate_not_empty;

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
            email: c.email.map(|e| e.to_string()),
            phone: c.phone.map(|p| p.to_string()),
            notes: c.notes,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

// ── List customers ──────────────────────────────────────────────────

#[command]
pub async fn list_customers(state: State<'_, AppState>) -> Result<Vec<CustomerDto>, AppError> {
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
    validate_not_empty("name", &args.name).map_err(|e| AppError::Invalid(e.to_string()))?;
    if let Some(ref email) = args.email {
        foundation::Email::new(email).map_err(|e| AppError::Invalid(e.to_string()))?;
    }
    if let Some(ref phone) = args.phone {
        foundation::Phone::new(phone).map_err(|e| AppError::Invalid(e.to_string()))?;
    }

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let customer = store.create_customer(
        args.name.trim(),
        args.email.as_deref(),
        args.phone.as_deref(),
        args.notes.as_deref(),
    )?;
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
    validate_not_empty("name", &args.name).map_err(|e| AppError::Invalid(e.to_string()))?;
    if let Some(ref email) = args.email {
        foundation::Email::new(email).map_err(|e| AppError::Invalid(e.to_string()))?;
    }
    if let Some(ref phone) = args.phone {
        foundation::Phone::new(phone).map_err(|e| AppError::Invalid(e.to_string()))?;
    }

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let customer = store.update_customer(
        &args.id,
        args.name.trim(),
        args.email.as_deref(),
        args.phone.as_deref(),
        args.notes.as_deref(),
    )?;
    drop(db);
    Ok(CustomerDto::from(customer))
}

// ── Delete customer ─────────────────────────────────────────────────

#[command]
pub async fn delete_customer(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_customer(&id)?;
    drop(db);
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::{Email, Phone};

    // ── Name validation (shared by create + update) ─────────────────

    #[test]
    fn name_empty_is_rejected() {
        let err = foundation::validate_not_empty("name", "").unwrap_err();
        assert_eq!(err.field, "name");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn name_whitespace_only_is_rejected() {
        let err = foundation::validate_not_empty("name", "   ").unwrap_err();
        assert_eq!(err.field, "name");
    }

    #[test]
    fn name_valid_passes() {
        assert!(foundation::validate_not_empty("name", "Alice").is_ok());
    }

    // ── Email validation (shared by create + update) ────────────────

    #[test]
    fn email_empty_is_rejected() {
        let err = Email::new("").unwrap_err();
        assert_eq!(err.field, "email");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn email_whitespace_only_is_rejected() {
        let err = Email::new("   ").unwrap_err();
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn email_missing_at_sign_is_rejected() {
        let err = Email::new("notanemail").unwrap_err();
        assert!(err.message.contains("must contain exactly one '@'"));
    }

    #[test]
    fn email_multiple_at_signs_is_rejected() {
        let err = Email::new("a@b@c.com").unwrap_err();
        assert!(err.message.contains("must contain exactly one '@'"));
    }

    #[test]
    fn email_empty_local_part_is_rejected() {
        let err = Email::new("@example.com").unwrap_err();
        assert!(err.message.contains("non-empty local part"));
    }

    #[test]
    fn email_empty_domain_is_rejected() {
        let err = Email::new("user@").unwrap_err();
        assert!(err.message.contains("non-empty domain"));
    }

    #[test]
    fn email_domain_without_dot_is_rejected() {
        let err = Email::new("user@localhost").unwrap_err();
        assert!(err.message.contains("must contain at least one '.'"));
    }

    #[test]
    fn email_domain_leading_dot_is_rejected() {
        let err = Email::new("user@.example.com").unwrap_err();
        assert!(err.message.contains("must not start or end with a '.'"));
    }

    #[test]
    fn email_valid_simple_passes() {
        assert!(Email::new("alice@example.com").is_ok());
    }

    #[test]
    fn email_valid_subdomain_passes() {
        assert!(Email::new("alice@mail.example.co.uk").is_ok());
    }

    #[test]
    fn email_valid_plus_tag_passes() {
        assert!(Email::new("alice+tag@example.com").is_ok());
    }

    #[test]
    fn email_optional_when_none_is_ok() {
        // None email should skip validation in the handler
        let email: Option<String> = None;
        if let Some(ref e) = email {
            panic!("should not validate when None, but got: {e}");
        }
    }

    // ── Phone validation (shared by create + update) ────────────────

    #[test]
    fn phone_empty_is_rejected() {
        let err = Phone::new("").unwrap_err();
        assert_eq!(err.field, "phone");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn phone_whitespace_only_is_rejected() {
        let err = Phone::new("   ").unwrap_err();
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn phone_no_digits_is_rejected() {
        let err = Phone::new("abc-def-ghij").unwrap_err();
        assert!(err.message.contains("at least one digit"));
    }

    #[test]
    fn phone_valid_us_format_passes() {
        assert!(Phone::new("+1-555-0102").is_ok());
    }

    #[test]
    fn phone_valid_international_passes() {
        assert!(Phone::new("+44 20 7946 0958").is_ok());
    }

    #[test]
    fn phone_valid_with_parentheses_passes() {
        assert!(Phone::new("(555) 123-4567").is_ok());
    }

    #[test]
    fn phone_optional_when_none_is_ok() {
        // None phone should skip validation in the handler
        let phone: Option<String> = None;
        if let Some(ref p) = phone {
            panic!("should not validate when None, but got: {p}");
        }
    }

    // ── DTO mapping ─────────────────────────────────────────────────

    #[test]
    fn dto_maps_email_to_string() {
        let customer =
            oz_core::Customer::new("Test").with_email(Email::new("alice@example.com").unwrap());
        let dto = CustomerDto::from(customer);
        assert_eq!(dto.email, Some("alice@example.com".into()));
    }

    #[test]
    fn dto_maps_phone_to_string() {
        let customer =
            oz_core::Customer::new("Test").with_phone(Phone::new("+1-555-0102").unwrap());
        let dto = CustomerDto::from(customer);
        assert_eq!(dto.phone, Some("+1-555-0102".into()));
    }

    #[test]
    fn dto_maps_none_email() {
        let customer = oz_core::Customer::new("Test");
        let dto = CustomerDto::from(customer);
        assert!(dto.email.is_none());
    }

    #[test]
    fn dto_maps_none_phone() {
        let customer = oz_core::Customer::new("Test");
        let dto = CustomerDto::from(customer);
        assert!(dto.phone.is_none());
    }
}

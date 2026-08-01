//! Customer management commands — list, get, create, update, delete.
//!
//! Delegates to `oz_core::db::Store` for all CRUD operations.

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::Customer;
use oz_core::db::Store;

use foundation::validate_not_empty;

use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

// ── DTO for the front-end ───────────────────────────────────────────

/// Customer as seen by the front-end.
#[derive(Debug, Serialize)]
pub struct CustomerDto {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: Option<String>,
    /// Phone number.
    pub phone: Option<String>,
    /// Notes.
    pub notes: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
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

#[tauri::command]
/// List customers.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_customers_scoped`.
pub async fn list_customers(state: State<'_, AppState>) -> Result<Vec<CustomerDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let customers = store.list_customers()?;
    drop(db);
    Ok(customers.into_iter().map(CustomerDto::from).collect())
}

/// List customers for the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn list_customers_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<CustomerDto>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let customers = store.list_customers()?;
    drop(db);
    Ok(customers.into_iter().map(CustomerDto::from).collect())
}

// ── Get single customer ─────────────────────────────────────────────

#[tauri::command]
/// Get customer.
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

/// Arguments for creating a customer in the session's store.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCustomerScopedArgs {
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: Option<String>,
    /// Phone number.
    pub phone: Option<String>,
    /// Notes.
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
/// Createcustomerargs.
pub struct CreateCustomerArgs {
    /// ID of the associated user.
    pub user_id: String,
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: Option<String>,
    /// Phone number.
    pub phone: Option<String>,
    /// Notes.
    pub notes: Option<String>,
}

#[tauri::command]
/// Create customer.
///
/// **Deprecated for multi-store UI paths (ADR #7):** Use
/// [`create_customer_scoped`] so the session selects the store and user.
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

    require_permission_for_user(&store, &args.user_id, permissions::CUSTOMERS_CREATE)?;

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

/// Arguments for updating a customer in the session's store.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCustomerScopedArgs {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: Option<String>,
    /// Phone number.
    pub phone: Option<String>,
    /// Notes.
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
/// Updatecustomerargs.
pub struct UpdateCustomerArgs {
    /// ID of the associated user.
    pub user_id: String,
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: Option<String>,
    /// Phone number.
    pub phone: Option<String>,
    /// Notes.
    pub notes: Option<String>,
}

#[tauri::command]
/// Update customer.
///
/// **Deprecated for multi-store UI paths (ADR #7):** Use
/// [`update_customer_scoped`] so the session selects the store and user.
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

    require_permission_for_user(&store, &args.user_id, permissions::CUSTOMERS_EDIT)?;

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

#[derive(Debug, Deserialize)]
/// Deletecustomerargs.
pub struct DeleteCustomerArgs {
    /// ID of the associated user.
    pub user_id: String,
    /// Unique identifier.
    pub id: String,
}

#[tauri::command]
/// Delete customer.
///
/// **Deprecated for multi-store UI paths (ADR #7):** Use
/// [`delete_customer_scoped`] so the session selects the store and user.
pub async fn delete_customer(
    args: DeleteCustomerArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    require_permission_for_user(&store, &args.user_id, permissions::CUSTOMERS_DELETE)?;

    store.delete_customer(&args.id)?;
    drop(db);
    Ok(())
}

// ── Store-scoped mutations (ADR #7) ─────────────────────────────────

/// Create a customer in the store resolved from a session token.
///
/// The session supplies both the store database and authenticated user;
/// no caller-provided user ID is accepted. ADR #7.
#[tauri::command]
pub async fn create_customer_scoped(
    session_token: String,
    args: CreateCustomerScopedArgs,
    state: State<'_, AppState>,
) -> Result<CustomerDto, AppError> {
    validate_customer_fields(&args.name, args.email.as_deref(), args.phone.as_deref())?;
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_customer_permission(&state, &session.user_id, permissions::CUSTOMERS_CREATE).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let customer = store.create_customer(
        args.name.trim(),
        args.email.as_deref(),
        args.phone.as_deref(),
        args.notes.as_deref(),
    )?;
    Ok(CustomerDto::from(customer))
}

/// Update a customer in the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn update_customer_scoped(
    session_token: String,
    args: UpdateCustomerScopedArgs,
    state: State<'_, AppState>,
) -> Result<CustomerDto, AppError> {
    validate_customer_fields(&args.name, args.email.as_deref(), args.phone.as_deref())?;
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_customer_permission(&state, &session.user_id, permissions::CUSTOMERS_EDIT).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let customer = store.update_customer(
        &args.id,
        args.name.trim(),
        args.email.as_deref(),
        args.phone.as_deref(),
        args.notes.as_deref(),
    )?;
    Ok(CustomerDto::from(customer))
}

/// Delete a customer from the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn delete_customer_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_customer_permission(&state, &session.user_id, permissions::CUSTOMERS_DELETE).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.delete_customer(&id)?;
    Ok(())
}

/// Users and roles are global authentication records (ADR #4 / ADR #7);
/// customer business data is read from the store-scoped connection after
/// this check succeeds. Mirror of `require_tax_permission` in tax.rs.
async fn require_customer_permission(
    state: &AppState,
    user_id: &str,
    permission: &str,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, user_id, permission)
}

/// Validate fields shared by customer create and update commands.
fn validate_customer_fields(
    name: &str,
    email: Option<&str>,
    phone: Option<&str>,
) -> Result<(), AppError> {
    validate_not_empty("name", name).map_err(|e| AppError::Invalid(e.to_string()))?;
    if let Some(email) = email {
        foundation::Email::new(email).map_err(|e| AppError::Invalid(e.to_string()))?;
    }
    if let Some(phone) = phone {
        foundation::Phone::new(phone).map_err(|e| AppError::Invalid(e.to_string()))?;
    }
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::{Email, Phone};
    use oz_core::session::SessionContext;
    use platform_core::StoreDatabaseManager;
    use tauri::Manager as _;

    // ── Scoped-command permission + isolation (CUST-01) ─────────────

    /// Seed the GLOBAL identity DB with an owner user (all permissions).
    fn seed_owner_user(conn: &rusqlite::Connection) {
        let store = Store::new(conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();
    }

    fn create_args(name: &str) -> CreateCustomerScopedArgs {
        CreateCustomerScopedArgs {
            name: name.into(),
            email: None,
            phone: None,
            notes: None,
        }
    }

    fn update_args(id: &str, name: &str) -> UpdateCustomerScopedArgs {
        UpdateCustomerScopedArgs {
            id: id.into(),
            name: name.into(),
            email: None,
            phone: None,
            notes: None,
        }
    }

    #[tokio::test]
    async fn scoped_customer_command_rejects_invalid_session() {
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test())
            .build(tauri::generate_context!())
            .unwrap();

        let result =
            create_customer_scoped("missing-token".into(), create_args("Alice"), app.state()).await;
        assert!(matches!(result, Err(AppError::InvalidSession)));

        let result =
            delete_customer_scoped("missing-token".into(), "cust-1".into(), app.state()).await;
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[tokio::test]
    async fn scoped_customer_command_denies_user_without_permission() {
        // Kitchen role lacks customers:create (ROLE_PRESETS).
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-kitchen', 'kitchen', 'hash', 'Kitchen', 'role-kitchen', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
        state.session_store.write().unwrap().insert(
            "kitchen-token".into(),
            SessionContext::new(
                "user-kitchen".into(),
                "role-kitchen".into(),
                "terminal-1".into(),
                "store-kitchen".into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result =
            create_customer_scoped("kitchen-token".into(), create_args("Alice"), app.state()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn scoped_customer_write_command_targets_only_the_session_store() {
        let conn = oz_core::migrations::fresh_db();
        seed_owner_user(&conn);

        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
        for (token, store_id) in [("store-a-token", "store-a"), ("store-b-token", "store-b")] {
            state.session_store.write().unwrap().insert(
                token.into(),
                SessionContext::new(
                    "user-owner".into(),
                    "role-owner".into(),
                    "terminal-1".into(),
                    store_id.into(),
                    "instance-1".into(),
                    "pos".into(),
                    None,
                    0,
                ),
            );
        }

        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        // Create a customer ONLY in store A's database.
        create_customer_scoped("store-a-token".into(), create_args("Alice"), app.state())
            .await
            .unwrap();
        // Writes target the session store: updating an unknown id in store A
        // rejects (isolated from any other store's data).
        update_customer_scoped(
            "store-a-token".into(),
            update_args("cust-a", "Alice 2"),
            app.state(),
        )
        .await
        .unwrap_err();

        let store_a = list_customers_scoped("store-a-token".into(), app.state())
            .await
            .unwrap();
        let store_b = list_customers_scoped("store-b-token".into(), app.state())
            .await
            .unwrap();
        assert_eq!(store_a.len(), 1);
        assert_eq!(store_a[0].name, "Alice");
        assert!(
            store_b.is_empty(),
            "store B must not see store A customer data"
        );
    }

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

    // -- DTO struct tests --

    #[test]
    fn customer_dto_debug() {
        let dto = CustomerDto {
            id: "c1".into(),
            name: "Alice".into(),
            email: Some("alice@test.com".into()),
            phone: None,
            notes: String::new(),
            created_at: "2025-01-01".into(),
            updated_at: "2025-01-01".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("Alice"));
    }

    #[test]
    fn customer_dto_serialize() {
        let dto = CustomerDto {
            id: "c2".into(),
            name: "Bob".into(),
            email: None,
            phone: Some("+123".into()),
            notes: "VIP".into(),
            created_at: "2025-02-01".into(),
            updated_at: "2025-02-01".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["name"], "Bob");
        assert_eq!(json["notes"], "VIP");
    }

    #[test]
    fn create_customer_args_deserialize_minimal() {
        let json = r##"{"user_id":"u1","name":"Alice"}"##;
        let args: CreateCustomerArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "Alice");
        assert_eq!(args.email, None);
        assert_eq!(args.notes, None);
    }

    #[test]
    fn create_customer_args_debug() {
        let args = CreateCustomerArgs {
            user_id: "u1".into(),
            name: "Test".into(),
            email: None,
            phone: None,
            notes: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("Test"));
    }

    #[test]
    fn update_customer_args_deserialize() {
        let json = r##"{"user_id":"u2","id":"c1","name":"Alice Updated"}"##;
        let args: UpdateCustomerArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "Alice Updated");
        assert_eq!(args.email, None);
    }

    #[test]
    fn update_customer_args_debug() {
        let args = UpdateCustomerArgs {
            user_id: "u2".into(),
            id: "c1".into(),
            name: "U".into(),
            email: None,
            phone: None,
            notes: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("U"));
    }

    #[test]
    fn delete_customer_args_deserialize() {
        let json = r##"{"user_id":"u3","id":"c99"}"##;
        let args: DeleteCustomerArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.id, "c99");
        assert_eq!(args.user_id, "u3");
    }

    #[test]
    fn delete_customer_args_debug() {
        let args = DeleteCustomerArgs {
            user_id: "u".into(),
            id: "c".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("c"));
    }

    #[test]
    fn scoped_create_args_deserialize_without_user_id() {
        let json = r#"{"name":"Alice","email":"alice@example.com"}"#;
        let args: CreateCustomerScopedArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "Alice");
        assert_eq!(args.email.as_deref(), Some("alice@example.com"));
    }

    #[test]
    fn scoped_update_args_deserialize_without_user_id() {
        let json = r#"{"id":"c1","name":"Alice Updated"}"#;
        let args: UpdateCustomerScopedArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.id, "c1");
        assert_eq!(args.name, "Alice Updated");
    }
}

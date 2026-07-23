//! Settings Tauri commands: get and persist receipt display options.
//!
//! This module exposes the receipt-related subset of the `settings` table
//! to the front-end. Other settings (store name, currency, features) are
//! managed by the setup wizard and may be exposed here in the future.

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::command;

use std::collections::HashMap;

use oz_core::permissions;
use oz_core::{Settings, UserPreferences};

use platform_core::terminal_profile::TerminalProfile;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

// ── Receipt settings DTO ─────────────────────────────────

/// All receipt display options in one shot – the UI loads these on
/// mount and sends the whole struct back on save.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptSettingsDto {
    /// Show currency symbol prefix on amounts.
    pub show_currency: bool,
    /// Decimal separator: `"dot"`, `"comma"`, or `"none"`.
    pub decimal_separator: String,
    /// Show the tax line.
    pub show_tax: bool,
    /// Footer text (empty = disabled).
    pub footer: String,
    /// Paper width: `"standard"` or `"narrow"`.
    pub paper_width: String,
    /// Show table number on cart and receipts.
    pub show_table_number: bool,
    /// Top margin (mm).
    pub margin_top: i64,
    /// Bottom margin (mm).
    pub margin_bottom: i64,
    /// Left margin (mm).
    pub margin_left: i64,
    /// Right margin (mm).
    pub margin_right: i64,
}

// ── Get receipt settings ──────────────────────────────────

#[command]
/// Get receipt settings.
pub async fn get_receipt_settings(
    state: State<'_, AppState>,
) -> Result<ReceiptSettingsDto, AppError> {
    let conn = state.db.lock().await;
    run_get_receipt_settings(&conn)
}

/// Business logic for `get_receipt_settings` (extracted for testing).
fn run_get_receipt_settings(conn: &rusqlite::Connection) -> Result<ReceiptSettingsDto, AppError> {
    Ok(ReceiptSettingsDto {
        show_currency: Settings::get_receipt_show_currency(conn)?,
        decimal_separator: Settings::get_receipt_decimal_separator(conn)?,
        show_tax: Settings::get_receipt_show_tax(conn)?,
        footer: Settings::get_receipt_footer(conn)?,
        paper_width: Settings::get_receipt_paper_width(conn)?,
        show_table_number: Settings::get_receipt_show_table_number(conn)?,
        margin_top: Settings::get_receipt_margin_top(conn)?,
        margin_bottom: Settings::get_receipt_margin_bottom(conn)?,
        margin_left: Settings::get_receipt_margin_left(conn)?,
        margin_right: Settings::get_receipt_margin_right(conn)?,
    })
}

// ── Set receipt settings ──────────────────────────────────

/// **Deprecated — use `set_receipt_settings_scoped` (ADR #7).**
#[command]
pub async fn set_receipt_settings(
    args: ReceiptSettingsDto,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::db::Store::new(&conn);
    require_permission_for_user(&store, &user_id, permissions::SETTINGS_EDIT)?;
    run_set_receipt_settings(&conn, &args)
}

/// Set receipt settings resolved from a session token. ADR #7.
#[command]
pub async fn set_receipt_settings_scoped(
    session_token: String,
    args: ReceiptSettingsDto,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = oz_core::db::Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::SETTINGS_EDIT)?;
    run_set_receipt_settings(&db, &args)
}

/// Business logic for `set_receipt_settings` (extracted for testing).
fn run_set_receipt_settings(
    conn: &rusqlite::Connection,
    args: &ReceiptSettingsDto,
) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;

    Settings::set_receipt_show_currency(&tx, args.show_currency)?;
    Settings::set_receipt_decimal_separator(&tx, &args.decimal_separator)?;
    Settings::set_receipt_show_tax(&tx, args.show_tax)?;
    Settings::set_receipt_footer(&tx, &args.footer)?;
    Settings::set_receipt_paper_width(&tx, &args.paper_width)?;
    Settings::set_receipt_show_table_number(&tx, args.show_table_number)?;
    Settings::set_receipt_margin_top(&tx, args.margin_top)?;
    Settings::set_receipt_margin_bottom(&tx, args.margin_bottom)?;
    Settings::set_receipt_margin_left(&tx, args.margin_left)?;
    Settings::set_receipt_margin_right(&tx, args.margin_right)?;

    tx.commit()?;

    Ok(())
}

// ── Store info DTO ────────────────────────────────────────────

/// Store name, address, tax ID, currency, branch, and logo – shown on printed receipts.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreSettingsDto {
    /// Display name.
    pub name: String,
    /// Street address.
    pub address: String,
    /// ID of the associated tax.
    pub tax_id: String,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Branch.
    pub branch: String,
    /// Logo.
    pub logo: String,
}

// ── Get store settings ────────────────────────────────────────

#[command]
/// Get store settings.
pub async fn get_store_settings(state: State<'_, AppState>) -> Result<StoreSettingsDto, AppError> {
    let conn = state.db.lock().await;
    run_get_store_settings(&conn)
}

/// Business logic for `get_store_settings` (extracted for testing).
fn run_get_store_settings(conn: &rusqlite::Connection) -> Result<StoreSettingsDto, AppError> {
    Ok(StoreSettingsDto {
        name: Settings::get_store_name(conn)?.unwrap_or_default(),
        address: Settings::get_store_address(conn)?.unwrap_or_default(),
        tax_id: Settings::get_store_tax_id(conn)?.unwrap_or_default(),
        currency: Settings::get_default_currency(conn)?.unwrap_or_else(|| "IDR".into()),
        branch: Settings::get_store_branch(conn)?.unwrap_or_default(),
        logo: Settings::get_store_logo(conn)?.unwrap_or_default(),
    })
}

// ── Set store settings ────────────────────────────────────────

/// **Deprecated — use `set_store_settings_scoped` (ADR #7).**
#[command]
pub async fn set_store_settings(
    args: StoreSettingsDto,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::db::Store::new(&conn);
    require_permission_for_user(&store, &user_id, permissions::SETTINGS_EDIT)?;
    run_set_store_settings(&conn, &args)
}

/// Set store settings resolved from a session token. ADR #7.
#[command]
pub async fn set_store_settings_scoped(
    session_token: String,
    args: StoreSettingsDto,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = oz_core::db::Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::SETTINGS_EDIT)?;
    run_set_store_settings(&db, &args)
}

/// Business logic for `set_store_settings` (extracted for testing).
fn run_set_store_settings(
    conn: &rusqlite::Connection,
    args: &StoreSettingsDto,
) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;

    Settings::set_store_name(&tx, &args.name)?;
    Settings::set_store_address(&tx, &args.address)?;
    Settings::set_store_tax_id(&tx, &args.tax_id)?;
    Settings::set_default_currency(&tx, &args.currency)?;
    Settings::set_store_branch(&tx, &args.branch)?;
    Settings::set_store_logo(&tx, &args.logo)?;

    tx.commit()?;

    Ok(())
}

// ── Credit Settings DTO ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Creditsettingsdto.
pub struct CreditSettingsDto {
    /// Enabled.
    pub enabled: bool,
    /// Reminder Interval Hours.
    pub reminder_interval_hours: i64,
    /// Max Limit Minor.
    pub max_limit_minor: i64,
}

#[command]
/// Get credit settings.
pub async fn get_credit_settings(
    state: State<'_, AppState>,
) -> Result<CreditSettingsDto, AppError> {
    let conn = state.db.lock().await;
    Ok(CreditSettingsDto {
        enabled: Settings::is_credit_enabled(&conn)?,
        reminder_interval_hours: Settings::get_credit_reminder_interval(&conn)?,
        max_limit_minor: Settings::get_credit_max_limit(&conn)?,
    })
}

/// **Deprecated — use `set_credit_settings_scoped` (ADR #7).**
#[command]
pub async fn set_credit_settings(
    args: CreditSettingsDto,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::db::Store::new(&conn);
    require_permission_for_user(&store, &user_id, permissions::SETTINGS_EDIT)?;
    let tx = conn.unchecked_transaction()?;
    Settings::set_credit_enabled(&tx, args.enabled)?;
    Settings::set_credit_reminder_interval(&tx, args.reminder_interval_hours)?;
    Settings::set_credit_max_limit(&tx, args.max_limit_minor)?;
    tx.commit()?;
    Ok(())
}

/// Set credit settings resolved from a session token. ADR #7.
#[command]
pub async fn set_credit_settings_scoped(
    session_token: String,
    args: CreditSettingsDto,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = oz_core::db::Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::SETTINGS_EDIT)?;
    let tx = db.unchecked_transaction()?;
    Settings::set_credit_enabled(&tx, args.enabled)?;
    Settings::set_credit_reminder_interval(&tx, args.reminder_interval_hours)?;
    Settings::set_credit_max_limit(&tx, args.max_limit_minor)?;
    tx.commit()?;
    Ok(())
}

// ── Credit sale DTO ──────────────────────────────────────────────

/// A credit sale for the reminders list.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreditSaleDto {
    /// ID of the associated sale.
    pub sale_id: String,
    /// Customer Name.
    pub customer_name: String,
    /// Total amount in minor currency units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Settled At.
    pub settled_at: Option<String>,
    /// Cashier Name.
    pub cashier_name: String,
}

#[command]
/// List credit sales.
pub async fn list_credit_sales(state: State<'_, AppState>) -> Result<Vec<CreditSaleDto>, AppError> {
    let conn = state.db.lock().await;
    let mut stmt = conn.prepare(
        "SELECT s.id, p.gateway_reference, s.total_minor, s.currency, s.created_at,
                p.settled_at, COALESCE(u.display_name, '')
         FROM sales s
         JOIN payments p ON p.sale_id = s.id
         LEFT JOIN users u ON u.id = s.user_id
         WHERE s.status = 'completed'
           AND p.method = 'credit'
         ORDER BY s.created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(CreditSaleDto {
            sale_id: row.get(0)?,
            customer_name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            total_minor: row.get(2)?,
            currency: row.get(3)?,
            created_at: row.get(4)?,
            settled_at: row.get(5)?,
            cashier_name: row.get(6)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// **Deprecated — use `settle_credit_scoped` (ADR #7).**
#[command]
pub async fn settle_credit(
    sale_id: String,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::db::Store::new(&conn);
    require_permission_for_user(&store, &user_id, permissions::SETTINGS_EDIT)?;
    let tx = conn.unchecked_transaction()?;
    let now = chrono::Utc::now().to_rfc3339();
    tx.execute(
        "UPDATE payments SET settled_at = ?1 WHERE sale_id = ?2 AND method = 'credit'",
        rusqlite::params![now, sale_id],
    )?;
    tx.commit()?;
    Ok(())
}

/// Settle a credit sale resolved from a session token. ADR #7.
#[command]
pub async fn settle_credit_scoped(
    session_token: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = oz_core::db::Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::SETTINGS_EDIT)?;
    let tx = db.unchecked_transaction()?;
    let now = chrono::Utc::now().to_rfc3339();
    tx.execute(
        "UPDATE payments SET settled_at = ?1 WHERE sale_id = ?2 AND method = 'credit'",
        rusqlite::params![now, sale_id],
    )?;
    tx.commit()?;
    Ok(())
}

// ── Hardware settings (printer + scanner) ───────────────────────

/// Printer and scanner configuration.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareSettingsDto {
    /// Printer Connection.
    pub printer_connection: String,
    /// Printer Device Path.
    pub printer_device_path: String,
    /// Printer Paper Size.
    pub printer_paper_size: String,
    /// ID of the associated scanner device.
    pub scanner_device_id: String,
    /// Scanner Input Mode.
    pub scanner_input_mode: String,
}

impl From<TerminalProfile> for HardwareSettingsDto {
    fn from(p: TerminalProfile) -> Self {
        Self {
            printer_connection: p.printer_connection,
            printer_device_path: p.printer_device_path,
            printer_paper_size: p.printer_paper_size,
            scanner_device_id: p.scanner_device_id,
            scanner_input_mode: p.scanner_input_mode,
        }
    }
}

impl From<HardwareSettingsDto> for TerminalProfile {
    fn from(dto: HardwareSettingsDto) -> Self {
        Self {
            printer_connection: dto.printer_connection,
            printer_device_path: dto.printer_device_path,
            printer_paper_size: dto.printer_paper_size,
            scanner_device_id: dto.scanner_device_id,
            scanner_input_mode: dto.scanner_input_mode,
        }
    }
}

/// Helper: resolve the app data directory from db_path.
fn app_data_dir(state: &AppState) -> Result<std::path::PathBuf, AppError> {
    state
        .db_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| AppError::Internal("db_path has no parent directory".into()))
}

#[command]
/// Get hardware settings for the current terminal from `terminal_profiles/<id>.json`.
///
/// On first access after upgrading from a version that stored hardware
/// settings in SQLite, the old values are migrated to JSON automatically.
/// Returns defaults only when neither JSON nor SQLite has saved values.
pub async fn get_hardware_settings(
    state: State<'_, AppState>,
) -> Result<HardwareSettingsDto, AppError> {
    let terminal_id = state
        .terminal_id
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let base_dir = app_data_dir(&state)?;
    let path = TerminalProfile::profile_path(&base_dir, &terminal_id);

    // Fast path: JSON file already exists (post-migration or fresh install).
    if let Some(profile) = TerminalProfile::load(&path)? {
        return Ok(HardwareSettingsDto::from(profile));
    }

    // Migration path: no JSON file yet → read from SQLite, write to JSON.
    let conn = state.db.lock().await;
    let profile = TerminalProfile {
        printer_connection: Settings::get_printer_connection(&conn)?,
        printer_device_path: Settings::get_printer_device_path(&conn)?,
        printer_paper_size: Settings::get_printer_paper_size(&conn)?,
        scanner_device_id: Settings::get_scanner_device_id(&conn)?,
        scanner_input_mode: Settings::get_scanner_input_mode(&conn)?,
    };

    // Persist to JSON so future reads take the fast path.
    if let Err(e) = profile.save(&path) {
        tracing::warn!(
            terminal_id = %terminal_id,
            error = %e,
            "failed to migrate hardware settings to JSON — will retry next read"
        );
    }

    Ok(HardwareSettingsDto::from(profile))
}

/// **Deprecated — use `set_hardware_settings_scoped` (ADR #7).**
///
/// Now writes to `terminal_profiles/<id>.json` instead of SQLite (ADR #22).
#[command]
pub async fn set_hardware_settings(
    args: HardwareSettingsDto,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let terminal_id = state
        .terminal_id
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // Permission check still requires DB access.
    {
        let conn = state.db.lock().await;
        let store = oz_core::db::Store::new(&conn);
        require_permission_for_user(&store, &user_id, permissions::SETTINGS_EDIT)?;
    }

    let base_dir = app_data_dir(&state)?;
    let path = TerminalProfile::profile_path(&base_dir, &terminal_id);
    let profile = TerminalProfile::from(args);
    profile.save(&path)?;
    Ok(())
}

/// Set hardware settings resolved from a session token. ADR #7.
///
/// Hardware settings are now per-terminal, stored in
/// `terminal_profiles/<terminal_id>.json` (ADR #22).
#[command]
pub async fn set_hardware_settings_scoped(
    session_token: String,
    args: HardwareSettingsDto,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;

    // Extract terminal_id before locking DB (avoids Send guard across .await).
    let terminal_id = state
        .terminal_id
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // Permission check requires the store-scoped DB.
    {
        let conn = state
            .db_manager
            .open_store(&session.store_id)
            .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = oz_core::db::Store::new(&db);
        require_permission_for_user(&store, &session.user_id, permissions::SETTINGS_EDIT)?;
    }

    let base_dir = app_data_dir(&state)?;
    let path = TerminalProfile::profile_path(&base_dir, &terminal_id);
    let profile = TerminalProfile::from(args);
    profile.save(&path)?;
    Ok(())
}

// ── User preferences ───────────────────────────────────────────

/// One key-value pair within a user's preferences.
#[derive(Debug, Serialize, Deserialize)]
pub struct UserPrefEntry {
    /// Key.
    pub key: String,
    /// Value.
    pub value: String,
}

/// **Deprecated — use `get_user_preferences_scoped` (ADR #7).**
#[command]
pub async fn get_user_preferences(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, AppError> {
    let conn = state.db.lock().await;
    Ok(UserPreferences::get_all(&conn, &user_id)?)
}

/// Get user preferences resolved from a session token. ADR #7.
/// Uses `session.user_id` for the preference lookup.
#[command]
pub async fn get_user_preferences_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(UserPreferences::get_all(&db, &session.user_id)?)
}

/// **Deprecated — use `set_user_preferences_scoped` (ADR #7).**
#[command]
pub async fn set_user_preferences(
    user_id: String,
    prefs: Vec<UserPrefEntry>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    let pairs: Vec<(String, String)> = prefs.into_iter().map(|e| (e.key, e.value)).collect();
    Ok(UserPreferences::set_batch(&conn, &user_id, &pairs)?)
}

/// Set user preferences resolved from a session token. ADR #7.
/// Uses `session.user_id` for the preference write.
#[command]
pub async fn set_user_preferences_scoped(
    session_token: String,
    prefs: Vec<UserPrefEntry>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let pairs: Vec<(String, String)> = prefs.into_iter().map(|e| (e.key, e.value)).collect();
    Ok(UserPreferences::set_batch(&db, &session.user_id, &pairs)?)
}

// ── Generic key-value settings ────────────────────────────────

/// Read a single setting value by key.
///
/// Returns `None` when the key does not exist.
#[command]
pub async fn get_setting(
    key: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    let conn = state.db.lock().await;
    run_get_setting(&conn, &key)
}

/// Business logic for `get_setting` (extracted for testing).
fn run_get_setting(conn: &rusqlite::Connection, key: &str) -> Result<Option<String>, AppError> {
    Ok(Settings::get(conn, key)?)
}
/// **Deprecated — use `set_setting_scoped` (ADR #7).**
///
/// Write (or overwrite) a single setting value.
///
/// Pass an empty string to store an empty value.
#[command]
pub async fn set_setting(
    key: String,
    value: String,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Extract terminal_id first.
    let terminal_id = state
        .terminal_id
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // Scope block: drop sync guards before .await below.
    {
        let conn = state.db.lock().await;
        let store = oz_core::db::Store::new(&conn);
        require_permission_for_user(&store, &user_id, permissions::SETTINGS_EDIT)?;
        run_set_setting(&conn, &key, &value, &terminal_id)?;
    } // conn, store dropped here

    // Publish SettingsUpdated event for cross-terminal reactivity (ADR #22).
    let kernel = state.kernel.lock().await;
    let bus = kernel.event_bus();
    let event = oz_core::events::SettingsUpdated {
        changed_keys: vec![key.clone()],
        terminal_id,
    };
    if let Err(e) = bus.publish(&event) {
        tracing::warn!(key = %key, error = %e, "failed to publish SettingsUpdated event");
    }

    Ok(())
}

/// Write (or overwrite) a single setting value resolved from a session token. ADR #7.
///
/// Pass an empty string to store an empty value.
/// Writes a delta record and publishes a `SettingsUpdated` event (ADR #22).
#[command]
pub async fn set_setting_scoped(
    session_token: String,
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;

    // Extract terminal_id before locking the store DB to avoid
    // holding a non-Send MutexGuard across an .await point.
    let terminal_id = state
        .terminal_id
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // Scope block: all sync guards (MutexGuard, Store) must be
    // dropped before any .await below.
    {
        let conn = state
            .db_manager
            .open_store(&session.store_id)
            .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = oz_core::db::Store::new(&db);
        require_permission_for_user(&store, &session.user_id, permissions::SETTINGS_EDIT)?;
        run_set_setting(&db, &key, &value, &terminal_id)?;
    } // db, store, conn dropped here — safe to .await below

    // Publish SettingsUpdated event for cross-terminal reactivity (ADR #22).
    let kernel = state.kernel.lock().await;
    let bus = kernel.event_bus();
    let event = oz_core::events::SettingsUpdated {
        changed_keys: vec![key.clone()],
        terminal_id,
    };
    if let Err(e) = bus.publish(&event) {
        tracing::warn!(key = %key, error = %e, "failed to publish SettingsUpdated event");
    }

    Ok(())
}

/// Business logic for `set_setting` (extracted for testing).
/// Uses `set_tracked` so every settings change writes a delta record
/// (ADR #22).
fn run_set_setting(
    conn: &rusqlite::Connection,
    key: &str,
    value: &str,
    terminal_id: &str,
) -> Result<(), AppError> {
    Ok(Settings::set_tracked(conn, key, value, terminal_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    // ── Token rejection tests ──────────────────────────────

    #[test]
    fn settings_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    // ── Receipt settings tests ─────────────────────────────

    #[test]
    fn get_receipt_settings_returns_defaults() {
        let conn = fresh_conn();
        let result = run_get_receipt_settings(&conn).unwrap();

        assert!(!result.show_currency, "show_currency defaults to false");
        assert_eq!(result.decimal_separator, "dot");
        assert!(result.show_tax, "show_tax defaults to true");
        assert_eq!(result.footer, "");
        assert_eq!(result.paper_width, "standard");
        assert!(
            !result.show_table_number,
            "show_table_number defaults to false"
        );
        assert_eq!(result.margin_top, 0);
        assert_eq!(result.margin_bottom, 0);
        assert_eq!(result.margin_left, 0);
        assert_eq!(result.margin_right, 0);
    }

    #[test]
    fn set_receipt_settings_persists() {
        let conn = fresh_conn();
        let dto = ReceiptSettingsDto {
            show_currency: false,
            decimal_separator: "comma".into(),
            show_tax: false,
            footer: "Thanks!".into(),
            paper_width: "narrow".into(),
            show_table_number: true,
            margin_top: 5,
            margin_bottom: 3,
            margin_left: 2,
            margin_right: 2,
        };

        run_set_receipt_settings(&conn, &dto).unwrap();
        let result = run_get_receipt_settings(&conn).unwrap();

        assert!(!result.show_currency);
        assert_eq!(result.decimal_separator, "comma");
        assert!(!result.show_tax);
        assert_eq!(result.footer, "Thanks!");
        assert_eq!(result.paper_width, "narrow");
        assert!(result.show_table_number);
        assert_eq!(result.margin_top, 5);
        assert_eq!(result.margin_bottom, 3);
        assert_eq!(result.margin_left, 2);
        assert_eq!(result.margin_right, 2);
    }

    #[test]
    fn get_store_settings_returns_defaults() {
        let conn = fresh_conn();
        let result = run_get_store_settings(&conn).unwrap();

        assert_eq!(result.name, "");
        assert_eq!(result.address, "");
        assert_eq!(result.tax_id, "");
        assert_eq!(result.currency, "IDR");
        assert_eq!(result.branch, "");
        assert_eq!(result.logo, "");
    }

    #[test]
    fn set_store_settings_persists() {
        let conn = fresh_conn();
        let dto = StoreSettingsDto {
            name: "My Coffee Shop".into(),
            address: "123 Main St".into(),
            tax_id: "TAX-12345".into(),
            currency: "USD".into(),
            branch: "Downtown".into(),
            logo: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAA".into(),
        };

        run_set_store_settings(&conn, &dto).unwrap();
        let result = run_get_store_settings(&conn).unwrap();

        assert_eq!(result.name, "My Coffee Shop");
        assert_eq!(result.address, "123 Main St");
        assert_eq!(result.tax_id, "TAX-12345");
        assert_eq!(result.currency, "USD");
        assert_eq!(result.branch, "Downtown");
        assert_eq!(result.logo, "iVBORw0KGgoAAAANSUhEUgAAAAEAAAA");
    }

    #[test]
    fn set_receipt_settings_overwrites_previous() {
        let conn = fresh_conn();

        run_set_receipt_settings(
            &conn,
            &ReceiptSettingsDto {
                show_currency: true,
                decimal_separator: "dot".into(),
                show_tax: false,
                footer: "v1".into(),
                paper_width: "standard".into(),
                show_table_number: false,
                margin_top: 0,
                margin_bottom: 0,
                margin_left: 0,
                margin_right: 0,
            },
        )
        .unwrap();

        run_set_receipt_settings(
            &conn,
            &ReceiptSettingsDto {
                show_currency: false,
                decimal_separator: "comma".into(),
                show_tax: true,
                footer: "v2".into(),
                paper_width: "narrow".into(),
                show_table_number: true,
                margin_top: 10,
                margin_bottom: 5,
                margin_left: 0,
                margin_right: 0,
            },
        )
        .unwrap();

        let result = run_get_receipt_settings(&conn).unwrap();

        assert!(!result.show_currency);
        assert_eq!(result.decimal_separator, "comma");
        assert!(result.show_tax);
        assert_eq!(result.footer, "v2");
        assert_eq!(result.paper_width, "narrow");
        assert!(result.show_table_number);
        assert_eq!(result.margin_top, 10);
        assert_eq!(result.margin_bottom, 5);
        assert_eq!(result.margin_left, 0);
        assert_eq!(result.margin_right, 0);
    }

    #[test]
    fn set_store_settings_overwrites_previous() {
        let conn = fresh_conn();

        run_set_store_settings(
            &conn,
            &StoreSettingsDto {
                name: "Old Name".into(),
                address: "Old Address".into(),
                tax_id: "".into(),
                currency: "USD".into(),
                branch: "".into(),
                logo: "".into(),
            },
        )
        .unwrap();

        run_set_store_settings(
            &conn,
            &StoreSettingsDto {
                name: "New Name".into(),
                address: "New Address".into(),
                tax_id: "TAX-999".into(),
                currency: "IDR".into(),
                branch: "Mall".into(),
                logo: "logo_data".into(),
            },
        )
        .unwrap();

        let result = run_get_store_settings(&conn).unwrap();

        assert_eq!(result.name, "New Name");
        assert_eq!(result.address, "New Address");
        assert_eq!(result.tax_id, "TAX-999");
        assert_eq!(result.currency, "IDR");
        assert_eq!(result.branch, "Mall");
        assert_eq!(result.logo, "logo_data");
    }

    // -- DTO struct tests --

    #[test]
    fn receipt_settings_dto_debug() {
        let dto = ReceiptSettingsDto {
            show_currency: false,
            decimal_separator: "dot".into(),
            show_tax: true,
            footer: "Thanks".into(),
            paper_width: "standard".into(),
            show_table_number: false,
            margin_top: 0,
            margin_bottom: 0,
            margin_left: 0,
            margin_right: 0,
        };
        let d = format!("{dto:?}");
        assert!(d.contains("Thanks"));
        assert!(d.contains("dot"));
    }

    #[test]
    fn receipt_settings_dto_deserialize() {
        let json = r##"{"showCurrency":true,"decimalSeparator":"comma","showTax":false,"footer":"","paperWidth":"narrow","showTableNumber":true,"marginTop":5,"marginBottom":3,"marginLeft":2,"marginRight":2}"##;
        let dto: ReceiptSettingsDto = serde_json::from_str(json).unwrap();
        assert!(dto.show_currency);
        assert_eq!(dto.decimal_separator, "comma");
        assert_eq!(dto.margin_top, 5);
    }

    #[test]
    fn store_settings_dto_debug() {
        let dto = StoreSettingsDto {
            name: "Test Store".into(),
            address: "123 Rd".into(),
            tax_id: "T1".into(),
            currency: "IDR".into(),
            branch: "Main".into(),
            logo: String::new(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("Test Store"));
    }

    #[test]
    fn store_settings_dto_serialize() {
        let dto = StoreSettingsDto {
            name: "S".into(),
            address: "A".into(),
            tax_id: "T".into(),
            currency: "USD".into(),
            branch: "B".into(),
            logo: "L".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["name"], "S");
        assert_eq!(json["currency"], "USD");
    }

    #[test]
    fn credit_settings_dto_deserialize() {
        let json = r##"{"enabled":true,"reminderIntervalHours":24,"maxLimitMinor":500000}"##;
        let dto: CreditSettingsDto = serde_json::from_str(json).unwrap();
        assert!(dto.enabled);
        assert_eq!(dto.reminder_interval_hours, 24);
    }

    #[test]
    fn credit_settings_dto_debug() {
        let dto = CreditSettingsDto {
            enabled: false,
            reminder_interval_hours: 12,
            max_limit_minor: 100000,
        };
        let d = format!("{dto:?}");
        assert!(d.contains("100000"));
    }

    #[test]
    fn hardware_settings_dto_serialize() {
        let dto = HardwareSettingsDto {
            printer_connection: "USB".into(),
            printer_device_path: "/dev/usb/lp0".into(),
            printer_paper_size: "80mm".into(),
            scanner_device_id: "scanner-1".into(),
            scanner_input_mode: "keyboard".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["printerConnection"], "USB");
    }

    #[test]
    fn user_pref_entry_deserialize() {
        let json = r##"{"key":"theme","value":"dark"}"##;
        let entry: UserPrefEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.key, "theme");
        assert_eq!(entry.value, "dark");
    }

    // ── Generic get_setting / set_setting tests ──────────────────

    #[test]
    fn get_setting_returns_none_for_missing_key() {
        let conn = fresh_conn();
        let result = run_get_setting(&conn, "nonexistent.key").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn set_setting_persists_and_get_returns_it() {
        let conn = fresh_conn();
        run_set_setting(
            &conn,
            "payment.stripe_key",
            "sk_test_abc123",
            "test-terminal",
        )
        .unwrap();
        let result = run_get_setting(&conn, "payment.stripe_key").unwrap();
        assert_eq!(result, Some("sk_test_abc123".into()));
    }

    #[test]
    fn set_setting_overwrites_previous_value() {
        let conn = fresh_conn();
        run_set_setting(&conn, "my.key", "v1", "test-terminal").unwrap();
        run_set_setting(&conn, "my.key", "v2", "test-terminal").unwrap();
        let result = run_get_setting(&conn, "my.key").unwrap();
        assert_eq!(result, Some("v2".into()));
    }

    #[test]
    fn set_setting_empty_string_clears_value() {
        let conn = fresh_conn();
        run_set_setting(&conn, "key", "hello", "test-terminal").unwrap();
        run_set_setting(&conn, "key", "", "test-terminal").unwrap();
        let result = run_get_setting(&conn, "key").unwrap();
        assert_eq!(result, Some("".into()));
    }

    #[test]
    /// After wiring ADR #22, `run_set_setting` writes a delta row
    /// in addition to updating the settings table. This test verifies
    /// the Tauri command layer actually produces delta records.
    #[test]
    fn run_set_setting_writes_delta_row() {
        let conn = fresh_conn();
        run_set_setting(&conn, "delta.test", "delta-val", "term-delta").unwrap();
        // Settings value must be persisted.
        assert_eq!(
            Settings::get(&conn, "delta.test").unwrap(),
            Some("delta-val".into())
        );
        // Delta row must exist at version 1.
        assert_eq!(
            Settings::get_version(&conn, "delta.test", "term-delta").unwrap(),
            Some(1)
        );
    }

    fn get_setting_after_multiple_keys_only_returns_requested() {
        let conn = fresh_conn();
        run_set_setting(&conn, "a", "1", "test-terminal").unwrap();
        run_set_setting(&conn, "b", "2", "test-terminal").unwrap();
        run_set_setting(&conn, "c", "3", "test-terminal").unwrap();
        assert_eq!(run_get_setting(&conn, "b").unwrap(), Some("2".into()));
        assert_eq!(run_get_setting(&conn, "d").unwrap(), None);
    }

    // ── CamelCase serde round-trip tests ─────────────────────────

    #[test]
    fn receipt_settings_dto_serde_roundtrip() {
        let dto = ReceiptSettingsDto {
            show_currency: true,
            decimal_separator: "comma".into(),
            show_tax: false,
            footer: "Round Trip".into(),
            paper_width: "narrow".into(),
            show_table_number: true,
            margin_top: 5,
            margin_bottom: 3,
            margin_left: 2,
            margin_right: 1,
        };
        let json = serde_json::to_value(&dto).unwrap();
        let back: ReceiptSettingsDto = serde_json::from_value(json).unwrap();
        assert!(back.show_currency);
        assert_eq!(back.decimal_separator, "comma");
        assert!(!back.show_tax);
        assert_eq!(back.footer, "Round Trip");
        assert_eq!(back.paper_width, "narrow");
        assert!(back.show_table_number);
        assert_eq!(back.margin_top, 5);
    }

    #[test]
    fn store_settings_dto_serde_roundtrip() {
        let dto = StoreSettingsDto {
            name: "Round".into(),
            address: "Trip St".into(),
            tax_id: "RT-001".into(),
            currency: "EUR".into(),
            branch: "Main".into(),
            logo: "logo_data".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        let back: StoreSettingsDto = serde_json::from_value(json).unwrap();
        assert_eq!(back.name, "Round");
        assert_eq!(back.tax_id, "RT-001");
        assert_eq!(back.logo, "logo_data");
    }

    #[test]
    fn credit_settings_dto_serde_roundtrip() {
        let dto = CreditSettingsDto {
            enabled: true,
            reminder_interval_hours: 48,
            max_limit_minor: 999999,
        };
        let json = serde_json::to_value(&dto).unwrap();
        let back: CreditSettingsDto = serde_json::from_value(json).unwrap();
        assert!(back.enabled);
        assert_eq!(back.reminder_interval_hours, 48);
    }

    #[test]
    fn hardware_settings_dto_serde_roundtrip() {
        let dto = HardwareSettingsDto {
            printer_connection: "Network".into(),
            printer_device_path: "192.168.1.100".into(),
            printer_paper_size: "58mm".into(),
            scanner_device_id: "scanner-2".into(),
            scanner_input_mode: "serial".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        let back: HardwareSettingsDto = serde_json::from_value(json).unwrap();
        assert_eq!(back.printer_connection, "Network");
        assert_eq!(back.scanner_device_id, "scanner-2");
    }
}

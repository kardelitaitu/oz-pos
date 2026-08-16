//! Settings Tauri commands: get and persist receipt display options.
//!
//! This module exposes the receipt-related subset of the `settings` table
//! to the front-end. Other settings (store name, currency, features) are
//! managed by the setup wizard and may be exposed here in the future.

use serde::{Deserialize, Serialize};
use tauri::State;

use std::collections::HashMap;

use oz_core::permissions;
use oz_core::{Settings, Store, UserPreferences};

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
    /// Tax rounding mode: `"half_up"` or `"truncate"`. Default `"half_up"`.
    #[serde(default = "default_tax_rounding_mode")]
    pub tax_rounding_mode: String,
}

fn default_tax_rounding_mode() -> String {
    "half_up".to_string()
}

// ── Get receipt settings ──────────────────────────────────

#[tauri::command]
/// Get receipt settings.
pub async fn get_receipt_settings(
    state: State<'_, AppState>,
) -> Result<ReceiptSettingsDto, AppError> {
    let conn = state.db.lock().await;
    run_get_receipt_settings(&conn)
}

/// Get receipt settings resolved from a session token. ADR #7.
#[tauri::command]
pub async fn get_receipt_settings_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<ReceiptSettingsDto, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_get_receipt_settings(&db)
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
        tax_rounding_mode: Settings::get_tax_rounding_mode(conn)?
            .wire_name()
            .to_string(),
    })
}

// ── Set receipt settings ──────────────────────────────────

/// **Deprecated — use `set_receipt_settings_scoped` (ADR #7).**
#[tauri::command]
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
#[tauri::command]
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
    Settings::set_tax_rounding_mode_str(&tx, &args.tax_rounding_mode)?;

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

#[tauri::command]
/// Get store settings.
pub async fn get_store_settings(state: State<'_, AppState>) -> Result<StoreSettingsDto, AppError> {
    let conn = state.db.lock().await;
    run_get_store_settings(&conn)
}

/// Get store settings resolved from a session token. ADR #7.
#[tauri::command]
pub async fn get_store_settings_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<StoreSettingsDto, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_get_store_settings(&db)
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
#[tauri::command]
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
#[tauri::command]
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

#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
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

/// List credit sales.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_credit_sales_scoped`.
#[tauri::command]
pub async fn list_credit_sales(state: State<'_, AppState>) -> Result<Vec<CreditSaleDto>, AppError> {
    let conn = state.db.lock().await;
    run_list_credit_sales(&conn)
}

/// List credit sales for the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn list_credit_sales_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<CreditSaleDto>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_list_credit_sales(&db)
}

/// Business logic for listing credit sales (extracted for testing).
fn run_list_credit_sales(conn: &rusqlite::Connection) -> Result<Vec<CreditSaleDto>, AppError> {
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
#[tauri::command]
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
#[tauri::command]
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

// ── Hardware settings (printer + scanner + scale + localPrefs) ───

/// Full terminal hardware and local-preference configuration.
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
    /// Scale connection type: "serial", "usb", "none".
    #[serde(default = "default_scale_connection")]
    pub scale_connection: String,
    /// Scale device path.
    #[serde(default)]
    pub scale_device_path: String,
    /// Scale baud rate (default 9600).
    #[serde(default = "default_scale_baud_rate")]
    pub scale_baud_rate: i64,
    /// Zero the scale automatically on boot.
    #[serde(default)]
    pub scale_zero_on_boot: bool,
    /// Sound volume percentage (0–100).
    #[serde(default = "default_sound_volume")]
    pub sound_volume: i64,
    /// Dark mode enabled.
    #[serde(default)]
    pub dark_mode: bool,
    /// Kitchen printer connection type.
    #[serde(default = "default_kitchen_printer_connection")]
    pub kitchen_printer_connection: String,

    /// Kitchen printer device path or IP.
    #[serde(default)]
    pub kitchen_printer_device_path: String,

    /// Schema version of the hardware profile (for forward-compatible evolution).
    #[serde(default = "default_hw_schema_version")]
    pub schema_version: i64,

    /// Scale auto-zero after each transaction.
    #[serde(default = "default_scale_auto_zero")]
    pub scale_auto_zero: bool,
}

fn default_scale_connection() -> String {
    "none".into()
}
fn default_scale_baud_rate() -> i64 {
    9600
}
fn default_kitchen_printer_connection() -> String {
    "disabled".into()
}
fn default_hw_schema_version() -> i64 {
    1
}
fn default_sound_volume() -> i64 {
    80
}
fn default_scale_auto_zero() -> bool {
    true
}

impl From<TerminalProfile> for HardwareSettingsDto {
    fn from(p: TerminalProfile) -> Self {
        Self {
            printer_connection: p.printer_connection,
            printer_device_path: p.printer_device_path,
            printer_paper_size: p.printer_paper_size,
            scanner_device_id: p.scanner_device_id,
            scanner_input_mode: p.scanner_input_mode,
            scale_connection: p.scale_connection,
            scale_device_path: p.scale_device_path,
            scale_baud_rate: p.scale_baud_rate as i64,
            scale_zero_on_boot: p.scale_zero_on_boot,
            kitchen_printer_connection: p.kitchen_printer_connection,
            kitchen_printer_device_path: p.kitchen_printer_device_path,
            schema_version: p.schema_version as i64,
            sound_volume: p.sound_volume as i64,
            dark_mode: p.dark_mode,
            scale_auto_zero: p.scale_auto_zero,
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
            scale_connection: dto.scale_connection,
            scale_device_path: dto.scale_device_path,
            scale_baud_rate: dto.scale_baud_rate as u32,
            scale_zero_on_boot: dto.scale_zero_on_boot,
            kitchen_printer_connection: dto.kitchen_printer_connection,
            kitchen_printer_device_path: dto.kitchen_printer_device_path,
            schema_version: dto.schema_version as u32,
            sound_volume: dto.sound_volume as u32,
            dark_mode: dto.dark_mode,
            scale_auto_zero: dto.scale_auto_zero,
        }
    }
}

fn app_data_dir(state: &AppState) -> Result<std::path::PathBuf, AppError> {
    state
        .db_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| AppError::Internal("db_path has no parent directory".into()))
}

#[tauri::command]
/// Get hardware settings for the current terminal from the DB.
///
/// Read order:
/// 1. DB (`hardware_profiles` table) — canonical store (TODO 4e)
/// 2. JSON file (`terminal_profiles/<id>.json`) — fallback
/// 3. Old SQLite settings — legacy fallback
///
/// Returns defaults only when none of the above have saved values.
pub async fn get_hardware_settings(
    state: State<'_, AppState>,
) -> Result<HardwareSettingsDto, AppError> {
    let terminal_id = state
        .terminal_id
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // 1. Try DB first (canonical store).
    {
        let conn = state.db.lock().await;
        let profile_json: Option<String> = conn
            .query_row(
                "SELECT profile_json FROM hardware_profiles WHERE terminal_id = ?1",
                rusqlite::params![&terminal_id],
                |row| row.get(0),
            )
            .ok();
        if let Some(json) = profile_json {
            if let Ok(profile) = serde_json::from_str::<TerminalProfile>(&json) {
                return Ok(HardwareSettingsDto::from(profile));
            }
            tracing::warn!(
                terminal_id = %terminal_id,
                "failed to parse hardware profile JSON from DB — falling back to file"
            );
        }
    } // conn dropped

    let base_dir = app_data_dir(&state)?;
    let path = TerminalProfile::profile_path(&base_dir, &terminal_id);

    // 2. Try JSON file as fallback.
    if let Some(profile) = TerminalProfile::load(&path)? {
        // Sync the JSON profile into the DB for future fast reads.
        let json = serde_json::to_string(&profile)
            .map_err(|e| AppError::Internal(format!("serializing profile: {e}")))?;
        let conn = state.db.lock().await;
        if let Err(e) = conn.execute(
            "INSERT OR REPLACE INTO hardware_profiles (terminal_id, profile_json, schema_version, updated_at)
             VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            rusqlite::params![&terminal_id, &json, profile.schema_version],
        ) {
            tracing::warn!(
                terminal_id = %terminal_id,
                error = %e,
                "failed to sync JSON profile to DB — will retry next read"
            );
        }
        return Ok(HardwareSettingsDto::from(profile));
    }

    // 3. Fallback: read from old SQLite settings (pre-ADR #22).
    let conn = state.db.lock().await;
    let profile = TerminalProfile {
        printer_connection: Settings::get_printer_connection(&conn)?,
        printer_device_path: Settings::get_printer_device_path(&conn)?,
        printer_paper_size: Settings::get_printer_paper_size(&conn)?,
        scanner_device_id: Settings::get_scanner_device_id(&conn)?,
        scanner_input_mode: Settings::get_scanner_input_mode(&conn)?,
        ..Default::default()
    };

    // Persist to both JSON (for backward compat readers) and DB (canonical).
    let json = serde_json::to_string(&profile)
        .map_err(|e| AppError::Internal(format!("serializing profile: {e}")))?;
    if let Err(e) = profile.save(&path) {
        tracing::warn!(
            terminal_id = %terminal_id,
            error = %e,
            "failed to save migrated hardware settings to JSON — will retry"
        );
    }
    if let Err(e) = conn.execute(
        "INSERT OR REPLACE INTO hardware_profiles (terminal_id, profile_json, schema_version, updated_at)
         VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        rusqlite::params![&terminal_id, &json, profile.schema_version],
    ) {
        tracing::warn!(
            terminal_id = %terminal_id,
            error = %e,
            "failed to save migrated hardware settings to DB — will retry next read"
        );
    }

    // Clean up old SQLite keys after successful migration.
    let hw_keys = [
        "printer.connection",
        "printer.device_path",
        "printer.paper_size",
        "scanner.device_id",
        "scanner.input_mode",
    ];
    for key in hw_keys {
        if let Err(e) = Settings::remove(&conn, key) {
            tracing::warn!(
                key,
                error = %e,
                "failed to remove orphaned SQLite hardware setting"
            );
        }
    }

    Ok(HardwareSettingsDto::from(profile))
}

/// **Deprecated — use `set_hardware_settings_scoped` (ADR #7).**
///
/// Writes to both DB (canonical) and JSON file (fallback).
#[tauri::command]
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

    let profile = TerminalProfile::from(args);
    let json = serde_json::to_string(&profile)
        .map_err(|e| AppError::Internal(format!("serializing profile: {e}")))?;

    // Write to DB (canonical store).
    {
        let conn = state.db.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO hardware_profiles (terminal_id, profile_json, schema_version, updated_at)
             VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            rusqlite::params![&terminal_id, &json, profile.schema_version],
        )?;
    }

    // Write to JSON file (backward compat fallback).
    let base_dir = app_data_dir(&state)?;
    let path = TerminalProfile::profile_path(&base_dir, &terminal_id);
    if let Err(e) = profile.save(&path) {
        tracing::warn!(
            terminal_id = %terminal_id,
            error = %e,
            "failed to save hardware settings to JSON — DB write succeeded"
        );
    }

    Ok(())
}

/// Set hardware settings resolved from a session token. ADR #7.
///
/// Writes to both DB (canonical) and JSON file (fallback).
///
/// The `hardware_profiles` table lives in the global DB (not per-store)
/// since terminal hardware configuration is global across all stores.
/// Permission checking uses the store-scoped DB from the session.
#[tauri::command]
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

    let profile = TerminalProfile::from(args);
    let json = serde_json::to_string(&profile)
        .map_err(|e| AppError::Internal(format!("serializing profile: {e}")))?;

    // Write to DB (canonical store).
    // We use the global DB since hardware_profiles is a global table.
    {
        let conn = state.db.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO hardware_profiles (terminal_id, profile_json, schema_version, updated_at)
             VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            rusqlite::params![&terminal_id, &json, profile.schema_version],
        )?;
    }

    // Write to JSON file (backward compat fallback).
    let base_dir = app_data_dir(&state)?;
    let path = TerminalProfile::profile_path(&base_dir, &terminal_id);
    if let Err(e) = profile.save(&path) {
        tracing::warn!(
            terminal_id = %terminal_id,
            error = %e,
            "failed to save hardware settings to JSON — DB write succeeded"
        );
    }

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
#[tauri::command]
pub async fn get_user_preferences(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, AppError> {
    let conn = state.db.lock().await;
    Ok(UserPreferences::get_all(&conn, &user_id)?)
}

/// Get user preferences resolved from a session token. ADR #7.
/// Uses `session.user_id` for the preference lookup.
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
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
        if let Err(e) = enqueue_settings_updates(
            &store,
            &HashMap::from([(key.clone(), value.clone())]),
            &terminal_id,
            "default",
        ) {
            tracing::warn!(key = %key, error = %e, "failed to enqueue settings.update sync item");
        }
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
#[tauri::command]
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

    // Enqueue `settings.update` sync items on the GLOBAL db — the sync
    // daemon only watches the global queue, so a store-scoped write must
    // fan out from here (SYNC-10 enqueue side).
    {
        let conn = state.db.lock().await;
        let store = oz_core::db::Store::new(&conn);
        if let Err(e) = enqueue_settings_updates(
            &store,
            &HashMap::from([(key.clone(), value.clone())]),
            &terminal_id,
            &session.store_id,
        ) {
            tracing::warn!(key = %key, error = %e, "failed to enqueue settings.update sync item");
        }
    } // conn dropped — safe to .await below

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

/// Enqueue one `settings.update` sync item per changed key (SYNC-10).
///
/// Delegates to [`Store::enqueue_settings_update_superseding`] (oz-core),
/// which owns the `settings.update` wire contract: payload shape, Low
/// priority, and supersede-any-pending-same-key semantics. Callers enqueue
/// on the GLOBAL db (the sync daemon only watches the global queue), never
/// the store db the value was written to.
fn enqueue_settings_updates(
    store: &Store,
    entries: &HashMap<String, String>,
    terminal_id: &str,
    tenant_id: &str,
) -> Result<(), AppError> {
    for (key, value) in entries {
        store.enqueue_settings_update_superseding(key, value, terminal_id, tenant_id)?;
    }
    Ok(())
}

// ── Batch key-value settings (single transaction) ───────────────

/// Write (or overwrite) multiple settings in a single transaction.
///
/// All entries are written atomically — either all succeed or none
/// do. A single `SettingsUpdated` event is published with all changed
/// keys after the transaction commits.
#[tauri::command]
pub async fn set_settings(
    entries: HashMap<String, String>,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let terminal_id = state
        .terminal_id
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let keys: Vec<String> = entries.keys().cloned().collect();

    {
        let conn = state.db.lock().await;
        let store = oz_core::db::Store::new(&conn);
        require_permission_for_user(&store, &user_id, permissions::SETTINGS_EDIT)?;
        let tx = conn.unchecked_transaction()?;
        for (key, value) in &entries {
            Settings::set_tracked(&tx, key, value, &terminal_id)?;
        }
        tx.commit()?;
        if let Err(e) = enqueue_settings_updates(&store, &entries, &terminal_id, "default") {
            tracing::warn!(key_count = entries.len(), error = %e, "failed to enqueue settings.update sync items");
        }
    }

    // Publish a single SettingsUpdated event for all changed keys.
    let kernel = state.kernel.lock().await;
    let bus = kernel.event_bus();
    let event = oz_core::events::SettingsUpdated {
        changed_keys: keys,
        terminal_id,
    };
    if let Err(e) = bus.publish(&event) {
        tracing::warn!(
            key_count = entries.len(),
            error = %e,
            "failed to publish SettingsUpdated event"
        );
    }

    Ok(())
}

/// Write (or overwrite) multiple settings in a single transaction, resolved from a session token. ADR #7.
///
/// All entries are written atomically — either all succeed or none
/// do. A single `SettingsUpdated` event is published with all changed
/// keys after the transaction commits.
#[tauri::command]
pub async fn set_settings_scoped(
    session_token: String,
    entries: HashMap<String, String>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;

    let terminal_id = state
        .terminal_id
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let keys: Vec<String> = entries.keys().cloned().collect();

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
        let tx = db.unchecked_transaction()?;
        for (key, value) in &entries {
            Settings::set_tracked(&tx, key, value, &terminal_id)?;
        }
        tx.commit()?;
    }

    // Enqueue `settings.update` sync items on the GLOBAL db — the sync
    // daemon only watches the global queue, so a store-scoped write must
    // fan out from here (SYNC-10 enqueue side).
    {
        let conn = state.db.lock().await;
        let store = oz_core::db::Store::new(&conn);
        if let Err(e) = enqueue_settings_updates(&store, &entries, &terminal_id, &session.store_id)
        {
            tracing::warn!(key_count = entries.len(), error = %e, "failed to enqueue settings.update sync items");
        }
    } // conn dropped — safe to .await below

    // Publish a single SettingsUpdated event for all changed keys.
    let kernel = state.kernel.lock().await;
    let bus = kernel.event_bus();
    let event = oz_core::events::SettingsUpdated {
        changed_keys: keys,
        terminal_id,
    };
    if let Err(e) = bus.publish(&event) {
        tracing::warn!(
            key_count = entries.len(),
            error = %e,
            "failed to publish SettingsUpdated event"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::SyncPriority;
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
        assert_eq!(result.tax_rounding_mode, "half_up");
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
            tax_rounding_mode: "truncate".into(),
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
        assert_eq!(result.tax_rounding_mode, "truncate");
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
                tax_rounding_mode: "half_up".into(),
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
                tax_rounding_mode: "half_up".into(),
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
            tax_rounding_mode: "half_up".into(),
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
            scale_connection: "serial".into(),
            scale_device_path: "COM3".into(),
            scale_baud_rate: 115200,
            scale_zero_on_boot: true,
            kitchen_printer_connection: "network".into(),
            kitchen_printer_device_path: "192.168.1.51".into(),
            schema_version: 1,
            sound_volume: 60,
            dark_mode: true,
            scale_auto_zero: false,
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["printerConnection"], "USB");
        assert_eq!(json["scaleConnection"], "serial");
        assert_eq!(json["soundVolume"], 60);
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

    /// After wiring ADR #22, `run_set_setting` writes a delta row
    /// in addition to updating the settings table. This test verifies
    /// the Tauri command layer actually produces delta records.
    /// SYNC-10 enqueue side: a settings write must also enqueue a
    /// `settings.update` sync item so the daemon can push the change to
    /// the cloud and other terminals re-pull it. The payload shape must
    /// match `platform_sync::queue::SettingsUpdatePayload`
    /// (key/value/terminal_id) that the apply side parses.
    #[test]
    fn settings_write_enqueues_settings_update_item() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        enqueue_settings_updates(
            &store,
            &HashMap::from([("receipt_footer".to_string(), "Thanks".to_string())]),
            "term-1",
            "default",
        )
        .unwrap();

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 1);
        let item = &pending[0];
        assert_eq!(item.action, "settings.update");
        assert_eq!(item.tenant_id, "default");
        assert_eq!(item.priority, SyncPriority::Low);

        let payload: serde_json::Value = serde_json::from_str(&item.payload).unwrap();
        assert_eq!(payload["key"], "receipt_footer");
        assert_eq!(payload["value"], "Thanks");
        assert_eq!(payload["terminal_id"], "term-1");
    }

    /// A batch save fans out one item per changed key — the apply side
    /// applies each as its own version-LWW entry.
    #[test]
    fn settings_batch_enqueues_one_item_per_key() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        enqueue_settings_updates(
            &store,
            &HashMap::from([
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ]),
            "term-2",
            "store-x",
        )
        .unwrap();

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending.iter().all(|i| i.action == "settings.update"));
        assert!(pending.iter().all(|i| i.tenant_id == "store-x"));
        assert!(pending.iter().all(|i| i.priority == SyncPriority::Low));

        let mut keys: Vec<String> = pending
            .iter()
            .map(|i| {
                let v: serde_json::Value = serde_json::from_str(&i.payload).unwrap();
                v["key"].as_str().unwrap().to_string()
            })
            .collect();
        keys.sort();
        assert_eq!(keys, vec!["a".to_string(), "b".to_string()]);
    }

    /// A second local save of the SAME key must supersede the still-pending
    /// item (replace it), not append a duplicate — otherwise the daemon
    /// pushes stale values in order and a save of v1→v2→v1 while offline
    /// ends with the remote at v2 while the local is at v1.
    #[test]
    fn settings_save_supersedes_pending_item_for_same_key() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        enqueue_settings_updates(
            &store,
            &HashMap::from([("theme".to_string(), "dark".to_string())]),
            "term-1",
            "default",
        )
        .unwrap();
        enqueue_settings_updates(
            &store,
            &HashMap::from([("theme".to_string(), "light".to_string())]),
            "term-1",
            "default",
        )
        .unwrap();

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(
            pending.len(),
            1,
            "second save must replace the pending item"
        );
        let v: serde_json::Value = serde_json::from_str(&pending[0].payload).unwrap();
        assert_eq!(
            v["value"], "light",
            "pending item must carry the newest value"
        );
    }

    /// Superseding one key must leave pending items for OTHER keys intact.
    #[test]
    fn settings_save_keeps_pending_items_for_other_keys() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        enqueue_settings_updates(
            &store,
            &HashMap::from([
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ]),
            "term-1",
            "default",
        )
        .unwrap();
        enqueue_settings_updates(
            &store,
            &HashMap::from([("a".to_string(), "3".to_string())]),
            "term-1",
            "default",
        )
        .unwrap();

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 2);
        let mut keyed: Vec<(String, String)> = pending
            .iter()
            .map(|i| {
                let v: serde_json::Value = serde_json::from_str(&i.payload).unwrap();
                (
                    v["key"].as_str().unwrap().to_string(),
                    v["value"].as_str().unwrap().to_string(),
                )
            })
            .collect();
        keyed.sort();
        assert_eq!(
            keyed,
            vec![
                ("a".to_string(), "3".to_string()),
                ("b".to_string(), "2".to_string())
            ]
        );
    }

    /// Supersede must be tenant-scoped — store-y's save of the same key
    /// must not remove store-x's pending item (multi-store isolation).
    #[test]
    fn settings_supersede_is_tenant_scoped() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        enqueue_settings_updates(
            &store,
            &HashMap::from([("theme".to_string(), "dark".to_string())]),
            "term-1",
            "store-x",
        )
        .unwrap();
        enqueue_settings_updates(
            &store,
            &HashMap::from([("theme".to_string(), "dark".to_string())]),
            "term-1",
            "store-y",
        )
        .unwrap();

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(
            pending.len(),
            2,
            "cross-tenant items must not be superseded"
        );
    }

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

    #[test]
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
            tax_rounding_mode: "half_up".into(),
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
            scale_connection: "usb".into(),
            scale_device_path: "/dev/hidraw0".into(),
            scale_baud_rate: 9600,
            scale_zero_on_boot: false,
            kitchen_printer_connection: "network".into(),
            kitchen_printer_device_path: "10.0.0.50".into(),
            schema_version: 1,
            sound_volume: 80,
            dark_mode: false,
            scale_auto_zero: true,
        };
        let json = serde_json::to_value(&dto).unwrap();
        let back: HardwareSettingsDto = serde_json::from_value(json).unwrap();
        assert_eq!(back.printer_connection, "Network");
        assert_eq!(back.scanner_device_id, "scanner-2");
        assert_eq!(back.scale_connection, "usb");
        assert_eq!(back.sound_volume, 80);
        assert!(back.scale_auto_zero);
    }

    /// The orphan-cleanup keys in `get_hardware_settings` must stay
    /// in sync with the constants in `platform_core::settings::keys`.
    /// If this test fails, update the `hw_keys` array.
    #[test]
    fn hw_orphan_keys_match_platform_core_constants() {
        use platform_core::settings::keys;
        let expected = [
            keys::PRINTER_CONNECTION,
            keys::PRINTER_DEVICE_PATH,
            keys::PRINTER_PAPER_SIZE,
            keys::SCANNER_DEVICE_ID,
            keys::SCANNER_INPUT_MODE,
        ];
        // These must match the hw_keys array in get_hardware_settings.
        assert_eq!(expected[0], "printer.connection");
        assert_eq!(expected[1], "printer.device_path");
        assert_eq!(expected[2], "printer.paper_size");
        assert_eq!(expected[3], "scanner.device_id");
        assert_eq!(expected[4], "scanner.input_mode");
    }
}

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
use oz_core::{Settings, Store, UserPreferences};

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
        tax_rounding_mode: Settings::get_tax_rounding_mode(conn)?
            .wire_name()
            .to_string(),
    })
}

// ── Set receipt settings ──────────────────────────────────

#[command]
/// Set receipt settings.
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

#[command]
/// Set store settings.
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

#[command]
/// Set credit settings.
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

// ── Credit sale DTO ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
/// Creditsaledto.
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

#[command]
/// Settle credit.
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

// ── Hardware settings (printer + scanner) ───────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Hardwaresettingsdto.
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

#[command]
/// Get hardware settings.
pub async fn get_hardware_settings(
    state: State<'_, AppState>,
) -> Result<HardwareSettingsDto, AppError> {
    let conn = state.db.lock().await;
    Ok(HardwareSettingsDto {
        printer_connection: Settings::get_printer_connection(&conn)?,
        printer_device_path: Settings::get_printer_device_path(&conn)?,
        printer_paper_size: Settings::get_printer_paper_size(&conn)?,
        scanner_device_id: Settings::get_scanner_device_id(&conn)?,
        scanner_input_mode: Settings::get_scanner_input_mode(&conn)?,
    })
}

#[command]
/// Set hardware settings.
pub async fn set_hardware_settings(
    args: HardwareSettingsDto,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::db::Store::new(&conn);
    require_permission_for_user(&store, &user_id, permissions::SETTINGS_EDIT)?;
    let tx = conn.unchecked_transaction()?;
    Settings::set_printer_connection(&tx, &args.printer_connection)?;
    Settings::set_printer_device_path(&tx, &args.printer_device_path)?;
    Settings::set_printer_paper_size(&tx, &args.printer_paper_size)?;
    Settings::set_scanner_device_id(&tx, &args.scanner_device_id)?;
    Settings::set_scanner_input_mode(&tx, &args.scanner_input_mode)?;
    tx.commit()?;
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

#[command]
/// Get user preferences.
pub async fn get_user_preferences(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, AppError> {
    let conn = state.db.lock().await;
    Ok(UserPreferences::get_all(&conn, &user_id)?)
}

#[command]
/// Set user preferences.
pub async fn set_user_preferences(
    user_id: String,
    prefs: Vec<UserPrefEntry>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    let pairs: Vec<(String, String)> = prefs.into_iter().map(|e| (e.key, e.value)).collect();
    Ok(UserPreferences::set_batch(&conn, &user_id, &pairs)?)
}

#[command]
/// Get user preferences resolved from a session token. ADR #7.
///
/// Uses `session.user_id` for the preference lookup against the
/// session's store database, so a tablet terminal persists the same
/// per-user preferences (menu sort, card/font size) that the desktop
/// client writes.
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

#[command]
/// Set user preferences resolved from a session token. ADR #7.
///
/// Uses `session.user_id` for the preference write against the
/// session's store database — parity with the desktop client so
/// the restaurant-menu hamburger configuration persists to the
/// shared user settings on tablet terminals.
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
    // Extract terminal_id before locking the DB — no await inside the lock.
    let terminal_id = state
        .terminal_id
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let conn = state.db.lock().await;
    let store = oz_core::db::Store::new(&conn);
    require_permission_for_user(&store, &user_id, permissions::SETTINGS_EDIT)?;
    run_set_setting(&conn, &key, &value, &terminal_id)?;
    // SYNC-10 parity: enqueue the change so the tablet's sync daemon
    // pushes it to the cloud (and the desktop's pull re-applies it).
    // Warn-and-continue — the local write already committed.
    if let Err(e) = enqueue_settings_update(&store, &key, &value, &terminal_id) {
        tracing::warn!(key = %key, error = %e, "failed to enqueue settings.update sync item");
    }
    Ok(())
}

/// Business logic for `set_setting` (extracted for testing).
/// Uses `set_tracked` so every settings change writes a delta record
/// (ADR #22) — the basis for version-LWW when the change syncs.
fn run_set_setting(
    conn: &rusqlite::Connection,
    key: &str,
    value: &str,
    terminal_id: &str,
) -> Result<(), AppError> {
    Ok(Settings::set_tracked(conn, key, value, terminal_id)?)
}

/// Enqueue a `settings.update` sync item for a tablet settings save,
/// scoped to the "default" tenant on the global queue (SYNC-10).
/// Supersede semantics live in oz-core's
/// [`Store::enqueue_settings_update_superseding`].
fn enqueue_settings_update(
    store: &Store,
    key: &str,
    value: &str,
    terminal_id: &str,
) -> Result<(), AppError> {
    Ok(store.enqueue_settings_update_superseding(key, value, terminal_id, "default")?)
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
            margin_top: 3,
            margin_bottom: 5,
            margin_left: 1,
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
        assert_eq!(result.margin_top, 3);
        assert_eq!(result.margin_bottom, 5);
        assert_eq!(result.margin_left, 1);
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
                show_table_number: true,
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
                show_table_number: false,
                margin_top: 5,
                margin_bottom: 2,
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
        assert!(
            !result.show_table_number,
            "v2 overwrites show_table_number to false"
        );
        assert_eq!(result.margin_top, 5);
        assert_eq!(result.margin_bottom, 2);
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

    // ── DTO struct tests ──────────────────────────────────────────

    #[test]
    fn receipt_settings_dto_debug() {
        let dto = ReceiptSettingsDto {
            show_currency: true,
            decimal_separator: "comma".into(),
            show_tax: false,
            footer: "Thank you".into(),
            paper_width: "narrow".into(),
            show_table_number: true,
            margin_top: 5,
            margin_bottom: 3,
            margin_left: 2,
            margin_right: 2,
            tax_rounding_mode: "half_up".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("comma"));
        assert!(d.contains("narrow"));
    }

    #[test]
    fn receipt_settings_dto_serialize() {
        let dto = ReceiptSettingsDto {
            show_currency: false,
            decimal_separator: "dot".into(),
            show_tax: true,
            footer: "".into(),
            paper_width: "standard".into(),
            show_table_number: false,
            margin_top: 0,
            margin_bottom: 0,
            margin_left: 0,
            margin_right: 0,
            tax_rounding_mode: "half_up".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert!(!json["showCurrency"].as_bool().unwrap());
        assert_eq!(json["decimalSeparator"], "dot");
        assert_eq!(json["paperWidth"], "standard");
    }

    #[test]
    fn receipt_settings_dto_deserialize() {
        let json = r#"{"showCurrency":true,"decimalSeparator":"comma","showTax":false,"footer":"Thanks","paperWidth":"narrow","showTableNumber":false,"marginTop":4,"marginBottom":2,"marginLeft":1,"marginRight":1}"#;
        let dto: ReceiptSettingsDto = serde_json::from_str(json).unwrap();
        assert!(dto.show_currency);
        assert_eq!(dto.decimal_separator, "comma");
        assert_eq!(dto.margin_top, 4);
    }

    #[test]
    fn store_settings_dto_debug() {
        let dto = StoreSettingsDto {
            name: "My Store".into(),
            address: "123 Main".into(),
            tax_id: "TAX-001".into(),
            currency: "USD".into(),
            branch: "Main".into(),
            logo: "abc123".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("My Store"));
        assert!(d.contains("USD"));
    }

    #[test]
    fn store_settings_dto_serialize() {
        let dto = StoreSettingsDto {
            name: "Cafe".into(),
            address: "456 Oak".into(),
            tax_id: "".into(),
            currency: "IDR".into(),
            branch: "Mall".into(),
            logo: "".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["name"], "Cafe");
        assert_eq!(json["currency"], "IDR");
        assert_eq!(json["address"], "456 Oak");
    }

    #[test]
    fn store_settings_dto_deserialize() {
        let json = r#"{"name":"Shop","address":"1 Rd","taxId":"TX","currency":"EUR","branch":"A","logo":"L"}"#;
        let dto: StoreSettingsDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.name, "Shop");
        assert_eq!(dto.currency, "EUR");
        assert_eq!(dto.branch, "A");
    }

    #[test]
    fn credit_settings_dto_serialize() {
        let dto = CreditSettingsDto {
            enabled: true,
            reminder_interval_hours: 24,
            max_limit_minor: 500000,
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert!(json["enabled"].as_bool().unwrap());
        assert_eq!(json["reminderIntervalHours"], 24);
        assert_eq!(json["maxLimitMinor"], 500000);
    }

    #[test]
    fn hardware_settings_dto_serialize() {
        let dto = HardwareSettingsDto {
            printer_connection: "usb".into(),
            printer_device_path: "/dev/usb/lp0".into(),
            printer_paper_size: "80mm".into(),
            scanner_device_id: "scanner-01".into(),
            scanner_input_mode: "keyboard".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["printerConnection"], "usb");
        assert_eq!(json["scannerInputMode"], "keyboard");
    }

    #[test]
    fn user_pref_entry_debug() {
        let entry = UserPrefEntry {
            key: "theme".into(),
            value: "dark".into(),
        };
        let d = format!("{entry:?}");
        assert!(d.contains("theme"));
        assert!(d.contains("dark"));
    }

    #[test]
    fn user_pref_entry_serialize() {
        let entry = UserPrefEntry {
            key: "lang".into(),
            value: "en".into(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["key"], "lang");
        assert_eq!(json["value"], "en");
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
        };
        let json = serde_json::to_value(&dto).unwrap();
        let back: HardwareSettingsDto = serde_json::from_value(json).unwrap();
        assert_eq!(back.printer_connection, "Network");
        assert_eq!(back.scanner_device_id, "scanner-2");
    }

    // ── Generic get_setting / set_setting tests (C-3 fix verification) ─

    #[test]
    fn get_setting_returns_none_for_missing_key() {
        let conn = fresh_conn();
        let result = run_get_setting(&conn, "nonexistent.key").unwrap();
        assert!(result.is_none());
    }

    /// ADR #22 parity: the tablet's settings write must record a delta
    /// (version 1), not just overwrite the row — the delta ledger is the
    /// basis for version-LWW when the change syncs.
    #[test]
    fn run_set_setting_writes_delta_row() {
        let conn = fresh_conn();
        run_set_setting(&conn, "delta.test", "delta-val", "term-delta").unwrap();
        assert_eq!(
            Settings::get(&conn, "delta.test").unwrap(),
            Some("delta-val".into())
        );
        assert_eq!(
            Settings::get_version(&conn, "delta.test", "term-delta").unwrap(),
            Some(1)
        );
    }

    /// SYNC-10 parity: a tablet settings save must enqueue a
    /// `settings.update` item on the global queue so the tablet's sync
    /// daemon pushes it to the cloud (and the desktop's pull re-applies it).
    #[test]
    fn set_setting_enqueues_settings_update_item() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        enqueue_settings_update(&store, "theme", "dark", "term-1").unwrap();

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action, "settings.update");
        assert_eq!(pending[0].tenant_id, "default");
        assert_eq!(pending[0].priority, SyncPriority::Low);
        let v: serde_json::Value = serde_json::from_str(&pending[0].payload).unwrap();
        assert_eq!(v["key"], "theme");
        assert_eq!(v["value"], "dark");
        assert_eq!(v["terminal_id"], "term-1");
    }

    #[test]
    fn set_setting_persists_and_get_returns_it() {
        let conn = fresh_conn();
        run_set_setting(&conn, "sync.auth_token", "sk_test_abc123", "term-1").unwrap();
        let result = run_get_setting(&conn, "sync.auth_token").unwrap();
        assert_eq!(result, Some("sk_test_abc123".into()));
    }

    #[test]
    fn set_setting_overwrites_previous_value() {
        let conn = fresh_conn();
        run_set_setting(&conn, "my.key", "v1", "term-1").unwrap();
        run_set_setting(&conn, "my.key", "v2", "term-1").unwrap();
        let result = run_get_setting(&conn, "my.key").unwrap();
        assert_eq!(result, Some("v2".into()));
    }

    #[test]
    fn set_setting_empty_string_is_stored_as_empty() {
        let conn = fresh_conn();
        run_set_setting(&conn, "key", "hello", "term-1").unwrap();
        run_set_setting(&conn, "key", "", "term-1").unwrap();
        let result = run_get_setting(&conn, "key").unwrap();
        assert_eq!(result, Some("".into()));
    }

    #[test]
    fn get_setting_after_multiple_keys_only_returns_requested() {
        let conn = fresh_conn();
        run_set_setting(&conn, "a", "1", "term-1").unwrap();
        run_set_setting(&conn, "b", "2", "term-1").unwrap();
        run_set_setting(&conn, "c", "3", "term-1").unwrap();
        assert_eq!(run_get_setting(&conn, "b").unwrap(), Some("2".into()));
        assert_eq!(run_get_setting(&conn, "d").unwrap(), None);
    }

    #[test]
    fn sync_auth_token_cross_screen_roundtrip() {
        // C-3 fix verification: the sync.auth_token key written by
        // one screen (SettingsPage) must be readable by another
        // (RetailOptionsScreen / useCloudSync) via get_setting.
        let conn = fresh_conn();

        // Simulate SettingsPage saving a token
        run_set_setting(&conn, "sync.auth_token", "jwt-token-xyz", "term-1").unwrap();

        // Simulate useCloudSync loading the token on the other screen
        let loaded = run_get_setting(&conn, "sync.auth_token").unwrap();
        assert_eq!(
            loaded,
            Some("jwt-token-xyz".into()),
            "C-3 regression: token saved via SettingsPage must be readable via get_setting"
        );
    }

    // ── Scoped user preferences (tablet parity — AUDIT-25) ─────────

    use oz_core::session::SessionContext;
    use platform_core::StoreDatabaseManager;
    use tauri::Manager as _;

    /// Seed a session for `token` bound to `store_id` and `user_id`.
    fn seed_session(state: &mut AppState, token: &str, store_id: &str, user_id: &str) {
        state.session_store.write().unwrap().insert(
            token.into(),
            SessionContext::new(
                user_id.into(),
                "role-cashier".into(),
                "terminal-1".into(),
                store_id.into(),
                "instance-1".into(),
                "restaurant-pos".into(),
                None,
                0,
            ),
        );
    }

    fn pref(key: &str, value: &str) -> UserPrefEntry {
        UserPrefEntry {
            key: key.into(),
            value: value.into(),
        }
    }

    #[tokio::test]
    async fn scoped_user_preferences_rejects_invalid_token() {
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test())
            .build(tauri::generate_context!())
            .unwrap();

        let read = get_user_preferences_scoped("missing-token".into(), app.state()).await;
        assert!(matches!(read, Err(AppError::InvalidSession)));

        let write = set_user_preferences_scoped(
            "missing-token".into(),
            vec![pref("cardsize", "3")],
            app.state(),
        )
        .await;
        assert!(matches!(write, Err(AppError::InvalidSession)));
    }

    #[tokio::test]
    async fn scoped_user_preferences_roundtrip_targets_session_store_and_user() {
        let conn = oz_core::migrations::fresh_db();
        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
        seed_session(&mut state, "store-a-token", "store-a", "cashier-a");
        seed_session(&mut state, "store-b-token", "store-b", "cashier-a");
        seed_session(&mut state, "other-user-token", "store-a", "cashier-b");
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        // The restaurant-menu hamburger configuration for cashier-a in
        // store-a — the exact keys RestaurantMenu persists scoped.
        set_user_preferences_scoped(
            "store-a-token".into(),
            vec![
                pref("sort", "popularity"),
                pref("cardsize", "3"),
                pref("fontsize", "2"),
            ],
            app.state(),
        )
        .await
        .unwrap();

        let prefs = get_user_preferences_scoped("store-a-token".into(), app.state())
            .await
            .unwrap();
        assert_eq!(prefs.get("sort").map(String::as_str), Some("popularity"));
        assert_eq!(prefs.get("cardsize").map(String::as_str), Some("3"));
        assert_eq!(prefs.get("fontsize").map(String::as_str), Some("2"));

        // Isolated per store: the same user in store-b must not see store-a.
        let store_b = get_user_preferences_scoped("store-b-token".into(), app.state())
            .await
            .unwrap();
        assert!(
            store_b.is_empty(),
            "store B must not see store A user preferences"
        );

        // Isolated per user: another user in store-a must not see them.
        let other = get_user_preferences_scoped("other-user-token".into(), app.state())
            .await
            .unwrap();
        assert!(
            other.is_empty(),
            "another user in the same store must not see cashier-a preferences"
        );
    }
}

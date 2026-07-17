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

#[cfg(test)]
mod tests {
    use super::*;
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

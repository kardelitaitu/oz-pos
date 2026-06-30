//! Settings Tauri commands: get and persist receipt display options.
//!
//! This module exposes the receipt-related subset of the `settings` table
//! to the front-end. Other settings (store name, currency, features) are
//! managed by the setup wizard and may be exposed here in the future.

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::command;

use oz_core::Settings;

use crate::error::AppError;
use crate::state::AppState;

// ── Receipt settings DTO ─────────────────────────────────

/// All receipt display options in one shot – the UI loads these on
/// mount and sends the whole struct back on save.
#[derive(Debug, Serialize, Deserialize)]
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
}

// ── Get receipt settings ──────────────────────────────────

#[command]
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
    })
}

// ── Set receipt settings ──────────────────────────────────

#[command]
pub async fn set_receipt_settings(
    args: ReceiptSettingsDto,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
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

    tx.commit()?;

    Ok(())
}

// ── Store info DTO ────────────────────────────────────────────

/// Store name, address, and tax ID – shown on printed receipts.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoreSettingsDto {
    pub name: String,
    pub address: String,
    pub tax_id: String,
}

// ── Get store settings ────────────────────────────────────────

#[command]
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
    })
}

// ── Set store settings ────────────────────────────────────────

#[command]
pub async fn set_store_settings(
    args: StoreSettingsDto,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
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

    tx.commit()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        conn
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
        };

        run_set_receipt_settings(&conn, &dto).unwrap();
        let result = run_get_receipt_settings(&conn).unwrap();

        assert!(!result.show_currency);
        assert_eq!(result.decimal_separator, "comma");
        assert!(!result.show_tax);
        assert_eq!(result.footer, "Thanks!");
        assert_eq!(result.paper_width, "narrow");
    }

    #[test]
    fn get_store_settings_returns_defaults() {
        let conn = fresh_conn();
        let result = run_get_store_settings(&conn).unwrap();

        assert_eq!(result.name, "");
        assert_eq!(result.address, "");
        assert_eq!(result.tax_id, "");
    }

    #[test]
    fn set_store_settings_persists() {
        let conn = fresh_conn();
        let dto = StoreSettingsDto {
            name: "My Coffee Shop".into(),
            address: "123 Main St".into(),
            tax_id: "TAX-12345".into(),
        };

        run_set_store_settings(&conn, &dto).unwrap();
        let result = run_get_store_settings(&conn).unwrap();

        assert_eq!(result.name, "My Coffee Shop");
        assert_eq!(result.address, "123 Main St");
        assert_eq!(result.tax_id, "TAX-12345");
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
            },
        )
        .unwrap();

        let result = run_get_receipt_settings(&conn).unwrap();

        assert!(!result.show_currency);
        assert_eq!(result.decimal_separator, "comma");
        assert!(result.show_tax);
        assert_eq!(result.footer, "v2");
        assert_eq!(result.paper_width, "narrow");
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
            },
        )
        .unwrap();

        run_set_store_settings(
            &conn,
            &StoreSettingsDto {
                name: "New Name".into(),
                address: "New Address".into(),
                tax_id: "TAX-999".into(),
            },
        )
        .unwrap();

        let result = run_get_store_settings(&conn).unwrap();

        assert_eq!(result.name, "New Name");
        assert_eq!(result.address, "New Address");
        assert_eq!(result.tax_id, "TAX-999");
    }
}

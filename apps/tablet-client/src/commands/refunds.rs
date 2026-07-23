//! Refund commands — process refund against a completed sale.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::permissions;
use oz_core::{Money, Refund, RefundLine, Sale};

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
/// Refundlinearg.
pub struct RefundLineArg {
    /// ID of the associated sale line.
    pub sale_line_id: String,
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Quantity.
    pub qty: i64,
    /// Unit Price Minor.
    pub unit_price_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Total amount in minor currency units.
    pub line_total_minor: i64,
}

#[derive(Debug, Deserialize)]
/// Processrefundargs.
pub struct ProcessRefundArgs {
    /// ID of the original completed sale.
    pub sale_id: String,
    /// Reason for the refund.
    pub reason: String,
    /// Optional internal note.
    pub note: Option<String>,
    /// User ID of the staff processing the refund.
    pub user_id: String,
    /// Lines being refunded.
    pub lines: Vec<RefundLineArg>,
}

#[derive(Debug, Serialize)]
/// Processrefundresult.
pub struct ProcessRefundResult {
    /// ID of the associated refund.
    pub refund_id: String,
    /// Total amount in minor currency units.
    pub total_minor: i64,
}

/// Process a refund against a completed sale.
///
/// Requires `sales:refund` permission.
#[command]
pub async fn process_refund(
    args: ProcessRefundArgs,
    state: State<'_, AppState>,
) -> Result<ProcessRefundResult, AppError> {
    let db = state.db.lock().await;
    let result = run_process_refund(
        &db,
        &args.user_id,
        &args.sale_id,
        &args.reason,
        args.note.as_deref(),
        &args.lines,
    );
    drop(db);
    result
}

/// Args for `process_refund_scoped` — without `user_id`.
#[derive(Debug, Deserialize)]
pub struct ProcessRefundScopedArgs {
    /// ID of the associated sale.
    pub sale_id: String,
    /// Reason.
    pub reason: String,
    /// Note.
    pub note: Option<String>,
    /// Lines.
    pub lines: Vec<RefundLineArg>,
}

/// Process a refund within the session scope. ADR #7.
///
/// The `user_id` for permission checks and the refund record is read
/// from the resolved session context.
#[command]
pub async fn process_refund_scoped(
    session_token: String,
    args: ProcessRefundScopedArgs,
    state: State<'_, AppState>,
) -> Result<ProcessRefundResult, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    run_process_refund(
        &db,
        &session.user_id,
        &args.sale_id,
        &args.reason,
        args.note.as_deref(),
        &args.lines,
    )
}

/// Shared business logic for processing a refund.
fn run_process_refund(
    db: &rusqlite::Connection,
    user_id: &str,
    sale_id: &str,
    reason: &str,
    note: Option<&str>,
    lines: &[RefundLineArg],
) -> Result<ProcessRefundResult, AppError> {
    let store = Store::new(db);

    require_permission_for_user(&store, user_id, permissions::SALES_REFUND)?;

    let sale = store
        .get_sale(sale_id)?
        .ok_or_else(|| AppError::Invalid(format!("sale {} not found", sale_id)))?;
    if sale.status != oz_core::SaleStatus::Completed {
        return Err(AppError::Invalid(format!(
            "cannot refund a sale with status {:?}",
            sale.status
        )));
    }

    let refund_lines: Vec<RefundLine> = lines
        .iter()
        .map(|l| {
            let currency: oz_core::Currency = l
                .currency
                .parse()
                .map_err(|_| AppError::Invalid(format!("invalid currency code: {}", l.currency)))?;
            let unit_price = Money {
                minor_units: l.unit_price_minor,
                currency,
            };
            let line_total = Money {
                minor_units: l.line_total_minor,
                currency,
            };
            Ok(RefundLine::new(
                &l.sale_line_id,
                &l.sku,
                l.qty,
                unit_price,
                line_total,
            ))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let total_minor: i64 = refund_lines.iter().map(|l| l.line_total.minor_units).sum();
    let total = Money {
        minor_units: total_minor,
        currency: sale.currency,
    };

    let refund = Refund::new(
        sale_id,
        total,
        reason,
        note.unwrap_or(""),
        user_id,
        refund_lines,
    );

    store.create_refund(&refund)?;

    tracing::info!(refund_id = %refund.id, sale_id, total_minor, reason, "refund processed");

    Ok(ProcessRefundResult {
        refund_id: refund.id,
        total_minor,
    })
}

/// Look up a sale by its receipt barcode for quick return.
#[command]
pub async fn lookup_sale_by_receipt_barcode(
    barcode: String,
    state: State<'_, AppState>,
) -> Result<Option<Sale>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let sale = store.lookup_sale_by_receipt_barcode(&barcode)?;
    drop(db);
    Ok(sale)
}

/// Look up a sale by receipt barcode in the session scope. ADR #7.
#[command]
pub async fn lookup_sale_by_receipt_barcode_scoped(
    session_token: String,
    barcode: String,
    state: State<'_, AppState>,
) -> Result<Option<Sale>, AppError> {
    let _session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let sale = store.lookup_sale_by_receipt_barcode(&barcode)?;
    drop(db);
    Ok(sale)
}

/// List all refunds for a sale.
#[command]
pub async fn list_refunds(
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Refund>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let refunds = store.list_refunds_for_sale(&sale_id)?;
    drop(db);
    Ok(refunds)
}

/// List refunds in the session scope. ADR #7.
#[command]
pub async fn list_refunds_scoped(
    session_token: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Refund>, AppError> {
    let _session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let refunds = store.list_refunds_for_sale(&sale_id)?;
    drop(db);
    Ok(refunds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    fn seed_completed_sale(conn: &Connection) -> String {
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
                ('p1', 'COFFEE', 'Coffee', 350, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
                ('sale-1', 700, 'USD', 2, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
                ('sl-1', 'sale-1', 'COFFEE', 2, 350, 700, 'USD', 1);"
        ).unwrap();
        "sale-1".to_string()
    }

    /// Seed a user with refund permission so the permission check in
    /// `run_process_refund` passes.
    fn seed_user_with_refund_permission(conn: &Connection, user_id: &str) {
        conn.execute_batch(&format!(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-refund', 'Refund Tester', 'Refund Tester', '[\"sales:refund\"]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, display_name, role_id, pin_hash, is_active, created_at, updated_at) VALUES
                ('{user_id}', '{user_id}', 'Test User', 'role-refund', 'hashed', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        )).unwrap();
    }

    #[test]
    fn process_full_refund() {
        let conn = fresh_conn();
        let sale_id = seed_completed_sale(&conn);
        let store = Store::new(&conn);

        let lines = [RefundLineArg {
            sale_line_id: "sl-1".into(),
            sku: "COFFEE".into(),
            qty: 2,
            unit_price_minor: 350,
            currency: "USD".into(),
            line_total_minor: 700,
        }];

        let refund_lines: Vec<RefundLine> = lines
            .iter()
            .map(|l| {
                let currency: oz_core::Currency = l.currency.parse().unwrap();
                RefundLine::new(
                    &l.sale_line_id,
                    &l.sku,
                    l.qty,
                    Money {
                        minor_units: l.unit_price_minor,
                        currency,
                    },
                    Money {
                        minor_units: l.line_total_minor,
                        currency,
                    },
                )
            })
            .collect();

        let refund = Refund::new(
            &sale_id,
            Money {
                minor_units: 700,
                currency: "USD".parse().unwrap(),
            },
            "Customer changed mind",
            "",
            "user-1",
            refund_lines,
        );

        store.create_refund(&refund).unwrap();

        let refunds = store.list_refunds_for_sale(&sale_id).unwrap();
        assert_eq!(refunds.len(), 1);
        assert_eq!(refunds[0].total.minor_units, 700);
        assert_eq!(refunds[0].lines.len(), 1);
    }

    #[test]
    fn refund_nonexistent_sale_fails() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let lines = vec![RefundLine::new(
            "sl-x",
            "COFFEE",
            1,
            Money {
                minor_units: 350,
                currency: "USD".parse().unwrap(),
            },
            Money {
                minor_units: 350,
                currency: "USD".parse().unwrap(),
            },
        )];
        let refund = Refund::new(
            "nonexistent",
            Money {
                minor_units: 350,
                currency: "USD".parse().unwrap(),
            },
            "test",
            "",
            "user-1",
            lines,
        );
        let result = store.create_refund(&refund);
        assert!(result.is_err());
    }

    #[test]
    fn refund_with_invalid_currency_returns_error_not_silent_fallback() {
        let conn = fresh_conn();
        let sale_id = seed_completed_sale(&conn);
        seed_user_with_refund_permission(&conn, "user-refund-tester");

        let lines = [RefundLineArg {
            sale_line_id: "sl-1".into(),
            sku: "COFFEE".into(),
            qty: 2,
            unit_price_minor: 350,
            currency: "INVALID_ZZZ".into(),
            line_total_minor: 700,
        }];

        let result =
            run_process_refund(&conn, "user-refund-tester", &sale_id, "test", None, &lines);
        // The bug: `unwrap_or(sale.currency)` silently falls back to USD
        // when the currency parse fails. After the fix, this must return
        // a proper error mentioning the invalid currency.
        assert!(
            result.is_err(),
            "refund with invalid currency 'INVALID_ZZZ' must return Err, \
             got Ok — currency parse failure was silently swallowed (bug #1)"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("invalid currency") || err.contains("INVALID_ZZZ"),
            "error should mention invalid currency, got: {err}"
        );
    }

    #[test]
    fn refund_with_valid_currency_succeeds_through_run_process_refund() {
        // Regression: the collect::<Result> refactor must not regress valid flows.
        let conn = fresh_conn();
        let sale_id = seed_completed_sale(&conn);
        seed_user_with_refund_permission(&conn, "user-valid");

        let lines = [RefundLineArg {
            sale_line_id: "sl-1".into(),
            sku: "COFFEE".into(),
            qty: 2,
            unit_price_minor: 350,
            currency: "USD".into(),
            line_total_minor: 700,
        }];

        let result = run_process_refund(&conn, "user-valid", &sale_id, "test", None, &lines);
        assert!(
            result.is_ok(),
            "valid currency must succeed, got: {:?}",
            result.err()
        );
        let r = result.unwrap();
        assert_eq!(r.total_minor, 700);
    }

    #[test]
    fn refund_line_arg_deserialize() {
        let json = r#"{"sale_line_id":"sl-1","sku":"CAKE","qty":1,"unit_price_minor":500,"currency":"USD","line_total_minor":500}"#;
        let arg: RefundLineArg = serde_json::from_str(json).unwrap();
        assert_eq!(arg.sale_line_id, "sl-1");
        assert_eq!(arg.sku, "CAKE");
        assert_eq!(arg.qty, 1);
        assert_eq!(arg.unit_price_minor, 500);
        assert_eq!(arg.line_total_minor, 500);
    }

    #[test]
    fn process_refund_args_deserialize() {
        let json = r#"{"sale_id":"s1","reason":"damaged","note":"box was crushed","user_id":"u1","lines":[{"sale_line_id":"sl-1","sku":"CAKE","qty":1,"unit_price_minor":500,"currency":"USD","line_total_minor":500}]}"#;
        let args: ProcessRefundArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sale_id, "s1");
        assert_eq!(args.reason, "damaged");
        assert_eq!(args.note, Some("box was crushed".into()));
        assert_eq!(args.lines.len(), 1);
        assert_eq!(args.lines[0].sku, "CAKE");
    }

    #[test]
    fn process_refund_result_serialize() {
        let result = ProcessRefundResult {
            refund_id: "ref-1".into(),
            total_minor: 1500,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("ref-1"));
        assert!(json.contains("1500"));
    }
}

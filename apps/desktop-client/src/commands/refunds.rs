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
pub struct RefundLineArg {
    pub sale_line_id: String,
    pub sku: String,
    pub qty: i64,
    pub unit_price_minor: i64,
    pub currency: String,
    pub line_total_minor: i64,
}

#[derive(Debug, Deserialize)]
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

/// Args for `process_refund_scoped` — identical to `ProcessRefundArgs`
/// but without `user_id` (read from the session token instead).
#[derive(Debug, Deserialize)]
pub struct ProcessRefundScopedArgs {
    pub sale_id: String,
    pub reason: String,
    pub note: Option<String>,
    pub lines: Vec<RefundLineArg>,
}

#[derive(Debug, Serialize)]
pub struct ProcessRefundResult {
    pub refund_id: String,
    pub total_minor: i64,
}

/// Process a refund against a completed sale using the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `process_refund_scoped`
/// with a `session_token` instead. The `user_id` is read from the
/// resolved session.
#[command]
pub async fn process_refund(
    args: ProcessRefundArgs,
    state: State<'_, AppState>,
) -> Result<ProcessRefundResult, AppError> {
    let db = state.db.lock().await;
    run_process_refund(
        &db,
        &args.sale_id,
        &args.reason,
        args.note.as_deref(),
        &args.user_id,
        &args.lines,
    )
}

/// Process a refund within the store resolved from a session token.
///
/// ADR #7: Scoped variant of `process_refund`. The `user_id` for
/// permission checks and the refund record is read from the resolved
/// `SessionContext`.
#[command]
pub async fn process_refund_scoped(
    session_token: String,
    args: ProcessRefundScopedArgs,
    state: State<'_, AppState>,
) -> Result<ProcessRefundResult, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_process_refund(
        &db,
        &args.sale_id,
        &args.reason,
        args.note.as_deref(),
        &session.user_id,
        &args.lines,
    )
}

/// Shared business logic for processing a refund.
fn run_process_refund(
    db: &rusqlite::Connection,
    sale_id: &str,
    reason: &str,
    note: Option<&str>,
    user_id: &str,
    lines: &[RefundLineArg],
) -> Result<ProcessRefundResult, AppError> {
    let store = Store::new(db);

    // Permission check: caller must have sales:refund.
    require_permission_for_user(&store, user_id, permissions::SALES_REFUND)?;

    // Verify the sale exists and is completed.
    let sale = store
        .get_sale(sale_id)?
        .ok_or_else(|| AppError::Invalid(format!("sale {} not found", sale_id)))?;
    if sale.status != oz_core::SaleStatus::Completed {
        return Err(AppError::Invalid(format!(
            "cannot refund a sale with status {:?}; only completed sales can be refunded",
            sale.status
        )));
    }

    // Build refund domain objects.
    let refund_lines: Vec<RefundLine> = lines
        .iter()
        .map(|l| {
            let currency: oz_core::Currency = l.currency.parse().unwrap_or(sale.currency);
            let unit_price = Money {
                minor_units: l.unit_price_minor,
                currency,
            };
            let line_total = Money {
                minor_units: l.line_total_minor,
                currency,
            };
            RefundLine::new(&l.sale_line_id, &l.sku, l.qty, unit_price, line_total)
        })
        .collect();

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

    tracing::info!(
        refund_id = %refund.id,
        sale_id,
        total_minor,
        reason,
        "refund processed"
    );

    Ok(ProcessRefundResult {
        refund_id: refund.id,
        total_minor,
    })
}

/// Look up a sale by its receipt barcode from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `lookup_sale_by_receipt_barcode_scoped`.
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

/// Look up a sale by receipt barcode from the store resolved from a session token.
///
/// ADR #7: Scoped variant of `lookup_sale_by_receipt_barcode`.
#[command]
pub async fn lookup_sale_by_receipt_barcode_scoped(
    session_token: String,
    barcode: String,
    state: State<'_, AppState>,
) -> Result<Option<Sale>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let sale = store.lookup_sale_by_receipt_barcode(&barcode)?;
    drop(db);
    Ok(sale)
}

/// List all refunds for a sale from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_refunds_scoped`.
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

/// List all refunds for a sale from the store resolved from a session token.
///
/// ADR #7: Scoped variant of `list_refunds`.
#[command]
pub async fn list_refunds_scoped(
    session_token: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Refund>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
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

    // ── DTO struct tests ─────────────────────────────────────────────

    #[test]
    fn refund_line_arg_fields() {
        let arg = RefundLineArg {
            sale_line_id: "sl-1".into(),
            sku: "COFFEE".into(),
            qty: 2,
            unit_price_minor: 350,
            currency: "USD".into(),
            line_total_minor: 700,
        };
        assert_eq!(arg.sale_line_id, "sl-1");
        assert_eq!(arg.sku, "COFFEE");
        assert_eq!(arg.qty, 2);
        assert_eq!(arg.unit_price_minor, 350);
        assert_eq!(arg.currency, "USD");
        assert_eq!(arg.line_total_minor, 700);
    }

    #[test]
    fn refund_line_arg_debug() {
        let arg = RefundLineArg {
            sale_line_id: "sl-1".into(),
            sku: "COFFEE".into(),
            qty: 1,
            unit_price_minor: 100,
            currency: "USD".into(),
            line_total_minor: 100,
        };
        let debug = format!("{arg:?}");
        assert!(debug.contains("COFFEE"));
    }

    #[test]
    fn process_refund_args_debug() {
        let args = ProcessRefundArgs {
            sale_id: "sale-1".into(),
            reason: "Customer changed mind".into(),
            note: Some("Returned item".into()),
            user_id: "user-1".into(),
            lines: vec![],
        };
        let debug = format!("{args:?}");
        assert!(debug.contains("sale-1"));
        assert!(debug.contains("changed mind"));
    }

    #[test]
    fn process_refund_result_fields() {
        let result = ProcessRefundResult {
            refund_id: "ref-1".into(),
            total_minor: 700,
        };
        assert_eq!(result.refund_id, "ref-1");
        assert_eq!(result.total_minor, 700);
    }

    // ── Scoped struct & token tests ─────────────────────────────────

    #[test]
    fn process_refund_scoped_args_deserialize() {
        let json = r##"{"sale_id":"sale-1","reason":"Changed mind","note":null,"lines":[]}"##;
        let args: ProcessRefundScopedArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sale_id, "sale-1");
        assert_eq!(args.reason, "Changed mind");
        assert!(args.note.is_none());
    }

    #[test]
    fn process_refund_scoped_args_debug() {
        let args = ProcessRefundScopedArgs {
            sale_id: "sale-1".into(),
            reason: "Changed mind".into(),
            note: Some("Note".into()),
            lines: vec![],
        };
        let debug = format!("{args:?}");
        assert!(debug.contains("sale-1"));
    }

    #[test]
    fn process_refund_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }
}

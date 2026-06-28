//! Refund commands — process refund against a completed sale.

use serde::{Deserialize, Serialize};
use tauri::{command, State};

use oz_core::{Money, Refund, RefundLine};
use oz_core::db::Store;

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

#[derive(Debug, Serialize)]
pub struct ProcessRefundResult {
    pub refund_id: String,
    pub total_minor: i64,
}

/// Process a refund against a completed sale.
#[command]
pub async fn process_refund(
    args: ProcessRefundArgs,
    state: State<'_, AppState>,
) -> Result<ProcessRefundResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Verify the sale exists and is completed.
    let sale = store.get_sale(&args.sale_id)?
        .ok_or_else(|| AppError::Invalid(format!("sale {} not found", args.sale_id)))?;
    if sale.status != oz_core::SaleStatus::Completed {
        return Err(AppError::Invalid(format!(
            "cannot refund a sale with status {:?}; only completed sales can be refunded",
            sale.status
        )));
    }

    // Build refund domain objects.
    let refund_lines: Vec<RefundLine> = args.lines.iter().map(|l| {
        let currency: oz_core::Currency = l.currency.parse()
            .unwrap_or(sale.currency);
        let unit_price = Money { minor_units: l.unit_price_minor, currency };
        let line_total = Money { minor_units: l.line_total_minor, currency };
        RefundLine::new(&l.sale_line_id, &l.sku, l.qty, unit_price, line_total)
    }).collect();

    let total_minor: i64 = refund_lines.iter().map(|l| l.line_total.minor_units).sum();
    let total = Money { minor_units: total_minor, currency: sale.currency };

    let refund = Refund::new(
        &args.sale_id,
        total,
        &args.reason,
        args.note.as_deref().unwrap_or(""),
        &args.user_id,
        refund_lines,
    );

    store.create_refund(&refund)?;
    drop(db);

    tracing::info!(
        refund_id = %refund.id,
        sale_id = %args.sale_id,
        total_minor,
        reason = %args.reason,
        "refund processed"
    );

    Ok(ProcessRefundResult {
        refund_id: refund.id,
        total_minor,
    })
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

        let lines = vec![RefundLineArg {
            sale_line_id: "sl-1".into(),
            sku: "COFFEE".into(),
            qty: 2,
            unit_price_minor: 350,
            currency: "USD".into(),
            line_total_minor: 700,
        }];

        let refund_lines: Vec<RefundLine> = lines.iter().map(|l| {
            let currency: oz_core::Currency = l.currency.parse().unwrap();
            RefundLine::new(&l.sale_line_id, &l.sku, l.qty,
                Money { minor_units: l.unit_price_minor, currency },
                Money { minor_units: l.line_total_minor, currency })
        }).collect();

        let refund = Refund::new(
            &sale_id,
            Money { minor_units: 700, currency: "USD".parse().unwrap() },
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

        let lines = vec![RefundLine::new("sl-x", "COFFEE", 1,
            Money { minor_units: 350, currency: "USD".parse().unwrap() },
            Money { minor_units: 350, currency: "USD".parse().unwrap() })];
        let refund = Refund::new("nonexistent",
            Money { minor_units: 350, currency: "USD".parse().unwrap() },
            "test", "", "user-1", lines);
        let result = store.create_refund(&refund);
        assert!(result.is_err());
    }
}

//! Payment CRUD — split-payment support for completed sales.
//!
//! Each payment record represents one tender against a sale. Most sales
//! have a single payment (the legacy `payment_method` column), but split
//! payments produce multiple rows in `payments`.

use crate::Store;
use crate::error::CoreError;
use crate::money::{Currency, Money};
use crate::payment::{Payment, PaymentSplitArg};
use rusqlite::params;

impl Store<'_> {
    /// Insert one or more payment records for a sale inside a transaction.
    ///
    /// Each [`PaymentSplitArg`] produces a single row in the `payments`
    /// table. The caller is responsible for ensuring the total of all
    /// splits equals the sale total (enforced at the application layer).
    pub fn create_payments(
        &self,
        sale_id: &str,
        splits: &[PaymentSplitArg],
        currency: &Currency,
        created_at: &str,
    ) -> Result<Vec<Payment>, CoreError> {
        let mut payments = Vec::with_capacity(splits.len());
        let tx = self.conn.unchecked_transaction()?;

        let cur_str = std::str::from_utf8(&currency.0).expect("currency bytes are valid UTF-8");

        for split in splits {
            let id = uuid::Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO payments (id, sale_id, method, amount_minor, currency, created_at,
                                       gateway_reference, gateway_status, gateway_response)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    sale_id,
                    split.method,
                    split.amount_minor,
                    cur_str,
                    created_at,
                    split.gateway_reference,
                    split.gateway_status,
                    split.gateway_response,
                ],
            )?;
            payments.push(Payment {
                id,
                sale_id: sale_id.to_string(),
                method: split.method.clone(),
                amount: Money {
                    minor_units: split.amount_minor,
                    currency: *currency,
                },
                created_at: created_at.to_string(),
                gateway_reference: split.gateway_reference.clone(),
                gateway_status: split.gateway_status.clone(),
                gateway_response: split.gateway_response.clone(),
            });
        }

        tx.commit()?;
        Ok(payments)
    }

    /// Retrieve all payment records for a given sale.
    pub fn list_payments_for_sale(&self, sale_id: &str) -> Result<Vec<Payment>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sale_id, method, amount_minor, currency, created_at,
                    gateway_reference, gateway_status, gateway_response
             FROM payments WHERE sale_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![sale_id], |row| {
            let cur_str: String = row.get("currency")?;
            let currency: Currency = cur_str.parse().expect("valid currency in DB");
            Ok(Payment {
                id: row.get("id")?,
                sale_id: row.get("sale_id")?,
                method: row.get("method")?,
                amount: Money {
                    minor_units: row.get("amount_minor")?,
                    currency,
                },
                created_at: row.get("created_at")?,
                gateway_reference: row.get("gateway_reference")?,
                gateway_status: row.get("gateway_status")?,
                gateway_response: row.get("gateway_response")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Currency;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn insert_sale(conn: &Connection, id: &str, total_minor: i64, now: &str) {
        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status,
                                payment_method, tendered_minor,
                                discount_percent, discount_label, user_id,
                                created_at, updated_at,
                                subtotal_minor, tax_total_minor)
             VALUES (?1, ?2, 'USD', 1, 'completed', 'split', ?2,
                     0, NULL, 'user-1',
                     ?3, ?3,
                     ?2, 0)",
            params![id, total_minor, now],
        )
        .unwrap();
    }

    #[test]
    fn create_and_list_payments() {
        let conn = fresh();
        let store = store(&conn);

        let sale_id = uuid::Uuid::new_v4().to_string();
        let now = "2025-06-01T12:00:00Z";
        insert_sale(&conn, &sale_id, 1000, now);

        let splits = vec![
            PaymentSplitArg {
                method: "cash".into(),
                amount_minor: 600,
                gateway_reference: None,
                gateway_status: None,
                gateway_response: None,
            },
            PaymentSplitArg {
                method: "card".into(),
                amount_minor: 400,
                gateway_reference: None,
                gateway_status: None,
                gateway_response: None,
            },
        ];

        let currency: Currency = "USD".parse().unwrap();
        let payments = store
            .create_payments(&sale_id, &splits, &currency, now)
            .unwrap();
        assert_eq!(payments.len(), 2);
        assert_eq!(payments[0].method, "cash");
        assert_eq!(payments[0].amount.minor_units, 600);
        assert_eq!(payments[1].method, "card");
        assert_eq!(payments[1].amount.minor_units, 400);

        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn create_payment_with_gateway_ref() {
        let conn = fresh();
        let store = store(&conn);

        let sale_id = uuid::Uuid::new_v4().to_string();
        let now = "2025-06-01T12:00:00Z";
        insert_sale(&conn, &sale_id, 500, now);

        let splits = vec![PaymentSplitArg {
            method: "card".into(),
            amount_minor: 500,
            gateway_reference: Some("txn_abc123".into()),
            gateway_status: Some("approved".into()),
            gateway_response: Some(r#"{"id":"txn_abc123","status":"approved"}"#.into()),
        }];

        let currency: Currency = "USD".parse().unwrap();
        let payments = store
            .create_payments(&sale_id, &splits, &currency, now)
            .unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].gateway_reference.as_deref(), Some("txn_abc123"));
        assert_eq!(payments[0].gateway_status.as_deref(), Some("approved"));

        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].gateway_reference.as_deref(), Some("txn_abc123"));
        assert_eq!(listed[0].gateway_status.as_deref(), Some("approved"));
        assert_eq!(
            listed[0].gateway_response.as_deref(),
            Some(r#"{"id":"txn_abc123","status":"approved"}"#)
        );
    }

    #[test]
    fn empty_splits_produces_no_payments() {
        let conn = fresh();
        let store = store(&conn);

        let sale_id = uuid::Uuid::new_v4().to_string();
        let now = "2025-06-01T12:00:00Z";
        insert_sale(&conn, &sale_id, 0, now);

        let currency: Currency = "USD".parse().unwrap();
        let payments = store
            .create_payments(&sale_id, &[], &currency, now)
            .unwrap();
        assert!(payments.is_empty());
    }
}

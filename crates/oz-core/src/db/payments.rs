//! Payment CRUD — split-payment support for completed sales.
//!
//! Each payment record represents one tender against a sale. Most sales
//! have a single payment (the legacy `payment_method` column), but split
//! payments produce multiple rows in `payments`.

use crate::Store;
use crate::error::CoreError;
use crate::money::{Currency, Money};
use crate::payment::{Payment, PaymentSplitArg};
use rusqlite::OptionalExtension;
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
            let id = uuid::Uuid::now_v7().to_string();

            // If the split has an idempotency key, check whether a payment
            // with this key already exists — if so, return the existing
            // payment instead of creating a duplicate.
            if let Some(ref key) = split.idempotency_key {
                let existing: Option<String> = tx
                    .query_row(
                        "SELECT id FROM payments WHERE idempotency_key = ?1",
                        params![key],
                        |row| row.get(0),
                    )
                    .optional()?
                    .flatten();
                if let Some(existing_id) = existing {
                    // Return the existing payment record — no duplicate created.
                    let existing_payment = tx.query_row(
                        "SELECT id, sale_id, method, amount_minor, currency, created_at,
                                gateway_reference, gateway_status, gateway_response, idempotency_key
                         FROM payments WHERE id = ?1",
                        params![existing_id],
                        |row| {
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
                                idempotency_key: row.get("idempotency_key")?,
                            })
                        },
                    )?;
                    payments.push(existing_payment);
                    continue;
                }
            }

            tx.execute(
                "INSERT INTO payments (id, sale_id, method, amount_minor, currency, created_at,
                                       gateway_reference, gateway_status, gateway_response, idempotency_key)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
                    split.idempotency_key,
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
                idempotency_key: split.idempotency_key.clone(),
            });
        }

        tx.commit()?;
        Ok(payments)
    }

    /// Retrieve all payment records for a given sale.
    pub fn list_payments_for_sale(&self, sale_id: &str) -> Result<Vec<Payment>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sale_id, method, amount_minor, currency, created_at,
                    gateway_reference, gateway_status, gateway_response, idempotency_key
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
                idempotency_key: row.get("idempotency_key")?,
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
        crate::migrations::fresh_db()
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

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = "2025-06-01T12:00:00Z";
        insert_sale(&conn, &sale_id, 1000, now);

        let splits = vec![
            PaymentSplitArg {
                method: "cash".into(),
                amount_minor: 600,
                gateway_reference: None,
                gateway_status: None,
                gateway_response: None,
                idempotency_key: None,
            },
            PaymentSplitArg {
                method: "card".into(),
                amount_minor: 400,
                gateway_reference: None,
                gateway_status: None,
                gateway_response: None,
                idempotency_key: None,
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

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = "2025-06-01T12:00:00Z";
        insert_sale(&conn, &sale_id, 500, now);

        let splits = vec![PaymentSplitArg {
            method: "card".into(),
            amount_minor: 500,
            gateway_reference: Some("txn_abc123".into()),
            gateway_status: Some("approved".into()),
            gateway_response: Some(r#"{"id":"txn_abc123","status":"approved"}"#.into()),
            idempotency_key: None,
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

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = "2025-06-01T12:00:00Z";
        insert_sale(&conn, &sale_id, 0, now);

        let currency: Currency = "USD".parse().unwrap();
        let payments = store
            .create_payments(&sale_id, &[], &currency, now)
            .unwrap();
        assert!(payments.is_empty());
    }

    #[test]
    fn list_payments_for_nonexistent_sale() {
        let conn = fresh();
        let store = store(&conn);
        let payments = store.list_payments_for_sale("nonexistent").unwrap();
        assert!(payments.is_empty());
    }

    #[test]
    fn create_single_payment() {
        let conn = fresh();
        let store = store(&conn);

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = "2025-06-01T12:00:00Z";
        insert_sale(&conn, &sale_id, 1000, now);

        let splits = vec![PaymentSplitArg {
            method: "cash".into(),
            amount_minor: 1000,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
            idempotency_key: None,
        }];

        let currency: Currency = "USD".parse().unwrap();
        let payments = store
            .create_payments(&sale_id, &splits, &currency, now)
            .unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].method, "cash");
        assert_eq!(payments[0].amount.minor_units, 1000);
        assert_eq!(payments[0].amount.currency, currency);
        assert!(!payments[0].id.is_empty());
        assert!(!payments[0].created_at.is_empty());
    }

    #[test]
    fn create_payment_zero_amount() {
        let conn = fresh();
        let store = store(&conn);

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = "2025-06-01T12:00:00Z";
        insert_sale(&conn, &sale_id, 0, now);

        let splits = vec![PaymentSplitArg {
            method: "voucher".into(),
            amount_minor: 0,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
            idempotency_key: None,
        }];

        let currency: Currency = "USD".parse().unwrap();
        let payments = store
            .create_payments(&sale_id, &splits, &currency, now)
            .unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].amount.minor_units, 0);
    }

    #[test]
    fn create_payment_large_amount() {
        let conn = fresh();
        let store = store(&conn);

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = "2025-06-01T12:00:00Z";
        insert_sale(&conn, &sale_id, i64::MAX, now);

        let splits = vec![PaymentSplitArg {
            method: "card".into(),
            amount_minor: i64::MAX,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
            idempotency_key: None,
        }];

        let currency: Currency = "USD".parse().unwrap();
        let payments = store
            .create_payments(&sale_id, &splits, &currency, now)
            .unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].amount.minor_units, i64::MAX);
    }

    #[test]
    fn create_payment_with_declined_gateway() {
        let conn = fresh();
        let store = store(&conn);

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = "2025-06-01T12:00:00Z";
        insert_sale(&conn, &sale_id, 5000, now);

        let splits = vec![PaymentSplitArg {
            method: "card".into(),
            amount_minor: 5000,
            gateway_reference: Some("txn_declined".into()),
            gateway_status: Some("declined".into()),
            gateway_response: Some(r#"{"error":"insufficient_funds"}"#.into()),
            idempotency_key: None,
        }];

        let currency: Currency = "USD".parse().unwrap();
        let payments = store
            .create_payments(&sale_id, &splits, &currency, now)
            .unwrap();
        assert_eq!(payments[0].gateway_status.as_deref(), Some("declined"));

        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed[0].gateway_status.as_deref(), Some("declined"));
    }

    #[test]
    fn list_payments_filters_by_sale() {
        let conn = fresh();
        let store = store(&conn);
        let currency: Currency = "USD".parse().unwrap();
        let now = "2025-06-01T12:00:00Z";

        let sale_a = uuid::Uuid::now_v7().to_string();
        let sale_b = uuid::Uuid::now_v7().to_string();
        insert_sale(&conn, &sale_a, 500, now);
        insert_sale(&conn, &sale_b, 300, now);

        store
            .create_payments(
                &sale_a,
                &[PaymentSplitArg {
                    method: "cash".into(),
                    amount_minor: 500,
                    gateway_reference: None,
                    gateway_status: None,
                    gateway_response: None,
                    idempotency_key: None,
                }],
                &currency,
                now,
            )
            .unwrap();
        store
            .create_payments(
                &sale_b,
                &[PaymentSplitArg {
                    method: "card".into(),
                    amount_minor: 300,
                    gateway_reference: None,
                    gateway_status: None,
                    gateway_response: None,
                    idempotency_key: None,
                }],
                &currency,
                now,
            )
            .unwrap();

        let a_payments = store.list_payments_for_sale(&sale_a).unwrap();
        assert_eq!(a_payments.len(), 1);
        assert_eq!(a_payments[0].method, "cash");

        let b_payments = store.list_payments_for_sale(&sale_b).unwrap();
        assert_eq!(b_payments.len(), 1);
        assert_eq!(b_payments[0].method, "card");
    }

    #[test]
    fn create_payments_preserves_currency() {
        let conn = fresh();
        let store = store(&conn);

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = "2025-06-01T12:00:00Z";
        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status,
                                payment_method, tendered_minor,
                                discount_percent, discount_label, user_id,
                                created_at, updated_at,
                                subtotal_minor, tax_total_minor)
             VALUES (?1, ?2, 'IDR', 1, 'completed', 'cash', ?2,
                     0, NULL, 'user-1', ?3, ?3, ?2, 0)",
            params![sale_id, 50000_i64, now],
        )
        .unwrap();

        let currency: Currency = "IDR".parse().unwrap();
        let payments = store
            .create_payments(
                &sale_id,
                &[PaymentSplitArg {
                    method: "cash".into(),
                    amount_minor: 50000,
                    gateway_reference: None,
                    gateway_status: None,
                    gateway_response: None,
                    idempotency_key: None,
                }],
                &currency,
                now,
            )
            .unwrap();
        assert_eq!(payments[0].amount.currency, currency);
        assert_eq!(payments[0].amount.currency.to_string(), "IDR");

        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed[0].amount.currency.to_string(), "IDR");
    }

    // ── Orphaned-file edge cases (from payments_new_tests.rs) ─────

    #[test]
    fn create_payments_multiple_calls_same_sale() {
        let conn = fresh();
        let store = store(&conn);
        let currency: Currency = "USD".parse().unwrap();
        let now = "2025-06-01T12:00:00Z";

        let sale_id = uuid::Uuid::now_v7().to_string();
        insert_sale(&conn, &sale_id, 3000, now);

        // First batch: cash + card
        let batch1 = store
            .create_payments(
                &sale_id,
                &[
                    PaymentSplitArg {
                        method: "cash".into(),
                        amount_minor: 1000,
                        gateway_reference: None,
                        gateway_status: None,
                        gateway_response: None,
                        idempotency_key: None,
                    },
                    PaymentSplitArg {
                        method: "card".into(),
                        amount_minor: 1000,
                        gateway_reference: Some("txn_b1".into()),
                        gateway_status: Some("approved".into()),
                        gateway_response: None,
                        idempotency_key: None,
                    },
                ],
                &currency,
                now,
            )
            .unwrap();
        assert_eq!(batch1.len(), 2);

        // Second batch: voucher
        let batch2 = store
            .create_payments(
                &sale_id,
                &[PaymentSplitArg {
                    method: "voucher".into(),
                    amount_minor: 1000,
                    gateway_reference: None,
                    gateway_status: None,
                    gateway_response: None,
                    idempotency_key: None,
                }],
                &currency,
                now,
            )
            .unwrap();
        assert_eq!(batch2.len(), 1);

        // All 3 should appear in list
        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed.len(), 3);
        let total: i64 = listed.iter().map(|p| p.amount.minor_units).sum();
        assert_eq!(total, 3000);
    }

    #[test]
    fn create_payment_negative_amount() {
        let conn = fresh();
        let store = store(&conn);
        let currency: Currency = "USD".parse().unwrap();
        let now = "2025-06-01T12:00:00Z";

        let sale_id = uuid::Uuid::now_v7().to_string();
        insert_sale(&conn, &sale_id, -500, now);

        let splits = vec![PaymentSplitArg {
            method: "cash".into(),
            amount_minor: -500,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
            idempotency_key: None,
        }];

        let payments = store
            .create_payments(&sale_id, &splits, &currency, now)
            .unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].amount.minor_units, -500);

        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed[0].amount.minor_units, -500);
    }

    #[test]
    fn payment_very_long_gateway_reference() {
        let conn = fresh();
        let store = store(&conn);
        let currency: Currency = "USD".parse().unwrap();
        let now = "2025-06-01T12:00:00Z";

        let sale_id = uuid::Uuid::now_v7().to_string();
        insert_sale(&conn, &sale_id, 1000, now);

        // 500-char gateway reference
        let long_ref = "txn_".to_owned() + &"x".repeat(496);
        assert_eq!(long_ref.len(), 500);

        let splits = vec![PaymentSplitArg {
            method: "card".into(),
            amount_minor: 1000,
            gateway_reference: Some(long_ref.clone()),
            gateway_status: Some("approved".into()),
            gateway_response: None,
            idempotency_key: None,
        }];

        let payments = store
            .create_payments(&sale_id, &splits, &currency, now)
            .unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(
            payments[0].gateway_reference.as_deref(),
            Some(long_ref.as_str())
        );

        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed[0].gateway_reference.as_deref().unwrap().len(), 500);
    }

    #[test]
    fn payment_gateway_response_large_payload() {
        let conn = fresh();
        let store = store(&conn);
        let currency: Currency = "USD".parse().unwrap();
        let now = "2025-06-01T12:00:00Z";

        let sale_id = uuid::Uuid::now_v7().to_string();
        insert_sale(&conn, &sale_id, 999, now);

        // Large JSON payload (10 KB)
        let large_body = "x".repeat(10_000);
        let large_response = format!(
            r#"{{"id":"txn_big","status":"approved","data":"{}"}}"#,
            large_body
        );
        assert!(large_response.len() > 10_000);

        let splits = vec![PaymentSplitArg {
            method: "card".into(),
            amount_minor: 999,
            gateway_reference: Some("txn_big".into()),
            gateway_status: Some("approved".into()),
            gateway_response: Some(large_response.clone()),
            idempotency_key: None,
        }];

        let payments = store
            .create_payments(&sale_id, &splits, &currency, now)
            .unwrap();
        assert_eq!(payments.len(), 1);
        assert!(payments[0].gateway_response.as_deref().unwrap().len() > 10_000);

        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert!(
            listed[0]
                .gateway_response
                .as_deref()
                .unwrap()
                .contains("txn_big")
        );
    }

    #[test]
    fn payment_currency_different_from_sale() {
        let conn = fresh();
        let store = store(&conn);
        let now = "2025-06-01T12:00:00Z";

        let sale_id = uuid::Uuid::now_v7().to_string();
        // Sale recorded in USD
        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status,
                                payment_method, tendered_minor,
                                discount_percent, discount_label, user_id,
                                created_at, updated_at,
                                subtotal_minor, tax_total_minor)
             VALUES (?1, ?2, 'USD', 1, 'completed', 'split', ?2,
                     0, NULL, 'user-1', ?3, ?3, ?2, 0)",
            params![sale_id, 3000, now],
        )
        .unwrap();

        // Payment in different currency
        let currency: Currency = "IDR".parse().unwrap();
        let splits = vec![PaymentSplitArg {
            method: "card".into(),
            amount_minor: 45000,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
            idempotency_key: None,
        }];

        let payments = store
            .create_payments(&sale_id, &splits, &currency, now)
            .unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].amount.currency.to_string(), "IDR");
        assert_eq!(payments[0].amount.minor_units, 45000);

        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed[0].amount.currency.to_string(), "IDR");
    }

    // ── Idempotency key tests (P5-2) ─────────────────────────────

    #[test]
    fn create_payments_with_idempotency_key_dedup() {
        let conn = fresh();
        let store = store(&conn);
        let currency: Currency = "USD".parse().unwrap();
        let now = "2025-06-01T12:00:00Z";

        let sale_id = uuid::Uuid::now_v7().to_string();
        insert_sale(&conn, &sale_id, 300, now);

        let ik = uuid::Uuid::now_v7().to_string();

        // First call — creates the payment with idempotency_key
        let splits1 = vec![PaymentSplitArg {
            method: "card".into(),
            amount_minor: 300,
            gateway_reference: Some("txn_first".into()),
            gateway_status: Some("approved".into()),
            gateway_response: None,
            idempotency_key: Some(ik.clone()),
        }];
        let payments1 = store
            .create_payments(&sale_id, &splits1, &currency, now)
            .unwrap();
        assert_eq!(payments1.len(), 1);
        assert_eq!(payments1[0].gateway_reference.as_deref(), Some("txn_first"));
        assert_eq!(payments1[0].idempotency_key.as_deref(), Some(ik.as_str()));

        // Second call — same idempotency_key, should return existing payment (dedup)
        let splits2 = vec![PaymentSplitArg {
            method: "card".into(),
            amount_minor: 999, // different amount, should be ignored due to dedup
            gateway_reference: Some("txn_second".into()),
            gateway_status: Some("approved".into()),
            gateway_response: None,
            idempotency_key: Some(ik.clone()),
        }];
        let payments2 = store
            .create_payments(&sale_id, &splits2, &currency, now)
            .unwrap();
        assert_eq!(payments2.len(), 1);
        // Should return the original payment, not the new one
        assert_eq!(payments2[0].gateway_reference.as_deref(), Some("txn_first"));
        assert_eq!(payments2[0].amount.minor_units, 300);

        // Only 1 payment should exist in the DB
        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].gateway_reference.as_deref(), Some("txn_first"));
    }

    #[test]
    fn create_payments_different_idempotency_keys_create_separate_records() {
        let conn = fresh();
        let store = store(&conn);
        let currency: Currency = "USD".parse().unwrap();
        let now = "2025-06-01T12:00:00Z";

        let sale_id = uuid::Uuid::now_v7().to_string();
        insert_sale(&conn, &sale_id, 1000, now);

        let ik1 = uuid::Uuid::now_v7().to_string();
        let ik2 = uuid::Uuid::now_v7().to_string();
        assert_ne!(ik1, ik2);

        // Two different sales with different idempotency keys — both should be created
        store
            .create_payments(
                &sale_id,
                &[PaymentSplitArg {
                    method: "cash".into(),
                    amount_minor: 500,
                    gateway_reference: None,
                    gateway_status: None,
                    gateway_response: None,
                    idempotency_key: Some(ik1),
                }],
                &currency,
                now,
            )
            .unwrap();

        store
            .create_payments(
                &sale_id,
                &[PaymentSplitArg {
                    method: "card".into(),
                    amount_minor: 500,
                    gateway_reference: None,
                    gateway_status: None,
                    gateway_response: None,
                    idempotency_key: Some(ik2),
                }],
                &currency,
                now,
            )
            .unwrap();

        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn create_payments_idempotency_key_none_allows_duplicates() {
        // Without idempotency key, duplicate payments are NOT deduplicated
        // (no implicit dedup for None keys)
        let conn = fresh();
        let store = store(&conn);
        let currency: Currency = "USD".parse().unwrap();
        let now = "2025-06-01T12:00:00Z";

        let sale_id = uuid::Uuid::now_v7().to_string();
        insert_sale(&conn, &sale_id, 2000, now);

        store
            .create_payments(
                &sale_id,
                &[PaymentSplitArg {
                    method: "cash".into(),
                    amount_minor: 1000,
                    gateway_reference: None,
                    gateway_status: None,
                    gateway_response: None,
                    idempotency_key: None,
                }],
                &currency,
                now,
            )
            .unwrap();

        // Same data, no idempotency key — creates a new record
        store
            .create_payments(
                &sale_id,
                &[PaymentSplitArg {
                    method: "cash".into(),
                    amount_minor: 1000,
                    gateway_reference: None,
                    gateway_status: None,
                    gateway_response: None,
                    idempotency_key: None,
                }],
                &currency,
                now,
            )
            .unwrap();

        let listed = store.list_payments_for_sale(&sale_id).unwrap();
        assert_eq!(listed.len(), 2);
    }
}

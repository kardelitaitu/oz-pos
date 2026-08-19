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
    // camelCase — the exact format the frontend sends
    // (ui/src/api/sales.ts ProcessRefundScopedArgs: { saleId, reason, note, lines }).
    let json = r##"{"saleId":"sale-1","reason":"Changed mind","note":null,"lines":[]}"##;
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

// ── Bug #11: invalid currency must not silently fall back ────

/// Seed a user with refund permission so the permission check passes.
fn seed_user_with_refund_permission(conn: &Connection, user_id: &str) {
    conn.execute_batch(&format!(
        "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
            ('role-refund', 'Refund Tester', 'Refund Tester', '[\"sales:refund\"]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO users (id, username, display_name, role_id, pin_hash, is_active, created_at, updated_at) VALUES
            ('{user_id}', '{user_id}', 'Test User', 'role-refund', 'hashed', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
    )).unwrap();
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

    let result = run_process_refund(&conn, &sale_id, "test", None, "user-refund-tester", &lines);
    // Bug #11: .unwrap_or(sale.currency) silently fell back to USD.
    // After the fix, this must return an error mentioning the invalid currency.
    assert!(
        result.is_err(),
        "refund with invalid currency 'INVALID_ZZZ' must return Err, \
         got Ok — currency parse failure was silently swallowed (bug #11)"
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

    let result = run_process_refund(&conn, &sale_id, "test", None, "user-valid", &lines);
    assert!(
        result.is_ok(),
        "valid currency must succeed, got: {:?}",
        result.err()
    );
    let r = result.unwrap();
    assert_eq!(r.total_minor, 700);
}

// ── Frontend camelCase parity (Bug #13) ──────────────────────────────
//
// The frontend (ui/src/api/sales.ts) sends camelCase keys:
//   RefundLineArg:      { saleLineId, sku, qty, unitPriceMinor, currency, lineTotalMinor }
//   ProcessRefundArgs:  { saleId, reason, note, userId, lines }
//   ProcessRefundScopedArgs: { saleId, reason, note, lines }
// wrapped in { args: { ... } }. Tauri does NOT rename struct fields
// — serde uses the exact field names. Without #[serde(rename_all =
// "camelCase")], serde looks for "sale_line_id"/"unit_price_minor"/
// "line_total_minor"/"sale_id"/"user_id" and fails on the real
// frontend payload, breaking every refund call.

#[test]
fn refund_line_arg_deserialize_frontend_camelcase() {
    let json = r##"{"saleLineId":"sl-1","sku":"COFFEE","qty":2,"unitPriceMinor":350,"currency":"USD","lineTotalMinor":700}"##;
    let line: RefundLineArg = serde_json::from_str(json)
        .expect("RefundLineArg must accept the frontend's camelCase payload");
    assert_eq!(line.sale_line_id, "sl-1");
    assert_eq!(line.unit_price_minor, 350);
    assert_eq!(line.line_total_minor, 700);
}

#[test]
fn process_refund_args_deserialize_frontend_camelcase() {
    let json = r##"{"saleId":"sale-1","reason":"Customer return","note":"damaged","userId":"u1","lines":[{"saleLineId":"sl-1","sku":"COFFEE","qty":2,"unitPriceMinor":350,"currency":"USD","lineTotalMinor":700}]}"##;
    let args: ProcessRefundArgs = serde_json::from_str(json)
        .expect("ProcessRefundArgs must accept the frontend's camelCase payload");
    assert_eq!(args.sale_id, "sale-1");
    assert_eq!(args.user_id, "u1");
    assert_eq!(args.lines.len(), 1);
    assert_eq!(args.lines[0].sale_line_id, "sl-1");
}

#[test]
fn process_refund_scoped_args_deserialize_frontend_camelcase() {
    let json = r##"{"saleId":"sale-1","reason":"Customer return","note":null,"lines":[]}"##;
    let args: ProcessRefundScopedArgs = serde_json::from_str(json)
        .expect("ProcessRefundScopedArgs must accept the frontend's camelCase payload");
    assert_eq!(args.sale_id, "sale-1");
    assert_eq!(args.reason, "Customer return");
    assert!(args.lines.is_empty());
}

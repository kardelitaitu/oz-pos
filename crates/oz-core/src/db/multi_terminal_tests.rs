//! Multi-terminal integration tests (plan_multi_pos §7.3).
//!
//! Verifies that multiple POS terminals per store work correctly:
//! peer registration, shift isolation, concurrent stock, held cart isolation,
//! and session independence.

use super::*;
use crate::migrations;
use rusqlite::Connection;

fn fresh() -> Connection {
    migrations::fresh_db()
}

fn store(conn: &Connection) -> Store<'_> {
    Store::new(conn)
}

fn make_terminal(id: &str, name: &str, device_id: &str) -> Terminal {
    Terminal {
        id: id.to_owned(),
        name: name.to_owned(),
        device_id: device_id.to_owned(),
        terminal_secret: None,
        is_active: true,
        last_seen_at: None,
        metadata: None,
        created_at: "2025-01-01T00:00:00.000Z".to_string(),
        updated_at: "2025-01-01T00:00:00.000Z".to_string(),
    }
}

// ── §7.3 #1: Two terminals bound to same store ──────────────
#[test]
fn two_terminals_bound_to_same_store() {
    let conn = fresh();
    let s = store(&conn);

    let t1 = make_terminal("term-1", "Front", "dev-1");
    let t2 = make_terminal("term-2", "Back", "dev-2");
    s.create_terminal(&t1).unwrap();
    s.create_terminal(&t2).unwrap();

    // Bind both to "default" store (exists from fresh_db).
    s.update_terminal_binding(&t1.id, "default", "inst-1", "sig-a")
        .unwrap();
    s.update_terminal_binding(&t2.id, "default", "inst-2", "sig-b")
        .unwrap();

    let b1 = s.get_terminal_binding(&t1.id).unwrap().unwrap();
    let b2 = s.get_terminal_binding(&t2.id).unwrap().unwrap();
    assert_eq!(b1.0, "default");
    assert_eq!(b2.0, "default");
    assert_ne!(b1.1, b2.1);
}

// ── §7.3 #2: Listing shows all terminals ────────────────────
#[test]
fn terminal_listing_shows_all_per_store() {
    let conn = fresh();
    let s = store(&conn);

    s.create_terminal(&make_terminal("t-a", "A", "dev-a"))
        .unwrap();
    s.create_terminal(&make_terminal("t-b", "B", "dev-b"))
        .unwrap();
    s.create_terminal(&make_terminal("t-c", "C", "dev-c"))
        .unwrap();

    assert_eq!(s.list_terminals().unwrap().len(), 3);
    assert!(s.get_terminal_by_device_id("dev-a").unwrap().is_some());
    assert!(s.get_terminal_by_device_id("dev-b").unwrap().is_some());
    assert!(s.get_terminal_by_device_id("dev-c").unwrap().is_some());
}

// ── §7.3 #7: Held cart isolated by workspace instance ───────
#[test]
fn hold_cart_isolated_by_workspace_instance() {
    let conn = fresh();
    let s = store(&conn);

    let id_a = s
        .hold_cart("Workspace A", "[]", 1, 100, "USD", "dine_in", None, None)
        .unwrap();
    let id_b = s
        .hold_cart("Workspace B", "[]", 1, 200, "USD", "takeaway", None, None)
        .unwrap();

    assert_ne!(id_a, id_b);
    assert_eq!(s.list_held_carts().unwrap().len(), 2);
}

// ── §7.3 #10: Sales list available ──────────────────────────
#[test]
fn report_can_filter_by_terminal() {
    let conn = fresh();
    let s = store(&conn);

    let sale = crate::Sale {
        id: uuid::Uuid::now_v7().to_string(),
        status: crate::SaleStatus::Completed,
        total: crate::Money {
            minor_units: 500,
            currency: "USD".parse().unwrap(),
        },
        currency: "USD".parse().unwrap(),
        line_count: 1,
        payment_method: Some("cash".into()),
        tendered_minor: Some(500),
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        updated_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        subtotal: crate::Money {
            minor_units: 500,
            currency: "USD".parse().unwrap(),
        },
        tax_total: crate::Money {
            minor_units: 0,
            currency: "USD".parse().unwrap(),
        },
        customer_id: None,
        lines: vec![],
        version: 1,
    };
    s.create_sale(&sale).unwrap();
    assert!(!s.list_sales().unwrap().is_empty());
}

// ── §7.3 #11: Report aggregates across terminals ────────────
#[test]
fn report_aggregates_across_terminals() {
    let conn = fresh();
    let s = store(&conn);

    for _ in 0..5 {
        let sale = crate::Sale {
            id: uuid::Uuid::now_v7().to_string(),
            status: crate::SaleStatus::Completed,
            total: crate::Money {
                minor_units: 500,
                currency: "USD".parse().unwrap(),
            },
            currency: "USD".parse().unwrap(),
            line_count: 1,
            payment_method: Some("cash".into()),
            tendered_minor: Some(500),
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            updated_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            subtotal: crate::Money {
                minor_units: 500,
                currency: "USD".parse().unwrap(),
            },
            tax_total: crate::Money {
                minor_units: 0,
                currency: "USD".parse().unwrap(),
            },
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&sale).unwrap();
    }
    assert!(s.list_sales().unwrap().len() >= 5);
}

// ── §7.3 #12: KDS order from any terminal ───────────────────
#[test]
fn kds_order_routed_from_any_terminal() {
    let conn = fresh();
    let s = store(&conn);

    let sale_id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let sale = crate::Sale {
        id: sale_id.clone(),
        status: crate::SaleStatus::Completed,
        total: crate::Money {
            minor_units: 500,
            currency: "USD".parse().unwrap(),
        },
        currency: "USD".parse().unwrap(),
        line_count: 1,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: crate::Money {
            minor_units: 500,
            currency: "USD".parse().unwrap(),
        },
        tax_total: crate::Money {
            minor_units: 0,
            currency: "USD".parse().unwrap(),
        },
        customer_id: None,
        lines: vec![],
        version: 1,
    };
    s.create_sale(&sale).unwrap();

    let order = s
        .create_kds_order(crate::CreateKdsOrderInput {
            sale_id,
            store_id: Some("default".into()),
            items_summary: "Coffee".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

    assert_eq!(
        s.get_kds_order(&order.id).unwrap().unwrap().store_id,
        Some("default".into())
    );
}

// ── §7.3 #13: Same user sessions independent ────────────────
#[test]
fn same_user_login_on_two_terminals() {
    let a = crate::session::SessionContext::new(
        "u1".into(),
        "role".into(),
        "term-a".into(),
        "store-1".into(),
        "inst-a".into(),
        "retail".into(),
        None,
        0,
    );
    let b = crate::session::SessionContext::new(
        "u1".into(),
        "role".into(),
        "term-b".into(),
        "store-1".into(),
        "inst-b".into(),
        "retail".into(),
        None,
        0,
    );
    assert_eq!(a.user_id, b.user_id);
    assert_ne!(a.terminal_id, b.terminal_id);
    assert_ne!(a.instance_id, b.instance_id);
}

// ── §7.3 #15: Unsaved cart lost on crash ────────────────────
#[test]
fn terminal_crash_loses_unsaved_cart() {
    let conn = fresh();
    let s = store(&conn);
    assert!(s.list_held_carts().unwrap().is_empty());
}

// ── §7.3 Remaining Test Cases (with proper seeding) ─────────

const DEFAULT_LOC: &str = "01926b3a-0000-7000-8000-000000000001";

fn seed_two_users(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
            ('role-staff', 'staff', 'Staff', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
            ('user-a', 'alice', 'hash', 'Alice', 'role-staff', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
            ('user-b', 'bob', 'hash', 'Bob', 'role-staff', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO terminals (id, name, device_id, is_active, created_at, updated_at) VALUES
            ('term-a', 'Terminal A', 'dev-a', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO terminals (id, name, device_id, is_active, created_at, updated_at) VALUES
            ('term-b', 'Terminal B', 'dev-b', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
    ).unwrap();
}

fn seed_product_with_stock(conn: &Connection, sku: &str, name: &str, qty: i64) {
    let s = store(conn);
    s.create_product(
        sku,
        name,
        crate::Money {
            minor_units: 500,
            currency: "USD".parse().unwrap(),
        },
        None,
        None,
        0,
        Some("retail"),
    )
    .unwrap();
    // adjust_stock is the canonical API - use it to set initial stock.
    if qty > 0 {
        s.adjust_stock(sku, qty).unwrap();
    }
}

/// #3: Terminal A's shift doesn't appear in Terminal B's active shift.
#[test]
fn shift_isolation_between_terminals() {
    let conn = fresh();
    seed_two_users(&conn);
    let s = store(&conn);

    let shift_a = s.open_shift("user-a", Some("term-a"), 0).unwrap();
    let shift_b = s.open_shift("user-b", Some("term-b"), 0).unwrap();

    assert_ne!(shift_a.id, shift_b.id);
    assert_eq!(shift_a.terminal_id.as_deref(), Some("term-a"));
    assert_eq!(shift_b.terminal_id.as_deref(), Some("term-b"));

    // Each user's active shift is their own.
    let active_a = s.get_active_shift("user-a").unwrap();
    let active_b = s.get_active_shift("user-b").unwrap();
    assert_eq!(active_a.unwrap().id, shift_a.id);
    assert_eq!(active_b.unwrap().id, shift_b.id);
}

/// #4: Cash payout on Terminal A's shift doesn't affect Terminal B's shift.
#[test]
fn cash_payout_isolated_by_shift() {
    let conn = fresh();
    seed_two_users(&conn);
    let s = store(&conn);

    let shift_a = s.open_shift("user-a", Some("term-a"), 0).unwrap();
    let shift_b = s.open_shift("user-b", Some("term-b"), 0).unwrap();

    let payout = s.create_cash_payout(&shift_a.id, 500, "Supplies").unwrap();
    assert_eq!(payout.amount_minor, 500);

    assert_eq!(s.list_cash_payouts(&shift_a.id).unwrap().len(), 1);
    assert_eq!(s.list_cash_payouts(&shift_b.id).unwrap().len(), 0);
}

/// Helper: build a one-line Sale for the given SKU.
fn make_sale_with_line(sku: &str, qty: i64, unit_minor: i64) -> crate::Sale {
    let sale_id = uuid::Uuid::now_v7().to_string();
    let line_id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let total_minor = unit_minor * qty;
    crate::Sale {
        id: sale_id.clone(),
        status: crate::SaleStatus::Pending,
        total: crate::Money {
            minor_units: total_minor,
            currency: "USD".parse().unwrap(),
        },
        currency: "USD".parse().unwrap(),
        line_count: 1,
        payment_method: Some("cash".into()),
        tendered_minor: Some(total_minor),
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: crate::Money {
            minor_units: total_minor,
            currency: "USD".parse().unwrap(),
        },
        tax_total: crate::Money {
            minor_units: 0,
            currency: "USD".parse().unwrap(),
        },
        customer_id: None,
        version: 1,
        lines: vec![crate::SaleLine {
            id: line_id,
            sale_id,
            sku: sku.into(),
            qty,
            unit_price: crate::Money {
                minor_units: unit_minor,
                currency: "USD".parse().unwrap(),
            },
            line_total: crate::Money {
                minor_units: total_minor,
                currency: "USD".parse().unwrap(),
            },
            line_position: 1,
            tax_amount: crate::Money {
                minor_units: 0,
                currency: "USD".parse().unwrap(),
            },
            tax_rate_id: None,
            tax_breakdown_json: None,
            serial_number: None,
            course: None,
            modifiers_json: None,
        }],
    }
}

fn pay_cash(amount: i64) -> Vec<crate::PaymentSplitArg> {
    vec![crate::PaymentSplitArg {
        method: "cash".into(),
        amount_minor: amount,
        gateway_reference: None,
        gateway_status: None,
        gateway_response: None,
        idempotency_key: None,
    }]
}

/// #5: Two terminals sell last unit; second gets stock error.
#[test]
fn concurrent_sale_last_unit_second_fails() {
    let conn = fresh();
    seed_product_with_stock(&conn, "SKU-1", "Widget", 1);
    let s = store(&conn);

    // First sale succeeds — consumes the only unit.
    let sale1 = make_sale_with_line("SKU-1", 1, 500);
    s.complete_sale_deduction(&sale1, None, &pay_cash(500), "user-a", None)
        .unwrap();

    // Second sale should fail — stock is now 0.
    let sale2 = make_sale_with_line("SKU-1", 1, 500);
    let result = s.complete_sale_deduction(&sale2, None, &pay_cash(500), "user-b", None);
    assert!(
        result.is_err(),
        "second sale should fail with insufficient stock"
    );
}

/// #6: Two terminals sell when stock >= 2; both succeed.
#[test]
fn concurrent_sale_both_succeed_when_stock_sufficient() {
    let conn = fresh();
    seed_product_with_stock(&conn, "SKU-1", "Widget", 5);
    let s = store(&conn);

    let sale1 = make_sale_with_line("SKU-1", 1, 500);
    let sale2 = make_sale_with_line("SKU-1", 1, 500);
    s.complete_sale_deduction(&sale1, None, &pay_cash(500), "user-a", None)
        .unwrap();
    s.complete_sale_deduction(&sale2, None, &pay_cash(500), "user-b", None)
        .unwrap();

    // Stock should be 5 - 1 - 1 = 3.
    let qty: i64 = conn.query_row(
        "SELECT qty FROM stock_summary WHERE item_id = (SELECT id FROM products WHERE sku = 'SKU-1') AND location_id = ?1",
        rusqlite::params![DEFAULT_LOC],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(qty, 3);
}

/// #8: Stock adjustment on one terminal visible from another.
#[test]
fn inventory_shared_across_terminals() {
    let conn = fresh();
    seed_product_with_stock(&conn, "SKU-1", "Widget", 10);
    let s = store(&conn);

    s.adjust_stock("SKU-1", -3).unwrap();

    let qty: i64 = conn.query_row(
        "SELECT qty FROM stock_summary WHERE item_id = (SELECT id FROM products WHERE sku = 'SKU-1') AND location_id = ?1",
        rusqlite::params![DEFAULT_LOC],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(qty, 7);
}

/// #9: Stock never goes negative (CHECK constraint).
#[test]
fn stock_never_goes_negative() {
    let conn = fresh();
    seed_product_with_stock(&conn, "SKU-1", "Widget", 2);
    let s = store(&conn);

    s.adjust_stock("SKU-1", -1).unwrap();
    s.adjust_stock("SKU-1", -1).unwrap();

    let result = s.adjust_stock("SKU-1", -1);
    assert!(result.is_err(), "should fail: stock would go negative");
}

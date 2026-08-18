use super::*;
use crate::migrations;
use crate::{Cart, CartLine, Sku};
use rusqlite::Connection;
use std::collections::HashSet;

fn fresh() -> Connection {
    migrations::fresh_db()
}

fn store(conn: &Connection) -> Store<'_> {
    Store::new(conn)
}

fn usd() -> Currency {
    "USD".parse().unwrap()
}

fn price(minor: i64) -> Money {
    Money {
        minor_units: minor,
        currency: usd(),
    }
}

fn make_cart() -> Cart {
    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("COFFEE"), 2, price(350)))
        .unwrap();
    cart.add_line(CartLine::new(Sku::new("BAGEL"), 1, price(450)))
        .unwrap();
    cart
}

// ── Sale CRUD ────────────────────────────────────────────────

#[test]
fn create_sale_persists_header() {
    let conn = fresh();
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    store(&conn).create_sale(&sale).unwrap();

    let loaded = store(&conn).get_sale(&sale.id).unwrap().unwrap();
    assert_eq!(loaded.id, sale.id);
    assert_eq!(loaded.status, SaleStatus::Pending);
    assert_eq!(loaded.total.minor_units, 1150);
    assert_eq!(loaded.line_count, 2);

    // The desktop Store stamps the same identity contract as the cloud
    // REST path: every sale belongs to the `default` tenant.
    let tenant: String = conn
        .query_row(
            "SELECT tenant_id FROM sales WHERE id = ?1",
            [&sale.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tenant, "default");
}

#[test]
fn test_sales_history_cap_free_tier() {
    // C1.2: the Free tier caps history to the last 30 days; the list
    // command drops older rows and flags `capped` so the UI can show the
    // upgrade teaser. Unlimited tiers return everything, uncapped.
    let conn = fresh();
    let store = store(&conn);

    let mut recent = Sale::from_cart(&make_cart()).unwrap();
    recent.created_at = chrono::Utc::now().to_rfc3339();
    let mut old = Sale::from_cart(&make_cart()).unwrap();
    old.created_at = (chrono::Utc::now() - chrono::Duration::days(40)).to_rfc3339();
    store.create_sale(&recent).unwrap();
    store.create_sale(&old).unwrap();

    // Free tier (30-day window): only the recent sale survives, capped=true.
    let (capped_sales, capped) = store.list_sales_with_history_cap(Some(30)).unwrap();
    assert!(capped, "Free tier must flag the history window as capped");
    assert_eq!(capped_sales.len(), 1, "40-day-old sale must be excluded");
    assert_eq!(capped_sales[0].id, recent.id);

    // Plus/Pro/Premium (no window): everything, capped=false.
    let (all_sales, capped) = store.list_sales_with_history_cap(None).unwrap();
    assert!(!capped);
    assert_eq!(all_sales.len(), 2);

    // The plain list_sales() path stays uncapped.
    assert_eq!(store.list_sales().unwrap().len(), 2);
}

#[test]
fn create_sale_persists_lines() {
    let conn = fresh();
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    store(&conn).create_sale(&sale).unwrap();

    let loaded = store(&conn).get_sale(&sale.id).unwrap().unwrap();
    assert_eq!(loaded.lines.len(), 2);
    assert_eq!(loaded.lines[0].sku, "COFFEE");
    assert_eq!(loaded.lines[0].qty, 2);
    assert_eq!(loaded.lines[0].unit_price.minor_units, 350);
    assert_eq!(loaded.lines[0].line_total.minor_units, 700);
    assert_eq!(loaded.lines[0].line_position, 1);
    assert_eq!(loaded.lines[1].sku, "BAGEL");
    assert_eq!(loaded.lines[1].line_position, 2);
}

#[test]
fn create_sale_snapshots_product_cost_at_checkout() {
    let conn = fresh();
    let s = store(&conn);

    // A product with a known HPP: the snapshot must freeze it into the
    // line at write time (ADR #36 reporting follow-up).
    s.create_product("STEAK", "STEAK", price(2500), None, None, 100, None)
        .unwrap();
    conn.execute(
        "UPDATE products SET cost_minor = 800 WHERE sku = 'STEAK'",
        [],
    )
    .unwrap();
    let sale = make_single_line_sale("STEAK", 2, 2500);
    s.create_sale(&sale).unwrap();

    let cost: Option<i64> = conn
        .query_row(
            "SELECT cost_minor FROM sale_lines WHERE sku = 'STEAK'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        cost,
        Some(800),
        "product cost must be frozen into the line at checkout"
    );

    // A product without a cost set (0 = unset) must snapshot as NULL,
    // never 0 — otherwise the reporting fallback to a later-set product
    // cost would be shadowed.
    s.create_product("FREE", "FREE", price(500), None, None, 100, None)
        .unwrap();
    let sale2 = make_single_line_sale("FREE", 1, 500);
    s.create_sale(&sale2).unwrap();
    let cost2: Option<i64> = conn
        .query_row(
            "SELECT cost_minor FROM sale_lines WHERE sku = 'FREE'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(cost2, None, "unset cost must snapshot as NULL, not 0");
}

#[test]
fn create_sale_empty_cart() {
    let conn = fresh();
    let cart = Cart::new(usd());
    let sale = Sale::from_cart(&cart).unwrap();
    store(&conn).create_sale(&sale).unwrap();
    let loaded = store(&conn).get_sale(&sale.id).unwrap().unwrap();
    assert_eq!(loaded.line_count, 0);
    assert_eq!(loaded.lines.len(), 0);
    assert_eq!(loaded.total.minor_units, 0);
}

// MONEY-07: create_sale is the legacy global-db import door (oz-cli
// deserializes a Sale straight from JSON payloads, bypassing CartLine's
// qty > 0 assert). Every money/qty field must be validated the same way
// the complete_sale* entry points were in MONEY-06, or a hostile import
// writes negative ledger rows.
#[test]
fn create_sale_rejects_negative_line_qty() {
    let conn = fresh();
    let mut sale = Sale::from_cart(&make_cart()).unwrap();
    sale.lines[0].qty = -2;

    let err = store(&conn).create_sale(&sale).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "qty"));
    assert!(store(&conn).get_sale(&sale.id).unwrap().is_none());
}

#[test]
fn create_sale_rejects_negative_line_total() {
    let conn = fresh();
    let mut sale = Sale::from_cart(&make_cart()).unwrap();
    sale.lines[0].line_total = price(-500);

    let err = store(&conn).create_sale(&sale).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "line_total"));
    assert!(store(&conn).get_sale(&sale.id).unwrap().is_none());
}

#[test]
fn create_sale_rejects_negative_total() {
    let conn = fresh();
    let mut sale = Sale::from_cart(&make_cart()).unwrap();
    sale.total = price(-700);

    let err = store(&conn).create_sale(&sale).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "total"));
    assert!(store(&conn).get_sale(&sale.id).unwrap().is_none());
}

#[test]
fn create_sale_rejects_negative_tendered_minor() {
    let conn = fresh();
    let mut sale = Sale::from_cart(&make_cart()).unwrap();
    sale.tendered_minor = Some(-500);

    let err = store(&conn).create_sale(&sale).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "tendered_minor"));
    assert!(store(&conn).get_sale(&sale.id).unwrap().is_none());
}

#[test]
fn create_sale_rejects_negative_subtotal() {
    let conn = fresh();
    let mut sale = Sale::from_cart(&make_cart()).unwrap();
    sale.subtotal = price(-1150);

    let err = store(&conn).create_sale(&sale).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "subtotal"));
    assert!(store(&conn).get_sale(&sale.id).unwrap().is_none());
}

#[test]
fn create_sale_rejects_negative_tax_total() {
    let conn = fresh();
    let mut sale = Sale::from_cart(&make_cart()).unwrap();
    sale.tax_total = price(-100);

    let err = store(&conn).create_sale(&sale).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "tax_total"));
    assert!(store(&conn).get_sale(&sale.id).unwrap().is_none());
}

#[test]
fn create_sale_rejects_negative_line_tax_amount() {
    let conn = fresh();
    let mut sale = Sale::from_cart(&make_cart()).unwrap();
    sale.lines[0].tax_amount = price(-10);

    let err = store(&conn).create_sale(&sale).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "tax_amount"));
    assert!(store(&conn).get_sale(&sale.id).unwrap().is_none());
}

#[test]
fn list_sales_empty_db() {
    let conn = fresh();
    let sales = store(&conn).list_sales().unwrap();
    assert!(sales.is_empty());
}

#[test]
fn list_sales_returns_all() {
    let conn = fresh();
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    store(&conn).create_sale(&sale).unwrap();

    let mut cart2 = Cart::new(usd());
    cart2
        .add_line(CartLine::new(Sku::new("TEA"), 1, price(200)))
        .unwrap();
    let sale2 = Sale::from_cart(&cart2).unwrap();
    store(&conn).create_sale(&sale2).unwrap();

    let sales = store(&conn).list_sales().unwrap();
    assert_eq!(sales.len(), 2);
    // Most recent first.
    assert_eq!(sales[0].id, sale2.id);
    assert_eq!(sales[1].id, sale.id);
    // Lines should be empty (not loaded).
    assert!(sales[0].lines.is_empty());
}

#[test]
fn get_sale_not_found() {
    let conn = fresh();
    let result = store(&conn).get_sale("nope").unwrap();
    assert!(result.is_none());
}

#[test]
fn update_sale_status_active() {
    let conn = fresh();
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    store(&conn).create_sale(&sale).unwrap();

    let updated = store(&conn)
        .update_sale_status(&sale.id, SaleStatus::Active)
        .unwrap();
    assert_eq!(updated.status, SaleStatus::Active);
    assert!(!updated.updated_at.is_empty());
}

#[test]
fn update_sale_status_full_flow() {
    let conn = fresh();
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    store(&conn).create_sale(&sale).unwrap();

    // Pending -> Active.
    let s = store(&conn)
        .update_sale_status(&sale.id, SaleStatus::Active)
        .unwrap();
    assert_eq!(s.status, SaleStatus::Active);

    // Active -> Completed.
    let s = store(&conn)
        .update_sale_status(&sale.id, SaleStatus::Completed)
        .unwrap();
    assert_eq!(s.status, SaleStatus::Completed);

    // Terminal -> rejected (Completed -> Voided is invalid).
    let err = store(&conn)
        .update_sale_status(&sale.id, SaleStatus::Voided)
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation { .. }));
}

#[test]
fn update_sale_status_not_found() {
    let conn = fresh();
    let err = store(&conn)
        .update_sale_status("nope", SaleStatus::Active)
        .unwrap_err();
    assert!(matches!(err, CoreError::NotFound { .. }));
}

#[test]
fn update_sale_status_invalid_transition() {
    let conn = fresh();
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    store(&conn).create_sale(&sale).unwrap();

    // Pending -> Completed is invalid.
    let err = store(&conn)
        .update_sale_status(&sale.id, SaleStatus::Completed)
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation { .. }));
}

// ── Export / Report ───────────────────────────────────────────

#[test]
fn export_daily_summary_empty() {
    let conn = fresh();
    let rows = store(&conn).export_daily_summary().unwrap();
    assert!(rows.is_empty(), "no sales today → empty");
}

#[test]
fn export_sales_by_hour_empty() {
    let conn = fresh();
    let rows = store(&conn).export_sales_by_hour().unwrap();
    assert!(rows.is_empty());
}

// ── Held Carts ───────────────────────────────────────────────

#[test]
fn hold_cart_creates_and_list() {
    let conn = fresh();
    let s = store(&conn);
    let id = s
        .hold_cart("Cart 1", r#"{"items":[]}"#, 0, 0, "USD", "hold", None, None)
        .unwrap();
    assert!(!id.is_empty());

    let carts = s.list_held_carts().unwrap();
    assert_eq!(carts.len(), 1);
    assert_eq!(carts[0].label, "Cart 1");
    assert_eq!(carts[0].total_minor, 0);
}

#[test]
fn hold_cart_roundtrips_deduction_location_id() {
    let conn = fresh();
    let s = store(&conn);
    // Need to insert an inventory location first (FK constraint).
    conn.execute(
        "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES ('loc-wh-a', 'Warehouse A', 'warehouse')",
        [],
    )
    .unwrap();

    let id = s
        .hold_cart(
            "Loc-Locked",
            r#"{"items":[{"sku":"COFFEE","qty":2}]}"#,
            2,
            700,
            "USD",
            "hold",
            None,
            Some("loc-wh-a"),
        )
        .unwrap();

    // Verify get_held_cart returns the deduction_location_id.
    let full = s.get_held_cart(&id).unwrap().unwrap();
    assert_eq!(
        full.deduction_location_id.as_deref(),
        Some("loc-wh-a"),
        "deduction_location_id must roundtrip through hold_cart → get_held_cart"
    );
}

#[test]
fn hold_cart_with_items() {
    let conn = fresh();
    let s = store(&conn);
    s.hold_cart(
        "Active Cart",
        r#"{"lines":[{"sku":"COFFEE","qty":2}]}"#,
        2,
        700,
        "USD",
        "hold",
        None,
        None,
    )
    .unwrap();

    let carts = s.list_held_carts().unwrap();
    assert_eq!(carts.len(), 1);
    assert_eq!(carts[0].item_count, 2);
    assert_eq!(carts[0].total_minor, 700);
    assert_eq!(carts[0].currency, "USD");
}

#[test]
fn get_held_cart_found() {
    let conn = fresh();
    let s = store(&conn);
    let id = s
        .hold_cart(
            "Test Cart",
            "{\"data\":\"value\"}",
            3,
            1500,
            "EUR",
            "hold",
            None,
            None,
        )
        .unwrap();

    let full = s.get_held_cart(&id).unwrap().unwrap();
    assert_eq!(full.label, "Test Cart");
    assert_eq!(full.cart_data, "{\"data\":\"value\"}");
    assert_eq!(full.item_count, 3);
    assert_eq!(full.total_minor, 1500);
    assert_eq!(full.currency, "EUR");
    assert!(!full.created_at.is_empty());
}

#[test]
fn get_held_cart_not_found() {
    let conn = fresh();
    let s = store(&conn);
    let result = s.get_held_cart("nonexistent-id").unwrap();
    assert!(result.is_none());
}

#[test]
fn list_held_carts_empty() {
    let conn = fresh();
    let s = store(&conn);
    let carts = s.list_held_carts().unwrap();
    assert!(carts.is_empty());
}

#[test]
fn delete_held_cart_removes() {
    let conn = fresh();
    let s = store(&conn);
    let id = s
        .hold_cart("Delete Me", "{}", 0, 0, "USD", "hold", None, None)
        .unwrap();
    s.delete_held_cart(&id).unwrap();
    let result = s.get_held_cart(&id).unwrap();
    assert!(result.is_none());
}

#[test]
fn delete_held_cart_not_found() {
    let conn = fresh();
    let s = store(&conn);
    let err = s.delete_held_cart("nope").unwrap_err();
    assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "held_cart"));
}

#[test]
fn hold_cart_strips_label_whitespace() {
    let conn = fresh();
    let s = store(&conn);
    let id = s
        .hold_cart("  My Cart  ", "{}", 0, 0, "USD", "hold", None, None)
        .unwrap();
    let full = s.get_held_cart(&id).unwrap().unwrap();
    assert_eq!(full.label, "My Cart", "label should be trimmed");
}

// ── Open Bills ───────────────────────────────────────────────

#[test]
fn open_bill_persists_across_shifts() {
    let conn = fresh();
    let s = store(&conn);

    // Seed two users and a terminal.
    conn.execute_batch(
        "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
           ('role-staff', 'staff', 'Staff', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
           ('user-morning', 'alice', 'hash', 'Alice', 'role-staff', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
           ('user-evening', 'bob', 'hash', 'Bob', 'role-staff', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
    ).unwrap();

    // ── Morning shift ──
    let shift_morning = s.open_shift("user-morning", None, 200).unwrap();

    // Create an open bill.
    let _bill_id = s
        .hold_cart(
            "Table 4 — John",
            r#"{"lines":[{"sku":"STEAK","qty":1,"unit_price":1500}]}"#,
            1,
            1500,
            "USD",
            "open_bill",
            Some("John"),
            None,
        )
        .unwrap();

    // Open bill shows up immediately.
    let open = s.list_open_bills().unwrap();
    assert_eq!(open.len(), 1, "open bill visible in same shift");
    assert_eq!(open[0].customer_name.as_deref(), Some("John"));

    // Close morning shift.
    s.close_shift(&shift_morning.id, 1700, None).unwrap();

    // ── Evening shift (different user) ──
    let _shift_evening = s.open_shift("user-evening", None, 500).unwrap();

    // The open bill is still listed — it is NOT scoped to a shift.
    let open = s.list_open_bills().unwrap();
    assert_eq!(open.len(), 1, "open bill visible across shifts");
    assert_eq!(open[0].customer_name.as_deref(), Some("John"));
    assert_eq!(open[0].total_minor, 1500);
    assert_eq!(open[0].currency, "USD");
}

#[test]
fn open_bill_list_excludes_hold_carts() {
    let conn = fresh();
    let s = store(&conn);

    s.hold_cart("Hold 1", "{}", 0, 0, "USD", "hold", None, None)
        .unwrap();
    s.hold_cart("Hold 2", "{}", 0, 0, "USD", "hold", None, None)
        .unwrap();
    s.hold_cart(
        "Table 7 — Mary",
        r#"{"lines":[]}"#,
        2,
        850,
        "USD",
        "open_bill",
        Some("Mary"),
        None,
    )
    .unwrap();

    let open = s.list_open_bills().unwrap();
    assert_eq!(open.len(), 1, "only open bills, not hold carts");
    assert_eq!(open[0].customer_name.as_deref(), Some("Mary"));
}

#[test]
fn open_bill_created_without_customer_name() {
    let conn = fresh();
    let s = store(&conn);

    let id = s
        .hold_cart("Walk-in", "{}", 0, 0, "USD", "open_bill", None, None)
        .unwrap();

    let open = s.list_open_bills().unwrap();
    assert_eq!(open.len(), 1);
    assert!(open[0].customer_name.is_none());
    assert_eq!(open[0].label, "Walk-in");

    // Verify full record.
    let full = s.get_held_cart(&id).unwrap().unwrap();
    assert_eq!(full.bill_type, "open_bill");
    assert!(full.customer_name.is_none());
}

// ── Void Sale ────────────────────────────────────────────────

#[test]
fn void_sale_changes_status_and_logs_audit() {
    let conn = fresh();
    let s = store(&conn);

    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();
    s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();

    s.void_sale(&sale.id, "user-2", "customer request").unwrap();

    let loaded = s.get_sale(&sale.id).unwrap().unwrap();
    assert_eq!(loaded.status, SaleStatus::Voided);

    let audit_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'sale.void' AND target_id = ?1",
            rusqlite::params![sale.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(audit_count, 1);
}

#[test]
fn void_sale_not_found() {
    let conn = fresh();
    let s = store(&conn);
    let err = s.void_sale("nonexistent", "user-1", "test").unwrap_err();
    assert!(matches!(err, CoreError::NotFound { .. }));
}

#[test]
fn void_sale_only_active_can_be_voided() {
    let conn = fresh();
    let s = store(&conn);
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();
    // Sale is Pending, not Active — void should fail with validation error.
    let err = s.void_sale(&sale.id, "user-1", "test").unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
}

#[test]
fn void_sale_completed_cannot_be_voided() {
    let conn = fresh();
    let s = store(&conn);
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();
    // Move to Active, then Completed.
    s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
    s.update_sale_status(&sale.id, SaleStatus::Completed)
        .unwrap();

    let err = s.void_sale(&sale.id, "user-1", "test").unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
}

// ── Export with data ─────────────────────────────────────────

#[test]
fn export_daily_summary_with_sales() {
    let conn = fresh();
    let s = store(&conn);
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();

    // Export uses date('now') so it should find the sale we just created.
    let rows = s.export_daily_summary().unwrap();
    assert!(!rows.is_empty(), "should find today's sale");
    assert_eq!(rows[0].total_minor, 1150);
}

#[test]
fn create_sale_with_user_and_discount() {
    let conn = fresh();
    let s = store(&conn);
    let mut cart = make_cart();
    cart.set_discount(
        foundation::Percentage::new(10).unwrap(),
        Some("Loyalty".into()),
    );
    let sale = Sale::from_cart(&cart).unwrap();
    // Add user_id to the sale.
    let sale_with_user = Sale {
        user_id: Some("cashier-1".into()),
        customer_id: None,
        version: 1,
        ..sale
    };
    s.create_sale(&sale_with_user).unwrap();

    let loaded = s.get_sale(&sale_with_user.id).unwrap().unwrap();
    assert_eq!(loaded.user_id, Some("cashier-1".into()));
}

#[test]
fn create_sale_discount_persisted() {
    let conn = fresh();
    let s = store(&conn);
    let mut cart = make_cart();
    cart.set_discount(foundation::Percentage::new(15).unwrap(), Some("VIP".into()));
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();

    let loaded = s.get_sale(&sale.id).unwrap().unwrap();
    assert_eq!(loaded.discount_percent, 15);
    assert_eq!(loaded.discount_label, Some("VIP".into()));
}

// ── Export with data ─────────────────────────────────────────

#[test]
fn export_sales_by_hour_with_sales() {
    let conn = fresh();
    let s = store(&conn);
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();

    let rows = s.export_sales_by_hour().unwrap();
    assert!(!rows.is_empty(), "should find today's hourly aggregation");
    assert_eq!(rows[0].sale_count, 1);
    assert_eq!(rows[0].total_minor, 1150);
}

// ── Status transition edge cases ─────────────────────────────

#[test]
fn update_sale_status_invalid_stored_status() {
    let conn = fresh();
    let s = store(&conn);
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();

    // Set a status that is valid at the SQL CHECK level ('refunded' is
    // in the CHECK constraint from migration 096) but NOT recognized
    // by SaleStatus::from_stored_str — this tests the Rust-layer
    // defensive guard against unknown stored values.
    conn.execute(
        "UPDATE sales SET status = 'refunded' WHERE id = ?1",
        rusqlite::params![sale.id],
    )
    .unwrap();

    let err = s
        .update_sale_status(&sale.id, SaleStatus::Active)
        .unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
}

// ── Void edge cases ──────────────────────────────────────────

#[test]
fn void_sale_with_unknown_sku() {
    let conn = fresh();
    let s = store(&conn);
    let cart = make_cart(); // COFFEE x 2 (350) + BAGEL x 1 (450)
    let sale = Sale::from_cart(&cart).unwrap();
    // Do NOT create products — product_id_by_sku will return None.
    s.create_sale(&sale).unwrap();
    s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();

    // Void should succeed and skip stock restoration for unknown SKUs.
    let result = s.void_sale(&sale.id, "user-1", "no product record");
    assert!(
        result.is_ok(),
        "void should succeed even when SKU has no product record"
    );
    let loaded = result.unwrap();
    assert_eq!(loaded.status, SaleStatus::Voided);
}

// ── Tax Computation ─────────────────────────────

fn seed_tax_rate(
    conn: &Connection,
    name: &str,
    rate_bps: i64,
    is_default: bool,
    is_inclusive: bool,
) -> String {
    let s = store(conn);
    s.create_tax_rate(name, rate_bps, is_default, is_inclusive)
        .unwrap()
        .id
}

fn seed_product(conn: &Connection, sku: &str, category_id: Option<&str>) -> String {
    let s = store(conn);
    let currency: crate::money::Currency = "USD".parse().unwrap();
    let money = crate::Money {
        minor_units: 1000,
        currency,
    };
    s.create_product(sku, sku, money, category_id, None, 100, None)
        .unwrap();
    sku.to_string()
}

fn seed_product_with_category(conn: &Connection, sku: &str, category_id: Option<&str>) {
    seed_product(conn, sku, category_id);
}

fn make_single_line_sale(sku: &str, qty: i64, unit_minor: i64) -> Sale {
    let line_id = uuid::Uuid::now_v7().to_string();
    let sale_id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    Sale {
        id: sale_id.clone(),
        total: price(unit_minor * qty),
        currency: usd(),
        line_count: 1,
        status: SaleStatus::Pending,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: price(unit_minor * qty),
        tax_total: price(0),
        customer_id: None,
        version: 1,
        lines: vec![SaleLine {
            id: line_id,
            sale_id,
            sku: sku.into(),
            qty,
            unit_price: price(unit_minor),
            line_total: price(unit_minor * qty),
            line_position: 1,
            tax_amount: price(0),
            tax_rate_id: None,
            tax_breakdown_json: None,
            serial_number: None,
            course: None,
            modifiers_json: None,
        }],
    }
}

// TAX-05: tests below that call `compute_sale_tax` with
// `RoundingMode::Truncate` pin the historical per-line integer-
// truncation results; new golden tests at the end of this section
// exercise `HalfUp` (the recommended default).
#[test]
fn compute_tax_no_rates() {
    let conn = fresh();
    let s = store(&conn);
    let mut sale = make_single_line_sale("COFFEE", 2, 350);
    s.compute_sale_tax(&mut sale, &[], RoundingMode::Truncate)
        .unwrap();
    assert_eq!(sale.subtotal.minor_units, 700);
    assert_eq!(sale.tax_total.minor_units, 0);
    assert_eq!(sale.lines[0].tax_amount.minor_units, 0);
    assert!(sale.lines[0].tax_rate_id.is_none());
}

#[test]
fn compute_tax_default_rate_exclusive() {
    let conn = fresh();
    let s = store(&conn);
    seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

    let mut sale = make_single_line_sale("COFFEE", 2, 350);
    s.compute_sale_tax(&mut sale, &[], RoundingMode::Truncate)
        .unwrap();
    // exclusive: tax = 700 * 1000 / 10000 = 70
    assert_eq!(sale.subtotal.minor_units, 700);
    assert_eq!(sale.tax_total.minor_units, 70);
    assert_eq!(sale.lines[0].tax_amount.minor_units, 70);
    assert!(sale.lines[0].tax_rate_id.is_some());
}

#[test]
fn compute_tax_default_rate_inclusive() {
    let conn = fresh();
    let s = store(&conn);
    seed_tax_rate(&conn, "GST 10%", 1000, true, true);

    let mut sale = make_single_line_sale("COFFEE", 2, 350);
    s.compute_sale_tax(&mut sale, &[], RoundingMode::Truncate)
        .unwrap();
    // inclusive: tax = 700 * 1000 / (10000 + 1000) = 700000 / 11000 = 63
    assert_eq!(sale.subtotal.minor_units, 700);
    assert_eq!(sale.tax_total.minor_units, 63);
    assert_eq!(sale.lines[0].tax_amount.minor_units, 63);
}

#[test]
fn compute_tax_product_level_wins() {
    let conn = fresh();
    let s = store(&conn);
    let _default_id = seed_tax_rate(&conn, "Default 5%", 500, true, false);
    let product_id = seed_tax_rate(&conn, "Product 10%", 1000, false, false);
    seed_product_with_category(&conn, "COFFEE", None);
    s.set_product_tax_rates("COFFEE", std::slice::from_ref(&product_id))
        .unwrap();

    let mut sale = make_single_line_sale("COFFEE", 1, 1000);
    s.compute_sale_tax(&mut sale, &[], RoundingMode::Truncate)
        .unwrap();
    // product rate (10%) wins over default (5%): tax = 1000 * 1000 / 10000 = 100
    assert_eq!(sale.tax_total.minor_units, 100);
    assert_eq!(
        sale.lines[0].tax_rate_id.as_deref(),
        Some(product_id.as_str())
    );
}

#[test]
fn compute_tax_category_level_wins_over_default() {
    let conn = fresh();
    let s = store(&conn);
    let _default_id = seed_tax_rate(&conn, "Default 5%", 500, true, false);
    let cat_id = seed_tax_rate(&conn, "Category 8%", 800, false, false);
    s.create_category("cat-1", "Beverages", "#fff", "").unwrap();
    s.set_category_tax_rates("cat-1", std::slice::from_ref(&cat_id))
        .unwrap();
    seed_product_with_category(&conn, "COFFEE", Some("cat-1"));

    let mut sale = make_single_line_sale("COFFEE", 1, 1000);
    s.compute_sale_tax(&mut sale, &[], RoundingMode::Truncate)
        .unwrap();
    // category rate (8%) wins over default (5%): tax = 1000 * 800 / 10000 = 80
    assert_eq!(sale.tax_total.minor_units, 80);
    assert_eq!(sale.lines[0].tax_rate_id.as_deref(), Some(cat_id.as_str()));
}

#[test]
fn compute_tax_multi_line() {
    let conn = fresh();
    let s = store(&conn);
    seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

    let line1 = SaleLine {
        id: uuid::Uuid::now_v7().to_string(),
        sale_id: "sale-1".into(),
        sku: "COFFEE".into(),
        qty: 2,
        unit_price: price(350),
        line_total: price(700),
        line_position: 1,
        tax_amount: price(0),
        tax_rate_id: None,
        tax_breakdown_json: None,
        serial_number: None,
        course: None,
        modifiers_json: None,
    };
    let line2 = SaleLine {
        id: uuid::Uuid::now_v7().to_string(),
        sale_id: "sale-1".into(),
        sku: "BAGEL".into(),
        qty: 1,
        unit_price: price(450),
        line_total: price(450),
        line_position: 2,
        tax_amount: price(0),
        tax_rate_id: None,
        tax_breakdown_json: None,
        serial_number: None,
        course: None,
        modifiers_json: None,
    };
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut sale = Sale {
        id: "sale-1".into(),
        total: price(1150),
        currency: usd(),
        line_count: 2,
        status: SaleStatus::Pending,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: price(1150),
        tax_total: price(0),
        customer_id: None,
        version: 1,
        lines: vec![line1, line2],
    };

    s.compute_sale_tax(&mut sale, &[], RoundingMode::Truncate)
        .unwrap();
    // line1: 700 * 1000 / 10000 = 70
    // line2: 450 * 1000 / 10000 = 45
    // total tax = 115
    assert_eq!(sale.subtotal.minor_units, 1150);
    assert_eq!(sale.tax_total.minor_units, 115);
    assert_eq!(sale.lines[0].tax_amount.minor_units, 70);
    assert_eq!(sale.lines[1].tax_amount.minor_units, 45);
}

#[test]
fn compute_tax_persisted_after_create() {
    let conn = fresh();
    let s = store(&conn);
    seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

    let mut sale = make_single_line_sale("COFFEE", 2, 350);
    s.compute_sale_tax(&mut sale, &[], RoundingMode::Truncate)
        .unwrap();
    s.create_sale(&sale).unwrap();

    let loaded = s.get_sale(&sale.id).unwrap().unwrap();
    assert_eq!(loaded.subtotal.minor_units, 700);
    assert_eq!(loaded.tax_total.minor_units, 70);
    assert_eq!(loaded.lines[0].tax_amount.minor_units, 70);
    assert!(loaded.lines[0].tax_rate_id.is_some());
}

#[test]
fn compute_tax_empty_sale_no_crash() {
    let conn = fresh();
    let s = store(&conn);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut sale = Sale {
        id: "empty".into(),
        total: price(0),
        currency: usd(),
        line_count: 0,
        status: SaleStatus::Pending,
        payment_method: None,
        tendered_minor: None,
        discount_percent: 0,
        discount_label: None,
        user_id: None,
        created_at: now.clone(),
        updated_at: now,
        subtotal: price(0),
        tax_total: price(0),
        customer_id: None,
        version: 1,
        lines: vec![],
    };
    s.compute_sale_tax(&mut sale, &[], RoundingMode::Truncate)
        .unwrap();
    assert_eq!(sale.subtotal.minor_units, 0);
    assert_eq!(sale.tax_total.minor_units, 0);
}

// ── TAX-05: Rounding policy golden tests ──────────────────────

#[test]
fn rounding_mode_default_is_half_up() {
    // TAX-05: HalfUp is the recommended default for new sales;
    // Truncate is preserved only for legacy reproduction.
    assert_eq!(RoundingMode::default(), RoundingMode::HalfUp);
}

#[test]
fn compute_line_tax_half_up_rounds_fractional_cents() {
    let c = usd();
    // 3333 * 1000 / 10000 = 333.3 — below the tie, both modes agree.
    assert_eq!(
        compute_line_tax(3333, 1000, false, c, RoundingMode::Truncate)
            .unwrap()
            .minor_units,
        333
    );
    assert_eq!(
        compute_line_tax(3333, 1000, false, c, RoundingMode::HalfUp)
            .unwrap()
            .minor_units,
        333
    );
    // 3335 * 1000 / 10000 = 333.5 — the tie: legacy truncates,
    // HalfUp rounds away from zero to 334.
    assert_eq!(
        compute_line_tax(3335, 1000, false, c, RoundingMode::Truncate)
            .unwrap()
            .minor_units,
        333
    );
    assert_eq!(
        compute_line_tax(3335, 1000, false, c, RoundingMode::HalfUp)
            .unwrap()
            .minor_units,
        334
    );
}

#[test]
fn compute_line_tax_half_up_inclusive() {
    let c = usd();
    // inclusive 10%: 3350 * 1000 / 11000 = 304.545…
    assert_eq!(
        compute_line_tax(3350, 1000, true, c, RoundingMode::Truncate)
            .unwrap()
            .minor_units,
        304
    );
    assert_eq!(
        compute_line_tax(3350, 1000, true, c, RoundingMode::HalfUp)
            .unwrap()
            .minor_units,
        305
    );
}

#[test]
fn compute_tax_multi_rate_line_half_up() {
    let conn = fresh();
    let s = store(&conn);
    let r1 = seed_tax_rate(&conn, "State 3%", 300, false, false);
    let r2 = seed_tax_rate(&conn, "Local 2%", 200, false, false);
    seed_product_with_category(&conn, "COFFEE", None);
    s.set_product_tax_rates("COFFEE", &[r1, r2]).unwrap();

    // base 3335: 3% = 100.05 → 100; 2% = 66.7 → 66 (Truncate) / 67 (HalfUp)
    let mut sale = make_single_line_sale("COFFEE", 1, 3335);
    s.compute_sale_tax(&mut sale, &[], RoundingMode::Truncate)
        .unwrap();
    assert_eq!(sale.tax_total.minor_units, 166);

    let mut sale2 = make_single_line_sale("COFFEE", 1, 3335);
    s.compute_sale_tax(&mut sale2, &[], RoundingMode::HalfUp)
        .unwrap();
    assert_eq!(sale2.tax_total.minor_units, 167);
}

// TAX-02: the full per-rate breakdown survives on the persisted line,
// even though `tax_rate_id` only keeps the first applicable rate.
#[test]
fn compute_tax_multi_rate_persists_breakdown_json() {
    let conn = fresh();
    let s = store(&conn);
    let r1 = seed_tax_rate(&conn, "State 3%", 300, false, false);
    let r2 = seed_tax_rate(&conn, "Local 2%", 200, false, false);
    seed_product_with_category(&conn, "COFFEE", None);
    s.set_product_tax_rates("COFFEE", &[r1.clone(), r2.clone()])
        .unwrap();

    let mut sale = make_single_line_sale("COFFEE", 1, 3335);
    s.compute_sale_tax(&mut sale, &[], RoundingMode::Truncate)
        .unwrap();

    // In-memory: breakdown carries BOTH rates with their tax amounts.
    let json = sale.lines[0].tax_breakdown_json.as_deref().unwrap();
    let breakdown: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
    assert_eq!(breakdown.len(), 2);
    assert_eq!(breakdown[0]["rate_id"], serde_json::json!(r1));
    assert_eq!(breakdown[0]["tax_minor"], 100);
    assert_eq!(breakdown[1]["rate_id"], serde_json::json!(r2));
    assert_eq!(breakdown[1]["tax_minor"], 66);
    // Legacy single-id field still points at the first rate.
    assert_eq!(sale.lines[0].tax_rate_id.as_deref(), Some(r1.as_str()));

    // Persist + reload: breakdown must survive the round-trip.
    s.create_sale(&sale).unwrap();
    let loaded = s.get_sale(&sale.id).unwrap().unwrap();
    let loaded_json = loaded.lines[0].tax_breakdown_json.as_deref().unwrap();
    let loaded_breakdown: Vec<serde_json::Value> = serde_json::from_str(loaded_json).unwrap();
    assert_eq!(loaded_breakdown, breakdown);
}

// TAX-02: Lua override lines get a breakdown entry with a null rate_id.
#[test]
fn compute_tax_override_persists_breakdown_with_null_rate_id() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_category(&conn, "COFFEE", None);

    let mut sale = make_single_line_sale("COFFEE", 2, 350);
    s.compute_sale_tax(
        &mut sale,
        &[("COFFEE".into(), 1000, false)],
        RoundingMode::Truncate,
    )
    .unwrap();

    let json = sale.lines[0].tax_breakdown_json.as_deref().unwrap();
    let breakdown: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
    assert_eq!(breakdown.len(), 1);
    assert!(breakdown[0]["rate_id"].is_null());
    assert_eq!(breakdown[0]["rate_bps"], 1000);
    assert_eq!(breakdown[0]["tax_minor"], 70);
    assert!(sale.lines[0].tax_rate_id.is_none());
}

#[test]
fn compute_cart_tax_zero_decimal_currency_jpy() {
    let conn = fresh();
    let s = store(&conn);
    let jpy: Currency = "JPY".parse().unwrap();
    seed_tax_rate(&conn, "Consumption Tax 10%", 1000, true, false);

    let lines = vec![CartLineTaxInput {
        sku: "COFFEE".into(),
        qty: 1,
        unit_price_minor: 3335,
    }];
    // JPY has no sub-unit: 3335 yen * 10% = 333.5 yen.
    let half_up = s
        .compute_cart_tax(&lines, jpy, RoundingMode::HalfUp)
        .unwrap();
    assert_eq!(half_up.minor_units, 334);
    let trunc = s
        .compute_cart_tax(&lines, jpy, RoundingMode::Truncate)
        .unwrap();
    assert_eq!(trunc.minor_units, 333);
}

// ── MONEY-01: unchecked qty × price overflow in compute_cart_tax ──
//
// CartLineTaxInput comes straight off the IPC boundary (untrusted
// renderer input) and compute_cart_tax runs on the hot path — the
// live tax preview fires on every cart change. The per-line total
// must use checked arithmetic like compute_line_tax (TAX-04); a
// wrap in release would feed a wrong tax to the register, and a
// debug build panics.
#[test]
fn compute_cart_tax_line_total_overflow_returns_validation_error() {
    let conn = fresh();
    let s = store(&conn);
    seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

    // (i64::MAX / 2) * 4 overflows i64.
    let lines = vec![CartLineTaxInput {
        sku: "COFFEE".into(),
        qty: i64::MAX / 2,
        unit_price_minor: 4,
    }];
    match s.compute_cart_tax(&lines, usd(), RoundingMode::HalfUp) {
        Err(CoreError::Validation { field, message }) => {
            assert_eq!(field, "tax");
            assert!(
                message.contains("overflow"),
                "unexpected overflow message: {message}"
            );
        }
        Err(other) => panic!("expected Validation overflow error, got: {other:?}"),
        Ok(m) => panic!("overflow must not wrap silently, got Ok({m:?})"),
    }
}

// ── MONEY-02: negative qty / unit price must be rejected ──
//
// CartLineTaxInput comes off the IPC boundary; a hostile renderer can
// send a negative qty or price. That yields a negative line total and a
// negative "tax" preview (the front-end displays it raw). The cart model
// never allows negative qty/price, so fail with a structured Validation
// error naming the offending field.
#[test]
fn compute_cart_tax_rejects_negative_qty_and_price() {
    let conn = fresh();
    let s = store(&conn);
    seed_tax_rate(&conn, "VAT 10%", 1000, true, false);

    let negative_qty = vec![CartLineTaxInput {
        sku: "COFFEE".into(),
        qty: -2,
        unit_price_minor: 350,
    }];
    match s.compute_cart_tax(&negative_qty, usd(), RoundingMode::HalfUp) {
        Err(CoreError::Validation { field, .. }) => assert_eq!(field, "qty"),
        Err(other) => panic!("expected qty Validation error, got: {other:?}"),
        Ok(m) => panic!("negative qty must be rejected, got Ok({m:?})"),
    }

    let negative_price = vec![CartLineTaxInput {
        sku: "COFFEE".into(),
        qty: 1,
        unit_price_minor: -350,
    }];
    match s.compute_cart_tax(&negative_price, usd(), RoundingMode::HalfUp) {
        Err(CoreError::Validation { field, .. }) => assert_eq!(field, "price"),
        Err(other) => panic!("expected price Validation error, got: {other:?}"),
        Ok(m) => panic!("negative price must be rejected, got Ok({m:?})"),
    }
}

#[test]
fn refund_full_amount_after_half_up_tax() {
    let conn = fresh();
    let s = store(&conn);
    seed_tax_rate(&conn, "VAT 10%", 1000, true, false);
    // Refund persistence resolves the product by SKU — seed it first.
    seed_product_with_category(&conn, "COFFEE", None);

    let mut sale = make_single_line_sale("COFFEE", 2, 350);
    s.compute_sale_tax(&mut sale, &[], RoundingMode::HalfUp)
        .unwrap();
    s.create_sale(&sale).unwrap();
    // 700 subtotal + 70 tax = 770 total.
    assert_eq!(sale.subtotal.minor_units, 700);
    assert_eq!(sale.tax_total.minor_units, 70);

    let line = crate::RefundLine::new(
        &sale.lines[0].id,
        "COFFEE",
        2,
        price(350),
        sale.lines[0].line_total,
    );
    let refund = crate::Refund::new(
        &sale.id,
        crate::Money {
            minor_units: 770,
            currency: usd(),
        },
        "full refund",
        "",
        "user-1",
        vec![line],
    );
    s.create_refund(&refund).unwrap();

    let refunds = s.list_refunds_for_sale(&sale.id).unwrap();
    assert_eq!(refunds.len(), 1);
    assert_eq!(refunds[0].total.minor_units, 770);
}

/// MONEY-02 follow-up: a hand-built `Sale` with a negative `line_total`
/// flows straight into `compute_line_tax` and records a negative tax on
/// the sale. `Sale::from_cart` only produces non-negative line totals
/// (CartLine asserts qty > 0), but this is the tax boundary — reject it
/// up front so a hostile or malformed sale cannot distort the ledger.
#[test]
fn compute_sale_tax_rejects_negative_line_total() {
    let conn = fresh();
    let s = store(&conn);
    seed_tax_rate(&conn, "VAT 10%", 1000, true, false);
    seed_product_with_category(&conn, "COFFEE", None);

    let mut sale = make_single_line_sale("COFFEE", 2, 350);
    sale.lines[0].line_total = price(-700);

    let err = s
        .compute_sale_tax(&mut sale, &[], RoundingMode::HalfUp)
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::Validation { field, .. } if field == "line_total"
    ));
}

#[test]
fn void_sale_succeeds_regardless_of_stock() {
    let conn = fresh();
    let s = store(&conn);

    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();
    s.create_sale(&sale).unwrap();
    s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();

    // Void succeeds without touching stock at all.
    s.void_sale(&sale.id, "user-1", "no stock impact").unwrap();

    let loaded = s.get_sale(&sale.id).unwrap().unwrap();
    assert_eq!(loaded.status, SaleStatus::Voided);
}

// ── complete_sale_deduction (ADR-19) ─────────────────────────

/// Seed a product and stock so that complete_sale_deduction can succeed.
fn seed_product_with_stock(conn: &Connection, sku: &str, qty: i64) -> String {
    use crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
    let product_id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
         VALUES (?1, ?2, ?3, 1000, 'USD', 'retail')",
        rusqlite::params![product_id, sku, sku],
    )
    .unwrap();
    // Seed stock at the canonical default location so the resolver finds it.
    conn.execute(
        "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) \
         VALUES (?1, ?2, ?3)",
        rusqlite::params![product_id, CANONICAL_DEFAULT_LOCATION_UUID, qty],
    )
    .unwrap();
    product_id
}

/// Seed a composite `service` product with a BOM recipe whose ingredient
/// tracks inventory. The composite does not track inventory itself, so
/// only the ingredient deduction path runs. Returns (parent, ingredient)
/// product ids.
fn seed_bom_composite(
    conn: &Connection,
    parent_sku: &str,
    ingredient_sku: &str,
    ingredient_stock: i64,
    qty_required: i64,
) -> (String, String) {
    let parent_id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO products (id, sku, name, price_minor, currency, product_type) \
         VALUES (?1, ?2, ?2, 1000, 'USD', 'service')",
        rusqlite::params![parent_id, parent_sku],
    )
    .unwrap();
    let ingredient_id = seed_product_with_stock(conn, ingredient_sku, ingredient_stock);
    conn.execute(
        "INSERT INTO product_recipes (id, parent_product_id, ingredient_product_id, \
         quantity_required, unit) VALUES (?1, ?2, ?3, ?4, 'pcs')",
        rusqlite::params![
            uuid::Uuid::now_v7().to_string(),
            parent_id,
            ingredient_id,
            qty_required,
        ],
    )
    .unwrap();
    (parent_id, ingredient_id)
}

/// Helper: seed a product with stock at TWO locations for split-fulfillment tests.
fn setup_locations_with_stock(
    conn: &Connection,
    sku: &str,
    loc_a_id: &str,
    loc_a_qty: i64,
    loc_b_id: &str,
    loc_b_qty: i64,
) -> String {
    let product_id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
         VALUES (?1, ?2, ?3, 1000, 'USD', 'retail')",
        rusqlite::params![product_id, sku, sku],
    )
    .unwrap();
    // Seed both locations into inventory_locations (creates IF NOT EXISTS).
    for loc_id in &[loc_a_id, loc_b_id] {
        conn.execute(
            "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES (?1, ?2, 'warehouse')",
            rusqlite::params![loc_id, loc_id],
        )
        .unwrap();
    }
    // Seed stock at both locations
    conn.execute(
        "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) VALUES (?1, ?2, ?3)",
        rusqlite::params![product_id, loc_a_id, loc_a_qty],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) VALUES (?1, ?2, ?3)",
        rusqlite::params![product_id, loc_b_id, loc_b_qty],
    )
    .unwrap();
    // Ensure canonical default location exists in inventory_locations (but don't
    // auto-seed stock — callers explicitly manage stock via loc_a/loc_b params).
    conn.execute(
        "INSERT OR IGNORE INTO inventory_locations (id, name, type) \
         VALUES (?1, 'Default', 'store')",
        rusqlite::params![crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
    )
    .unwrap();
    product_id
}

#[test]
fn complete_sale_deduction_topology_allocates_across_routes_atomically() {
    let conn = fresh();
    let s = store(&conn);
    setup_locations_with_stock(&conn, "TOPO-COFFEE", "loc-route-a", 3, "loc-route-b", 10);
    let sale = make_single_line_sale("TOPO-COFFEE", 8, 1000);
    let locations = vec![
        crate::inventory::LocationId::from("loc-route-a"),
        crate::inventory::LocationId::from("loc-route-b"),
    ];
    let result = s
        .complete_sale_deduction_with_locations(
            &sale,
            None,
            &locations,
            &tender(8000),
            "cashier-1",
            None,
        )
        .unwrap();
    assert_eq!(result.status, SaleStatus::Pending);

    let route_a: i64 = conn
        .query_row(
            "SELECT qty FROM stock_summary WHERE item_id = (SELECT id FROM products WHERE sku = 'TOPO-COFFEE') AND location_id = 'loc-route-a'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let route_b: i64 = conn
        .query_row(
            "SELECT qty FROM stock_summary WHERE item_id = (SELECT id FROM products WHERE sku = 'TOPO-COFFEE') AND location_id = 'loc-route-b'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(route_a, 0);
    assert_eq!(route_b, 5);

    let deduction_locations: String = conn
        .query_row(
            "SELECT deduction_locations FROM sales WHERE id = ?1",
            [&sale.id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(deduction_locations.contains("loc-route-a"));
    assert!(deduction_locations.contains("loc-route-b"));
}

#[test]
fn complete_sale_deduction_sufficient_stock_succeeds() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);
    seed_product_with_stock(&conn, "BAGEL", 5);

    let sale = make_single_line_sale("COFFEE", 2, 350);
    let result = s
        .complete_sale_deduction(&sale, None, &tender(700), "cashier-1", None)
        .unwrap();

    assert_eq!(result.sale_id, sale.id);
    assert_eq!(result.status, SaleStatus::Pending);
    assert!(!result.deduct_tx_id.as_str().is_empty());

    // Verify the sale row exists and is completed.
    let loaded = s.get_sale(&sale.id).unwrap().unwrap();
    assert_eq!(loaded.status, SaleStatus::Pending);

    // Verify stock was deducted.
    let remaining: i64 = conn
        .query_row(
            "SELECT qty FROM stock_summary \
             WHERE item_id = (SELECT id FROM products WHERE sku = 'COFFEE') \
             AND location_id = ?1",
            rusqlite::params![crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 8, "10 - 2 = 8");
}

#[test]
fn complete_sale_deduction_shortfall_returns_error_with_partial_result() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 1); // only 1 available, need 2

    let sale = make_single_line_sale("COFFEE", 2, 350);
    let err = s
        .complete_sale_deduction(&sale, None, &[], "cashier-1", None)
        .unwrap_err();

    // Should be a Validation error with serialized PartialStockResult.
    match &err {
        CoreError::Validation { field, message } if *field == "stock" => {
            let psr: crate::sale_deduction::PartialStockResult =
                serde_json::from_str(message).unwrap();
            assert!(psr.requires_resolution);
            assert_eq!(psr.shortfalls.len(), 1);
            assert_eq!(psr.shortfalls[0].sku, "COFFEE");
            assert_eq!(psr.shortfalls[0].requested_qty, 2);
            assert_eq!(psr.shortfalls[0].primary_qty_available, 1);
            assert_eq!(psr.shortfalls[0].deficit, 1);
        }
        other => panic!("expected Validation error with field=stock, got {other:?}"),
    }

    // Stock should NOT have been deducted (transaction rolled back).
    let remaining: i64 = conn
        .query_row(
            "SELECT qty FROM stock_summary \
             WHERE item_id = (SELECT id FROM products WHERE sku = 'COFFEE') \
             AND location_id = ?1",
            rusqlite::params![crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 1, "stock unchanged after shortfall rollback");
}

#[test]
fn complete_sale_deduction_empty_lines_succeeds() {
    let conn = fresh();
    let s = store(&conn);

    let sale = make_single_line_sale("COFFEE", 0, 0);
    let mut empty_sale = sale;
    empty_sale.lines.clear();
    empty_sale.line_count = 0;
    empty_sale.total = price(0);

    let result = s
        .complete_sale_deduction(&empty_sale, None, &[], "cashier-1", None)
        .unwrap();
    assert_eq!(result.status, SaleStatus::Pending);
}

#[test]
fn complete_sale_deduction_unknown_sku_shortfall() {
    let conn = fresh();
    let s = store(&conn);
    // Do NOT seed any product — the SKU is unknown.

    let sale = make_single_line_sale("GHOST", 2, 350);
    let err = s
        .complete_sale_deduction(&sale, None, &[], "cashier-1", None)
        .unwrap_err();

    match &err {
        CoreError::Validation { field, message } if *field == "stock" => {
            let psr: crate::sale_deduction::PartialStockResult =
                serde_json::from_str(message).unwrap();
            assert_eq!(psr.shortfalls.len(), 1);
            assert_eq!(psr.shortfalls[0].sku, "GHOST");
            assert_eq!(psr.shortfalls[0].primary_qty_available, 0);
        }
        other => panic!("expected Validation error, got {other:?}"),
    }
}

#[test]
fn complete_sale_deduction_multi_line_partial_shortfall() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);
    seed_product_with_stock(&conn, "BAGEL", 0); // no stock for BAGEL

    // Build a 2-line sale manually.
    let cart = make_cart();
    let mut sale = Sale::from_cart(&cart).unwrap();
    // Override qty for BAGEL to exceed available stock.
    sale.lines[1].qty = 1;

    let err = s
        .complete_sale_deduction(&sale, None, &[], "cashier-1", None)
        .unwrap_err();

    match &err {
        CoreError::Validation { field, message } if *field == "stock" => {
            let psr: crate::sale_deduction::PartialStockResult =
                serde_json::from_str(message).unwrap();
            assert!(psr.requires_resolution);
            // Only BAGEL should be listed as a shortfall (COFFEE sufficed).
            assert_eq!(psr.shortfalls.len(), 1);
            assert_eq!(psr.shortfalls[0].sku, "BAGEL");
        }
        other => panic!("expected Validation error, got {other:?}"),
    }

    // COFFEE stock should NOT have been deducted (full rollback).
    let coffee_qty: i64 = conn
        .query_row(
            "SELECT qty FROM stock_summary \
             WHERE item_id = (SELECT id FROM products WHERE sku = 'COFFEE') \
             AND location_id = ?1",
            rusqlite::params![crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(coffee_qty, 10, "COFFEE stock unchanged (full rollback)");
}

#[test]
fn complete_sale_deduction_with_payment_splits() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);

    let sale = make_single_line_sale("COFFEE", 2, 350);
    let splits = tender(700);
    let result = s
        .complete_sale_deduction(&sale, None, &splits, "cashier-1", None)
        .unwrap();
    assert_eq!(result.status, SaleStatus::Pending);

    // Verify payment was recorded.
    let payment_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM payments WHERE sale_id = ?1",
            rusqlite::params![sale.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(payment_count, 1, "one payment row created");
}

/// One full-tender cash split for the given amount (MONEY-04 tests).
fn tender(amount_minor: i64) -> Vec<crate::PaymentSplitArg> {
    vec![crate::PaymentSplitArg {
        method: "cash".into(),
        amount_minor,
        gateway_reference: None,
        gateway_status: None,
        gateway_response: None,
        idempotency_key: None,
    }]
}

/// MONEY-04: payment splits must cover the ledger total. Over-tender is
/// allowed (change back); under-payment must be rejected even though the
/// old code happily persisted the sale — a hostile IPC caller could
/// complete a 700-minor sale with a 500-minor "payment".
#[test]
fn complete_sale_deduction_rejects_underpaid_payment_splits() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);

    let sale = make_single_line_sale("COFFEE", 2, 350); // total 700
    let result = s.complete_sale_deduction(&sale, None, &tender(500), "cashier-1", None);

    match result {
        Err(CoreError::Validation { field, message }) => {
            assert_eq!(
                field, "payments",
                "expected field 'payments', got '{field}'"
            );
            assert!(
                message.contains("do not cover"),
                "expected an under-payment message, got: {message}"
            );
        }
        other => panic!("under-paid splits must not complete the sale, got: {other:?}"),
    }

    // Nothing may be persisted: no sale row, no payment rows.
    assert!(
        s.get_sale(&sale.id).unwrap().is_none(),
        "no sale row may exist"
    );
    let payment_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM payments WHERE sale_id = ?1",
            rusqlite::params![sale.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(payment_count, 0, "no payment rows may exist");
}

/// The worst case: `payment_splits: Some([])` bypasses the command layer's
/// full-tender default and would previously complete a sale with zero
/// payment records.
#[test]
fn complete_sale_deduction_rejects_empty_payment_splits() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);

    let sale = make_single_line_sale("COFFEE", 2, 350); // total 700
    let result = s.complete_sale_deduction(&sale, None, &[], "cashier-1", None);

    match result {
        Err(CoreError::Validation { field, .. }) => {
            assert_eq!(
                field, "payments",
                "expected field 'payments', got '{field}'"
            );
        }
        other => panic!("empty splits must not complete the sale, got: {other:?}"),
    }
}

/// Over-tender must remain legal — the difference is change back to the
/// customer.
#[test]
fn complete_sale_deduction_accepts_overpaid_payment_splits() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);

    let sale = make_single_line_sale("COFFEE", 2, 350); // total 700
    let result = s
        .complete_sale_deduction(&sale, None, &tender(1000), "cashier-1", None)
        .unwrap();
    assert_eq!(result.status, SaleStatus::Pending);

    let paid: i64 = conn
        .query_row(
            "SELECT amount_minor FROM payments WHERE sale_id = ?1",
            rusqlite::params![sale.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(paid, 1000, "the over-tendered split is recorded for change");
}

/// A negative split could game the sum check (e.g. [900, -200] sums to
/// 700) while still writing garbage payment rows into reports.
#[test]
fn complete_sale_deduction_rejects_negative_payment_split() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);

    let sale = make_single_line_sale("COFFEE", 2, 350); // total 700
    let splits = vec![
        crate::PaymentSplitArg {
            method: "cash".into(),
            amount_minor: 900,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
            idempotency_key: None,
        },
        crate::PaymentSplitArg {
            method: "other".into(),
            amount_minor: -200,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
            idempotency_key: None,
        },
    ];
    let result = s.complete_sale_deduction(&sale, None, &splits, "cashier-1", None);

    match result {
        Err(CoreError::Validation { field, message }) => {
            assert_eq!(
                field, "payments",
                "expected field 'payments', got '{field}'"
            );
            assert!(
                message.contains("non-negative"),
                "expected a non-negative message, got: {message}"
            );
        }
        other => panic!("negative splits must not complete the sale, got: {other:?}"),
    }
}

/// The resolved-shortfalls command shares the same ledger-write path and
/// must enforce the identical contract.
#[test]
fn complete_sale_with_resolved_shortfalls_rejects_underpaid_payment_splits() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);

    let sale = make_single_line_sale("COFFEE", 2, 350); // total 700
    let result =
        s.complete_sale_with_resolved_shortfalls(&sale, None, &tender(500), "cashier-1", None, &[]);

    match result {
        Err(CoreError::Validation { field, .. }) => {
            assert_eq!(
                field, "payments",
                "expected field 'payments', got '{field}'"
            );
        }
        other => {
            panic!("resolved-shortfalls under-paid splits must not complete, got: {other:?}")
        }
    }
}

/// The split sum uses checked arithmetic: a hostile list whose total
/// overflows i64 must fail with a structured error, not wrap past the
/// sale total.
#[test]
fn complete_sale_deduction_rejects_overflowing_payment_split_sum() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);

    let sale = make_single_line_sale("COFFEE", 2, 350); // total 700
    let splits = vec![
        crate::PaymentSplitArg {
            method: "cash".into(),
            amount_minor: i64::MAX,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
            idempotency_key: None,
        },
        crate::PaymentSplitArg {
            method: "card".into(),
            amount_minor: 1,
            gateway_reference: None,
            gateway_status: None,
            gateway_response: None,
            idempotency_key: None,
        },
    ];
    let result = s.complete_sale_deduction(&sale, None, &splits, "cashier-1", None);

    match result {
        Err(CoreError::Validation { field, message }) => {
            assert_eq!(
                field, "payments",
                "expected field 'payments', got '{field}'"
            );
            assert!(
                message.contains("overflow"),
                "expected an overflow message, got: {message}"
            );
        }
        other => panic!("overflowing split sum must not complete the sale, got: {other:?}"),
    }
}

/// MONEY-03 follow-up: a negative line qty on a hand-built `Sale` would
/// record a negative ledger total AND credit stock (the deduction delta is
/// `-qty`, positive when qty is negative). `CartLine::new` asserts qty > 0
/// so this is unreachable from the front-end, but this Store API is the
/// ledger boundary — reject it up front.
#[test]
fn complete_sale_deduction_rejects_negative_line_qty() {
    use crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);

    let sale = make_single_line_sale("COFFEE", -2, 350);
    let result = s.complete_sale_deduction(&sale, None, &tender(700), "cashier-1", None);

    match result {
        Err(CoreError::Validation { field, message }) => {
            assert_eq!(field, "qty", "expected field 'qty', got '{field}'");
            assert!(
                message.contains("positive"),
                "expected a positive-qty message, got: {message}"
            );
        }
        other => panic!("negative qty must not complete the sale, got: {other:?}"),
    }

    // Stock must be untouched — a negative qty must never credit it.
    let remaining: i64 = conn
        .query_row(
            "SELECT qty FROM stock_summary \
             WHERE item_id = (SELECT id FROM products WHERE sku = 'COFFEE') \
             AND location_id = ?1",
            rusqlite::params![CANONICAL_DEFAULT_LOCATION_UUID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        remaining, 10,
        "stock must not be credited by a negative qty"
    );
}

/// The resolved-shortfalls command shares the same ledger-write path.
#[test]
fn complete_sale_with_resolved_shortfalls_rejects_negative_line_qty() {
    let conn = fresh();
    let s = store(&conn);
    seed_product_with_stock(&conn, "COFFEE", 10);

    let sale = make_single_line_sale("COFFEE", -2, 350);
    let result =
        s.complete_sale_with_resolved_shortfalls(&sale, None, &tender(700), "cashier-1", None, &[]);

    match result {
        Err(CoreError::Validation { field, .. }) => {
            assert_eq!(field, "qty", "expected field 'qty', got '{field}'");
        }
        other => panic!("negative qty must not complete the sale, got: {other:?}"),
    }
}

// ── complete_sale_with_resolved_shortfalls (ADR-19 §6b) ─────

/// Resolution with sufficient stock at an alternative location success.
#[test]
fn complete_sale_with_resolved_shortfalls_splits_across_locations() {
    let conn = fresh();
    let s = store(&conn);
    let loc_a = "loc-a";
    let loc_b = "loc-b";
    setup_locations_with_stock(&conn, "COFFEE", loc_a, 5, loc_b, 10);

    let sale = make_single_line_sale("COFFEE", 12, 350);
    let resolution = crate::sale_deduction::ResolvedShortfall {
        sku: "COFFEE".into(),
        allocations: vec![
            crate::sale_deduction::LocationAllocation {
                location_id: crate::inventory::LocationId::from(loc_a),
                qty: 5,
            },
            crate::sale_deduction::LocationAllocation {
                location_id: crate::inventory::LocationId::from(loc_b),
                qty: 7,
            },
        ],
    };
    let result = s
        .complete_sale_with_resolved_shortfalls(
            &sale,
            None,
            &tender(4200),
            "cashier-1",
            None,
            &[resolution],
        )
        .unwrap();
    assert_eq!(result.status, SaleStatus::Pending);

    // Verify stock deducted correctly from both locations
    let stock_a: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
             (SELECT id FROM products WHERE sku = 'COFFEE') AND location_id = ?1",
            rusqlite::params![loc_a],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stock_a, 0, "loc-a had 5, deducted 5 → 0");

    let stock_b: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
             (SELECT id FROM products WHERE sku = 'COFFEE') AND location_id = ?1",
            rusqlite::params![loc_b],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stock_b, 3, "loc-b had 10, deducted 7 → 3");

    // Verify sale persisted with deduction_locations JSON
    let dl: String = conn
        .query_row(
            "SELECT deduction_locations FROM sales WHERE id = ?1",
            rusqlite::params![sale.id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        dl.contains(loc_a),
        "deduction_locations should reference loc-a"
    );
    assert!(
        dl.contains(loc_b),
        "deduction_locations should reference loc-b"
    );
}

/// Resolution sum validation rejects mismatch.
#[test]
fn complete_sale_with_resolved_shortfalls_rejects_bad_allocation_sum() {
    let conn = fresh();
    let s = store(&conn);
    let loc = "loc-a";
    setup_locations_with_stock(&conn, "TEA", loc, 10, "loc-other", 0);

    let sale = make_single_line_sale("TEA", 5, 200);
    // Allocation sum = 3, but requested = 5 → error
    let resolution = crate::sale_deduction::ResolvedShortfall {
        sku: "TEA".into(),
        allocations: vec![crate::sale_deduction::LocationAllocation {
            location_id: crate::inventory::LocationId::from(loc),
            qty: 3,
        }],
    };
    let err = s
        .complete_sale_with_resolved_shortfalls(&sale, None, &[], "cashier-1", None, &[resolution])
        .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == &"resolutions"),
        "expected Validation error for bad allocation sum, got: {err}"
    );
}

/// Insufficient stock at resolved location returns error.
#[test]
fn complete_sale_with_resolved_shortfalls_fails_on_second_check() {
    let conn = fresh();
    let s = store(&conn);
    let loc = "loc-a";
    setup_locations_with_stock(&conn, "CHA", loc, 2, "loc-other", 0);

    let sale = make_single_line_sale("CHA", 5, 150);
    // Try to allocate 5 from loc-a which only has 2 → error
    let resolution = crate::sale_deduction::ResolvedShortfall {
        sku: "CHA".into(),
        allocations: vec![crate::sale_deduction::LocationAllocation {
            location_id: crate::inventory::LocationId::from(loc),
            qty: 5,
        }],
    };
    let err = s
        .complete_sale_with_resolved_shortfalls(&sale, None, &[], "cashier-1", None, &[resolution])
        .unwrap_err();
    assert!(
        matches!(&err, CoreError::InsufficientStockAtLocation { .. }),
        "expected InsufficientStockAtLocation for over-allocation, got: {err}"
    );
}

/// MONEY-03: the BOM ingredient total (`line.qty × quantity_required`)
/// comes from untrusted IPC qty and must use checked arithmetic like
/// `compute_line_tax` (TAX-04). Dev/test builds disable overflow
/// checks, so a bare `*` silently wraps and the deduction path either
/// completes the sale with a corrupt stock delta or fails downstream.
#[test]
fn complete_sale_deduction_bom_quantity_overflow_returns_validation_error() {
    use crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
    let conn = fresh();
    let s = store(&conn);

    // Composite 'service' product with a BOM recipe: the composite does
    // not track inventory, so only the ingredient path runs.
    let (_burger_id, bun_id) = seed_bom_composite(&conn, "BURGER", "BUN", 10, 3);

    // (i64::MAX / 2) * 3 overflows i64.
    let sale = make_single_line_sale("BURGER", i64::MAX / 2, 1);
    let result = s.complete_sale_deduction(&sale, None, &[], "cashier-1", None);

    match result {
        Err(CoreError::Validation { field, message }) => {
            assert_eq!(field, "qty", "expected field 'qty', got '{field}'");
            assert!(
                message.contains("overflow"),
                "expected an overflow message, got: {message}"
            );
        }
        other => {
            panic!("BOM qty × quantity_required overflow must not wrap silently, got: {other:?}")
        }
    }

    // The deduction must never be applied.
    let bun_stock: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary \
             WHERE item_id = ?1 AND location_id = ?2",
            rusqlite::params![bun_id, CANONICAL_DEFAULT_LOCATION_UUID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        bun_stock, 10,
        "stock must be untouched when the BOM total overflows"
    );
}

/// MONEY-03: the resolved-shortfalls command shares the same unchecked
/// BOM multiply for non-resolution lines; pin the same overflow contract.
#[test]
fn complete_sale_with_resolved_shortfalls_bom_quantity_overflow_returns_validation_error() {
    use crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
    let conn = fresh();
    let s = store(&conn);

    let (_burger_id, bun_id) = seed_bom_composite(&conn, "BURGER", "BUN", 10, 3);

    // No resolutions: the non-resolution BOM path runs.
    let sale = make_single_line_sale("BURGER", i64::MAX / 2, 1);
    let result = s.complete_sale_with_resolved_shortfalls(&sale, None, &[], "cashier-1", None, &[]);

    match result {
        Err(CoreError::Validation { field, message }) => {
            assert_eq!(field, "qty", "expected field 'qty', got '{field}'");
            assert!(
                message.contains("overflow"),
                "expected an overflow message, got: {message}"
            );
        }
        other => {
            panic!("BOM qty × quantity_required overflow must not wrap silently, got: {other:?}")
        }
    }

    let bun_stock: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary \
             WHERE item_id = ?1 AND location_id = ?2",
            rusqlite::params![bun_id, CANONICAL_DEFAULT_LOCATION_UUID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        bun_stock, 10,
        "stock must be untouched when the BOM total overflows"
    );
}

/// Non-resolution lines still get deducted from primary location.
#[test]
fn complete_sale_with_resolved_shortfalls_deducts_unresolved_lines_at_primary() {
    use crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
    let conn = fresh();
    let s = store(&conn);
    setup_locations_with_stock(
        &conn,
        "COFFEE",
        crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
        20,
        "loc-wh",
        50,
    );
    setup_locations_with_stock(
        &conn,
        "BAGEL",
        crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
        10,
        "loc-wh",
        30,
    );

    // Only COFFEE has a resolution; BAGEL should be deducted from primary (default UUID)
    let sale = {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("COFFEE"), 3, price(350)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("BAGEL"), 2, price(450)))
            .unwrap();
        Sale::from_cart(&cart).unwrap()
    };

    let resolution = crate::sale_deduction::ResolvedShortfall {
        sku: "COFFEE".into(),
        allocations: vec![crate::sale_deduction::LocationAllocation {
            location_id: crate::inventory::LocationId::from("loc-wh"),
            qty: 3,
        }],
    };
    let result = s
        .complete_sale_with_resolved_shortfalls(
            &sale,
            None,
            &tender(1950),
            "cashier-1",
            None,
            &[resolution],
        )
        .unwrap();
    assert_eq!(result.status, SaleStatus::Pending);

    // COFFEE deducted 3 from loc-wh (50 → 47)
    let coffee_wh: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
             (SELECT id FROM products WHERE sku = 'COFFEE') AND location_id = ?1",
            rusqlite::params!["loc-wh"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(coffee_wh, 47, "loc-wh had 50, deducted 3 → 47");

    // BAGEL deductible from canonical default (10 seeded, 2 deducted → 8)
    let bagel_def: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
             (SELECT id FROM products WHERE sku = 'BAGEL') AND location_id = ?1",
            rusqlite::params![CANONICAL_DEFAULT_LOCATION_UUID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(bagel_def, 8, "canonical default had 10, deducted 2 → 8");
}

/// Empty resolutions (no shortfalls) still deducts all stock from primary.
#[test]
fn complete_sale_with_resolved_shortfalls_empty_resolutions_deducts_at_primary() {
    let conn = fresh();
    let s = store(&conn);
    setup_locations_with_stock(
        &conn,
        "COFFEE",
        crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
        10,
        "loc-wh",
        20,
    );

    let sale = make_single_line_sale("COFFEE", 3, 350);
    let result = s
        .complete_sale_with_resolved_shortfalls(&sale, None, &tender(1050), "cashier-1", None, &[])
        .unwrap();
    assert_eq!(result.status, SaleStatus::Pending);

    // COFFEE deducted 3 from canonical default
    let stock: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
             (SELECT id FROM products WHERE sku = 'COFFEE') AND location_id = ?1",
            rusqlite::params![crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stock, 7, "canonical default had 10, deducted 3 → 7");
    // loc-wh should be untouched
    let stock_wh: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
             (SELECT id FROM products WHERE sku = 'COFFEE') AND location_id = ?1",
            rusqlite::params!["loc-wh"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stock_wh, 20, "loc-wh untouched");
}

// ── ADR-19 §16.2 acceptance tests ──────────────────────────────

/// Multi-binding sale with insufficient primary stock → shortfall
/// returned AND no sale row persists (full rollback).
#[test]
fn complete_sale_partial_shortfall_rolls_back_sale_row() {
    let conn = fresh();
    let s = store(&conn);

    // ── set up a multi-binding workspace ────────────────────────
    conn.execute_batch(
        "INSERT OR IGNORE INTO inventory_locations (id, name, type) VALUES
            ('loc-pri', 'Primary', 'store'),
            ('loc-sec', 'Secondary', 'warehouse');
         INSERT OR IGNORE INTO store_profiles (id, name, is_primary) VALUES ('store-1', 'Test Store', 1);
         INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name)
            VALUES ('ws-multi-test',
                (SELECT key FROM workspace_types LIMIT 1),
                'store-1', 'Multi-Test');
         INSERT OR IGNORE INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, sort_order)
            VALUES ('wsl-pri', 'ws-multi-test', 'loc-pri', 1, 0),
                   ('wsl-sec', 'ws-multi-test', 'loc-sec', 0, 1);",
    )
    .unwrap();
    let product_id = seed_product_with_stock(&conn, "COFFEE", 0);
    // Stock only at the secondary location (primary has 0).
    conn.execute(
        "INSERT OR REPLACE INTO stock_summary (item_id, location_id, qty) VALUES (?1, 'loc-sec', 5)",
        rusqlite::params![product_id],
    )
    .unwrap();

    let sale = make_single_line_sale("COFFEE", 2, 350);
    let err = s
        .complete_sale_deduction(&sale, Some("ws-multi-test"), &[], "cashier-1", None)
        .unwrap_err();

    match &err {
        CoreError::Validation { field, message } if *field == "stock" => {
            let psr: crate::sale_deduction::PartialStockResult =
                serde_json::from_str(message).unwrap();
            assert!(psr.requires_resolution);
            assert_eq!(psr.shortfalls.len(), 1);
            assert_eq!(psr.shortfalls[0].sku, "COFFEE");
            assert_eq!(
                psr.shortfalls[0].primary_location_id,
                crate::inventory::LocationId::from("loc-pri")
            );
            assert_eq!(psr.shortfalls[0].primary_qty_available, 0);
            assert_eq!(psr.shortfalls[0].deficit, 2);
            // Should have loc-sec as an alternative
            assert!(
                psr.shortfalls[0]
                    .alternatives
                    .iter()
                    .any(|a| a.location_id == crate::inventory::LocationId::from("loc-sec")),
                "expected loc-sec as alternative"
            );
        }
        other => panic!("expected Validation error with field=stock, got {other:?}"),
    }

    // Verify NO sale row was created (full rollback).
    let sale_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sales WHERE id = ?1",
            rusqlite::params![sale.id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    assert!(
        !sale_exists,
        "sale row must not exist after shortfall rollback"
    );
}

/// Void of a multi-location pending sale credits stock back to
/// each original deduction source (ADR-19 §5.3 / §16.2).
#[test]
fn void_sale_credits_back_to_original_deduction_source() {
    let conn = fresh();
    let s = store(&conn);
    let loc_a = "loc-v-a";
    let loc_b = "loc-v-b";

    // ── create a sale with split-location deduction_locations ───
    setup_locations_with_stock(&conn, "TEA", loc_a, 10, loc_b, 5);
    let sale = make_single_line_sale("TEA", 8, 200);
    let resolution = crate::sale_deduction::ResolvedShortfall {
        sku: "TEA".into(),
        allocations: vec![
            crate::sale_deduction::LocationAllocation {
                location_id: crate::inventory::LocationId::from(loc_a),
                qty: 5,
            },
            crate::sale_deduction::LocationAllocation {
                location_id: crate::inventory::LocationId::from(loc_b),
                qty: 3,
            },
        ],
    };
    s.complete_sale_with_resolved_shortfalls(
        &sale,
        None,
        &tender(1600),
        "cashier-1",
        None,
        &[resolution],
    )
    .unwrap();

    // Confirm stock was deducted.
    let stock_a_before: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
             (SELECT id FROM products WHERE sku = 'TEA') AND location_id = ?1",
            rusqlite::params![loc_a],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stock_a_before, 5, "loc-a had 10, deducted 5 → 5");

    let stock_b_before: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
             (SELECT id FROM products WHERE sku = 'TEA') AND location_id = ?1",
            rusqlite::params![loc_b],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stock_b_before, 2, "loc-b had 5, deducted 3 → 2");

    // ── void the pending sale ───────────────────────────────────
    s.void_pending_sale(&sale.id).unwrap();

    // Verify stock was credited BACK to each location.
    let stock_a_after: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
             (SELECT id FROM products WHERE sku = 'TEA') AND location_id = ?1",
            rusqlite::params![loc_a],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stock_a_after, 10, "loc-a credited back to original 10");

    let stock_b_after: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary WHERE item_id = \
             (SELECT id FROM products WHERE sku = 'TEA') AND location_id = ?1",
            rusqlite::params![loc_b],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stock_b_after, 5, "loc-b credited back to original 5");

    // Verify sale status is voided.
    let loaded = s.get_sale(&sale.id).unwrap().unwrap();
    assert_eq!(loaded.status, SaleStatus::Voided);
}

/// Two threads attempting complete_sale_deduction on the same SKU:
/// one succeeds, the other fails with a constraint/serialization error
/// thanks to BEGIN IMMEDIATE (ADR-19 §5.2).
#[test]
fn concurrent_complete_sale_serialized_by_begin_immediate() {
    // Use a file-based DB so two connections can access it concurrently.
    let dir = std::env::temp_dir().join(format!("oz_concurrent_{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("test.db");

    // Clone the schema from a fresh_db() snapshot into the file DB.
    {
        let mut file_conn = rusqlite::Connection::open(&db_path).unwrap();
        {
            let template = crate::migrations::fresh_db();
            let backup = rusqlite::backup::Backup::new(&template, &mut file_conn).unwrap();
            backup
                .run_to_completion(10, std::time::Duration::from_millis(0), None)
                .unwrap();
        }
        let pid = uuid::Uuid::now_v7().to_string();
        file_conn
            .execute(
                "INSERT INTO products (id, sku, name, price_minor, currency, product_type) \
                 VALUES (?1, 'COFFEE', 'Coffee', 1000, 'USD', 'retail')",
                rusqlite::params![pid],
            )
            .unwrap();
        file_conn
            .execute(
                "INSERT INTO stock_summary (item_id, location_id, qty) VALUES (?1, ?2, 2)",
                rusqlite::params![pid, crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
            )
            .unwrap();
    }

    let sale = std::sync::Arc::new(make_single_line_sale("COFFEE", 2, 350));

    let mut handles = Vec::new();
    for i in 0..2 {
        let p = db_path.clone();
        let sl = sale.clone();
        handles.push(std::thread::spawn(move || {
            let conn = rusqlite::Connection::open(&p).unwrap();
            let store = Store::new(&conn);
            let result = store.complete_sale_deduction(&sl, None, &tender(700), "cashier-1", None);
            (i, result)
        }));
    }

    let mut success_count = 0;
    let mut failure_count = 0;
    for h in handles {
        match h.join().unwrap() {
            (_, Ok(_)) => success_count += 1,
            (i, Err(e)) => {
                failure_count += 1;
                tracing::info!(thread = i, error = %e, "concurrent sale failed as expected");
            }
        }
    }

    assert_eq!(
        success_count, 1,
        "exactly one thread should succeed with BEGIN IMMEDIATE"
    );
    assert!(
        failure_count >= 1,
        "second thread should fail with serialization error"
    );

    // Clean up.
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn void_pending_sale_nonexistent_sale_errors() {
    let conn = fresh();
    let s = store(&conn);
    let err = s.void_pending_sale("nonexistent").unwrap_err();
    assert!(matches!(
        err,
        CoreError::NotFound {
            entity: "pending sale",
            ..
        }
    ));
}

#[test]
fn void_pending_sale_malformed_deduction_locations_errors() {
    let conn = fresh();
    let s = store(&conn);
    let cart = make_cart();
    let sale = Sale::from_cart(&cart).unwrap();

    // Insert a sale with malformed JSON in deduction_locations
    conn.execute(
        "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method,
                            tendered_minor, discount_percent, discount_label, user_id,
                            created_at, updated_at, subtotal_minor, tax_total_minor,
                            deduction_locations, version)
         VALUES (?1, 1000, 'USD', 1, 'pending', 'CASH', 1000, 0, NULL, 'user-1',
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1000, 0, 'not-valid-json', 1)",
        rusqlite::params![sale.id],
    )
    .unwrap();

    let err = s.void_pending_sale(&sale.id).unwrap_err();
    assert!(matches!(
        err,
        CoreError::Validation {
            field: "deduction_locations",
            ..
        }
    ));
}

#[test]
fn void_pending_sale_twice_errors() {
    let conn = fresh();
    let s = store(&conn);

    // Seed a product and stock
    conn.execute(
        "INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
         VALUES ('prod-test', 'TEST-1', 'Test', 5000, 'IDR', 'retail')",
        [],
    )
    .unwrap();
    let default_loc = crate::location_resolver::get_default_location_id();
    conn.execute(
        "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) \
         VALUES ('prod-test', ?1, 10)",
        rusqlite::params![default_loc.as_str()],
    )
    .unwrap();

    // Use Sale::from_cart to create a sale — the only public constructor.
    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("TEST-1"), 3, price(5000)))
        .unwrap();
    let sale = Sale::from_cart(&cart).unwrap();

    s.complete_sale_deduction(&sale, None, &tender(15000), "staff-1", None)
        .unwrap();

    // First void succeeds
    s.void_pending_sale(&sale.id).unwrap();

    // Second void fails — sale is now 'voided', not 'pending'
    let err = s.void_pending_sale(&sale.id).unwrap_err();
    assert!(matches!(
        err,
        CoreError::NotFound {
            entity: "pending sale",
            ..
        }
    ));
}

// ── ADR-20 stale-pending-sale reaper ─────────────────────────

#[test]
fn reap_stale_pending_sales_voids_expired_sales() {
    // ADR-20 criterion 20-5: stale pending sale after 30 min is auto-voided.
    let conn = fresh();
    let s = store(&conn);

    // Seed a product and stock.
    conn.execute(
        "INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
         VALUES ('prod-reap', 'REAP-1', 'Reap Test', 1000, 'IDR', 'retail')",
        [],
    )
    .unwrap();
    let default_loc = crate::location_resolver::get_default_location_id();
    conn.execute(
        "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) \
         VALUES ('prod-reap', ?1, 10)",
        rusqlite::params![default_loc.as_str()],
    )
    .unwrap();

    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("REAP-1"), 2, price(1000)))
        .unwrap();
    let sale = Sale::from_cart(&cart).unwrap();

    s.complete_sale_deduction(&sale, None, &tender(2000), "staff-1", None)
        .unwrap();

    // Manually set pending_expires_at to 1 hour in the past.
    let past = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::hours(1))
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap();
    conn.execute(
        "UPDATE sales SET pending_expires_at = ?1 WHERE id = ?2",
        rusqlite::params![past, sale.id],
    )
    .unwrap();

    // Reap should find and void the stale sale.
    let count = s.reap_stale_pending_sales().unwrap();
    assert_eq!(count, 1, "expected 1 stale sale to be voided");

    // Verify sale is now voided.
    let status: String = conn
        .query_row(
            "SELECT status FROM sales WHERE id = ?1",
            rusqlite::params![sale.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "voided");

    // Verify stock was credited back (original qty restored).
    let qty: i64 = conn
        .query_row(
            "SELECT COALESCE(qty, 0) FROM stock_summary \
             WHERE item_id = 'prod-reap' AND location_id = ?1",
            rusqlite::params![default_loc.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(qty, 10, "stock should be restored to 10 after void");
}

#[test]
fn reap_stale_pending_sales_skips_fresh_sales() {
    // Verify that non-expired pending sales are NOT voided.
    let conn = fresh();
    let s = store(&conn);

    conn.execute(
        "INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
         VALUES ('prod-fresh', 'FRESH-1', 'Fresh Test', 1000, 'IDR', 'retail')",
        [],
    )
    .unwrap();
    let default_loc = crate::location_resolver::get_default_location_id();
    conn.execute(
        "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) \
         VALUES ('prod-fresh', ?1, 10)",
        rusqlite::params![default_loc.as_str()],
    )
    .unwrap();

    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("FRESH-1"), 1, price(1000)))
        .unwrap();
    let sale = Sale::from_cart(&cart).unwrap();

    // Use complete_sale_deduction which sets pending_expires_at = NOW + 30 min.
    s.complete_sale_deduction(&sale, None, &tender(1000), "staff-1", None)
        .unwrap();

    // Reap should NOT touch this fresh sale.
    let count = s.reap_stale_pending_sales().unwrap();
    assert_eq!(count, 0, "fresh pending sale should not be reaped");

    let status: String = conn
        .query_row(
            "SELECT status FROM sales WHERE id = ?1",
            rusqlite::params![sale.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "pending");
}

#[test]
fn finalize_and_void_concurrent_exclusive() {
    // ADR-20 criterion 20-6: concurrent finalize and void on same sale
    // — only the first should succeed; second must see status already changed.
    let conn = fresh();
    let s = store(&conn);

    conn.execute(
        "INSERT OR IGNORE INTO products (id, sku, name, price_minor, currency, product_type) \
         VALUES ('prod-c20-6', 'C206-1', 'C206', 1000, 'IDR', 'retail')",
        [],
    )
    .unwrap();
    let default_loc = crate::location_resolver::get_default_location_id();
    conn.execute(
        "INSERT OR IGNORE INTO stock_summary (item_id, location_id, qty) \
         VALUES ('prod-c20-6', ?1, 10)",
        rusqlite::params![default_loc.as_str()],
    )
    .unwrap();

    let mut cart = Cart::new(usd());
    cart.add_line(CartLine::new(Sku::new("C206-1"), 1, price(1000)))
        .unwrap();
    let sale = Sale::from_cart(&cart).unwrap();

    s.complete_sale_deduction(&sale, None, &tender(1000), "staff-1", None)
        .unwrap();

    // First operation succeeds (finalize).
    s.finalize_sale(&sale.id).unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM sales WHERE id = ?1",
            rusqlite::params![sale.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "completed");

    // Second operation (void) should fail because status is now 'completed'.
    let err = s.void_pending_sale(&sale.id).unwrap_err();
    assert!(
        matches!(err, CoreError::NotFound { .. }),
        "expected NotFound for already-finalized sale, got: {err:?}"
    );
}

// ── Customer history (CUST-05) ─────────────────────────────────

fn seed_customer_row(conn: &rusqlite::Connection, id: &str) {
    // INSERT OR IGNORE keeps repeated seeding of the same customer id
    // idempotent (several sales can share one customer).
    conn.execute(
        "INSERT OR IGNORE INTO customers (id, name, created_at, updated_at)
         VALUES (?1, ?2, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
        rusqlite::params![id, format!("Customer {id}")],
    )
    .unwrap();
}

fn seed_sale_for_customer(
    conn: &rusqlite::Connection,
    id: &str,
    customer_id: &str,
    total_minor: i64,
) {
    seed_customer_row(conn, customer_id);
    conn.execute(
        "INSERT INTO sales (id, total_minor, currency, line_count, status, customer_id, created_at, updated_at, subtotal_minor, tax_total_minor)
         VALUES (?1, ?2, 'USD', 1, 'completed', ?3, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', ?2, 0)",
        rusqlite::params![id, total_minor, customer_id],
    )
    .unwrap();
}

#[test]
fn list_sales_for_customer_returns_only_that_customers_sales() {
    let conn = fresh();
    seed_sale_for_customer(&conn, "s-1", "cust-1", 1000);
    seed_sale_for_customer(&conn, "s-2", "cust-1", 2000);
    seed_sale_for_customer(&conn, "s-3", "cust-2", 3000);

    let (items, total) = store(&conn)
        .list_sales_for_customer("cust-1", 100, 0)
        .unwrap();
    assert_eq!(total, 2);
    assert_eq!(items.len(), 2);
    assert!(
        items
            .iter()
            .all(|s| s.customer_id.as_deref() == Some("cust-1"))
    );
}

#[test]
fn list_sales_for_customer_orders_most_recent_first() {
    let conn = fresh();
    seed_customer_row(&conn, "cust-1");
    conn.execute(
        "INSERT INTO sales (id, total_minor, currency, line_count, status, customer_id, created_at, updated_at, subtotal_minor, tax_total_minor)
         VALUES ('s-old', 100, 'USD', 1, 'completed', 'cust-1', '2024-01-01T00:00:00.000Z', '2024-01-01T00:00:00.000Z', 100, 0),
                ('s-new', 200, 'USD', 1, 'completed', 'cust-1', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z', 200, 0)",
        [],
    )
    .unwrap();

    let (items, _) = store(&conn)
        .list_sales_for_customer("cust-1", 100, 0)
        .unwrap();
    assert_eq!(items[0].id, "s-new");
    assert_eq!(items[1].id, "s-old");
}

#[test]
fn list_sales_for_customer_paginates() {
    let conn = fresh();
    for i in 0..3 {
        seed_sale_for_customer(&conn, &format!("s-{i}"), "cust-1", i);
    }

    let (page, total) = store(&conn)
        .list_sales_for_customer("cust-1", 2, 0)
        .unwrap();
    assert_eq!(total, 3);
    assert_eq!(page.len(), 2);

    let (rest, _) = store(&conn)
        .list_sales_for_customer("cust-1", 2, 2)
        .unwrap();
    assert_eq!(rest.len(), 1);
}

#[test]
fn list_sales_for_customer_no_sales_returns_empty() {
    let conn = fresh();
    let (items, total) = store(&conn)
        .list_sales_for_customer("cust-ghost", 100, 0)
        .unwrap();
    assert!(items.is_empty());
    assert_eq!(total, 0);
}

// ── Tax Rate Resolution Direct Tests ───────────────────────────────────

#[test]
fn resolve_best_tax_rates_returns_product_level_rates() {
    let conn = fresh();
    let s = store(&conn);

    // Arrange: Create two tax rates and assign both to product
    let vat_rate_id = seed_tax_rate(&conn, "VAT 10%", 1000, false, false);
    let sales_tax_id = seed_tax_rate(&conn, "Sales Tax 5%", 500, false, false);
    let _product_id = seed_product(&conn, "TEST-SKU", None);

    // Assign both tax rates to the product
    s.set_product_tax_rates("TEST-SKU", &[vat_rate_id.clone(), sales_tax_id.clone()])
        .unwrap();

    // Act: Resolve tax rates for the SKU
    let rates = s.resolve_best_tax_rates_for_sku("TEST-SKU").unwrap();

    // Assert: Both product-level rates should be returned (order may vary)
    assert_eq!(rates.len(), 2);
    let rate_ids: HashSet<_> = rates.iter().map(|r| r.id.as_str()).collect();
    assert!(rate_ids.contains(vat_rate_id.as_str()));
    assert!(rate_ids.contains(sales_tax_id.as_str()));
}

#[test]
fn resolve_best_tax_rates_falls_back_to_category_level() {
    let conn = fresh();
    let s = store(&conn);

    // Arrange: Create category tax rate, product with no direct rates but category assigned
    let cat_rate_id = seed_tax_rate(&conn, "Category Tax 8%", 800, false, false);
    let category_id = "CAT-TEST";
    s.create_category(category_id, "Test Category", "#ffffff", "")
        .unwrap();
    s.set_category_tax_rates(category_id, std::slice::from_ref(&cat_rate_id))
        .unwrap();

    seed_product_with_category(&conn, "TEST-SKU", Some(category_id));
    // Note: No product-level tax rates assigned

    // Act: Resolve tax rates for the SKU
    let rates = s.resolve_best_tax_rates_for_sku("TEST-SKU").unwrap();

    // Assert: Category-level rate should be returned
    assert_eq!(rates.len(), 1);
    assert_eq!(rates[0].id, cat_rate_id);
    assert_eq!(rates[0].name, "Category Tax 8%");
    assert_eq!(rates[0].rate_bps, 800);
}

#[test]
fn resolve_best_tax_rates_falls_back_to_default_store_rate() {
    let conn = fresh();
    let s = store(&conn);

    // Arrange: Create default store tax rate, product with no direct or category rates
    let default_rate_id = seed_tax_rate(&conn, "Default Store Tax 5%", 500, true, false);
    let _product_id = seed_product(&conn, "TEST-SKU", None);
    // Note: No product-level tax rates, no category assigned

    // Act: Resolve tax rates for the SKU
    let rates = s.resolve_best_tax_rates_for_sku("TEST-SKU").unwrap();

    // Assert: Default store rate should be returned
    assert_eq!(rates.len(), 1);
    assert_eq!(rates[0].id, default_rate_id);
    assert_eq!(rates[0].name, "Default Store Tax 5%");
    assert!(rates[0].is_default);
}

#[test]
fn resolve_best_tax_rates_returns_empty_when_no_rates_exist() {
    let conn = fresh();
    let s = store(&conn);

    // Arrange: Create product with no tax rates assigned anywhere
    let _product_id = seed_product(&conn, "TEST-SKU", None);
    // Note: No tax rates created at all, no product/category assignments

    // Act: Resolve tax rates for the SKU
    let rates = s.resolve_best_tax_rates_for_sku("TEST-SKU").unwrap();

    // Assert: Empty vector should be returned
    assert!(rates.is_empty());
}

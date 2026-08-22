use super::*;
use crate::migrations;
use crate::{Refund, RefundLine};
use rusqlite::Connection;

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

fn seed_completed_sale(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
            ('ref-p1', 'COFFEE', 'Coffee', 350, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at,
                            deduction_locations) VALUES
            ('ref-sale-1', 700, 'USD', 2, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z',
             '{\"version\":1,\"lines\":[{\"sale_line_id\":\"ref-sl-1\",\"sku\":\"COFFEE\",\"deductions\":[{\"location_id\":\"01926b3a-0000-7000-8000-000000000001\",\"qty\":2}]}]}');
         INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
            ('ref-sl-1', 'ref-sale-1', 'COFFEE', 2, 350, 700, 'USD', 1);"
    ).unwrap();
}

/// Seed a sale with multi-location split deductions for FIFO testing.
/// Loc A gets 2, Loc B gets 3.
fn seed_split_location_sale(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO inventory_locations (id, name, type) VALUES
            ('loc-store', 'Store Inventory', 'store'),
            ('loc-wh-a', 'Warehouse A', 'warehouse');
         INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
            ('p-cho', 'CHO-001', 'Choco Bar', 500, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at,
                            deduction_locations) VALUES
            ('split-sale-1', 2500, 'USD', 1, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z',
             '{\"version\":1,\"lines\":[{\"sale_line_id\":\"split-sl-1\",\"sku\":\"CHO-001\",\"deductions\":[{\"location_id\":\"loc-store\",\"qty\":2,\"sold_at\":\"2026-07-19T10:00:00Z\"},{\"location_id\":\"loc-wh-a\",\"qty\":3,\"sold_at\":\"2026-07-19T10:00:01Z\"}]}]}');
         INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
            ('split-sl-1', 'split-sale-1', 'CHO-001', 5, 500, 2500, 'USD', 1);"
    ).unwrap();
}

#[test]
fn create_refund_persists() {
    let conn = fresh();
    seed_completed_sale(&conn);
    let s = store(&conn);

    let line = RefundLine::new("ref-sl-1", "COFFEE", 2, price(350), price(700));
    let refund = Refund::new(
        "ref-sale-1",
        price(700),
        "customer changed mind",
        "",
        "user-1",
        vec![line],
    );

    s.create_refund(&refund).unwrap();

    let refunds = s.list_refunds_for_sale("ref-sale-1").unwrap();
    assert_eq!(refunds.len(), 1);
    assert_eq!(refunds[0].total.minor_units, 700);
    assert_eq!(refunds[0].total.currency, usd());
    assert_eq!(refunds[0].reason, "customer changed mind");
    assert_eq!(refunds[0].processed_by, "user-1");
    assert_eq!(refunds[0].lines.len(), 1);
    assert_eq!(refunds[0].lines[0].sku, "COFFEE");
    assert_eq!(refunds[0].lines[0].qty, 2);
}

#[test]
fn create_refund_nonexistent_sale_fails() {
    let conn = fresh();
    let s = store(&conn);

    let line = RefundLine::new("sl-x", "COFFEE", 1, price(350), price(350));
    let refund = Refund::new("nonexistent", price(350), "test", "", "user-1", vec![line]);

    let result = s.create_refund(&refund);
    assert!(result.is_err());
}

#[test]
fn list_refunds_empty_for_sale() {
    let conn = fresh();
    seed_completed_sale(&conn);
    let s = store(&conn);
    let refunds = s.list_refunds_for_sale("ref-sale-1").unwrap();
    assert!(refunds.is_empty());
}

#[test]
fn total_refunded_for_sale_no_refunds() {
    let conn = fresh();
    seed_completed_sale(&conn);
    let s = store(&conn);
    // No refunds → zero balance (not NotFound — callers use this as a
    // refundable-balance check).
    let result = s.total_refunded_for_sale("ref-sale-1").unwrap();
    assert_eq!(result.minor_units, 0);
    assert_eq!(result.currency, usd());
}

/// RED: a sale must not be refunded for MORE than the original total. The
/// current code applies no over-refund guard — the same completed sale can
/// be refunded unlimited times, and each refund restores stock.
#[test]
fn create_refund_rejects_over_refund() {
    let conn = fresh();
    seed_completed_sale(&conn);
    let s = store(&conn);

    // First refund: $7 (full amount).
    let line1 = RefundLine::new("ref-sl-1", "COFFEE", 2, price(350), price(700));
    let refund1 = Refund::new(
        "ref-sale-1",
        price(700),
        "refund",
        "",
        "user-1",
        vec![line1],
    );
    s.create_refund(&refund1).unwrap();

    // Second refund: $3.50 (partial — total refunded would be $10.50 > $7 sale).
    let line2 = RefundLine::new("ref-sl-1", "COFFEE", 1, price(350), price(350));
    let refund2 = Refund::new(
        "ref-sale-1",
        price(350),
        "over-refund",
        "",
        "user-1",
        vec![line2],
    );
    let err = s.create_refund(&refund2).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { .. }),
        "over-refunding a sale must be rejected, got: {err:?}"
    );
}

#[test]
fn multiple_partial_refunds() {
    let conn = fresh();
    seed_completed_sale(&conn);
    let s = store(&conn);

    // First refund: 1 item.
    let line1 = RefundLine::new("ref-sl-1", "COFFEE", 1, price(350), price(350));
    let r1 = Refund::new(
        "ref-sale-1",
        price(350),
        "partial",
        "",
        "user-1",
        vec![line1],
    );
    s.create_refund(&r1).unwrap();

    // Second refund: 1 item.
    let line2 = RefundLine::new("ref-sl-1", "COFFEE", 1, price(350), price(350));
    let r2 = Refund::new(
        "ref-sale-1",
        price(350),
        "partial",
        "",
        "user-1",
        vec![line2],
    );
    s.create_refund(&r2).unwrap();

    let refunds = s.list_refunds_for_sale("ref-sale-1").unwrap();
    assert_eq!(refunds.len(), 2);
    assert_eq!(refunds[0].total.minor_units, 350);
    assert_eq!(refunds[1].total.minor_units, 350);

    // Verify audit log entries.
    let audit_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'sale.refund' AND target_id = 'ref-sale-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(audit_count, 2);
}

// ── ADR-19 §5.3 FIFO refund stock restoration tests ─────────────

fn get_stock_at(conn: &Connection, sku: &str, location_id: &str) -> i64 {
    conn.query_row(
        "SELECT COALESCE(qty, 0) FROM stock_summary
         WHERE item_id = (SELECT id FROM products WHERE sku = ?1)
         AND location_id = ?2",
        rusqlite::params![sku, location_id],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

/// Full refund of a split-location sale — stock should be credited
/// forward (oldest deduction first): loc-store gets +2, loc-wh-a gets +3.
#[test]
fn refund_credits_split_location_full_refund_forward_fifo() {
    let conn = fresh();
    seed_split_location_sale(&conn);
    let s = store(&conn);

    // Initial stock is 0 at both locations.
    assert_eq!(get_stock_at(&conn, "CHO-001", "loc-store"), 0);
    assert_eq!(get_stock_at(&conn, "CHO-001", "loc-wh-a"), 0);

    // Full refund of all 5 units.
    let line = RefundLine::new("split-sl-1", "CHO-001", 5, price(500), price(2500));
    let refund = Refund::new(
        "split-sale-1",
        price(2500),
        "full refund",
        "",
        "user-1",
        vec![line],
    );
    s.create_refund(&refund).unwrap();

    // Stock credited forward: 2 to loc-store, 3 to loc-wh-a.
    assert_eq!(
        get_stock_at(&conn, "CHO-001", "loc-store"),
        2,
        "store gets 2 (oldest deduction first)"
    );
    assert_eq!(
        get_stock_at(&conn, "CHO-001", "loc-wh-a"),
        3,
        "warehouse gets 3 (second deduction)"
    );

    // Verify audit log.
    let audit_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'sale.refund' AND target_id = 'split-sale-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(audit_count, 1);
}

/// Partial refund of a split-location line — stock should be credited
/// in REVERSE order (most recent deduction first): loc-wh-a gets credited
/// before loc-store.
#[test]
fn refund_credits_split_location_partial_refund_reverse_order() {
    let conn = fresh();
    seed_split_location_sale(&conn);
    let s = store(&conn);

    // Refund 2 of 5 units (partial).
    let line = RefundLine::new("split-sl-1", "CHO-001", 2, price(500), price(1000));
    let refund = Refund::new(
        "split-sale-1",
        price(1000),
        "partial refund 2",
        "",
        "user-1",
        vec![line],
    );
    s.create_refund(&refund).unwrap();

    // Stock credited in reverse: loc-wh-a (most recent) gets 2,
    // loc-store gets 0 (remaining = 0 after warehouse covers it).
    assert_eq!(
        get_stock_at(&conn, "CHO-001", "loc-store"),
        0,
        "store gets 0 (partial refund credits most recent deduction first)"
    );
    assert_eq!(
        get_stock_at(&conn, "CHO-001", "loc-wh-a"),
        2,
        "warehouse gets 2 (most recent deduction credited first)"
    );
}

/// Partial refund that spans two deduction locations — credit crosses
/// from most recent to oldest.
#[test]
fn refund_credits_across_two_locations_partial() {
    let conn = fresh();
    seed_split_location_sale(&conn);
    let s = store(&conn);

    // Refund 4 of 5 units — should exhaust loc-wh-a (3) and take 1 from loc-store.
    let line = RefundLine::new("split-sl-1", "CHO-001", 4, price(500), price(2000));
    let refund = Refund::new(
        "split-sale-1",
        price(2000),
        "partial refund 4",
        "",
        "user-1",
        vec![line],
    );
    s.create_refund(&refund).unwrap();

    assert_eq!(
        get_stock_at(&conn, "CHO-001", "loc-wh-a"),
        3,
        "warehouse gets full 3 (most recent deduction first)"
    );
    assert_eq!(
        get_stock_at(&conn, "CHO-001", "loc-store"),
        1,
        "store gets remaining 1 after warehouse exhausted"
    );
}

/// Refund with qty larger than original deduction should fail.
#[test]
fn refund_qty_exceeds_original_deduction_fails() {
    let conn = fresh();
    seed_split_location_sale(&conn);
    let s = store(&conn);

    let line = RefundLine::new("split-sl-1", "CHO-001", 99, price(500), price(49500));
    let refund = Refund::new(
        "split-sale-1",
        price(49500),
        "excessive refund",
        "",
        "user-1",
        vec![line],
    );
    let result = s.create_refund(&refund);
    assert!(result.is_err(), "refund exceeding original qty should fail");
    match result.unwrap_err() {
        // The over-refund (total) guard fires first — the refund amount
        // 49500 exceeds the sale total 2500. The qty guard is the
        // second-line check.
        CoreError::Validation { field, .. } => {
            assert!(
                field == "total" || field == "refund_line.qty",
                "expected total or qty validation, got field: {field}"
            );
        }
        other => panic!("expected Validation error, got: {other:?}"),
    }
}

/// Legacy sale (NULL deduction_locations) falls back to default location.
#[test]
fn refund_legacy_sale_with_null_deduction_locations() {
    let conn = fresh();
    // The default location (01926b3a-...-001) is seeded by migration 078.
    // Use the old seed that sets deduction_locations = NULL explicitly.
    conn.execute_batch(
        "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
            ('legacy-p1', 'LEGACY', 'Legacy Item', 100, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
            ('legacy-sale-1', 200, 'USD', 1, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
            ('legacy-sl-1', 'legacy-sale-1', 'LEGACY', 2, 100, 200, 'USD', 1);"
    ).unwrap();
    let s = store(&conn);

    let line = RefundLine::new("legacy-sl-1", "LEGACY", 2, price(100), price(200));
    let refund = Refund::new(
        "legacy-sale-1",
        price(200),
        "legacy refund",
        "",
        "user-1",
        vec![line],
    );
    s.create_refund(&refund).unwrap();

    // Stock should be credited to default location.
    assert_eq!(
        get_stock_at(&conn, "LEGACY", "01926b3a-0000-7000-8000-000000000001"),
        2,
        "legacy refund credits to default location"
    );

    // Audit log should have the legacy warning entry (targets the refund,
    // not the sale, to avoid shadowing the primary `sale.refund` entry).
    let warn_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'sale.refund.legacy' AND target_id = ?1",
            params![refund.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        warn_count, 1,
        "legacy fallback should emit a warning audit entry"
    );
}

/// Verify that a refund correctly updates the stock_movements ledger.
#[test]
fn refund_creates_positive_stock_movements() {
    let conn = fresh();
    seed_split_location_sale(&conn);
    let s = store(&conn);

    let line = RefundLine::new("split-sl-1", "CHO-001", 3, price(500), price(1500));
    let refund = Refund::new(
        "split-sale-1",
        price(1500),
        "partial refund 3",
        "",
        "user-1",
        vec![line],
    );
    s.create_refund(&refund).unwrap();

    // Check that stock_movements has positive entries with reason 'refund' and location_id set.
    let movement_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements
             WHERE item_id = (SELECT id FROM products WHERE sku = 'CHO-001')
             AND reason = 'refund' AND delta > 0",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        movement_count, 1,
        "should have one positive movement (wh-a gets 3)"
    );

    let total_delta: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(delta), 0) FROM stock_movements
             WHERE item_id = (SELECT id FROM products WHERE sku = 'CHO-001')
             AND reason = 'refund'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        total_delta, 3,
        "total credited delta should match refund qty"
    );
}

// ── Additional edge cases ─────────────────────────────────────

#[test]
fn create_refund_note_persisted() {
    let conn = fresh();
    seed_completed_sale(&conn);
    let s = store(&conn);

    let line = RefundLine::new("ref-sl-1", "COFFEE", 2, price(350), price(700));
    let refund = Refund::new(
        "ref-sale-1",
        price(700),
        "defective",
        "Customer reported broken seal",
        "user-2",
        vec![line],
    );
    s.create_refund(&refund).unwrap();

    let refunds = s.list_refunds_for_sale("ref-sale-1").unwrap();
    assert_eq!(refunds.len(), 1);
    assert_eq!(refunds[0].note.as_str(), "Customer reported broken seal");
    assert_eq!(refunds[0].processed_by, "user-2");
}

#[test]
fn list_refunds_nonexistent_sale_returns_empty() {
    let conn = fresh();
    let s = store(&conn);
    let refunds = s.list_refunds_for_sale("no-such-sale").unwrap();
    assert!(refunds.is_empty());
}

#[test]
fn total_refunded_for_sale_accumulates() {
    let conn = fresh();
    seed_completed_sale(&conn);
    let s = store(&conn);

    // First refund: 350
    let line1 = RefundLine::new("ref-sl-1", "COFFEE", 1, price(350), price(350));
    let r1 = Refund::new(
        "ref-sale-1",
        price(350),
        "partial",
        "",
        "user-1",
        vec![line1],
    );
    s.create_refund(&r1).unwrap();

    // Second refund: 350
    let line2 = RefundLine::new("ref-sl-1", "COFFEE", 1, price(350), price(350));
    let r2 = Refund::new(
        "ref-sale-1",
        price(350),
        "partial",
        "",
        "user-1",
        vec![line2],
    );
    s.create_refund(&r2).unwrap();

    let total = s.total_refunded_for_sale("ref-sale-1").unwrap();
    assert_eq!(total.minor_units, 700);
}

#[test]
fn refund_line_not_in_deductions_fails() {
    let conn = fresh();
    seed_completed_sale(&conn);
    let s = store(&conn);

    // Refund line references a sale_line_id that doesn't exist in deduction_locations JSON.
    let line = RefundLine::new("non-existent-sl", "COFFEE", 1, price(350), price(350));
    let refund = Refund::new("ref-sale-1", price(350), "test", "", "user-1", vec![line]);
    let err = s.create_refund(&refund).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "deduction_locations"));
}

#[test]
fn refund_malformed_deduction_locations_json_fails() {
    let conn = fresh();
    // Sale with deliberately bad JSON in deduction_locations.
    conn.execute_batch(
        "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
            ('bad-p1', 'BAD', 'Bad Item', 100, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at,
                            deduction_locations) VALUES
            ('bad-sale-1', 100, 'USD', 1, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z',
             '{invalid json}');
         INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
            ('bad-sl-1', 'bad-sale-1', 'BAD', 1, 100, 100, 'USD', 1);"
    ).unwrap();
    let s = store(&conn);

    let line = RefundLine::new("bad-sl-1", "BAD", 1, price(100), price(100));
    let refund = Refund::new("bad-sale-1", price(100), "test", "", "user-1", vec![line]);
    let err = s.create_refund(&refund).unwrap_err();
    assert!(matches!(err, CoreError::Validation { field, .. } if field == "deduction_locations"));
}

#[test]
fn list_refunds_multiple_sales_isolation() {
    let conn = fresh();
    seed_completed_sale(&conn);
    // Also seed a second sale with a different sale ID.
    conn.execute_batch(
        "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at,
                            deduction_locations) VALUES
            ('ref-sale-2', 350, 'USD', 1, 'completed', '2025-01-02T00:00:00.000Z', '2025-01-02T00:00:00.000Z',
             '{\"version\":1,\"lines\":[{\"sale_line_id\":\"ref-sl-2\",\"sku\":\"COFFEE\",\"deductions\":[{\"location_id\":\"01926b3a-0000-7000-8000-000000000001\",\"qty\":1}]}]}');
         INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
            ('ref-sl-2', 'ref-sale-2', 'COFFEE', 1, 350, 350, 'USD', 1);"
    ).unwrap();
    let s = store(&conn);

    // Refund for sale-1
    let line1 = RefundLine::new("ref-sl-1", "COFFEE", 1, price(350), price(350));
    let r1 = Refund::new(
        "ref-sale-1",
        price(350),
        "partial",
        "",
        "user-1",
        vec![line1],
    );
    s.create_refund(&r1).unwrap();

    // Only refunds for sale-1 should appear
    let refunds1 = s.list_refunds_for_sale("ref-sale-1").unwrap();
    assert_eq!(refunds1.len(), 1);

    // Sale-2 should have zero refunds
    let refunds2 = s.list_refunds_for_sale("ref-sale-2").unwrap();
    assert!(refunds2.is_empty());
}

#[test]
fn total_refunded_for_nonexistent_sale_returns_not_found() {
    let conn = fresh();
    let s = store(&conn);
    let err = s.total_refunded_for_sale("no-such-sale").unwrap_err();
    assert!(matches!(err, CoreError::NotFound { .. }));
}

#[test]
fn refund_zero_price_line_restores_stock() {
    let conn = fresh();
    seed_completed_sale(&conn);
    let s = store(&conn);

    // A refund with zero-price line but positive qty should still restore stock.
    let line = RefundLine::new("ref-sl-1", "COFFEE", 1, price(0), price(0));
    let refund = Refund::new(
        "ref-sale-1",
        price(0),
        "zero price",
        "",
        "user-1",
        vec![line],
    );
    s.create_refund(&refund).unwrap();

    // Stock should still be restored despite zero monetary value.
    let movement_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE reason = 'refund' AND delta > 0",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        movement_count, 1,
        "zero-price refund should still create stock movement"
    );
}

#[test]
fn refund_empty_lines_vector_persists_refund_header() {
    let conn = fresh();
    seed_completed_sale(&conn);
    let s = store(&conn);

    // Refund with empty lines vec — still creates the refund header row.
    let refund = Refund::new("ref-sale-1", price(0), "void", "", "user-1", vec![]);
    s.create_refund(&refund).unwrap();

    let refunds = s.list_refunds_for_sale("ref-sale-1").unwrap();
    assert_eq!(refunds.len(), 1);
    assert_eq!(refunds[0].reason, "void");
    assert_eq!(refunds[0].lines.len(), 0);
}

#[test]
fn refund_partial_exact_qty_treated_as_full_forward_fifo() {
    let conn = fresh();
    seed_split_location_sale(&conn);
    let s = store(&conn);

    // Refund qty = total_deducted = 5 (exact match). Code does
    // `if refund_qty >= total_deducted` → forward FIFO path.
    let line = RefundLine::new("split-sl-1", "CHO-001", 5, price(500), price(2500));
    let refund = Refund::new(
        "split-sale-1",
        price(2500),
        "full refund",
        "",
        "user-1",
        vec![line],
    );
    s.create_refund(&refund).unwrap();

    // Forward FIFO: loc-store gets 2, loc-wh-a gets 3.
    assert_eq!(get_stock_at(&conn, "CHO-001", "loc-store"), 2);
    assert_eq!(get_stock_at(&conn, "CHO-001", "loc-wh-a"), 3);
}

//! Integration tests for the stock transfer module — full lifecycle
//! from draft → send (decrement inventory) → receive (increment
//! destination inventory) → cancel. Also tests partial receive, line
//! management, and validation.
//!
//! Tests exercise the full persistence layer via the public
//! [`oz_core::Store`] API against an in-memory SQLite database.

use oz_core::db::stock_transfers::ReceivedLine;
use oz_core::{Store, migrations};
use rusqlite::Connection;

// ── Helpers ───────────────────────────────────────────────────────────

fn setup() -> Connection {
    migrations::fresh_db()
}

fn store(conn: &Connection) -> Store<'_> {
    Store::new(conn)
}

fn seed_user(conn: &Connection, id: &str) {
    conn.execute(
        "INSERT OR IGNORE INTO roles (id, name, description, permissions, created_at, updated_at)
         VALUES ('role-owner', 'Owner', 'Owner role', '[\"*\"]',
                 '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO users (id, username, pin_hash, display_name, role_id,
                            created_at, updated_at)
         VALUES (?1, ?2, 'hash', ?3, 'role-owner',
                 '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
        rusqlite::params![id, id, id],
    )
    .unwrap();
}

fn seed_product(conn: &Connection, sku: &str, name: &str) {
    conn.execute(
        "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at)
         VALUES (?1, ?2, ?3, 1000, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
        rusqlite::params![uuid::Uuid::now_v7().to_string(), sku, name],
    )
    .unwrap();
}

fn seed_inventory(conn: &Connection, sku: &str, qty: i64) {
    let pid: String = conn
        .query_row(
            "SELECT id FROM products WHERE sku=?1",
            rusqlite::params![sku],
            |r| r.get(0),
        )
        .unwrap();
    conn.execute(
        "INSERT INTO inventory (product_id, qty, updated_at) VALUES (?1, ?2, '2025-01-01T00:00:00.000Z')",
        rusqlite::params![pid, qty],
    )
    .unwrap();
}

fn get_inventory_qty(conn: &Connection, sku: &str) -> i64 {
    conn.query_row(
        "SELECT COALESCE(qty, 0) FROM inventory i
         JOIN products p ON i.product_id = p.id WHERE p.sku = ?1",
        rusqlite::params![sku],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

use oz_core::StockTransferLine;

fn make_line(sku: &str, name: &str, qty: i64) -> StockTransferLine {
    StockTransferLine {
        id: String::new(),
        transfer_id: String::new(),
        sku: sku.into(),
        product_name: name.into(),
        qty,
        received_qty: 0,
    }
}

fn create_draft(
    conn: &Connection,
    created_by: &str,
    lines: &[StockTransferLine],
) -> oz_core::StockTransfer {
    store(conn)
        .create_transfer(
            Some("Warehouse A"),
            Some("Store B"),
            None,
            None,
            "Integration test",
            created_by,
            lines,
        )
        .unwrap()
}

// ── Create ────────────────────────────────────────────────────────────

#[test]
fn create_transfer_starts_as_draft() {
    let conn = setup();
    seed_user(&conn, "staff-1");
    seed_product(&conn, "SKU-W", "Widget");
    seed_inventory(&conn, "SKU-W", 100);

    let t = create_draft(&conn, "staff-1", &[make_line("SKU-W", "Widget", 10)]);
    assert_eq!(t.status, "draft");
    assert!(t.transfer_number.starts_with("TRF-"));
    assert_eq!(t.source_location.as_deref(), Some("Warehouse A"));
    assert_eq!(t.destination_location.as_deref(), Some("Store B"));
    assert_eq!(t.notes, "Integration test");
    assert!(t.sent_at.is_none());
    assert!(t.received_at.is_none());
}

#[test]
fn create_transfer_without_lines_succeeds() {
    let conn = setup();
    seed_user(&conn, "staff-1");

    let t = create_draft(&conn, "staff-1", &[]);
    assert_eq!(t.status, "draft");
    let lines = store(&conn).get_transfer_lines(&t.id).unwrap();
    assert!(lines.is_empty());
}

// ── List ──────────────────────────────────────────────────────────────

#[test]
fn list_transfers_newest_first() {
    let conn = setup();
    seed_user(&conn, "staff-1");
    seed_product(&conn, "SKU-W", "Widget");
    seed_inventory(&conn, "SKU-W", 100);

    let lines = vec![make_line("SKU-W", "Widget", 5)];
    let t1 = store(&conn)
        .create_transfer(None, None, None, None, "first", "staff-1", &lines)
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let t2 = store(&conn)
        .create_transfer(None, None, None, None, "second", "staff-1", &lines)
        .unwrap();

    let all = store(&conn).list_transfers().unwrap();
    assert_eq!(all.len(), 2);
    // Newest first.
    assert_eq!(all[0].id, t2.id);
    assert_eq!(all[1].id, t1.id);
}

#[test]
fn get_transfer_not_found() {
    let conn = setup();
    let t = store(&conn).get_transfer("nope").unwrap();
    assert!(t.is_none());
}

// ── Send (decrement inventory) ──────────────────────────────────────

#[test]
fn send_transfer_moves_to_in_transit_and_decrements_inventory() {
    let conn = setup();
    seed_user(&conn, "staff-1");
    seed_product(&conn, "SKU-A", "Product A");
    seed_inventory(&conn, "SKU-A", 50);

    let lines = vec![make_line("SKU-A", "Product A", 15)];
    let t = create_draft(&conn, "staff-1", &lines);

    let sent = store(&conn).send_transfer(&t.id).unwrap();
    assert_eq!(sent.status, "in_transit");
    assert!(sent.sent_at.is_some());

    // Source inventory decremented: 50 - 15 = 35.
    assert_eq!(get_inventory_qty(&conn, "SKU-A"), 35);
}

#[test]
fn send_transfer_insufficient_stock_fails() {
    let conn = setup();
    seed_user(&conn, "staff-1");
    seed_product(&conn, "SKU-B", "Product B");
    seed_inventory(&conn, "SKU-B", 5);

    let lines = vec![make_line("SKU-B", "Product B", 20)];
    let t = create_draft(&conn, "staff-1", &lines);

    let err = store(&conn).send_transfer(&t.id).unwrap_err();
    assert!(matches!(
        err,
        oz_core::CoreError::Validation { field: "qty", .. }
    ));
    // Inventory unchanged.
    assert_eq!(get_inventory_qty(&conn, "SKU-B"), 5);
}

#[test]
fn send_already_sent_transfer_fails() {
    let conn = setup();
    seed_user(&conn, "staff-1");
    seed_product(&conn, "SKU-C", "Product C");
    seed_inventory(&conn, "SKU-C", 50);

    let lines = vec![make_line("SKU-C", "Product C", 10)];
    let t = create_draft(&conn, "staff-1", &lines);
    store(&conn).send_transfer(&t.id).unwrap();

    let err = store(&conn).send_transfer(&t.id).unwrap_err();
    assert!(matches!(
        err,
        oz_core::CoreError::Validation {
            field: "status",
            ..
        }
    ));
}

// ── Receive (increment destination inventory) ───────────────────────

#[test]
fn receive_transfer_increments_inventory_and_sets_received() {
    let conn = setup();
    seed_user(&conn, "staff-1");
    seed_user(&conn, "staff-2");
    seed_product(&conn, "SKU-D", "Product D");
    seed_inventory(&conn, "SKU-D", 50);

    let lines = vec![make_line("SKU-D", "Product D", 10)];
    let t = create_draft(&conn, "staff-1", &lines);
    store(&conn).send_transfer(&t.id).unwrap();

    let transfer_lines = store(&conn).get_transfer_lines(&t.id).unwrap();

    let received = store(&conn)
        .receive_transfer(
            &t.id,
            "staff-2",
            &[ReceivedLine {
                line_id: transfer_lines[0].id.clone(),
                received_qty: 10,
            }],
        )
        .unwrap();

    assert_eq!(received.status, "received");
    assert_eq!(received.received_by.as_deref(), Some("staff-2"));
    assert!(received.received_at.is_some());

    // Destination inventory: 50 - 10 (send) + 10 (receive) = 50.
    // The send decremented first because source and receiver use the same product.
    assert_eq!(get_inventory_qty(&conn, "SKU-D"), 50);
}

#[test]
fn partial_receive_stays_in_transit() {
    let conn = setup();
    seed_user(&conn, "staff-1");
    seed_user(&conn, "staff-2");
    seed_product(&conn, "SKU-E", "Product E");
    seed_inventory(&conn, "SKU-E", 30);

    let lines = vec![make_line("SKU-E", "Product E", 10)];
    let t = create_draft(&conn, "staff-1", &lines);
    store(&conn).send_transfer(&t.id).unwrap();

    let transfer_lines = store(&conn).get_transfer_lines(&t.id).unwrap();

    let result = store(&conn)
        .receive_transfer(
            &t.id,
            "staff-2",
            &[ReceivedLine {
                line_id: transfer_lines[0].id.clone(),
                received_qty: 4,
            }],
        )
        .unwrap();

    assert_eq!(result.status, "in_transit");
    // Only 4 received, 10 total — status stays in_transit.
}

#[test]
fn receive_non_in_transit_transfer_fails() {
    let conn = setup();
    seed_user(&conn, "staff-1");

    let lines = vec![make_line("SKU", "N/A", 1)];
    let t = create_draft(&conn, "staff-1", &lines);

    let err = store(&conn)
        .receive_transfer(&t.id, "staff-2", &[])
        .unwrap_err();
    assert!(matches!(
        err,
        oz_core::CoreError::Validation {
            field: "status",
            ..
        }
    ));
}

// ── Cancel ────────────────────────────────────────────────────────────

#[test]
fn cancel_draft_transfer_sets_status() {
    let conn = setup();
    seed_user(&conn, "staff-1");

    let t = create_draft(&conn, "staff-1", &[]);
    let cancelled = store(&conn).cancel_transfer(&t.id).unwrap();
    assert_eq!(cancelled.status, "cancelled");
}

#[test]
fn cancel_received_transfer_fails() {
    let conn = setup();
    seed_user(&conn, "staff-1");
    seed_user(&conn, "staff-2");
    seed_product(&conn, "SKU-F", "Product F");
    seed_inventory(&conn, "SKU-F", 50);

    let lines = vec![make_line("SKU-F", "Product F", 10)];
    let t = create_draft(&conn, "staff-1", &lines);
    store(&conn).send_transfer(&t.id).unwrap();

    let transfer_lines = store(&conn).get_transfer_lines(&t.id).unwrap();
    store(&conn)
        .receive_transfer(
            &t.id,
            "staff-2",
            &[ReceivedLine {
                line_id: transfer_lines[0].id.clone(),
                received_qty: 10,
            }],
        )
        .unwrap();

    let err = store(&conn).cancel_transfer(&t.id).unwrap_err();
    assert!(matches!(
        err,
        oz_core::CoreError::Validation {
            field: "status",
            ..
        }
    ));
}

// ── Line management ──────────────────────────────────────────────────

#[test]
fn add_and_remove_lines_from_draft() {
    let conn = setup();
    seed_user(&conn, "staff-1");
    seed_product(&conn, "SKU-G", "Product G");
    seed_inventory(&conn, "SKU-G", 100);

    let t = create_draft(&conn, "staff-1", &[]);

    let line = store(&conn)
        .add_transfer_line(&t.id, "SKU-G", "Product G", 5)
        .unwrap();
    assert_eq!(line.qty, 5);

    let lines = store(&conn).get_transfer_lines(&t.id).unwrap();
    assert_eq!(lines.len(), 1);

    store(&conn).remove_transfer_line(&line.id).unwrap();

    let lines = store(&conn).get_transfer_lines(&t.id).unwrap();
    assert_eq!(lines.len(), 0);
}

#[test]
fn add_line_to_non_draft_transfer_fails() {
    let conn = setup();
    seed_user(&conn, "staff-1");
    seed_product(&conn, "SKU-H", "Product H");
    seed_inventory(&conn, "SKU-H", 100);

    let t = create_draft(&conn, "staff-1", &[]);
    store(&conn).send_transfer(&t.id).unwrap();

    let err = store(&conn)
        .add_transfer_line(&t.id, "SKU-H", "Product H", 5)
        .unwrap_err();
    assert!(matches!(
        err,
        oz_core::CoreError::Validation {
            field: "status",
            ..
        }
    ));
}

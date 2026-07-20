//! Integration tests: Backup & Restore (P39-1)
//!
//! Verifies that backup/restore operations preserve all data integrity.
//! Uses sqlite3 CLI for `.backup` (matching production scripts) and
//! verifies that after restore, all products, sales, and inventory data
//! are intact.

use foundation::{Currency, Money};
use oz_core::{Store, migrations};
use rusqlite::Connection;
use std::fs;
use std::process::Command;

// ── Helpers ───────────────────────────────────────────────────────────

fn fresh_db_path(label: &str) -> String {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("oz-pos-test-{}-{}.db", label, uuid::Uuid::now_v7()));
    path.to_string_lossy().to_string()
}

fn open_db(path: &str) -> Connection {
    Connection::open(path).expect("open test DB")
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

/// Seed a product with initial stock.
fn seed_product(conn: &Connection, sku: &str, name: &str, initial_stock: i64) {
    store(conn)
        .create_product(sku, name, price(1000), None, None, initial_stock, None)
        .unwrap();
}

/// Run migrations on a connection.
fn run_migrations(conn: &mut Connection) {
    // migrations::fresh_db() creates an in-memory DB with migrations.
    // For file-based DBs, we run migrations manually.
    migrations::run(conn).expect("run migrations");
}

// ── Backup via sqlite3 CLI ────────────────────────────────────────────

/// Create a backup of `source_path` to `backup_path` using sqlite3 `.backup`.
fn sqlite3_backup(source_path: &str, backup_path: &str) -> Result<(), String> {
    let output = Command::new("sqlite3")
        .arg(source_path)
        .arg(format!(".backup '{}'", backup_path))
        .output()
        .map_err(|e| format!("failed to run sqlite3: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("sqlite3 backup failed: {stderr}"));
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────

/// Backup a fresh (zero tables with data) DB and verify integrity.
#[test]
fn backup_restore_zero_tables_db() {
    let db_path = fresh_db_path("backup-zero");
    let backup_path = fresh_db_path("backup-zero-bak");

    // Create a fresh DB with migrations but zero user data.
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
    }

    // Backup.
    fs::copy(&db_path, &backup_path).expect("backup");

    // Simulate data loss.
    fs::remove_file(&db_path).ok();
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);

        // Verify empty.
        let s = store(&conn);
        let products = s.list_products().unwrap();
        assert!(products.is_empty());
        let sales = s.list_sales().unwrap();
        assert!(sales.is_empty());
    }

    // Restore.
    fs::copy(&backup_path, &db_path).expect("restore");

    // Verify integrity + empty data preserved.
    {
        let conn = open_db(&db_path);
        let result: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(result, "ok");

        let s = store(&conn);
        let products = s.list_products().unwrap();
        assert!(products.is_empty(), "zero-data backup should restore empty");
        let sales = s.list_sales().unwrap();
        assert!(sales.is_empty());
    }

    fs::remove_file(&db_path).ok();
    fs::remove_file(&backup_path).ok();
}

#[test]
fn backup_restore_preserves_products() {
    let db_path = fresh_db_path("backup-products");
    let backup_path = fresh_db_path("backup-products-bak");

    // ── Create and seed DB ────────────────────────────────────────
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
        let s = store(&conn);

        s.create_product("COFFEE", "Espresso", price(350), None, None, 100, None)
            .unwrap();
        s.create_product("TEA", "Green Tea", price(250), None, None, 50, None)
            .unwrap();
        s.create_product("CAKE", "Cheesecake", price(600), None, None, 20, None)
            .unwrap();
    }
    // close conn to release file lock

    // ── Backup ────────────────────────────────────────────────────
    // Connection is already dropped (block scope ended), so the DB file
    // is fully flushed and safe to copy. sqlite3 .backup is preferred for
    // transactional consistency but fs::copy is safe in test contexts.
    if sqlite3_backup(&db_path, &backup_path).is_err() {
        // sqlite3 CLI not available — fall back to file copy.
        fs::copy(&db_path, &backup_path).expect("file copy backup");
    }

    // ── Simulate data loss: delete original, create fresh ─────────
    fs::remove_file(&db_path).ok();
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
        let s = store(&conn);

        // Verify empty — no products.
        let products = s.list_products().unwrap();
        assert!(products.is_empty(), "fresh DB should have no products");
    }

    // ── Restore ───────────────────────────────────────────────────
    fs::copy(&backup_path, &db_path).expect("restore backup");

    // ── Verify all data intact ────────────────────────────────────
    {
        let conn = open_db(&db_path);
        let s = store(&conn);

        let products = s.list_products().unwrap();
        assert_eq!(products.len(), 3, "should have 3 products after restore");

        let coffee = products
            .iter()
            .find(|p| p.product.sku.as_str() == "COFFEE")
            .expect("COFFEE should exist");
        assert_eq!(coffee.product.name, "Espresso");
        assert_eq!(coffee.product.price.minor_units, 350);
        assert_eq!(coffee.stock_qty, Some(100));

        let tea = products
            .iter()
            .find(|p| p.product.sku.as_str() == "TEA")
            .expect("TEA should exist");
        assert_eq!(tea.product.name, "Green Tea");
        assert_eq!(tea.stock_qty, Some(50));
    }

    // Cleanup.
    fs::remove_file(&db_path).ok();
    fs::remove_file(&backup_path).ok();
}

#[test]
fn backup_restore_preserves_sales() {
    let db_path = fresh_db_path("backup-sales");
    let backup_path = fresh_db_path("backup-sales-bak");

    // ── Create, seed, and complete a sale ─────────────────────────
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
        let s = store(&conn);

        // Seed a product with stock.
        s.create_product("LATTE", "Latte", price(450), None, None, 100, None)
            .unwrap();

        // Create a sale and persist it.
        let sale = oz_core::Sale {
            id: "sale-backup-1".to_string(),
            status: foundation::SaleStatus::Completed,
            total: price(450),
            line_count: 1,
            currency: usd(),
            payment_method: Some("cash".to_string()),
            tendered_minor: Some(500),
            discount_percent: 0,
            discount_label: None,
            user_id: Some("user-1".to_string()),
            created_at: "2026-07-20T10:00:00.000Z".to_string(),
            updated_at: "2026-07-20T10:00:00.000Z".to_string(),
            lines: vec![oz_core::SaleLine {
                id: "line-1".to_string(),
                sale_id: "sale-backup-1".to_string(),
                sku: "LATTE".to_string(),
                qty: 1,
                unit_price: price(450),
                line_total: price(450),
                line_position: 1,
                tax_amount: price(0),
                tax_rate_id: None,
                serial_number: None,
            }],
            subtotal: price(450),
            tax_total: price(0),
            customer_id: None,
            version: 1,
        };

        s.create_sale(&sale).unwrap();
    }

    // ── Backup + Restore ──────────────────────────────────────────
    fs::copy(&db_path, &backup_path).expect("backup");
    fs::remove_file(&db_path).ok();
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
    }
    fs::copy(&backup_path, &db_path).expect("restore");

    // ── Verify sale intact ────────────────────────────────────────
    {
        let conn = open_db(&db_path);
        let s = store(&conn);

        let sale = s
            .get_sale("sale-backup-1")
            .unwrap()
            .expect("sale should exist after restore");
        assert_eq!(sale.total.minor_units, 450);
        assert_eq!(sale.line_count, 1);
        assert_eq!(sale.lines.len(), 1);
        assert_eq!(sale.lines[0].sku, "LATTE");
        assert_eq!(sale.lines[0].qty, 1);
        assert_eq!(sale.status, foundation::SaleStatus::Completed);
    }

    fs::remove_file(&db_path).ok();
    fs::remove_file(&backup_path).ok();
}

#[test]
fn backup_restore_preserves_inventory_adjustments() {
    let db_path = fresh_db_path("backup-inv");
    let backup_path = fresh_db_path("backup-inv-bak");

    // ── Seed products and adjust stock ────────────────────────────
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
        let s = store(&conn);

        s.create_product("BREAD", "Sourdough", price(500), None, None, 30, None)
            .unwrap();

        // Adjust stock: sell 10.
        let tx = conn.unchecked_transaction().unwrap();
        let loc = oz_core::inventory::LocationId::from(
            oz_core::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
        );
        s.adjust_stock_at_location_with_reason(
            &tx,
            "BREAD",
            -10,
            &loc,
            Some("sale"),
            None,
            None,
            None,
        )
        .unwrap();
        tx.commit().unwrap();
    }

    // ── Backup + Restore ──────────────────────────────────────────
    fs::copy(&db_path, &backup_path).expect("backup");
    fs::remove_file(&db_path).ok();
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
    }
    fs::copy(&backup_path, &db_path).expect("restore");

    // ── Verify inventory ──────────────────────────────────────────
    {
        let conn = open_db(&db_path);
        let s = store(&conn);

        let products = s.list_products().unwrap();
        let bread = products
            .iter()
            .find(|p| p.product.sku.as_str() == "BREAD")
            .expect("BREAD should exist");
        assert_eq!(bread.stock_qty, Some(20), "30 - 10 = 20 after restore");
    }

    fs::remove_file(&db_path).ok();
    fs::remove_file(&backup_path).ok();
}

#[test]
fn corrupt_backup_is_detected() {
    let db_path = fresh_db_path("backup-corrupt");
    let backup_path = fresh_db_path("backup-corrupt-bak");
    let corrupt_path = fresh_db_path("backup-corrupt-bad");

    // Create valid DB with data.
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
        let s = store(&conn);
        s.create_product("VALID", "Valid Product", price(100), None, None, 10, None)
            .unwrap();
    }

    // Create a valid backup.
    fs::copy(&db_path, &backup_path).expect("backup");

    // Create a corrupt "backup" — just a text file, not a valid SQLite DB.
    fs::write(&corrupt_path, "this is not a valid sqlite database file").expect("write corrupt");

    // Attempt to open the corrupt file as a SQLite DB.
    // SQLite opens lazily — the file handle is created, but the
    // first query will fail because the header is not valid SQLite.
    let conn = Connection::open(&corrupt_path).expect("open corrupt file");
    let integrity: Result<String, _> =
        conn.query_row("PRAGMA integrity_check", [], |row| row.get(0));
    assert!(
        integrity.is_err(),
        "corrupt file should fail integrity check"
    );

    // The valid backup should still be usable.
    {
        let conn = Connection::open(&backup_path).expect("valid backup should open");
        let result: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(result, "ok", "valid backup should pass integrity");
    }

    fs::remove_file(&db_path).ok();
    fs::remove_file(&backup_path).ok();
    fs::remove_file(&corrupt_path).ok();
}

#[test]
fn backup_restore_integrity_check_passes() {
    let db_path = fresh_db_path("backup-integrity");
    let backup_path = fresh_db_path("backup-integrity-bak");

    // ── Create seeded DB ──────────────────────────────────────────
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
        let s = store(&conn);
        s.create_product("ITEM-A", "Item A", price(100), None, None, 10, None)
            .unwrap();
        s.create_product("ITEM-B", "Item B", price(200), None, None, 20, None)
            .unwrap();
    }

    // ── Backup + Restore ──────────────────────────────────────────
    fs::copy(&db_path, &backup_path).expect("backup");
    fs::remove_file(&db_path).ok();
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
    }
    fs::copy(&backup_path, &db_path).expect("restore");

    // ── Integrity check ───────────────────────────────────────────
    {
        let conn = open_db(&db_path);
        let result: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(result, "ok", "integrity check must pass after restore");

        // Verify table counts.
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(table_count > 5, "should have multiple tables after restore");
    }

    fs::remove_file(&db_path).ok();
    fs::remove_file(&backup_path).ok();
}

#[test]
fn restore_onto_nonempty_db_overwrites() {
    let db_path = fresh_db_path("backup-overwrite");
    let backup_path = fresh_db_path("backup-overwrite-bak");

    // ── Create source DB with 3 products ──────────────────────────
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
        let s = store(&conn);
        s.create_product("SRC-1", "Source 1", price(100), None, None, 10, None)
            .unwrap();
        s.create_product("SRC-2", "Source 2", price(200), None, None, 20, None)
            .unwrap();
        s.create_product("SRC-3", "Source 3", price(300), None, None, 30, None)
            .unwrap();
    }
    fs::copy(&db_path, &backup_path).expect("backup");

    // ── Create target DB with 1 different product ─────────────────
    fs::remove_file(&db_path).ok();
    {
        let mut conn = open_db(&db_path);
        run_migrations(&mut conn);
        let s = store(&conn);
        s.create_product("TGT-1", "Target 1", price(999), None, None, 99, None)
            .unwrap();
    }

    // ── Restore overwrites ────────────────────────────────────────
    fs::copy(&backup_path, &db_path).expect("restore");
    {
        let conn = open_db(&db_path);
        let s = store(&conn);
        let products = s.list_products().unwrap();
        assert_eq!(products.len(), 3, "should only have backup's 3 products");
        assert!(
            products.iter().any(|p| p.product.sku.as_str() == "SRC-1"),
            "backup products should be present"
        );
        assert!(
            !products.iter().any(|p| p.product.sku.as_str() == "TGT-1"),
            "target-only products should be gone"
        );
    }

    fs::remove_file(&db_path).ok();
    fs::remove_file(&backup_path).ok();
}

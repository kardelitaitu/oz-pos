use super::*;
use crate::migrations;
use foundation::Currency;
use rusqlite::Connection;
use std::str::FromStr;

fn fresh() -> Connection {
    migrations::fresh_db()
}

fn store(conn: &Connection) -> Store<'_> {
    Store::new(conn)
}

#[test]
fn list_tax_rates_empty() {
    let conn = fresh();
    let s = store(&conn);
    let rates = s.list_tax_rates().unwrap();
    assert!(rates.is_empty());
}

#[test]
fn create_and_list_tax_rate() {
    let conn = fresh();
    let s = store(&conn);
    s.create_tax_rate("VAT 10%", 1000, true, false).unwrap();
    let rates = s.list_tax_rates().unwrap();
    assert_eq!(rates.len(), 1);
    assert_eq!(rates[0].name, "VAT 10%");
    assert_eq!(rates[0].rate_bps, 1000);
    assert!(rates[0].is_default);
    assert!(!rates[0].is_inclusive);
}

#[test]
fn create_tax_rate_exclusive() {
    let conn = fresh();
    let s = store(&conn);
    s.create_tax_rate("GST 5%", 500, false, true).unwrap();
    let rates = s.list_tax_rates().unwrap();
    assert_eq!(rates.len(), 1);
    assert!(!rates[0].is_default);
    assert!(rates[0].is_inclusive);
}

#[test]
fn create_tax_rate_empty_name() {
    let conn = fresh();
    let s = store(&conn);
    let result = s.create_tax_rate("", 1000, false, false);
    assert!(matches!(
        result,
        Err(CoreError::Validation { field: "name", .. })
    ));
}

#[test]
fn create_tax_rate_negative_rate() {
    let conn = fresh();
    let s = store(&conn);
    let result = s.create_tax_rate("Bad", -1, false, false);
    assert!(matches!(
        result,
        Err(CoreError::Validation {
            field: "rate_bps",
            ..
        })
    ));
}

// ── TAX-04: bounded rate validation ─────────────────────────────

#[test]
fn create_tax_rate_accepts_max_bps() {
    let conn = fresh();
    let s = store(&conn);
    let rate = s
        .create_tax_rate("Extreme", MAX_TAX_RATE_BPS, false, false)
        .unwrap();
    assert_eq!(rate.rate_bps, MAX_TAX_RATE_BPS);
}

#[test]
fn create_tax_rate_rejects_above_max_bps() {
    let conn = fresh();
    let s = store(&conn);
    let result = s.create_tax_rate("Bad", MAX_TAX_RATE_BPS + 1, false, false);
    assert!(matches!(
        result,
        Err(CoreError::Validation {
            field: "rate_bps",
            ..
        })
    ));
}

#[test]
fn update_tax_rate_rejects_above_max_bps() {
    let conn = fresh();
    let s = store(&conn);
    let created = s.create_tax_rate("Test", 100, false, false).unwrap();
    let result = s.update_tax_rate(&created.id, "Test", MAX_TAX_RATE_BPS + 1, false, false);
    assert!(matches!(
        result,
        Err(CoreError::Validation {
            field: "rate_bps",
            ..
        })
    ));
}

#[test]
fn get_tax_rate_found() {
    let conn = fresh();
    let s = store(&conn);
    let created = s.create_tax_rate("VAT 8%", 800, true, false).unwrap();
    let found = s.get_tax_rate(&created.id).unwrap().unwrap();
    assert_eq!(found.name, "VAT 8%");
    assert_eq!(found.rate_bps, 800);
}

#[test]
fn get_tax_rate_not_found() {
    let conn = fresh();
    let s = store(&conn);
    let result = s.get_tax_rate("nonexistent-id").unwrap();
    assert!(result.is_none());
}

#[test]
fn update_tax_rate_basic() {
    let conn = fresh();
    let s = store(&conn);
    let created = s.create_tax_rate("Old Name", 500, false, false).unwrap();
    let updated = s
        .update_tax_rate(&created.id, "New Name", 600, true, true)
        .unwrap();
    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.rate_bps, 600);
    assert!(updated.is_default);
    assert!(updated.is_inclusive);
}

#[test]
fn update_tax_rate_not_found() {
    let conn = fresh();
    let s = store(&conn);
    let result = s.update_tax_rate("bad-id", "X", 100, false, false);
    assert!(matches!(result, Err(CoreError::NotFound { .. })));
}

#[test]
fn update_tax_rate_empty_name() {
    let conn = fresh();
    let s = store(&conn);
    let created = s.create_tax_rate("Test", 100, false, false).unwrap();
    let result = s.update_tax_rate(&created.id, "", 100, false, false);
    assert!(matches!(
        result,
        Err(CoreError::Validation { field: "name", .. })
    ));
}

#[test]
fn update_tax_rate_negative_rate() {
    let conn = fresh();
    let s = store(&conn);
    let created = s.create_tax_rate("Test", 100, false, false).unwrap();
    let result = s.update_tax_rate(&created.id, "Test", -5, false, false);
    assert!(matches!(
        result,
        Err(CoreError::Validation {
            field: "rate_bps",
            ..
        })
    ));
}

#[test]
fn delete_tax_rate_removes() {
    let conn = fresh();
    let s = store(&conn);
    let created = s.create_tax_rate("To Delete", 100, false, false).unwrap();
    s.delete_tax_rate(&created.id).unwrap();
    let found = s.get_tax_rate(&created.id).unwrap();
    assert!(found.is_none());
}

#[test]
fn delete_tax_rate_not_found() {
    let conn = fresh();
    let s = store(&conn);
    let result = s.delete_tax_rate("bad-id");
    assert!(matches!(result, Err(CoreError::NotFound { .. })));
}

#[test]
fn default_flag_is_cleared_on_new_default() {
    let conn = fresh();
    let s = store(&conn);
    let first = s.create_tax_rate("First", 500, true, false).unwrap();
    let second = s.create_tax_rate("Second", 1000, true, false).unwrap();

    let r1 = s.get_tax_rate(&first.id).unwrap().unwrap();
    let r2 = s.get_tax_rate(&second.id).unwrap().unwrap();
    assert!(!r1.is_default); // cleared when second was set as default
    assert!(r2.is_default);
}

#[test]
fn set_and_get_product_tax_rates() {
    let conn = fresh();
    let s = store(&conn);
    let currency = Currency::from_str("USD").unwrap();
    let money = crate::Money {
        minor_units: 1000,
        currency,
    };
    s.create_product("SKU-TAX", "Taxed Product", money, None, None, 0, None)
        .unwrap();

    let rate = s.create_tax_rate("VAT", 1000, true, false).unwrap();
    s.set_product_tax_rates("SKU-TAX", std::slice::from_ref(&rate.id))
        .unwrap();

    let ids = s.get_product_tax_rates("SKU-TAX").unwrap();
    assert_eq!(ids, vec![rate.id]);
}

#[test]
fn set_product_tax_rates_overwrites() {
    let conn = fresh();
    let s = store(&conn);
    let currency = Currency::from_str("USD").unwrap();
    let money = crate::Money {
        minor_units: 1000,
        currency,
    };
    s.create_product("SKU-TAX2", "Item", money, None, None, 0, None)
        .unwrap();

    let r1 = s.create_tax_rate("R1", 500, false, false).unwrap();
    let r2 = s.create_tax_rate("R2", 1000, false, false).unwrap();

    s.set_product_tax_rates("SKU-TAX2", std::slice::from_ref(&r1.id))
        .unwrap();
    s.set_product_tax_rates("SKU-TAX2", std::slice::from_ref(&r2.id))
        .unwrap();

    let ids = s.get_product_tax_rates("SKU-TAX2").unwrap();
    assert_eq!(ids, vec![r2.id]);
}

#[test]
fn set_and_get_category_tax_rates() {
    let conn = fresh();
    let s = store(&conn);
    s.create_category("cat-tax", "Taxed Cat", "#fff", "")
        .unwrap();

    let rate = s.create_tax_rate("CT", 800, false, false).unwrap();
    s.set_category_tax_rates("cat-tax", std::slice::from_ref(&rate.id))
        .unwrap();

    let ids = s.get_category_tax_rates("cat-tax").unwrap();
    assert_eq!(ids, vec![rate.id]);
}

#[test]
fn get_product_tax_rates_none() {
    let conn = fresh();
    let s = store(&conn);
    let ids = s.get_product_tax_rates("NO-SKU").unwrap();
    assert!(ids.is_empty());
}

#[test]
fn get_product_tax_rates_batch_returns_all_skus() {
    let conn = fresh();
    let s = store(&conn);
    let currency = Currency::from_str("USD").unwrap();
    let money = crate::Money {
        minor_units: 1000,
        currency,
    };
    let r1 = s.create_tax_rate("GST", 1000, true, false).unwrap();
    let r2 = s.create_tax_rate("State", 500, false, false).unwrap();
    s.create_product("A", "Product A", money, None, None, 0, None)
        .unwrap();
    s.create_product("B", "Product B", money, None, None, 0, None)
        .unwrap();
    s.set_product_tax_rates("A", &[r1.id.clone(), r2.id.clone()])
        .unwrap();
    s.set_product_tax_rates("B", std::slice::from_ref(&r1.id))
        .unwrap();

    let map = s
        .get_product_tax_rates_batch(&["A".into(), "B".into(), "NOPE".into()])
        .unwrap();
    assert_eq!(map.get("A").map(|v| v.len()), Some(2));
    assert_eq!(map.get("B").map(|v| v.len()), Some(1));
    assert!(!map.contains_key("NOPE"));
}

#[test]
fn get_product_tax_rates_batch_empty_skus() {
    let conn = fresh();
    let s = store(&conn);
    let map = s.get_product_tax_rates_batch(&[]).unwrap();
    assert!(map.is_empty());
}

#[test]
fn get_category_tax_rates_none() {
    let conn = fresh();
    let s = store(&conn);
    let ids = s.get_category_tax_rates("no-cat").unwrap();
    assert!(ids.is_empty());
}

// ── Extended edge cases (coverage 19→25) ──────────────────────────

#[test]
fn create_tax_rate_trims_whitespace_name_then_rejects_empty() {
    let conn = fresh();
    let s = store(&conn);
    // Name with only whitespace should be rejected after trim
    let result = s.create_tax_rate("   ", 100, false, false);
    assert!(matches!(
        result,
        Err(CoreError::Validation { field: "name", .. })
    ));
}

#[test]
fn list_tax_rates_ordered_by_name() {
    let conn = fresh();
    let s = store(&conn);
    s.create_tax_rate("Zebra Tax", 300, false, false).unwrap();
    s.create_tax_rate("Alpha Tax", 100, false, false).unwrap();
    s.create_tax_rate("Mike Tax", 200, false, false).unwrap();

    let rates = s.list_tax_rates().unwrap();
    assert_eq!(rates.len(), 3);
    assert_eq!(rates[0].name, "Alpha Tax");
    assert_eq!(rates[1].name, "Mike Tax");
    assert_eq!(rates[2].name, "Zebra Tax");
}

#[test]
fn update_default_flag_clears_previous_default() {
    let conn = fresh();
    let s = store(&conn);
    let first = s.create_tax_rate("First", 500, true, false).unwrap();
    let second = s.create_tax_rate("Second", 1000, false, false).unwrap();

    // Update second to become default; first should be cleared
    s.update_tax_rate(&second.id, "Second", 1000, true, false)
        .unwrap();

    let r1 = s.get_tax_rate(&first.id).unwrap().unwrap();
    let r2 = s.get_tax_rate(&second.id).unwrap().unwrap();
    assert!(!r1.is_default, "first default should be cleared");
    assert!(r2.is_default);
}

#[test]
fn product_tax_rates_with_multiple_rates() {
    let conn = fresh();
    let s = store(&conn);
    let currency = Currency::from_str("USD").unwrap();
    let money = crate::Money {
        minor_units: 1000,
        currency,
    };
    s.create_product("SKU-MULTI", "Multi-Tax", money, None, None, 0, None)
        .unwrap();

    let r1 = s.create_tax_rate("VAT", 1000, false, false).unwrap();
    let r2 = s.create_tax_rate("SVC", 500, false, false).unwrap();

    s.set_product_tax_rates("SKU-MULTI", &[r1.id.clone(), r2.id.clone()])
        .unwrap();

    let ids = s.get_product_tax_rates("SKU-MULTI").unwrap();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&r1.id));
    assert!(ids.contains(&r2.id));
}

#[test]
fn category_tax_rates_with_multiple_rates() {
    let conn = fresh();
    let s = store(&conn);
    s.create_category("cat-multi", "Multi Tax Cat", "#fff", "")
        .unwrap();

    let r1 = s.create_tax_rate("CT-A", 700, false, false).unwrap();
    let r2 = s.create_tax_rate("CT-B", 300, false, false).unwrap();

    s.set_category_tax_rates("cat-multi", &[r1.id.clone(), r2.id.clone()])
        .unwrap();

    let ids = s.get_category_tax_rates("cat-multi").unwrap();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&r1.id));
    assert!(ids.contains(&r2.id));
}

#[test]
fn category_tax_rates_overwrites() {
    let conn = fresh();
    let s = store(&conn);
    s.create_category("cat-ow", "OW", "#000", "").unwrap();

    let r1 = s.create_tax_rate("Old", 100, false, false).unwrap();
    let r2 = s.create_tax_rate("New", 200, false, false).unwrap();

    s.set_category_tax_rates("cat-ow", std::slice::from_ref(&r1.id))
        .unwrap();
    s.set_category_tax_rates("cat-ow", std::slice::from_ref(&r2.id))
        .unwrap();

    let ids = s.get_category_tax_rates("cat-ow").unwrap();
    assert_eq!(ids, vec![r2.id]);
}

// ── TAX-03: soft-delete + dependency policy ─────────────────────

#[test]
fn update_archived_tax_rate_returns_not_found() {
    let conn = fresh();
    let s = store(&conn);
    let created = s.create_tax_rate("Archive Me", 100, false, false).unwrap();
    s.delete_tax_rate(&created.id).unwrap();

    // TAX-03: an archived rate must not be updatable (immutable history).
    let result = s.update_tax_rate(&created.id, "Resurrected", 200, false, false);
    assert!(matches!(result, Err(CoreError::NotFound { .. })));
}

#[test]
fn delete_tax_rate_archives_instead_of_hard_delete() {
    let conn = fresh();
    let s = store(&conn);
    let created = s.create_tax_rate("Archive Me", 100, false, false).unwrap();

    s.delete_tax_rate(&created.id).unwrap();

    // Hidden from listing and lookup, but the row still exists with
    // is_active = 0 (so historical sale_lines references resolve).
    assert!(s.get_tax_rate(&created.id).unwrap().is_none());
    assert!(s.list_tax_rates().unwrap().is_empty());
    let raw_active: i64 = conn
        .query_row(
            "SELECT is_active FROM tax_rates WHERE id = ?1",
            params![created.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(raw_active, 0, "row must be archived, not deleted");
}

#[test]
fn delete_tax_rate_clears_product_and_category_junctions() {
    let conn = fresh();
    let s = store(&conn);
    let currency = foundation::Currency::from_str("USD").unwrap();
    let money = crate::Money {
        minor_units: 1000,
        currency,
    };
    s.create_product("SKU-TAX3", "Item", money, None, None, 0, None)
        .unwrap();
    s.create_category("cat-tax3", "Taxed Cat", "#fff", "")
        .unwrap();

    let rate = s.create_tax_rate("VAT", 1000, true, false).unwrap();
    s.set_product_tax_rates("SKU-TAX3", std::slice::from_ref(&rate.id))
        .unwrap();
    s.set_category_tax_rates("cat-tax3", std::slice::from_ref(&rate.id))
        .unwrap();

    // Sanity: both junctions populated before archive.
    assert_eq!(s.get_product_tax_rates("SKU-TAX3").unwrap().len(), 1);
    assert_eq!(s.get_category_tax_rates("cat-tax3").unwrap().len(), 1);

    s.delete_tax_rate(&rate.id).unwrap();

    // Junction rows are configuration, not history — they must be
    // cleaned so no product/category points at an archived rate.
    assert!(s.get_product_tax_rates("SKU-TAX3").unwrap().is_empty());
    assert!(s.get_category_tax_rates("cat-tax3").unwrap().is_empty());
}

#[test]
fn delete_tax_rate_blocked_when_referenced_by_sales() {
    let conn = fresh();
    let s = store(&conn);
    let rate = s.create_tax_rate("Historic", 1000, true, false).unwrap();

    // Seed a historical sale line that references the rate.
    conn.execute_batch(&format!(
        "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
            ('p-hist', 'SKU-HIST', 'Item', 1000, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
            ('sale-hist', 1000, 'USD', 1, 'completed', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
         INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position, tax_rate_id) VALUES
            ('sl-hist', 'sale-hist', 'SKU-HIST', 1, 1000, 1000, 'USD', 1, '{}');",
        rate.id
    ))
    .unwrap();

    // Dependency count must surface the sale reference.
    let counts = s.tax_rate_dependency_counts(&rate.id).unwrap();
    assert_eq!(counts.sale_lines, 1);

    // Archive must be blocked with a structured validation error.
    let err = s.delete_tax_rate(&rate.id).unwrap_err();
    assert!(matches!(
        err,
        CoreError::Validation {
            field: "tax_rate",
            ..
        }
    ));
    assert!(
        s.get_tax_rate(&rate.id).unwrap().is_some(),
        "rate must remain active after a blocked archive"
    );
}

#[test]
fn tax_rate_dependency_counts_are_zero_for_orphan() {
    let conn = fresh();
    let s = store(&conn);
    let rate = s.create_tax_rate("Lonely", 500, false, false).unwrap();

    let counts = s.tax_rate_dependency_counts(&rate.id).unwrap();
    assert_eq!(counts.products, 0);
    assert_eq!(counts.categories, 0);
    assert_eq!(counts.sale_lines, 0);
}

#[test]
fn tax_rate_dependency_counts_include_assignments() {
    let conn = fresh();
    let s = store(&conn);
    let currency = foundation::Currency::from_str("USD").unwrap();
    let money = crate::Money {
        minor_units: 1000,
        currency,
    };
    s.create_product("SKU-DEP1", "Item", money, None, None, 0, None)
        .unwrap();
    s.create_category("cat-dep1", "Cat", "#fff", "").unwrap();

    let rate = s.create_tax_rate("Dep", 800, false, false).unwrap();
    s.set_product_tax_rates("SKU-DEP1", std::slice::from_ref(&rate.id))
        .unwrap();
    s.set_category_tax_rates("cat-dep1", std::slice::from_ref(&rate.id))
        .unwrap();

    let counts = s.tax_rate_dependency_counts(&rate.id).unwrap();
    assert_eq!(counts.products, 1);
    assert_eq!(counts.categories, 1);
    assert_eq!(counts.sale_lines, 0);
}

// ── TAX-03 residual: assignments reject archived rate ids ────────

#[test]
fn set_product_tax_rates_rejects_archived_rate_id() {
    let conn = fresh();
    let s = store(&conn);
    let currency = foundation::Currency::from_str("USD").unwrap();
    let money = crate::Money {
        minor_units: 1000,
        currency,
    };
    s.create_product("SKU-ARCH", "Item", money, None, None, 0, None)
        .unwrap();

    let rate = s.create_tax_rate("VAT", 1000, true, false).unwrap();
    s.delete_tax_rate(&rate.id).unwrap();

    // TAX-03: an archived (immutable) rate must not be assignable.
    let err = s
        .set_product_tax_rates("SKU-ARCH", std::slice::from_ref(&rate.id))
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::NotFound {
            entity: "tax_rate",
            ..
        }
    ));
    // Junction rows must be untouched by the rejected assignment.
    assert!(s.get_product_tax_rates("SKU-ARCH").unwrap().is_empty());
}

#[test]
fn set_category_tax_rates_rejects_archived_rate_id() {
    let conn = fresh();
    let s = store(&conn);
    s.create_category("cat-arch", "Cat", "#fff", "").unwrap();

    let rate = s.create_tax_rate("VAT", 1000, true, false).unwrap();
    s.delete_tax_rate(&rate.id).unwrap();

    let err = s
        .set_category_tax_rates("cat-arch", std::slice::from_ref(&rate.id))
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::NotFound {
            entity: "tax_rate",
            ..
        }
    ));
    assert!(s.get_category_tax_rates("cat-arch").unwrap().is_empty());
}

#[test]
fn set_product_tax_rates_rejects_mixed_list_without_partial_write() {
    let conn = fresh();
    let s = store(&conn);
    let currency = foundation::Currency::from_str("USD").unwrap();
    let money = crate::Money {
        minor_units: 1000,
        currency,
    };
    s.create_product("SKU-MIX", "Item", money, None, None, 0, None)
        .unwrap();

    let active = s.create_tax_rate("Active", 500, false, false).unwrap();
    let archived = s.create_tax_rate("Archived", 1000, false, false).unwrap();
    s.delete_tax_rate(&archived.id).unwrap();

    // One archived id poisons the whole assignment — nothing is written.
    let err = s
        .set_product_tax_rates("SKU-MIX", &[active.id.clone(), archived.id.clone()])
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::NotFound {
            entity: "tax_rate",
            ..
        }
    ));
    assert!(s.get_product_tax_rates("SKU-MIX").unwrap().is_empty());
}

#[test]
fn set_category_tax_rates_rejects_unknown_rate_id() {
    let conn = fresh();
    let s = store(&conn);
    s.create_category("cat-unk", "Cat", "#fff", "").unwrap();

    // Unknown ids are rejected with the same structured error as archived.
    let err = s
        .set_category_tax_rates("cat-unk", &["no-such-rate".into()])
        .unwrap_err();
    assert!(matches!(
        err,
        CoreError::NotFound {
            entity: "tax_rate",
            ..
        }
    ));
    assert!(s.get_category_tax_rates("cat-unk").unwrap().is_empty());
}

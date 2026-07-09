//! Menu Engineering Analytics — volume, contribution margin, and quadrant
//! classification for restaurant menu items.
//!
//! This module provides SQL-backed aggregation queries that calculate:
//!
//! - **Total volume** — `SUM(sl.qty)` per product over a date range
//! - **Contribution margin** — `SUM((sl.unit_minor - p.cost_minor) * sl.qty)`
//!   per product over a date range
//!
//! The menu engineering matrix classifies each product into one of four
//! quadrants based on median volume and median margin:
//!
//! | Quadrant | Volume | Margin |
//! |---|---|---|
//! | **Star** | ≥ median | ≥ median |
//! | **Plowhorse** | ≥ median | < median |
//! | **Puzzle** | < median | ≥ median |
//! | **Dog** | < median | < median |

use rusqlite::params;
use serde::{Deserialize, Serialize};

use oz_core::CoreError;

/// Aggregated menu engineering row for a single product.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuEngineeringRow {
    /// Product unique identifier.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// Product display name.
    pub name: String,
    /// Total quantity sold over the selected period.
    pub total_volume: i64,
    /// Unit price in minor units (latest sale price or product price).
    pub unit_price_minor: i64,
    /// Cost per unit in minor units.
    pub unit_cost_minor: i64,
    /// Contribution margin per unit: unit_price - unit_cost.
    pub margin_per_unit: i64,
    /// Total contribution margin: (unit_price - unit_cost) * volume.
    pub total_margin_minor: i64,
    /// Total revenue: unit_price * volume.
    pub total_revenue_minor: i64,
}

/// Menu engineering classification quadrant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuQuadrant {
    /// High volume, high margin.
    Star,
    /// High volume, low margin.
    Plowhorse,
    /// Low volume, high margin.
    Puzzle,
    /// Low volume, low margin.
    Dog,
}

/// The full menu engineering result: per-product rows plus quadrant
/// classifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuEngineeringResult {
    /// Aggregated rows sorted by total revenue descending.
    pub rows: Vec<MenuEngineeringRow>,
    /// Median total volume across all products in the period.
    pub median_volume: f64,
    /// Median total margin across all products in the period.
    pub median_margin: f64,
}

/// Run the menu engineering aggregation query for a date range.
///
/// Returns per-product totals for volume, revenue, and contribution margin,
/// along with the median values used for quadrant classification.
pub fn query_menu_engineering(
    conn: &rusqlite::Connection,
    start_date: &str,
    end_date: &str,
) -> Result<MenuEngineeringResult, CoreError> {
    let mut stmt = conn.prepare(
        "SELECT p.id AS product_id, p.sku, p.name,
                COALESCE(SUM(sl.qty), 0) AS total_volume,
                sl.unit_minor AS unit_price_minor,
                COALESCE(p.cost_minor, 0) AS unit_cost_minor,
                (sl.unit_minor - COALESCE(p.cost_minor, 0)) AS margin_per_unit,
                SUM((sl.unit_minor - COALESCE(p.cost_minor, 0)) * sl.qty) AS total_margin_minor,
                SUM(sl.line_minor) AS total_revenue_minor
         FROM sale_lines sl
         JOIN sales s ON sl.sale_id = s.id
         JOIN products p ON sl.sku = p.sku
         WHERE s.status = 'completed'
           AND DATE(s.created_at) BETWEEN ?1 AND ?2
         GROUP BY p.id, sl.unit_minor
         ORDER BY total_revenue_minor DESC",
    )?;

    let mut rows: Vec<MenuEngineeringRow> = stmt
        .query_map(params![start_date, end_date], |row| {
            Ok(MenuEngineeringRow {
                product_id: row.get("product_id")?,
                sku: row.get("sku")?,
                name: row.get("name")?,
                total_volume: row.get("total_volume")?,
                unit_price_minor: row.get("unit_price_minor")?,
                unit_cost_minor: row.get("unit_cost_minor")?,
                margin_per_unit: row.get("margin_per_unit")?,
                total_margin_minor: row.get("total_margin_minor")?,
                total_revenue_minor: row.get("total_revenue_minor")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // Merge rows for the same product (same product sold at different prices).
    rows = merge_same_product_rows(rows);

    // Sort by total revenue descending.
    rows.sort_by_key(|b| std::cmp::Reverse(b.total_revenue_minor));

    // Calculate medians.
    let median_volume = median_of(&rows, |r| r.total_volume as f64);
    let median_margin = median_of(&rows, |r| r.total_margin_minor as f64);

    Ok(MenuEngineeringResult {
        rows,
        median_volume,
        median_margin,
    })
}

/// Merge rows that belong to the same product (different sale prices).
fn merge_same_product_rows(rows: Vec<MenuEngineeringRow>) -> Vec<MenuEngineeringRow> {
    let mut merged: std::collections::HashMap<String, MenuEngineeringRow> =
        std::collections::HashMap::new();

    for row in rows {
        use std::collections::hash_map::Entry;
        match merged.entry(row.sku.clone()) {
            Entry::Occupied(mut existing) => {
                let existing = existing.get_mut();
                existing.total_volume += row.total_volume;
                existing.total_margin_minor += row.total_margin_minor;
                existing.total_revenue_minor += row.total_revenue_minor;
                // Keep the first unit price/cost (most common / representative).
            }
            Entry::Vacant(entry) => {
                entry.insert(row);
            }
        }
    }

    let mut result: Vec<MenuEngineeringRow> = merged.into_values().collect();
    result.sort_by_key(|b| std::cmp::Reverse(b.total_revenue_minor));
    result
}

/// Compute the median of a numeric field extracted from a slice of rows.
fn median_of<T>(items: &[T], extract: impl Fn(&T) -> f64) -> f64 {
    if items.is_empty() {
        return 0.0;
    }

    let mut values: Vec<f64> = items.iter().map(extract).collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = values.len();
    if len.is_multiple_of(2) {
        (values[len / 2 - 1] + values[len / 2]) / 2.0
    } else {
        values[len / 2]
    }
}

/// Classify a product into a menu engineering quadrant based on median
/// thresholds.
pub fn classify_quadrant(
    volume: i64,
    margin_minor: i64,
    median_volume: f64,
    median_margin: f64,
) -> MenuQuadrant {
    let volume_high = (volume as f64) >= median_volume;
    let margin_high = (margin_minor as f64) >= median_margin;

    match (volume_high, margin_high) {
        (true, true) => MenuQuadrant::Star,
        (true, false) => MenuQuadrant::Plowhorse,
        (false, true) => MenuQuadrant::Puzzle,
        (false, false) => MenuQuadrant::Dog,
    }
}

/// Generate a human-readable recommendation for a menu quadrant.
pub fn quadrant_recommendation(quadrant: MenuQuadrant) -> &'static str {
    match quadrant {
        MenuQuadrant::Star => "Promote Star — high volume & high margin. Feature prominently.",
        MenuQuadrant::Plowhorse => {
            "Increase Price on Plowhorse — high volume but low margin. Raise price or reduce cost."
        }
        MenuQuadrant::Puzzle => {
            "Reposition Puzzle — low volume but high margin. Improve visibility or bundle."
        }
        MenuQuadrant::Dog => "Remove Dog — low volume & low margin. Consider delisting.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use oz_core::{Cart, CartLine, Currency, Money, Sale, SaleStatus, Sku};
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
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

    fn seed_product(conn: &Connection, sku: &str, price_minor: i64, cost_minor: i64) -> String {
        let store = oz_core::db::Store::new(conn);
        let money = Money {
            minor_units: price_minor,
            currency: usd(),
        };
        store
            .create_product(sku, sku, money, None, None, 100, None)
            .unwrap();

        // Set cost_minor after creation.
        conn.execute(
            "UPDATE products SET cost_minor = ?1 WHERE sku = ?2",
            params![cost_minor, sku],
        )
        .unwrap();

        // Return product id.
        conn.query_row(
            "SELECT id FROM products WHERE sku = ?1",
            params![sku],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn complete_sale(conn: &Connection, sku: &str, qty: i64, unit_minor: i64) -> String {
        let store = oz_core::db::Store::new(conn);
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new(sku), qty, price(unit_minor)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        sale.created_at = now.clone();
        sale.updated_at = now;
        store.create_sale(&sale).unwrap();
        store
            .update_sale_status(&sale.id, SaleStatus::Active)
            .unwrap();
        store
            .update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();
        sale.id
    }

    // ── Query tests ──────────────────────────────────────────────

    #[test]
    fn menu_engineering_empty_range() {
        let conn = fresh();
        let result = query_menu_engineering(&conn, "2000-01-01", "2099-12-31").unwrap();
        assert!(result.rows.is_empty());
        assert_eq!(result.median_volume, 0.0);
        assert_eq!(result.median_margin, 0.0);
    }

    #[test]
    fn menu_engineering_single_product() {
        let conn = fresh();
        seed_product(&conn, "STEAK", 2500, 800);
        complete_sale(&conn, "STEAK", 2, 2500);

        let result = query_menu_engineering(&conn, "2000-01-01", "2099-12-31").unwrap();

        assert_eq!(result.rows.len(), 1);
        let row = &result.rows[0];
        assert_eq!(row.sku, "STEAK");
        assert_eq!(row.total_volume, 2);
        assert_eq!(row.unit_cost_minor, 800);
        assert_eq!(row.margin_per_unit, 1700);
        assert_eq!(row.total_margin_minor, 3400);
        assert_eq!(row.total_revenue_minor, 5000);

        // Median should match the single product.
        assert!((result.median_volume - 2.0).abs() < f64::EPSILON);
        assert!((result.median_margin - 3400.0).abs() < f64::EPSILON);
    }

    #[test]
    fn menu_engineering_multiple_products() {
        let conn = fresh();
        seed_product(&conn, "STEAK", 2500, 800);
        seed_product(&conn, "SALAD", 1200, 400);
        seed_product(&conn, "SODA", 300, 100);
        complete_sale(&conn, "STEAK", 2, 2500);
        complete_sale(&conn, "SALAD", 3, 1200);
        complete_sale(&conn, "SODA", 5, 300);

        let result = query_menu_engineering(&conn, "2000-01-01", "2099-12-31").unwrap();

        assert_eq!(result.rows.len(), 3);

        // STEAK: volume=2, margin=(2500-800)*2=3400, revenue=5000
        // SALAD: volume=3, margin=(1200-400)*3=2400, revenue=3600
        // SODA:  volume=5, margin=(300-100)*5=1000, revenue=1500

        let steak = result.rows.iter().find(|r| r.sku == "STEAK").unwrap();
        assert_eq!(steak.total_margin_minor, 3400);

        let salad = result.rows.iter().find(|r| r.sku == "SALAD").unwrap();
        assert_eq!(salad.total_margin_minor, 2400);

        let soda = result.rows.iter().find(|r| r.sku == "SODA").unwrap();
        assert_eq!(soda.total_margin_minor, 1000);
    }

    #[test]
    fn menu_engineering_zero_cost() {
        let conn = fresh();
        seed_product(&conn, "FREE", 500, 0); // cost = 0
        complete_sale(&conn, "FREE", 1, 500);

        let result = query_menu_engineering(&conn, "2000-01-01", "2099-12-31").unwrap();

        let row = &result.rows[0];
        assert_eq!(row.unit_cost_minor, 0);
        assert_eq!(row.margin_per_unit, 500);
        assert_eq!(row.total_margin_minor, 500);
    }

    // ── Quadrant classification ──────────────────────────────────

    #[test]
    fn classify_star() {
        assert_eq!(
            classify_quadrant(100, 5000, 50.0, 2500.0),
            MenuQuadrant::Star
        );
    }

    #[test]
    fn classify_plowhorse() {
        assert_eq!(
            classify_quadrant(100, 1000, 50.0, 2500.0),
            MenuQuadrant::Plowhorse
        );
    }

    #[test]
    fn classify_puzzle() {
        assert_eq!(
            classify_quadrant(10, 5000, 50.0, 2500.0),
            MenuQuadrant::Puzzle
        );
    }

    #[test]
    fn classify_dog() {
        assert_eq!(classify_quadrant(10, 1000, 50.0, 2500.0), MenuQuadrant::Dog);
    }

    #[test]
    fn classify_boundary_equal_median() {
        assert_eq!(
            classify_quadrant(50, 2500, 50.0, 2500.0),
            MenuQuadrant::Star
        );
    }

    #[test]
    fn classify_zero_values() {
        assert_eq!(classify_quadrant(0, 0, 0.0, 0.0), MenuQuadrant::Star);
    }

    // ── Recommendations ──────────────────────────────────────────

    #[test]
    fn star_recommendation() {
        let rec = quadrant_recommendation(MenuQuadrant::Star);
        assert!(rec.contains("Promote Star"));
    }

    #[test]
    fn plowhorse_recommendation() {
        let rec = quadrant_recommendation(MenuQuadrant::Plowhorse);
        assert!(rec.contains("Increase Price"));
    }

    #[test]
    fn puzzle_recommendation() {
        let rec = quadrant_recommendation(MenuQuadrant::Puzzle);
        assert!(rec.contains("Reposition Puzzle"));
    }

    #[test]
    fn dog_recommendation() {
        let rec = quadrant_recommendation(MenuQuadrant::Dog);
        assert!(rec.contains("Remove Dog"));
    }

    // ── Median calculation ───────────────────────────────────────

    #[test]
    fn median_empty() {
        let items: Vec<i64> = vec![];
        let result = median_of(&items, |&x| x as f64);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn median_odd_count() {
        let items = vec![10, 20, 30];
        let result = median_of(&items, |&x| x as f64);
        assert!((result - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn median_even_count() {
        let items = vec![10, 20, 30, 40];
        let result = median_of(&items, |&x| x as f64);
        assert!((result - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn median_single_element() {
        let items = vec![42];
        let result = median_of(&items, |&x| x as f64);
        assert!((result - 42.0).abs() < f64::EPSILON);
    }

    // ── Serde ────────────────────────────────────────────────────

    #[test]
    fn menu_quadrant_serde_roundtrip() {
        for q in &[
            MenuQuadrant::Star,
            MenuQuadrant::Plowhorse,
            MenuQuadrant::Puzzle,
            MenuQuadrant::Dog,
        ] {
            let json = serde_json::to_string(q).unwrap();
            let back: MenuQuadrant = serde_json::from_str(&json).unwrap();
            assert_eq!(*q, back);
        }
    }

    #[test]
    fn menu_engineering_row_serde_roundtrip() {
        let row = MenuEngineeringRow {
            product_id: "p-1".into(),
            sku: "COFFEE".into(),
            name: "Coffee".into(),
            total_volume: 100,
            unit_price_minor: 350,
            unit_cost_minor: 100,
            margin_per_unit: 250,
            total_margin_minor: 25000,
            total_revenue_minor: 35000,
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: MenuEngineeringRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sku, "COFFEE");
        assert_eq!(back.total_margin_minor, 25000);
    }
}

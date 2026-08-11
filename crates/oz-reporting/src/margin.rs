//! Per-sale-line margin reporting (HPP / cost exposure).
//!
//! Retail merchandising records the product cost (`products.cost_minor`, the
//! "HPP" — *harga pokok penjualan* / cost of goods) as a local attribute.
//! This module exposes it in the reporting layer: for a given sale, each line
//! is enriched with the unit cost, the line margin, and the margin
//! percentage, so a manager can review a transaction's profitability.
//!
//! # Cost semantics
//!
//! Costs are **not snapshotted into `sale_lines`** — they are read from the
//! product's current `cost_minor` at query time (matching
//! [`crate::menu_engineering`]). A cost correction after a sale therefore
//! restates that sale's historical margin; this is the documented behavior
//! of the existing reporting layer and keeps the ledger unchanged. Lines
//! whose product is missing (deleted/unknown SKU) fall back to a zero cost
//! and the SKU as the display name.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use oz_core::CoreError;

/// One enriched sale line with cost and margin figures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleLineMargin {
    /// `sale_lines.id` — join key back to the sale detail.
    pub sale_line_id: String,
    /// Product SKU at time of sale.
    pub sku: String,
    /// Product display name (falls back to the SKU when unknown).
    pub name: String,
    /// Quantity sold (≥ 1).
    pub qty: i64,
    /// Unit price in minor units at time of sale.
    pub unit_price_minor: i64,
    /// Line total in minor units (`unit_price × qty`, discounts are
    /// sale-level and do not change per-line totals).
    pub line_total_minor: i64,
    /// Current unit cost in minor units (HPP); 0 when unset or unknown.
    pub unit_cost_minor: i64,
    /// Line margin in minor units: `(unit_price − cost) × qty`. Negative
    /// when the unit sells below cost.
    pub margin_minor: i64,
    /// Margin as a percentage of the line total (`margin / total × 100`).
    /// 0.0 when the line total is zero. Negative for loss-leader lines.
    pub margin_percent: f64,
}

/// Compute the margin percentage of a line.
///
/// `margin / line_total × 100`, guarded against a zero line total (fully
/// discounted lines report a 0% margin rather than ±∞).
pub fn margin_percent(margin_minor: i64, line_total_minor: i64) -> f64 {
    if line_total_minor == 0 {
        0.0
    } else {
        (margin_minor as f64) / (line_total_minor as f64) * 100.0
    }
}

/// Enrich every line of one sale with its cost and margin.
///
/// Returns the lines in `line_position` order. The sale's status is not
/// filtered here — the caller decides which sale to open; only its lines are
/// reported.
pub fn query_sale_lines_with_margin(
    conn: &rusqlite::Connection,
    sale_id: &str,
) -> Result<Vec<SaleLineMargin>, CoreError> {
    let mut stmt = conn.prepare(
        "SELECT sl.id, sl.sku,
                COALESCE(p.name, sl.sku) AS name,
                sl.qty,
                sl.unit_minor,
                sl.line_minor,
                COALESCE(p.cost_minor, 0) AS unit_cost_minor,
                (sl.unit_minor - COALESCE(p.cost_minor, 0)) * sl.qty AS margin_minor
         FROM sale_lines sl
         LEFT JOIN products p ON sl.sku = p.sku
         WHERE sl.sale_id = ?1
         ORDER BY sl.line_position",
    )?;

    let rows = stmt.query_map(params![sale_id], |row| {
        let unit_price_minor: i64 = row.get("unit_minor")?;
        let line_total_minor: i64 = row.get("line_minor")?;
        let unit_cost_minor: i64 = row.get("unit_cost_minor")?;
        let qty: i64 = row.get("qty")?;
        let margin_minor: i64 = row.get("margin_minor")?;
        Ok(SaleLineMargin {
            sale_line_id: row.get("id")?,
            sku: row.get("sku")?,
            name: row.get("name")?,
            qty,
            unit_price_minor,
            line_total_minor,
            unit_cost_minor,
            margin_minor,
            margin_percent: margin_percent(margin_minor, line_total_minor),
        })
    })?;

    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use oz_core::{Cart, CartLine, Currency, Money, Sale, SaleStatus, Sku};
    use rusqlite::{Connection, params};

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    fn seed_product(conn: &Connection, sku: &str, price_minor: i64, cost_minor: i64) {
        let store = oz_core::db::Store::new(conn);
        store
            .create_product(
                sku,
                sku,
                Money {
                    minor_units: price_minor,
                    currency: usd(),
                },
                None,
                None,
                100,
                None,
            )
            .unwrap();
        conn.execute(
            "UPDATE products SET cost_minor = ?1 WHERE sku = ?2",
            params![cost_minor, sku],
        )
        .unwrap();
    }

    fn complete_sale(conn: &Connection, lines: &[(&str, i64, i64)]) -> String {
        let store = oz_core::db::Store::new(conn);
        let mut cart = Cart::new(usd());
        for (sku, qty, unit_minor) in lines {
            cart.add_line(CartLine::new(
                Sku::new(*sku),
                *qty,
                Money {
                    minor_units: *unit_minor,
                    currency: usd(),
                },
            ))
            .unwrap();
        }
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

    // ── margin_percent ────────────────────────────────────────────

    #[test]
    fn margin_percent_positive() {
        assert!((margin_percent(1700, 2500) - 68.0).abs() < 1e-9);
    }

    #[test]
    fn margin_percent_negative_for_loss_leader() {
        assert!((margin_percent(-900, 1500) - (-60.0)).abs() < 1e-9);
    }

    #[test]
    fn margin_percent_zero_total_is_zero() {
        assert_eq!(margin_percent(0, 0), 0.0);
        // Even a negative margin on a zero-total line (fully discounted)
        // reports 0% — no division by zero, no ±∞.
        assert_eq!(margin_percent(-5, 0), 0.0);
    }

    // ── query_sale_lines_with_margin ──────────────────────────────

    #[test]
    fn margin_query_empty_sale() {
        let conn = fresh();
        let rows = query_sale_lines_with_margin(&conn, "no-such-sale").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn margin_query_enriches_lines_with_cost() {
        let conn = fresh();
        seed_product(&conn, "STEAK", 2500, 800);
        seed_product(&conn, "SODA", 300, 100);
        let sale_id = complete_sale(&conn, &[("STEAK", 2, 2500), ("SODA", 3, 300)]);

        let rows = query_sale_lines_with_margin(&conn, &sale_id).unwrap();
        assert_eq!(rows.len(), 2);

        let steak = &rows[0];
        assert_eq!(steak.sku, "STEAK");
        assert_eq!(steak.name, "STEAK");
        assert_eq!(steak.qty, 2);
        assert_eq!(steak.unit_price_minor, 2500);
        assert_eq!(steak.line_total_minor, 5000);
        assert_eq!(steak.unit_cost_minor, 800);
        assert_eq!(steak.margin_minor, 3400);
        assert!((steak.margin_percent - 68.0).abs() < 1e-9);

        let soda = &rows[1];
        assert_eq!(soda.line_total_minor, 900);
        assert_eq!(soda.margin_minor, 600);
        assert!((soda.margin_percent - 66.6666666667).abs() < 1e-6);
    }

    #[test]
    fn margin_query_negative_when_cost_above_price() {
        let conn = fresh();
        seed_product(&conn, "LOSS", 500, 800);
        let sale_id = complete_sale(&conn, &[("LOSS", 3, 500)]);

        let rows = query_sale_lines_with_margin(&conn, &sale_id).unwrap();
        assert_eq!(rows[0].margin_minor, -900);
        assert!((rows[0].margin_percent - (-60.0)).abs() < 1e-9);
    }

    #[test]
    fn margin_query_zero_cost_falls_back() {
        let conn = fresh();
        // Product with no cost set: cost_minor stays NULL → 0.
        seed_product(&conn, "FREE", 500, 0);
        let sale_id = complete_sale(&conn, &[("FREE", 1, 500)]);

        let rows = query_sale_lines_with_margin(&conn, &sale_id).unwrap();
        assert_eq!(rows[0].unit_cost_minor, 0);
        assert_eq!(rows[0].margin_minor, 500);
        assert_eq!(rows[0].margin_percent, 100.0);
    }

    // ── Serde ─────────────────────────────────────────────────────

    #[test]
    fn sale_line_margin_serde_roundtrip() {
        let row = SaleLineMargin {
            sale_line_id: "sl-1".into(),
            sku: "COFFEE".into(),
            name: "Coffee".into(),
            qty: 2,
            unit_price_minor: 350,
            line_total_minor: 700,
            unit_cost_minor: 100,
            margin_minor: 500,
            margin_percent: 71.42857,
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: SaleLineMargin = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sku, "COFFEE");
        assert_eq!(back.margin_minor, 500);
        assert!((back.margin_percent - 71.42857).abs() < 1e-9);
    }
}

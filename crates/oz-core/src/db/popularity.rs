//! Popularity recompute methods (ADR #37 D3).
//!
//! The sales signal is read straight from `sale_lines` (already the durable
//! ledger); search and edit signals come from the `product_activity` ledger
//! (migration 133). Each contributing event triggers a single-SKU recompute;
//! a full-catalog pass recomputes every score and refreshes the catalog means
//! cached in `settings`. Scores and the ledger are local-only (ADR #37 D4).

use std::collections::HashMap;

use rusqlite::params;

use crate::error::CoreError;
use crate::popularity::{DayCount, compute_score, decayed_sum, total_events};

use super::Store;

/// Settings keys caching the catalog means computed by the last full pass.
const MEAN_SALES: &str = "popularity.mean.sales";
const MEAN_SEARCH: &str = "popularity.mean.search";
const MEAN_EDITS: &str = "popularity.mean.edits";

/// Days between an ISO date and today; out-of-range becomes `i64::MAX` so the
/// formula window filters it out.
fn days_ago(day: &str) -> i64 {
    match chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d") {
        Ok(d) => {
            let today = chrono::Utc::now().date_naive();
            (today - d).num_days()
        }
        Err(_) => i64::MAX,
    }
}

fn window_modifier() -> String {
    format!("-{} days", crate::popularity::WINDOW_DAYS)
}

impl Store<'_> {
    /// Daily completed-sale unit counts for one SKU over the window.
    fn sale_day_counts(&self, sku: &str) -> Result<Vec<DayCount>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT strftime('%Y-%m-%d', s.created_at) AS day, SUM(sl.qty) AS qty
             FROM sale_lines sl
             JOIN sales s ON sl.sale_id = s.id
             WHERE sl.sku = ?1 AND s.status = 'completed'
               AND s.created_at >= datetime('now', ?2)
             GROUP BY day",
        )?;
        let rows = stmt.query_map(params![sku, window_modifier()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (day, qty) = row?;
            out.push(DayCount {
                days_ago: days_ago(&day),
                count: qty,
            });
        }
        Ok(out)
    }

    /// Daily counts of one activity type for a SKU over the window.
    fn activity_day_counts(&self, sku: &str, event_type: &str) -> Result<Vec<DayCount>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT strftime('%Y-%m-%d', created_at) AS day, COUNT(*) AS cnt
             FROM product_activity
             WHERE sku = ?1 AND event_type = ?2
               AND created_at >= datetime('now', ?3)
             GROUP BY day",
        )?;
        let rows = stmt.query_map(params![sku, event_type, window_modifier()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (day, cnt) = row?;
            out.push(DayCount {
                days_ago: days_ago(&day),
                count: cnt,
            });
        }
        Ok(out)
    }

    /// Read a cached catalog mean from `settings` (0.0 when absent).
    fn read_mean(&self, key: &str) -> f64 {
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0)
    }

    /// Cache a catalog mean in `settings`.
    fn write_mean(&self, key: &str, value: f64) -> Result<(), CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        self.conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value.to_string(), now],
        )?;
        Ok(())
    }

    /// Record an acted-upon search (ADR #37 D2) and refresh the SKU's score.
    ///
    /// Fire-and-forget by callers: a dropped event costs one popularity tick.
    pub fn record_product_search(&self, sku: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO product_activity (id, sku, event_type) VALUES (?1, ?2, 'search')",
            params![crate::new_id(), sku],
        )?;
        self.recompute_popularity(sku)
    }

    /// Recompute the materialized `popularity_score` for a single SKU.
    ///
    /// Uses the catalog means cached by the last full pass; absent means
    /// (fresh DB) are 0.0, which shrinks low-evidence scores toward zero until
    /// [`Store::recompute_all_popularity`] runs.
    pub fn recompute_popularity(&self, sku: &str) -> Result<(), CoreError> {
        let sales = self.sale_day_counts(sku)?;
        let searches = self.activity_day_counts(sku, "search")?;
        let edits = self.activity_day_counts(sku, "edit")?;
        let score = compute_score(
            &sales,
            &searches,
            &edits,
            self.read_mean(MEAN_SALES),
            self.read_mean(MEAN_SEARCH),
            self.read_mean(MEAN_EDITS),
        );
        self.conn.execute(
            "UPDATE products SET popularity_score = ?1 WHERE sku = ?2",
            params![score, sku],
        )?;
        Ok(())
    }

    /// Full-catalog popularity pass: recompute every product's score and
    /// refresh the cached catalog means.
    ///
    /// Called at store open so the retail grid's default popularity sort is
    /// meaningful from day one (sales history backfills immediately; search
    /// and edit signals accumulate from zero).
    pub fn recompute_all_popularity(&self) -> Result<(), CoreError> {
        // ── Per-SKU signals in one pass each ──────────────────────────
        let mut sales: HashMap<String, Vec<DayCount>> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT sl.sku, strftime('%Y-%m-%d', s.created_at) AS day, SUM(sl.qty) AS qty
                 FROM sale_lines sl JOIN sales s ON sl.sale_id = s.id
                 WHERE s.status = 'completed' AND s.created_at >= datetime('now', ?1)
                 GROUP BY sl.sku, day",
            )?;
            let rows = stmt.query_map(params![window_modifier()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;
            for row in rows {
                let (sku, day, qty) = row?;
                sales.entry(sku).or_default().push(DayCount {
                    days_ago: days_ago(&day),
                    count: qty,
                });
            }
        }

        let mut searches: HashMap<String, Vec<DayCount>> = HashMap::new();
        let mut edits: HashMap<String, Vec<DayCount>> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT sku, event_type, strftime('%Y-%m-%d', created_at) AS day, COUNT(*) AS cnt
                 FROM product_activity
                 WHERE created_at >= datetime('now', ?1)
                 GROUP BY sku, event_type, day",
            )?;
            let rows = stmt.query_map(params![window_modifier()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })?;
            for row in rows {
                let (sku, etype, day, cnt) = row?;
                let dc = DayCount {
                    days_ago: days_ago(&day),
                    count: cnt,
                };
                if etype == "search" {
                    searches.entry(sku).or_default().push(dc);
                } else if etype == "edit" {
                    edits.entry(sku).or_default().push(dc);
                }
            }
        }

        // ── Per-product raw + votes, and catalog means ────────────────
        let mut products: Vec<(String, f64, f64, f64, f64, f64, f64)> = Vec::new();
        {
            let mut stmt = self.conn.prepare("SELECT sku FROM products")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for row in rows {
                let sku = row?;
                let s_events = sales.get(&sku).map(Vec::as_slice).unwrap_or(&[]);
                let q_events = searches.get(&sku).map(Vec::as_slice).unwrap_or(&[]);
                let e_events = edits.get(&sku).map(Vec::as_slice).unwrap_or(&[]);
                products.push((
                    sku,
                    decayed_sum(s_events),
                    total_events(s_events),
                    decayed_sum(q_events),
                    total_events(q_events),
                    decayed_sum(e_events),
                    total_events(e_events),
                ));
            }
        }

        let n = products.len() as f64;
        let mean_sales = if n > 0.0 {
            products.iter().map(|p| p.1).sum::<f64>() / n
        } else {
            0.0
        };
        let mean_search = if n > 0.0 {
            products.iter().map(|p| p.3).sum::<f64>() / n
        } else {
            0.0
        };
        let mean_edits = if n > 0.0 {
            products.iter().map(|p| p.5).sum::<f64>() / n
        } else {
            0.0
        };

        self.write_mean(MEAN_SALES, mean_sales)?;
        self.write_mean(MEAN_SEARCH, mean_search)?;
        self.write_mean(MEAN_EDITS, mean_edits)?;

        // ── Write scores (one transaction: the per-SKU updates are atomic) ─
        let tx = self.conn.unchecked_transaction()?;
        for (sku, sr, sv, qr, qv, er, ev) in products {
            let score = crate::popularity::score_from_raw(
                sr,
                sv,
                qr,
                qv,
                er,
                ev,
                mean_sales,
                mean_search,
                mean_edits,
            );
            tx.execute(
                "UPDATE products SET popularity_score = ?1 WHERE sku = ?2",
                params![score, sku],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn fresh() -> rusqlite::Connection {
        migrations::fresh_db()
    }

    fn seed_product(conn: &rusqlite::Connection, sku: &str) -> String {
        let id = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) \
             VALUES (?1, ?2, ?2, 1000, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            params![id, sku],
        )
        .unwrap();
        id
    }

    #[test]
    fn record_product_search_writes_ledger_and_recomputes() {
        let conn = fresh();
        seed_product(&conn, "SKU-A");
        let store = Store::new(&conn);

        store.record_product_search("SKU-A").unwrap();

        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM product_activity WHERE sku = 'SKU-A' AND event_type = 'search'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 1, "search event must write exactly one ledger row");

        let score: f64 = conn
            .query_row(
                "SELECT popularity_score FROM products WHERE sku = 'SKU-A'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // Search weight (0.3) × smoothed search raw — with zero means the
        // smoothed value is 0, so the score reflects only the search signal.
        assert!(score > 0.0, "score must move after a search event");
    }

    #[test]
    fn update_product_attributes_writes_edit_ledger_row() {
        let conn = fresh();
        seed_product(&conn, "SKU-B");
        let store = Store::new(&conn);

        store
            .update_product_attributes(
                "SKU-B",
                &crate::db::UpdateProductAttributes {
                    cost_minor: Some(500),
                    brand: Some(Some("Acme".into())),
                    rack_location: None,
                    notes: None,
                    unit: None,
                    is_active: None,
                    default_supplier_id: None,
                },
            )
            .unwrap();

        let edits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM product_activity WHERE sku = 'SKU-B' AND event_type = 'edit'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            edits, 1,
            "product update must write an edit signal (ADR #37 D2)"
        );

        let cost: i64 = conn
            .query_row(
                "SELECT cost_minor FROM products WHERE sku = 'SKU-B'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cost, 500);
    }

    #[test]
    fn full_pass_backfills_scores_from_sale_lines() {
        let conn = fresh();
        let pid = seed_product(&conn, "SKU-SOLD");
        // A completed sale with 4 units, plus a pending sale that must NOT
        // count (only completed sales feed the sales signal).
        // Use today's timestamp so the sales land inside the 90-day decay
        // window (a 2025 seed would be filtered out by the formula).
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute_batch(
            &format!(
                "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
                ('sale-1', 4000, 'USD', 1, 'completed', '{now}', '{now}'),
                ('sale-2', 3000, 'USD', 1, 'pending',   '{now}', '{now}');
                 INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
                ('sl-1', 'sale-1', 'SKU-SOLD', 4, 1000, 4000, 'USD', 1),
                ('sl-2', 'sale-2', 'SKU-SOLD', 3, 1000, 3000, 'USD', 1);"
            ),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty) VALUES (?1, ?2, 10)",
            params![pid, crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID],
        )
        .unwrap();

        let store = Store::new(&conn);
        store.recompute_all_popularity().unwrap();

        let score: f64 = conn
            .query_row(
                "SELECT popularity_score FROM products WHERE sku = 'SKU-SOLD'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            score > 0.0,
            "sold product must get a positive score after backfill"
        );

        let pending_influence: f64 = conn
            .query_row(
                "SELECT popularity_score FROM products WHERE sku = 'SKU-SOLD'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // Only completed sales feed the signal; the score must equal the
        // 4-unit backfill, not the 7-unit (pending-included) figure.
        let sales_raw = crate::popularity::decayed_sum(&[DayCount {
            days_ago: 0,
            count: 4,
        }]);
        // The pending sale's 3 units must NOT inflate the signal: the score
        // can at most reflect the 4 completed units (smoothing toward the
        // catalog mean can only shrink it, never grow it past the raw).
        let seven_units = crate::popularity::decayed_sum(&[DayCount {
            days_ago: 0,
            count: 7,
        }]);
        let score_raw_share = crate::popularity::WEIGHT_SALES * sales_raw;
        assert!(
            score_raw_share <= pending_influence + 1e-9
                && pending_influence < crate::popularity::WEIGHT_SALES * seven_units,
            "backfilled score must reflect completed sales only (pending excluded)"
        );
    }

    #[test]
    fn full_pass_ranks_backfilled_edit_events_by_recency() {
        // Day-one upgrade: migration 134 seeded one synthetic edit event per
        // recently-touched product at its last update. With zero sales and
        // zero searches the full pass must rank the most-recently managed
        // product first, a product last touched 80 days ago below the
        // catalog mean (stale), and an untouched product exactly at it.
        let conn = fresh();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let ts = |days_ago: i64| {
            chrono::Utc::now()
                .checked_sub_signed(chrono::Duration::days(days_ago))
                .unwrap()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        };
        conn.execute_batch(
            &format!(
                "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
                ('p-a', 'SKU-A', 'Managed today',    1000, 'USD', '{now}', '{now}'),
                ('p-b', 'SKU-B', 'Touched 80d ago',  1000, 'USD', '{}', '{}'),
                ('p-c', 'SKU-C', 'Never touched',    1000, 'USD', '{}', '{}');",
                ts(400),
                ts(80),
                ts(400),
                ts(400),
            ),
        )
        .unwrap();
        // The ledger rows migration 134 would have written.
        conn.execute_batch(&format!(
            "INSERT INTO product_activity (id, sku, event_type, created_at) VALUES
                ('backfill-edit-p-a', 'SKU-A', 'edit', '{now}'),
                ('backfill-edit-p-b', 'SKU-B', 'edit', '{}');",
            ts(80),
        ))
        .unwrap();

        let store = Store::new(&conn);
        store.recompute_all_popularity().unwrap();

        let score = |sku: &str| -> f64 {
            conn.query_row(
                "SELECT popularity_score FROM products WHERE sku = ?1",
                params![sku],
                |r| r.get(0),
            )
            .unwrap()
        };
        let (a, b, c) = (score("SKU-A"), score("SKU-B"), score("SKU-C"));
        assert!(
            a > c && c > b && a > 0.0,
            "day-one sort must rank recently-managed products first \
             (A={a}, C={c}, B={b})"
        );
    }
}

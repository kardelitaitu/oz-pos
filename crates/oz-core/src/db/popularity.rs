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
/// Settings key caching per-category means as JSON
/// (`{"<category_id>": {"sales": s, "search": q, "edits": e}, "": {…}}`,
/// where the `""` entry is the global fallback for uncategorized products).
/// Per-category popularity (ADR #37 D6): each product is smoothed toward its
/// own category's mean so a quiet category's products are not drowned by a
/// hot one — the retail grid's popularity sort becomes fair within a
/// selected category.
const CATEGORY_MEANS: &str = "popularity.category_means";

/// One product's full-pass raw signals: `(sku, category_id, sales_raw,
/// sales_votes, search_raw, search_votes, edits_raw, edits_votes)`.
type ProductSignals = (String, Option<String>, f64, f64, f64, f64, f64, f64);

/// One product inside a category's popularity leaderboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CategoryTopProduct {
    /// Product SKU.
    pub sku: String,
    /// Product display name.
    pub name: String,
    /// Materialized popularity score (category-smoothed, ADR #37 D6).
    pub popularity_score: f64,
    /// 1-based rank within the category by score (descending, SKU tiebreak).
    pub rank: i64,
    /// Category-relative standing: 1.0 = most popular in the category,
    /// 0.0 = least, evenly spaced; 1.0 for single-product categories.
    pub percentile: f64,
}

/// One (period, category) point of the popularity trend series.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CategoryTrendPoint {
    /// Period bucket start: `YYYY-MM-DD` (daily/weekly) or `YYYY-MM` (monthly).
    pub period_start: String,
    /// Category id; empty string for uncategorized products.
    pub category_id: String,
    /// Category name; `None` for uncategorized (the UI localizes the label).
    pub category_name: Option<String>,
    /// Period popularity score — the ADR #37 blend evaluated over the
    /// period's raw signals, smoothed toward the cached category means, so
    /// the series is on the same scale as the current `popularity_score`.
    pub score: f64,
    /// Units sold in the period (completed sales).
    pub units_sold: i64,
    /// Distinct transactions in the period (breadth input).
    pub distinct_transactions: i64,
    /// Acted-upon searches in the period.
    pub searches: i64,
    /// Edit events in the period.
    pub edits: i64,
}

/// Per-category popularity summary (ADR #37 — the per-category evolution:
/// the smoothing means are used for scoring, and this query surfaces the
/// resulting category standings for the reporting layer).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CategoryPopularityRow {
    /// Category id; empty string for uncategorized products.
    pub category_id: String,
    /// Category name; `None` for uncategorized (the UI localizes the label).
    pub category_name: Option<String>,
    /// Number of products in the category.
    pub product_count: i64,
    /// Mean popularity score across the category's products.
    pub mean_score: f64,
    /// `mean_score` relative to the catalog-wide mean (1.0 = average,
    /// 2.0 = twice as popular as the catalog average; 0.0 when no scores).
    pub catalog_ratio: f64,
    /// The category's most popular products, ranked by score.
    pub top_products: Vec<CategoryTopProduct>,
}

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

/// Accepted granularities for [`Store::category_popularity_trend`].
pub const TREND_GRANULARITIES: [&str; 3] = ["daily", "weekly", "monthly"];

impl Store<'_> {
    /// Per-period popularity trend for the top `top_categories` categories.
    ///
    /// Buckets the sale/search/edit ledgers by `granularity` (`daily`,
    /// `weekly`, `monthly`; the weekly bucket mirrors `weekly_revenue`'s
    /// `DATE(created_at, 'weekday 0', '-7 days')`) over `[start_date,
    /// end_date]` and evaluates the ADR #37 blend per (period, category)
    /// with the raw period counts smoothed toward the cached category
    /// means — the same scale as the materialized `popularity_score`, so a
    /// category's trend line reads directly against its current standing.
    pub fn category_popularity_trend(
        &self,
        start_date: &str,
        end_date: &str,
        granularity: &str,
        top_categories: i64,
    ) -> Result<Vec<CategoryTrendPoint>, CoreError> {
        // Qualified period expressions: the sales query joins `sales s` and
        // the activity query joins `products p` (which also has a
        // `created_at`), so the column must be explicit per query.
        let (s_period, a_period) = match granularity {
            "weekly" => (
                "DATE(s.created_at, 'weekday 0', '-7 days')",
                "DATE(a.created_at, 'weekday 0', '-7 days')",
            ),
            "monthly" => (
                "strftime('%Y-%m', s.created_at)",
                "strftime('%Y-%m', a.created_at)",
            ),
            _ => ("DATE(s.created_at)", "DATE(a.created_at)"),
        };

        // The most popular categories by current mean score — the chart's
        // series (kept small so a line chart stays readable).
        let top: Vec<(String, Option<String>)> = {
            let mut stmt = self.conn.prepare(
                "SELECT p.category_id, c.name
                 FROM products p
                 LEFT JOIN categories c ON p.category_id = c.id
                 GROUP BY p.category_id
                 ORDER BY AVG(p.popularity_score) DESC, p.category_id ASC
                 LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![top_categories.max(1)], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    r.get::<_, Option<String>>(1)?,
                ))
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            out
        };
        if top.is_empty() {
            return Ok(Vec::new());
        }
        // Order categories by their ranking index so points sort sensibly.
        let rank: HashMap<String, usize> = top
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (id.clone(), i))
            .collect();

        // (period, category) → raw signals.
        let mut agg: HashMap<(String, String), (i64, i64, i64, i64)> = HashMap::new();
        {
            // Sales: units + distinct transactions per (period, category).
            let mut stmt = self.conn.prepare(&format!(
                "SELECT {s_period} AS period_start, p.category_id,
                        SUM(sl.qty) AS units, COUNT(DISTINCT sl.sale_id) AS txns
                 FROM sale_lines sl
                 JOIN sales s ON sl.sale_id = s.id
                 JOIN products p ON sl.sku = p.sku
                 WHERE s.status = 'completed' AND DATE(s.created_at) BETWEEN ?1 AND ?2
                 GROUP BY period_start, p.category_id"
            ))?;
            let rows = stmt.query_map(params![start_date, end_date], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })?;
            for row in rows {
                let (period_start, cat, units, txns) = row?;
                let e = agg.entry((period_start, cat)).or_insert((0, 0, 0, 0));
                e.0 += units;
                e.1 += txns;
            }
        }
        {
            // Search + edit events per (period, category).
            let mut stmt = self.conn.prepare(&format!(
                "SELECT {a_period} AS period_start, p.category_id, a.event_type, COUNT(*) AS cnt
                 FROM product_activity a
                 JOIN products p ON a.sku = p.sku
                 WHERE DATE(a.created_at) BETWEEN ?1 AND ?2
                 GROUP BY period_start, p.category_id, a.event_type"
            ))?;
            let rows = stmt.query_map(params![start_date, end_date], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })?;
            for row in rows {
                let (period_start, cat, etype, cnt) = row?;
                let e = agg.entry((period_start, cat)).or_insert((0, 0, 0, 0));
                if etype == "search" {
                    e.2 += cnt;
                } else {
                    e.3 += cnt;
                }
            }
        }

        // Only the top categories' points survive (the charts stay small).
        let mut points: Vec<CategoryTrendPoint> = Vec::new();
        for ((period_start, cat), (units, txns, searches, edits)) in agg {
            if !rank.contains_key(&cat) {
                continue;
            }
            let (ms, mq, me) = self.category_means(&cat).unwrap_or((0.0, 0.0, 0.0));
            let score = crate::popularity::score_from_raw(
                units as f64,
                units as f64,
                txns as f64,
                searches as f64,
                searches as f64,
                edits as f64,
                edits as f64,
                ms,
                mq,
                me,
            );
            let name = top
                .iter()
                .find(|(id, _)| *id == cat)
                .and_then(|(_, n)| n.clone());
            points.push(CategoryTrendPoint {
                period_start,
                category_id: cat,
                category_name: name,
                score,
                units_sold: units,
                distinct_transactions: txns,
                searches,
                edits,
            });
        }
        points.sort_by(|a, b| {
            a.period_start
                .cmp(&b.period_start)
                .then_with(|| rank[&a.category_id].cmp(&rank[&b.category_id]))
        });
        Ok(points)
    }

    /// Per-category popularity standings over the whole catalog.
    ///
    /// Returns every category (including the `""` bucket for uncategorized
    /// products) with its product count, mean materialized score, the mean
    /// relative to the catalog average, and the top `top_per_category`
    /// products ranked by score with their category-relative rank and
    /// percentile. Reads are O(catalog) against the materialized column, so
    /// it is cheap and never touches the ledgers.
    pub fn category_popularity(
        &self,
        top_per_category: i64,
    ) -> Result<Vec<CategoryPopularityRow>, CoreError> {
        let catalog_mean: f64 = self
            .conn
            .query_row("SELECT AVG(popularity_score) FROM products", [], |r| {
                r.get(0)
            })
            .unwrap_or(0.0);

        // Per-category aggregates: count + mean score.
        let mut cats: HashMap<String, CategoryPopularityRow> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT p.category_id, c.name, COUNT(*) AS cnt, AVG(p.popularity_score) AS mean
                 FROM products p
                 LEFT JOIN categories c ON p.category_id = c.id
                 GROUP BY p.category_id",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, f64>(3)?,
                ))
            })?;
            for row in rows {
                let (category, name, cnt, mean) = row?;
                let key = category.unwrap_or_default();
                cats.insert(
                    key.clone(),
                    CategoryPopularityRow {
                        category_id: key,
                        category_name: name,
                        product_count: cnt,
                        mean_score: mean,
                        catalog_ratio: if catalog_mean > 0.0 {
                            mean / catalog_mean
                        } else {
                            0.0
                        },
                        top_products: Vec::new(),
                    },
                );
            }
        }

        // Ranked products per category (score desc, SKU tiebreak).
        let mut per_cat: HashMap<String, Vec<(String, String, f64)>> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT p.category_id, p.sku, p.name, p.popularity_score
                 FROM products p
                 ORDER BY p.category_id, p.popularity_score DESC, p.sku ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, f64>(3)?,
                ))
            })?;
            for row in rows {
                let (category, sku, name, score) = row?;
                per_cat
                    .entry(category.unwrap_or_default())
                    .or_default()
                    .push((sku, name, score));
            }
        }

        for (key, rows) in per_cat {
            let count = rows.len() as f64;
            let top: Vec<CategoryTopProduct> = rows
                .into_iter()
                .take(top_per_category.max(0) as usize)
                .enumerate()
                .map(|(i, (sku, name, score))| CategoryTopProduct {
                    sku,
                    name,
                    popularity_score: score,
                    rank: i as i64 + 1,
                    // Linear spread: best = 1.0, worst = 0.0 (1.0 when the
                    // category holds a single product).
                    percentile: if count > 1.0 {
                        (count - 1.0 - i as f64) / (count - 1.0)
                    } else {
                        1.0
                    },
                })
                .collect();
            if let Some(cat) = cats.get_mut(&key) {
                cat.top_products = top;
            }
        }

        let mut out: Vec<CategoryPopularityRow> = cats.into_values().collect();
        out.sort_by(|a, b| {
            b.mean_score
                .partial_cmp(&a.mean_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.category_id.cmp(&b.category_id))
        });
        Ok(out)
    }

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

    /// Distinct completed transactions containing a SKU inside the window —
    /// the breadth input to the sales signal (ADR #37 D6: reach over
    /// one-customer bulk).
    fn sale_distinct_transactions(&self, sku: &str) -> Result<i64, CoreError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT sl.sale_id)
             FROM sale_lines sl
             JOIN sales s ON sl.sale_id = s.id
             WHERE sl.sku = ?1 AND s.status = 'completed'
               AND s.created_at >= datetime('now', ?2)",
            params![sku, window_modifier()],
            |r| r.get(0),
        )?;
        Ok(n)
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

    /// Read a raw `settings` value (None when absent).
    fn read_setting(&self, key: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get::<_, String>(0),
            )
            .ok()
    }

    /// Read a cached catalog mean from `settings` (0.0 when absent).
    fn read_mean(&self, key: &str) -> f64 {
        self.read_setting(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0)
    }

    /// Look up the cached smoothing means for a product's category.
    ///
    /// Returns the category's means when the per-category map (written by
    /// the last full pass) has an entry for it; falls back to the `""`
    /// global entry, then to the legacy `MEAN_*` keys (fresh DB).
    fn category_means(&self, category: &str) -> Option<(f64, f64, f64)> {
        let raw = self.read_setting(CATEGORY_MEANS)?;
        let map: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let entry = map.get(category).or_else(|| map.get(""))?;
        Some((
            entry.get("sales").and_then(|v| v.as_f64()).unwrap_or(0.0),
            entry.get("search").and_then(|v| v.as_f64()).unwrap_or(0.0),
            entry.get("edits").and_then(|v| v.as_f64()).unwrap_or(0.0),
        ))
    }

    /// Smoothing means for a single SKU: its category's cached means, else
    /// the global fallback.
    fn sku_means(&self, sku: &str) -> (f64, f64, f64) {
        let category: Option<String> = self
            .conn
            .query_row(
                "SELECT category_id FROM products WHERE sku = ?1",
                params![sku],
                |r| r.get(0),
            )
            .ok()
            .flatten();
        if let Some(means) = category.as_deref().and_then(|cat| self.category_means(cat)) {
            return means;
        }
        (
            self.read_mean(MEAN_SALES),
            self.read_mean(MEAN_SEARCH),
            self.read_mean(MEAN_EDITS),
        )
    }

    /// Cache a raw `settings` value (JSON or number string).
    fn write_setting(&self, key: &str, value: &str) -> Result<(), CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        self.conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now],
        )?;
        Ok(())
    }

    /// Cache a catalog mean in `settings`.
    fn write_mean(&self, key: &str, value: f64) -> Result<(), CoreError> {
        self.write_setting(key, &value.to_string())
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
        let distinct = self.sale_distinct_transactions(sku)?;
        let searches = self.activity_day_counts(sku, "search")?;
        let edits = self.activity_day_counts(sku, "edit")?;
        let (mean_sales, mean_search, mean_edits) = self.sku_means(sku);
        let score = compute_score(
            &sales,
            distinct,
            &searches,
            &edits,
            mean_sales,
            mean_search,
            mean_edits,
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
        } // Distinct transactions per SKU inside the window (breadth input).
        let mut distinct: HashMap<String, i64> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT sl.sku, COUNT(DISTINCT sl.sale_id)
                 FROM sale_lines sl
                 JOIN sales s ON sl.sale_id = s.id
                 WHERE s.status = 'completed' AND s.created_at >= datetime('now', ?1)
                 GROUP BY sl.sku",
            )?;
            let rows = stmt.query_map(params![window_modifier()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (sku, cnt) = row?;
                distinct.insert(sku, cnt);
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

        // ── Per-product raw + votes, grouped by category ───────────────
        let mut products: Vec<ProductSignals> = Vec::new();
        {
            let mut stmt = self.conn.prepare("SELECT sku, category_id FROM products")?;
            let rows = stmt.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })?;
            for row in rows {
                let (sku, category) = row?;
                let s_events = sales.get(&sku).map(Vec::as_slice).unwrap_or(&[]);
                let q_events = searches.get(&sku).map(Vec::as_slice).unwrap_or(&[]);
                let e_events = edits.get(&sku).map(Vec::as_slice).unwrap_or(&[]);
                products.push((
                    sku,
                    category,
                    decayed_sum(s_events),
                    total_events(s_events),
                    decayed_sum(q_events),
                    total_events(q_events),
                    decayed_sum(e_events),
                    total_events(e_events),
                ));
            }
        }

        // Global means — the fallback for uncategorized products and the
        // `""` entry of the per-category cache. The sales mean is computed
        // over breadth-scaled raws so the smoothing scale always matches the
        // scaled raw inside `score_from_raw`.
        let n = products.len() as f64;
        let mean_sales = if n > 0.0 {
            products
                .iter()
                .map(|p| {
                    p.2 * crate::popularity::breadth_factor(
                        distinct.get(&p.0).copied().unwrap_or(0),
                    )
                })
                .sum::<f64>()
                / n
        } else {
            0.0
        };
        let mean_search = if n > 0.0 {
            products.iter().map(|p| p.4).sum::<f64>() / n
        } else {
            0.0
        };
        let mean_edits = if n > 0.0 {
            products.iter().map(|p| p.6).sum::<f64>() / n
        } else {
            0.0
        };

        // Per-category means (ADR #37 D6): each product is smoothed toward
        // its own category's mean, so a quiet category is not drowned by a
        // hot one. `""` = uncategorized bucket, kept in the map as the
        // explicit global entry.
        let mut cat_sums: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
        for p in &products {
            let key = p.1.clone().unwrap_or_default();
            let entry = cat_sums.entry(key).or_insert((0.0, 0.0, 0.0, 0.0));
            entry.0 +=
                p.2 * crate::popularity::breadth_factor(distinct.get(&p.0).copied().unwrap_or(0));
            entry.1 += p.4;
            entry.2 += p.6;
            entry.3 += 1.0;
        }
        let mut cat_means: HashMap<String, (f64, f64, f64)> = HashMap::new();
        for (cat, (sr, qr, er, count)) in cat_sums {
            if count > 0.0 {
                cat_means.insert(cat, (sr / count, qr / count, er / count));
            }
        }
        cat_means.insert(String::new(), (mean_sales, mean_search, mean_edits));

        // Persist the per-category cache as JSON, and keep the global
        // MEAN_* keys (single-SKU fallback before the first full pass).
        let mut cat_json = serde_json::Map::new();
        for (cat, (ms, mq, me)) in &cat_means {
            cat_json.insert(
                cat.clone(),
                serde_json::json!({ "sales": ms, "search": mq, "edits": me }),
            );
        }
        self.write_setting(
            CATEGORY_MEANS,
            &serde_json::Value::Object(cat_json).to_string(),
        )?;
        self.write_mean(MEAN_SALES, mean_sales)?;
        self.write_mean(MEAN_SEARCH, mean_search)?;
        self.write_mean(MEAN_EDITS, mean_edits)?;

        // ── Write scores (one transaction: the per-SKU updates are atomic) ─
        let tx = self.conn.unchecked_transaction()?;
        for (sku, category, sr, sv, qr, qv, er, ev) in products {
            let key = category.unwrap_or_default();
            let (ms, mq, me) =
                cat_means
                    .get(&key)
                    .copied()
                    .unwrap_or((mean_sales, mean_search, mean_edits));
            let distinct = distinct.get(&sku).copied().unwrap_or(0) as f64;
            let score =
                crate::popularity::score_from_raw(sr, sv, distinct, qr, qv, er, ev, ms, mq, me);
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
        // Breadth-scaled raw: 4 completed units across 1 distinct sale.
        let sales_raw = crate::popularity::decayed_sum(&[DayCount {
            days_ago: 0,
            count: 4,
        }]) * crate::popularity::breadth_factor(1);
        // The pending sale's 3 units must NOT inflate the signal: the score
        // can at most reflect the 4 completed units (smoothing toward the
        // catalog mean can only shrink it, never grow it past the raw).
        let seven_units = crate::popularity::decayed_sum(&[DayCount {
            days_ago: 0,
            count: 7,
        }]) * crate::popularity::breadth_factor(2);
        let score_raw_share = crate::popularity::WEIGHT_SALES * sales_raw;
        assert!(
            score_raw_share <= pending_influence + 1e-9
                && pending_influence < crate::popularity::WEIGHT_SALES * seven_units,
            "backfilled score must reflect completed sales only (pending excluded)"
        );
    }

    fn seed_category(conn: &rusqlite::Connection, id: &str, name: &str) {
        conn.execute(
            "INSERT OR IGNORE INTO categories (id, name, colour, icon, created_at, updated_at) \
             VALUES (?1, ?2, '#06b6d4', '', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            params![id, name],
        )
        .unwrap();
    }

    /// Seed a product in a category with `units` sold today (completed sale).
    fn seed_sold_in_category(
        conn: &rusqlite::Connection,
        sku: &str,
        category: Option<&str>,
        units: i64,
    ) {
        let id = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, category_id, created_at, updated_at) \
             VALUES (?1, ?2, ?2, 1000, 'USD', ?3, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            params![id, sku, category],
        )
        .unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
             (?1, ?2, 'USD', 1, 'completed', ?3, ?3)",
            params![format!("sale-{sku}"), units * 1000, now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
             (?1, ?2, ?3, ?4, 1000, ?5, 'USD', 1)",
            params![format!("sl-{sku}"), format!("sale-{sku}"), sku, units, units * 1000],
        )
        .unwrap();
    }

    #[test]
    fn full_pass_smooths_toward_category_mean_not_catalog_mean() {
        // A hot category (5 × 100 units) inflates the catalog mean to 63.125.
        // Global smoothing then pulls the LOWEST-evidence product up hardest
        // (closest to the inflated mean), inverting the ranking inside the
        // quiet category — the 1-unit seller would outrank the 2-unit seller.
        // Per-category means keep the ordering honest: 2 units beats 1 unit
        // within the quiet category.
        let conn = fresh();
        seed_category(&conn, "cat-hot", "Hot");
        seed_category(&conn, "cat-quiet", "Quiet");
        for i in 0..5 {
            seed_sold_in_category(&conn, &format!("HOT-{i}"), Some("cat-hot"), 100);
        }
        seed_sold_in_category(&conn, "QUIET-2", Some("cat-quiet"), 2);
        seed_sold_in_category(&conn, "QUIET-1", Some("cat-quiet"), 1);
        // An uncategorized product uses the global fallback.
        seed_sold_in_category(&conn, "NO-CAT", None, 2);

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
        let (two, one) = (score("QUIET-2"), score("QUIET-1"));
        assert!(
            two > one,
            "within the quiet category the 2-unit seller must outrank the \
             1-unit seller (two={two}, one={one})"
        );
        // Reference: the catalog-mean blend inverts that ordering — the
        // whole pathology per-category popularity fixes. The catalog mean is
        // the breadth-scaled average: 5×100, 2, 1, and 2 units, each sold in
        // one transaction → (5·100 + 2 + 1 + 2)·ln2 / 8.
        let catalog_mean = (5.0 * 100.0 + 2.0 + 1.0 + 2.0) * std::f64::consts::LN_2 / 8.0;
        let global_two = crate::popularity::compute_score(
            &[DayCount {
                days_ago: 0,
                count: 2,
            }],
            1,
            &[],
            &[],
            catalog_mean,
            0.0,
            0.0,
        );
        let global_one = crate::popularity::compute_score(
            &[DayCount {
                days_ago: 0,
                count: 1,
            }],
            1,
            &[],
            &[],
            catalog_mean,
            0.0,
            0.0,
        );
        assert!(
            global_one > global_two,
            "catalog-mean blend must invert the ordering (one={global_one}, two={global_two})"
        );
        // Uncategorized falls back to the global mean (identical inputs to
        // the catalog blend above).
        let no_cat = score("NO-CAT");
        assert!(
            (no_cat - global_two).abs() < 1e-9,
            "uncategorized must use the global mean, not the quiet category's \
             (no-cat={no_cat}, expected={global_two})"
        );
        // The hot category still ranks highest.
        let hot = score("HOT-0");
        assert!(hot > two, "hot category must still outrank the quiet one");
    }

    #[test]
    fn single_sku_recompute_uses_cached_category_means() {
        let conn = fresh();
        seed_category(&conn, "cat-hot", "Hot");
        seed_category(&conn, "cat-quiet", "Quiet");
        for i in 0..5 {
            seed_sold_in_category(&conn, &format!("HOT-{i}"), Some("cat-hot"), 100);
        }
        seed_sold_in_category(&conn, "QUIET-1", Some("cat-quiet"), 2);

        let store = Store::new(&conn);
        store.recompute_all_popularity().unwrap();
        let full_pass_score: f64 = conn
            .query_row(
                "SELECT popularity_score FROM products WHERE sku = 'QUIET-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();

        // A later single-SKU recompute (e.g. after a search event) must
        // reproduce the same category-relative score via the JSON cache.
        store.recompute_popularity("QUIET-1").unwrap();
        let after: f64 = conn
            .query_row(
                "SELECT popularity_score FROM products WHERE sku = 'QUIET-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            (full_pass_score - after).abs() < 1e-9,
            "single-SKU recompute must reuse the category means \
             (full-pass={full_pass_score}, recomputed={after})"
        );
    }

    #[test]
    fn category_popularity_ranks_categories_and_top_products() {
        let conn = fresh();
        seed_category(&conn, "cat-hot", "Hot");
        seed_category(&conn, "cat-quiet", "Quiet");
        for (sku, cat, score) in [
            ("HOT-A", "cat-hot", 9.0),
            ("HOT-B", "cat-hot", 5.0),
            ("HOT-C", "cat-hot", 3.0),
            ("HOT-D", "cat-hot", 1.0),
            ("QUIET-1", "cat-quiet", 4.0),
            ("QUIET-2", "cat-quiet", 2.0),
            ("NO-CAT", "", 6.0),
        ] {
            let id = uuid::Uuid::now_v7().to_string();
            let cat = if cat.is_empty() { None } else { Some(cat) };
            conn.execute(
                "INSERT INTO products (id, sku, name, price_minor, currency, category_id, \
                 popularity_score, created_at, updated_at) VALUES (?1, ?2, ?2, 1000, 'USD', \
                 ?3, ?4, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
                params![id, sku, cat, score],
            )
            .unwrap();
        }

        let store = Store::new(&conn);
        let rows = store.category_popularity(3).unwrap();

        // Catalog mean = (9+5+3+1+4+2+6)/7 = 4.2857…
        assert_eq!(rows.len(), 3, "hot, quiet, and the uncategorized bucket");

        let hot = rows.iter().find(|r| r.category_id == "cat-hot").unwrap();
        assert_eq!(hot.category_name.as_deref(), Some("Hot"));
        assert_eq!(hot.product_count, 4);
        assert!((hot.mean_score - 4.5).abs() < 1e-9);
        assert!((hot.catalog_ratio - 4.5 / (30.0 / 7.0)).abs() < 1e-9);
        // Top-3 limited to 3, ranked by score with SKU tiebreak.
        let top: Vec<(&str, i64, f64)> = hot
            .top_products
            .iter()
            .map(|t| (t.sku.as_str(), t.rank, t.percentile))
            .collect();
        assert_eq!(
            top,
            vec![
                ("HOT-A", 1, 1.0),
                ("HOT-B", 2, 2.0 / 3.0),
                ("HOT-C", 3, 1.0 / 3.0)
            ]
        );

        let uncat = rows.iter().find(|r| r.category_id.is_empty()).unwrap();
        assert_eq!(uncat.category_name, None);
        assert_eq!(uncat.product_count, 1);
        // Single-product category: percentile is 1.0 by definition.
        assert_eq!(uncat.top_products[0].percentile, 1.0);

        // Categories sort by mean score descending: hot (4.5) > uncat (6.0)?
        // No — the uncategorized bucket's mean is 6.0, so it ranks first.
        assert_eq!(rows[0].category_id, "");
        assert_eq!(rows[1].category_id, "cat-hot");
        assert_eq!(rows[2].category_id, "cat-quiet");
        assert!((rows[2].mean_score - 3.0).abs() < 1e-9);

        // top_per_category=1 keeps only the leader.
        let one = store.category_popularity(1).unwrap();
        let hot_one = one.iter().find(|r| r.category_id == "cat-hot").unwrap();
        assert_eq!(hot_one.top_products.len(), 1);
        assert_eq!(hot_one.top_products[0].sku, "HOT-A");
    }

    #[test]
    fn category_popularity_trend_buckets_daily_and_scores() {
        let conn = fresh();
        seed_category(&conn, "cat-a", "A");
        seed_category(&conn, "cat-b", "B");
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let yesterday = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(1))
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute_batch(
            &format!(
                "INSERT INTO products (id, sku, name, price_minor, currency, category_id, created_at, updated_at) VALUES
                ('p-1', 'A-1', 'A one', 1000, 'USD', 'cat-a', '{now}', '{now}'),
                ('p-2', 'A-2', 'A two', 1000, 'USD', 'cat-a', '{now}', '{now}'),
                ('p-3', 'B-1', 'B one', 1000, 'USD', 'cat-b', '{now}', '{now}');
                INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
                ('s1', 2000, 'USD', 1, 'completed', '{now}', '{now}'),
                ('s2', 1000, 'USD', 1, 'completed', '{yesterday}', '{yesterday}'),
                ('s3', 3000, 'USD', 1, 'completed', '{now}', '{now}');
                INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
                ('sl1', 's1', 'A-1', 2, 1000, 2000, 'USD', 1),
                ('sl2', 's2', 'A-2', 1, 1000, 1000, 'USD', 1),
                ('sl3', 's3', 'B-1', 3, 1000, 3000, 'USD', 1);"
            ),
        )
        .unwrap();
        // Cache means via a full pass so single-point scores smooth
        // consistently with the rest of the system.
        let store = Store::new(&conn);
        store.recompute_all_popularity().unwrap();

        let today = chrono::Utc::now().date_naive().to_string();
        let yesterday_s = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(1))
            .unwrap()
            .date_naive()
            .to_string();
        let points = store
            .category_popularity_trend("2000-01-01", "2099-12-31", "daily", 5)
            .unwrap();

        // Only (period, category) pairs with activity produce points:
        // cat-a sold both days, cat-b only today (uncategorized never sold).
        // Within a period, categories sort by mean-score rank (cat-b first).
        let keys: Vec<(&str, &str)> = points
            .iter()
            .map(|p| (p.period_start.as_str(), p.category_id.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![
                (yesterday_s.as_str(), "cat-a"),
                (today.as_str(), "cat-b"),
                (today.as_str(), "cat-a"),
            ]
        );

        let a_today = points
            .iter()
            .find(|p| p.period_start == today && p.category_id == "cat-a")
            .unwrap();
        assert_eq!(a_today.units_sold, 2);
        assert_eq!(a_today.distinct_transactions, 1);
        assert_eq!(a_today.searches, 0);
        assert_eq!(a_today.edits, 0);
        assert!(a_today.score > 0.0, "sales-driven period score must be > 0");

        let b_today = points
            .iter()
            .find(|p| p.period_start == today && p.category_id == "cat-b")
            .unwrap();
        assert!(b_today.score > a_today.score, "3 units must beat 2 units");
    }

    #[test]
    fn category_popularity_trend_monthly_and_top_limit() {
        let conn = fresh();
        seed_category(&conn, "cat-a", "A");
        seed_category(&conn, "cat-b", "B");
        for (sku, cat, score) in [
            ("A-1", "cat-a", 8.0),
            ("A-2", "cat-a", 6.0),
            ("B-1", "cat-b", 2.0),
            ("C-1", "", 9.0),
        ] {
            let id = uuid::Uuid::now_v7().to_string();
            let cat = if cat.is_empty() { None } else { Some(cat) };
            conn.execute(
                "INSERT INTO products (id, sku, name, price_minor, currency, category_id, \
                 popularity_score, created_at, updated_at) VALUES (?1, ?2, ?2, 1000, 'USD', \
                 ?3, ?4, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
                params![id, sku, cat, score],
            )
            .unwrap();
        }
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute_batch(&format!(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
             ('s1', 1000, 'USD', 1, 'completed', '{now}', '{now}');
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
             ('sl1', 's1', 'A-1', 1, 1000, 1000, 'USD', 1);"
        ))
        .unwrap();

        let store = Store::new(&conn);
        let points = store
            .category_popularity_trend("2000-01-01", "2099-12-31", "monthly", 2)
            .unwrap();

        // Top 2 categories by mean score: uncategorized (9.0) and cat-a
        // (7.0) — cat-b (2.0) is excluded. Only cat-a has a sales point.
        let cats: Vec<&str> = points.iter().map(|p| p.category_id.as_str()).collect();
        assert_eq!(cats, vec!["cat-a"], "only top categories appear");
        assert_eq!(
            points[0].period_start,
            chrono::Utc::now().date_naive().format("%Y-%m").to_string(),
            "monthly bucket is YYYY-MM"
        );
        assert_eq!(points[0].units_sold, 1);
    }

    #[test]
    fn category_popularity_empty_catalog() {
        let conn = fresh();
        let store = Store::new(&conn);
        let rows = store.category_popularity(3).unwrap();
        assert!(rows.is_empty(), "no products → no category rows");
    }

    #[test]
    fn breadth_weighting_ranks_spread_over_single_bulk_sale() {
        // Same volume (10 units), same day: 10 units in one sale vs 10 units
        // spread across 5 different customers. The spread seller must
        // outrank the bulk seller (ADR #37 D6 reach-over-bulk).
        let conn = fresh();
        for sku in ["BULK", "SPREAD"] {
            let id = uuid::Uuid::now_v7().to_string();
            conn.execute(
                "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) \
                 VALUES (?1, ?2, ?2, 1000, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
                params![id, sku],
            )
            .unwrap();
        }
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut batch = String::new();
        // BULK: one sale of 10 units.
        batch.push_str(&format!(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
             ('s-bulk', 10000, 'USD', 1, 'completed', '{now}', '{now}');
             INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
             ('sl-bulk', 's-bulk', 'BULK', 10, 1000, 10000, 'USD', 1);"
        ));
        // SPREAD: five sales of 2 units each.
        for i in 0..5 {
            batch.push_str(&format!(
                "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at) VALUES
                 ('s-spread-{i}', 2000, 'USD', 1, 'completed', '{now}', '{now}');
                 INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position) VALUES
                 ('sl-spread-{i}', 's-spread-{i}', 'SPREAD', 2, 1000, 2000, 'USD', 1);"
            ));
        }
        conn.execute_batch(&batch).unwrap();

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
        let (spread, bulk) = (score("SPREAD"), score("BULK"));
        assert!(
            spread > bulk,
            "same volume across more customers must rank higher \
             (spread={spread}, bulk={bulk})"
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

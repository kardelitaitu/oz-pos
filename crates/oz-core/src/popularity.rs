//! Product popularity scoring (ADR #37 D1).
//!
//! Pure, unit-tested formula: a recency-decayed, evidence-smoothed, weighted
//! blend of three signals — units sold, acted-upon searches, and product
//! edits. The formula lives here in code; the ledgers (`sale_lines` +
//! `product_activity`) keep history so the formula can be retuned without a
//! migration.
//!
//! # Formula
//!
//! ```text
//! raw_c      = Σ_t events(t) × λ^t           t = days ago, λ = 0.93, 90-day window
//! sales_raw  = raw_sales × ln(1 + distinct transactions)     breadth weighting (D6)
//! component' = mean_c + (raw_c − mean_c) × v / (m + v)     v = event count, m = 5
//! score      = 0.6·Sales' + 0.3·Search' + 0.1·Edits'
//! ```
//!
//! The sales signal is additionally **breadth-weighted**: the decayed unit
//! volume is multiplied by `ln(1 + distinct transactions)`, so the same
//! volume sold to many different customers outranks one bulk buyer — reach
//! over one-customer bulk (ADR #37 D6).
//!
//! The smoothing term shrinks the *deviation from the catalog mean* by the
//! evidence fraction `v/(m + v)`: a product with zero events sits exactly at
//! the catalog mean (fair cold start), a two-sale fluke is pulled toward the
//! mean and cannot top the list, and a high-evidence product approaches its
//! raw score (evidence wins). This refines the ADR's pseudocode, which scaled
//! a sum-scale `raw` against a count-scale `v`; the shrinkage form below
//! achieves the ADR's stated goals on the correct scale.

/// Daily recency decay factor (λ). `1/(1−λ) ≈ 14` effective days of memory.
pub const DECAY_PER_DAY: f64 = 0.93;
/// Hard lookback window in days. At the edge `λ^90 ≈ 0.0015`, so truncation
/// loses nothing measurable.
pub const WINDOW_DAYS: i64 = 90;
/// Minimum votes for Bayesian smoothing (m).
pub const MIN_VOTES: f64 = 5.0;
/// Signal weights (sales is the dominant, correct signal; edits are
/// operational attention and deliberately capped).
pub const WEIGHT_SALES: f64 = 0.6;
/// Weight of the search signal in the blended score.
pub const WEIGHT_SEARCH: f64 = 0.3;
/// Weight of the edit signal in the blended score.
pub const WEIGHT_EDITS: f64 = 0.1;

/// One day of events: `count` events occurring `days_ago` days in the past.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayCount {
    /// Days before today (0 = today). Outside `[0, WINDOW_DAYS)` is ignored.
    pub days_ago: i64,
    /// Number of events (units sold, searches, edits) that day.
    pub count: i64,
}

/// Recency-decayed sum of the events: `Σ count × λ^days_ago`.
pub fn decayed_sum(events: &[DayCount]) -> f64 {
    events
        .iter()
        .filter(|e| e.count > 0 && (0..WINDOW_DAYS).contains(&e.days_ago))
        .map(|e| e.count as f64 * DECAY_PER_DAY.powi(e.days_ago as i32))
        .sum()
}

/// Breadth multiplier for the sales signal: `ln(1 + distinct transactions)`.
///
/// - 0 transactions → 0 (no sales signal).
/// - 1 transaction (a single bulk buyer) → `ln 2 ≈ 0.69`.
/// - 9 transactions → `ln 10 ≈ 2.30` — the same volume spread across nine
///   customers is worth ~3.3× a single-customer bulk order.
pub fn breadth_factor(distinct_transactions: i64) -> f64 {
    (1.0 + distinct_transactions.max(0) as f64).ln()
}

/// Total raw events inside the window — the evidence count `v` for smoothing.
pub fn total_events(events: &[DayCount]) -> f64 {
    events
        .iter()
        .filter(|e| e.count > 0 && (0..WINDOW_DAYS).contains(&e.days_ago))
        .map(|e| e.count as f64)
        .sum()
}

/// Evidence-smoothed component: `mean + (raw − mean) × v / (m + v)`.
///
/// - `v = 0` → exactly `mean` (fair cold start, no flukes).
/// - `v → ∞` → `raw` (evidence dominates).
/// - low evidence → pulled toward `mean` (a two-sale fluke cannot top a
///   steady seller).
pub fn smoothed(raw: f64, votes: f64, mean: f64) -> f64 {
    mean + (raw - mean) * votes / (MIN_VOTES + votes)
}

/// Weighted blend of the three smoothed components (ADR #37 D1).
///
/// `sales_transactions` is the number of distinct completed sales containing
/// the product inside the window — the breadth input to the sales signal.
pub fn compute_score(
    sales: &[DayCount],
    sales_transactions: i64,
    searches: &[DayCount],
    edits: &[DayCount],
    sales_mean: f64,
    search_mean: f64,
    edit_mean: f64,
) -> f64 {
    score_from_raw(
        decayed_sum(sales),
        total_events(sales),
        sales_transactions as f64,
        decayed_sum(searches),
        total_events(searches),
        decayed_sum(edits),
        total_events(edits),
        sales_mean,
        search_mean,
        edit_mean,
    )
}

/// A linear least-squares forecast over a recent per-period unit series.
///
/// Simple next-period estimate used by the demand-forecast surface: the
/// slope is fitted over the series' own index (period 0..n−1) and projected
/// one period ahead (`x = n`). Two or fewer points fall back to the recent
/// average (no trend can be fit).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitForecast {
    /// Fitted slope — units per period; 0 when fewer than 2 points.
    pub trend_per_period: f64,
    /// Baseline — mean units per period over the series.
    pub recent_avg_units: f64,
    /// Predicted units for the next period (never negative).
    pub forecast_units: i64,
}

/// Fit [`UnitForecast`] to a chronological units series (oldest first).
pub fn linear_forecast(units: &[f64]) -> UnitForecast {
    let n = units.len() as f64;
    if n == 0.0 {
        return UnitForecast {
            trend_per_period: 0.0,
            recent_avg_units: 0.0,
            forecast_units: 0,
        };
    }
    let mean_y = units.iter().sum::<f64>() / n;
    if units.len() < 2 {
        return UnitForecast {
            trend_per_period: 0.0,
            recent_avg_units: mean_y,
            forecast_units: mean_y.round().max(0.0) as i64,
        };
    }
    let mean_x = (n - 1.0) / 2.0;
    let (mut num, mut den) = (0.0, 0.0);
    for (i, y) in units.iter().enumerate() {
        let x = i as f64;
        num += (x - mean_x) * (y - mean_y);
        den += (x - mean_x) * (x - mean_x);
    }
    let slope = if den > 0.0 { num / den } else { 0.0 };
    let intercept = mean_y - slope * mean_x;
    UnitForecast {
        trend_per_period: slope,
        recent_avg_units: mean_y,
        forecast_units: (intercept + slope * n).max(0.0).round() as i64,
    }
}

/// Weighted blend over already-computed raw values (full-pass path).
///
/// Ten positional args mirror the three (raw, votes) signal pairs plus the
/// sales breadth input plus the three catalog means; the full-pass path
/// already holds all of them. The breadth multiplier is applied here — the
/// single place where the sales raw is scaled — so the full pass and the
/// per-event recompute can never drift apart. `sales_raw` must therefore be
/// the *unscaled* decayed unit sum; `sales_distinct` carries the distinct
/// transaction count.
#[allow(clippy::too_many_arguments)]
pub fn score_from_raw(
    sales_raw: f64,
    sales_votes: f64,
    sales_distinct: f64,
    search_raw: f64,
    search_votes: f64,
    edit_raw: f64,
    edit_votes: f64,
    sales_mean: f64,
    search_mean: f64,
    edit_mean: f64,
) -> f64 {
    let sales_scaled = sales_raw * breadth_factor(sales_distinct as i64);
    let s = smoothed(sales_scaled, sales_votes, sales_mean);
    let q = smoothed(search_raw, search_votes, search_mean);
    let e = smoothed(edit_raw, edit_votes, edit_mean);
    WEIGHT_SALES * s + WEIGHT_SEARCH * q + WEIGHT_EDITS * e
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(days_ago: i64, count: i64) -> DayCount {
        DayCount { days_ago, count }
    }

    #[test]
    fn decayed_sum_weights_recent_events() {
        // 10 units today + 10 units 1 day ago + 10 units 30 days ago.
        let events = [day(0, 10), day(1, 10), day(30, 10)];
        let expected = 10.0 + 10.0 * DECAY_PER_DAY + 10.0 * DECAY_PER_DAY.powi(30);
        assert!((decayed_sum(&events) - expected).abs() < 1e-9);
    }

    #[test]
    fn decayed_sum_ignores_out_of_window_and_zero() {
        let events = [
            day(-1, 10),
            day(WINDOW_DAYS, 10),
            day(WINDOW_DAYS + 5, 3),
            day(2, 0),
        ];
        assert_eq!(decayed_sum(&events), 0.0);
        assert_eq!(total_events(&events), 0.0);
    }

    #[test]
    fn total_events_counts_raw_evidence() {
        let events = [day(0, 5), day(1, 7), day(95, 100)];
        assert_eq!(total_events(&events), 12.0);
    }

    #[test]
    fn smoothed_zero_evidence_equals_mean() {
        assert_eq!(smoothed(0.0, 0.0, 42.0), 42.0);
    }

    #[test]
    fn smoothed_high_evidence_approaches_raw() {
        let v = 100_000.0;
        let s = smoothed(500.0, v, 42.0);
        // 42 + 458 × 100000/100005 ≈ 499.977 — within 5 cents of raw.
        assert!((s - 500.0).abs() < 0.05);
    }

    #[test]
    fn smoothed_low_evidence_fluke_pulled_to_mean() {
        // A 2-sale fluke (raw 2, v 2) must stay below the catalog mean (50).
        let s = smoothed(2.0, 2.0, 50.0);
        assert!(s < 50.0);
        assert!(s > 2.0);
    }

    #[test]
    fn compute_score_blends_with_weights() {
        let sales = [day(0, 100)];
        let searches: [DayCount; 0] = [];
        let edits: [DayCount; 0] = [];
        // sales raw = 100 × ln(1+9) = 100 × 2.3026; v = 100 → ~230.3; others = mean.
        let score = compute_score(&sales, 9, &searches, &edits, 50.0, 10.0, 5.0);
        let sales_c = smoothed(100.0 * breadth_factor(9), 100.0, 50.0);
        let expected = WEIGHT_SALES * sales_c + WEIGHT_SEARCH * 10.0 + WEIGHT_EDITS * 5.0;
        assert!((score - expected).abs() < 1e-9);
    }

    #[test]
    fn compute_score_empty_catalog_is_zero() {
        let empty: [DayCount; 0] = [];
        assert_eq!(compute_score(&empty, 0, &empty, &empty, 0.0, 0.0, 0.0), 0.0);
    }

    #[test]
    fn breadth_factor_rewards_spread_over_bulk() {
        assert_eq!(breadth_factor(0), 0.0);
        assert_eq!(breadth_factor(1), std::f64::consts::LN_2);
        // Ten units sold to one customer is worth less than the same ten
        // units sold to nine different customers.
        let bulk = breadth_factor(1);
        let spread = breadth_factor(9);
        assert!(spread > bulk * 3.0, "spread={spread}, bulk={bulk}");
    }

    #[test]
    fn breadth_never_scales_up_zero_sales() {
        let empty: [DayCount; 0] = [];
        // No units → raw 0 × factor = 0 regardless of transaction count.
        let score = compute_score(&empty, 10, &empty, &empty, 0.0, 0.0, 0.0);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn linear_forecast_fits_a_perfect_line() {
        // 10, 12, 14, 16 → slope 2/day, next period = 18.
        let f = linear_forecast(&[10.0, 12.0, 14.0, 16.0]);
        assert!((f.trend_per_period - 2.0).abs() < 1e-9);
        assert_eq!(f.forecast_units, 18);
        assert!((f.recent_avg_units - 13.0).abs() < 1e-9);
    }

    #[test]
    fn linear_forecast_flat_series_has_zero_slope() {
        let f = linear_forecast(&[7.0, 7.0, 7.0, 7.0]);
        assert_eq!(f.trend_per_period, 0.0);
        assert_eq!(f.forecast_units, 7);
    }

    #[test]
    fn linear_forecast_declining_never_goes_negative() {
        // 20, 10, 5 → steep decline; the next-period projection would be
        // negative, so it must floor at 0.
        let f = linear_forecast(&[20.0, 10.0, 5.0]);
        assert!(f.trend_per_period < 0.0);
        assert!(f.forecast_units >= 0, "forecast must never be negative");
    }

    #[test]
    fn linear_forecast_single_point_falls_back_to_average() {
        let f = linear_forecast(&[9.0]);
        assert_eq!(f.trend_per_period, 0.0);
        assert_eq!(f.forecast_units, 9);
        assert_eq!(linear_forecast(&[]).forecast_units, 0);
    }

    #[test]
    fn window_edge_is_lossless() {
        // A sale at exactly the window edge contributes ~0.0015, negligible.
        let edge = decayed_sum(&[day(WINDOW_DAYS - 1, 1)]);
        assert!(edge < 0.01);
    }
}

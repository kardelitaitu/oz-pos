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
//! component' = mean_c + (raw_c − mean_c) × v / (m + v)     v = event count, m = 5
//! score      = 0.6·Sales' + 0.3·Search' + 0.1·Edits'
//! ```
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
pub fn compute_score(
    sales: &[DayCount],
    searches: &[DayCount],
    edits: &[DayCount],
    sales_mean: f64,
    search_mean: f64,
    edit_mean: f64,
) -> f64 {
    score_from_raw(
        decayed_sum(sales),
        total_events(sales),
        decayed_sum(searches),
        total_events(searches),
        decayed_sum(edits),
        total_events(edits),
        sales_mean,
        search_mean,
        edit_mean,
    )
}

/// Weighted blend over already-computed raw values (full-pass path).
///
/// Nine positional args mirror the three (raw, votes) signal pairs plus the
/// three catalog means; the full-pass path already holds all of them.
#[allow(clippy::too_many_arguments)]
pub fn score_from_raw(
    sales_raw: f64,
    sales_votes: f64,
    search_raw: f64,
    search_votes: f64,
    edit_raw: f64,
    edit_votes: f64,
    sales_mean: f64,
    search_mean: f64,
    edit_mean: f64,
) -> f64 {
    let s = smoothed(sales_raw, sales_votes, sales_mean);
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
        // sales raw = 100, v = 100 → ~100; others = mean.
        let score = compute_score(&sales, &searches, &edits, 50.0, 10.0, 5.0);
        let sales_c = smoothed(100.0, 100.0, 50.0);
        let expected = WEIGHT_SALES * sales_c + WEIGHT_SEARCH * 10.0 + WEIGHT_EDITS * 5.0;
        assert!((score - expected).abs() < 1e-9);
    }

    #[test]
    fn compute_score_empty_catalog_is_zero() {
        let empty: [DayCount; 0] = [];
        assert_eq!(compute_score(&empty, &empty, &empty, 0.0, 0.0, 0.0), 0.0);
    }

    #[test]
    fn window_edge_is_lossless() {
        // A sale at exactly the window edge contributes ~0.0015, negligible.
        let edge = decayed_sum(&[day(WINDOW_DAYS - 1, 1)]);
        assert!(edge < 0.01);
    }
}

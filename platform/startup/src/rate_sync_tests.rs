//! Unit tests for the untrusted-API rate → fixed-point conversion.

use super::*;

#[test]
fn converts_ordinary_rates_exactly() {
    assert_eq!(rate_to_millionths(1.0), Some(1_000_000));
    assert_eq!(rate_to_millionths(1.08), Some(1_080_000));
    assert_eq!(rate_to_millionths(16_000.0), Some(16_000_000_000));
    assert_eq!(rate_to_millionths(0.92), Some(920_000));
}

#[test]
fn neutralises_fp_offbyone_at_the_sixth_decimal() {
    // 1.495 * 1e6 = 1494999.9999999998 in binary — .round() must recover 1495000.
    assert_eq!(rate_to_millionths(1.495), Some(1_495_000));
    assert_eq!(rate_to_millionths(149.5), Some(149_500_000));
}

#[test]
fn rejects_non_positive_and_non_finite() {
    assert_eq!(rate_to_millionths(0.0), None);
    assert_eq!(rate_to_millionths(-1.0), None);
    assert_eq!(rate_to_millionths(f64::NAN), None);
    assert_eq!(rate_to_millionths(f64::INFINITY), None);
    assert_eq!(rate_to_millionths(f64::NEG_INFINITY), None);
}

#[test]
fn rejects_absurd_magnitudes_that_would_saturate_the_cast() {
    // The pre-fix `(rate * RATE_SCALE).round() as i64` saturates these to
    // i64::MAX, which then PASSES the repo's >0 validation and persists a
    // garbage rate from an untrusted network response.
    assert_eq!(rate_to_millionths(1e300), None);
    assert_eq!(rate_to_millionths(1e10), None);
    assert_eq!(rate_to_millionths(f64::MAX), None);
}

#[test]
fn rejects_rates_below_the_fixed_point_resolution() {
    // 5e-7 rounds to 1 (half-up); 1e-7 rounds to 0 — a zero millionths
    // rate would be rejected by the repo anyway, so the helper says None
    // and the sync logs it instead of swallowing an error.
    assert_eq!(rate_to_millionths(5e-7), Some(1));
    assert_eq!(rate_to_millionths(1e-7), None);
}

#[test]
fn accepts_the_top_of_the_bounded_range() {
    // Just under the 1e10 bound; the product stays inside f64 exactness.
    assert_eq!(rate_to_millionths(999_999_999.0), Some(999_999_999_000_000));
}

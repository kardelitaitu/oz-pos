//! Prometheus metrics collection for OZ-POS.
//!
//! Gauge and counter helpers that report key business and system
//! metrics to a `/metrics` HTTP endpoint.
//!
//! Feature-gated behind `metrics` — compiled out when the feature
//! is not enabled.

use prometheus::{Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry};

use std::sync::OnceLock;

/// Global Prometheus registry.
fn registry() -> &'static Registry {
    static REG: OnceLock<Registry> = OnceLock::new();
    REG.get_or_init(Registry::new)
}

// ── Business metrics ──────────────────────────────────────────────

/// Total sales completed (counter).
pub fn sales_completed() -> &'static IntCounter {
    static METRIC: OnceLock<IntCounter> = OnceLock::new();
    METRIC.get_or_init(|| {
        let opts = Opts::new(
            "oz_pos_sales_completed_total",
            "Total number of completed sales",
        )
        .namespace("oz_pos")
        .subsystem("sales");
        // SAFETY: static metric name/opts are compile-time constants; construction cannot fail
        let counter = IntCounter::with_opts(opts).expect("invalid sales_completed counter opts");
        registry()
            .register(Box::new(counter.clone()))
            .expect("register sales_completed counter"); // SAFETY: static registration of a freshly-constructed metric cannot fail
        counter
    })
}

/// Current inventory count per product (gauge).
pub fn inventory_level() -> &'static IntGauge {
    static METRIC: OnceLock<IntGauge> = OnceLock::new();
    METRIC.get_or_init(|| {
        let opts = Opts::new("oz_pos_inventory_level", "Current inventory level")
            .namespace("oz_pos")
            .subsystem("inventory");
        // SAFETY: static metric name/opts are compile-time constants; construction cannot fail
        let gauge = IntGauge::with_opts(opts).expect("invalid inventory_level gauge opts");
        registry()
            .register(Box::new(gauge.clone()))
            .expect("register inventory_level gauge"); // SAFETY: static registration of a freshly-constructed metric cannot fail
        gauge
    })
}

/// Active cash session amount (gauge).
pub fn cash_session_amount() -> &'static IntGauge {
    static METRIC: OnceLock<IntGauge> = OnceLock::new();
    METRIC.get_or_init(|| {
        let opts = Opts::new(
            "oz_pos_cash_session_amount",
            "Current cash session amount in minor units",
        )
        .namespace("oz_pos")
        .subsystem("cash");
        // SAFETY: static metric name/opts are compile-time constants; construction cannot fail
        let gauge = IntGauge::with_opts(opts).expect("invalid cash_session_amount gauge opts");
        registry()
            .register(Box::new(gauge.clone()))
            .expect("register cash_session_amount gauge"); // SAFETY: static registration of a freshly-constructed metric cannot fail
        gauge
    })
}

/// Sync queue depth (gauge).
pub fn sync_queue_depth() -> &'static IntGauge {
    static METRIC: OnceLock<IntGauge> = OnceLock::new();
    METRIC.get_or_init(|| {
        let opts = Opts::new(
            "oz_pos_sync_queue_depth",
            "Number of pending sync queue items",
        )
        .namespace("oz_pos")
        .subsystem("sync");
        // SAFETY: static metric name/opts are compile-time constants; construction cannot fail
        let gauge = IntGauge::with_opts(opts).expect("invalid sync_queue_depth gauge opts");
        registry()
            .register(Box::new(gauge.clone()))
            .expect("register sync_queue_depth gauge"); // SAFETY: static registration of a freshly-constructed metric cannot fail
        gauge
    })
}

/// Barcode lookup latency (histogram in seconds).
pub fn barcode_lookup_duration() -> &'static Histogram {
    static METRIC: OnceLock<Histogram> = OnceLock::new();
    METRIC.get_or_init(|| {
        let opts = HistogramOpts::new(
            "oz_pos_barcode_lookup_duration",
            "Barcode lookup latency in seconds",
        )
        .namespace("oz_pos")
        .subsystem("db")
        .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]);
        // SAFETY: static metric name/opts are compile-time constants; construction cannot fail
        let histogram = Histogram::with_opts(opts)
            // SAFETY: static metric name/opts are compile-time constants; construction cannot fail
            .expect("invalid barcode_lookup_duration histogram opts");
        registry()
            .register(Box::new(histogram.clone()))
            .expect("register barcode_lookup_duration histogram"); // SAFETY: static registration of a freshly-constructed metric cannot fail
        histogram
    })
}

/// Transaction commit latency (histogram in seconds).
pub fn transaction_commit_duration() -> &'static Histogram {
    static METRIC: OnceLock<Histogram> = OnceLock::new();
    METRIC.get_or_init(|| {
        let opts = HistogramOpts::new(
            "oz_pos_transaction_commit_duration",
            "Transaction commit latency in seconds",
        )
        .namespace("oz_pos")
        .subsystem("db")
        .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]);
        // SAFETY: static metric name/opts are compile-time constants; construction cannot fail
        let histogram = Histogram::with_opts(opts)
            // SAFETY: static metric name/opts are compile-time constants; construction cannot fail
            .expect("invalid transaction_commit_duration histogram opts");
        registry()
            .register(Box::new(histogram.clone()))
            .expect("register transaction_commit_duration histogram"); // SAFETY: static registration of a freshly-constructed metric cannot fail
        histogram
    })
}

/// Gather all metrics as Prometheus-format text.
pub fn gather_metrics() -> String {
    use prometheus::TextEncoder;
    let encoder = TextEncoder::new();
    let metric_families = registry().gather();
    encoder
        .encode_to_string(&metric_families)
        .unwrap_or_else(|e| format!("# Error encoding metrics: {e}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gather_metrics_returns_text() {
        // Trigger lazy registration for at least one metric.
        sales_completed();
        let output = gather_metrics();
        assert!(output.contains("oz_pos_sales_completed_total"));
        assert!(output.starts_with('#'));
    }

    #[test]
    fn test_counters_are_incrementable() {
        sales_completed().inc();
        sales_completed().inc_by(5);
        assert_eq!(sales_completed().get(), 6);
    }

    #[test]
    fn test_histogram_observable() {
        barcode_lookup_duration().observe(0.001);
        barcode_lookup_duration().observe(0.002);
        let count = barcode_lookup_duration().get_sample_count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_gauge_set_and_get() {
        inventory_level().set(42);
        assert_eq!(inventory_level().get(), 42);
        inventory_level().set(0);
        assert_eq!(inventory_level().get(), 0);
    }

    #[test]
    fn test_transaction_commit_histogram_observable() {
        transaction_commit_duration().observe(0.01);
        transaction_commit_duration().observe(0.05);
        transaction_commit_duration().observe(0.1);
        let count = transaction_commit_duration().get_sample_count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_cash_and_sync_gauges_default_zero() {
        // New gauges should default to 0.
        assert_eq!(cash_session_amount().get(), 0);
        assert_eq!(sync_queue_depth().get(), 0);
        cash_session_amount().set(5000);
        sync_queue_depth().set(3);
        assert_eq!(cash_session_amount().get(), 5000);
        assert_eq!(sync_queue_depth().get(), 3);
    }
}

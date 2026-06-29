//! Prometheus metrics collection for OZ-POS.
//!
//! Gauge and counter helpers that report key business and system
//! metrics to a `/metrics` HTTP endpoint.
//!
//! Feature-gated behind `metrics` — compiled out when the feature
//! is not enabled.

#[cfg(feature = "metrics")]
pub use metrics_impl::*;

#[cfg(feature = "metrics")]
mod metrics_impl {
    use prometheus::{
        Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry,
        register_histogram, register_int_counter, register_int_gauge,
    };

    use std::sync::OnceLock;

    /// Global Prometheus registry.
    fn registry() -> &'static Registry {
        static REG: OnceLock<Registry> = OnceLock::new();
        REG.get_or_init(Registry::new)
    }

    macro_rules! opts {
        ($name:expr, $help:expr, $ns:expr, $sub:expr) => {
            Opts::new($name, $help).namespace($ns).subsystem($sub)
        };
    }

    macro_rules! hopts {
        ($name:expr, $help:expr, $ns:expr, $sub:expr, $buckets:expr) => {
            HistogramOpts::new($name, $help).namespace($ns).subsystem($sub).buckets($buckets)
        };
    }

    // ── Business metrics ──────────────────────────────────────────

    /// Total sales completed (counter).
    pub fn sales_completed() -> &'static IntCounter {
        static METRIC: OnceLock<IntCounter> = OnceLock::new();
        METRIC.get_or_init(|| {
            let o = opts!("oz_pos_sales_completed_total", "Total number of completed sales", "oz_pos", "sales");
            register_int_counter!(o, registry()).unwrap()
        })
    }

    /// Current inventory count per product (gauge).
    pub fn inventory_level() -> &'static IntGauge {
        static METRIC: OnceLock<IntGauge> = OnceLock::new();
        METRIC.get_or_init(|| {
            let o = opts!("oz_pos_inventory_level", "Current inventory level", "oz_pos", "inventory");
            register_int_gauge!(o, registry()).unwrap()
        })
    }

    /// Active cash session amount (gauge).
    pub fn cash_session_amount() -> &'static IntGauge {
        static METRIC: OnceLock<IntGauge> = OnceLock::new();
        METRIC.get_or_init(|| {
            let o = opts!("oz_pos_cash_session_amount", "Current cash session amount in minor units", "oz_pos", "cash");
            register_int_gauge!(o, registry()).unwrap()
        })
    }

    /// Sync queue depth (gauge).
    pub fn sync_queue_depth() -> &'static IntGauge {
        static METRIC: OnceLock<IntGauge> = OnceLock::new();
        METRIC.get_or_init(|| {
            let o = opts!("oz_pos_sync_queue_depth", "Number of pending sync queue items", "oz_pos", "sync");
            register_int_gauge!(o, registry()).unwrap()
        })
    }

    /// Barcode lookup latency (histogram in seconds).
    pub fn barcode_lookup_duration() -> &'static Histogram {
        static METRIC: OnceLock<Histogram> = OnceLock::new();
        METRIC.get_or_init(|| {
            let o = hopts!("oz_pos_barcode_lookup_duration", "Barcode lookup latency in seconds", "oz_pos", "db",
                vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]);
            register_histogram!(o, registry()).unwrap()
        })
    }

    /// Transaction commit latency (histogram in seconds).
    pub fn transaction_commit_duration() -> &'static Histogram {
        static METRIC: OnceLock<Histogram> = OnceLock::new();
        METRIC.get_or_init(|| {
            let o = hopts!("oz_pos_transaction_commit_duration", "Transaction commit latency in seconds", "oz_pos", "db",
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]);
        register_histogram!(o, registry()).unwrap()
        })
    }

    /// Gather all metrics as Prometheus-format text.
    pub fn gather_metrics() -> String {
        let encoder = prometheus::TextEncoder::new();
        let metric_families = registry().gather();
        encoder.encode_to_string(&metric_families).unwrap_or_else(|e| {
            format!("# Error encoding metrics: {e}\n")
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_gather_metrics_returns_text() {
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
    }
}

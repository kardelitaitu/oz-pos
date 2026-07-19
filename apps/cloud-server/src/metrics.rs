//! Prometheus metrics for the cloud sync server (P-3 Step 7).
//!
//! Exposes counters and histograms for sync push/pull performance,
//! anchor expiry events, and DB contention. All metrics are registered
//! in a default [`prometheus::Registry`] and exposed via `GET /metrics`.

use std::sync::LazyLock;

use prometheus::{
    Counter, CounterVec, Histogram, HistogramOpts, HistogramVec, Opts, Registry, TextEncoder,
};

/// The global metrics registry. All metrics defined in this module are
/// registered here during static initialisation.
static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

// ── Counters ──────────────────────────────────────────────────────────

/// Total number of items pushed to the server, labelled by outcome.
pub static SYNC_PUSHES_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    let c = CounterVec::new(
        Opts::new("sync_pushes_total", "Total items pushed to the server"),
        &["outcome"], // accepted | conflict | rejected
    )
    .unwrap();
    REGISTRY.register(Box::new(c.clone())).unwrap();
    c
});

/// Total number of anchor-expired responses returned to clients.
pub static SYNC_ANCHOR_EXPIRED_TOTAL: LazyLock<Counter> = LazyLock::new(|| {
    let c = Counter::new(
        "sync_anchor_expired_total",
        "Total anchor-expired (410 Gone) responses",
    )
    .unwrap();
    REGISTRY.register(Box::new(c.clone())).unwrap();
    c
});

// ── Histograms ────────────────────────────────────────────────────────

/// Duration of push requests in milliseconds.
pub static SYNC_PUSH_DURATION_MS: LazyLock<Histogram> = LazyLock::new(|| {
    let h = Histogram::with_opts(HistogramOpts::new(
        "sync_push_duration_ms",
        "Push handler duration in milliseconds",
    ))
    .unwrap();
    REGISTRY.register(Box::new(h.clone())).unwrap();
    h
});

/// Duration of pull requests in milliseconds.
pub static SYNC_PULL_DURATION_MS: LazyLock<Histogram> = LazyLock::new(|| {
    let h = Histogram::with_opts(HistogramOpts::new(
        "sync_pull_duration_ms",
        "Pull handler duration in milliseconds",
    ))
    .unwrap();
    REGISTRY.register(Box::new(h.clone())).unwrap();
    h
});

/// Size of push request bodies in bytes (before compression).
pub static SYNC_BATCH_SIZE_BYTES: LazyLock<Histogram> = LazyLock::new(|| {
    let h = Histogram::with_opts(HistogramOpts::new(
        "sync_batch_size_bytes",
        "Push request body size in bytes",
    ))
    .unwrap();
    REGISTRY.register(Box::new(h.clone())).unwrap();
    h
});

// ── P8-3: Health-check metrics ──────────────────────────────────────

/// Total number of health check requests served.
pub static HEALTH_CHECKS_TOTAL: LazyLock<Counter> = LazyLock::new(|| {
    let c = Counter::new("health_checks_total", "Total health check requests served").unwrap();
    REGISTRY.register(Box::new(c.clone())).unwrap();
    c
});

/// Total number of health check failures (DB unreachable).
pub static HEALTH_CHECK_FAILURES_TOTAL: LazyLock<Counter> = LazyLock::new(|| {
    let c = Counter::new(
        "health_check_failures_total",
        "Total health check requests where DB ping failed",
    )
    .unwrap();
    REGISTRY.register(Box::new(c.clone())).unwrap();
    c
});

/// Database ping latency in microseconds.
pub static HEALTH_DB_LATENCY_MICROS: LazyLock<Histogram> = LazyLock::new(|| {
    let h = Histogram::with_opts(HistogramOpts::new(
        "health_db_latency_micros",
        "Database ping latency in microseconds",
    ))
    .unwrap();
    REGISTRY.register(Box::new(h.clone())).unwrap();
    h
});

/// Duration of database lock acquisitions in seconds.
pub static DB_CONTENTION_SECONDS: LazyLock<HistogramVec> = LazyLock::new(|| {
    let h = HistogramVec::new(
        HistogramOpts::new(
            "db_connection_contention_seconds",
            "Database lock acquisition time in seconds",
        ),
        &["handler"], // push | pull | snapshot | status
    )
    .unwrap();
    REGISTRY.register(Box::new(h.clone())).unwrap();
    h
});

// ── Rendering ─────────────────────────────────────────────────────────

/// Ensure all LazyLock metrics are registered before rendering.
fn ensure_registered() {
    // Force initialisation of all lazy metrics by touching each one.
    // CounterVec metrics need at least one label value pre-created
    // otherwise they won't appear in the Prometheus text output.
    let _ = &*SYNC_PUSHES_TOTAL;
    let _ = SYNC_PUSHES_TOTAL.with_label_values(&["accepted"]);
    let _ = SYNC_PUSHES_TOTAL.with_label_values(&["conflict"]);
    let _ = SYNC_PUSHES_TOTAL.with_label_values(&["rejected"]);
    let _ = &*SYNC_ANCHOR_EXPIRED_TOTAL;
    let _ = &*SYNC_PUSH_DURATION_MS;
    let _ = &*SYNC_PULL_DURATION_MS;
    let _ = &*SYNC_BATCH_SIZE_BYTES;
    let _ = &*HEALTH_CHECKS_TOTAL;
    let _ = &*HEALTH_CHECK_FAILURES_TOTAL;
    let _ = &*HEALTH_DB_LATENCY_MICROS;
    let _ = &*DB_CONTENTION_SECONDS;
}

/// Render all registered metrics in Prometheus text format.
pub fn render_metrics() -> String {
    ensure_registered();
    let encoder = TextEncoder::new();
    encoder
        .encode_to_string(&REGISTRY.gather())
        .unwrap_or_default()
}

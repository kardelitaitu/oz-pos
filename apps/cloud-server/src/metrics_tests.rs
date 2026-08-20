//! Tests for `metrics.rs` — registration and rendering invariants.
//!
//! The metrics are static `LazyLock` declarations that self-register on
//! first access. The invariants worth pinning:
//! - `render_metrics()` emits every metric family (none silently dropped)
//! - every metric name is a valid Prometheus name (`[a-zA-Z_:][a-zA-Z0-9_:]*`)
//! - the label vectors expose their declared label dimensions

use super::*;

/// Prometheus metric name grammar: `[a-zA-Z_:][a-zA-Z0-9_:]*`.
fn is_valid_prometheus_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == ':') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

#[test]
fn render_metrics_emits_every_family() {
    let output = render_metrics();
    for name in [
        "sync_pushes_total",
        "sync_anchor_expired_total",
        "sync_pull_row_decode_failures_total",
        "prune_queue_deleted_total",
        "prune_sent_reports_deleted_total",
        "rate_limit_429_total",
        "webhook_5xx_total",
        "sync_push_duration_ms",
        "sync_pull_duration_ms",
        "sync_batch_size_bytes",
        "health_checks_total",
        "health_check_failures_total",
        "health_db_latency_micros",
        "db_connection_contention_seconds",
    ] {
        assert!(
            output.contains(name),
            "render_metrics() must emit {name} (output: {output})"
        );
    }
}

#[test]
fn every_metric_name_is_a_valid_prometheus_name() {
    let output = render_metrics();
    // Collect the `# HELP <name>` / `# TYPE <name>` declarations.
    let declared: Vec<&str> = output
        .lines()
        .filter(|l| l.starts_with("# TYPE "))
        .map(|l| {
            l.trim_start_matches("# TYPE ")
                .split_whitespace()
                .next()
                .unwrap_or("")
        })
        .filter(|n| !n.is_empty())
        .collect();
    assert!(!declared.is_empty(), "render_metrics() declares families");
    for name in &declared {
        assert!(
            is_valid_prometheus_name(name),
            "invalid Prometheus metric name: {name}"
        );
    }
}

#[test]
fn all_declared_names_are_known_families() {
    // Guard against a typo'd metric name registering silently: every
    // declared family must be one we intentionally expose.
    let known = [
        "sync_pushes_total",
        "sync_anchor_expired_total",
        "sync_pull_row_decode_failures_total",
        "prune_queue_deleted_total",
        "prune_sent_reports_deleted_total",
        "rate_limit_429_total",
        "webhook_5xx_total",
        "sync_push_duration_ms",
        "sync_pull_duration_ms",
        "sync_batch_size_bytes",
        "health_checks_total",
        "health_check_failures_total",
        "health_db_latency_micros",
        "db_connection_contention_seconds",
    ];
    let output = render_metrics();
    for line in output.lines().filter(|l| l.starts_with("# TYPE ")) {
        let name = line
            .trim_start_matches("# TYPE ")
            .split_whitespace()
            .next()
            .unwrap_or("");
        assert!(
            known.contains(&name),
            "undeclared metric family in output: {name}"
        );
    }
}

#[test]
fn rate_limit_429_exposes_both_limiter_labels() {
    // ensure_registered() pre-creates sync + token labels; both must
    // appear in the rendered output as distinct series.
    let output = render_metrics();
    assert!(
        output.contains("limiter=\"sync\""),
        "sync limiter series must render"
    );
    assert!(
        output.contains("limiter=\"token\""),
        "token limiter series must render"
    );
}

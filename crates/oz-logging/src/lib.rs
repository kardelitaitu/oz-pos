//! Structured logging facade for OZ-POS.
//!
//! `oz-logging` wraps the `tracing` ecosystem with context-tagged
//! record format, file + stdout writers, log rotation, and platform-
//! specific outputs (syslog on Linux, Event Log on Windows).
//!
//! # Initialisers
//!
//! - [`init`] — human-readable text format (stdout). Best for local dev.
//! - [`init_json`] — JSON-formatted log records (stdout). Best for
//!   production environments where logs are shipped to ELK/Loki.
//! - [`init_with_file`] — human-readable text + rolling file writer.
//! - [`init_json_with_file`] — JSON + rolling file writer.
//!
//! # Platform outputs
//!
//! - **Linux**: Syslog output is available via the `syslog` module.
//! - **Windows**: Event Log output is available via the `eventlog` module.

#![warn(missing_docs)]
// Note: unsafe blocks are permitted for platform-specific FFI
// calls (libc syslog, Windows Event Log).

pub mod error;
pub mod visitor;
#[cfg(target_os = "linux")]
pub mod syslog;
#[cfg(target_os = "windows")]
pub mod eventlog;

use tracing_subscriber::EnvFilter;
pub use error::LoggingError;

/// Initialise structured logging via `tracing-subscriber` with
/// human-readable text output to stdout.
///
/// Reads `RUST_LOG` from the environment; falls back to `info` if unset.
/// Call this once, early in `main` / `run`, before any `tracing` macro
/// is hit.
///
/// # Panics
///
/// Panics if the global subscriber has already been set.
pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

/// Initialise log output as newline-delimited JSON records (stdout).
///
/// Reads `RUST_LOG` from the environment; falls back to `info` if unset.
/// Each log line is a flat JSON object with `timestamp`, `level`,
/// `message`, and optional `fields` / `span` attributes.
///
/// Use this in production deployments where logs are shipped to
/// ELK, Loki, or Datadog.
///
/// # Panics
///
/// Panics if the global subscriber has already been set.
pub fn init_json() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_target(false)
        .flatten_event(false)
        .with_current_span(false)
        .with_span_list(false)
        .init();
}

/// Initialise human-readable log output to both stdout and a rolling
/// file writer.
///
/// The file appender rotates hourly (default) and uses the given
/// directory and file prefix for the log files. Logs older than
/// `retention_days` are automatically cleaned up.
///
/// # Panics
///
/// Panics if the global subscriber has already been set.
///
/// # Example
///
/// ```no_run
/// oz_logging::init_with_file("logs", "oz-pos", 30);
/// ```
pub fn init_with_file(log_dir: &str, file_prefix: &str, retention_days: u32) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let file_appender = tracing_appender::rolling::hourly(log_dir, file_prefix);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(non_blocking)
        .init();

    // Spawn a background task for log retention cleanup.
    if retention_days > 0 {
        let dir = log_dir.to_owned();
        let prefix = file_prefix.to_owned();
        std::thread::spawn(move || {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str())
                        && name.starts_with(&prefix)
                        && let Ok(metadata) = std::fs::metadata(&path)
                        && let Ok(modified) = metadata.modified()
                    {
                        let modified: chrono::DateTime<chrono::Utc> = modified.into();
                        if modified < cutoff {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        });
    }
}

/// Initialise JSON log output to both stdout and a rolling file writer.
///
/// Same as [`init_with_file`] but uses JSON formatting.
///
/// # Panics
///
/// Panics if the global subscriber has already been set.
pub fn init_json_with_file(log_dir: &str, file_prefix: &str, retention_days: u32) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let file_appender = tracing_appender::rolling::hourly(log_dir, file_prefix);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_target(false)
        .flatten_event(false)
        .with_current_span(false)
        .with_span_list(false)
        .with_writer(non_blocking)
        .init();

    // Spawn a background task for log retention cleanup.
    if retention_days > 0 {
        let dir = log_dir.to_owned();
        let prefix = file_prefix.to_owned();
        std::thread::spawn(move || {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str())
                        && name.starts_with(&prefix)
                        && let Ok(metadata) = std::fs::metadata(&path)
                        && let Ok(modified) = metadata.modified()
                    {
                        let modified: chrono::DateTime<chrono::Utc> = modified.into();
                        if modified < cutoff {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        });
    }
}

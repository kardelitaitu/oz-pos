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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visitor::MessageVisitor;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;

    // ── Retention cleanup ─────────────────────────────────────────

    #[test]
    fn cleanup_retention_zero_does_nothing() {
        let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("oz-pos.log");
        std::fs::write(&file_path, "test data").unwrap();

        cleanup_old_log_files(dir.to_str().unwrap(), "oz-pos", 0);

        // File should still exist (retention 0 means skip cleanup).
        assert!(file_path.exists(), "file should not be removed when retention_days is 0");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cleanup_retention_removes_old_files() {
        let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();

        // Create an old file (modification time in the past).
        let old_file = dir.join("oz-pos-old.log");
        std::fs::write(&old_file, "old data").unwrap();

        // Set modification time to 30 days ago.
        let old_time = filetime::FileTime::from_unix_time(
            chrono::Utc::now().timestamp() - 30 * 86400,
            0,
        );
        filetime::set_file_mtime(&old_file, old_time).unwrap();

        // Create a recent file (should NOT be removed).
        let new_file = dir.join("oz-pos-new.log");
        std::fs::write(&new_file, "new data").unwrap();

        cleanup_old_log_files(dir.to_str().unwrap(), "oz-pos", 7);

        assert!(!old_file.exists(), "old file should be removed");
        assert!(new_file.exists(), "new file should be kept");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cleanup_retention_skips_non_matching_prefix() {
        let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();

        let other_file = dir.join("other-app.log");
        std::fs::write(&other_file, "other data").unwrap();
        let old_time = filetime::FileTime::from_unix_time(
            chrono::Utc::now().timestamp() - 30 * 86400,
            0,
        );
        filetime::set_file_mtime(&other_file, old_time).unwrap();

        cleanup_old_log_files(dir.to_str().unwrap(), "oz-pos", 7);

        // File with non-matching prefix should be kept.
        assert!(other_file.exists(), "file with non-matching prefix should not be removed");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cleanup_retention_nonexistent_dir() {
        // Should not panic.
        cleanup_old_log_files("C:\\nonexistent_dir_xyzzy", "oz-pos", 7);
    }

    #[test]
    fn cleanup_retention_empty_dir() {
        let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();

        // Should not panic or fail on empty dir.
        cleanup_old_log_files(dir.to_str().unwrap(), "oz-pos", 7);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn logging_error_open_file_display() {
        let inner = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = LoggingError::OpenFile(inner);
        assert!(err.to_string().contains("could not open log file"));
        assert!(err.to_string().contains("access denied"));
    }

    #[test]
    fn logging_error_invalid_level_display() {
        let err = LoggingError::InvalidLevel("bogus".into());
        assert_eq!(err.to_string(), "invalid log level: bogus");
    }

    #[test]
    fn logging_error_is_debug() {
        let err = LoggingError::InvalidLevel("x".into());
        assert!(!format!("{err:?}").is_empty());
    }

    /// A test layer that captures the last event's fields via MessageVisitor.
    struct CaptureLayer(Arc<Mutex<String>>);

    impl<S: tracing::Subscriber + for<'a> LookupSpan<'a>> Layer<S> for CaptureLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let mut msg = String::new();
            let mut visitor = MessageVisitor(&mut msg);
            event.record(&mut visitor);
            let mut guard = self.0.lock().unwrap();
            *guard = msg;
        }
    }

    fn capture_event<F>(f: F) -> String
    where
        F: Fn(),
    {
        let buf = Arc::new(Mutex::new(String::new()));
        let layer = CaptureLayer(buf.clone());

        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, f);

        let guard = buf.lock().unwrap();
        guard.clone()
    }

    #[test]
    fn message_visitor_records_message_field() {
        let output = capture_event(|| {
            tracing::event!(tracing::Level::INFO, "hello world");
        });
        assert_eq!(output, "hello world");
    }

    #[test]
    fn message_visitor_records_other_fields_as_pairs() {
        let output = capture_event(|| {
            tracing::event!(
                tracing::Level::INFO,
                message = "processing",
                sku = "ABC-123"
            );
        });
        assert!(output.contains("sku=ABC-123"));
    }

    #[test]
    fn message_visitor_records_i64_field() {
        let output = capture_event(|| {
            tracing::event!(tracing::Level::INFO, qty = 42);
        });
        assert!(output.contains("qty=42"));
    }

    #[test]
    fn message_visitor_records_bool_field() {
        let output = capture_event(|| {
            tracing::event!(tracing::Level::INFO, active = true);
        });
        assert!(output.contains("active=true"));
    }

    #[test]
    fn message_visitor_combines_fields() {
        let output = capture_event(|| {
            tracing::event!(
                tracing::Level::INFO,
                message = "stock adjusted",
                sku = "XYZ"
            );
        });
        assert!(output.contains("stock adjusted"));
        assert!(output.contains("sku=XYZ"));
    }
}

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

/// Remove log files in `dir` that start with `file_prefix` and whose
/// modification time is older than `retention_days`.
fn cleanup_old_log_files(dir: &str, file_prefix: &str, retention_days: u32) {
    if retention_days == 0 {
        return;
    }
    let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with(file_prefix)
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
    let dir = log_dir.to_owned();
    let prefix = file_prefix.to_owned();
    std::thread::spawn(move || {
        cleanup_old_log_files(&dir, &prefix, retention_days);
    });
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
    let dir = log_dir.to_owned();
    let prefix = file_prefix.to_owned();
    std::thread::spawn(move || {
        cleanup_old_log_files(&dir, &prefix, retention_days);
    });
}

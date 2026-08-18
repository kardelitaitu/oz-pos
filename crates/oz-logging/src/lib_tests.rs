
use super::*;
use crate::visitor::MessageVisitor;
use std::sync::{Arc, Mutex};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

// ── Retention cleanup ─────────────────────────────────────────

#[test]
fn cleanup_retention_zero_does_nothing() {
    let dir = std::env::temp_dir().join(uuid::Uuid::now_v7().to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let file_path = dir.join("oz-pos.log");
    std::fs::write(&file_path, "test data").unwrap();

    cleanup_old_log_files(dir.to_str().unwrap(), "oz-pos", 0);

    // File should still exist (retention 0 means skip cleanup).
    assert!(
        file_path.exists(),
        "file should not be removed when retention_days is 0"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cleanup_retention_removes_old_files() {
    let dir = std::env::temp_dir().join(uuid::Uuid::now_v7().to_string());
    std::fs::create_dir_all(&dir).unwrap();

    // Create an old file (modification time in the past).
    let old_file = dir.join("oz-pos-old.log");
    std::fs::write(&old_file, "old data").unwrap();

    // Set modification time to 30 days ago.
    let old_time =
        filetime::FileTime::from_unix_time(chrono::Utc::now().timestamp() - 30 * 86400, 0);
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
    let dir = std::env::temp_dir().join(uuid::Uuid::now_v7().to_string());
    std::fs::create_dir_all(&dir).unwrap();

    let other_file = dir.join("other-app.log");
    std::fs::write(&other_file, "other data").unwrap();
    let old_time =
        filetime::FileTime::from_unix_time(chrono::Utc::now().timestamp() - 30 * 86400, 0);
    filetime::set_file_mtime(&other_file, old_time).unwrap();

    cleanup_old_log_files(dir.to_str().unwrap(), "oz-pos", 7);

    // File with non-matching prefix should be kept.
    assert!(
        other_file.exists(),
        "file with non-matching prefix should not be removed"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cleanup_retention_nonexistent_dir() {
    // Should not panic.
    cleanup_old_log_files("C:\\nonexistent_dir_xyzzy", "oz-pos", 7);
}

#[test]
fn cleanup_retention_empty_dir() {
    let dir = std::env::temp_dir().join(uuid::Uuid::now_v7().to_string());
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

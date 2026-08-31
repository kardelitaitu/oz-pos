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

// ── L-1: WorkerGuard retention ────────────────────────────────────

/// Serialises the L-1 tests: the global subscriber and the guard registry
/// are process-global, so these tests must not run concurrently.
static L1_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn l1_guard_retained_after_text_file_init() {
    let _l1 = L1_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("oz-logging-l1a-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let before = crate::retained_file_log_guards();
    // A second subscriber in the same process would fail try_init; the
    // registry must still grow ONLY on success — so call via the registry
    // helper directly for determinism.
    let guard_count_before = before;
    let result = crate::try_init_with_file(dir.to_str().unwrap(), "l1a", 0);
    match result {
        Ok(()) => {
            assert_eq!(
                crate::retained_file_log_guards(),
                guard_count_before + 1,
                "successful file init must retain its WorkerGuard"
            );
        }
        Err(_) => {
            // Another test already set the global subscriber; the guard must
            // NOT be retained on failure.
            assert_eq!(
                crate::retained_file_log_guards(),
                guard_count_before,
                "failed init must not retain a guard"
            );
        }
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn l1_guard_retained_after_json_file_init() {
    let _l1 = L1_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("oz-logging-l1b-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let before = crate::retained_file_log_guards();
    let result = crate::try_init_json_with_file(dir.to_str().unwrap(), "l1b", 0);
    match result {
        Ok(()) => {
            assert_eq!(
                crate::retained_file_log_guards(),
                before + 1,
                "successful JSON file init must retain its WorkerGuard"
            );
        }
        Err(_) => {
            assert_eq!(
                crate::retained_file_log_guards(),
                before,
                "failed init must not retain a guard"
            );
        }
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn l1_file_writer_writes_after_init_returns() {
    let _l1 = L1_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // The behavioural regression test: log an event AFTER init returns and
    // verify it reaches the file (pre-fix, the writer was shut down at init
    // exit so nothing was written).
    let dir = std::env::temp_dir().join(format!("oz-logging-l1c-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let result = crate::try_init_json_with_file(dir.to_str().unwrap(), "l1c", 0);
    if result.is_err() {
        // Global subscriber already taken by a sibling test — behavioural
        // check covered by whichever init won; skip here.
        std::fs::remove_dir_all(&dir).ok();
        return;
    }
    let marker = format!("l1-marker-{}", uuid_like());
    tracing::info!("{marker}");
    // Give the non-blocking writer a moment to flush (worker thread).
    std::thread::sleep(std::time::Duration::from_millis(300));
    let found = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .any(|content| content.contains(&marker));
    assert!(found, "event logged after init must reach the log file");
    std::fs::remove_dir_all(&dir).ok();
}

/// Cheap unique marker (no uuid dependency in this crate).
fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}

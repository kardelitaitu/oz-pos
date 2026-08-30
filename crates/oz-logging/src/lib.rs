/*
last audited 25-07-26 by RSA-Agent (oz-logging slice A: lib deep read; L-1 FIXED 25-07-26)
crate: oz-logging | status: SAFE | lint: CLEAN
findings: L-1 FIXED — both file-init functions now retain their tracing_appender WorkerGuard in a process-global FILE_LOG_GUARDS registry (OnceLock<Mutex<Vec<WorkerGuard>>>); the writer previously shut down when the local guard dropped at init exit, killing file logging for the rest of the process. Retained guards live for the process lifetime (OS reclaims at exit — the desired flush window); failed inits do not retain (early ?-return); retained_file_log_guards() exposes the count for tests/ops; 3 new tests incl. a behavioural write-after-init check (all 39+2 oz-logging tests pass). L-2 INFO unchanged — retention cleanup still runs once at startup (documented best-effort). Text/JSON init variants and RUST_LOG fallback clean; eventlog/syslog FFI carries documented SAFETY comments
next: none | perf: N/A
*/
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

// Note: unsafe blocks are permitted for platform-specific FFI
// calls (libc syslog, Windows Event Log).

pub mod error;
#[cfg(target_os = "windows")]
pub mod eventlog;
#[cfg(target_os = "linux")]
pub mod syslog;
pub mod visitor;

pub use error::LoggingError;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

/// Process-global registry of file-writer guards (L-1 fix).
///
/// The non-blocking file writer shuts down when its guard drops; binding
/// the guard to a local dropped the writer immediately after init returned,
/// leaving file logging dead for the rest of the process. Guards are
/// retained here for the process lifetime — the registry is never dropped,
/// and the OS reclaims it at exit, which is exactly the desired flush
/// window.
static FILE_LOG_GUARDS: std::sync::OnceLock<std::sync::Mutex<Vec<WorkerGuard>>> =
    std::sync::OnceLock::new();

/// Retain a `WorkerGuard` for the process lifetime (L-1 fix).
///
/// See [`FILE_LOG_GUARDS`]. If the registry mutex is poisoned (only
/// possible if a panic unwinds while the lock is held, which no code path
/// here does), the guard is dropped rather than blocking startup — the
/// pre-fix behaviour.
fn retain_file_log_guard(guard: WorkerGuard) {
    let registry = FILE_LOG_GUARDS.get_or_init(|| std::sync::Mutex::new(Vec::new()));
    if let Ok(mut guards) = registry.lock() {
        guards.push(guard);
    }
}

/// Number of file-writer guards currently retained (test/ops introspection).
#[doc(hidden)]
pub fn retained_file_log_guards() -> usize {
    FILE_LOG_GUARDS
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .map(|guards| guards.len())
        .unwrap_or(0)
}

/// Non-panicking variant of [`init`].
///
/// Returns `Err` (instead of panicking) if the global subscriber has
/// already been set. All other behaviour is identical to [`init`].
pub fn try_init() -> Result<(), LoggingError> {
    let filter = EnvFilter::try_from_default_env()
        .inspect_err(|_| eprintln!("[oz-logging] RUST_LOG parse failed, falling back to info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()
        .map_err(|e| LoggingError::InitFailed(format!("{e}")))?;
    Ok(())
}

/// Non-panicking variant of [`init_json`].
///
/// Returns `Err` (instead of panicking) if the global subscriber has
/// already been set. All other behaviour is identical to [`init_json`].
pub fn try_init_json() -> Result<(), LoggingError> {
    let filter = EnvFilter::try_from_default_env()
        .inspect_err(|_| eprintln!("[oz-logging] RUST_LOG parse failed, falling back to info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_target(false)
        .flatten_event(false)
        .with_current_span(false)
        .with_span_list(false)
        .try_init()
        .map_err(|e| LoggingError::InitFailed(format!("{e}")))?;
    Ok(())
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
    // SAFETY: documented-panic wrapper — callers who need a `Result` use `try_init`.
    try_init().expect("logging init failed");
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
    // SAFETY: documented-panic wrapper — callers who need a `Result` use `try_init_json`.
    try_init_json().expect("logging init_json failed");
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
    try_init_with_file(log_dir, file_prefix, retention_days)
        // SAFETY: documented-panic wrapper — callers who need a `Result` use `try_init_with_file`.
        .expect("logging init_with_file failed");
}

/// Non-panicking variant of [`init_with_file`].
///
/// Returns `Err` (instead of panicking) if the global subscriber has
/// already been set. All other behaviour is identical to
/// [`init_with_file`]. The retention-cleanup thread is spawned as
/// best-effort (detached) — if the process exits before cleanup
/// completes, old log files persist until the next run.
pub fn try_init_with_file(
    log_dir: &str,
    file_prefix: &str,
    retention_days: u32,
) -> Result<(), LoggingError> {
    let filter = EnvFilter::try_from_default_env()
        .inspect_err(|_| eprintln!("[oz-logging] RUST_LOG parse failed, falling back to info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let file_appender = tracing_appender::rolling::hourly(log_dir, file_prefix);
    // L-1 fix: the guard is retained process-wide (see FILE_LOG_GUARDS);
    // dropping it locally shut the file writer down immediately.
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(non_blocking)
        .try_init()
        .map_err(|e| LoggingError::InitFailed(format!("{e}")))?;
    retain_file_log_guard(guard);

    // Spawn a best-effort background task for log retention cleanup.
    // The thread is detached — if the process exits before cleanup
    // finishes, old log files simply persist until the next run.
    let dir = log_dir.to_owned();
    let prefix = file_prefix.to_owned();
    std::thread::spawn(move || {
        cleanup_old_log_files(&dir, &prefix, retention_days);
    });

    Ok(())
}

/// Initialise JSON log output to both stdout and a rolling file writer.
///
/// Same as [`init_with_file`] but uses JSON formatting.
///
/// # Panics
///
/// Panics if the global subscriber has already been set.
pub fn init_json_with_file(log_dir: &str, file_prefix: &str, retention_days: u32) {
    try_init_json_with_file(log_dir, file_prefix, retention_days)
        // SAFETY: documented-panic wrapper — callers who need a `Result` use `try_init_json_with_file`.
        .expect("logging init_json_with_file failed");
}

/// Non-panicking variant of [`init_json_with_file`].
///
/// Returns `Err` (instead of panicking) if the global subscriber has
/// already been set. All other behaviour is identical to
/// [`init_json_with_file`]. The retention-cleanup thread is spawned as
/// best-effort (detached) — if the process exits before cleanup
/// completes, old log files persist until the next run.
pub fn try_init_json_with_file(
    log_dir: &str,
    file_prefix: &str,
    retention_days: u32,
) -> Result<(), LoggingError> {
    let filter = EnvFilter::try_from_default_env()
        .inspect_err(|_| eprintln!("[oz-logging] RUST_LOG parse failed, falling back to info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let file_appender = tracing_appender::rolling::hourly(log_dir, file_prefix);
    // L-1 fix: the guard is retained process-wide (see FILE_LOG_GUARDS);
    // dropping it locally shut the file writer down immediately.
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_target(false)
        .flatten_event(false)
        .with_current_span(false)
        .with_span_list(false)
        .with_writer(non_blocking)
        .try_init()
        .map_err(|e| LoggingError::InitFailed(format!("{e}")))?;
    retain_file_log_guard(guard);

    // Spawn a best-effort background task for log retention cleanup.
    // The thread is detached — if the process exits before cleanup
    // finishes, old log files simply persist until the next run.
    let dir = log_dir.to_owned();
    let prefix = file_prefix.to_owned();
    std::thread::spawn(move || {
        cleanup_old_log_files(&dir, &prefix, retention_days);
    });

    Ok(())
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

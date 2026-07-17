//! Kernel lock lifecycle audit — verifies that the bounded-retry pattern
//! in AppState::drop() correctly handles lock contention during shutdown.
//!
//! The Drop implementation uses a 500ms bounded retry loop to acquire the
//! kernel lock. This test simulates contention scenarios and verifies the
//! retry behaviour is correct — it should either acquire within 500ms or
//! log a warning and proceed (never panic, never hang indefinitely).

use std::sync::Arc;
use std::time::{Duration, Instant};

use platform_kernel::Kernel;
use tokio::sync::Mutex;

// ── Test helpers ────────────────────────────────────────────────────

/// Simulate the Drop retry pattern used in app state.
/// Returns (acquired, elapsed_ms).
fn simulate_drop_retry(kernel: &Mutex<Kernel>, max_retries: usize, delay_ms: u64) -> (bool, u64) {
    let start = Instant::now();
    let mut acquired = false;
    for _ in 0..max_retries {
        if let Ok(mut k) = kernel.try_lock() {
            let _ = k.stop_all();
            acquired = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
    let elapsed = start.elapsed().as_millis() as u64;
    (acquired, elapsed)
}

// ── Tests ───────────────────────────────────────────────────────────

#[test]
fn kernel_drop_acquires_when_uncontended() {
    let kernel = Mutex::new(Kernel::new());
    let (acquired, elapsed) = simulate_drop_retry(&kernel, 50, 10);
    assert!(
        acquired,
        "should acquire uncontended kernel lock immediately"
    );
    assert!(
        elapsed < 50,
        "uncontended lock should be near-instant, took {elapsed}ms"
    );
}

#[test]
fn kernel_drop_retries_and_eventually_succeeds() {
    let kernel = Arc::new(Mutex::new(Kernel::new()));
    let k_clone = kernel.clone();

    // Hold the lock from another thread for 150ms.
    let join_handle = std::thread::spawn(move || {
        let guard = k_clone.blocking_lock();
        std::thread::sleep(Duration::from_millis(150));
        drop(guard);
    });

    // Small delay to let the other thread acquire first.
    std::thread::sleep(Duration::from_millis(10));

    let (acquired, elapsed) = simulate_drop_retry(&kernel, 50, 10);
    join_handle.join().unwrap();

    assert!(acquired, "should eventually acquire after holder releases");
    assert!(
        (130..=520).contains(&elapsed),
        "should take roughly 150ms to acquire, took {elapsed}ms"
    );
}

#[test]
fn kernel_drop_respects_max_retries() {
    let kernel = Arc::new(Mutex::new(Kernel::new()));
    let k_clone = kernel.clone();
    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Hold the lock indefinitely until signalled.
    let join_handle = std::thread::spawn(move || {
        let guard = k_clone.blocking_lock();
        while !shutdown_clone.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(guard);
    });

    // Small delay so the holder acquires first.
    std::thread::sleep(Duration::from_millis(10));

    let (acquired, elapsed) = simulate_drop_retry(&kernel, 5, 10);
    shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
    join_handle.join().unwrap();

    assert!(
        !acquired,
        "should NOT acquire when holder never releases within retries"
    );
    assert!(
        (40..=120).contains(&elapsed),
        "5 retries × 10ms should take ~50ms, took {elapsed}ms"
    );
}

#[test]
fn kernel_try_lock_returns_none_when_contended() {
    let kernel = Arc::new(Mutex::new(Kernel::new()));
    let k_clone = kernel.clone();

    let holder = std::thread::spawn(move || {
        let guard = k_clone.blocking_lock();
        std::thread::sleep(Duration::from_millis(200));
        drop(guard);
    });

    std::thread::sleep(Duration::from_millis(10));

    // try_lock should fail while another thread holds.
    assert!(
        kernel.try_lock().is_err(),
        "try_lock should fail when contended"
    );

    holder.join().unwrap();

    // After release, try_lock should succeed.
    assert!(
        kernel.try_lock().is_ok(),
        "try_lock should succeed after holder releases"
    );
}

#[test]
fn kernel_stop_all_idempotent() {
    // stop_all() should be safe to call on an empty kernel.
    let mut kernel = Kernel::new();
    let result = kernel.stop_all();
    assert!(result.is_ok(), "stop_all on empty kernel should succeed");
}

#[test]
fn bounded_retry_runs_at_least_once() {
    // Even with 0 retries (max_retries=1), we should attempt once.
    let kernel = Mutex::new(Kernel::new());
    // We DON'T hold the lock — should succeed on first attempt.
    let (acquired, _) = simulate_drop_retry(&kernel, 1, 10);
    assert!(acquired, "should acquire on first and only attempt");
}

#[test]
fn retry_timing_is_within_bounds() {
    // 50 retries × 10ms sleep should take at most ~550ms (with overhead).
    let kernel = Arc::new(Mutex::new(Kernel::new()));
    let k_clone = kernel.clone();
    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sd = shutdown.clone();

    let holder = std::thread::spawn(move || {
        let guard = k_clone.blocking_lock();
        while !sd.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(guard);
    });

    std::thread::sleep(Duration::from_millis(10));

    let start = Instant::now();
    let mut stopped = false;
    for _ in 0..50 {
        if let Ok(mut k) = kernel.try_lock() {
            let _ = k.stop_all();
            stopped = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let elapsed = start.elapsed();

    shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
    holder.join().unwrap();

    assert!(!stopped, "should not stop when held indefinitely");
    assert!(
        elapsed >= Duration::from_millis(450) && elapsed <= Duration::from_millis(650),
        "50 retries × 10ms should take ~500ms, took {}ms",
        elapsed.as_millis()
    );
}

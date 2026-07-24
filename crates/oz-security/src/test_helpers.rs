//! Shared helpers for tests that touch the OS credential stores.
//!
//! These helpers are only compiled when running tests.

use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Generate a unique test credential name.
///
/// Combines the given prefix, the current process id, and an atomic
/// counter so parallel nextest threads never collide on the same
/// keyring item.
pub fn unique_test_name(prefix: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{n}", std::process::id())
}

//! Sliding-window rate limiter for login PIN attempts.
//!
//! Tracks failed PIN attempts per username. After [`LoginRateLimiter::max_attempts`]
//! failures within [`LoginRateLimiter::window_secs`], the username is locked out
//! until the oldest attempt falls outside the window.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Sliding-window rate limiter for login PIN attempts.
///
/// Tracks failed attempts per username. When the number of attempts within
/// the configured window reaches the maximum, the caller is locked out until
/// the oldest attempt falls outside the window.
///
/// Lock is never held across `.await` points.
pub struct LoginRateLimiter {
    /// Per-username list of attempt timestamps (oldest first after pruning).
    attempts: Mutex<HashMap<String, Vec<Instant>>>,
    /// Maximum failed attempts within the sliding window before lockout.
    max_attempts: usize,
    /// Sliding window duration in seconds.
    window_secs: u64,
}

impl LoginRateLimiter {
    /// Create a new rate limiter.
    ///
    /// * `max_attempts` — number of failed attempts allowed within the window.
    /// * `window_secs` — sliding window duration in seconds.
    #[must_use]
    pub fn new(max_attempts: usize, window_secs: u64) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max_attempts,
            window_secs,
        }
    }

    /// Record a failed PIN attempt for `username`.
    ///
    /// # Returns
    ///
    /// * `Ok(remaining)` — the number of attempts remaining before lockout.
    /// * `Err(retry_after_secs)` — the caller is locked out; must wait this
    ///   many seconds before trying again.
    pub fn record_failure(&self, username: &str) -> Result<usize, u64> {
        let mut map = self.attempts.lock().expect("rate limiter lock poisoned");
        let now = Instant::now();
        let window = Duration::from_secs(self.window_secs);
        let attempts = map.entry(username.to_string()).or_default();

        // Prune entries whose window has expired.
        attempts.retain(|t| now.duration_since(*t) < window);

        // Check lockout BEFORE recording.
        if attempts.len() >= self.max_attempts {
            let oldest = attempts[0];
            let elapsed = now.duration_since(oldest).as_secs();
            let retry_after = self.window_secs.saturating_sub(elapsed);
            return Err(retry_after.max(1));
        }

        // Record this attempt.
        attempts.push(now);

        // Check if this attempt pushed us over the limit.
        if attempts.len() >= self.max_attempts {
            return Err(self.window_secs);
        }

        let remaining = self.max_attempts.saturating_sub(attempts.len());
        Ok(remaining)
    }

    /// Reset the attempt counter for `username` (call on successful login).
    pub fn reset(&self, username: &str) {
        if let Ok(mut map) = self.attempts.lock() {
            map.remove(username);
        }
    }

    /// Clear all records (for testing or admin reset).
    pub fn clear(&self) {
        if let Ok(mut map) = self.attempts.lock() {
            map.clear();
        }
    }
}

impl Default for LoginRateLimiter {
    /// Default: 3 attempts per 60-second sliding window.
    fn default() -> Self {
        Self::new(3, 60)
    }
}

impl std::fmt::Debug for LoginRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginRateLimiter")
            .field("max_attempts", &self.max_attempts)
            .field("window_secs", &self.window_secs)
            .field("attempts", &"(locked)")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_max_attempts() {
        let limiter = LoginRateLimiter::new(3, 60);
        assert_eq!(limiter.record_failure("alice").unwrap(), 2);
        assert_eq!(limiter.record_failure("alice").unwrap(), 1);
    }

    #[test]
    fn locks_out_after_max_attempts() {
        let limiter = LoginRateLimiter::new(3, 60);
        limiter.record_failure("alice").ok();
        limiter.record_failure("alice").ok();
        // Third attempt triggers lockout.
        let result = limiter.record_failure("alice");
        assert!(result.is_err());
        // Fourth attempt is also locked out.
        let result = limiter.record_failure("alice");
        assert!(result.is_err());
    }

    #[test]
    fn lockout_has_reasonable_duration() {
        let limiter = LoginRateLimiter::new(3, 60);
        limiter.record_failure("alice").ok();
        limiter.record_failure("alice").ok();
        let result = limiter.record_failure("alice");
        let err = result.unwrap_err();
        assert!(err <= 60, "lockout should not exceed window");
        assert!(err >= 1, "lockout should be at least 1 second");
    }

    #[test]
    fn reset_clears_attempts() {
        let limiter = LoginRateLimiter::new(3, 60);
        limiter.record_failure("alice").ok();
        limiter.record_failure("alice").ok();
        limiter.reset("alice");
        assert_eq!(limiter.record_failure("alice").unwrap(), 2);
    }

    #[test]
    fn different_usernames_independent() {
        let limiter = LoginRateLimiter::new(3, 60);
        limiter.record_failure("alice").ok();
        limiter.record_failure("alice").ok();
        assert!(limiter.record_failure("alice").is_err()); // locked out
        // bob still has full quota
        assert_eq!(limiter.record_failure("bob").unwrap(), 2);
    }

    #[test]
    fn clear_resets_all() {
        let limiter = LoginRateLimiter::new(3, 60);
        limiter.record_failure("alice").ok();
        limiter.record_failure("bob").ok();
        limiter.record_failure("alice").ok();
        limiter.clear();
        assert_eq!(limiter.record_failure("alice").unwrap(), 2);
        assert_eq!(limiter.record_failure("bob").unwrap(), 2);
    }
}

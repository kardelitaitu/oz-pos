//! Sliding-window rate limiter for login PIN attempts.
//!
//! Tracks failed PIN attempts per username. After `LoginRateLimiter::max_attempts`
//! failures within `LoginRateLimiter::window_secs`, the username is locked out
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
        let mut map = self
            .attempts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Instant::now();
        let window = Duration::from_secs(self.window_secs);
        let attempts = map.entry(username.to_string()).or_default();

        // Prune entries whose window has expired.
        attempts.retain(|t| now.duration_since(*t) < window);

        // Check lockout BEFORE recording.
        //
        // Guard against empty vec when max_attempts is 0 — the caller is
        // always locked out so we return the full window duration.
        if attempts.len() >= self.max_attempts {
            if attempts.is_empty() {
                return Err(self.window_secs.max(1));
            }
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

    // ── Edge-case: remaining count ─────────────────────────────────

    #[test]
    fn remaining_count_decrements_correctly() {
        let limiter = LoginRateLimiter::new(5, 60);
        assert_eq!(limiter.record_failure("alice").unwrap(), 4);
        assert_eq!(limiter.record_failure("alice").unwrap(), 3);
        assert_eq!(limiter.record_failure("alice").unwrap(), 2);
        assert_eq!(limiter.record_failure("alice").unwrap(), 1);
        // Fifth attempt hits the lockout boundary.
        assert!(limiter.record_failure("alice").is_err());
    }

    // ── Edge-case: zero max_attempts always locked ─────────────────

    #[test]
    fn zero_max_attempts_always_locked() {
        let limiter = LoginRateLimiter::new(0, 60);
        // Even a single attempt should be rejected — zero tolerance.
        let result = limiter.record_failure("alice");
        assert!(result.is_err());
    }

    // ── Edge-case: single-attempt window ───────────────────────────
    //
    // With max_attempts=1, the Nth attempt (the first) is recorded but
    // immediately triggers lockout: the len check fires after push.

    #[test]
    fn single_attempt_locks_out() {
        let limiter = LoginRateLimiter::new(1, 60);
        // First attempt is recorded, then immediately locked out.
        assert!(limiter.record_failure("alice").is_err());
        // Second attempt is also locked out (step-2 check catches before push).
        assert!(limiter.record_failure("alice").is_err());
    }

    // ── Edge-case: serialized fail → reset → fail again ──────────

    #[test]
    fn fail_reset_fail_cycle() {
        let limiter = LoginRateLimiter::new(2, 60);
        limiter.record_failure("alice").ok();
        limiter.record_failure("alice").ok();
        assert!(limiter.record_failure("alice").is_err());
        limiter.reset("alice");
        assert_eq!(limiter.record_failure("alice").unwrap(), 1);
        // Should lock out again after 2 more.
        assert!(limiter.record_failure("alice").is_err());
    }

    // ── Edge-case: record_failure on never-failed user ────────────

    #[test]
    fn first_attempt_returns_max_minus_one() {
        let limiter = LoginRateLimiter::new(3, 60);
        assert_eq!(limiter.record_failure("new-user").unwrap(), 2);
    }

    // ── Edge-case: Debug output does not leak timestamps ──────────

    #[test]
    fn debug_format_does_not_leak_attempts() {
        let limiter = LoginRateLimiter::new(3, 5);
        limiter.record_failure("alice").ok();
        let debug = format!("{limiter:?}");
        assert!(debug.contains("max_attempts: 3"));
        assert!(debug.contains("window_secs: 5"));
        assert!(debug.contains("(locked)"));
        // Should NOT contain timestamps or attempt details.
        assert!(!debug.contains("Instant"));
        assert!(!debug.contains("alice"));
    }

    // ── Edge-case: default parameters ──────────────────────────────

    #[test]
    fn default_limiter_uses_3_per_60() {
        let limiter = LoginRateLimiter::default();
        assert_eq!(limiter.record_failure("alice").unwrap(), 2);
        assert_eq!(limiter.record_failure("alice").unwrap(), 1);
        assert!(limiter.record_failure("alice").is_err());
    }

    // ── First attempt recorded after reset does NOT inherit old state ─

    #[test]
    fn reset_removes_user_completely() {
        let limiter = LoginRateLimiter::new(3, 60);
        limiter.record_failure("alice").ok();
        limiter.reset("alice");
        // The internal HashMap should have no entry for alice.
        {
            let map = limiter.attempts.lock().unwrap();
            assert!(!map.contains_key("alice"));
        }
    }

    // ── Lockout does NOT record the rejected attempt ───────────────
    //
    // When a user is locked out, the rejected attempt should NOT be
    // appended, so the retry_after value is based on the original set.
    // This means after the window expires, the user has fewer than
    // max_attempts remaining (the rejected ones are not counted).

    #[test]
    fn lockout_does_not_count_rejected_attempts() {
        let limiter = LoginRateLimiter::new(2, 3600); // 1-hour window
        limiter.record_failure("alice").ok();
        limiter.record_failure("alice").ok();
        // Locked out — this attempt is rejected and NOT recorded.
        assert!(limiter.record_failure("alice").is_err());
        assert!(limiter.record_failure("alice").is_err());
        // Internal count should still be exactly 2.
        {
            let map = limiter.attempts.lock().unwrap();
            let attempts = map.get("alice").unwrap();
            assert_eq!(attempts.len(), 2, "rejected attempts must not be appended");
        }
    }

    // ── Empty-string usernames are valid keys ─────────────────────

    #[test]
    fn empty_username_is_independent_key() {
        let limiter = LoginRateLimiter::new(2, 60);
        limiter.record_failure("").ok();
        // Second attempt with empty username triggers lockout (max=2).
        assert!(limiter.record_failure("").is_err());
        // 'alice' should still have full quota — independent key space.
        assert_eq!(limiter.record_failure("alice").unwrap(), 1);
    }
}

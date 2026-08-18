
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

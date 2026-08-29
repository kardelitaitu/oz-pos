package main

// Tests for the escalating brute-force login lockout (login_lockout.go):
// the cooldown formula, lockout/clear behavior, and per-key independence.

import (
	"testing"
	"time"
)

func TestLoginLockout_Formula(t *testing.T) {
	cases := []struct {
		failures int
		want     time.Duration
	}{
		{0, 5 * time.Second},                   // floor even at 0
		{1, 5 * time.Second},                   // 1st failure → 5s
		{2, 5 * time.Second},                   // 2nd failure → 5s
		{3, 35 * time.Second},                  // 3rd → +30s → 35s
		{4, 65 * time.Second},                  // 4th → +60s → 65s
		{5, 95 * time.Second},                  // 5th → +90s → 95s
		{10, 5*time.Second + 8*30*time.Second}, // 10th → 5 + 8×30 = 245s
		{32, 15 * time.Minute},                 // capped at 15 min
		{100, 15 * time.Minute},                // stays capped
	}
	for _, c := range cases {
		got := loginLockout(c.failures)
		if got != c.want {
			t.Errorf("loginLockout(%d) = %v, want %v", c.failures, got, c.want)
		}
	}
}

func TestLoginLockout_Cap(t *testing.T) {
	if loginLockout(32) != 15*time.Minute {
		t.Error("expected cap at 15 minutes for 32 failures")
	}
	if loginLockout(1000) != 15*time.Minute {
		t.Error("expected cap to hold for very large failure counts")
	}
}

func TestLoginLockout_LockoutThenClear(t *testing.T) {
	tracker := loginLockoutTrackerInst
	tracker.clearKey("email:lockout@test.com")

	// 3 failures → locked.
	for i := 0; i < 3; i++ {
		tracker.recordFailure("email:lockout@test.com")
	}
	locked, remaining := tracker.isLocked("email:lockout@test.com")
	if !locked {
		t.Fatal("expected locked after 3 failures")
	}
	if remaining <= 0 || remaining > 15*time.Minute {
		t.Errorf("unexpected remaining lockout: %v", remaining)
	}

	// Clear on success.
	tracker.clearKey("email:lockout@test.com")
	locked, _ = tracker.isLocked("email:lockout@test.com")
	if locked {
		t.Error("expected unlocked after clear")
	}
}

func TestLoginLockout_MinimumGap(t *testing.T) {
	tracker := loginLockoutTrackerInst
	tracker.clearKey("email:gap@test.com")

	// Two rapid failures still enforce the 5s minimum gap even below
	// the escalation threshold.
	tracker.recordFailure("email:gap@test.com")
	locked, remaining := tracker.isLocked("email:gap@test.com")
	if !locked {
		t.Error("expected the 5s minimum gap to lock an immediate retry")
	}
	if remaining > 5*time.Second {
		t.Errorf("expected gap ≤ 5s, got %v", remaining)
	}
	tracker.clearKey("email:gap@test.com")
}

func TestLoginLockout_PerKeyIndependence(t *testing.T) {
	tracker := loginLockoutTrackerInst
	tracker.clearKey("email:a@test.com")
	tracker.clearKey("email:b@test.com")

	for i := 0; i < 3; i++ {
		tracker.recordFailure("email:a@test.com")
	}
	// Key b should NOT be locked by a's failures.
	locked, _ := tracker.isLocked("email:b@test.com")
	if locked {
		t.Error("key b should not be locked by key a's failures")
	}
	// Key a IS locked.
	locked, _ = tracker.isLocked("email:a@test.com")
	if !locked {
		t.Error("key a should be locked")
	}
	tracker.clearKey("email:a@test.com")
	tracker.clearKey("email:b@test.com")
}

func TestLoginLockout_CooldownEscalatesWithMoreFailures(t *testing.T) {
	tracker := loginLockoutTrackerInst
	tracker.clearKey("email:esc@test.com")

	for i := 0; i < 3; i++ {
		tracker.recordFailure("email:esc@test.com")
	}
	_, rem3 := tracker.isLocked("email:esc@test.com")

	// 5 failures → longer lockout than 3 failures.
	for i := 0; i < 2; i++ {
		tracker.recordFailure("email:esc@test.com")
	}
	_, rem5 := tracker.isLocked("email:esc@test.com")

	if rem5 <= rem3 {
		t.Errorf("expected lockout to escalate with more failures (3→%v, 5→%v)", rem3, rem5)
	}
	tracker.clearKey("email:esc@test.com")
}

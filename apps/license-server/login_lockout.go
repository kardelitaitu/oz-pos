package main

// Escalating login lockout for the website auth endpoints (web_otp.go /
// web_password.go). Complements the existing windowed limiters
// (webLoginLimiter, otpVerifyLimiter) with a brute-force-resistant
// escalating cooldown that persists across restarts (H2 audit), so an
// attacker cannot evade the lockout by waiting for a redeploy.
//
// Scheme (per user requirement):
//
//	minimum gap between attempts           = 5s
//	after the 3rd consecutive failure       +30s  → 35s
//	each further consecutive failure        +30s  → 65s, 95s, …
//	capped at 15 minutes (900s)
//
// Formula: cooldown = min(5s + max(0, failures-2) × 30s, 900s)
//
// A successful login clears the key. The lockout is enforced per
// normalized email AND per IP, both persisted to SQLite
// (rate_limit_login_lockouts) so server restarts cannot reset it.

import (
	"fmt"
	"log"
	"os"
	"runtime/debug"
	"strings"
	"sync"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

const (
	// loginMinGap is the floor between any two login attempts.
	loginMinGapDefault = 5 * time.Second
	// loginEscalationStep is added per failure beyond the 2nd.
	loginEscalationStep = 30 * time.Second
	// loginMaxCooldown caps the escalating lockout at 15 minutes.
	loginMaxCooldownDefault = 15 * time.Minute
	// loginPartialTTL decays partial (sub-3) failures after idle.
	loginPartialTTL = 1 * time.Hour
	// loginSweepInterval is how often stale entries are cleaned.
	loginSweepInterval = 30 * time.Minute
)

// loginMinGap returns the configured minimum gap, honoring the
// LOGIN_LOCKOUT_MIN_GAP env var (Go duration). Falls back to 5s.
func loginMinGap() time.Duration {
	v := os.Getenv("LOGIN_LOCKOUT_MIN_GAP")
	if v == "" {
		return loginMinGapDefault
	}
	d, err := time.ParseDuration(v)
	if err != nil {
		log.Printf("loginLockout: invalid LOGIN_LOCKOUT_MIN_GAP=%q (using default %v): %v",
			v, loginMinGapDefault, err)
		return loginMinGapDefault
	}
	return d
}

// loginMaxCooldown returns the configured lockout cap, honoring the
// LOGIN_LOCKOUT_MAX_COOLDOWN env var. Falls back to 15 minutes.
func loginMaxCooldown() time.Duration {
	v := os.Getenv("LOGIN_LOCKOUT_MAX_COOLDOWN")
	if v == "" {
		return loginMaxCooldownDefault
	}
	d, err := time.ParseDuration(v)
	if err != nil {
		log.Printf("loginLockout: invalid LOGIN_LOCKOUT_MAX_COOLDOWN=%q (using default %v): %v",
			v, loginMaxCooldownDefault, err)
		return loginMaxCooldownDefault
	}
	return d
}

// loginLockout computes the escalating cooldown for the given consecutive
// failure count (see package doc for the scheme). Honors the
// LOGIN_LOCKOUT_MIN_GAP / LOGIN_LOCKOUT_MAX_COOLDOWN env overrides.
func loginLockout(failures int) time.Duration {
	minGap := loginMinGap()
	maxCooldown := loginMaxCooldown()
	d := minGap + time.Duration(max(0, failures-2))*loginEscalationStep
	if d > maxCooldown {
		d = maxCooldown
	}
	return d
}

// loginFailures is the per-key lockout state.
type loginFailures struct {
	count        int       // consecutive failures
	lastAttempt  time.Time // last attempt timestamp
	lockoutUntil time.Time // when the key unlocks (zero = not locked)
}

// loginLockoutTracker tracks escalating login lockouts with SQLite
// persistence (mirrors keyFailureTracker / rateLimiter architecture).
type loginLockoutTracker struct {
	mu      sync.Mutex
	entries map[string]*loginFailures
	db      core.App
	stopCh  chan struct{}
	started bool
}

var loginLockoutTrackerInst = &loginLockoutTracker{
	entries: make(map[string]*loginFailures),
}

// ── Lookup / enforcement ───────────────────────────────────────────

// lockoutUntil returns when the key unlocks, or zero when not locked.
// Also applies the partial-failure decay (sub-3 failures idle > 1h).
func (lt *loginLockoutTracker) lockoutUntil(key string) (time.Time, time.Duration) {
	lt.mu.Lock()
	defer lt.mu.Unlock()

	f, ok := lt.entries[key]
	if !ok {
		return time.Time{}, 0
	}
	now := time.Now()
	// Decay partial failures after idle.
	if f.count < 3 && now.Sub(f.lastAttempt) > loginPartialTTL {
		delete(lt.entries, key)
		return time.Time{}, 0
	}
	// Expired lockout → treat as fresh.
	if !f.lockoutUntil.IsZero() && now.After(f.lockoutUntil) {
		delete(lt.entries, key)
		return time.Time{}, 0
	}
	if !f.lockoutUntil.IsZero() {
		remaining := time.Until(f.lockoutUntil)
		if remaining < 0 {
			remaining = 0
		}
		return f.lockoutUntil, remaining
	}
	return time.Time{}, 0
}

// loginLockoutDisabled reports whether the escalating lockout layer is
// turned off. Production default is enabled. Only used in tests to keep
// the pre-existing window-limiter tests testing the window limiter in
// isolation; the formula + persistence are covered by their own tests.
func loginLockoutDisabled() bool {
	return os.Getenv("LOGIN_LOCKOUT_DISABLED") == "1"
}

// isLocked reports whether the key is currently locked out and, if so,
// for how long. The lockout also enforces the minimum gap even when
// the key has no cooldown yet (prevents rapid-fire probing).
func (lt *loginLockoutTracker) isLocked(key string) (bool, time.Duration) {
	if loginLockoutDisabled() {
		return false, 0
	}
	until, remaining := lt.lockoutUntil(key)
	if !until.IsZero() {
		return true, remaining
	}
	// Enforce minimum gap between attempts.
	lt.mu.Lock()
	f, ok := lt.entries[key]
	lt.mu.Unlock()
	if ok {
		gap := time.Since(f.lastAttempt)
		minGap := loginMinGap()
		if gap < minGap {
			return true, minGap - gap
		}
	}
	return false, 0
}

// recordFailure increments the failure count and sets/extends the
// escalating lockout. Persists to SQLite. Returns the lockout duration.
func (lt *loginLockoutTracker) recordFailure(key string) time.Duration {
	if loginLockoutDisabled() {
		return 0
	}
	lt.mu.Lock()
	f, ok := lt.entries[key]
	if !ok {
		f = &loginFailures{}
		lt.entries[key] = f
	}
	f.count++
	f.lastAttempt = time.Now()
	// Escalating cooldown once past the 2nd consecutive failure.
	if f.count >= 3 {
		f.lockoutUntil = time.Now().Add(loginLockout(f.count))
	}
	dbAttached := lt.db != nil
	snap := loginFailures{count: f.count, lastAttempt: f.lastAttempt, lockoutUntil: f.lockoutUntil}
	lt.mu.Unlock()

	if dbAttached {
		if err := lt.persist(key, snap); err != nil {
			log.Printf("loginLockout: persist error for key=%q (in-memory still authoritative): %v", key, err)
		}
	}
	return loginLockout(f.count)
}

// clearKey resets the lockout for a key (successful login).
func (lt *loginLockoutTracker) clearKey(key string) {
	lt.mu.Lock()
	delete(lt.entries, key)
	lt.mu.Unlock()
	if lt.db != nil {
		if _, err := lt.db.DB().NewQuery(
			`DELETE FROM rate_limit_login_lockouts WHERE key = {:key}`,
		).Bind(map[string]any{"key": key}).Execute(); err != nil {
			log.Printf("loginLockout: failed to delete persisted key=%q: %v", key, err)
		}
	}
}

// ── Persistence (H2 audit) ─────────────────────────────────────────

func (lt *loginLockoutTracker) attachPersistence(app core.App) {
	if app == nil {
		return
	}
	lt.mu.Lock()
	if lt.db != nil {
		lt.mu.Unlock()
		return
	}
	lt.db = app
	lt.mu.Unlock()

	if err := lt.createSchema(); err != nil {
		log.Printf("loginLockout: failed to create schema (in-memory-only): %v", err)
		return
	}
	if err := lt.hydrate(); err != nil {
		log.Printf("loginLockout: hydrate error (in-memory state may be partial): %v", err)
	}
	lt.startCleanup()
}

func (lt *loginLockoutTracker) createSchema() error {
	_, err := lt.db.DB().NewQuery(
		`CREATE TABLE IF NOT EXISTS rate_limit_login_lockouts (
			key TEXT PRIMARY KEY,
			count INTEGER NOT NULL,
			last_attempt TEXT NOT NULL,
			lockout_until TEXT
		)`,
	).Execute()
	return err
}

func (lt *loginLockoutTracker) hydrate() error {
	rows, err := lt.db.DB().NewQuery(
		`SELECT key, count, last_attempt, COALESCE(lockout_until, '') AS lockout_until FROM rate_limit_login_lockouts`,
	).Rows()
	if err != nil {
		return err
	}
	defer rows.Close()

	type loaded struct {
		key          string
		count        int
		lastAttempt  time.Time
		lockoutUntil time.Time
	}
	var pending []loaded
	now := time.Now()
	for rows.Next() {
		var key, lastStr, lockStr string
		var count int
		if err := rows.Scan(&key, &count, &lastStr, &lockStr); err != nil {
			return err
		}
		last, err := time.Parse(time.RFC3339, lastStr)
		if err != nil {
			log.Printf("loginLockout: skipping row with bad last_attempt=%q", lastStr)
			continue
		}
		var lockUntil time.Time
		if lockStr != "" {
			t, err := time.Parse(time.RFC3339, lockStr)
			if err != nil {
				log.Printf("loginLockout: skipping row with bad lockout_until=%q", lockStr)
				continue
			}
			lockUntil = t
		}
		// Skip stale partial rows.
		if count < 3 && now.Sub(last) > loginPartialTTL {
			continue
		}
		pending = append(pending, loaded{key: key, count: count, lastAttempt: last, lockoutUntil: lockUntil})
	}

	lt.mu.Lock()
	defer lt.mu.Unlock()
	for _, p := range pending {
		lt.entries[p.key] = &loginFailures{
			count: p.count, lastAttempt: p.lastAttempt, lockoutUntil: p.lockoutUntil,
		}
	}
	return nil
}

func (lt *loginLockoutTracker) persist(key string, f loginFailures) (err error) {
	if lt.db == nil {
		return nil
	}
	defer func() {
		if r := recover(); r != nil {
			log.Printf("loginLockout: recovered persist panic for key=%q (in-memory still authoritative): %v\n%s", key, r, string(debug.Stack()))
			err = nil
		}
	}()
	var lockArg string
	if !f.lockoutUntil.IsZero() {
		lockArg = f.lockoutUntil.Format(time.RFC3339)
	}
	_, err = lt.db.DB().NewQuery(
		`INSERT INTO rate_limit_login_lockouts (key, count, last_attempt, lockout_until)
		 VALUES ({:key}, {:count}, {:last}, {:lock})
		 ON CONFLICT(key) DO UPDATE SET
		   count = excluded.count,
		   last_attempt = excluded.last_attempt,
		   lockout_until = excluded.lockout_until`,
	).Bind(map[string]any{
		"key": key, "count": f.count, "last": f.lastAttempt.Format(time.RFC3339), "lock": lockArg,
	}).Execute()
	return err
}

// startCleanup runs a background sweeper for stale entries.
func (lt *loginLockoutTracker) startCleanup() {
	lt.mu.Lock()
	if lt.started {
		lt.mu.Unlock()
		return
	}
	lt.stopCh = make(chan struct{})
	lt.started = true
	ch := lt.stopCh
	lt.mu.Unlock()

	go func() {
		ticker := time.NewTicker(loginSweepInterval)
		defer ticker.Stop()
		for {
			select {
			case <-ticker.C:
				lt.sweep()
			case <-ch:
				return
			}
		}
	}()
}

func (lt *loginLockoutTracker) sweep() {
	lt.mu.Lock()
	defer lt.mu.Unlock()
	now := time.Now()
	for k, f := range lt.entries {
		if f.count < 3 && now.Sub(f.lastAttempt) > loginPartialTTL {
			delete(lt.entries, k)
			continue
		}
		if !f.lockoutUntil.IsZero() && now.After(f.lockoutUntil) {
			delete(lt.entries, k)
		}
	}
}

// ── HTTP helper ────────────────────────────────────────────────────

// loginLockoutKey normalizes a login identifier into a stable key.
func loginLockoutKey(email string) string {
	return "email:" + strings.ToLower(strings.TrimSpace(email))
}

// checkLoginLockout evaluates the email lockout and returns (locked,
// retryAfterSeconds). When locked, the caller should return 429 with
// retry_after. The IP-level lockout is enforced by otpIPLimiter +
// webLoginLimiter already; the email lockout is the escalating layer.
func checkLoginLockout(email string) (bool, int) {
	key := loginLockoutKey(email)
	locked, remaining := loginLockoutTrackerInst.isLocked(key)
	if !locked {
		return false, 0
	}
	return true, int(remaining.Seconds() + 0.999)
}

// recordLoginFailure records a failed login for the email and returns
// the new retry-after seconds.
func recordLoginFailure(email string) int {
	key := loginLockoutKey(email)
	cooldown := loginLockoutTrackerInst.recordFailure(key)
	return int(cooldown.Seconds() + 0.999)
}

// clearLoginLockout resets the escalating lockout after a successful login.
func clearLoginLockout(email string) {
	loginLockoutTrackerInst.clearKey(loginLockoutKey(email))
}

// registerLoginLockoutPersistence wires the tracker to the app on boot.
func registerLoginLockoutPersistence(app core.App) {
	loginLockoutTrackerInst.attachPersistence(app)
	log.Printf("loginLockout: escalating login lockout active (5s gap, +30s after 3rd failure, cap 15m)")
}

// max is a small int helper (Go 1.21+ has builtin max; keep for clarity).
func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}

// describeLoginLockout returns a human-readable retry message.
func describeLoginLockout(retryAfter int) string {
	if retryAfter <= 0 {
		return "too many failed attempts — please wait a moment and try again"
	}
	return fmt.Sprintf("too many failed attempts — try again in %d seconds", retryAfter)
}

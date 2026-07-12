package main

import (
	"log"
	"os"
	"runtime/debug"
	"sync"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// rateLimiter is a token-bucket per IP rate limiter with SQLite
// persistence (H2 audit) so server restarts cannot reset a brute-force
// attacker's rate-limit state. The in-memory map is the authority for
// allow() decisions during the request hot path; SQLite is mirrored via
// a write-through UPSERT after each in-memory update so a fresh process
// boot hydrates from disk. Background cleanup via a goroutine avoids
// O(N) lock contention on idle buckets.
type rateLimiter struct {
	mu             sync.Mutex
	buckets        map[string]*tokenBucket
	maxPerHr       int
	stopCleanup    chan struct{}
	cleanupRunning bool

	// db is the PocketBase app used for SQLite persistence of bucket
	// state. Wired by attachPersistence; may be nil (in tests that
	// don't set up an app, or before attachPersistence runs in
	// main.go). When set, allow() writes through to app.DB() after
	// each in-memory update.
	db core.App
}

type tokenBucket struct {
	tokens   int
	lastFill time.Time
}

const ipCleanupInterval = 30 * time.Minute
const ipBucketTTL = 2 * time.Hour

// ipRateLimiter limits activation attempts to 5 per IP per hour.
var ipRateLimiter = &rateLimiter{
	buckets:  make(map[string]*tokenBucket),
	maxPerHr: 5,
}

// startCleanup launches a background goroutine that periodically sweeps
// expired buckets to prevent unbounded memory growth. Call stopCleanup
// to shut it down (e.g. in tests). Idempotent; no-op if already running.
func (rl *rateLimiter) startCleanup() {
	rl.mu.Lock()
	if rl.cleanupRunning {
		rl.mu.Unlock()
		return
	}
	rl.stopCleanup = make(chan struct{})
	rl.cleanupRunning = true
	ch := rl.stopCleanup
	rl.mu.Unlock()

	go func() {
		ticker := time.NewTicker(ipCleanupInterval)
		defer ticker.Stop()
		for {
			select {
			case <-ticker.C:
				rl.sweep()
			case <-ch:
				return
			}
		}
	}()
}

// stop terminates the background cleanup goroutine. Idempotent.
func (rl *rateLimiter) stop() {
	rl.mu.Lock()
	defer rl.mu.Unlock()
	if !rl.cleanupRunning {
		return
	}
	close(rl.stopCleanup)
	rl.cleanupRunning = false
}

// sweep removes buckets that haven't been used in ipBucketTTL.
// Called by the background goroutine; also exposed for tests.
func (rl *rateLimiter) sweep() {
	rl.mu.Lock()
	defer rl.mu.Unlock()
	now := time.Now()
	for k, v := range rl.buckets {
		if now.Sub(v.lastFill) > ipBucketTTL {
			delete(rl.buckets, k)
		}
	}
}

func (rl *rateLimiter) allow(ip string) bool {
	rl.mu.Lock()

	bucket, ok := rl.buckets[ip]
	if !ok {
		bucket = &tokenBucket{tokens: rl.maxPerHr, lastFill: time.Now()}
		rl.buckets[ip] = bucket
	}

	// Refill tokens if an hour has passed since the last fill.
	if time.Since(bucket.lastFill) >= time.Hour {
		bucket.tokens = rl.maxPerHr
		bucket.lastFill = time.Now()
	}

	allowed := false
	if bucket.tokens > 0 {
		bucket.tokens--
		allowed = true
	}

	// Snapshot for persistence, then release the lock so other IPs
	// (and the keyFailTracker) can flow concurrently while SQLite is
	// being written to. In-memory is the authority for the allow()
	// decision; SQLite is just for restart survival.
	dbAttached := rl.db != nil
	var (
		snapIP       string
		snapTokens   int
		snapLastFill time.Time
	)
	if dbAttached {
		snapIP = ip
		snapTokens = bucket.tokens
		snapLastFill = bucket.lastFill
	}
	rl.mu.Unlock()

	if dbAttached {
		if err := rl.persistBucket(snapIP, snapTokens, snapLastFill); err != nil {
			log.Printf("ipRateLimiter: persist error for ip=%q (in-memory still authoritative): %v",
				ip, err)
		}
	}
	return allowed
}

// ── Persistence (H2 audit) ────────────────────────────────────────

// attachPersistence wires the rate limiter to a PocketBase app's
// SQLite so bucket state survives server restarts. Idempotent: no-op
// if already attached. createSchema then hydrate. Logs and returns on
// schema/hydrate errors so in-memory-only mode is the last-resort
// fallback (tests + warm-up phases benefit from graceful degradation).
func (rl *rateLimiter) attachPersistence(app core.App) {
	if app == nil {
		return
	}
	rl.mu.Lock()
	if rl.db != nil {
		rl.mu.Unlock()
		return
	}
	rl.db = app
	rl.mu.Unlock()

	if err := rl.createSchema(); err != nil {
		log.Printf("ipRateLimiter: failed to create schema (in-memory-only mode): %v", err)
		return
	}
	if err := rl.hydrate(); err != nil {
		log.Printf("ipRateLimiter: hydrate error (in-memory state may be partial): %v", err)
	}
}

// createSchema runs once on attach. PocketBase does not auto-migrate
// table-only collections, so we drive the DDL ourselves.
func (rl *rateLimiter) createSchema() error {
	_, err := rl.db.DB().NewQuery(
		`CREATE TABLE IF NOT EXISTS rate_limit_ip_buckets (
			ip TEXT PRIMARY KEY,
			tokens INTEGER NOT NULL,
			last_fill TEXT NOT NULL
		)`,
	).Execute()
	return err
}

// hydrate loads non-stale rows from SQLite into the in-memory map.
// Stale rows (last_fill older than ipBucketTTL=2h) are skipped:
// they're effectively expired and would only slow the next
// read-modify-write without affecting the allow() decision.
func (rl *rateLimiter) hydrate() error {
	rows, err := rl.db.DB().NewQuery(
		`SELECT ip, tokens, last_fill FROM rate_limit_ip_buckets`,
	).Rows()
	if err != nil {
		return err
	}
	defer rows.Close()

	type loaded struct {
		ip       string
		tokens   int
		lastFill time.Time
	}
	var pending []loaded
	now := time.Now()
	for rows.Next() {
		var ip, lastFillStr string
		var tokens int
		if err := rows.Scan(&ip, &tokens, &lastFillStr); err != nil {
			return err
		}
		lastFill, err := time.Parse(time.RFC3339, lastFillStr)
		if err != nil {
			log.Printf("ipRateLimiter: skipping row with unparseable last_fill=%q: %v",
				lastFillStr, err)
			continue
		}
		if now.Sub(lastFill) > ipBucketTTL {
			continue
		}
		pending = append(pending, loaded{ip: ip, tokens: tokens, lastFill: lastFill})
	}

	rl.mu.Lock()
	defer rl.mu.Unlock()
	for _, p := range pending {
		rl.buckets[p.ip] = &tokenBucket{tokens: p.tokens, lastFill: p.lastFill}
	}
	return nil
}

// persistBucket UPSERTs the bucket state. Called outside the in-memory
// mutex so concurrent IPs flow freely; last-write-wins is correct
// because all concurrent writers commit the same post-decrement value
// for the same (ip) row.
func (rl *rateLimiter) persistBucket(ip string, tokens int, lastFill time.Time) (err error) {
	if rl.db == nil {
		return nil
	}
	defer func() {
		if r := recover(); r != nil {
			log.Printf("ipRateLimiter: recovered persist panic for ip=%q (in-memory still authoritative): %v\n%s", ip, r, string(debug.Stack()))
			err = nil
		}
	}()
	_, err = rl.db.DB().NewQuery(
		`INSERT INTO rate_limit_ip_buckets (ip, tokens, last_fill)
		 VALUES ({:ip}, {:tokens}, {:last_fill})
		 ON CONFLICT(ip) DO UPDATE SET
		   tokens = excluded.tokens,
		   last_fill = excluded.last_fill`,
	).Bind(map[string]any{
		"ip":        ip,
		"tokens":    tokens,
		"last_fill": lastFill.Format(time.RFC3339),
	}).Execute()
	return err
}

// keyFailureTracker tracks per-key brute-force attempts with a TTL and
// persists state to SQLite (H2 audit) so server restarts cannot reset an
// attacker's cooldown window. Mirrors rateLimiter: in-memory is the
// request-hot-path authority, SQLite is mirrored via write-through.
type keyFailureTracker struct {
	mu             sync.Mutex
	failures        map[string]*keyFailures
	maxAttempts    int
	cooldown        time.Duration
	stopCleanup    chan struct{}
	cleanupRunning bool

	// db is the PocketBase app used for SQLite persistence. Wired by
	// attachPersistence. When set, recordFailure() writes through to
	// app.DB() after each in-memory update.
	db core.App
}

type keyFailures struct {
	count       int
	cooldownAt  time.Time
	lastAttempt time.Time
}

const keyCleanupInterval = 1 * time.Hour
const keyPartialFailureTTL = 1 * time.Hour // decay partial failures after 1h idle

// defaultKeyCooldown is the cooldown applied after maxAttempts failures
// are recorded against a single key. Production default; overridable via
// the LICENSE_KEY_COOLDOWN env var (e.g. for development to use a much
// shorter cooldown that doesn't punish legitimate retries).
const defaultKeyCooldown = 15 * time.Minute

// parseCooldown returns the cooldown duration to apply to keyFailTracker,
// honoring the LICENSE_KEY_COOLDOWN env var. Falls back to the production
// default if the env var is unset or unparseable — never weakens security
// implicitly.
func parseCooldown() time.Duration {
	v := os.Getenv("LICENSE_KEY_COOLDOWN")
	if v == "" {
		return defaultKeyCooldown
	}
	d, err := time.ParseDuration(v)
	if err != nil {
		log.Printf("keyFailTracker: invalid LICENSE_KEY_COOLDOWN=%q (using default %v): %v",
			v, defaultKeyCooldown, err)
		return defaultKeyCooldown
	}
	log.Printf("keyFailTracker: cooldown overridden to %v via LICENSE_KEY_COOLDOWN", d)
	return d
}

// keyFailTracker limits to 3 failed attempts per key, then a cooldown
// (default 15 min; LICENSE_KEY_COOLDOWN env var overrides for dev).
var keyFailTracker = &keyFailureTracker{
	failures:    make(map[string]*keyFailures),
	maxAttempts: 3,
	cooldown:    defaultKeyCooldown,
}

// startCleanup launches a background goroutine that sweeps expired entries.
// Idempotent; no-op if already running.
func (kf *keyFailureTracker) startCleanup() {
	kf.mu.Lock()
	if kf.cleanupRunning {
		kf.mu.Unlock()
		return
	}
	kf.stopCleanup = make(chan struct{})
	kf.cleanupRunning = true
	ch := kf.stopCleanup
	kf.mu.Unlock()

	go func() {
		ticker := time.NewTicker(keyCleanupInterval)
		defer ticker.Stop()
		for {
			select {
			case <-ticker.C:
				kf.sweep()
			case <-ch:
				return
			}
		}
	}()
}

// stop terminates the background cleanup goroutine. Idempotent.
func (kf *keyFailureTracker) stop() {
	kf.mu.Lock()
	defer kf.mu.Unlock()
	if !kf.cleanupRunning {
		return
	}
	close(kf.stopCleanup)
	kf.cleanupRunning = false
}

// sweep removes stale partial-failure entries and expired cooldowns.
func (kf *keyFailureTracker) sweep() {
	kf.mu.Lock()
	defer kf.mu.Unlock()
	now := time.Now()
	for k, v := range kf.failures {
		// Partial failures that haven't been seen in keyPartialFailureTTL are stale.
		if v.count < kf.maxAttempts && now.Sub(v.lastAttempt) > keyPartialFailureTTL {
			delete(kf.failures, k)
			continue
		}
		// Cooldown entries that have passed their cooldown.
		if v.count >= kf.maxAttempts && now.After(v.cooldownAt) {
			delete(kf.failures, k)
		}
	}
}

func (kf *keyFailureTracker) isBlocked(key string) bool {
	kf.mu.Lock()
	defer kf.mu.Unlock()

	f, ok := kf.failures[key]
	if !ok {
		return false
	}

	// Decay partial failures: if the user made a few wrong attempts
	// but hasn't tried again in keyPartialFailureTTL, reset the counter
	// so they get a fresh start (prevents "one typo away from block" forever).
	if f.count < kf.maxAttempts && time.Since(f.lastAttempt) > keyPartialFailureTTL {
		f.count = 0
		f.lastAttempt = time.Now()
		return false
	}

	// Clean up entries that have reached maxAttempts AND passed cooldown.
	if f.count >= kf.maxAttempts && time.Now().After(f.cooldownAt) {
		delete(kf.failures, key)
		return false
	}
	return f.count >= kf.maxAttempts
}

func (kf *keyFailureTracker) recordFailure(key string) {
	kf.mu.Lock()

	f, ok := kf.failures[key]
	if !ok {
		f = &keyFailures{count: 0}
		kf.failures[key] = f
	}
	f.count++
	f.lastAttempt = time.Now()
	if f.count >= kf.maxAttempts {
		f.cooldownAt = time.Now().Add(kf.cooldown)
	}

	// Snapshot for persistence, then release the lock so other keys can
	// flow concurrently while SQLite is being written to. In-memory is
	// the authority; SQLite is just for restart survival.
	dbAttached := kf.db != nil
	var (
		snapKey         string
		snapCount       int
		snapLastAttempt time.Time
		snapCooldownAt  time.Time
	)
	if dbAttached {
		snapKey = key
		snapCount = f.count
		snapLastAttempt = f.lastAttempt
		snapCooldownAt = f.cooldownAt
	}
	kf.mu.Unlock()

	if dbAttached {
		if err := kf.persistFailure(snapKey, snapCount, snapLastAttempt, snapCooldownAt); err != nil {
			log.Printf("keyFailTracker: persist error for key=%q (in-memory still authoritative): %v",
				key, err)
		}
	}
}

// ── Persistence (H2 audit) ────────────────────────────────────────

// attachPersistence wires the tracker to a PocketBase app's SQLite
// for restart-survival (H2 audit). Idempotent: no-op if already
// attached. Logs and returns on schema/hydrate errors so in-memory
// mode is the last-resort fallback.
func (kf *keyFailureTracker) attachPersistence(app core.App) {
	if app == nil {
		return
	}
	kf.mu.Lock()
	if kf.db != nil {
		kf.mu.Unlock()
		return
	}
	kf.db = app
	kf.mu.Unlock()

	if err := kf.createSchema(); err != nil {
		log.Printf("keyFailTracker: failed to create schema (in-memory-only mode): %v", err)
		return
	}
	if err := kf.hydrate(); err != nil {
		log.Printf("keyFailTracker: hydrate error (in-memory state may be partial): %v", err)
	}
}

// createSchema runs once on attach. cooldown_until is intentionally
// nullable because not all rows have an active cooldown.
func (kf *keyFailureTracker) createSchema() error {
	_, err := kf.db.DB().NewQuery(
		`CREATE TABLE IF NOT EXISTS rate_limit_key_failures (
			key TEXT PRIMARY KEY,
			count INTEGER NOT NULL,
			last_attempt TEXT NOT NULL,
			cooldown_until TEXT
		)`,
	).Execute()
	return err
}

// hydrate loads failure rows from SQLite into the in-memory map.
// Stale partial-failure rows (last_attempt older than
// keyPartialFailureTTL=1h) are skipped. Rows with an expired
// cooldown_until are kept; isBlocked() treats them as not-blocked
// during request handling and the sweep goroutine cleans them up.
func (kf *keyFailureTracker) hydrate() error {
	rows, err := kf.db.DB().NewQuery(
		`SELECT key, count, last_attempt, COALESCE(cooldown_until, '') AS cooldown_until FROM rate_limit_key_failures`,
	).Rows()
	if err != nil {
		return err
	}
	defer rows.Close()

	type loaded struct {
		key         string
		count       int
		lastAttempt time.Time
		cooldownAt  time.Time
	}
	var pending []loaded
	now := time.Now()
	for rows.Next() {
		var key, lastAttemptStr, cooldownStr string
		var count int
		if err := rows.Scan(&key, &count, &lastAttemptStr, &cooldownStr); err != nil {
			return err
		}
		lastAttempt, err := time.Parse(time.RFC3339, lastAttemptStr)
		if err != nil {
			log.Printf("keyFailTracker: skipping row with unparseable last_attempt=%q: %v",
				lastAttemptStr, err)
			continue
		}
		var cooldownAt time.Time
		if cooldownStr != "" {
			t, err := time.Parse(time.RFC3339, cooldownStr)
			if err != nil {
				log.Printf("keyFailTracker: skipping row with unparseable cooldown_until=%q: %v",
					cooldownStr, err)
				continue
			}
			cooldownAt = t
		}
		if count < kf.maxAttempts && now.Sub(lastAttempt) > keyPartialFailureTTL {
			continue
		}
		pending = append(pending, loaded{
			key:         key,
			count:       count,
			lastAttempt: lastAttempt,
			cooldownAt:  cooldownAt,
		})
	}

	kf.mu.Lock()
	defer kf.mu.Unlock()
	for _, p := range pending {
		kf.failures[p.key] = &keyFailures{
			count:       p.count,
			cooldownAt:  p.cooldownAt,
			lastAttempt: p.lastAttempt,
		}
	}
	return nil
}

// persistFailure UPSERTs a row's failure state. cooldown_until is
// RFC3339 when non-zero, empty string when no cooldown. hydrate()
// interprets empty as zero time on read.
func (kf *keyFailureTracker) persistFailure(key string, count int, lastAttempt, cooldownAt time.Time) (err error) {
	if kf.db == nil {
		return nil
	}
	defer func() {
		if r := recover(); r != nil {
			log.Printf("keyFailTracker: recovered persist panic for key=%q (in-memory still authoritative): %v\n%s", key, r, string(debug.Stack()))
			err = nil
		}
	}()
	var cooldownArg string
	if !cooldownAt.IsZero() {
		cooldownArg = cooldownAt.Format(time.RFC3339)
	}
	_, err = kf.db.DB().NewQuery(
		`INSERT INTO rate_limit_key_failures (key, count, last_attempt, cooldown_until)
		 VALUES ({:key}, {:count}, {:last_attempt}, {:cooldown_until})
		 ON CONFLICT(key) DO UPDATE SET
		   count = excluded.count,
		   last_attempt = excluded.last_attempt,
		   cooldown_until = excluded.cooldown_until`,
	).Bind(map[string]any{
		"key":            key,
		"count":          count,
		"last_attempt":   lastAttempt.Format(time.RFC3339),
		"cooldown_until": cooldownArg,
	}).Execute()
	return err
}

func init() {
	// Apply the LICENSE_KEY_COOLDOWN env override (or fall back to
	// defaultKeyCooldown) before cleanup goroutines start.
	keyFailTracker.cooldown = parseCooldown()

	ipRateLimiter.startCleanup()
	keyFailTracker.startCleanup()
}

// keyActivationLocks provides per-key mutual exclusion to prevent
// concurrent activation of the same license key. Two goroutines
// racing to activate the same key will be serialised.
type keyActivationLocks struct {
	mu    sync.Mutex
	locks map[string]*sync.Mutex
}

var activationLocks = &keyActivationLocks{
	locks: make(map[string]*sync.Mutex),
}

// lock acquires a mutex for the given key, creating one if needed.
// Returns a function that must be called to release the lock.
func (kal *keyActivationLocks) lock(key string) func() {
	kal.mu.Lock()
	mu, ok := kal.locks[key]
	if !ok {
		mu = &sync.Mutex{}
		kal.locks[key] = mu
	}
	kal.mu.Unlock()

	mu.Lock()
	return func() { mu.Unlock() }
}

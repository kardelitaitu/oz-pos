package main

import (
	"sync"
	"time"
)

// rateLimiter is a simple in-memory token bucket per IP.
// Not persisted; resets on server restart (acceptable for activation volume).
// Background cleanup via a goroutine avoids O(N) lock contention on every
// request. Stale buckets (>2h idle) are swept every 30 minutes.
type rateLimiter struct {
	mu             sync.Mutex
	buckets        map[string]*tokenBucket
	maxPerHr       int
	stopCleanup    chan struct{}
	cleanupRunning bool
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
	defer rl.mu.Unlock()

	bucket, ok := rl.buckets[ip]
	if !ok {
		bucket = &tokenBucket{tokens: rl.maxPerHr, lastFill: time.Now()}
		rl.buckets[ip] = bucket
	}

	// Refill tokens if an hour has passed
	if time.Since(bucket.lastFill) >= time.Hour {
		bucket.tokens = rl.maxPerHr
		bucket.lastFill = time.Now()
	}

	if bucket.tokens > 0 {
		bucket.tokens--
		return true
	}
	return false
}

// keyFailureTracker tracks per-key brute-force attempts with a TTL.
type keyFailureTracker struct {
	mu             sync.Mutex
	failures        map[string]*keyFailures
	maxAttempts    int
	cooldown        time.Duration
	stopCleanup    chan struct{}
	cleanupRunning bool
}

type keyFailures struct {
	count       int
	cooldownAt  time.Time
	lastAttempt time.Time
}

const keyCleanupInterval = 1 * time.Hour
const keyPartialFailureTTL = 1 * time.Hour // decay partial failures after 1h idle

// keyFailTracker limits to 3 failed attempts per key, then 15-minute cooldown.
var keyFailTracker = &keyFailureTracker{
	failures:    make(map[string]*keyFailures),
	maxAttempts: 3,
	cooldown:    15 * time.Minute,
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
	defer kf.mu.Unlock()

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
}

func init() {
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

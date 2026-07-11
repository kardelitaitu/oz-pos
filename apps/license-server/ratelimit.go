package main

import (
	"sync"
	"time"
)

// rateLimiter is a simple in-memory token bucket per IP.
// Not persisted; resets on server restart (acceptable for activation volume).
type rateLimiter struct {
	mu       sync.Mutex
	buckets  map[string]*tokenBucket
	maxPerHr int
}

type tokenBucket struct {
	tokens   int
	lastFill time.Time
}

// ipRateLimiter limits activation attempts to 5 per IP per hour.
var ipRateLimiter = &rateLimiter{
	buckets:  make(map[string]*tokenBucket),
	maxPerHr: 5,
}

func (rl *rateLimiter) allow(ip string) bool {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	// Clean up old buckets periodically (e.g. > 2 hours old) to prevent memory leaks
	for k, v := range rl.buckets {
		if time.Since(v.lastFill) > 2*time.Hour {
			delete(rl.buckets, k)
		}
	}

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
	mu          sync.Mutex
	failures    map[string]*keyFailures
	maxAttempts int
	cooldown    time.Duration
}

type keyFailures struct {
	count      int
	cooldownAt time.Time
	lastAttempt time.Time
}

// keyFailTracker limits to 3 failed attempts per key, then 15-minute cooldown.
var keyFailTracker = &keyFailureTracker{
	failures:    make(map[string]*keyFailures),
	maxAttempts: 3,
	cooldown:    15 * time.Minute,
}

func (kf *keyFailureTracker) isBlocked(key string) bool {
	kf.mu.Lock()
	defer kf.mu.Unlock()

	// Clean up old partial failures (memory leak fix)
	for k, v := range kf.failures {
		if v.count < kf.maxAttempts && time.Since(v.lastAttempt) > 24*time.Hour {
			delete(kf.failures, k)
		}
	}

	f, ok := kf.failures[key]
	if !ok {
		return false
	}
	// Only clean up entries that have reached maxAttempts AND passed cooldown.
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

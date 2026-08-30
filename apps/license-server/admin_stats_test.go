package main

// Tests for the admin dashboard stats endpoint (admin_stats.go) — bug hunt
// round 6. B25: the FX cache pinned a transient upstream failure to the
// 16000 fallback for the full 1-hour success TTL. (B26 was investigated
// and dropped — see the warning note at the bottom of this file.)

import (
	"testing"
	"time"
)

// ── B25: FX negative cache ───────────────────────────────────────────

func resetFxCacheForTest() {
	fxCacheMu.Lock()
	defer fxCacheMu.Unlock()
	fxCache = nil
}

func TestGetFxRateFailureIsNotPinnedToSuccessTTL(t *testing.T) {
	resetFxCacheForTest()
	defer resetFxCacheForTest()
	origFetcher, origRetry := fxFetcher, fxRetryTTL
	defer func() { fxFetcher, fxRetryTTL = origFetcher, origRetry }()

	// Negative-cache TTL made observable: a failed fetch must be retried
	// after fxRetryTTL (short), NOT after fxCacheTTL (1 hour). The test
	// sets it negative so the very next call re-fetches deterministically.
	fxRetryTTL = -time.Second

	// First call: upstream down → fallback 16000, live=false.
	fxFetcher = func() (float64, bool) { return 0, false }
	rate, _, live := getFxRate()
	if rate != 16000 || live {
		t.Fatalf("expected fallback 16000/live=false, got rate=%v live=%v", rate, live)
	}

	// Upstream recovers immediately. B25: the old code cached the failure
	// with the SAME 1h TTL as a success, so this call returned the stale
	// fallback for an hour — the dashboard showed a wrong rate the whole
	// time (IDR conversions ~3% off) even though the API was healthy.
	fxFetcher = func() (float64, bool) { return 17123.45, true }
	rate, _, live = getFxRate()
	if !live || rate != 17123.45 {
		t.Fatalf("B25: failure pinned to success TTL — got rate=%v live=%v, want live 17123.45", rate, live)
	}
}

func TestGetFxRateSuccessCachedForTTL(t *testing.T) {
	resetFxCacheForTest()
	defer resetFxCacheForTest()
	origFetcher := fxFetcher
	defer func() { fxFetcher = origFetcher }()

	calls := 0
	fxFetcher = func() (float64, bool) { calls++; return 16500, true }
	if rate, _, live := getFxRate(); rate != 16500 || !live {
		t.Fatalf("live fetch expected, got rate=%v live=%v", rate, live)
	}
	getFxRate()
	if calls != 1 {
		t.Fatalf("success must cache for fxCacheTTL: fetcher called %d times, want 1", calls)
	}
}

// ── B26 (DROPPED hypothesis, kept as a warning) ──────────────────────
//
// Round-6 first read the revenue merge as "IDR-only months collapse to
// $0" and test-driven a fix that added realIdr/fxRate to realUsd. The
// pre-existing TestAdminStats_RealRevenue (revenue_events_test.go) then
// FAILED — and it was right: revenue_events.go stores BOTH currencies of
// every payment (native amount + FX-converted counterpart at write time),
// so amount_usd already includes Midtrans IDR revenue. Adding idr/fx
// double-counts every payment. Hypothesis dropped, merge reverted.
// Lesson: check the WRITER's data model before "fixing" a reader.

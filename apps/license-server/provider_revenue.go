package main

// Provider-revenue ledger — income/gross source of truth for the admin
// dashboard (ADR #42 Phase 3+).  Revenue_events is an append-only ledger
// written exclusively by signature-verified Paddle/Midtrans webhooks (see
// revenue_events.go); revenue_adjustments (see revenue_adjustments.go)
// records the refunds/chargebacks that claw gross back.  This module buckets
// both ledgers per month and computes lifetime totals, with a short-TTL
// cache so repeated stats calls don't rescan the whole table.  Only
// webhook-verified amounts ever appear here — a subscription price-map
// projection is never mixed in, so manual admin DB edits (tier overrides,
// renews, grants) cannot move the money figures (see admin_stats.go).
//
// Cache TTL: 5 minutes by default, overridable for tests.

import (
	"fmt"
	"log"
	"sync"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// defaultProviderRevTTL is how long a revenue snapshot stays fresh.
const defaultProviderRevTTL = 5 * time.Minute

// providerRevenueCacheEntry holds one computed snapshot of the revenue_events
// and revenue_adjustments ledgers plus metadata.
type providerRevenueCacheEntry struct {
	ByMonth           map[string]monthRevenue
	Providers         map[string]int // paddle → count, midtrans → count
	LifetimeUsd       float64
	LifetimeIdr       float64
	LifetimeRefundUsd float64
	LifetimeRefundIdr float64
	LifetimePaddleUsd float64
	LifetimePaddleIdr float64
	LifetimeMidUsd    float64
	LifetimeMidIdr    float64
	UpdatedAt         time.Time
	ttl               time.Duration
}

// monthRevenue holds the gross amounts for one calendar month from provider
// webhook data.  Sources records which providers had events in this month.
//
// Currency honesty: every revenue_events row stores BOTH currencies
// (native + the other converted at webhook write time).  Summing the
// combined Usd/Idr totals mixes native and converted amounts across
// providers, so this struct ALSO carries per-provider splits computed
// from each row's native currency:
//
//	PaddleUsd    = amount_usd of paddle rows (native USD)
//	PaddleIdr    = amount_idr of paddle rows (converted at write time)
//	MidtransUsd  = amount_usd of midtrans rows (converted at write time)
//	MidtransIdr  = amount_idr of midtrans rows (native IDR)
//
// PaddleIdr + MidtransIdr == Idr and PaddleUsd + MidtransUsd == Usd.
// RefundUsd/RefundIdr are the month's claw-backs from revenue_adjustments
// (kept separate from gross so net = gross - refunds is never hidden).
type monthRevenue struct {
	Usd         float64
	Idr         float64
	PaddleUsd   float64
	PaddleIdr   float64
	MidtransUsd float64
	MidtransIdr float64
	RefundUsd   float64
	RefundIdr   float64
	Count       int
	Sources     map[string]bool // "paddle", "midtrans"
}

var (
	providerRevCache   *providerRevenueCacheEntry
	providerRevCacheMu sync.Mutex
	providerRevTTL     = defaultProviderRevTTL
)

// loadProviderRevenue scans revenue_events and revenue_adjustments once,
// buckets each by month, and returns a fresh snapshot.  The caller owns the
// returned value.
func loadProviderRevenue(app core.App) *providerRevenueCacheEntry {
	byMonth := make(map[string]monthRevenue)
	providers := make(map[string]int)
	var lifetimeUsd, lifetimeIdr float64
	var lifetimeRefundUsd, lifetimeRefundIdr float64
	var lifetimePaddleUsd, lifetimePaddleIdr float64
	var lifetimeMidUsd, lifetimeMidIdr float64

	events, err := app.FindRecordsByFilter("revenue_events",
		"id != ''", "-created", 0, 0)
	if err != nil {
		log.Printf("provider-revenue: revenue_events scan failed: %v", err)
		return &providerRevenueCacheEntry{
			ByMonth:   byMonth,
			Providers: providers,
			UpdatedAt: time.Now().UTC(),
			ttl:       providerRevTTL,
		}
	}

	for _, re := range events {
		usd := re.GetFloat("amount_usd")
		idr := float64(re.GetInt("amount_idr"))
		created := re.GetDateTime("created").Time()
		provider := re.GetString("provider")
		key := fmt.Sprintf("%d-%02d", created.Year(), created.Month())

		m := byMonth[key]
		m.Usd += usd
		m.Idr += idr
		// Per-provider split from each row's native currency (see the
		// monthRevenue doc comment for the currency semantics).
		switch provider {
		case "paddle":
			m.PaddleUsd += usd
			m.PaddleIdr += idr
		case "midtrans":
			m.MidtransUsd += usd
			m.MidtransIdr += idr
		}
		m.Count++
		if m.Sources == nil {
			m.Sources = make(map[string]bool)
		}
		if provider != "" {
			m.Sources[provider] = true
		}
		byMonth[key] = m

		// Track provider counts for the split.
		providers[provider]++

		lifetimeUsd += usd
		lifetimeIdr += idr
		switch provider {
		case "paddle":
			lifetimePaddleUsd += usd
			lifetimePaddleIdr += idr
		case "midtrans":
			lifetimeMidUsd += usd
			lifetimeMidIdr += idr
		}
	}

	// Second ledger: refunds / chargebacks (kept separate from gross).
	adjustments, err := app.FindRecordsByFilter("revenue_adjustments",
		"id != ''", "-created", 0, 0)
	if err != nil {
		log.Printf("provider-revenue: revenue_adjustments scan failed: %v", err)
		return &providerRevenueCacheEntry{
			ByMonth:   byMonth,
			Providers: providers,
			UpdatedAt: time.Now().UTC(),
			ttl:       providerRevTTL,
		}
	}
	for _, adj := range adjustments {
		usd := adj.GetFloat("amount_usd")
		idr := float64(adj.GetInt("amount_idr"))
		created := adj.GetDateTime("created").Time()
		key := fmt.Sprintf("%d-%02d", created.Year(), created.Month())

		m := byMonth[key]
		m.RefundUsd += usd
		m.RefundIdr += idr
		byMonth[key] = m

		lifetimeRefundUsd += usd
		lifetimeRefundIdr += idr
	}

	return &providerRevenueCacheEntry{
		ByMonth:           byMonth,
		Providers:         providers,
		LifetimeUsd:       lifetimeUsd,
		LifetimeIdr:       lifetimeIdr,
		LifetimeRefundUsd: lifetimeRefundUsd,
		LifetimeRefundIdr: lifetimeRefundIdr,
		LifetimePaddleUsd: lifetimePaddleUsd,
		LifetimePaddleIdr: lifetimePaddleIdr,
		LifetimeMidUsd:    lifetimeMidUsd,
		LifetimeMidIdr:    lifetimeMidIdr,
		UpdatedAt:         time.Now().UTC(),
		ttl:               providerRevTTL,
	}
}

// getProviderRevenue returns the cached revenue snapshot, refreshing it
// when the TTL has expired.  Thread-safe.
func getProviderRevenue(app core.App) *providerRevenueCacheEntry {
	providerRevCacheMu.Lock()
	defer providerRevCacheMu.Unlock()

	now := time.Now()
	if providerRevCache != nil && now.Sub(providerRevCache.UpdatedAt) < providerRevCache.ttl {
		return providerRevCache
	}

	providerRevCache = loadProviderRevenue(app)
	return providerRevCache
}

// resetProviderRevenueCache clears the cache (test hook).
func resetProviderRevenueCache() {
	providerRevCacheMu.Lock()
	defer providerRevCacheMu.Unlock()
	providerRevCache = nil
}

// providerRevenueSource returns a human-readable source label for a month
// given its provider-source set.  Returns "provider" when at least one
// provider event exists, "estimate" when the month has no events.
func providerRevenueSource(m monthRevenue) string {
	if m.Count > 0 && m.Usd+m.Idr > 0 {
		// Build a short label from the providers present.
		if len(m.Sources) == 1 {
			for p := range m.Sources {
				return p + "_webhook"
			}
		}
		return "provider"
	}
	return "estimate"
}

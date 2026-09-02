package main

// Provider-revenue ledger — income/gross source of truth for the admin
// dashboard (ADR #42 Phase 3+).  Revenue_events is an append-only ledger
// written exclusively by signature-verified Paddle/Midtrans webhooks (see
// revenue_events.go).  This module buckets that ledger per month and
// computes lifetime totals, with a short-TTL cache so repeated stats calls
// don't rescan the whole table.  The price-map / subscription estimate
// survives only as a clearly-labeled fallback when a month has no provider
// events at all.
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
// ledger plus metadata.
type providerRevenueCacheEntry struct {
	ByMonth     map[string]monthRevenue
	Providers   map[string]int // paddle → count, midtrans → count
	LifetimeUsd float64
	LifetimeIdr float64
	UpdatedAt   time.Time
	ttl         time.Duration
}

// monthRevenue holds the gross amounts for one calendar month from provider
// webhook data.  Sources records which providers had events in this month.
type monthRevenue struct {
	Usd     float64
	Idr     float64
	Count   int
	Sources map[string]bool // "paddle", "midtrans"
}

var (
	providerRevCache   *providerRevenueCacheEntry
	providerRevCacheMu sync.Mutex
	providerRevTTL     = defaultProviderRevTTL
)

// loadProviderRevenue scans revenue_events once, buckets by month, and
// returns a fresh snapshot.  The caller owns the returned value.
func loadProviderRevenue(app core.App) *providerRevenueCacheEntry {
	byMonth := make(map[string]monthRevenue)
	providers := make(map[string]int)
	lifetimeUsd, lifetimeIdr := 0.0, 0.0

	events, err := app.FindRecordsByFilter("revenue_events",
		"id != ''", "-created", 0, 0)
	if err != nil {
		log.Printf("provider-revenue: scan failed: %v", err)
		return &providerRevenueCacheEntry{
			ByMonth:     byMonth,
			Providers:   providers,
			LifetimeUsd: 0,
			LifetimeIdr: 0,
			UpdatedAt:   time.Now().UTC(),
			ttl:         providerRevTTL,
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
	}

	return &providerRevenueCacheEntry{
		ByMonth:     byMonth,
		Providers:   providers,
		LifetimeUsd: lifetimeUsd,
		LifetimeIdr: lifetimeIdr,
		UpdatedAt:   time.Now().UTC(),
		ttl:         providerRevTTL,
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

package main

// Admin dashboard stats endpoint (ADR #42 Phase 3+) — real aggregates from
// tenants, subscriptions, and tenant_machines. Returns the same shape as the
// mock data so the frontend switches seamlessly.
//
// GET /api/v1/admin/stats
//
// Auth: adminAuth (OZ_ADMIN_KEY bearer or admin tenant session)
//
// MRR is computed from active subscriptions × tier price map (not from
// transaction amounts — Paddle/Midtrans revenue data lands later). The FX
// rate for USD→IDR is fetched from open.er-api.com with a 1-hour cache.

import (
	"encoding/json"
	"fmt"
	"log"
	"math"
	"net/http"
	"os"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// TierPriceUSD maps each tier key to its monthly USD price. Enterprise is
// custom; the default is an estimate that can be overridden via env.
var TierPriceUSD = map[string]float64{
	"free":       0,
	"plus":       4.99,
	"pro":        9.99,
	"premium":    39.99,
	"enterprise": 99.99,
}

// ── Server-side FX rate cache (1-hour TTL) ───────────────────────────

type fxCacheEntry struct {
	rate      float64
	updatedAt time.Time
	live      bool
}

var (
	fxCache   *fxCacheEntry
	fxCacheMu sync.Mutex
)

const fxCacheTTL = 1 * time.Hour

// getFxRate returns the USD→IDR rate. On first call or after TTL expiry it
// fetches from open.er-api.com; fallback is 16000.
func getFxRate() (rate float64, updatedAt time.Time, live bool) {
	fxCacheMu.Lock()
	defer fxCacheMu.Unlock()

	now := time.Now()
	if fxCache != nil && now.Sub(fxCache.updatedAt) < fxCacheTTL {
		return fxCache.rate, fxCache.updatedAt, fxCache.live
	}

	// Fetch fresh rate.
	client := &http.Client{Timeout: 5 * time.Second}
	resp, err := client.Get("https://open.er-api.com/v6/latest/USD")
	if err != nil {
		log.Printf("admin-stats: FX rate fetch failed: %v — using fallback", err)
		rate = 16000
		live = false
	} else {
		defer resp.Body.Close()
		var body struct {
			Rates map[string]float64 `json:"rates"`
		}
		if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
			log.Printf("admin-stats: FX rate decode failed: %v — using fallback", err)
			rate = 16000
			live = false
		} else if body.Rates == nil || body.Rates["IDR"] == 0 {
			rate = 16000
			live = false
		} else {
			rate = body.Rates["IDR"]
			live = true
		}
	}

	fxCache = &fxCacheEntry{rate: rate, updatedAt: now, live: live}
	return rate, now, live
}

// ── Stats handler ────────────────────────────────────────────────────

func handleAdminStats(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		if !adminAuth(app, e) {
			return nil
		}

		// ── KPIs ────────────────────────────────────────────────────
		totalUsers := countRecordsByFilter(app, "tenants", "")
		activeUsers := countRecordsByFilter(app, "tenants", "status = 'active'")

		// Active non-free subscriptions.
		activeSubs, _ := app.FindRecordsByFilter("subscriptions",
			"status = 'active' && tier_key != 'free'",
			"-created", 0, 0)
		totalSubscribers := len(activeSubs)

		// MRR: sum of active non-free subscriptions' tier prices.
		mrrUsd := 0.0
		for _, sub := range activeSubs {
			tier := sub.GetString("tier_key")
			mrrUsd += TierPriceUSD[tier]
		}
		arpuUsd := 0.0
		if totalSubscribers > 0 {
			arpuUsd = math.Round(mrrUsd/float64(totalSubscribers)*100) / 100
		}

		// Active devices (tenant_machines without revoked_at).
		activeDevices := countRecordsByFilter(app, "tenant_machines", "revoked_at IS NULL")

		// Trial → paid rate (approximate: trial tenant → active subscription).
		// Query subscriptions that were once trials (is_trial = true) and
		// are now active with a paid tier.
		paidFromTrial, _ := app.FindRecordsByFilter("subscriptions",
			"is_trial = true && status = 'active' && tier_key != 'free'",
			"", 0, 0)
		allTrials, _ := app.FindRecordsByFilter("subscriptions",
			"is_trial = true", "", 0, 0)
		trialToPaidRate := 0.0
		if len(allTrials) > 0 {
			trialToPaidRate = math.Round(float64(len(paidFromTrial))/float64(len(allTrials))*1000) / 10
		}

		fxRate, fxUpdatedAt, fxLive := getFxRate()

		// ── Time series (12 months) ─────────────────────────────────
		now := time.Now()
		type monthBucket struct {
			Month string  `json:"month"`
			Usd   float64 `json:"usd"`
			Idr   float64 `json:"idr"`
			Count int     `json:"count"`
			Churn int     `json:"churn"`
		}
		// Build 12-month window.
		buckets := make([]monthBucket, 12)
		bucketKeys := make([]string, 12)
		for i := 0; i < 12; i++ {
			t := now.AddDate(0, -11+i, 0)
			key := fmt.Sprintf("%d-%02d", t.Year(), t.Month())
			bucketKeys[i] = key
			buckets[i] = monthBucket{Month: key}
		}

		// Revenue trend: group active subscriptions by starts_at month,
		// sum tier prices.
		// Subscriber growth: cumulative count over time.
		// Signups: group tenants.created by month.
		// Churn: group expired/revoked subscriptions by expires_at month.

		// Scan active subscriptions (MRR contribution per month).
		revenueByMonth := make(map[string]float64)
		subGrowthByMonth := make(map[string]int)
		churnByMonth := make(map[string]int)

		// All subscriptions (active + expired + revoked) for time series.
		allSubs, _ := app.FindRecordsByFilter("subscriptions",
			"tier_key != 'free'",
			"-created", 0, 0)
		for _, sub := range allSubs {
			tier := sub.GetString("tier_key")
			price := TierPriceUSD[tier]

			// Active: add to revenue + growth.
			if sub.GetString("status") == "active" {
				startsAt := sub.GetDateTime("starts_at").Time()
				key := fmt.Sprintf("%d-%02d", startsAt.Year(), startsAt.Month())
				revenueByMonth[key] += price
				subGrowthByMonth[key]++
			}

			// Expired/revoked: add to churn.
			if sub.GetString("status") == "expired" || sub.GetString("status") == "revoked" {
				expiresAt := sub.GetDateTime("expires_at").Time()
				if !expiresAt.IsZero() {
					key := fmt.Sprintf("%d-%02d", expiresAt.Year(), expiresAt.Month())
					churnByMonth[key]++
				}
			}
		}

		// Build revenue trend, subscriber growth, churn arrays.
		revenueTrend := make([]monthBucket, 0, 12)
		subGrowth := make([]monthBucket, 0, 12)
		churnArr := make([]monthBucket, 0, 12)
		cumulative := 0
		for _, key := range bucketKeys {
			rev := revenueByMonth[key]
			revenueTrend = append(revenueTrend, monthBucket{
				Month: key, Usd: math.Round(rev*100) / 100, Idr: math.Round(rev*fxRate*100) / 100,
			})
			cumulative += subGrowthByMonth[key]
			subGrowth = append(subGrowth, monthBucket{
				Month: key, Count: cumulative,
			})
			churnArr = append(churnArr, monthBucket{
				Month: key, Churn: churnByMonth[key],
			})
		}

		// Signups per month.
		signupsByMonth := make(map[string]int)
		tenants, _ := app.FindRecordsByFilter("tenants", "status != ''", "-created", 0, 0)
		for _, t := range tenants {
			created := t.GetDateTime("created").Time()
			key := fmt.Sprintf("%d-%02d", created.Year(), created.Month())
			signupsByMonth[key]++
		}
		signupsArr := make([]monthBucket, 0, 12)
		for _, key := range bucketKeys {
			signupsArr = append(signupsArr, monthBucket{
				Month: key, Count: signupsByMonth[key],
			})
		}

		// ── Tier distribution ───────────────────────────────────────
		tierCount := make(map[string]int)
		for _, sub := range activeSubs {
			tier := sub.GetString("tier_key")
			tierCount[tier]++
		}
		tierDist := make([]map[string]any, 0)
		for _, t := range []string{"plus", "pro", "premium", "enterprise"} {
			if c := tierCount[t]; c > 0 {
				tierDist = append(tierDist, map[string]any{"tier": t, "count": c})
			}
		}

		// ── Payment provider split ─────────────────────────────────
		providerCount := make(map[string]int)
		for _, sub := range activeSubs {
			p := sub.GetString("payment_provider")
			if p == "" {
				p = "unknown"
			}
			providerCount[p]++
		}
		providerSplit := make([]map[string]any, 0)
		for _, p := range []string{"paddle", "midtrans", "unknown"} {
			if c := providerCount[p]; c > 0 {
				providerSplit = append(providerSplit, map[string]any{"provider": p, "count": c})
			}
		}

		// ── Top subscribers ─────────────────────────────────────────
		type topSub struct {
			Email    string  `json:"email"`
			Tier     string  `json:"tier"`
			MrrUsd   float64 `json:"mrrUsd"`
			Renewal  string  `json:"renewal"`
			Provider string  `json:"provider"`
		}
		topSubs := make([]topSub, 0)
		// Sort active subs by tier price descending.
		sort.Slice(activeSubs, func(i, j int) bool {
			return TierPriceUSD[activeSubs[i].GetString("tier_key")] > TierPriceUSD[activeSubs[j].GetString("tier_key")]
		})
		limit := 20
		if len(activeSubs) < limit {
			limit = len(activeSubs)
		}
		for _, sub := range activeSubs[:limit] {
			tenantID := sub.GetString("tenant_id")
			tenant, err := app.FindRecordById("tenants", tenantID)
			if err != nil {
				continue
			}
			tier := sub.GetString("tier_key")
			topSubs = append(topSubs, topSub{
				Email:    tenant.GetString("email"),
				Tier:     tier,
				MrrUsd:   TierPriceUSD[tier],
				Renewal:  sub.GetString("expires_at"),
				Provider: sub.GetString("payment_provider"),
			})
		}

		// ── Recent signups ──────────────────────────────────────────
		type recentSignup struct {
			Email    string `json:"email"`
			Created  string `json:"created"`
			Verified bool   `json:"verified"`
			Tier     string `json:"tier"`
		}
		recentSignups := make([]recentSignup, 0)
		recentTenants, _ := app.FindRecordsByFilter("tenants",
			"status != ''", "-created", 10, 0)
		for _, t := range recentTenants {
			// Find the tenant's latest subscription tier.
			tier := "free"
			subs, _ := app.FindRecordsByFilter("subscriptions",
				"tenant_id = {:tid}", "-starts_at", 1, 0,
				map[string]any{"tid": t.Id})
			if len(subs) > 0 && subs[0].GetString("tier_key") != "" {
				tier = subs[0].GetString("tier_key")
			}
			recentSignups = append(recentSignups, recentSignup{
				Email:    t.GetString("email"),
				Created:  t.GetDateTime("created").Time().Format("2006-01-02"),
				Verified: t.GetBool("email_verified"),
				Tier:     tier,
			})
		}

		// ── Expiring soon (within 30 days) ─────────────────────────
		type expiring struct {
			Email     string `json:"email"`
			Tier      string `json:"tier"`
			ExpiresAt string `json:"expiresAt"`
			DaysLeft  int    `json:"daysLeft"`
		}
		expiringSoon := make([]expiring, 0)
		cutoff := now.Add(30 * 24 * time.Hour).Format(time.RFC3339)
		expiringSubs, _ := app.FindRecordsByFilter("subscriptions",
			"status = 'active' && tier_key != 'free' && expires_at <= {:cutoff} && expires_at >= {:now}",
			"-expires_at", 50, 0,
			map[string]any{"cutoff": cutoff, "now": now.Format(time.RFC3339)})
		for _, sub := range expiringSubs {
			tenantID := sub.GetString("tenant_id")
			tenant, err := app.FindRecordById("tenants", tenantID)
			if err != nil {
				continue
			}
			expAt := sub.GetDateTime("expires_at").Time()
			expiringSoon = append(expiringSoon, expiring{
				Email:     tenant.GetString("email"),
				Tier:      sub.GetString("tier_key"),
				ExpiresAt: expAt.Format("2006-01-02"),
				DaysLeft:  int(time.Until(expAt).Hours() / 24),
			})
		}

		// ── Response ────────────────────────────────────────────────
		return e.JSON(http.StatusOK, map[string]any{
			"kpis": map[string]any{
				"totalUsers":       totalUsers,
				"activeUsers":      activeUsers,
				"totalSubscribers": totalSubscribers,
				"mrrUsd":           math.Round(mrrUsd*100) / 100,
				"mrrIdr":           math.Round(mrrUsd * fxRate),
				"arpuUsd":          arpuUsd,
				"activeDevices":    activeDevices,
				"trialToPaidRate":  trialToPaidRate,
				"fxRate":           fxRate,
				"fxLive":           fxLive,
				"fxUpdatedAt":      fxUpdatedAt.Format(time.RFC3339),
			},
			"revenueTrend":     revenueTrend,
			"subscriberGrowth": subGrowth,
			"signupsPerMonth":  signupsArr,
			"churnPerMonth":    churnArr,
			"tierDistribution": tierDist,
			"providerSplit":    providerSplit,
			"topSubscribers":   topSubs,
			"recentSignups":    recentSignups,
			"expiringSoon":     expiringSoon,
		})
	}
}

// countRecords returns the number of records matching the filter.
func countRecordsByFilter(app core.App, collection, filter string) int {
	if filter == "" {
		recs, err := app.FindRecordsByFilter(collection, "id != ''", "", 0, 0)
		if err != nil {
			return 0
		}
		return len(recs)
	}
	recs, err := app.FindRecordsByFilter(collection, filter, "", 0, 0)
	if err != nil {
		return 0
	}
	return len(recs)
}

func init() {
	// Ensure TierPriceUSD can be overridden via env for enterprise.
	v := strings.TrimSpace(os.Getenv("OZ_ENTERPRISE_MRR_USD"))
	if v != "" {
		var f float64
		if _, err := fmt.Sscanf(v, "%f", &f); err == nil && f > 0 {
			TierPriceUSD["enterprise"] = f
		}
	}
}

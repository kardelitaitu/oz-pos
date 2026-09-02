package main

// Provider-revenue ledger tests — bucketing, source labels, cache TTL,
// and DB-edit immunity: admin subscription changes must not move the
// provider-verified income/gross figures.

import (
	"encoding/json"
	"net/http"
	"sync"
	"testing"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// ── Bucketing and sources ───────────────────────────────────────────

func TestLoadProviderRevenueBucketsAndSources(t *testing.T) {
	resetProviderRevenueCache()
	app, _ := dashboardMux(t)
	defer app.Cleanup()

	tenantID, _ := seedDashboardTenant(t, app, "prov-rev-bucket@test.com")

	// Seed a Paddle event and a Midtrans event — both bucket to the current
	// month (PocketBase autodate overwrites manual created).  Bucketing is
	// simple YYYY-MM string formatting; the key behaviour tested here is
	// correct per-provider aggregation, lifetime sums, and source tracking.
	now := time.Now().UTC()
	curKey := now.Format("2006-01")
	col, _ := app.FindCollectionByNameOrId("revenue_events")
	pad := core.NewRecord(col)
	pad.Set("event_id", "evt-bucket-paddle")
	pad.Set("provider", "paddle")
	pad.Set("tenant_id", tenantID)
	pad.Set("currency", "USD")
	pad.Set("amount_usd", 42.50)
	pad.Set("amount_idr", 680000)
	if err := app.Save(pad); err != nil {
		t.Fatalf("save paddle event: %v", err)
	}
	mid := core.NewRecord(col)
	mid.Set("event_id", "evt-bucket-midtrans")
	mid.Set("provider", "midtrans")
	mid.Set("tenant_id", tenantID)
	mid.Set("currency", "IDR")
	mid.Set("amount_usd", 9.30)
	mid.Set("amount_idr", 149000)
	if err := app.Save(mid); err != nil {
		t.Fatalf("save midtrans event: %v", err)
	}

	// Load without cache.
	rev := loadProviderRevenue(app)

	if rev.LifetimeUsd != 51.80 {
		t.Errorf("lifetimeUsd = %v, want 51.80", rev.LifetimeUsd)
	}
	if rev.LifetimeIdr != 829000 {
		t.Errorf("lifetimeIdr = %v, want 829000", rev.LifetimeIdr)
	}

	m, ok := rev.ByMonth[curKey]
	if !ok {
		t.Fatalf("missing current month %s", curKey)
	}
	if m.Usd != 51.80 {
		t.Errorf("month usd = %v, want 51.80", m.Usd)
	}
	if m.Idr != 829000 {
		t.Errorf("month idr = %v, want 829000", m.Idr)
	}
	if m.Count != 2 {
		t.Errorf("month count = %d, want 2", m.Count)
	}
	if !m.Sources["paddle"] {
		t.Error("should have paddle source")
	}
	if !m.Sources["midtrans"] {
		t.Error("should have midtrans source")
	}
}

// ── Source label helpers ────────────────────────────────────────────

func TestProviderRevenueSourceLabels(t *testing.T) {
	// Single provider → "paddle_webhook" / "midtrans_webhook"
	pOnly := monthRevenue{Count: 1, Usd: 10, Idr: 160000, Sources: map[string]bool{"paddle": true}}
	if s := providerRevenueSource(pOnly); s != "paddle_webhook" {
		t.Errorf("paddle-only: got %q, want paddle_webhook", s)
	}
	mOnly := monthRevenue{Count: 1, Usd: 9.30, Idr: 149000, Sources: map[string]bool{"midtrans": true}}
	if s := providerRevenueSource(mOnly); s != "midtrans_webhook" {
		t.Errorf("midtrans-only: got %q, want midtrans_webhook", s)
	}
	// Both providers → "provider"
	both := monthRevenue{Count: 2, Usd: 51.80, Idr: 829000, Sources: map[string]bool{"paddle": true, "midtrans": true}}
	if s := providerRevenueSource(both); s != "provider" {
		t.Errorf("both: got %q, want provider", s)
	}
	// Empty / no events → "estimate"
	empty := monthRevenue{}
	if s := providerRevenueSource(empty); s != "estimate" {
		t.Errorf("empty: got %q, want estimate", s)
	}
}

// ── Cache TTL ───────────────────────────────────────────────────────

func TestGetProviderRevenueCacheTTL(t *testing.T) {
	resetProviderRevenueCache()
	origTTL := providerRevTTL
	providerRevTTL = 5 * time.Minute
	defer func() { providerRevTTL = origTTL }()

	app, _ := dashboardMux(t)
	defer app.Cleanup()

	tenantID, _ := seedDashboardTenant(t, app, "cache-ttl@test.com")

	// Seed one event.
	col, _ := app.FindCollectionByNameOrId("revenue_events")
	e1 := core.NewRecord(col)
	e1.Set("event_id", "evt-cache-001")
	e1.Set("provider", "paddle")
	e1.Set("tenant_id", tenantID)
	e1.Set("currency", "USD")
	e1.Set("amount_usd", 10.00)
	e1.Set("amount_idr", 160000)
	e1.Set("created", time.Now().UTC().Format(time.RFC3339))
	if err := app.Save(e1); err != nil {
		t.Fatalf("save e1: %v", err)
	}

	// First load: cache populated.
	rev1 := getProviderRevenue(app)
	if rev1.LifetimeUsd != 10.00 {
		t.Fatalf("first load lifetimeUsd = %v, want 10.00", rev1.LifetimeUsd)
	}

	// Add a second event within the cache TTL.
	e2 := core.NewRecord(col)
	e2.Set("event_id", "evt-cache-002")
	e2.Set("provider", "midtrans")
	e2.Set("tenant_id", tenantID)
	e2.Set("currency", "IDR")
	e2.Set("amount_usd", 9.30)
	e2.Set("amount_idr", 149000)
	e2.Set("created", time.Now().UTC().Format(time.RFC3339))
	if err := app.Save(e2); err != nil {
		t.Fatalf("save e2: %v", err)
	}

	// Within TTL: cache should still return the old value (10.00 only).
	rev2 := getProviderRevenue(app)
	if rev2.LifetimeUsd != 10.00 {
		t.Errorf("within TTL: lifetimeUsd = %v, want 10.00 (stale)", rev2.LifetimeUsd)
	}

	// Reset cache and verify it picks up the new event.
	resetProviderRevenueCache()
	rev3 := getProviderRevenue(app)
	if rev3.LifetimeUsd != 19.30 {
		t.Errorf("after cache reset: lifetimeUsd = %v, want 19.30", rev3.LifetimeUsd)
	}
}

// ── DB-edit immunity: stats endpoint uses provider ledger ───────────

func TestAdminStats_MonthlyGrossProviderSourced(t *testing.T) {
	resetProviderRevenueCache()
	app, mux := dashboardMux(t)
	defer app.Cleanup()

	tenantID, _ := seedDashboardTenant(t, app, "db-immunity@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	// Seed a revenue_events record in the current month.
	now := time.Now().UTC()
	curKey := now.Format("2006-01")
	col, _ := app.FindCollectionByNameOrId("revenue_events")
	rev := core.NewRecord(col)
	rev.Set("event_id", "evt-db-immunity-001")
	rev.Set("provider", "paddle")
	rev.Set("tenant_id", tenantID)
	rev.Set("currency", "USD")
	rev.Set("amount_usd", 42.50)
	rev.Set("amount_idr", 680000)
	rev.Set("created", now.Format(time.RFC3339))
	if err := app.Save(rev); err != nil {
		t.Fatalf("save revenue event: %v", err)
	}

	// Fetch stats: should contain provider-verified monthly gross.
	rec := doJSON(mux, http.MethodGet, "/api/v1/admin/stats", "Bearer secret-admin-key", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body struct {
		KPIs struct {
			MonthlyGrossUsd float64 `json:"monthlyGrossUsd"`
			MonthlyGrossIdr float64 `json:"monthlyGrossIdr"`
			GrossSource     string  `json:"grossSource"`
			MmrUsd          float64 `json:"mrrUsd"`
			LifetimeUsd     float64 `json:"lifetimeUsd"`
		} `json:"kpis"`
		RevenueTrend []struct {
			Month  string  `json:"month"`
			Usd    float64 `json:"usd"`
			Idr    float64 `json:"idr"`
			Source string  `json:"source,omitempty"`
		} `json:"revenueTrend"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}

	// Monthly gross must be from the provider-verified event.
	if body.KPIs.MonthlyGrossUsd != 42.50 {
		t.Errorf("monthlyGrossUsd = %v, want 42.50", body.KPIs.MonthlyGrossUsd)
	}
	if body.KPIs.MonthlyGrossIdr != 680000 {
		t.Errorf("monthlyGrossIdr = %v, want 680000", body.KPIs.MonthlyGrossIdr)
	}
	if body.KPIs.GrossSource != "paddle_webhook" {
		t.Errorf("grossSource = %q, want paddle_webhook", body.KPIs.GrossSource)
	}
	if body.KPIs.LifetimeUsd != 42.50 {
		t.Errorf("lifetimeUsd = %v, want 42.50", body.KPIs.LifetimeUsd)
	}

	// Revenue trend for current month must be provider-verified.
	found := false
	for _, m := range body.RevenueTrend {
		if m.Month == curKey {
			found = true
			if m.Source != "paddle_webhook" && m.Source != "provider" {
				t.Errorf("current month source = %q, want paddle_webhook/provider", m.Source)
			}
			if m.Usd < 42.00 || m.Usd > 43.00 {
				t.Errorf("current month usd = %v, want ~42.50", m.Usd)
			}
			break
		}
	}
	if !found {
		t.Errorf("current month %s not found in revenueTrend", curKey)
	}

	// DB-edit immunity: edit the subscription tier (simulate admin override).
	subs, _ := app.FindRecordsByFilter("subscriptions",
		"tenant_id = {:tid}", "", 0, 0,
		map[string]any{"tid": tenantID})
	if len(subs) > 0 {
		subs[0].Set("tier_key", "premium")
		subs[0].Set("status", "active")
		if err := app.Save(subs[0]); err != nil {
			t.Fatalf("save subscription tier override: %v", err)
		}
	}

	// Reset cache so stats re-reads revenue_events (not the subscription).
	resetProviderRevenueCache()
	rec2 := doJSON(mux, http.MethodGet, "/api/v1/admin/stats", "Bearer secret-admin-key", "")
	if rec2.Code != http.StatusOK {
		t.Fatalf("expected 200 post-edit, got %d", rec2.Code)
	}
	var body2 struct {
		KPIs struct {
			MonthlyGrossUsd float64 `json:"monthlyGrossUsd"`
			MonthlyGrossIdr float64 `json:"monthlyGrossIdr"`
			GrossSource     string  `json:"grossSource"`
			MmrUsd          float64 `json:"mrrUsd"`
		} `json:"kpis"`
	}
	if err := json.Unmarshal(rec2.Body.Bytes(), &body2); err != nil {
		t.Fatalf("bad JSON after edit: %v", err)
	}

	// Monthly gross must STILL be 42.50 (from the provider event, not the
	// new premium tier price $39.99), proving DB-edit immunity.
	if body2.KPIs.MonthlyGrossUsd != 42.50 {
		t.Errorf("after tier edit: monthlyGrossUsd = %v, want 42.50 (DB-edit immunity)", body2.KPIs.MonthlyGrossUsd)
	}
	if body2.KPIs.MonthlyGrossIdr != 680000 {
		t.Errorf("after tier edit: monthlyGrossIdr = %v, want 680000", body2.KPIs.MonthlyGrossIdr)
	}
	// MRR (subscription estimate) DOES change — that's expected and labeled.
	if body2.KPIs.MmrUsd != 39.99 {
		t.Errorf("mrrUsd after tier edit = %v, want 39.99 (premium tier price)", body2.KPIs.MmrUsd)
	}
}

// ── Concurrency smoke test ──────────────────────────────────────────

func TestGetProviderRevenueConcurrent(t *testing.T) {
	app, _ := dashboardMux(t)
	defer app.Cleanup()

	var wg sync.WaitGroup
	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			_ = getProviderRevenue(app)
		}()
	}
	wg.Wait()
}

// ── revenueCachedAt + ?refresh=1 cache bypass ───────────────────────

func TestAdminStats_RevenueCachedAtAndRefresh(t *testing.T) {
	resetProviderRevenueCache()
	app, mux := dashboardMux(t)
	defer app.Cleanup()

	tenantID, _ := seedDashboardTenant(t, app, "cache-refresh@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	// Seed one revenue event, then fetch stats.
	now := time.Now().UTC()
	col, _ := app.FindCollectionByNameOrId("revenue_events")
	rev := core.NewRecord(col)
	rev.Set("event_id", "evt-cache-refresh-001")
	rev.Set("provider", "paddle")
	rev.Set("tenant_id", tenantID)
	rev.Set("currency", "USD")
	rev.Set("amount_usd", 10.00)
	rev.Set("amount_idr", 160000)
	rev.Set("created", now.Format(time.RFC3339))
	if err := app.Save(rev); err != nil {
		t.Fatalf("save revenue event: %v", err)
	}

	rec := doJSON(mux, http.MethodGet, "/api/v1/admin/stats", "Bearer secret-admin-key", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rec.Code)
	}
	var body struct {
		KPIs struct {
			RevenueCachedAt string  `json:"revenueCachedAt"`
			LifetimeUsd     float64 `json:"lifetimeUsd"`
		} `json:"kpis"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	if body.KPIs.RevenueCachedAt == "" {
		t.Error("expected revenueCachedAt to be set")
	}
	if _, err := time.Parse(time.RFC3339, body.KPIs.RevenueCachedAt); err != nil {
		t.Errorf("revenueCachedAt=%q is not RFC3339: %v", body.KPIs.RevenueCachedAt, err)
	}

	// Add a second event, then fetch WITHOUT ?refresh=1 — the 5-minute
	// cache must serve the stale lifetime (10.00 only).
	rev2 := core.NewRecord(col)
	rev2.Set("event_id", "evt-cache-refresh-002")
	rev2.Set("provider", "midtrans")
	rev2.Set("tenant_id", tenantID)
	rev2.Set("currency", "IDR")
	rev2.Set("amount_usd", 9.30)
	rev2.Set("amount_idr", 149000)
	rev2.Set("created", now.Format(time.RFC3339))
	if err := app.Save(rev2); err != nil {
		t.Fatalf("save second event: %v", err)
	}

	rec2 := doJSON(mux, http.MethodGet, "/api/v1/admin/stats", "Bearer secret-admin-key", "")
	var body2 struct {
		KPIs struct {
			LifetimeUsd float64 `json:"lifetimeUsd"`
		} `json:"kpis"`
	}
	if err := json.Unmarshal(rec2.Body.Bytes(), &body2); err != nil {
		t.Fatalf("bad JSON (cached): %v", err)
	}
	if body2.KPIs.LifetimeUsd != 10.00 {
		t.Errorf("cached lifetimeUsd = %v, want 10.00 (cache serves stale)", body2.KPIs.LifetimeUsd)
	}

	// Fetch WITH ?refresh=1 — the cache is bypassed and the new event shows.
	rec3 := doJSON(mux, http.MethodGet, "/api/v1/admin/stats?refresh=1", "Bearer secret-admin-key", "")
	var body3 struct {
		KPIs struct {
			LifetimeUsd float64 `json:"lifetimeUsd"`
		} `json:"kpis"`
	}
	if err := json.Unmarshal(rec3.Body.Bytes(), &body3); err != nil {
		t.Fatalf("bad JSON (refreshed): %v", err)
	}
	if body3.KPIs.LifetimeUsd != 19.30 {
		t.Errorf("refreshed lifetimeUsd = %v, want 19.30", body3.KPIs.LifetimeUsd)
	}
}

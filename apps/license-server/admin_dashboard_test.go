package main

// Tests for the admin dashboard ACTION endpoints (admin_dashboard.go) —
// bug hunt round 7. B29: renew anchored the new expiry at time.Now(),
// silently destroying every remaining paid day of a live subscription.
// B30: tier-override accepted any string (unknown keys zeroed MRR).
// B31: the health endpoint hardcoded a stale version.

import (
	"encoding/json"
	"net/http"
	"testing"
	"time"
)

// ── B29: renew truncates remaining paid time ─────────────────────────

func TestAdminRenewB29_ExtendsFromCurrentExpiryNotNow(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	tenantID, _ := seedDashboardTenant(t, app, "renew-b29@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	// seedDashboardTenant's subscription expires 2027-01-01T00:00:00Z —
	// in the future at test time. Renewing +30d must extend THAT date.
	// The old code anchored at time.Now(): a subscription with months
	// of paid time left was silently truncated to now+30d.
	rec := doJSON(mux, http.MethodPost,
		"/api/v1/admin/tenants/"+tenantID+"/renew",
		"Bearer secret-admin-key", `{"days": 30}`)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	exp, err := time.Parse(time.RFC3339, body["expires_at"].(string))
	if err != nil {
		t.Fatalf("parse expires_at %v: %v", body["expires_at"], err)
	}
	want := time.Date(2027, 1, 31, 0, 0, 0, 0, time.UTC)
	if !exp.Equal(want) {
		t.Fatalf("B29: expires_at=%s, want %s (extend from current expiry, not now)",
			exp.Format(time.RFC3339), want.Format(time.RFC3339))
	}
}

func TestAdminRenewB29_ExpiredSubStillRenewsFromNow(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	tenantID, _ := seedDashboardTenant(t, app, "renew-b29-exp@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	// Push the seeded subscription's expiry into the past.
	subs, err := app.FindRecordsByFilter("subscriptions",
		"tenant_id = {:tid}", "-starts_at", 1, 0, map[string]any{"tid": tenantID})
	if err != nil || len(subs) == 0 {
		t.Fatalf("seed sub lookup: %v", err)
	}
	subs[0].Set("expires_at", "2020-01-01T00:00:00Z")
	subs[0].Set("status", "expired")
	if err := app.Save(subs[0]); err != nil {
		t.Fatalf("expire sub: %v", err)
	}

	rec := doJSON(mux, http.MethodPost,
		"/api/v1/admin/tenants/"+tenantID+"/renew",
		"Bearer secret-admin-key", `{"days": 30}`)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body map[string]any
	_ = json.Unmarshal(rec.Body.Bytes(), &body)
	exp, err := time.Parse(time.RFC3339, body["expires_at"].(string))
	if err != nil {
		t.Fatalf("parse expires_at: %v", err)
	}
	// Expired subs renew from NOW (max(now, oldExpiry) semantics) — not
	// stacked onto a two-year-old expiry.
	if exp.Before(time.Now().Add(29*24*time.Hour)) || exp.After(time.Now().Add(31*24*time.Hour)) {
		t.Fatalf("B29: expired sub should renew ~now+30d, got %s", exp.Format(time.RFC3339))
	}
}

// ── B30: tier-override accepts any string ────────────────────────────

func TestAdminTierOverrideB30_RejectsUnknownTier(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	tenantID, _ := seedDashboardTenant(t, app, "override-b30@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	// An unknown tier_key silently zeroed the subscription's MRR
	// contribution (TierPriceUSD lookup misses → 0) and made the
	// dashboard pill render the raw garbage string.
	rec := doJSON(mux, http.MethodPost,
		"/api/v1/admin/tenants/"+tenantID+"/tier-override",
		"Bearer secret-admin-key", `{"tier_key":"not-a-tier","reason":"probe"}`)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("B30: unknown tier must be 400, got %d: %s", rec.Code, rec.Body.String())
	}
	// And the subscription must be unchanged.
	subs, _ := app.FindRecordsByFilter("subscriptions",
		"tenant_id = {:tid}", "-starts_at", 1, 0, map[string]any{"tid": tenantID})
	if len(subs) == 0 || subs[0].GetString("tier_key") != "pro" {
		t.Fatalf("B30: rejected override must not mutate the subscription, got %v", subs)
	}
}

// ── B31: health reports a stale version ──────────────────────────────

func TestAdminHealthB31_ReportsCurrentVersion(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	seedDashboardTenant(t, app, "health-b31@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	rec := doJSON(mux, http.MethodGet, "/api/v1/admin/health", "Bearer secret-admin-key", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rec.Code)
	}
	var body map[string]any
	_ = json.Unmarshal(rec.Body.Bytes(), &body)
	// The repo is version-locked at 0.0.33 (AGENTS.md). The endpoint
	// hardcoded 0.0.31 — the admin health card lied about what was
	// deployed. This assertion is the bump reminder: update the const
	// (and the lock) together.
	if body["version"] != "0.0.33" {
		t.Fatalf("B31: health version=%v, want 0.0.33 (version lock — update with the bump)", body["version"])
	}
}

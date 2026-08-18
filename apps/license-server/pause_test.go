package main

import (
	"encoding/json"
	"testing"
	"time"

	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
)

// seedSubscriptionForPause creates a subscription record for pause/resume tests.
func seedSubscriptionForPause(t *testing.T, app *tests.TestApp, tenantID, tierKey, status string) {
	t.Helper()
	coll, err := app.FindCollectionByNameOrId("subscriptions")
	if err != nil {
		t.Fatalf("subscriptions collection not found: %v", err)
	}
	rec := core.NewRecord(coll)
	rec.Set("tenant_id", []string{tenantID})
	rec.Set("tier_key", tierKey)
	rec.Set("status", status)
	rec.Set("starts_at", "2026-01-01T00:00:00Z")
	rec.Set("expires_at", "2027-01-01T00:00:00Z")
	rec.Set("signed_payload", "test-payload")
	rec.Set("signature", "test-sig")
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed subscription: %v", err)
	}
}

// ── Pause subscription tests ───────────────────────────────────────

func TestPauseSubscription(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	tenantID := "pausetenant0001" // 15 chars
	apiKey := "oz_pausetestkey01"
	seedTenant(t, app, tenantID, apiKey, "active")
	seedSubscriptionForPause(t, app, tenantID, "plus", "active")

	// Pause for 1 month
	body := servePost(t, se, "/api/v1/license/pause", "Bearer "+apiKey, nil,
		`{"pause_months":1}`)
	if body.Code != 200 {
		t.Fatalf("expected 200 for pause, got %d: %s", body.Code, body.Body.String())
	}

	var resp map[string]any
	if err := json.Unmarshal(body.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}
	if resp["status"] != "paused" {
		t.Errorf("expected status 'paused', got %v", resp["status"])
	}
	if resp["tier_key"] != "plus" {
		t.Errorf("expected tier_key 'plus', got %v", resp["tier_key"])
	}

	// Verify subscription is now paused in DB
	sub, err := app.FindFirstRecordByFilter("subscriptions",
		"tenant_id = {:tenant_id} && status = 'paused'",
		map[string]any{"tenant_id": tenantID})
	if err != nil {
		t.Fatalf("subscription not paused in DB: %v", err)
	}

	// Verify paused_until is ~1 month from now
	pausedUntil := sub.GetDateTime("paused_until").Time()
	expected := time.Now().UTC().AddDate(0, 1, 0)
	diff := pausedUntil.Sub(expected)
	if diff < -2*time.Minute || diff > 2*time.Minute {
		t.Errorf("paused_until should be ~1 month from now, got %v (diff %v)", pausedUntil, diff)
	}

	// Verify paused_at is set
	pausedAt := sub.GetDateTime("paused_at")
	if pausedAt.IsZero() {
		t.Error("paused_at should be set")
	}

	t.Logf("pause: subscription paused for 1 month until %s", pausedUntil.Format(time.RFC3339))
}

func TestPauseSubscription_InvalidMonths(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	tenantID := "pauseinvalid001" // 15 chars
	apiKey := "oz_pauseinvalid01"
	seedTenant(t, app, tenantID, apiKey, "active")
	seedSubscriptionForPause(t, app, tenantID, "plus", "active")

	// Try pause with invalid months (0)
	body := servePost(t, se, "/api/v1/license/pause", "Bearer "+apiKey, nil,
		`{"pause_months":0}`)
	if body.Code != 400 {
		t.Fatalf("expected 400 for pause_months=0, got %d: %s", body.Code, body.Body.String())
	}

	// Try pause with invalid months (4)
	body = servePost(t, se, "/api/v1/license/pause", "Bearer "+apiKey, nil,
		`{"pause_months":4}`)
	if body.Code != 400 {
		t.Fatalf("expected 400 for pause_months=4, got %d: %s", body.Code, body.Body.String())
	}
}

func TestPauseSubscription_NoActiveSubscription(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	tenantID := "pausenosub00001" // 15 chars
	apiKey := "oz_pausenosub01"
	seedTenant(t, app, tenantID, apiKey, "active")
	// No subscription seeded

	// Try to pause without an active subscription
	body := servePost(t, se, "/api/v1/license/pause", "Bearer "+apiKey, nil,
		`{"pause_months":1}`)
	if body.Code != 404 {
		t.Fatalf("expected 404 for no active subscription, got %d: %s", body.Code, body.Body.String())
	}
}

func TestPauseSubscription_MaxMonths(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	tenantID := "pausemaxm000001" // 15 chars
	apiKey := "oz_pausemaxm01"
	seedTenant(t, app, tenantID, apiKey, "active")
	seedSubscriptionForPause(t, app, tenantID, "plus", "active")

	// Pause for 3 months (max allowed)
	body := servePost(t, se, "/api/v1/license/pause", "Bearer "+apiKey, nil,
		`{"pause_months":3}`)
	if body.Code != 200 {
		t.Fatalf("expected 200 for pause_months=3, got %d: %s", body.Code, body.Body.String())
	}

	// Verify paused_until is ~3 months from now
	sub, err := app.FindFirstRecordByFilter("subscriptions",
		"tenant_id = {:tenant_id} && status = 'paused'",
		map[string]any{"tenant_id": tenantID})
	if err != nil {
		t.Fatalf("subscription not paused in DB: %v", err)
	}
	pausedUntil := sub.GetDateTime("paused_until").Time()
	expected := time.Now().UTC().AddDate(0, 3, 0)
	diff := pausedUntil.Sub(expected)
	if diff < -2*time.Minute || diff > 2*time.Minute {
		t.Errorf("paused_until should be ~3 months from now, got %v (diff %v)", pausedUntil, diff)
	}
}

// ── Resume subscription tests ──────────────────────────────────────

func TestResumeSubscription(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	tenantID := "resumetenant001" // 15 chars
	apiKey := "oz_resumetest01"
	seedTenant(t, app, tenantID, apiKey, "active")
	seedSubscriptionForPause(t, app, tenantID, "plus", "paused")

	// Resume
	body := servePost(t, se, "/api/v1/license/resume", "Bearer "+apiKey, nil, "")
	if body.Code != 200 {
		t.Fatalf("expected 200 for resume, got %d: %s", body.Code, body.Body.String())
	}

	var resp map[string]any
	if err := json.Unmarshal(body.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}
	if resp["status"] != "active" {
		t.Errorf("expected status 'active', got %v", resp["status"])
	}
	if resp["tier_key"] != "plus" {
		t.Errorf("expected tier_key 'plus', got %v", resp["tier_key"])
	}

	// Verify active in DB
	sub, err := app.FindFirstRecordByFilter("subscriptions",
		"tenant_id = {:tenant_id} && status = 'active'",
		map[string]any{"tenant_id": tenantID})
	if err != nil {
		t.Fatalf("subscription not active in DB: %v", err)
	}

	// Verify paused_at and paused_until are cleared
	pausedAt := sub.GetDateTime("paused_at")
	if !pausedAt.IsZero() {
		t.Errorf("paused_at should be zero after resume, got %v", pausedAt)
	}
	pausedUntil := sub.GetDateTime("paused_until")
	if !pausedUntil.IsZero() {
		t.Errorf("paused_until should be zero after resume, got %v", pausedUntil)
	}

	t.Log("resume: subscription resumed successfully")
}

func TestResumeSubscription_NotPaused(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	tenantID := "resumenotp00001" // 15 chars
	apiKey := "oz_resumenotp01"
	seedTenant(t, app, tenantID, apiKey, "active")
	seedSubscriptionForPause(t, app, tenantID, "plus", "active")

	// Try to resume an active subscription
	body := servePost(t, se, "/api/v1/license/resume", "Bearer "+apiKey, nil, "")
	if body.Code != 404 {
		t.Fatalf("expected 404 for no paused subscription, got %d: %s", body.Code, body.Body.String())
	}
}

func TestResumeSubscription_AuthRequired(t *testing.T) {
	resetRateLimiters()
	_, se := setupDirectApp(t)

	// Try resume without auth
	body := servePost(t, se, "/api/v1/license/resume", "", nil, "")
	if body.Code != 401 {
		t.Fatalf("expected 401 for missing auth, got %d: %s", body.Code, body.Body.String())
	}
}

func TestPauseSubscription_AuthRequired(t *testing.T) {
	resetRateLimiters()
	_, se := setupDirectApp(t)

	// Try pause without auth
	body := servePost(t, se, "/api/v1/license/pause", "", nil, `{"pause_months":1}`)
	if body.Code != 401 {
		t.Fatalf("expected 401 for missing auth, got %d: %s", body.Code, body.Body.String())
	}
}

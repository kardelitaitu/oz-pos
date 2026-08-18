package main

import (
	"encoding/json"
	"net/http"
	"strings"
	"testing"

	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
)

// 15-char lowercase-hex machine fingerprints (the format the desktop's
// SHA-256-derived machine_id uses) for the trial-lock tests.
const (
	trialFP1 = "a1b2c3d4e5f6078" // 15 hex chars
	trialFP2 = "feedface0123456" // 15 hex chars
)

// seedTrialRegistration inserts a trial_registrations row directly (the
// claim endpoint's happy path is covered by TestTrialClaimEndpoint).
func seedTrialRegistration(t *testing.T, app *tests.TestApp, fp, tenantID string) {
	t.Helper()
	coll, err := app.FindCollectionByNameOrId("trial_registrations")
	if err != nil {
		t.Fatalf("trial_registrations collection not found: %v", err)
	}
	rec := core.NewRecord(coll)
	rec.Set("hardware_fingerprint", fp)
	rec.Set("first_seen_at", "2026-08-01T00:00:00Z")
	rec.Set("trial_expires_at", "2026-08-15T00:00:00Z")
	rec.Set("platform", "windows")
	rec.Set("app_version", "0.0.28")
	if tenantID != "" {
		rec.Set("tenant_id", []string{tenantID})
	}
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed trial registration: %v", err)
	}
}

// ── POST /api/v1/license/trial ─────────────────────────────────────

// TestTrialClaimEndpoint verifies the claim endpoint: a first claim
// returns 200 with the trial window, and a second claim for the SAME
// fingerprint (regardless of email) answers 403 TRIAL_ALREADY_CLAIMED —
// the one-trial-per-device gate. A malformed fingerprint answers 400.
func TestTrialClaimEndpoint(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// ── First claim: 200 + active + a trial window. ──
	rec := servePost(t, se, trialPath, "", nil,
		`{"hardware_fingerprint":"`+trialFP1+`","platform":"windows","app_version":"0.0.28","email":"firstbuyer@example.com"}`)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 on first claim, got %d: %s", rec.Code, rec.Body.String())
	}
	var body map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse claim response: %v", err)
	}
	if body["status"] != "active" {
		t.Errorf("expected status active, got %v", body["status"])
	}
	if body["hardware_fingerprint"] != trialFP1 {
		t.Errorf("expected fingerprint echoed, got %v", body["hardware_fingerprint"])
	}
	if days, _ := body["days_remaining"].(float64); days != 14 {
		t.Errorf("expected default 14-day trial, got %v", body["days_remaining"])
	}

	// The registration landed in the collection, associated with the tenant.
	reg, err := app.FindFirstRecordByData("trial_registrations", "hardware_fingerprint", trialFP1)
	if err != nil {
		t.Fatalf("claim not persisted: %v", err)
	}
	if reg.GetString("platform") != "windows" || reg.GetString("app_version") != "0.0.28" {
		t.Errorf("claim metadata not stored: platform=%q app_version=%q", reg.GetString("platform"), reg.GetString("app_version"))
	}

	// ── Second claim (different email, same device): 403 ──
	rec2 := servePost(t, se, trialPath, "", nil,
		`{"hardware_fingerprint":"`+trialFP1+`","platform":"windows","app_version":"0.0.28","email":"secondbuyer@example.com"}`)
	if rec2.Code != http.StatusForbidden {
		t.Fatalf("expected 403 on reuse, got %d: %s", rec2.Code, rec2.Body.String())
	}
	if !strings.Contains(rec2.Body.String(), "TRIAL_ALREADY_CLAIMED") {
		t.Errorf("expected TRIAL_ALREADY_CLAIMED code, got: %s", rec2.Body.String())
	}

	// ── Malformed fingerprint: 400 ──
	rec3 := servePost(t, se, trialPath, "", nil,
		`{"hardware_fingerprint":"not-a-fingerprint","platform":"windows","app_version":"0.0.28"}`)
	if rec3.Code != http.StatusBadRequest {
		t.Fatalf("expected 400 for malformed fingerprint, got %d: %s", rec3.Code, rec3.Body.String())
	}
}

// TestTrialClaimEndpoint_VerticalDays verifies the claim endpoint records
// the segmented vertical's trial length (restaurant/cafe → 14-day Pro is
// the same length, but enterprise_referral → 30 days is observable).
func TestTrialClaimEndpoint_VerticalDays(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	rec := servePost(t, se, trialPath, "", nil,
		`{"hardware_fingerprint":"`+trialFP2+`","platform":"windows","app_version":"0.0.28","trial_vertical":"enterprise_referral"}`)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse claim response: %v", err)
	}
	if days, _ := body["days_remaining"].(float64); days != 30 {
		t.Errorf("expected 30-day enterprise-referral trial, got %v", body["days_remaining"])
	}
}

// ── Activation-time gate ───────────────────────────────────────────

// TestActivateTrialLock_DifferentTenantRejected is the reset-abuse case:
// tenant A claims a trial on a device, then tenant B tries to activate a
// trial key on the SAME device — 403, nothing minted.
func TestActivateTrialLock_DifferentTenantRejected(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTrialKey(t, app, "OZ-TRIAL-LOCK-A", "plus", "unused", "2099-12-31 23:59:59.000Z")
	seedTrialKey(t, app, "OZ-TRIAL-LOCK-B", "plus", "unused", "2099-12-31 23:59:59.000Z")

	// Tenant A (new tenant, no api_key yet) activates first on this device
	// → the claim is registered and the trial mints, 200.
	recA := servePost(t, se, "/api/v1/license/activate", "", nil,
		`{"key":"OZ-TRIAL-LOCK-A","email":"trialtenanta001@example.com","machine_id":"`+trialFP1+`","phone":"081234567890"}`)
	if recA.Code != http.StatusOK {
		t.Fatalf("expected 200 for tenant A first trial, got %d: %s", recA.Code, recA.Body.String())
	}

	// Tenant B (fresh email, new tenant) on the SAME physical device: a NEW
	// per-installation machine_id but the SAME hardware fingerprint (the
	// reset-abuse reinstall shape) → the lock fires 403 before anything
	// is minted for B.
	recB := servePost(t, se, "/api/v1/license/activate", "", nil,
		`{"key":"OZ-TRIAL-LOCK-B","email":"trialtenantb001@example.com","machine_id":"`+trialFP2+`","hardware_fingerprint":"`+trialFP1+`","phone":"081234567891"}`)
	if recB.Code != http.StatusForbidden {
		t.Fatalf("expected 403 for tenant B on claimed device, got %d: %s", recB.Code, recB.Body.String())
	}
	if !strings.Contains(recB.Body.String(), "TRIAL_ALREADY_CLAIMED") {
		t.Errorf("expected TRIAL_ALREADY_CLAIMED, got: %s", recB.Body.String())
	}
	// B's subscription was NOT minted.
	subs, err := app.FindRecordsByFilter("subscriptions", "tier_key = 'plus'", "", 10, 0, nil)
	if err != nil || len(subs) != 1 {
		t.Fatalf("expected exactly ONE subscription (tenant A's), got %d (err %v)", len(subs), err)
	}
}

// TestActivateTrialLock_SameTenantReinstallAllowed is the legitimate
// re-install case: the same tenant re-activates on the same device (new
// key, same machine) — the lock lets it through.
func TestActivateTrialLock_SameTenantReinstallAllowed(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTrialKey(t, app, "OZ-TRIAL-LOCK-C1", "plus", "unused", "2099-12-31 23:59:59.000Z")
	seedTrialKey(t, app, "OZ-TRIAL-LOCK-C2", "plus", "unused", "2099-12-31 23:59:59.000Z")

	// First activation creates the tenant and mints the trial.
	rec1 := servePost(t, se, "/api/v1/license/activate", "", nil,
		`{"key":"OZ-TRIAL-LOCK-C1","email":"trialtenantc001@example.com","machine_id":"`+trialFP2+`","phone":"081234567892"}`)
	if rec1.Code != http.StatusOK {
		t.Fatalf("expected 200 on first trial, got %d: %s", rec1.Code, rec1.Body.String())
	}
	var resp1 map[string]any
	if err := json.Unmarshal(rec1.Body.Bytes(), &resp1); err != nil {
		t.Fatalf("failed to parse first activation: %v", err)
	}
	apiKey, _ := resp1["api_key"].(string)
	if apiKey == "" {
		t.Fatal("expected api_key on first activation")
	}

	// Same tenant, same device, a fresh trial key → re-activation, allowed.
	rec2 := servePost(t, se, "/api/v1/license/activate", "Bearer "+apiKey, nil,
		`{"key":"OZ-TRIAL-LOCK-C2","email":"trialtenantc001@example.com","machine_id":"`+trialFP2+`","phone":"081234567892"}`)
	if rec2.Code != http.StatusOK {
		t.Fatalf("expected 200 for same-tenant re-install, got %d: %s", rec2.Code, rec2.Body.String())
	}
}

// TestActivateTrialLock_PaidKeyUnaffected verifies the lock only fires
// for trial keys: a PAID key on an already-claimed device activates
// normally (paying customers are never locked out by the trial gate).
func TestActivateTrialLock_PaidKeyUnaffected(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Device already claimed a trial by another tenant.
	seedTenant(t, app, "trialtenantd001", "trialtenantd001-key", "active")
	seedTrialRegistration(t, app, trialFP1, "trialtenantd001")
	seedTenant(t, app, "paidtenant00001", "paidtenant00001-key", "active")
	seedLicenseKey(t, app, "OZ-PAID-LOCK-E1", "pro", "unused", "2099-12-31 23:59:59.000Z")

	// The paid tenant authenticates with its api_key; the trial lock must
	// not fire for a PAID key on an already-claimed device.
	rec := servePost(t, se, "/api/v1/license/activate", "Bearer paidtenant00001-key", nil,
		`{"key":"OZ-PAID-LOCK-E1","email":"paidtenant00001@example.com","machine_id":"`+trialFP1+`","phone":"081234567893"}`)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 for paid key on claimed device, got %d: %s", rec.Code, rec.Body.String())
	}
}

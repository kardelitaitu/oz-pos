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

// ── Lightweight repeat-email detector (trial_claims) ─────────────

// TestTrialClaimHash verifies the (email, device) fingerprint: deterministic,
// 64 lowercase hex chars, email normalization (case + whitespace), and
// device sensitivity (same email on a different device → different hash).
func TestTrialClaimHash(t *testing.T) {
	h1 := trialClaimHash("store@example.com", "a1b2c3d4e5f6078")
	if len(h1) != 64 {
		t.Fatalf("expected 64 hex chars, got %d: %s", len(h1), h1)
	}
	for _, c := range h1 {
		if !((c >= 'a' && c <= 'f') || (c >= '0' && c <= '9')) {
			t.Fatalf("non-hex char %q in hash %s", c, h1)
		}
	}
	// Deterministic.
	if trialClaimHash("store@example.com", "a1b2c3d4e5f6078") != h1 {
		t.Error("hash must be deterministic")
	}
	// Email normalization: case + surrounding whitespace.
	if trialClaimHash("  Store@Example.COM ", "a1b2c3d4e5f6078") != h1 {
		t.Error("email must be normalized (lowercase + trim) before hashing")
	}
	// Device sensitivity.
	if trialClaimHash("store@example.com", "feedface0123456") == h1 {
		t.Error("different device must produce a different hash")
	}
}

// TestTrialClaims_RepeatEmailDetected is the detector's core case: the same
// email claiming a SECOND trial key on the same device (the same-tenant
// reinstall the full SPEC-2026-TRIAL-LOCK gate allows by design). Both
// activations succeed, but the second response carries repeat_claim and the
// trial_claims row bumps to count 2 with both keys in the audit trail.
func TestTrialClaims_RepeatEmailDetected(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTrialKey(t, app, "OZ-CLAIM-REP-01", "plus", "unused", "2099-12-31 23:59:59.000Z")
	seedTrialKey(t, app, "OZ-CLAIM-REP-02", "plus", "unused", "2099-12-31 23:59:59.000Z")

	// First claim: new tenant, no api_key yet, mints normally.
	rec1 := servePost(t, se, "/api/v1/license/activate", "", nil,
		`{"key":"OZ-CLAIM-REP-01","email":"repeat@example.com","machine_id":"`+trialFP1+`","phone":"081234567894"}`)
	if rec1.Code != http.StatusOK {
		t.Fatalf("expected 200 on first trial, got %d: %s", rec1.Code, rec1.Body.String())
	}
	var resp1 map[string]any
	if err := json.Unmarshal(rec1.Body.Bytes(), &resp1); err != nil {
		t.Fatalf("failed to parse first activation: %v", err)
	}
	if _, seen := resp1["repeat_claim"]; seen {
		t.Errorf("first claim must NOT carry repeat_claim, got %v", resp1["repeat_claim"])
	}
	apiKey, _ := resp1["api_key"].(string)
	if apiKey == "" {
		t.Fatal("expected api_key on first activation")
	}

	// Second claim: same email, same device, fresh key, same tenant.
	rec2 := servePost(t, se, "/api/v1/license/activate", "Bearer "+apiKey, nil,
		`{"key":"OZ-CLAIM-REP-02","email":"repeat@example.com","machine_id":"`+trialFP1+`","phone":"081234567894"}`)
	if rec2.Code != http.StatusOK {
		t.Fatalf("expected 200 on repeat trial (same tenant reinstall), got %d: %s", rec2.Code, rec2.Body.String())
	}
	var resp2 map[string]any
	if err := json.Unmarshal(rec2.Body.Bytes(), &resp2); err != nil {
		t.Fatalf("failed to parse second activation: %v", err)
	}
	rc, ok := resp2["repeat_claim"].(map[string]any)
	if !ok {
		t.Fatalf("expected repeat_claim on second activation, got %v", resp2["repeat_claim"])
	}
	if rc["count"].(float64) != 2 {
		t.Errorf("expected repeat_claim.count 2, got %v", rc["count"])
	}
	if _, ok := rc["first_claimed_at"].(string); !ok {
		t.Errorf("expected first_claimed_at in repeat_claim, got %v", rc)
	}

	// DB state: one row, count 2, both keys in the audit trail.
	rows, err := app.FindRecordsByFilter("trial_claims", "email = 'repeat@example.com'", "", 10, 0, nil)
	if err != nil || len(rows) != 1 {
		t.Fatalf("expected exactly ONE trial_claims row, got %d (err %v)", len(rows), err)
	}
	if rows[0].GetInt("claim_count") != 2 {
		t.Errorf("expected claim_count 2, got %d", rows[0].GetInt("claim_count"))
	}
	if !strings.Contains(rows[0].GetString("trial_keys"), "OZ-CLAIM-REP-01") || !strings.Contains(rows[0].GetString("trial_keys"), "OZ-CLAIM-REP-02") {
		t.Errorf("expected both keys in trial_keys audit trail, got %q", rows[0].GetString("trial_keys"))
	}
}

// TestTrialClaims_SameEmailDifferentDeviceNotFlagged verifies the detector
// is device-scoped: the same merchant's email on a SECOND device (a legit
// Plus-tier multi-machine install) produces a different hash and no
// repeat_claim — unlike the same device, which is the abuse shape.
func TestTrialClaims_SameEmailDifferentDeviceNotFlagged(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTrialKey(t, app, "OZ-CLAIM-DEV-01", "plus", "unused", "2099-12-31 23:59:59.000Z")
	seedTrialKey(t, app, "OZ-CLAIM-DEV-02", "plus", "unused", "2099-12-31 23:59:59.000Z")

	rec1 := servePost(t, se, "/api/v1/license/activate", "", nil,
		`{"key":"OZ-CLAIM-DEV-01","email":"devices@example.com","machine_id":"`+trialFP1+`","phone":"081234567895"}`)
	if rec1.Code != http.StatusOK {
		t.Fatalf("expected 200 on first device, got %d: %s", rec1.Code, rec1.Body.String())
	}
	var resp1 map[string]any
	if err := json.Unmarshal(rec1.Body.Bytes(), &resp1); err != nil {
		t.Fatalf("failed to parse first activation: %v", err)
	}
	apiKey, _ := resp1["api_key"].(string)

	// Second device, same email (Plus allows 2 machines) — NOT flagged.
	rec2 := servePost(t, se, "/api/v1/license/activate", "Bearer "+apiKey, nil,
		`{"key":"OZ-CLAIM-DEV-02","email":"devices@example.com","machine_id":"`+trialFP2+`","phone":"081234567895"}`)
	if rec2.Code != http.StatusOK {
		t.Fatalf("expected 200 on second device, got %d: %s", rec2.Code, rec2.Body.String())
	}
	if strings.Contains(rec2.Body.String(), "repeat_claim") {
		t.Errorf("different device must NOT be flagged as a repeat, got: %s", rec2.Body.String())
	}

	rows, err := app.FindRecordsByFilter("trial_claims", "email = 'devices@example.com'", "", 10, 0, nil)
	if err != nil || len(rows) != 2 {
		t.Fatalf("expected TWO trial_claims rows (one per device), got %d (err %v)", len(rows), err)
	}
	for _, r := range rows {
		if r.GetInt("claim_count") != 1 {
			t.Errorf("expected claim_count 1 per device, got %d", r.GetInt("claim_count"))
		}
	}
}

// TestTrialClaims_ClaimEndpointRecords verifies POST /api/v1/license/trial
// also feeds the detector when the claim carries an email (so a claim that
// never reaches activation is still observable).
func TestTrialClaims_ClaimEndpointRecords(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	rec := servePost(t, se, trialPath, "", nil,
		`{"hardware_fingerprint":"`+trialFP1+`","platform":"windows","app_version":"0.0.28","email":"claimonly@example.com"}`)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 on claim, got %d: %s", rec.Code, rec.Body.String())
	}
	rows, err := app.FindRecordsByFilter("trial_claims", "email = 'claimonly@example.com'", "", 10, 0, nil)
	if err != nil || len(rows) != 1 {
		t.Fatalf("expected one trial_claims row from the claim endpoint, got %d (err %v)", len(rows), err)
	}
	if rows[0].GetInt("claim_count") != 1 {
		t.Errorf("expected claim_count 1, got %d", rows[0].GetInt("claim_count"))
	}
	if rows[0].GetString("device_id") != trialFP1 {
		t.Errorf("expected device_id = hardware fingerprint, got %q", rows[0].GetString("device_id"))
	}
}

// TestTrialClaims_PaidKeyNotRecorded verifies the trust boundary: a PAID
// activation on the same (email, device) never writes a trial_claims row —
// the detector watches trial claims only.
func TestTrialClaims_PaidKeyNotRecorded(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Seed the tenant for the claim email FIRST so the claim endpoint's
	// upsert reuses it (keeping its real api_key instead of the webhook
	// placeholder) — the paid activation then authenticates as the admin.
	// (Inline rather than seedTenant: the tenant id must be 15 chars while
	// the email must stay exactly "paidclaim@example.com".)
	tenantColl, err := app.FindCollectionByNameOrId("tenants")
	if err != nil {
		t.Fatalf("tenants collection not found: %v", err)
	}
	tenantRec := core.NewRecord(tenantColl)
	tenantRec.Set("id", "paidclaim000001")
	tenantRec.Set("email", "paidclaim@example.com")
	tenantRec.Set("phone", "-")
	hash, lookup, err := hashAPIKey("paidclaim-key")
	if err != nil {
		t.Fatalf("failed to hash api_key: %v", err)
	}
	tenantRec.Set("api_key", hash)
	tenantRec.Set("api_key_lookup", lookup)
	tenantRec.Set("status", "active")
	if err := app.Save(tenantRec); err != nil {
		t.Fatalf("failed to seed paid tenant: %v", err)
	}

	// Device already claimed a trial under this email.
	rec0 := servePost(t, se, trialPath, "", nil,
		`{"hardware_fingerprint":"`+trialFP1+`","platform":"windows","app_version":"0.0.28","email":"paidclaim@example.com"}`)
	if rec0.Code != http.StatusOK {
		t.Fatalf("expected 200 on trial claim, got %d: %s", rec0.Code, rec0.Body.String())
	}

	seedLicenseKey(t, app, "OZ-PAID-CLAIM-F1", "pro", "unused", "2099-12-31 23:59:59.000Z")
	rec := servePost(t, se, "/api/v1/license/activate", "Bearer paidclaim-key", nil,
		`{"key":"OZ-PAID-CLAIM-F1","email":"paidclaim@example.com","machine_id":"`+trialFP1+`","phone":"081234567896"}`)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 for paid key, got %d: %s", rec.Code, rec.Body.String())
	}
	if strings.Contains(rec.Body.String(), "repeat_claim") {
		t.Errorf("paid activation must never surface repeat_claim, got: %s", rec.Body.String())
	}
	rows, err := app.FindRecordsByFilter("trial_claims", "email = 'paidclaim@example.com'", "", 10, 0, nil)
	if err != nil || len(rows) != 1 {
		t.Fatalf("expected exactly ONE trial_claims row (the trial claim only), got %d (err %v)", len(rows), err)
	}
	if rows[0].GetInt("claim_count") != 1 {
		t.Errorf("paid activation must not bump claim_count, got %d", rows[0].GetInt("claim_count"))
	}
}

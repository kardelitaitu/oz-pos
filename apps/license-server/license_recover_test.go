package main

// Tests for LSE-11 phase A: recovery-code-gated api_key rotation.
//
// Covered here:
//   - the full happy cycle: activate → re-activate without api_key
//     (recovery_required, no rotation) → /recover (code emailed, stubbed)
//     → activate with the code (rotated api_key returned, owner notified)
//   - the 24h rotation cooldown still applying to code-backed rotations
//   - /recover refusing a key that isn't activated by the email's tenant
//   - activation refusing a wrong/expired recovery code (and not rotating)
//
// The email senders are package vars (sendLicenseRecoveryEmail,
// sendAPIKeyRotationNotice) and are stubbed per test with defer restore.

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// runRecoveryActivation posts one /activate request and decodes the JSON
// response. Returns the status code and parsed body.
func runRecoveryActivation(t *testing.T, mux http.Handler, key, email, machineID, recoveryCode string) (int, map[string]any) {
	t.Helper()
	body := fmt.Sprintf(`{
		"key": "%s",
		"email": "%s",
		"machine_id": "%s",
		"recovery_code": "%s"
	}`, key, email, machineID, recoveryCode)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	var resp map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse activation response (%d): %s", rec.Code, rec.Body.String())
	}
	return rec.Code, resp
}

func TestLicenseRecovery_FullCycle(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	const (
		email = "recovercycle001@example.com"
		key1  = "OZ-RECOVER-CYCLE1"
	)

	seedLicenseKey(t, app, key1, "pro", "unused", "2099-12-31 23:59:59.000Z")

	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}

	var sentRecoveryCodes []string
	var sentRotationNotices []string
	origRecover := sendLicenseRecoveryEmail
	origNotice := sendAPIKeyRotationNotice
	sendLicenseRecoveryEmail = func(to, code string) error {
		sentRecoveryCodes = append(sentRecoveryCodes, code)
		return nil
	}
	sendAPIKeyRotationNotice = func(to string) error {
		sentRotationNotices = append(sentRotationNotices, to)
		return nil
	}
	defer func() {
		sendLicenseRecoveryEmail = origRecover
		sendAPIKeyRotationNotice = origNotice
	}()

	// ── Step 1: first activation issues the original api_key ──
	code1, resp1 := runRecoveryActivation(t, mux, key1, email, "recovermac001", "")
	if code1 != http.StatusOK {
		t.Fatalf("first activation: expected 200, got %d: %v", code1, resp1)
	}
	origAPIKey, _ := resp1["api_key"].(string)
	if origAPIKey == "" {
		t.Fatal("first activation must issue an api_key")
	}

	// ── Step 2: re-activation without api_key → recovery_required ──
	resetRateLimiters()
	code2, resp2 := runRecoveryActivation(t, mux, key1, email, "recovermac002", "")
	if code2 != http.StatusOK {
		t.Fatalf("no-key re-activation: expected 200, got %d: %v", code2, resp2)
	}
	if _, ok := resp2["api_key"]; ok {
		t.Errorf("api_key must NOT be re-emitted without a recovery code: %v", resp2["api_key"])
	}
	if rot, ok := resp2["api_key_rotation"].(map[string]any); !ok || rot["status"] != "recovery_required" {
		t.Fatalf("expected api_key_rotation.status=recovery_required, got %v", resp2["api_key_rotation"])
	}
	if len(sentRecoveryCodes) != 0 {
		t.Errorf("no recovery email may be sent by /activate itself, got %d", len(sentRecoveryCodes))
	}

	// ── Step 3: /recover emails a code to the tenant ──────────
	resetRateLimiters()
	recBody := fmt.Sprintf(`{"email": "%s", "key": "%s"}`, email, key1)
	recReq := httptest.NewRequest("POST", "/api/v1/license/recover", strings.NewReader(recBody))
	recReq.Header.Set("Content-Type", "application/json")
	recRec := httptest.NewRecorder()
	mux.ServeHTTP(recRec, recReq)
	if recRec.Code != http.StatusOK {
		t.Fatalf("recover: expected 200, got %d: %s", recRec.Code, recRec.Body.String())
	}
	if len(sentRecoveryCodes) != 1 {
		t.Fatalf("expected exactly 1 recovery code emailed, got %d", len(sentRecoveryCodes))
	}
	recoveryCode := sentRecoveryCodes[0]
	if len(recoveryCode) != 6 {
		t.Errorf("expected a 6-digit recovery code, got %q", recoveryCode)
	}

	// ── Step 4: activate WITH the code → rotated api_key ──────
	// (No resetRateLimiters here: it would wipe webOtpStore.codes. The IP
	// budget has room — steps 1–3 used 3 of 5 tokens.)
	code4, resp4 := runRecoveryActivation(t, mux, key1, email, "recovermac002", recoveryCode)
	if code4 != http.StatusOK {
		t.Fatalf("code-backed re-activation: expected 200, got %d: %v", code4, resp4)
	}
	newAPIKey, ok := resp4["api_key"].(string)
	if !ok || newAPIKey == "" {
		t.Fatalf("expected a rotated api_key, got %v", resp4["api_key"])
	}
	if newAPIKey == origAPIKey {
		t.Error("rotated api_key must differ from the original")
	}
	if len(sentRotationNotices) != 1 || sentRotationNotices[0] != email {
		t.Errorf("expected exactly 1 rotation notice to %q, got %v", email, sentRotationNotices)
	}

	// The rotated key must actually authenticate /status.
	tenant, err := app.FindFirstRecordByData("tenants", "email", email)
	if err != nil || tenant == nil {
		t.Fatalf("tenant lookup failed: %v", err)
	}
	if !verifyAPIKey(tenant.GetString("api_key"), newAPIKey) {
		t.Error("stored api_key hash must verify against the rotated key")
	}
	if verifyAPIKey(tenant.GetString("api_key"), origAPIKey) {
		t.Error("the original api_key must no longer verify after rotation")
	}

	// ── Step 5: a SECOND code-backed rotation inside 24h → 429 ──
	// resetIPBudget clears only the per-IP bucket: the recovery code stored
	// below must survive, and the 24h rotation cooldown from step 4 must
	// still be active for the 429.
	resetIPBudget()
	recReq2 := httptest.NewRequest("POST", "/api/v1/license/recover", strings.NewReader(recBody))
	recReq2.Header.Set("Content-Type", "application/json")
	recRec2 := httptest.NewRecorder()
	mux.ServeHTTP(recRec2, recReq2)
	if recRec2.Code != http.StatusOK {
		t.Fatalf("second recover: expected 200, got %d: %s", recRec2.Code, recRec2.Body.String())
	}
	if len(sentRecoveryCodes) != 2 {
		t.Fatalf("expected 2 recovery codes total, got %d", len(sentRecoveryCodes))
	}
	code5, resp5 := runRecoveryActivation(t, mux, key1, email, "recovermac003", sentRecoveryCodes[1])
	if code5 != http.StatusTooManyRequests {
		t.Fatalf("second rotation inside 24h: expected 429, got %d: %v", code5, resp5)
	}
	if _, ok := resp5["retry_after"]; !ok {
		t.Error("expected retry_after in the cooldown response")
	}
	if _, ok := resp5["api_key"]; ok {
		t.Error("api_key must NOT be re-emitted while the rotation cooldown is active")
	}
}

func TestLicenseRecover_BadKey(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedLicenseKey(t, app, "OZ-RECOVER-KEY01", "pro", "unused", "2099-12-31 23:59:59.000Z")

	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}

	var sent []string
	orig := sendLicenseRecoveryEmail
	sendLicenseRecoveryEmail = func(to, code string) error {
		sent = append(sent, code)
		return nil
	}
	defer func() { sendLicenseRecoveryEmail = orig }()

	// Unknown key: generic 401, no email.
	body := `{"email": "recoverbad001@example.com", "key": "OZ-RECOVER-NOPE"}`
	req := httptest.NewRequest("POST", "/api/v1/license/recover", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("unknown key: expected 401, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "invalid or already used license key") {
		t.Errorf("expected the generic no-enumeration message, got: %s", rec.Body.String())
	}
	if len(sent) != 0 {
		t.Errorf("no recovery email may be sent for an unproven caller, got %d", len(sent))
	}
}

func TestActivate_RecoveryCodeWrong(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	const (
		email = "recoverwrong001@example.com"
		key1  = "OZ-RECOVER-WRONG1"
	)
	seedLicenseKey(t, app, key1, "pro", "unused", "2099-12-31 23:59:59.000Z")

	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}

	code1, resp1 := runRecoveryActivation(t, mux, key1, email, "recoverwrongmac1", "")
	if code1 != http.StatusOK {
		t.Fatalf("first activation: expected 200, got %d: %v", code1, resp1)
	}

	resetRateLimiters()
	// A wrong code must be refused AND must not rotate anything.
	code2, resp2 := runRecoveryActivation(t, mux, key1, email, "recoverwrongmac1", "000001")
	if code2 != http.StatusUnauthorized {
		t.Fatalf("wrong recovery code: expected 401, got %d: %v", code2, resp2)
	}
	if !strings.Contains(recBody(resp2), "invalid or expired recovery code") {
		t.Errorf("expected the invalid-code message, got: %s", recBody(resp2))
	}
	if _, ok := resp2["api_key"]; ok {
		t.Error("api_key must NOT be re-emitted for a wrong recovery code")
	}

	// And the tenant's stored api_key is untouched.
	tenant, err := app.FindFirstRecordByData("tenants", "email", email)
	if err != nil || tenant == nil {
		t.Fatalf("tenant lookup failed: %v", err)
	}
	origAPIKey, _ := resp1["api_key"].(string)
	if !verifyAPIKey(tenant.GetString("api_key"), origAPIKey) {
		t.Error("a failed recovery attempt must not invalidate the existing api_key")
	}
}

// recBody re-encodes a parsed response map for substring assertions.
func recBody(resp map[string]any) string {
	b, _ := json.Marshal(resp)
	return string(b)
}

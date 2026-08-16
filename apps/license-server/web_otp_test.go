package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// ── Test helpers ────────────────────────────────────────────────────

// stubOTPEmail captures the OTP code passed to sendOTPEmail so tests can
// drive the full request-otp → verify-otp flow without a real SMTP relay.
// Returns a restore func that puts the production sender back.
func stubOTPEmail(t *testing.T, captured *string) func() {
	t.Helper()
	// The handler gates on OZ_SMTP_HOST being configured before calling
	// sendOTPEmail; the stub replaces the sender, so point the gate at a
	// fake relay.
	t.Setenv("OZ_SMTP_HOST", "test.local")
	orig := sendOTPEmail
	sendOTPEmail = func(to, code string) error {
		*captured = code
		return nil
	}
	return func() { sendOTPEmail = orig }
}

// webRequest issues an HTTP request against the test router and returns
// the recorder. origin sets the CORS Origin header ("" = non-browser).
func webRequest(t *testing.T, se *core.ServeEvent, method, path, body, origin, auth string) *httptest.ResponseRecorder {
	t.Helper()
	var reader *bytes.Reader
	if body == "" {
		reader = bytes.NewReader(nil)
	} else {
		reader = bytes.NewReader([]byte(body))
	}
	req := httptest.NewRequest(method, path, reader)
	if origin != "" {
		req.Header.Set("Origin", origin)
	}
	if auth != "" {
		req.Header.Set("Authorization", auth)
	}
	if body != "" {
		req.Header.Set("Content-Type", "application/json")
	}
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)
	return rec
}

// ── request-otp ─────────────────────────────────────────────────────

func TestRequestOTP_SendsCodeToActiveTenant(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "webotptenant001", "webotpkey000001", "active")

	var sentCode string
	restore := stubOTPEmail(t, &sentCode)
	defer restore()

	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{"email":"WEBOTPTENANT001@example.com"}`, "http://localhost:4321", "")

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	if sentCode == "" || len(sentCode) != 6 {
		t.Fatalf("expected a 6-digit code to be sent, got %q", sentCode)
	}
	// The code must be stored (hashed) so verify-otp can consume it.
	webOtpStore.mu.Lock()
	_, stored := webOtpStore.codes["webotptenant001@example.com"]
	webOtpStore.mu.Unlock()
	if !stored {
		t.Error("expected a pending code to be stored for the tenant email")
	}
}

func TestRequestOTP_SelfSignupCreatesTenantAndSendsCode(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	var sentCode string
	restore := stubOTPEmail(t, &sentCode)
	defer restore()

	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{"email":"nobody@example.com"}`, "http://localhost:4321", "")

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	// Self-signup: an unknown email receives a code AND becomes an
	// active tenant (register-or-login — the response stays a plain 200,
	// so the endpoint never reveals whether the account pre-existed).
	if sentCode == "" || len(sentCode) != 6 {
		t.Fatalf("expected a 6-digit code to be sent for a new email, got %q", sentCode)
	}
	tenant, err := app.FindFirstRecordByData("tenants", "email", "nobody@example.com")
	if err != nil || tenant == nil {
		t.Fatalf("expected a tenant to be created by self-signup: %v", err)
	}
	if tenant.GetString("status") != "active" {
		t.Errorf("expected new tenant status active, got %q", tenant.GetString("status"))
	}
	if tenant.GetString("api_key") == "" || tenant.GetString("api_key_lookup") == "" {
		t.Error("expected placeholder api_key + lookup to be set on the new tenant")
	}
	// Self-signup does NOT verify the email — that happens at verify-otp.
	if tenant.GetBool("email_verified") {
		t.Error("expected a self-signed tenant to start email_verified=false")
	}
	webOtpStore.mu.Lock()
	_, stored := webOtpStore.codes["nobody@example.com"]
	webOtpStore.mu.Unlock()
	if !stored {
		t.Error("expected a pending code to be stored for the new email")
	}
}

// TestVerifyOTP_AfterSelfSignupIssuesSession drives the full register
// flow: request-otp creates the tenant, verify-otp with the emailed code
// issues a session token and returns the tenant summary.
func TestVerifyOTP_AfterSelfSignupIssuesSession(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	var sentCode string
	restore := stubOTPEmail(t, &sentCode)
	defer restore()

	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{"email":"newuser@example.com"}`, "http://localhost:4321", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("request-otp expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	if sentCode == "" {
		t.Fatal("expected a code to be sent")
	}

	rec = webRequest(t, se, http.MethodPost, "/api/v1/web/verify-otp",
		`{"email":"newuser@example.com","code":"`+sentCode+`"}`, "http://localhost:4321", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("verify-otp expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var resp struct {
		Token  string         `json:"token"`
		Tenant map[string]any `json:"tenant"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("bad response JSON: %v", err)
	}
	if resp.Token == "" {
		t.Error("expected a session token")
	}
	if resp.Tenant == nil {
		t.Error("expected a tenant summary in the verify response")
	}
	if resp.Tenant["email"] != "newuser@example.com" {
		t.Errorf("expected tenant email newuser@example.com, got %v", resp.Tenant["email"])
	}
	// Completing OTP verification must flip email_verified to true, both
	// in the response and on the persisted record (the dashboard reads
	// this from /me).
	if resp.Tenant["emailVerified"] != true {
		t.Errorf("expected emailVerified=true in the verify response, got %v", resp.Tenant["emailVerified"])
	}
	tenant, err := app.FindFirstRecordByData("tenants", "email", "newuser@example.com")
	if err != nil || tenant == nil {
		t.Fatalf("tenant should exist after verify: %v", err)
	}
	if !tenant.GetBool("email_verified") {
		t.Error("expected the persisted tenant record to be email_verified=true after verify-otp")
	}
}

func TestRequestOTP_NoEnumerationForSuspendedTenant(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "webotpsusp00001", "webotpsusp0001", "suspended")

	var sentCode string
	restore := stubOTPEmail(t, &sentCode)
	defer restore()

	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{"email":"webotpsusp00001@example.com"}`, "http://localhost:4321", "")

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 (no enumeration), got %d: %s", rec.Code, rec.Body.String())
	}
	if sentCode != "" {
		t.Error("no code should be emailed for a suspended tenant")
	}
}

func TestRequestOTP_RateLimitedPerEmail(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	var sentCode string
	restore := stubOTPEmail(t, &sentCode)
	defer restore()

	// 3 allowed per email per 15 min.
	for i := 0; i < 3; i++ {
		rec := webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
			`{"email":"ratelimited@example.com"}`, "http://localhost:4321", "")
		if rec.Code != http.StatusOK {
			t.Fatalf("call %d should succeed, got %d: %s", i+1, rec.Code, rec.Body.String())
		}
	}
	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{"email":"ratelimited@example.com"}`, "http://localhost:4321", "")
	if rec.Code != http.StatusTooManyRequests {
		t.Fatalf("4th request should be 429, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestRequestOTP_CORSRejectsForeignOrigin(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{"email":"cors@example.com"}`, "https://evil.example.com", "")
	if rec.Code != http.StatusForbidden {
		t.Fatalf("expected 403 for disallowed origin, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestRequestOTP_MissingSMTPReturns503(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "webotpnosmtp001", "webotpnosmtp001", "active")
	t.Setenv("OZ_SMTP_HOST", "") // ensure unset

	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{"email":"webotpnosmtp001@example.com"}`, "http://localhost:4321", "")
	if rec.Code != http.StatusServiceUnavailable {
		t.Fatalf("expected 503 when SMTP unconfigured, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestRequestOTP_InvalidBodyAndEmail(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Invalid JSON → 400.
	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`not json`, "http://localhost:4321", "")
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("expected 400 for invalid JSON, got %d: %s", rec.Code, rec.Body.String())
	}

	// Missing email → 400.
	rec = webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{}`, "http://localhost:4321", "")
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("expected 400 for missing email, got %d: %s", rec.Code, rec.Body.String())
	}

	// Malformed email → 400.
	rec = webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{"email":"not-an-email"}`, "http://localhost:4321", "")
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("expected 400 for malformed email, got %d: %s", rec.Code, rec.Body.String())
	}
}

// ── verify-otp ──────────────────────────────────────────────────────

func TestVerifyOTP_HappyPathIssuesSession(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	const tenantID = "webotpverify001"
	seedTenant(t, app, tenantID, "webotpverify001", "active")
	seedSubscription(t, app, tenantID, "pro", "active")

	var sentCode string
	restore := stubOTPEmail(t, &sentCode)
	defer restore()

	// Request a code.
	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{"email":"webotpverify001@example.com"}`, "http://localhost:4321", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("request-otp failed: %d %s", rec.Code, rec.Body.String())
	}
	if sentCode == "" {
		t.Fatal("no code captured")
	}

	// Verify it.
	body := `{"email":"webotpverify001@example.com","code":"` + sentCode + `"}`
	rec = webRequest(t, se, http.MethodPost, "/api/v1/web/verify-otp",
		body, "http://localhost:4321", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("verify-otp failed: %d %s", rec.Code, rec.Body.String())
	}

	var resp struct {
		Token   string         `json:"token"`
		Expires string         `json:"expires_at"`
		Tenant  map[string]any `json:"tenant"`
		License any            `json:"license"`
		Sub     any            `json:"subscription"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}
	if resp.Token == "" {
		t.Error("expected a session token")
	}
	if resp.Expires == "" {
		t.Error("expected expires_at")
	}
	if resp.Tenant["email"] != "webotpverify001@example.com" {
		t.Errorf("unexpected tenant email: %v", resp.Tenant["email"])
	}
	if resp.Sub == nil {
		t.Error("expected a subscription summary (tenant has an active subscription)")
	}
	if resp.License != nil {
		t.Error("expected license to be null (tenant has no activated license key)")
	}

	// The code must be single-use.
	rec = webRequest(t, se, http.MethodPost, "/api/v1/web/verify-otp",
		body, "http://localhost:4321", "")
	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("reusing a code should 401, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestVerifyOTP_WrongCodeGeneric401(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "webotpwrong0001", "webotpwrong0001", "active")

	var sentCode string
	restore := stubOTPEmail(t, &sentCode)
	defer restore()

	webRequest(t, se, http.MethodPost, "/api/v1/web/request-otp",
		`{"email":"webotpwrong0001@example.com"}`, "http://localhost:4321", "")
	if sentCode == "" {
		t.Fatal("no code captured")
	}

	// Wrong code (and also an unknown email) must both be the same 401.
	for _, body := range []string{
		`{"email":"webotpwrong0001@example.com","code":"000000"}`,
		`{"email":"unknown@example.com","code":"` + sentCode + `"}`,
	} {
		rec := webRequest(t, se, http.MethodPost, "/api/v1/web/verify-otp",
			body, "http://localhost:4321", "")
		if rec.Code != http.StatusUnauthorized {
			t.Fatalf("expected generic 401 for body %s, got %d: %s", body, rec.Code, rec.Body.String())
		}
	}
}

func TestVerifyOTP_RateLimited(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// 5 attempts per email per 15 min (no code needed — limiter fires first
	// on the 6th regardless of validity).
	for i := 0; i < 5; i++ {
		rec := webRequest(t, se, http.MethodPost, "/api/v1/web/verify-otp",
			`{"email":"verifyspam@example.com","code":"000000"}`,
			"http://localhost:4321", "")
		if rec.Code == http.StatusTooManyRequests {
			t.Fatalf("call %d should not be rate limited yet", i+1)
		}
	}
	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/verify-otp",
		`{"email":"verifyspam@example.com","code":"000000"}`,
		"http://localhost:4321", "")
	if rec.Code != http.StatusTooManyRequests {
		t.Fatalf("6th attempt should be 429, got %d: %s", rec.Code, rec.Body.String())
	}
}

// ── /me ─────────────────────────────────────────────────────────────

func TestMe_ReturnsTenantProfile(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	const tenantID = "webotpme0000001"
	seedTenant(t, app, tenantID, "webotpme000001", "active")
	seedSubscription(t, app, tenantID, "premium", "active")
	seedLicenseKey(t, app, "OZ-ME-KEY-000001", "premium", "activated", "2099-12-31 23:59:59.000Z")
	// Bind the license key to the tenant via the activated_by relation.
	keys, err := app.FindRecordsByFilter("license_keys", "key = 'OZ-ME-KEY-000001'", "", 1, 0, nil)
	if err != nil || len(keys) == 0 {
		t.Fatalf("seeded key not found: %v", err)
	}
	keys[0].Set("activated_by", []string{tenantID})
	if err := app.Save(keys[0]); err != nil {
		t.Fatalf("failed to bind key: %v", err)
	}

	// Mint a session directly (bypasses the OTP flow for this test).
	token := "me-session-token-0001"
	webOtpStore.createSession(hashWebToken(token), tenantID)

	rec := webRequest(t, se, http.MethodGet, "/api/v1/web/me", "",
		"http://localhost:4321", "Bearer "+token)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var resp struct {
		Tenant  map[string]any `json:"tenant"`
		License map[string]any `json:"license"`
		Sub     map[string]any `json:"subscription"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}
	if resp.Tenant["email"] != "webotpme0000001@example.com" {
		t.Errorf("unexpected tenant email: %v", resp.Tenant["email"])
	}
	if resp.Tenant["status"] != "active" {
		t.Errorf("unexpected tenant status: %v", resp.Tenant["status"])
	}
	if resp.Tenant["emailVerified"] != false {
		t.Errorf("expected emailVerified=false for a seeded (never-verified) tenant, got %v", resp.Tenant["emailVerified"])
	}
	if resp.License["key"] != "OZ-ME-KEY-000001" {
		t.Errorf("unexpected license key: %v", resp.License["key"])
	}
	if resp.License["tierKey"] != "premium" {
		t.Errorf("unexpected license tier: %v", resp.License["tierKey"])
	}
	if resp.Sub["tierKey"] != "premium" {
		t.Errorf("unexpected subscription tier: %v", resp.Sub["tierKey"])
	}
	if _, ok := resp.License["expiresAt"]; !ok {
		t.Error("expected license expiresAt in response")
	}
}

// TestMe_ShowsUnusedKeyViaSubscription covers the register-first purchase
// state: the webhook minted the license key (status "unused", linked by
// paddle_sub_id) but the POS hasn't activated it yet — /me must still
// surface the key the tenant paid for.
func TestMe_ShowsUnusedKeyViaSubscription(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	const tenantID = "webotpmeunused1"
	seedTenant(t, app, tenantID, "webotpmeunused1", "active")
	seedSubscription(t, app, tenantID, "pro", "active")
	// Link the subscription to a paddle sub id, and the key to the same id
	// (exactly what the webhook does on subscription.created).
	subs, err := app.FindRecordsByFilter("subscriptions", "tenant_id = {:t}", "", 1, 0, map[string]any{"t": tenantID})
	if err != nil || len(subs) == 0 {
		t.Fatalf("seeded subscription not found: %v", err)
	}
	subs[0].Set("paddle_sub_id", "sub_01m05unused")
	if err := app.Save(subs[0]); err != nil {
		t.Fatalf("failed to set paddle_sub_id: %v", err)
	}
	seedLicenseKey(t, app, "OZ-UNUSED-KEY-0001", "pro", "unused", "2099-12-31 23:59:59.000Z")
	keys, err := app.FindRecordsByFilter("license_keys", "key = 'OZ-UNUSED-KEY-0001'", "", 1, 0, nil)
	if err != nil || len(keys) == 0 {
		t.Fatalf("seeded key not found: %v", err)
	}
	keys[0].Set("paddle_sub_id", "sub_01m05unused")
	if err := app.Save(keys[0]); err != nil {
		t.Fatalf("failed to set key paddle_sub_id: %v", err)
	}

	token := "me-session-unused-0001"
	webOtpStore.createSession(hashWebToken(token), tenantID)

	rec := webRequest(t, se, http.MethodGet, "/api/v1/web/me", "",
		"http://localhost:4321", "Bearer "+token)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var resp struct {
		License map[string]any `json:"license"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}
	if resp.License == nil {
		t.Fatal("expected the unused license key to be surfaced via the subscription link")
	}
	if resp.License["key"] != "OZ-UNUSED-KEY-0001" {
		t.Errorf("unexpected license key: %v", resp.License["key"])
	}
	if resp.License["tierKey"] != "pro" {
		t.Errorf("unexpected license tier: %v", resp.License["tierKey"])
	}
	if resp.License["status"] != "unused" {
		t.Errorf("expected status unused, got %v", resp.License["status"])
	}
}

func TestMe_UnauthorizedWithoutToken(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	for _, auth := range []string{"", "Bearer ", "Bearer notarealtoken"} {
		rec := webRequest(t, se, http.MethodGet, "/api/v1/web/me", "",
			"http://localhost:4321", auth)
		if rec.Code != http.StatusUnauthorized {
			t.Fatalf("expected 401 for auth %q, got %d: %s", auth, rec.Code, rec.Body.String())
		}
	}
}

func TestMe_ExpiredSessionReturns401(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "webotpmeexp0001", "webotpmeexp0001", "active")

	token := "expired-session-tok"
	// Insert a session that is already past its TTL.
	webOtpStore.mu.Lock()
	webOtpStore.sessions[hashWebToken(token)] = &webSession{
		tenantID:  "webotpmeexp0001",
		expiresAt: time.Now().Add(-time.Minute),
	}
	webOtpStore.mu.Unlock()

	rec := webRequest(t, se, http.MethodGet, "/api/v1/web/me", "",
		"http://localhost:4321", "Bearer "+token)
	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401 for expired session, got %d: %s", rec.Code, rec.Body.String())
	}
}

// ── logout ──────────────────────────────────────────────────────────

func TestLogout_InvalidatesSession(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "webotplogout001", "webotplogout001", "active")

	token := "logout-session-tok"
	webOtpStore.createSession(hashWebToken(token), "webotplogout001")

	// /me works before logout.
	rec := webRequest(t, se, http.MethodGet, "/api/v1/web/me", "",
		"http://localhost:4321", "Bearer "+token)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected /me 200 before logout, got %d: %s", rec.Code, rec.Body.String())
	}

	// Logout (idempotent — repeat call also 200).
	for i := 0; i < 2; i++ {
		rec = webRequest(t, se, http.MethodPost, "/api/v1/web/logout", "",
			"http://localhost:4321", "Bearer "+token)
		if rec.Code != http.StatusOK {
			t.Fatalf("logout call %d should be 200, got %d: %s", i+1, rec.Code, rec.Body.String())
		}
	}

	// /me now 401 — session gone.
	rec = webRequest(t, se, http.MethodGet, "/api/v1/web/me", "",
		"http://localhost:4321", "Bearer "+token)
	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected /me 401 after logout, got %d: %s", rec.Code, rec.Body.String())
	}
}

// ── Helper unit tests ───────────────────────────────────────────────

func TestGenerateOtpCode_IsSixDigits(t *testing.T) {
	for i := 0; i < 200; i++ {
		code, err := generateOtpCode()
		if err != nil {
			t.Fatalf("generateOtpCode failed: %v", err)
		}
		if len(code) != 6 {
			t.Fatalf("expected 6-digit code, got %q", code)
		}
		for _, c := range code {
			if c < '0' || c > '9' {
				t.Fatalf("non-digit char in code %q", code)
			}
		}
	}
}

func TestIs6DigitCode(t *testing.T) {
	for _, tc := range []struct {
		in   string
		want bool
	}{
		{"123456", true},
		{"000000", true},
		{"12345", false},
		{"1234567", false},
		{"12a456", false},
		{"", false},
		{" 123456", false},
	} {
		if got := is6DigitCode(tc.in); got != tc.want {
			t.Errorf("is6DigitCode(%q) = %v, want %v", tc.in, got, tc.want)
		}
	}
}

func TestNormalizeEmail(t *testing.T) {
	if got := normalizeEmail("  Foo@Example.COM "); got != "foo@example.com" {
		t.Errorf("normalizeEmail = %q, want foo@example.com", got)
	}
}

func TestConstantTimeHashEq(t *testing.T) {
	a := hashOtpCode("123456")
	b := hashOtpCode("123456")
	c := hashOtpCode("654321")
	if !constantTimeHashEq(a, b) {
		t.Error("identical hashes should match")
	}
	if constantTimeHashEq(a, c) {
		t.Error("different hashes must not match")
	}
	if constantTimeHashEq(a, "") {
		t.Error("empty hash must not match")
	}
}

func TestWebSessionTTL_Default(t *testing.T) {
	if got := webSessionTTL(); got != defaultWebSessionTTL {
		t.Errorf("default TTL = %v, want %v", got, defaultWebSessionTTL)
	}
}

func TestWebSessionTTL_EnvOverride(t *testing.T) {
	t.Setenv("OZ_WEB_SESSION_TTL", "1h")
	if got := webSessionTTL(); got != time.Hour {
		t.Errorf("env TTL = %v, want 1h", got)
	}
}

func TestWebSessionTTL_InvalidEnvFallsBack(t *testing.T) {
	t.Setenv("OZ_WEB_SESSION_TTL", "not-a-duration")
	if got := webSessionTTL(); got != defaultWebSessionTTL {
		t.Errorf("invalid env TTL = %v, want default %v", got, defaultWebSessionTTL)
	}
}

func TestWebAllowedOrigins_Default(t *testing.T) {
	origins := webAllowedOrigins()
	if len(origins) != 3 {
		t.Fatalf("expected 3 default origins, got %d", len(origins))
	}
	if !strings.Contains(strings.Join(origins, ","), "oz-pos.adikaradwiatmaja.workers.dev") {
		t.Errorf("expected workers.dev origin in defaults, got %v", origins)
	}
}

func TestWebAllowedOrigins_EnvOverride(t *testing.T) {
	t.Setenv("OZ_WEB_ALLOWED_ORIGINS", "https://a.com, https://b.com")
	origins := webAllowedOrigins()
	if len(origins) != 2 || origins[0] != "https://a.com" || origins[1] != "https://b.com" {
		t.Errorf("unexpected origins from env: %v", origins)
	}
}

func TestOTPStore_SweepRemovesExpired(t *testing.T) {
	store := &otpStore{
		codes:    make(map[string]*otpCode),
		sessions: make(map[string]*webSession),
	}
	store.codes["expired@example.com"] = &otpCode{hash: "x", expiresAt: time.Now().Add(-time.Minute)}
	store.codes["fresh@example.com"] = &otpCode{hash: "y", expiresAt: time.Now().Add(time.Hour)}
	store.sessions["expiredhash"] = &webSession{tenantID: "t", expiresAt: time.Now().Add(-time.Minute)}
	store.sessions["freshhash"] = &webSession{tenantID: "t", expiresAt: time.Now().Add(time.Hour)}

	store.sweep()

	if _, ok := store.codes["expired@example.com"]; ok {
		t.Error("expired code should be swept")
	}
	if _, ok := store.codes["fresh@example.com"]; !ok {
		t.Error("fresh code should remain")
	}
	if _, ok := store.sessions["expiredhash"]; ok {
		t.Error("expired session should be swept")
	}
	if _, ok := store.sessions["freshhash"]; !ok {
		t.Error("fresh session should remain")
	}
}

func TestWindowLimiter_AllowsThenBlocks(t *testing.T) {
	wl := &windowLimiter{entries: make(map[string]*windowEntry), limit: 2, window: time.Minute}
	if !wl.allow("k") || !wl.allow("k") {
		t.Error("first two should be allowed")
	}
	if wl.allow("k") {
		t.Error("third should be blocked")
	}
	if !wl.allow("other") {
		t.Error("different key should not be blocked")
	}
}

func TestWindowLimiter_Sweep(t *testing.T) {
	wl := &windowLimiter{entries: make(map[string]*windowEntry), limit: 1, window: time.Minute}
	wl.allow("k")
	wl.mu.Lock()
	wl.entries["k"].start = time.Now().Add(-2 * time.Minute)
	wl.mu.Unlock()
	wl.sweep()
	if _, ok := wl.entries["k"]; ok {
		t.Error("expired window should be swept")
	}
}

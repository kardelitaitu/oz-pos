package main

import (
	"encoding/json"
	"net/http"
	"strings"
	"testing"

	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
	"golang.org/x/crypto/bcrypt"
)

// ── Test helpers ────────────────────────────────────────────────────

// seedTenantWithPassword seeds an active tenant AND sets its web password
// hash, mirroring the state after an account holder used set-password.
func seedTenantWithPassword(t *testing.T, app *tests.TestApp, tenantID, apiKey, password string) {
	t.Helper()
	seedTenant(t, app, tenantID, apiKey, "active")
	tenant, err := app.FindFirstRecordByData("tenants", "email", strings.ToLower(tenantID+"@example.com"))
	if err != nil || tenant == nil {
		t.Fatalf("seeded tenant not found: %v", err)
	}
	hash, err := hashPassword(password)
	if err != nil {
		t.Fatalf("hashPassword failed: %v", err)
	}
	tenant.Set("password_hash", hash)
	if err := app.Save(tenant); err != nil {
		t.Fatalf("failed to set password_hash: %v", err)
	}
}

// ── POST /api/v1/web/login ──────────────────────────────────────────

func TestLoginPassword_HappyPathIssuesSession(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenantWithPassword(t, app, "pwlogin00000001", "pwloginapikey01", "correct-horse-1")

	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/login",
		`{"email":"pwlogin00000001@example.com","password":"correct-horse-1"}`,
		"http://localhost:4321", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var resp struct {
		Token   string         `json:"token"`
		Expires string         `json:"expires_at"`
		Tenant  map[string]any `json:"tenant"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("bad response JSON: %v", err)
	}
	if resp.Token == "" {
		t.Error("expected a session token")
	}
	if resp.Expires == "" {
		t.Error("expected expires_at")
	}
	if resp.Tenant["email"] != "pwlogin00000001@example.com" {
		t.Errorf("unexpected tenant email: %v", resp.Tenant["email"])
	}

	// The issued token must work on /me exactly like an OTP session.
	me := webRequest(t, se, http.MethodGet, "/api/v1/web/me", "",
		"http://localhost:4321", "Bearer "+resp.Token)
	if me.Code != http.StatusOK {
		t.Fatalf("expected /me 200 with the login token, got %d: %s", me.Code, me.Body.String())
	}
}

// TestLoginPassword_Generic401ForEveryFailure pins the no-enumeration
// contract: unknown email, no password set, wrong password, and
// non-active tenant all return the identical 401 body — the endpoint can
// never be used to probe account state.
func TestLoginPassword_Generic401ForEveryFailure(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Active tenant WITH a password.
	seedTenantWithPassword(t, app, "pwloginactive01", "pwloginactive01", "right-password-1")
	// Active tenant WITHOUT a password (OTP-only account).
	seedTenant(t, app, "pwloginotponly1", "pwloginotponly1", "active")
	// Suspended tenant WITH a password.
	seedTenant(t, app, "pwloginsusp0001", "pwloginsusp0001", "suspended")
	susp, err := app.FindFirstRecordByData("tenants", "email", "pwloginsusp0001@example.com")
	if err != nil || susp == nil {
		t.Fatalf("suspended tenant not found: %v", err)
	}
	hash, _ := hashPassword("right-password-1")
	susp.Set("password_hash", hash)
	if err := app.Save(susp); err != nil {
		t.Fatalf("failed to set suspended tenant password: %v", err)
	}

	cases := []struct {
		name string
		body string
	}{
		{"wrong password", `{"email":"pwloginactive01@example.com","password":"wrong-password"}`},
		{"unknown email", `{"email":"nobody@example.com","password":"right-password-1"}`},
		{"otp-only account (no password set)", `{"email":"pwloginotponly1@example.com","password":"right-password-1"}`},
		{"suspended tenant", `{"email":"pwloginsusp0001@example.com","password":"right-password-1"}`},
		{"empty password", `{"email":"pwloginactive01@example.com","password":""}`},
	}
	for _, tc := range cases {
		rec := webRequest(t, se, http.MethodPost, "/api/v1/web/login",
			tc.body, "http://localhost:4321", "")
		if rec.Code != http.StatusUnauthorized {
			t.Errorf("%s: expected 401, got %d: %s", tc.name, rec.Code, rec.Body.String())
			continue
		}
		if !strings.Contains(rec.Body.String(), "invalid email or password") {
			t.Errorf("%s: expected generic 'invalid email or password', got %s", tc.name, rec.Body.String())
		}
	}
}

func TestLoginPassword_RateLimitedPerEmail(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// 5 attempts allowed per email per 15 min (wrong password → 401, but
	// the limiter counts every attempt).
	for i := 0; i < 5; i++ {
		rec := webRequest(t, se, http.MethodPost, "/api/v1/web/login",
			`{"email":"pwspam000000001@example.com","password":"wrong-password"}`,
			"http://localhost:4321", "")
		if rec.Code == http.StatusTooManyRequests {
			t.Fatalf("call %d should not be rate limited yet", i+1)
		}
	}
	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/login",
		`{"email":"pwspam000000001@example.com","password":"wrong-password"}`,
		"http://localhost:4321", "")
	if rec.Code != http.StatusTooManyRequests {
		t.Fatalf("6th attempt should be 429, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestLoginPassword_CORSRejectsForeignOrigin(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/login",
		`{"email":"pwlogincors0001@example.com","password":"whatever-123"}`,
		"https://evil.example.com", "")
	if rec.Code != http.StatusForbidden {
		t.Fatalf("expected 403 for disallowed origin, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestLoginPassword_InvalidBodyAndEmail(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	for _, body := range []string{
		`not json`,
		`{}`,
		`{"email":"not-an-email","password":"whatever-123"}`,
	} {
		rec := webRequest(t, se, http.MethodPost, "/api/v1/web/login",
			body, "http://localhost:4321", "")
		if rec.Code != http.StatusBadRequest {
			t.Errorf("body %q: expected 400, got %d: %s", body, rec.Code, rec.Body.String())
		}
	}
}

// ── POST /api/v1/web/set-password ───────────────────────────────────

func TestSetPassword_RequiresSession(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	for _, auth := range []string{"", "Bearer ", "Bearer notarealtoken"} {
		rec := webRequest(t, se, http.MethodPost, "/api/v1/web/set-password",
			`{"password":"brand-new-password"}`, "http://localhost:4321", auth)
		if rec.Code != http.StatusUnauthorized {
			t.Fatalf("expected 401 for auth %q, got %d: %s", auth, rec.Code, rec.Body.String())
		}
	}
}

func TestSetPassword_WeakPasswordRejected(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "pwsetweak000001", "pwsetweak000001", "active")
	token := "pwset-weak-token-01"
	webOtpStore.createSession(hashWebToken(token), "pwsetweak000001")

	for _, pw := range []string{
		"",                       // empty
		"short",                  // < 8 chars
		"  padded-password-123 ", // edge whitespace
		strings.Repeat("x", 73),  // > 72 bytes (bcrypt truncation)
	} {
		body := `{"password":"` + pw + `"}`
		rec := webRequest(t, se, http.MethodPost, "/api/v1/web/set-password",
			body, "http://localhost:4321", "Bearer "+token)
		if rec.Code != http.StatusBadRequest {
			t.Errorf("password %q: expected 400, got %d: %s", pw, rec.Code, rec.Body.String())
		}
	}
}

// TestSetPassword_SetsAndRotates drives the full lifecycle: an OTP-only
// account sets its first password, logs in with it, rotates it, and the
// old password stops working while the new one authenticates.
func TestSetPassword_SetsAndRotates(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "pwsetlife000001", "pwsetlife000001", "active")
	token := "pwset-life-token-01"
	webOtpStore.createSession(hashWebToken(token), "pwsetlife000001")

	// First-time set.
	rec := webRequest(t, se, http.MethodPost, "/api/v1/web/set-password",
		`{"password":"first-password-123"}`, "http://localhost:4321", "Bearer "+token)
	if rec.Code != http.StatusOK {
		t.Fatalf("set-password expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	// At-rest form must be a bcrypt hash, never the plaintext.
	tenant, err := app.FindFirstRecordByData("tenants", "email", "pwsetlife000001@example.com")
	if err != nil || tenant == nil {
		t.Fatalf("tenant not found: %v", err)
	}
	stored := tenant.GetString("password_hash")
	if stored == "" {
		t.Fatal("expected password_hash to be persisted")
	}
	if stored == "first-password-123" {
		t.Fatal("password must never be stored in plaintext")
	}
	if bcrypt.CompareHashAndPassword([]byte(stored), []byte("first-password-123")) != nil {
		t.Error("stored hash must verify against the first password")
	}

	// Login with the first password works.
	rec = webRequest(t, se, http.MethodPost, "/api/v1/web/login",
		`{"email":"pwsetlife000001@example.com","password":"first-password-123"}`,
		"http://localhost:4321", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("login with the first password should succeed, got %d: %s", rec.Code, rec.Body.String())
	}

	// Rotate.
	rec = webRequest(t, se, http.MethodPost, "/api/v1/web/set-password",
		`{"password":"second-password-456"}`, "http://localhost:4321", "Bearer "+token)
	if rec.Code != http.StatusOK {
		t.Fatalf("rotate expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	// Old password now fails, new one succeeds.
	rec = webRequest(t, se, http.MethodPost, "/api/v1/web/login",
		`{"email":"pwsetlife000001@example.com","password":"first-password-123"}`,
		"http://localhost:4321", "")
	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("old password should 401 after rotation, got %d: %s", rec.Code, rec.Body.String())
	}
	rec = webRequest(t, se, http.MethodPost, "/api/v1/web/login",
		`{"email":"pwsetlife000001@example.com","password":"second-password-456"}`,
		"http://localhost:4321", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("new password should succeed after rotation, got %d: %s", rec.Code, rec.Body.String())
	}
}

// ── Helper unit tests ───────────────────────────────────────────────

func TestIsValidPassword(t *testing.T) {
	for _, tc := range []struct {
		in   string
		want bool
	}{
		{"password123", true},
		{"8chars!!", true},
		{strings.Repeat("x", 72), true}, // at the bcrypt byte ceiling
		{"", false},
		{"short", false},
		{strings.Repeat("x", 73), false}, // beyond bcrypt's 72-byte input
		{"  password123", false},         // leading whitespace
		{"password123  ", false},         // trailing whitespace
		{"😀😀😀😀", false},                  // 4 runes but 16 bytes — rune floor applies
		{"😀😀😀😀😀😀😀😀", true},               // 8 runes, 32 bytes — valid
	} {
		if got := isValidPassword(tc.in); got != tc.want {
			t.Errorf("isValidPassword(%q) = %v, want %v", tc.in, got, tc.want)
		}
	}
}

// TestEnsurePasswordHashField_MigratesExistingCollection simulates a
// deployment that predates the field: the tenants collection exists
// WITHOUT password_hash, and the migration must add it (idempotently)
// as a hidden text field without touching other fields.
func TestEnsurePasswordHashField_MigratesExistingCollection(t *testing.T) {
	app, err := tests.NewTestApp()
	if err != nil {
		t.Fatalf("failed to create test app: %v", err)
	}
	defer app.Cleanup()

	tenants := core.NewBaseCollection("tenants")
	tenants.Fields.Add(
		&core.EmailField{Name: "email", Required: true},
		&core.SelectField{Name: "status", Required: true, Values: []string{"active", "suspended", "revoked"}},
	)
	if err := app.Save(tenants); err != nil {
		t.Fatalf("failed to create tenants collection: %v", err)
	}
	if tenants.Fields.GetByName("password_hash") != nil {
		t.Fatal("precondition failed: tenants should not have password_hash yet")
	}

	if err := ensurePasswordHashField(app); err != nil {
		t.Fatalf("ensurePasswordHashField failed: %v", err)
	}
	after, err := app.FindCollectionByNameOrId("tenants")
	if err != nil {
		t.Fatalf("find tenants after migration: %v", err)
	}
	field := after.Fields.GetByName("password_hash")
	if field == nil {
		t.Fatal("expected password_hash field to be added by the migration")
	}
	if field.Type() != core.FieldTypeText {
		t.Errorf("expected a text field, got %q", field.Type())
	}

	// Idempotent: a second run must be a no-op.
	if err := ensurePasswordHashField(app); err != nil {
		t.Fatalf("second ensurePasswordHashField should be a no-op: %v", err)
	}
}

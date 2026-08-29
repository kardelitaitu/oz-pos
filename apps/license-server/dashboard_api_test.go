package main

// Tests for the user + admin dashboard API endpoints (ADR #42 Phases 2–3):
// /api/v1/web/usage, /api/v1/web/devices, and the /api/v1/admin/tenants*
// management endpoints. Each test seeds a tenant + subscription + machine
// and asserts the JSON response.

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
)

// dashboardMux builds the test app (collections + dashboard routes) and
// returns the built mux for direct request serving.
func dashboardMux(t *testing.T) (*tests.TestApp, http.Handler) {
	t.Helper()
	resetRateLimiters()
	app, se := setupDirectApp(t)
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	return app, mux
}

// seedDashboardTenant creates a tenant + active subscription + machine and
// issues a session token for the tenant.
func seedDashboardTenant(t *testing.T, app *tests.TestApp, email string) (tenantID string, token string) {
	t.Helper()

	tenant, err := app.FindFirstRecordByData("tenants", "email", email)
	if err != nil || tenant == nil {
		col, _ := app.FindCollectionByNameOrId("tenants")
		tenant = core.NewRecord(col)
		tenant.Set("email", email)
		tenant.Set("api_key", "key-"+email)
		tenant.Set("api_key_lookup", apiKeyLookup("key-"+email))
		tenant.Set("status", "active")
		if err := app.Save(tenant); err != nil {
			t.Fatalf("save tenant: %v", err)
		}
	}
	tenantID = tenant.Id

	// Active subscription.
	subCol, _ := app.FindCollectionByNameOrId("subscriptions")
	sub := core.NewRecord(subCol)
	sub.Set("tenant_id", tenantID)
	sub.Set("tier_key", "pro")
	sub.Set("status", "active")
	sub.Set("max_stores", 3)
	sub.Set("max_pos_instances", 5)
	sub.Set("starts_at", "2026-01-01T00:00:00Z")
	sub.Set("expires_at", "2027-01-01T00:00:00Z")
	sub.Set("signature", "test")
	sub.Set("signed_payload", "{}")
	if err := app.Save(sub); err != nil {
		t.Fatalf("save subscription: %v", err)
	}

	// Machine/device.
	machCol, _ := app.FindCollectionByNameOrId("tenant_machines")
	m := core.NewRecord(machCol)
	m.Set("tenant_id", tenantID)
	m.Set("machine_id", "mach-1")
	if err := app.Save(m); err != nil {
		t.Fatalf("save machine: %v", err)
	}

	// Session token.
	token = "test-session-token"
	webOtpStore.createSession(hashWebToken(token), tenantID)
	return tenantID, token
}

// doJSON performs a request against the mux and returns the recorder.
func doJSON(mux http.Handler, method, path, auth, body string) *httptest.ResponseRecorder {
	var reader *strings.Reader
	if body == "" {
		reader = strings.NewReader("")
	} else {
		reader = strings.NewReader(body)
	}
	req := httptest.NewRequest(method, path, reader)
	if auth != "" {
		req.Header.Set("Authorization", auth)
	}
	if body != "" {
		req.Header.Set("Content-Type", "application/json")
	}
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	return rec
}

// ── GET /api/v1/web/usage ─────────────────────────────────────────

func TestWebUsage_ReturnsUsageStats(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	_, token := seedDashboardTenant(t, app, "usage@test.com")

	rec := doJSON(mux, http.MethodGet, "/api/v1/web/usage", "Bearer "+token, "")

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	if body["device_count"] != float64(1) {
		t.Errorf("expected device_count 1, got %v", body["device_count"])
	}
	if body["subscription_count"] != float64(1) {
		t.Errorf("expected subscription_count 1, got %v", body["subscription_count"])
	}
	if body["max_stores"] != float64(3) {
		t.Errorf("expected max_stores 3, got %v", body["max_stores"])
	}
	if body["max_pos_instances"] != float64(5) {
		t.Errorf("expected max_pos_instances 5, got %v", body["max_pos_instances"])
	}
}

func TestWebUsage_RejectsNoSession(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()

	rec := doJSON(mux, http.MethodGet, "/api/v1/web/usage", "", "")

	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401, got %d", rec.Code)
	}
}

// ── GET /api/v1/web/devices ───────────────────────────────────────

func TestWebDevices_ReturnsMachines(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	_, token := seedDashboardTenant(t, app, "devices@test.com")

	rec := doJSON(mux, http.MethodGet, "/api/v1/web/devices", "Bearer "+token, "")

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body struct {
		Devices []map[string]any `json:"devices"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	if len(body.Devices) != 1 {
		t.Fatalf("expected 1 device, got %d", len(body.Devices))
	}
	if body.Devices[0]["machine_id"] != "mach-1" {
		t.Errorf("expected machine_id mach-1, got %v", body.Devices[0]["machine_id"])
	}
}

// ── POST /api/v1/web/devices/{id}/revoke ─────────────────────────

func TestWebRevokeDevice_SetsRevokedAt(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	_, token := seedDashboardTenant(t, app, "revoke@test.com")

	// Fetch the device to learn its record id.
	rec := doJSON(mux, http.MethodGet, "/api/v1/web/devices", "Bearer "+token, "")
	var list struct {
		Devices []map[string]any `json:"devices"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &list); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	if len(list.Devices) != 1 {
		t.Fatalf("expected 1 device, got %d", len(list.Devices))
	}
	id := list.Devices[0]["id"].(string)

	rec = doJSON(mux, http.MethodPost, "/api/v1/web/devices/"+id+"/revoke", "Bearer "+token, "")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body struct {
		Status    string `json:"status"`
		RevokedAt string `json:"revoked_at"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	if body.Status != "revoked" {
		t.Errorf("expected status revoked, got %q", body.Status)
	}
	if body.RevokedAt == "" {
		t.Error("expected revoked_at timestamp")
	}

	// Second call is idempotent — still 200, still revoked.
	rec2 := doJSON(mux, http.MethodPost, "/api/v1/web/devices/"+id+"/revoke", "Bearer "+token, "")
	if rec2.Code != http.StatusOK {
		t.Fatalf("expected 200 on repeat revoke, got %d", rec2.Code)
	}
}

func TestWebRevokeDevice_RejectsMissingDevice(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	_, token := seedDashboardTenant(t, app, "revoke-missing@test.com")

	rec := doJSON(mux, http.MethodPost, "/api/v1/web/devices/nonexistent/revoke", "Bearer "+token, "")
	if rec.Code != http.StatusNotFound {
		t.Fatalf("expected 404, got %d", rec.Code)
	}
}

func TestWebRevokeDevice_RejectsNoSession(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()

	rec := doJSON(mux, http.MethodPost, "/api/v1/web/devices/x/revoke", "", "")
	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401, got %d", rec.Code)
	}
}

// ── GET /api/v1/admin/tenants ─────────────────────────────────────

func TestAdminListTenants_RequiresKey(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()

	rec := doJSON(mux, http.MethodGet, "/api/v1/admin/tenants", "", "")

	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401 without admin key, got %d", rec.Code)
	}
}

func TestAdminListTenants_ReturnsTenants(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	seedDashboardTenant(t, app, "admin-list@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	rec := doJSON(mux, http.MethodGet, "/api/v1/admin/tenants", "Bearer secret-admin-key", "")

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body struct {
		Tenants []map[string]any `json:"tenants"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	found := false
	for _, tn := range body.Tenants {
		if tn["email"] == "admin-list@test.com" {
			found = true
		}
	}
	if !found {
		t.Error("expected seeded tenant in list")
	}
}

// ── GET /api/v1/admin/tenants?search= + pagination ────────────────

func TestAdminListTenants_SearchFilters(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	seedDashboardTenant(t, app, "search-alpha@test.com")
	seedDashboardTenant(t, app, "search-beta@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	rec := doJSON(mux, http.MethodGet,
		"/api/v1/admin/tenants?search=ALPHA", "Bearer secret-admin-key", "")

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body struct {
		Tenants []map[string]any `json:"tenants"`
		Total   int              `json:"total"`
		Search  string           `json:"search"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	if body.Total != 1 {
		t.Errorf("expected total=1 for search=ALPHA, got %d", body.Total)
	}
	if len(body.Tenants) != 1 || body.Tenants[0]["email"] != "search-alpha@test.com" {
		t.Errorf("expected only alpha tenant, got %v", body.Tenants)
	}
	if body.Search != "ALPHA" {
		t.Errorf("expected search echo ALPHA, got %q", body.Search)
	}
}

func TestAdminListTenants_Pagination(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	seedDashboardTenant(t, app, "page-a@test.com")
	seedDashboardTenant(t, app, "page-b@test.com")
	seedDashboardTenant(t, app, "page-c@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	// perPage=2 → page 1 should return 2 tenants, total=3.
	rec := doJSON(mux, http.MethodGet,
		"/api/v1/admin/tenants?page=1&perPage=2", "Bearer secret-admin-key", "")

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body struct {
		Tenants []map[string]any `json:"tenants"`
		Total   int              `json:"total"`
		Page    int              `json:"page"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	if body.Total != 3 {
		t.Errorf("expected total=3, got %d", body.Total)
	}
	if len(body.Tenants) != 2 {
		t.Errorf("expected 2 tenants on page 1, got %d", len(body.Tenants))
	}
	if body.Page != 1 {
		t.Errorf("expected page=1, got %d", body.Page)
	}
}

// ── POST /api/v1/admin/tenants/{id}/renew ─────────────────────────

func TestAdminRenew_ExtendsSubscription(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	tenantID, _ := seedDashboardTenant(t, app, "renew@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

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
	if body["status"] != "active" {
		t.Errorf("expected status active, got %v", body["status"])
	}
	if body["expires_at"] == "" {
		t.Error("expected expires_at to be set")
	}
}

// ── POST /api/v1/admin/tenants/{id}/tier-override ─────────────────

func TestAdminTierOverride_UpdatesTier(t *testing.T) {
	app, mux := dashboardMux(t)
	defer app.Cleanup()
	tenantID, _ := seedDashboardTenant(t, app, "override@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	rec := doJSON(mux, http.MethodPost,
		"/api/v1/admin/tenants/"+tenantID+"/tier-override",
		"Bearer secret-admin-key", `{"tier_key":"premium","reason":"test"}`)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	if body["tier_key"] != "premium" {
		t.Errorf("expected tier_key premium, got %v", body["tier_key"])
	}
}

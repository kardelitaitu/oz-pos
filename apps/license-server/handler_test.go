package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/pocketbase/pocketbase/apis"
	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
	"github.com/pocketbase/pocketbase/tools/types"
)

// ── Test Infrastructure ──────────────────────────────────────────

func newTestAppFactory(t *testing.T) *tests.TestApp {
	t.Helper()

	app, err := tests.NewTestApp()
	if err != nil {
		t.Fatalf("failed to create test app: %v", err)
	}

	createTestCollections(t, app)
	registerTestRoutes(t, app)

	return app
}

func createTestCollections(t *testing.T, app *tests.TestApp) {
	t.Helper()

	tenants := core.NewBaseCollection("tenants")
	tenants.Fields.Add(
		&core.EmailField{Name: "email", Required: true},
		&core.TextField{Name: "phone", Required: true},
		&core.TextField{Name: "api_key", Required: true},
		&core.SelectField{Name: "status", Required: true, Values: []string{"active", "suspended", "revoked"}},
	)
	tenants.CreateRule = types.Pointer("")
	tenants.ListRule = types.Pointer("")
	tenants.ViewRule = types.Pointer("")
	if err := app.Save(tenants); err != nil {
		t.Fatalf("failed to create tenants collection: %v", err)
	}

	licenseKeys := core.NewBaseCollection("license_keys")
	licenseKeys.Fields.Add(
		&core.TextField{Name: "key", Required: true},
		&core.SelectField{Name: "tier_key", Required: true, Values: []string{"free", "pro", "premium", "enterprise"}},
		&core.NumberField{Name: "max_stores", Required: true},
		&core.NumberField{Name: "max_pos_instances", Required: true},
		&core.JSONField{Name: "allowed_types", Required: true},
		&core.SelectField{Name: "status", Required: true, Values: []string{"unused", "activated", "expired", "revoked"}},
		&core.DateField{Name: "expires_at", Required: true},
		&core.DateField{Name: "activated_at"},
		&core.RelationField{Name: "activated_by", CollectionId: tenants.Id, MaxSelect: 1},
		&core.DateField{Name: "revoked_at"},
		&core.TextField{Name: "notes"},
	)
	licenseKeys.CreateRule = types.Pointer("")
	licenseKeys.ListRule = types.Pointer("")
	licenseKeys.ViewRule = types.Pointer("")
	licenseKeys.UpdateRule = types.Pointer("")
	if err := app.Save(licenseKeys); err != nil {
		t.Fatalf("failed to create license_keys collection: %v", err)
	}

	subscriptions := core.NewBaseCollection("subscriptions")
	subscriptions.Fields.Add(
		&core.RelationField{Name: "tenant_id", Required: true, CollectionId: tenants.Id, MaxSelect: 1},
		&core.SelectField{Name: "tier_key", Required: true, Values: []string{"free", "pro", "premium", "enterprise"}},
		&core.NumberField{Name: "max_stores"},
		&core.NumberField{Name: "max_pos_instances"},
		&core.JSONField{Name: "allowed_types"},
		&core.SelectField{Name: "status", Required: true, Values: []string{"active", "expired", "grace_period", "revoked"}},
		&core.DateField{Name: "starts_at", Required: true},
		&core.DateField{Name: "expires_at", Required: true},
		&core.DateField{Name: "grace_until"},
		&core.TextField{Name: "signed_payload", Required: true},
		&core.TextField{Name: "signature", Required: true},
	)
	subscriptions.CreateRule = types.Pointer("")
	subscriptions.ListRule = types.Pointer("")
	subscriptions.ViewRule = types.Pointer("")
	subscriptions.UpdateRule = types.Pointer("")
	if err := app.Save(subscriptions); err != nil {
		t.Fatalf("failed to create subscriptions collection: %v", err)
	}

	tenantMachines := core.NewBaseCollection("tenant_machines")
	tenantMachines.Fields.Add(
		&core.RelationField{Name: "tenant_id", Required: true, CollectionId: tenants.Id, MaxSelect: 1},
		&core.DateField{Name: "first_seen_at"},
		&core.DateField{Name: "last_seen_at"},
	)
	tenantMachines.CreateRule = types.Pointer("")
	tenantMachines.ListRule = types.Pointer("")
	tenantMachines.ViewRule = types.Pointer("")
	if err := app.Save(tenantMachines); err != nil {
		t.Fatalf("failed to create tenant_machines collection: %v", err)
	}
}

func registerTestRoutes(t *testing.T, app *tests.TestApp) {
	t.Helper()
	app.OnServe().BindFunc(func(se *core.ServeEvent) error {
		se.Router.POST("/api/v1/license/activate", handleActivate(app))
		se.Router.POST("/api/v1/license/renew", handleRenew(app))
		se.Router.POST("/api/v1/license/status", handleStatus(app))
		return se.Next()
	})
}

func runScenario(t *testing.T, scenario *tests.ApiScenario) {
	t.Helper()
	if scenario.TestAppFactory == nil {
		scenario.TestAppFactory = func(t testing.TB) *tests.TestApp {
			return newTestAppFactory(t.(*testing.T))
		}
	}
	scenario.Test(t)
}

// ── Seed helpers ─────────────────────────────────────────────────

func seedTenant(t *testing.T, app *tests.TestApp, tenantID, apiKey, status string) {
	t.Helper()
	col, err := app.FindCollectionByNameOrId("tenants")
	if err != nil {
		t.Fatalf("tenants collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("id", tenantID)
	// Stored emails are normalized to lowercase to match the
	// case-insensitive lookup performed by the activate handler
	// (which lowercases incoming requests before searching by email).
	rec.Set("email", strings.ToLower(tenantID+"@example.com"))
	rec.Set("phone", "-")
	rec.Set("api_key", apiKey)
	rec.Set("status", status)
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed tenant %q: %v", tenantID, err)
	}
}

// setupDirectApp creates a TestApp with collections, generates a throwaway RSA
// key for signing, creates a router, triggers OnServe to register custom
// routes, and returns both the app (for direct DB inspection) and the
// ServeEvent (whose Router can handle httptest requests).
//
// This bypasses ApiScenario's transaction isolation so that subscriptions
// seeded via seedSubscription and tenants seeded via seedTenant are visible
// to FindRecordsByFilter during handler execution.
func setupDirectApp(t *testing.T) (*tests.TestApp, *core.ServeEvent) {
	t.Helper()

	// Ensure a throwaway RSA key exists so signSubscription doesn't panic.
	initPrivateKey(t)

	app := newTestAppFactory(t)

	router, err := apis.NewRouter(app)
	if err != nil {
		t.Fatalf("failed to create router: %v", err)
	}

	se := &core.ServeEvent{
		App:    app,
		Router: router,
	}

	// Fire OnServe to register routes from registerTestRoutes bindings.
	if err := app.OnServe().Trigger(se, func(e *core.ServeEvent) error {
		return nil
	}); err != nil {
		t.Fatalf("OnServe trigger failed: %v", err)
	}

	return app, se
}

// ── Misconfiguration test helpers ────────────────────────────────
//
// createMinimalCollections creates only the collections NOT listed in the
// skip set. This lets us simulate a misconfigured server where a required
// collection (tenants, tenant_machines, subscriptions) is missing.
func createMinimalCollections(t *testing.T, app *tests.TestApp, skip map[string]bool) {
	t.Helper()

	// Create tenants first so other collections can reference its ID.
	var tenantsID string
	if !skip["tenants"] {
		tenants := core.NewBaseCollection("tenants")
		tenants.Fields.Add(
			&core.EmailField{Name: "email", Required: true},
			&core.TextField{Name: "phone", Required: true},
			&core.TextField{Name: "api_key", Required: true},
			&core.SelectField{Name: "status", Required: true, Values: []string{"active", "suspended", "revoked"}},
		)
		tenants.CreateRule = types.Pointer("")
		tenants.ListRule = types.Pointer("")
		tenants.ViewRule = types.Pointer("")
		if err := app.Save(tenants); err != nil {
			t.Fatalf("failed to create tenants collection: %v", err)
		}
		tenantsID = tenants.Id
	}

	// license_keys is always required (the handler validates keys before
	// reaching any misconfiguration path). Its activated_by relation field
	// references tenants — only add it when tenants is present.
	licenseKeys := core.NewBaseCollection("license_keys")
	licenseKeys.Fields.Add(
		&core.TextField{Name: "key", Required: true},
		&core.SelectField{Name: "tier_key", Required: true, Values: []string{"free", "pro", "premium", "enterprise"}},
		&core.NumberField{Name: "max_stores", Required: true},
		&core.NumberField{Name: "max_pos_instances", Required: true},
		&core.JSONField{Name: "allowed_types", Required: true},
		&core.SelectField{Name: "status", Required: true, Values: []string{"unused", "activated", "expired", "revoked"}},
		&core.DateField{Name: "expires_at", Required: true},
		&core.DateField{Name: "activated_at"},
		&core.DateField{Name: "revoked_at"},
		&core.TextField{Name: "notes"},
	)
	if tenantsID != "" {
		licenseKeys.Fields.Add(&core.RelationField{Name: "activated_by", CollectionId: tenantsID, MaxSelect: 1})
	}
	licenseKeys.CreateRule = types.Pointer("")
	licenseKeys.ListRule = types.Pointer("")
	licenseKeys.ViewRule = types.Pointer("")
	licenseKeys.UpdateRule = types.Pointer("")
	if err := app.Save(licenseKeys); err != nil {
		t.Fatalf("failed to create license_keys collection: %v", err)
	}

	if !skip["tenant_machines"] && tenantsID != "" {
		tenantMachines := core.NewBaseCollection("tenant_machines")
		tenantMachines.Fields.Add(
			&core.RelationField{Name: "tenant_id", Required: true, CollectionId: tenantsID, MaxSelect: 1},
			&core.DateField{Name: "first_seen_at"},
			&core.DateField{Name: "last_seen_at"},
		)
		tenantMachines.CreateRule = types.Pointer("")
		tenantMachines.ListRule = types.Pointer("")
		tenantMachines.ViewRule = types.Pointer("")
		if err := app.Save(tenantMachines); err != nil {
			t.Fatalf("failed to create tenant_machines collection: %v", err)
		}
	}

	if !skip["subscriptions"] && tenantsID != "" {
		subscriptions := core.NewBaseCollection("subscriptions")
		subscriptions.Fields.Add(
			&core.RelationField{Name: "tenant_id", Required: true, CollectionId: tenantsID, MaxSelect: 1},
			&core.SelectField{Name: "tier_key", Required: true, Values: []string{"free", "pro", "premium", "enterprise"}},
			&core.NumberField{Name: "max_stores"},
			&core.NumberField{Name: "max_pos_instances"},
			&core.JSONField{Name: "allowed_types"},
			&core.SelectField{Name: "status", Required: true, Values: []string{"active", "expired", "grace_period", "revoked"}},
			&core.DateField{Name: "starts_at", Required: true},
			&core.DateField{Name: "expires_at", Required: true},
			&core.DateField{Name: "grace_until"},
			&core.TextField{Name: "signed_payload", Required: true},
			&core.TextField{Name: "signature", Required: true},
		)
		subscriptions.CreateRule = types.Pointer("")
		subscriptions.ListRule = types.Pointer("")
		subscriptions.ViewRule = types.Pointer("")
		subscriptions.UpdateRule = types.Pointer("")
		if err := app.Save(subscriptions); err != nil {
			t.Fatalf("failed to create subscriptions collection: %v", err)
		}
	}
}

// setupDirectAppWithoutCollection creates a TestApp and ServeEvent with all
// collections EXCEPT the ones listed in skip. This is used to test the
// "server misconfiguration" error paths in the handlers.
func setupDirectAppWithoutCollection(t *testing.T, skip map[string]bool) (*tests.TestApp, *core.ServeEvent) {
	t.Helper()

	initPrivateKey(t)

	app, err := tests.NewTestApp()
	if err != nil {
		t.Fatalf("failed to create test app: %v", err)
	}

	createMinimalCollections(t, app, skip)

	// Register routes manually instead of using registerTestRoutes so that
	// we match the same set of collections the handler expects.
	app.OnServe().BindFunc(func(se *core.ServeEvent) error {
		se.Router.POST("/api/v1/license/activate", handleActivate(app))
		se.Router.POST("/api/v1/license/renew", handleRenew(app))
		se.Router.POST("/api/v1/license/status", handleStatus(app))
		return se.Next()
	})

	router, err := apis.NewRouter(app)
	if err != nil {
		t.Fatalf("failed to create router: %v", err)
	}

	se := &core.ServeEvent{
		App:    app,
		Router: router,
	}

	if err := app.OnServe().Trigger(se, func(e *core.ServeEvent) error {
		return nil
	}); err != nil {
		t.Fatalf("OnServe trigger failed: %v", err)
	}

	return app, se
}

// seedSubscription inserts a subscription record directly via app.Save
// (bypassing ApiScenario transaction isolation). Only use with setupDirectApp.
func seedSubscription(t *testing.T, app *tests.TestApp, tenantID, tierKey, status string) {
	t.Helper()
	col, err := app.FindCollectionByNameOrId("subscriptions")
	if err != nil {
		t.Fatalf("subscriptions collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	// Relation fields store as JSON arrays; supply as []string to match
	// PocketBase's internal JSON_EXTRACT filter resolution.
	rec.Set("tenant_id", []string{tenantID})
	rec.Set("tier_key", tierKey)
	rec.Set("max_stores", 5)
	rec.Set("max_pos_instances", 3)
	rec.Set("allowed_types", `["restaurant-pos", "store-pos"]`)
	rec.Set("status", status)
	rec.Set("starts_at", time.Now().UTC().Format(time.RFC3339))
	rec.Set("expires_at", time.Now().UTC().AddDate(1, 0, 0).Format(time.RFC3339))
	rec.Set("grace_until", time.Now().UTC().AddDate(1, 0, 14).Format(time.RFC3339))
	rec.Set("signed_payload", "{}")
	rec.Set("signature", "dummy-sig")
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed subscription for %q: %v", tenantID, err)
	}
}

// seedSubscriptionWithLimits is seedSubscription with caller-supplied
// per-tier quota fields. Used by M5-audit tests to seed an active
// Enterprise subscription at non-default limits, before renewing with
// a Pro key to verify the Enterprise→Pro downgrade path.
func seedSubscriptionWithLimits(t *testing.T, app *tests.TestApp, tenantID, tierKey, status string, maxStores, maxPOSInstances int, allowedTypes string) {
	t.Helper()
	col, err := app.FindCollectionByNameOrId("subscriptions")
	if err != nil {
		t.Fatalf("subscriptions collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("tenant_id", []string{tenantID})
	rec.Set("tier_key", tierKey)
	rec.Set("max_stores", maxStores)
	rec.Set("max_pos_instances", maxPOSInstances)
	rec.Set("allowed_types", allowedTypes)
	rec.Set("status", status)
	rec.Set("starts_at", time.Now().UTC().Format(time.RFC3339))
	rec.Set("expires_at", time.Now().UTC().AddDate(1, 0, 0).Format(time.RFC3339))
	rec.Set("grace_until", time.Now().UTC().AddDate(1, 0, 14).Format(time.RFC3339))
	rec.Set("signed_payload", "{}")
	rec.Set("signature", "dummy-sig")
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed subscription for %q: %v", tenantID, err)
	}
}

func seedLicenseKey(t *testing.T, app *tests.TestApp, key, tierKey, status, expiresAt string) {
	t.Helper()
	col, err := app.FindCollectionByNameOrId("license_keys")
	if err != nil {
		t.Fatalf("license_keys collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("key", key)
	rec.Set("tier_key", tierKey)
	rec.Set("max_stores", 5)
	rec.Set("max_pos_instances", 3)
	rec.Set("allowed_types", `["restaurant-pos", "store-pos"]`)
	rec.Set("status", status)
	rec.Set("expires_at", expiresAt)
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed license key %q: %v", key, err)
	}
}

// seedLicenseKeyWithLimits is seedLicenseKey with caller-supplied
// per-tier quota fields (max_stores, max_pos_instances, allowed_types).
// Used by M5-audit tests to express distinct limits per key so the
// handler's fix (source quotas from NEW key, not OLD sub) can be
// unambiguously observed.
func seedLicenseKeyWithLimits(t *testing.T, app *tests.TestApp, key, tierKey, status, expiresAt string, maxStores, maxPOSInstances int, allowedTypes string) {
	t.Helper()
	col, err := app.FindCollectionByNameOrId("license_keys")
	if err != nil {
		t.Fatalf("license_keys collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("key", key)
	rec.Set("tier_key", tierKey)
	rec.Set("max_stores", maxStores)
	rec.Set("max_pos_instances", maxPOSInstances)
	rec.Set("allowed_types", allowedTypes)
	rec.Set("status", status)
	rec.Set("expires_at", expiresAt)
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed license key %q: %v", key, err)
	}
}

// ── Tests: Status Handler ────────────────────────────────────────

// TestStatusHandler_UnknownAPIKey verifies that a request with an api_key
// that does not match any tenant in the database returns 401, not 404.
// This is the C1-audit successor to the old TestStatusHandler_TenantNotFound:
// with Bearer authentication, the server can no longer leak whether a given
// tenant_id vs. api_key-vs-tenant combination is "valid".
func TestStatusHandler_UnknownAPIKey(t *testing.T) {
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	req := httptest.NewRequest(http.MethodPost, "/api/v1/license/status", nil)
	req.Header.Set("Authorization", "Bearer notarealapikey0001")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "invalid api_key") {
		t.Errorf("expected 'invalid api_key' in response, got: %s", rec.Body.String())
	}
}

func TestStatusHandler_TenantNoSubscription(t *testing.T) {
	// Seeds a tenant without any subscription — handler should return
	// fallback response with active:false and tier:"unknown".
	// Auth happens via Bearer header (C1 audit fix); no path or query
	// parameters carry credentials.
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "nosubtest000001", "nosubapikey0001", "active")

	req := httptest.NewRequest(http.MethodPost, "/api/v1/license/status", nil)
	req.Header.Set("Authorization", "Bearer nosubapikey0001")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var body map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}
	if body["tenant_id"] != "nosubtest000001" {
		t.Errorf("expected tenant_id 'nosubtest000001', got %v", body["tenant_id"])
	}
	if body["active"] != false {
		t.Errorf("expected active=false, got %v", body["active"])
	}
	if body["tier"] != "unknown" {
		t.Errorf("expected tier 'unknown', got %v", body["tier"])
	}
}

// ── Tests: Activate Handler ──────────────────────────────────────

func TestActivateHandler_MissingFields(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/activate",
		Body:            strings.NewReader(`{}`),
		ExpectedStatus:  400,
		ExpectedContent: []string{`"error"`, "required"},
	})
}

func TestActivateHandler_InvalidKey(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method: "POST",
		URL:    "/api/v1/license/activate",
		Body: strings.NewReader(`{
			"key": "OZINVALIDKEY001",
			"email": "testactivate001@example.com",
			"machine_id": "machinetest0001"
		}`),
		ExpectedStatus:  401,
		ExpectedContent: []string{`"error"`},
	})
}

func TestActivateHandler_InvalidJSON(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/activate",
		Body:            strings.NewReader(`not json`),
		ExpectedStatus:  400,
		ExpectedContent: []string{`"error"`},
	})
}

func TestActivateHandler_AlreadyUsedKey(t *testing.T) {
	// Seed a license key with status "activated" — handler should return 401.
	runScenario(t, &tests.ApiScenario{
		Method: "POST",
		URL:    "/api/v1/license/activate",
		Body: strings.NewReader(`{
			"key": "OZ-USED-KEY-00001",
			"email": "usedkeytest0001@example.com",
			"machine_id": "usedmachin00001"
		}`),
		ExpectedStatus:  401,
		ExpectedContent: []string{`"error"`, "already used"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKey(t.(*testing.T), app,
				"OZ-USED-KEY-00001", "pro", "activated",
				"2099-12-31 23:59:59.000Z")
		},
	})
}

func TestActivateHandler_ExpiredKey(t *testing.T) {
	// Seed an unused but expired license key — handler should return 410.
	runScenario(t, &tests.ApiScenario{
		Method: "POST",
		URL:    "/api/v1/license/activate",
		Body: strings.NewReader(`{
			"key": "OZ-EXPIRED-KEY01",
			"email": "expiredkeytes001@example.com",
			"machine_id": "expmachint00001"
		}`),
		ExpectedStatus:  410,
		ExpectedContent: []string{`"error"`, "expired"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKey(t.(*testing.T), app,
				"OZ-EXPIRED-KEY01", "pro", "unused",
				"2020-01-01 00:00:00.000Z")
		},
	})
}

func TestActivateHandler_RevokedKey(t *testing.T) {
	// Seed a revoked license key — handler should return 401.
	runScenario(t, &tests.ApiScenario{
		Method: "POST",
		URL:    "/api/v1/license/activate",
		Body: strings.NewReader(`{
			"key": "OZ-REVOKED-KEY01",
			"email": "revokedkeyte0001@example.com",
			"machine_id": "revmachint00001"
		}`),
		ExpectedStatus:  401,
		ExpectedContent: []string{`"error"`, "already used"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKey(t.(*testing.T), app,
				"OZ-REVOKED-KEY01", "pro", "revoked",
				"2099-12-31 23:59:59.000Z")
		},
	})
}

// ── Tests: Renew Handler ─────────────────────────────────────────

func TestRenewHandler_MissingFields(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/renew",
		Body:            strings.NewReader(`{}`),
		ExpectedStatus:  400,
		ExpectedContent: []string{`"error"`, "required"},
	})
}

func TestRenewHandler_InvalidAPIKey(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method: "POST",
		URL:    "/api/v1/license/renew",
		Body: strings.NewReader(`{
			"tenant_id": "tsxinvalid00001",
			"api_key": "invalidkey00001",
			"key": "OZ-RENEW-KEY"
		}`),
		ExpectedStatus:  401,
		ExpectedContent: []string{`"error"`},
	})
}

func TestRenewHandler_InvalidJSON(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/renew",
		Body:            strings.NewReader(`not json`),
		ExpectedStatus:  400,
		ExpectedContent: []string{`"error"`},
	})
}

func TestRenewHandler_WrongTenantID(t *testing.T) {
	resetRateLimiters()
	// Seed a tenant with api_key "wrongapik000001" and id "wrongtenant0002",
	// then send a request with a different tenant_id.
	// The handler should authenticate via api_key but reject the mismatched tenant_id.
	runScenario(t, &tests.ApiScenario{
		Method: "POST",
		URL:    "/api/v1/license/renew",
		Body: strings.NewReader(`{
			"tenant_id": "wrongtenant0001",
			"api_key": "wrongapik000001",
			"key": "OZ-RENEW-KEY"
		}`),
		ExpectedStatus:  401,
		ExpectedContent: []string{`tenant_id does not match api_key`},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedTenant(t.(*testing.T), app, "wrongtenant0002", "wrongapik000001", "active")
		},
	})
}

func TestRenewHandler_SuspendedTenant(t *testing.T) {
	resetRateLimiters()
	// Seed a tenant with status "suspended" — the handler should reject renewal.
	runScenario(t, &tests.ApiScenario{
		Method: "POST",
		URL:    "/api/v1/license/renew",
		Body: strings.NewReader(`{
			"tenant_id": "susptest0000001",
			"api_key": "suspapikey00001",
			"key": "OZ-RENEW-KEY"
		}`),
		ExpectedStatus:  401,
		ExpectedContent: []string{`not active`},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedTenant(t.(*testing.T), app, "susptest0000001", "suspapikey00001", "suspended")
		},
	})
}

// ── Direct Tests (bypass ApiScenario for subscription lookups) ───
//
// These tests use setupDirectApp (apis.NewRouter + OnServe.Trigger) instead
// of ApiScenario because ApiScenario's transaction isolation prevents
// FindRecordsByFilter on relation fields (subscriptions.tenant_id) from
// seeing records seeded in BeforeTestFunc.

func TestStatusHandler_WithSubscription(t *testing.T) {
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Seed tenant + subscription directly on the app.
	seedTenant(t, app, "stathappy000001", "stathappykey001", "active")
	seedSubscription(t, app, "stathappy000001", "pro", "active")

	req := httptest.NewRequest(http.MethodPost, "/api/v1/license/status", nil)
	req.Header.Set("Authorization", "Bearer stathappykey001")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var body map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}

	if body["tier"] != "pro" {
		t.Errorf("expected tier 'pro', got %v", body["tier"])
	}
	if body["active"] != true {
		t.Errorf("expected active=true, got %v", body["active"])
	}
	if body["tenant_id"] != "stathappy000001" {
		t.Errorf("expected tenant_id, got %v", body["tenant_id"])
	}
	if _, ok := body["expires_at"]; !ok {
		t.Error("expected expires_at in response")
	}
	if _, ok := body["grace_until"]; !ok {
		t.Error("expected grace_until in response")
	}
}

func resetRateLimiters() {
	ipRateLimiter.stop()
	keyFailTracker.stop()

	ipRateLimiter.mu.Lock()
	ipRateLimiter.buckets = make(map[string]*tokenBucket)
	ipRateLimiter.mu.Unlock()

	keyFailTracker.mu.Lock()
	keyFailTracker.failures = make(map[string]*keyFailures)
	keyFailTracker.mu.Unlock()

	ipRateLimiter.startCleanup()
	keyFailTracker.startCleanup()
}

func TestRenewHandler_NoSubscription(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Seed an active tenant but NO subscription.
	seedTenant(t, app, "rnwsub000000001", "rnwsubkey000001", "active")

	// Need a seeded unused key too so it passes key validation before checking subscriptions
	seedLicenseKey(t, app, "rnwsubkey000001-key", "pro", "unused", "2099-12-31 23:59:59.000Z")

	body := strings.NewReader(`{"tenant_id":"rnwsub000000001","api_key":"rnwsubkey000001","key":"rnwsubkey000001-key"}`)
	req := httptest.NewRequest("POST", "/api/v1/license/renew", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Fatalf("expected 404, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "no active subscription found") {
		t.Errorf("expected 'no active subscription found', got: %s", rec.Body.String())
	}
}

func TestRenewHandler_WithSubscription(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Seed tenant + active subscription.
	seedTenant(t, app, "rnwhappy0000001", "rnwhappykey0001", "active")
	seedSubscription(t, app, "rnwhappy0000001", "pro", "active")

	// Seed valid unused key for renewal
	seedLicenseKey(t, app, "rnwhappykey0001-key", "pro", "unused", "2099-12-31 23:59:59.000Z")

	body := strings.NewReader(`{"tenant_id":"rnwhappy0000001","api_key":"rnwhappykey0001","key":"rnwhappykey0001-key"}`)
	req := httptest.NewRequest("POST", "/api/v1/license/renew", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}

	if _, ok := resp["signed_payload"]; !ok {
		t.Error("expected signed_payload in response")
	}
	if _, ok := resp["signature"]; !ok {
		t.Error("expected signature in response")
	}

	// Verify the signed payload includes quota fields carried over from the old subscription.
	if payloadStr, ok := resp["signed_payload"].(string); ok {
		var sp SubscriptionPayload
		if err := json.Unmarshal([]byte(payloadStr), &sp); err != nil {
			t.Errorf("failed to parse signed_payload: %v", err)
		} else {
			if sp.MaxStores != 5 {
				t.Errorf("expected max_stores=5 in renewal payload, got %d", sp.MaxStores)
			}
			if sp.MaxPOSInstances != 3 {
				t.Errorf("expected max_pos_instances=3 in renewal payload, got %d", sp.MaxPOSInstances)
			}
		}
	}

	// Verify old subscription is now expired.
	subs, err := app.FindRecordsByFilter(
		"subscriptions",
		"tenant_id = {:tenant_id}",
		"", 2, 0,
		map[string]any{"tenant_id": "rnwhappy0000001"},
	)
	if err != nil {
		t.Fatalf("failed to query subscriptions: %v", err)
	}
	if len(subs) != 2 {
		t.Fatalf("expected 2 subscriptions, got %d", len(subs))
	}
	// Order is non-deterministic (records may share the same starts_at).
	// Just verify one is active and one is expired.
	var foundActive, foundExpired bool
	for _, s := range subs {
		switch s.GetString("status") {
		case "active":
			foundActive = true
		case "expired":
			foundExpired = true
		}
	}
	if !foundActive {
		t.Error("expected one active subscription after renewal")
	}
	if !foundExpired {
		t.Error("expected one expired subscription after renewal")
	}
}

// TestRenewHandler_TierChange_UsesNewKeyLimits verifies the M5-audit fix:
// when a customer churns to a different tier via renewal, the renewed
// subscription's quota fields (max_stores, max_pos_instances, allowed_types)
// MUST come from the NEW license key, not the OLD subscription. Previously
// an upgrade (Pro→Enterprise) silently capped the customer at Pro limits,
// and a downgrade (Enterprise→Pro) silently over-provisioned them at
// Enterprise limits until the next renewal carried the correct values.
func TestRenewHandler_TierChange_UsesNewKeyLimits(t *testing.T) {
	t.Run("ProToEnterprise_Upgrade", func(t *testing.T) {
		resetRateLimiters()
		app, se := setupDirectApp(t)
		defer app.Cleanup()

		// Tenant starts on Pro (max_stores=5, max_pos=3, 2 types).
		seedTenant(t, app, "rnwupgradetn001", "rnwupgradetn001-key", "active")
		seedSubscription(t, app, "rnwupgradetn001", "pro", "active")

		// New Enterprise key with HIGHER limits.
		seedLicenseKeyWithLimits(t, app,
			"OZ-RNW-UPG-ENT01",
			"enterprise", "unused", "2099-12-31 23:59:59.000Z",
			20, 10, `["restaurant-pos","store-pos","kds"]`)

		body := strings.NewReader(`{"tenant_id":"rnwupgradetn001","api_key":"rnwupgradetn001-key","key":"OZ-RNW-UPG-ENT01"}`)
		req := httptest.NewRequest("POST", "/api/v1/license/renew", body)
		req.Header.Set("Content-Type", "application/json")
		rec := httptest.NewRecorder()
		mux, err := se.Router.BuildMux()
		if err != nil {
			t.Fatalf("BuildMux failed: %v", err)
		}
		mux.ServeHTTP(rec, req)

		if rec.Code != http.StatusOK {
			t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
		}
		var resp map[string]any
		if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
			t.Fatalf("failed to parse response: %v", err)
		}
		var sp SubscriptionPayload
		payloadStr, _ := resp["signed_payload"].(string)
		if err := json.Unmarshal([]byte(payloadStr), &sp); err != nil {
			t.Fatalf("failed to parse signed_payload: %v", err)
		}

		// M5 audit assertion: quotas must come from the NEW Enterprise
		// key, NOT from the OLD Pro subscription (which had 5/3/2).
		if sp.MaxStores != 20 {
			t.Errorf("Pro→Enterprise upgrade: expected max_stores=20 (from NEW key), got %d", sp.MaxStores)
		}
		if sp.MaxPOSInstances != 10 {
			t.Errorf("Pro→Enterprise upgrade: expected max_pos_instances=10 (from NEW key), got %d", sp.MaxPOSInstances)
		}
		if len(sp.AllowedTypes) != 3 {
			t.Errorf("Pro→Enterprise upgrade: expected 3 allowed_types (from NEW key), got %v", sp.AllowedTypes)
		}
	})

	t.Run("EnterpriseToPro_Downgrade", func(t *testing.T) {
		resetRateLimiters()
		app, se := setupDirectApp(t)
		defer app.Cleanup()

		// Tenant starts on Enterprise (max_stores=20, max_pos=10, 3 types).
		seedTenant(t, app, "rnwdowngrade001", "rnwdowngrade001-key", "active")
		seedSubscriptionWithLimits(t, app, "rnwdowngrade001", "enterprise", "active",
			20, 10, `["restaurant-pos","store-pos","kds"]`)

		// New Pro key with LOWER limits.
		seedLicenseKeyWithLimits(t, app,
			"OZ-RNW-DWN-PRO01",
			"pro", "unused", "2099-12-31 23:59:59.000Z",
			5, 3, `["restaurant-pos","store-pos"]`)

		body := strings.NewReader(`{"tenant_id":"rnwdowngrade001","api_key":"rnwdowngrade001-key","key":"OZ-RNW-DWN-PRO01"}`)
		req := httptest.NewRequest("POST", "/api/v1/license/renew", body)
		req.Header.Set("Content-Type", "application/json")
		rec := httptest.NewRecorder()
		mux, err := se.Router.BuildMux()
		if err != nil {
			t.Fatalf("BuildMux failed: %v", err)
		}
		mux.ServeHTTP(rec, req)

		if rec.Code != http.StatusOK {
			t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
		}
		var resp map[string]any
		if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
			t.Fatalf("failed to parse response: %v", err)
		}
		var sp SubscriptionPayload
		payloadStr, _ := resp["signed_payload"].(string)
		if err := json.Unmarshal([]byte(payloadStr), &sp); err != nil {
			t.Fatalf("failed to parse signed_payload: %v", err)
		}

		// M5 audit assertion: quotas must come from the NEW Pro key,
		// NOT from the OLD Enterprise subscription (which had 20/10/3).
		if sp.MaxStores != 5 {
			t.Errorf("Enterprise→Pro downgrade: expected max_stores=5 (from NEW key), got %d", sp.MaxStores)
		}
		if sp.MaxPOSInstances != 3 {
			t.Errorf("Enterprise→Pro downgrade: expected max_pos_instances=3 (from NEW key), got %d", sp.MaxPOSInstances)
		}
		if len(sp.AllowedTypes) != 2 {
			t.Errorf("Enterprise→Pro downgrade: expected 2 allowed_types (from NEW key), got %v", sp.AllowedTypes)
		}
	})
}

// TestRenewHandler_ConcurrentSameKey_OnlyOneWins verifies the C2/C3 audit
// fix: when N parallel requests try to renew with the SAME license key
// for the SAME tenant, exactly one wins (200 + signed_payload) and the
// others get 401 "invalid or already used license key". The
// activationLocks wrapper around the key-status check in renew.go
// serialises concurrent attempts so the loser goroutines see
// key.status="activated" (saved by the winner) and short-circuit.
//
// Without this lock, two goroutines can both read key.status="unused"
// before either saves "activated", silently granting an extra renewed
// subscription for one key purchase — the worst data-integrity bug in
// the audit. N=5 chosen so all 5 goroutines pass ipRateLimiter.maxPerHr
// (5 tokens/hr per IP) and reach the activationLocks contention.
func TestRenewHandler_ConcurrentSameKey_OnlyOneWins(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	const tenantID = "racerenew000001"
	const apiKey = "racerenewkey00001"
	const newKey = "OZ-RACE-RENEW-01"

	seedTenant(t, app, tenantID, apiKey, "active")
	seedSubscription(t, app, tenantID, "pro", "active")
	seedLicenseKey(t, app, newKey, "pro", "unused", "2099-12-31 23:59:59.000Z")

	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}

	const N = 5
	var wg sync.WaitGroup
	var codesMu sync.Mutex
	codes := make([]int, N)
	bodies := make([]string, N)

	// Start-gate: fan out with zero stagger so the 5 goroutines hit
	// the activationLocks window within the same scheduler tick. Without
	// this, per-goroutine spawn-stagger (10s of µs) can let goroutine 1
	// finish its renewal and unlock before goroutine 2 enters, which still
	// proves the lock works but lets the test pass even if the lock were
	// only PARTIALLY effective.
	start := make(chan struct{})

	for i := 0; i < N; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			<-start
			body := strings.NewReader(
				`{"tenant_id":"racerenew000001","api_key":"racerenewkey00001","key":"OZ-RACE-RENEW-01"}`)
			req := httptest.NewRequest("POST", "/api/v1/license/renew", body)
			req.Header.Set("Content-Type", "application/json")
			rec := httptest.NewRecorder()
			mux.ServeHTTP(rec, req)
			codesMu.Lock()
			codes[idx] = rec.Code
			bodies[idx] = rec.Body.String()
			codesMu.Unlock()
		}(i)
	}
	close(start)
	wg.Wait()

	successes, failLose := 0, 0
	var firstWinBody, firstLoseBody string
	for i, c := range codes {
		switch c {
		case http.StatusOK:
			successes++
			if firstWinBody == "" {
				firstWinBody = bodies[i]
			}
		case http.StatusUnauthorized:
			failLose++
			if firstLoseBody == "" {
				firstLoseBody = bodies[i]
			}
		}
	}
	if successes != 1 {
		t.Errorf("expected exactly 1 winning renewal (race winner), got %d — all codes: %v",
			successes, codes)
	}
	if successes+failLose != N {
		t.Errorf("internal: result count %d != N=%d (all codes: %v)",
			successes+failLose, N, codes)
	}
	// Pin the expected error message: the 4 losers should all report
	// "invalid or already used license key" because the winner saved
	// key.status="activated" before their key-record lookup. A regression
	// returning 401 with a different body (e.g. "rate limit") would mean
	// the rate limit caught them — which means the test setup leaked
	// tokens and the lock was bypassed by ordering, not by fixing the
	// race. Catch it explicitly.
	if firstLoseBody != "" && !strings.Contains(firstLoseBody, "invalid or already used") {
		t.Errorf("expected loser body to contain 'invalid or already used license key', got: %s", firstLoseBody)
	}
	if firstWinBody != "" && !strings.Contains(firstWinBody, "signed_payload") {
		t.Errorf("expected winner body to contain 'signed_payload', got: %s", firstWinBody)
	}
}

func TestActivateHandler_RateLimited(t *testing.T) {
	resetRateLimiters()

	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Exhaust rate limiter for a specific IP. PocketBase's RealIP()
	// parses RemoteAddr via net.SplitHostPort; supply port:IP form.
	testIP := "10.99.99.99"
	testAddr := testIP + ":1234"
	for i := 0; i < ipRateLimiter.maxPerHr; i++ {
		ipRateLimiter.allow(testIP)
	}

	body := strings.NewReader(`{"key":"OZ-RATELIM-KEY01","email":"rlimtenant00001@example.com","machine_id":"rlimmachin00001"}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	req.RemoteAddr = testAddr
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusTooManyRequests {
		t.Fatalf("expected 429, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "rate limit") {
		t.Errorf("expected rate-limit error message, got: %s", rec.Body.String())
	}
}

func TestActivateHandler_KeyBruteForceBlocked(t *testing.T) {
	resetRateLimiters()

	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Exhaust the key failure tracker — handler checks this before
	// looking up the key, so no seedLicenseKey needed.
	for i := 0; i < keyFailTracker.maxAttempts; i++ {
		keyFailTracker.recordFailure("OZ-BRUTE-KEY001")
	}

	body := strings.NewReader(`{"key":"OZ-BRUTE-KEY001","email":"brutteten00001@example.com","machine_id":"bruttemach00001"}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusTooManyRequests {
		t.Fatalf("expected 429, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "too many attempts") {
		t.Errorf("expected brute-force error message, got: %s", rec.Body.String())
	}
}

func TestActivateHandler_Success(t *testing.T) {
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Seed an unused license key.
	seedLicenseKey(t, app, "OZ-HAPPY-KEY0001", "pro", "unused",
		"2099-12-31 23:59:59.000Z")

	body := strings.NewReader(`{
		"key": "OZ-HAPPY-KEY0001",
		"email": "hppytenant00001@example.com",
		"machine_id": "hppymachine0001"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}

	if _, ok := resp["signed_payload"]; !ok {
		t.Error("expected signed_payload in response")
	}
	if _, ok := resp["signature"]; !ok {
		t.Error("expected signature in response")
	}
	if _, ok := resp["api_key"]; !ok {
		t.Error("expected api_key in response")
	}

	// Verify the signed payload includes quota fields.
	if payloadStr, ok := resp["signed_payload"].(string); ok {
		var sp SubscriptionPayload
		if err := json.Unmarshal([]byte(payloadStr), &sp); err != nil {
			t.Errorf("failed to parse signed_payload: %v", err)
		} else {
			if sp.MaxStores != 5 {
				t.Errorf("expected max_stores=5 in payload, got %d", sp.MaxStores)
			}
			if sp.MaxPOSInstances != 3 {
				t.Errorf("expected max_pos_instances=3 in payload, got %d", sp.MaxPOSInstances)
			}
			if len(sp.AllowedTypes) != 2 || sp.AllowedTypes[0] != "restaurant-pos" || sp.AllowedTypes[1] != "store-pos" {
				t.Errorf("expected allowed_types=[restaurant-pos, store-pos] in payload, got %v", sp.AllowedTypes)
			}
		}
	}

	// Verify tenant was created.
	tenant, err := app.FindFirstRecordByData("tenants", "email", "hppytenant00001@example.com")
	if err != nil {
		t.Fatalf("tenant should be created: %v", err)
	}
	tenantID := tenant.Id
	if tenant.GetString("status") != "active" {
		t.Errorf("tenant status should be active, got %s", tenant.GetString("status"))
	}

	// Verify key was marked activated.
	keyRec, err := app.FindFirstRecordByData("license_keys", "key", "OZ-HAPPY-KEY0001")
	if err != nil {
		t.Fatalf("license key should exist: %v", err)
	}
	if keyRec.GetString("status") != "activated" {
		t.Errorf("key status should be activated, got %s", keyRec.GetString("status"))
	}

	// Verify subscription was created.
	subs, err := app.FindRecordsByFilter(
		"subscriptions",
		"tenant_id = {:tenant_id}",
		"-starts_at", 1, 0,
		map[string]any{"tenant_id": tenantID},
	)
	if err != nil || len(subs) == 0 {
		t.Fatal("subscription should have been created")
	}
	if subs[0].GetString("status") != "active" {
		t.Errorf("subscription status should be active, got %s", subs[0].GetString("status"))
	}

	// Verify machine was registered.
	machines, err := app.FindRecordsByFilter(
		"tenant_machines",
		"tenant_id = {:tenant_id}",
		"", 1, 0,
		map[string]any{"tenant_id": tenantID},
	)
	if err != nil || len(machines) == 0 {
		t.Fatal("machine should have been registered")
	}
}

// TestActivateHandler_ExistingTenantRequiresAPIKey verifies the H1-audit gate:
// an activation request that targets an EXISTING tenant but supplies no
// api_key must be rejected with 401. Without this gate, anyone who knew
// a tenant's email + a valid license key could exfiltrate the tenant's
// signed_payload (which is sufficient to forge a working offline license
// until expires_at, since the Rust client verifies the RSA signature
// locally and trusts the payload).
func TestActivateHandler_ExistingTenantRequiresAPIKey(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Seed a pre-existing tenant with a known api_key.
	seedTenant(t, app, "existtenan00001", "existapikey0001", "active")

	// Seed an unused license key.
	seedLicenseKey(t, app, "OZ-EXIST-KEY001", "pro", "unused",
		"2099-12-31 23:59:59.000Z")

	// Activate with the existing tenant's email but NO api_key.
	// Must be rejected (H1 audit fix).
	body := strings.NewReader(`{
		"key": "OZ-EXIST-KEY001",
		"email": "existtenan00001@example.com",
		"machine_id": "existmachin0001"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401 (H1 gate), got %d: %s", rec.Code, rec.Body.String())
	}
	// Must not reveal which factor failed (email-existing vs. api_key-mismatch).
	if !strings.Contains(rec.Body.String(), "api_key required") {
		t.Errorf("expected 'api_key required' error, got: %s", rec.Body.String())
	}
}

// TestActivateHandler_ExistingTenantRejectsWrongAPIKey verifies that supplying
// any wrong api_key against an existing tenant is rejected with 401, regardless
// of whether the api_key is missing, malformed, or belongs to a different
// tenant. This guarantees the gate cannot be bypassed by guessing.
func TestActivateHandler_ExistingTenantRejectsWrongAPIKey(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "wrongkeyten0001", "correctkey0000001", "active")
	seedLicenseKey(t, app, "OZ-WRONGKEY-001", "pro", "unused",
		"2099-12-31 23:59:59.000Z")

	body := strings.NewReader(`{
		"key": "OZ-WRONGKEY-001",
		"email": "wrongkeyten0001@example.com",
		"machine_id": "wrongkeymach001",
		"api_key": "this-is-the-wrong-key00"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401 (wrong api_key), got %d: %s", rec.Code, rec.Body.String())
	}
	// Do NOT leak the real api_key (or any other tenant-record field) on failure.
	bodyStr := rec.Body.String()
	if strings.Contains(bodyStr, "correctkey0000001") {
		t.Error("response leaked the real tenant api_key on wrong-key rejection")
	}
}

// TestActivateHandler_ExistingTenantAcceptsValidAPIKey verifies the happy
// path of the H1 gate: a caller who proves tenant-administrator status by
// supplying the correct api_key for an existing tenant CAN re-activate and
// receives the signed_payload, but the server does NOT re-emit the api_key
// in the response (the caller already proved they have it; echoing it back
// would be redundant on the wire and risks accidental logging leakage).
func TestActivateHandler_ExistingTenantAcceptsValidAPIKey(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "validkeyten0001", "validkeykey00001", "active")
	seedLicenseKey(t, app, "OZ-VALIDKEY-001", "pro", "unused",
		"2099-12-31 23:59:59.000Z")

	body := strings.NewReader(`{
		"key": "OZ-VALIDKEY-001",
		"email": "validkeyten0001@example.com",
		"machine_id": "validmachin0001",
		"api_key": "validkeykey00001"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 (correct api_key), got %d: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}

	// Core fields must be present.
	if _, ok := resp["signed_payload"]; !ok {
		t.Error("expected signed_payload in response")
	}
	if _, ok := resp["signature"]; !ok {
		t.Error("expected signature in response")
	}
	if _, ok := resp["tenant_id"]; !ok {
		t.Error("expected tenant_id in response")
	}

	// api_key must NOT be re-issued on an existing-tenant first-time
	// activation of an unused key — caller proved they already have it.
	if _, ok := resp["api_key"]; ok {
		t.Error("api_key must NOT be re-issued when caller proved they already hold it (no leak on the wire)")
	}
}

// ── Tests: Misconfiguration error paths ──────────────────────────
//
// These tests verify that handleActivate returns 500 with a descriptive
// "server misconfiguration" message when a required PocketBase collection
// is missing from the database schema.

func TestActivateHandler_MissingTenantsCollection(t *testing.T) {
	app, se := setupDirectAppWithoutCollection(t, map[string]bool{"tenants": true})
	defer app.Cleanup()

	// Seed a valid license key so we pass the key-validation step.
	seedLicenseKey(t, app, "OZ-MISCFG-KEY01", "pro", "unused",
		"2099-12-31 23:59:59.000Z")

	body := strings.NewReader(`{
		"key": "OZ-MISCFG-KEY01",
		"email": "miscfgtenan0001@example.com",
		"machine_id": "miscfgmachin001"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusInternalServerError {
		t.Fatalf("expected 500, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "tenants collection not found") {
		t.Errorf("expected 'tenants collection not found', got: %s", rec.Body.String())
	}
}

func TestActivateHandler_MissingMachinesCollection(t *testing.T) {
	app, se := setupDirectAppWithoutCollection(t, map[string]bool{"tenant_machines": true})
	defer app.Cleanup()

	seedLicenseKey(t, app, "OZ-MISCFG-KEY02", "pro", "unused",
		"2099-12-31 23:59:59.000Z")

	body := strings.NewReader(`{
		"key": "OZ-MISCFG-KEY02",
		"tenant_id": "miscfgtenan0002",
		"machine_id": "miscfgmachin002",
		"contact_name": "Test",
		"email": "test@example.com"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusInternalServerError {
		t.Fatalf("expected 500, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "tenant_machines collection not found") {
		t.Errorf("expected 'tenant_machines collection not found', got: %s", rec.Body.String())
	}
}

func TestActivateHandler_MissingSubscriptionsCollection(t *testing.T) {
	app, se := setupDirectAppWithoutCollection(t, map[string]bool{"subscriptions": true})
	defer app.Cleanup()

	seedLicenseKey(t, app, "OZ-MISCFG-KEY03", "pro", "unused",
		"2099-12-31 23:59:59.000Z")

	body := strings.NewReader(`{
		"key": "OZ-MISCFG-KEY03",
		"tenant_id": "miscfgtenan0003",
		"machine_id": "miscfgmachin003",
		"contact_name": "Test",
		"email": "test@example.com"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusInternalServerError {
		t.Fatalf("expected 500, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "subscriptions collection not found") {
		t.Errorf("expected 'subscriptions collection not found', got: %s", rec.Body.String())
	}
}

// ── Tests: Renew handler misconfiguration ────────────────────────
//
// handleRenew first calls FindRecordsByFilter("subscriptions", ...) to
// locate the current active subscription. When the subscriptions collection
// is entirely missing, this returns an error (caught by the if-err guard),
// so the handler returns 404 "no active subscription found" rather than
// reaching the later FindCollectionByNameOrId misconfiguration 500 path.
// This test verifies that defensive behavior.

func TestRenewHandler_MissingSubscriptionsCollection(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectAppWithoutCollection(t, map[string]bool{"subscriptions": true})
	defer app.Cleanup()

	// Seed an active tenant so authentication succeeds.
	seedTenant(t, app, "rnwmiscfg000001", "rnwmiscfgkey001", "active")
	// Seed an unused key so key validation succeeds
	seedLicenseKey(t, app, "rnwmiscfgkey001-key", "pro", "unused", "2099-12-31 23:59:59.000Z")

	body := strings.NewReader(`{
		"tenant_id": "rnwmiscfg000001",
		"api_key": "rnwmiscfgkey001",
		"key": "rnwmiscfgkey001-key"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/renew", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	// Missing subscriptions caught by FindRecordsByFilter guard → 404.
	if rec.Code != http.StatusNotFound {
		t.Fatalf("expected 404, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "no active subscription found") {
		t.Errorf("expected 'no active subscription found', got: %s", rec.Body.String())
	}
}

// ── Tests: Activate handler regression coverage ───────────────────
//

// seedActivatedLicenseKey seeds a license key with a non-empty
// activated_by relation field, simulating a key that was already
// activated by the given tenant_id. Use this in tests that exercise
// the "key reused by same tenant" branch of the handler.
func seedActivatedLicenseKey(t *testing.T, app *tests.TestApp, key, tierKey, status, expiresAt, activatedBy string) {
	t.Helper()
	col, err := app.FindCollectionByNameOrId("license_keys")
	if err != nil {
		t.Fatalf("license_keys collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("key", key)
	rec.Set("tier_key", tierKey)
	rec.Set("max_stores", 5)
	rec.Set("max_pos_instances", 3)
	rec.Set("allowed_types", `["restaurant-pos", "store-pos"]`)
	rec.Set("status", status)
	rec.Set("expires_at", expiresAt)
	rec.Set("activated_at", time.Now().UTC().Format(time.RFC3339))
	if activatedBy != "" {
		// Relation fields are stored as JSON arrays internally;
		// pass a single-element slice to match PocketBase's layout.
		rec.Set("activated_by", []string{activatedBy})
	}
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed activated license key %q: %v", key, err)
	}
}

// TestActivateHandler_SameTenantKeyReuse verifies that a tenant who
// already activated a key can re-activate it (after re-install /
// new-machine setup) and receive the cached subscription payload.
// Previously broken: the activated_by relation field was stored as a
// JSON-encoded array (no MaxSelect), so keyRecord.GetString("activated_by")
// returned "[\"<id>\"]" which never string-matched the bare tenantID,
// causing spurious 401 "invalid or already used" errors on legitimate
// re-activation.
func TestActivateHandler_SameTenantKeyReuse(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Seed an active tenant.
	seedTenant(t, app, "reuseten0000001", "reuseApikey00001", "active")

	// Seed an active subscription for the tenant — the reuse
	// branch in activate.go looks up an existing subscription
	// to return its already-signed payload.
	seedSubscription(t, app, "reuseten0000001", "pro", "active")

	// Seed a license key already activated by that tenant.
	seedActivatedLicenseKey(t, app, "OZ-REUSE-KEY0001", "pro", "activated",
		"2099-12-31 23:59:59.000Z", "reuseten0000001")

	body := strings.NewReader(`{
		"key": "OZ-REUSE-KEY0001",
		"email": "reuseTen0000001@example.com",
		"machine_id": "newmachine00001",
		"api_key": "reuseApikey00001"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 (same-tenant reuse), got %d: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}
	if _, ok := resp["signed_payload"]; !ok {
		t.Error("expected signed_payload in response")
	}
	if _, ok := resp["signature"]; !ok {
		t.Error("expected signature in response")
	}
	// api_key is included because keyStatus was "activated" — caller
	// proved they know both a key already bound to this tenant AND
	// the matching api_key. Without the api_key match this branch
	// is gated by the H1 fix in activate.go and returns 401.
	if _, ok := resp["api_key"]; !ok {
		t.Error("expected api_key in response on key reuse by same tenant")
	}
}

// TestActivateHandler_EmailCaseInsensitive verifies that the activate
// handler normalizes incoming email so an arbitrary casing matches the
// existing tenant. Without this, "User@…com" and "user@…com" create
// duplicate tenants — each with its own api_key — and the user sees
// "invalid or already used license key" when their key was bound to
// the lower-case variant they originally typed.
func TestActivateHandler_EmailCaseInsensitive(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Seed an existing tenant at lowercase email.
	seedTenant(t, app, "caseinsen000001", "caseInsenKey0001", "active")

	// Seed an unused license key.
	seedLicenseKey(t, app, "OZ-CASE-INSEN01", "pro", "unused",
		"2099-12-31 23:59:59.000Z")

	// POST with UPPERCASE email + the existing tenant's api_key.
	// The handler must:
	// (a) normalize the email to lowercase and reuse the existing
	//     tenant (matched on email) — NOT create a new one.
	// (b) pass the H1 api_key gate since we provided the correct key.
	body := strings.NewReader(`{
		"key": "OZ-CASE-INSEN01",
		"email": "CASEINSEN000001@EXAMPLE.COM",
		"machine_id": "casemachin00001",
		"api_key": "caseInsenKey0001"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 (case-insensitive email match), got %d: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}

	// api_key must NOT be returned — if it is, that means a NEW tenant
	// was created (isNewTenant=true), which would mean our
	// normalization failed and we leaked credentials.
	if _, ok := resp["api_key"]; ok {
		t.Error("api_key must NOT be returned (existing tenant must be matched, no new tenant created)")
	}

	// Confirm only ONE tenant exists at the lowercase email — a
	// failure here means our normalization didn't take effect.
	tenants, err := app.FindRecordsByFilter(
		"tenants", "email = {:email}", "", 0, 0,
		map[string]any{"email": "caseinsen000001@example.com"},
	)
	if err != nil {
		t.Fatalf("failed to query tenants: %v", err)
	}
	if len(tenants) != 1 {
		t.Errorf("expected exactly 1 tenant matching lowercased email, got %d", len(tenants))
	}
}

// ── Tests: keyFailTracker cooldown env override ───────────────────

// TestKeyFailureTracker_CooldownEnvOverride verifies that the
// LICENSE_KEY_COOLDOWN env var shortens the per-key cooldown for
// development use, while preserving the production default (15
// minutes) when the env is unset.
func TestKeyFailureTracker_CooldownEnvOverride(t *testing.T) {
	// 1) Override path: short cooldown via env var.
	t.Setenv("LICENSE_KEY_COOLDOWN", "200ms")
	kf := &keyFailureTracker{
		failures:    make(map[string]*keyFailures),
		maxAttempts: 3,
		cooldown:    parseCooldown(),
	}
	if kf.cooldown != 200*time.Millisecond {
		t.Fatalf("expected cooldown=200ms after env override, got %v", kf.cooldown)
	}

	kf.recordFailure("OZ-COOL-OVR-001")
	kf.recordFailure("OZ-COOL-OVR-001")
	kf.recordFailure("OZ-COOL-OVR-001")

	if !kf.isBlocked("OZ-COOL-OVR-001") {
		t.Error("expected key to be blocked immediately after 3 failures")
	}

	// Wait past the short cooldown and confirm the lock releases.
	time.Sleep(250 * time.Millisecond)

	if kf.isBlocked("OZ-COOL-OVR-001") {
		t.Error("expected key to be unblocked after 200ms cooldown expired")
	}

	// 2) Default path: unset env falls back to production default.
	t.Setenv("LICENSE_KEY_COOLDOWN", "")
	if got := parseCooldown(); got != defaultKeyCooldown {
		t.Errorf("expected defaultKeyCooldown=%v with empty env, got %v", defaultKeyCooldown, got)
	}

	// 3) Malformed env falls back to default (and logs a warning
	//    that's acceptable to ignore here).
	t.Setenv("LICENSE_KEY_COOLDOWN", "not-a-duration")
	if got := parseCooldown(); got != defaultKeyCooldown {
		t.Errorf("expected defaultKeyCooldown=%v with malformed env, got %v", defaultKeyCooldown, got)
	}
}

func TestActivateHandler_MachineAlreadyExists(t *testing.T) {
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "reinstallten001", "reinstallkey0001", "active")
	seedLicenseKey(t, app, "OZ-REINSTALL-01", "pro", "unused", "2099-12-31 23:59:59.000Z")

	body := strings.NewReader(`{
		"key": "OZ-REINSTALL-01",
		"email": "reinstallten001@example.com",
		"machine_id": "reinstallmac001",
		"api_key": "reinstallkey0001"
	}`)
	req := httptest.NewRequest("POST", "/api/v1/license/activate", body)
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("first activation failed: %v", rec.Body.String())
	}

	body2 := strings.NewReader(`{
		"key": "OZ-REINSTALL-01",
		"email": "reinstallten001@example.com",
		"machine_id": "reinstallmac001",
		"api_key": "reinstallkey0001"
	}`)
	req2 := httptest.NewRequest("POST", "/api/v1/license/activate", body2)
	req2.Header.Set("Content-Type", "application/json")
	rec2 := httptest.NewRecorder()
	mux.ServeHTTP(rec2, req2)

	if rec2.Code != http.StatusOK {
		t.Fatalf("second activation failed (machine already exists): %v", rec2.Body.String())
	}
}

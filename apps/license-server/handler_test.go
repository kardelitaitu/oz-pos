package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
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
		&core.RelationField{Name: "activated_by", CollectionId: tenants.Id},
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
		se.Router.GET("/api/v1/license/status/{tenant_id}", handleStatus(app))
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
	rec.Set("email", tenantID+"@example.com")
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
		licenseKeys.Fields.Add(&core.RelationField{Name: "activated_by", CollectionId: tenantsID})
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
		se.Router.GET("/api/v1/license/status/{tenant_id}", handleStatus(app))
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

// ── Tests: Status Handler ────────────────────────────────────────

func TestStatusHandler_TenantNotFound(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "GET",
		URL:             "/api/v1/license/status/n0t4f0und000000",
		ExpectedStatus:  404,
		ExpectedContent: []string{`"error"`, "tenant not found"},
	})
}

func TestStatusHandler_TenantNoSubscription(t *testing.T) {
	// Seeds a tenant without any subscription — handler should return
	// fallback response with active:false and tier:"unknown".
	runScenario(t, &tests.ApiScenario{
		Method:          "GET",
		URL:             "/api/v1/license/status/nosubtest000001",
		ExpectedStatus:  200,
		ExpectedContent: []string{`"tenant_id":"nosubtest000001"`, `"active":false`, `"tier":"unknown"`},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedTenant(t.(*testing.T), app, "nosubtest000001", "nosubapikey0001", "active")
		},
	})
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
			"api_key": "invalidkey00001"
		}`),
		ExpectedStatus:  401,
		ExpectedContent: []string{`"error"`},
	})
}

func TestRenewHandler_InvalidJSON(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:            "/api/v1/license/renew",
		Body:            strings.NewReader(`not json`),
		ExpectedStatus:  400,
		ExpectedContent: []string{`"error"`},
	})
}

func TestRenewHandler_WrongTenantID(t *testing.T) {
	// Seed a tenant with api_key "wrongapik000001" and id "wrongtenant0002",
	// then send a request with a different tenant_id.
	// The handler should authenticate via api_key but reject the mismatched tenant_id.
	runScenario(t, &tests.ApiScenario{
		Method: "POST",
		URL:    "/api/v1/license/renew",
		Body: strings.NewReader(`{
			"tenant_id": "wrongtenant0001",
			"api_key": "wrongapik000001"
		}`),
		ExpectedStatus:  401,
		ExpectedContent: []string{`tenant_id does not match api_key`},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedTenant(t.(*testing.T), app, "wrongtenant0002", "wrongapik000001", "active")
		},
	})
}

func TestRenewHandler_SuspendedTenant(t *testing.T) {
	// Seed a tenant with status "suspended" — the handler should reject renewal.
	runScenario(t, &tests.ApiScenario{
		Method: "POST",
		URL:    "/api/v1/license/renew",
		Body: strings.NewReader(`{
			"tenant_id": "susptest0000001",
			"api_key": "suspapikey00001"
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

	req := httptest.NewRequest("GET", "/api/v1/license/status/stathappy000001", nil)
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

func TestRenewHandler_NoSubscription(t *testing.T) {
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Seed an active tenant but NO subscription.
	seedTenant(t, app, "rnwsub000000001", "rnwsubkey000001", "active")

	body := strings.NewReader(`{"tenant_id":"rnwsub000000001","api_key":"rnwsubkey000001"}`)
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
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Seed tenant + active subscription.
	seedTenant(t, app, "rnwhappy0000001", "rnwhappykey0001", "active")
	seedSubscription(t, app, "rnwhappy0000001", "pro", "active")

	body := strings.NewReader(`{"tenant_id":"rnwhappy0000001","api_key":"rnwhappykey0001"}`)
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

// resetRateLimiters clears the package-level rate limiter and key failure
// tracker so that tests manipulating these globals don't interfere with
// each other.
func resetRateLimiters() {
	ipRateLimiter.buckets = make(map[string]*tokenBucket)
	keyFailTracker.failures = make(map[string]*keyFailures)
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
	app, se := setupDirectAppWithoutCollection(t, map[string]bool{"subscriptions": true})
	defer app.Cleanup()

	// Seed an active tenant so authentication succeeds.
	seedTenant(t, app, "rnwmiscfg000001", "rnwmiscfgkey001", "active")

	body := strings.NewReader(`{
		"tenant_id": "rnwmiscfg000001",
		"api_key": "rnwmiscfgkey001"
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

package main

import (
	"strings"
	"testing"

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
		&core.TextField{Name: "business_name", Required: true},
		&core.TextField{Name: "contact_name", Required: true},
		&core.EmailField{Name: "email", Required: true},
		&core.TextField{Name: "phone"},
		&core.TextField{Name: "company"},
		&core.TextField{Name: "address_line1"},
		&core.TextField{Name: "address_line2"},
		&core.TextField{Name: "city"},
		&core.TextField{Name: "state"},
		&core.TextField{Name: "postal_code"},
		&core.TextField{Name: "country"},
		&core.TextField{Name: "notes"},
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
		&core.NumberField{Name: "max_stores", Required: true},
		&core.NumberField{Name: "max_pos_instances", Required: true},
		&core.JSONField{Name: "allowed_types", Required: true},
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
	rec.Id = tenantID
	rec.Set("business_name", "Test Business")
	rec.Set("contact_name", "Test Contact")
	rec.Set("email", "test@example.com")
	rec.Set("api_key", apiKey)
	rec.Set("status", status)
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed tenant %q: %v", tenantID, err)
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
			"tenant_id": "testactivate001",
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
			"tenant_id": "usedkeytest0001",
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
			"tenant_id": "expiredkeytes001",
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
			"tenant_id": "revokedkeyte0001",
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

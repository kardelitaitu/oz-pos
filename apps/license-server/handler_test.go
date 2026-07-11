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
		&core.RelationField{Name: "tenant_id", Required: true, CollectionId: tenants.Id},
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
		&core.RelationField{Name: "tenant_id", Required: true, CollectionId: tenants.Id},
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

// ── Tests: Status Handler ────────────────────────────────────────

func TestStatusHandler_TenantNotFound(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "GET",
		URL:             "/api/v1/license/status/n0t4f0und000000",
		ExpectedStatus:  404,
		ExpectedContent: []string{`"error"`, "tenant not found"},
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
			"tenant_id": "test000000000001",
			"machine_id": "mach00000000001"
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
			"tenant_id": "tsx000000000001",
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

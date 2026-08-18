package main

import (
	"strings"
	"testing"

	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
)

// ── Seed helper ────────────────────────────────────────────────

func seedLicenseKeyWithAddons(t *testing.T, app *tests.TestApp, key, tierKey, addonsJSON string) {
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
	rec.Set("status", "unused")
	rec.Set("expires_at", "2099-12-31 23:59:59.000Z")
	if addonsJSON != "" {
		rec.Set("addons", addonsJSON)
	}
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed license key %q: %v", key, err)
	}
}

// ── Add License Addon Tests ───────────────────────────────────

func TestAddLicenseAddon_Unauthorized(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/license-addons",
		ExpectedStatus:  401,
		ExpectedContent: []string{`"error"`, "header required"},
	})
}

func TestAddLicenseAddon_Success(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-ADDON-TEST-01", "addon_id": "advanced_analytics"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00001"},
		ExpectedStatus:  200,
		ExpectedContent: []string{`"status"`, "addon_added", `"addon_id"`, "advanced_analytics"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKeyWithAddons(t.(*testing.T), app, "OZ-ADDON-TEST-01", "plus", "[]")
			seedAdminTenant(t.(*testing.T), app, "addonadmin@test.com", "addonadmin00001")
		},
	})
}

func TestAddLicenseAddon_AlreadyActive(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-ADDON-DUP-01", "addon_id": "priority_support"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00002"},
		ExpectedStatus:  409,
		ExpectedContent: []string{`"error"`, "already active"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKeyWithAddons(t.(*testing.T), app, "OZ-ADDON-DUP-01", "pro", `["priority_support"]`)
			seedAdminTenant(t.(*testing.T), app, "addonadmin2@test.com", "addonadmin00002")
		},
	})
}

func TestAddLicenseAddon_KeyNotFound(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-NONEXIST-KEY", "addon_id": "advanced_analytics"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00003"},
		ExpectedStatus:  404,
		ExpectedContent: []string{`"error"`, "license key not found"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedAdminTenant(t.(*testing.T), app, "addonadmin3@test.com", "addonadmin00003")
		},
	})
}

func TestAddLicenseAddon_MissingFields(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-SOMETHING"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00004"},
		ExpectedStatus:  400,
		ExpectedContent: []string{`"error"`, "addon_id are required"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedAdminTenant(t.(*testing.T), app, "addonadmin4@test.com", "addonadmin00004")
		},
	})
}

// ── Remove License Addon Tests ────────────────────────────────

func TestRemoveLicenseAddon_Success(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "DELETE",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-ADDON-RM-01", "addon_id": "advanced_analytics"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00005"},
		ExpectedStatus:  200,
		ExpectedContent: []string{`"status"`, "addon_removed"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKeyWithAddons(t.(*testing.T), app, "OZ-ADDON-RM-01", "plus", `["advanced_analytics"]`)
			seedAdminTenant(t.(*testing.T), app, "addonadmin5@test.com", "addonadmin00005")
		},
	})
}

func TestRemoveLicenseAddon_NotFound(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "DELETE",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-ADDON-RM-02", "addon_id": "nonexistent"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00006"},
		ExpectedStatus:  404,
		ExpectedContent: []string{`"error"`, "addon not found"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKeyWithAddons(t.(*testing.T), app, "OZ-ADDON-RM-02", "plus", `["priority_support"]`)
			seedAdminTenant(t.(*testing.T), app, "addonadmin6@test.com", "addonadmin00006")
		},
	})
}

// ── List License Addons Tests ─────────────────────────────────

func TestListLicenseAddons_Success(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "GET",
		URL:             "/api/v1/admin/license-addons?key=OZ-ADDON-LIST-01",
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00007"},
		ExpectedStatus:  200,
		ExpectedContent: []string{`"license_key"`, "OZ-ADDON-LIST-01", `"addons"`},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKeyWithAddons(t.(*testing.T), app, "OZ-ADDON-LIST-01", "pro", `["advanced_analytics", "priority_support"]`)
			seedAdminTenant(t.(*testing.T), app, "addonadmin7@test.com", "addonadmin00007")
		},
	})
}

func TestListLicenseAddons_MissingKey(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "GET",
		URL:             "/api/v1/admin/license-addons",
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00008"},
		ExpectedStatus:  400,
		ExpectedContent: []string{`"error"`, "key query parameter is required"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedAdminTenant(t.(*testing.T), app, "addonadmin8@test.com", "addonadmin00008")
		},
	})
}

func TestListLicenseAddons_KeyNotFound(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "GET",
		URL:             "/api/v1/admin/license-addons?key=OZ-NONEXISTENT",
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00009"},
		ExpectedStatus:  404,
		ExpectedContent: []string{`"error"`, "license key not found"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedAdminTenant(t.(*testing.T), app, "addonadmin9@test.com", "addonadmin00009")
		},
	})
}

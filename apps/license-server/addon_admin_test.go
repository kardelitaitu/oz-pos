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

// ── Edge case tests ──────────────────────────────────────────

func TestAddLicenseAddon_CaseInsensitiveAddonId(t *testing.T) {
	// Addon ID should be lowercased by the handler
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-ADDON-CASE-01", "addon_id": "Advanced_Analytics"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00010"},
		ExpectedStatus:  200,
		ExpectedContent: []string{"addon_added", "advanced_analytics"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKeyWithAddons(t.(*testing.T), app, "OZ-ADDON-CASE-01", "plus", "[]")
			seedAdminTenant(t.(*testing.T), app, "addonadmin10@test.com", "addonadmin00010")
		},
	})
}

func TestRemoveLicenseAddon_CaseInsensitive(t *testing.T) {
	// Remove with different casing should still work
	runScenario(t, &tests.ApiScenario{
		Method:          "DELETE",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-ADDON-RMC-01", "addon_id": "PRIORITY_SUPPORT"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00011"},
		ExpectedStatus:  200,
		ExpectedContent: []string{"addon_removed"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKeyWithAddons(t.(*testing.T), app, "OZ-ADDON-RMC-01", "pro", `["priority_support"]`)
			seedAdminTenant(t.(*testing.T), app, "addonadmin11@test.com", "addonadmin00011")
		},
	})
}

func TestRemoveLicenseAddon_MissingFields(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "DELETE",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-SOMETHING"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00012"},
		ExpectedStatus:  400,
		ExpectedContent: []string{"addon_id are required"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedAdminTenant(t.(*testing.T), app, "addonadmin12@test.com", "addonadmin00012")
		},
	})
}

func TestListLicenseAddons_EmptyAddons(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "GET",
		URL:             "/api/v1/admin/license-addons?key=OZ-ADDON-EMPTY",
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00013"},
		ExpectedStatus:  200,
		ExpectedContent: []string{"license_key", "OZ-ADDON-EMPTY", "addons"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKeyWithAddons(t.(*testing.T), app, "OZ-ADDON-EMPTY", "free", "[]")
			seedAdminTenant(t.(*testing.T), app, "addonadmin13@test.com", "addonadmin00013")
		},
	})
}

func TestParseAddonsFromRecord_Empty(t *testing.T) {
	app, se := setupDirectApp(t)
	defer app.Cleanup()
	_ = se // unused but ensures routes are registered

	col, err := app.FindCollectionByNameOrId("license_keys")
	if err != nil {
		t.Fatalf("license_keys collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("addons", "")
	result := parseAddonsFromRecord(rec)
	if len(result) != 0 {
		t.Errorf("expected empty addons, got %v", result)
	}
}

func TestParseAddonsFromRecord_ValidJSON(t *testing.T) {
	app, se := setupDirectApp(t)
	defer app.Cleanup()
	_ = se

	col, err := app.FindCollectionByNameOrId("license_keys")
	if err != nil {
		t.Fatalf("license_keys collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("addons", `["a","b"]`)
	result := parseAddonsFromRecord(rec)
	if len(result) != 2 {
		t.Errorf("expected 2 addons, got %d", len(result))
	}
	if result[0] != "a" || result[1] != "b" {
		t.Errorf("expected [a,b], got %v", result)
	}
}

func TestParseAddonsFromRecord_InvalidJSON(t *testing.T) {
	app, se := setupDirectApp(t)
	defer app.Cleanup()
	_ = se

	col, err := app.FindCollectionByNameOrId("license_keys")
	if err != nil {
		t.Fatalf("license_keys collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("addons", "not json")
	result := parseAddonsFromRecord(rec)
	if len(result) != 0 {
		t.Errorf("expected empty addons for invalid JSON, got %v", result)
	}
}

func TestListEnterpriseCodes_Unauthorized(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "GET",
		URL:             "/api/v1/admin/enterprise-codes",
		ExpectedStatus:  401,
		ExpectedContent: []string{"header required"},
	})
}

func TestEnterpriseTrial_ApprovalCodeLength(t *testing.T) {
	// Approval code validation: too short should fail
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/enterprise-trial",
		Body:            strings.NewReader(`{"approval_code": "AB", "email": "a@b.com"}`),
		ExpectedStatus:  400,
		ExpectedContent: []string{"invalid approval_code format"},
	})
}

func TestEnterpriseTrial_CodeNotRedeemedYet(t *testing.T) {
	// Unused code should not be blocked
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/enterprise-trial",
		Body:            strings.NewReader(`{"approval_code": "ENT-UNUSED-01", "email": "unused@test.com"}`),
		ExpectedStatus:  200,
		ExpectedContent: []string{"trial_key_minted"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedApprovalCode(t.(*testing.T), app, "ENT-UNUSED-01", "unused@test.com", "unused")
		},
	})
}

func TestEnterpriseTrial_LicenseKeyHasCorrectTier(t *testing.T) {
	// Verify the minted key has is_trial=true and tier_key=enterprise
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/enterprise-trial",
		Body:            strings.NewReader(`{"approval_code": "ENT-TIER-TEST-01", "email": "tier@test.com"}`),
		ExpectedStatus:  200,
		ExpectedContent: []string{`"tier_key"`, "enterprise", `"days"`, "30"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedApprovalCode(t.(*testing.T), app, "ENT-TIER-TEST-01", "tier@test.com", "unused")
		},
	})
}

func TestAddLicenseAddon_AddsToList(t *testing.T) {
	// Add a second addon to a key that already has one
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-ADDON-MULTI", "addon_id": "extra_storage"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00020"},
		ExpectedStatus:  200,
		ExpectedContent: []string{"addon_added", "extra_storage"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKeyWithAddons(t.(*testing.T), app, "OZ-ADDON-MULTI", "plus", `["advanced_analytics"]`)
			seedAdminTenant(t.(*testing.T), app, "addonadmin20@test.com", "addonadmin00020")
		},
	})
}

func TestRemoveLicenseAddon_FromMultiple(t *testing.T) {
	// Remove one addon from a key with multiple
	runScenario(t, &tests.ApiScenario{
		Method:          "DELETE",
		URL:             "/api/v1/admin/license-addons",
		Body:            strings.NewReader(`{"license_key": "OZ-ADDON-RMM", "addon_id": "priority_support"}`),
		Headers:         map[string]string{"Authorization": "Bearer addonadmin00021"},
		ExpectedStatus:  200,
		ExpectedContent: []string{"addon_removed", "priority_support"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedLicenseKeyWithAddons(t.(*testing.T), app, "OZ-ADDON-RMM", "pro", `["advanced_analytics","priority_support","extra_storage"]`)
			seedAdminTenant(t.(*testing.T), app, "addonadmin21@test.com", "addonadmin00021")
		},
	})
}

func TestGenerateCode_InvalidApiKey(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/enterprise-codes",
		Headers:         map[string]string{"Authorization": "Bearer notarealapikey0000"},
		ExpectedStatus:  401,
		ExpectedContent: []string{"invalid api_key"},
	})
}

func TestGenerateCode_ShortCustomCode(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/enterprise-codes",
		Body:            strings.NewReader(`{"custom_code": "SHORT"}`),
		Headers:         map[string]string{"Authorization": "Bearer adminapikey00012"},
		ExpectedStatus:  400,
		ExpectedContent: []string{"custom_code must be at least 8 characters"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedAdminTenant(t.(*testing.T), app, "adminshort@test.com", "adminapikey00012")
		},
	})
}

func TestGenerateCode_EmptyBody(t *testing.T) {
	// Empty body should generate a random code
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/enterprise-codes",
		Headers:         map[string]string{"Authorization": "Bearer adminapikey00013"},
		ExpectedStatus:  200,
		ExpectedContent: []string{"status", "unused"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedAdminTenant(t.(*testing.T), app, "adminempty@test.com", "adminapikey00013")
		},
	})
}

func TestRemoveLicenseAddon_Unauthorized(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "DELETE",
		URL:             "/api/v1/admin/license-addons",
		ExpectedStatus:  401,
		ExpectedContent: []string{"header required"},
	})
}

func TestListLicenseAddons_Unauthorized(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "GET",
		URL:             "/api/v1/admin/license-addons?key=OZ-ANYTHING",
		ExpectedStatus:  401,
		ExpectedContent: []string{"header required"},
	})
}

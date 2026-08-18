package main

import (
	"strings"
	"testing"

	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
)

// ── Seed helpers ────────────────────────────────────────────────

func seedApprovalCode(t *testing.T, app *tests.TestApp, code, email, status string) {
	t.Helper()
	coll, err := app.FindCollectionByNameOrId("enterprise_approvals")
	if err != nil {
		t.Fatalf("enterprise_approvals collection not found: %v", err)
	}
	rec := core.NewRecord(coll)
	rec.Set("code", code)
	rec.Set("email", email)
	rec.Set("status", status)
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed approval code %q: %v", code, err)
	}
}

func seedAdminTenant(t *testing.T, app *tests.TestApp, email, apiKey string) {
	t.Helper()
	tenantColl, _ := app.FindCollectionByNameOrId("tenants")
	tenant := core.NewRecord(tenantColl)
	tenant.Set("email", email)
	tenant.Set("phone", "-")
	tenant.Set("status", "active")
	hash, lookup, err := hashAPIKey(apiKey)
	if err != nil {
		t.Fatalf("failed to hash api_key: %v", err)
	}
	tenant.Set("api_key", hash)
	tenant.Set("api_key_lookup", lookup)
	if err := app.Save(tenant); err != nil {
		t.Fatalf("failed to seed admin tenant %q: %v", email, err)
	}
}

// ── Enterprise Trial Handler Tests ──────────────────────────────

func TestEnterpriseTrial_MissingApprovalCode(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/enterprise-trial",
		Body:            strings.NewReader(`{"email": "test@example.com"}`),
		ExpectedStatus:  400,
		ExpectedContent: []string{`"error"`, "approval_code is required"},
	})
}

func TestEnterpriseTrial_MissingEmail(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/enterprise-trial",
		Body:            strings.NewReader(`{"approval_code": "ENT-TESTCODE123"}`),
		ExpectedStatus:  400,
		ExpectedContent: []string{`"error"`, "email is required"},
	})
}

func TestEnterpriseTrial_InvalidApprovalCode(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/enterprise-trial",
		Body:            strings.NewReader(`{"approval_code": "ENT-INVALIDCODE", "email": "test@example.com"}`),
		ExpectedStatus:  403,
		ExpectedContent: []string{`"error"`, "invalid approval code"},
	})
}

func TestEnterpriseTrial_CodeAlreadyRedeemed(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/enterprise-trial",
		Body:            strings.NewReader(`{"approval_code": "ENT-REDEEMED-01", "email": "test@example.com"}`),
		ExpectedStatus:  409,
		ExpectedContent: []string{`"error"`, "already been redeemed"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedApprovalCode(t.(*testing.T), app, "ENT-REDEEMED-01", "used@example.com", "redeemed")
		},
	})
}

func TestEnterpriseTrial_Success_NewTenant(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/enterprise-trial",
		Body:            strings.NewReader(`{"approval_code": "ENT-VALIDCODE-01", "email": "enterprise@test.com"}`),
		ExpectedStatus:  200,
		ExpectedContent: []string{`"status"`, "trial_key_minted", `"tier_key"`, "enterprise"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedApprovalCode(t.(*testing.T), app, "ENT-VALIDCODE-01", "enterprise@test.com", "unused")
		},
	})
}

func TestEnterpriseTrial_Success_ExistingTenant(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/license/enterprise-trial",
		Body:            strings.NewReader(`{"approval_code": "ENT-EXISTING-01", "email": "existing@test.com"}`),
		ExpectedStatus:  200,
		ExpectedContent: []string{`"status"`, "trial_key_minted"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedApprovalCode(t.(*testing.T), app, "ENT-EXISTING-01", "existing@test.com", "unused")
			seedTenant(t.(*testing.T), app, "existten0000001", "dummyapikey00001", "active")
		},
	})
}

// ── Admin Code Generation Tests ────────────────────────────────

func TestGenerateCode_Unauthorized(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/enterprise-codes",
		ExpectedStatus:  401,
		ExpectedContent: []string{`"error"`, "header required"},
	})
}

func TestGenerateCode_Success(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/enterprise-codes",
		Body:            strings.NewReader(`{"email": "prospect@corp.com", "prospect_name": "Acme Corp"}`),
		Headers:         map[string]string{"Authorization": "Bearer adminapikey00001"},
		ExpectedStatus:  200,
		ExpectedContent: []string{`"status"`, "unused", `"email"`, "prospect@corp.com"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedAdminTenant(t.(*testing.T), app, "admin@test.com", "adminapikey00001")
		},
	})
}

func TestGenerateCode_CustomCode(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/enterprise-codes",
		Body:            strings.NewReader(`{"custom_code": "ENT-MYCUSTOM-CODE1"}`),
		Headers:         map[string]string{"Authorization": "Bearer adminapikey00002"},
		ExpectedStatus:  200,
		ExpectedContent: []string{`"code"`, "ENT-MYCUSTOM-CODE1"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedAdminTenant(t.(*testing.T), app, "admin2@test.com", "adminapikey00002")
		},
	})
}

func TestGenerateCode_DuplicateCode(t *testing.T) {
	runScenario(t, &tests.ApiScenario{
		Method:          "POST",
		URL:             "/api/v1/admin/enterprise-codes",
		Body:            strings.NewReader(`{"custom_code": "ENT-DUPLICATE-01"}`),
		Headers:         map[string]string{"Authorization": "Bearer adminapikey00003"},
		ExpectedStatus:  409,
		ExpectedContent: []string{`"error"`, "code already exists"},
		BeforeTestFunc: func(t testing.TB, app *tests.TestApp, e *core.ServeEvent) {
			seedApprovalCode(t.(*testing.T), app, "ENT-DUPLICATE-01", "first@test.com", "unused")
			seedAdminTenant(t.(*testing.T), app, "admin3@test.com", "adminapikey00003")
		},
	})
}

// ── Trial Segmentation Tests ───────────────────────────────────

func TestTrialSegmentation_EnterpriseSelfServe(t *testing.T) {
	tier, days := trialSegmentation("enterprise_self_serve")
	if tier != "enterprise" {
		t.Errorf("expected tier 'enterprise', got %q", tier)
	}
	if days != 30 {
		t.Errorf("expected 30 days, got %d", days)
	}
}

func TestTrialSegmentation_BackwardCompatibility(t *testing.T) {
	tests := []struct {
		vertical string
		tier     string
		days     int
	}{
		{"", "plus", 14},
		{"restaurant", "pro", 14},
		{"cafe", "pro", 14},
		{"enterprise_referral", "pro", 30},
		{"enterprise_self_serve", "enterprise", 30},
		{"unknown", "plus", 14},
	}
	for _, tt := range tests {
		tier, days := trialSegmentation(tt.vertical)
		if tier != tt.tier || days != tt.days {
			t.Errorf("trialSegmentation(%q) = (%q, %d), want (%q, %d)",
				tt.vertical, tier, days, tt.tier, tt.days)
		}
	}
}

func TestGenerateEnterpriseTrialKey_Prefix(t *testing.T) {
	key := generateEnterpriseTrialKey()
	if !strings.HasPrefix(key, "OZ-ENTR-") {
		t.Errorf("expected key prefix 'OZ-ENTR-', got %q", key)
	}
	if len(key) < 16 {
		t.Errorf("key too short: %q (len=%d)", key, len(key))
	}
}

func TestGenerateApprovalCode_Prefix(t *testing.T) {
	code := generateApprovalCode()
	if !strings.HasPrefix(code, "ENT-") {
		t.Errorf("expected code prefix 'ENT-', got %q", code)
	}
	// ENT- + 4 bytes hex-encoded = ENT-XXXXXXXX (12 chars)
	if len(code) != 12 {
		t.Errorf("expected code length 12, got %d: %q", len(code), code)
	}
}

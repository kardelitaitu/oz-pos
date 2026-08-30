package main

// Direct unit tests for the /me summary functions (tenantSummary,
// licenseSummary, subscriptionSummary) in web_otp.go. These pin every
// branch the frontend dashboard depends on — the HTTP-level /me tests cover
// the happy paths, but the fallback branches (no activated key, subscription
// without paddle_sub_id, subscription-linked key missing) were only reached
// indirectly. These test the contract at the function boundary.

import (
	"testing"

	"github.com/pocketbase/pocketbase/tests"
)

// summaryApp builds a test app + serve event like setupDirectApp (collections
// registered), returning just the app for direct record seeding.
func summaryApp(t *testing.T) *tests.TestApp {
	t.Helper()
	app, _ := setupDirectApp(t)
	return app
}

func TestTenantSummary_FieldMapping(t *testing.T) {
	app := summaryApp(t)
	defer app.Cleanup()

	const tenantID = "summarytnt00001"
	seedTenant(t, app, tenantID, "summaryapikey01", "active")

	tenant, err := app.FindFirstRecordByData("tenants", "id", tenantID)
	if err != nil || tenant == nil {
		t.Fatalf("seeded tenant not found: %v", err)
	}

	got := tenantSummary(tenant)
	want := map[string]string{
		"id":            tenantID,
		"email":         "summarytnt00001@example.com", // seedTenant lowercases {id}@example.com
		"emailVerified": "false",                       // seedTenant does not set it → false
		"status":        "active",
	}
	for field, wantVal := range want {
		if field == "emailVerified" {
			if b, ok := got["emailVerified"].(bool); !ok || (b != (wantVal == "true")) {
				t.Errorf("tenantSummary[%q] = %v (%T), want bool %v", field, got["emailVerified"], got["emailVerified"], wantVal == "true")
			}
			continue
		}
		if s, ok := got[field].(string); !ok || s != wantVal {
			t.Errorf("tenantSummary[%q] = %v, want %q", field, got[field], wantVal)
		}
	}
}

// TestLicenseSummary_ActivatedKey is the primary path: the tenant activated a
// key (activated_by set) → the key block is returned.
func TestLicenseSummary_ActivatedKey(t *testing.T) {
	app := summaryApp(t)
	defer app.Cleanup()

	const tenantID = "licsactenant001"
	seedTenant(t, app, tenantID, "licsactenant001", "active")
	seedLicenseKey(t, app, "OZ-SUMMARY-ACT-01", "pro", "activated", "2027-01-01T00:00:00Z")
	keys, err := app.FindRecordsByFilter("license_keys", "key = 'OZ-SUMMARY-ACT-01'", "", 1, 0, nil)
	if err != nil || len(keys) == 0 {
		t.Fatalf("seeded key not found: %v", err)
	}
	keys[0].Set("activated_by", []string{tenantID})
	if err := app.Save(keys[0]); err != nil {
		t.Fatalf("failed to bind activated_by: %v", err)
	}

	got := licenseSummary(app, tenantID)
	m, ok := got.(map[string]any)
	if !ok {
		t.Fatalf("expected a license map, got %T", got)
	}
	if m["key"] != "OZ-SUMMARY-ACT-01" {
		t.Errorf("key = %v, want OZ-SUMMARY-ACT-01", m["key"])
	}
	if m["tierKey"] != "pro" {
		t.Errorf("tierKey = %v, want pro", m["tierKey"])
	}
	if m["status"] != "activated" {
		t.Errorf("status = %v, want activated", m["status"])
	}
	// expiresAt must be RFC3339 UTC (contract), not the raw storage form.
	if m["expiresAt"] != "2027-01-01T00:00:00Z" {
		t.Errorf("expiresAt = %v, want RFC3339 2027-01-01T00:00:00Z", m["expiresAt"])
	}
}

// TestLicenseSummary_NoActivatedKeyNoSubscription: nothing to fall back to.
func TestLicenseSummary_NoActivatedKeyNoSubscription(t *testing.T) {
	app := summaryApp(t)
	defer app.Cleanup()

	const tenantID = "licsnone0000001"
	seedTenant(t, app, tenantID, "licsnone0000001", "active")
	// No license key, no subscription.

	if got := licenseSummary(app, tenantID); got != nil {
		t.Errorf("expected nil license, got %v", got)
	}
}

// TestLicenseSummary_SubscriptionWithoutPaddleSubId: the fallback path finds
// a subscription but it has no paddle_sub_id → no key to show → nil.
func TestLicenseSummary_SubscriptionWithoutPaddleSubId(t *testing.T) {
	app := summaryApp(t)
	defer app.Cleanup()

	const tenantID = "licssubno000001"
	seedTenant(t, app, tenantID, "licssubno000001", "active")
	seedSubscription(t, app, tenantID, "pro", "active")
	// No paddle_sub_id on the subscription (unusual, but possible).

	if got := licenseSummary(app, tenantID); got != nil {
		t.Errorf("expected nil license (no paddle_sub_id), got %v", got)
	}
}

// TestLicenseSummary_SubscriptionLinkedKey: the fallback path finds the key
// via the subscription's paddle_sub_id — the "unused key" case the webhook
// leaves behind (covered at HTTP level too, pinned here directly).
func TestLicenseSummary_SubscriptionLinkedKey(t *testing.T) {
	app := summaryApp(t)
	defer app.Cleanup()

	const tenantID = "licslinktena001"
	seedTenant(t, app, tenantID, "licslinktena001", "active")
	seedSubscription(t, app, tenantID, "pro", "active")
	subs, err := app.FindRecordsByFilter("subscriptions", "tenant_id = {:t}", "", 1, 0, map[string]any{"t": tenantID})
	if err != nil || len(subs) == 0 {
		t.Fatalf("seeded subscription not found: %v", err)
	}
	subs[0].Set("paddle_sub_id", "sub_summary_link")
	if err := app.Save(subs[0]); err != nil {
		t.Fatalf("failed to set paddle_sub_id: %v", err)
	}
	seedLicenseKey(t, app, "OZ-SUMMARY-LINK-01", "pro", "unused", "2099-12-31 23:59:59.000Z")
	keys, err := app.FindRecordsByFilter("license_keys", "key = 'OZ-SUMMARY-LINK-01'", "", 1, 0, nil)
	if err != nil || len(keys) == 0 {
		t.Fatalf("seeded key not found: %v", err)
	}
	keys[0].Set("paddle_sub_id", "sub_summary_link")
	if err := app.Save(keys[0]); err != nil {
		t.Fatalf("failed to set key paddle_sub_id: %v", err)
	}

	got := licenseSummary(app, tenantID)
	m, ok := got.(map[string]any)
	if !ok {
		t.Fatalf("expected a license map, got %T", got)
	}
	if m["key"] != "OZ-SUMMARY-LINK-01" {
		t.Errorf("key = %v, want OZ-SUMMARY-LINK-01", m["key"])
	}
	if m["status"] != "unused" {
		t.Errorf("status = %v, want unused", m["status"])
	}
}

// TestLicenseSummary_SubscriptionLinkedKeyMissing: the subscription has a
// paddle_sub_id but no key carries it → nil (the webhook may have raced).
func TestLicenseSummary_SubscriptionLinkedKeyMissing(t *testing.T) {
	app := summaryApp(t)
	defer app.Cleanup()

	const tenantID = "licsmissg000001"
	seedTenant(t, app, tenantID, "licsmissg000001", "active")
	seedSubscription(t, app, tenantID, "pro", "active")
	subs, err := app.FindRecordsByFilter("subscriptions", "tenant_id = {:t}", "", 1, 0, map[string]any{"t": tenantID})
	if err != nil || len(subs) == 0 {
		t.Fatalf("seeded subscription not found: %v", err)
	}
	subs[0].Set("paddle_sub_id", "sub_summary_missing")
	if err := app.Save(subs[0]); err != nil {
		t.Fatalf("failed to set paddle_sub_id: %v", err)
	}
	// No license key carries sub_summary_missing.

	if got := licenseSummary(app, tenantID); got != nil {
		t.Errorf("expected nil license (no linked key), got %v", got)
	}
}

func TestSubscriptionSummary_NoSubscription(t *testing.T) {
	app := summaryApp(t)
	defer app.Cleanup()

	const tenantID = "subsnonetenant1"
	seedTenant(t, app, tenantID, "subsnonetenant1", "active")

	if got := subscriptionSummary(app, tenantID); got != nil {
		t.Errorf("expected nil subscription, got %v", got)
	}
}

func TestSubscriptionSummary_FullShape(t *testing.T) {
	app := summaryApp(t)
	defer app.Cleanup()

	const tenantID = "subsfull0000001"
	seedTenant(t, app, tenantID, "subsfull0000001", "active")
	seedSubscription(t, app, tenantID, "premium", "grace_period")
	subs, err := app.FindRecordsByFilter("subscriptions", "tenant_id = {:t}", "", 1, 0, map[string]any{"t": tenantID})
	if err != nil || len(subs) == 0 {
		t.Fatalf("seeded subscription not found: %v", err)
	}
	subs[0].Set("bundle_id", "restaurant_starter")
	subs[0].Set("paddle_sub_id", "sub_summary_full")
	if err := app.Save(subs[0]); err != nil {
		t.Fatalf("failed to set sub fields: %v", err)
	}

	got := subscriptionSummary(app, tenantID)
	m, ok := got.(map[string]any)
	if !ok {
		t.Fatalf("expected a subscription map, got %T", got)
	}
	if m["tierKey"] != "premium" {
		t.Errorf("tierKey = %v, want premium", m["tierKey"])
	}
	if m["status"] != "grace_period" {
		t.Errorf("status = %v, want grace_period", m["status"])
	}
	if m["bundleId"] != "restaurant_starter" {
		t.Errorf("bundleId = %v, want restaurant_starter", m["bundleId"])
	}
	// Dates must be RFC3339 UTC strings.
	for _, f := range []string{"startsAt", "expiresAt", "graceUntil"} {
		s, ok := m[f].(string)
		if !ok {
			t.Errorf("%s = %v (%T), want a string", f, m[f], m[f])
			continue
		}
		if s == "" {
			t.Errorf("%s should be non-empty", f)
		}
	}
}

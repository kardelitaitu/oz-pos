package main

import (
	"os"
	"strings"
	"testing"
	"time"

	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
	"github.com/pocketbase/pocketbase/tools/types"
)

// newTrialEmailTestApp creates a test app with trial_email_log collection.
func newTrialEmailTestApp(t *testing.T) *tests.TestApp {
	t.Helper()
	app, _ := tests.NewTestApp()
	collections := []string{
		"tenants", "license_keys", "subscriptions",
		"tenant_machines", "trial_registrations", "trial_claims",
		"trial_email_log",
	}
	for _, name := range collections {
		ensureTestCollection(t, app, name)
	}
	return app
}

func ensureTestCollection(t *testing.T, app *tests.TestApp, name string) {
	t.Helper()
	_, err := app.FindCollectionByNameOrId(name)
	if err == nil {
		return // already exists
	}
	switch name {
	case "tenants":
		c := core.NewBaseCollection("tenants")
		c.Fields.Add(
			&core.EmailField{Name: "email", Required: true},
			&core.TextField{Name: "phone"},
			&core.TextField{Name: "api_key", Required: true},
			&core.TextField{Name: "api_key_lookup"},
			&core.SelectField{Name: "status", Required: true, Values: []string{"active", "suspended", "revoked"}},
		)
		c.CreateRule = types.Pointer("")
		c.ListRule = types.Pointer("")
		c.ViewRule = types.Pointer("")
		if err := app.Save(c); err != nil {
			t.Fatalf("create tenants: %v", err)
		}
	case "license_keys":
		c := core.NewBaseCollection("license_keys")
		c.Fields.Add(
			&core.TextField{Name: "key", Required: true},
			&core.SelectField{Name: "tier_key", Required: true, Values: []string{"free", "plus", "pro", "premium", "enterprise"}},
			&core.NumberField{Name: "max_stores"},
			&core.NumberField{Name: "max_pos_instances"},
			&core.TextField{Name: "paddle_sub_id"},
			&core.TextField{Name: "midtrans_sub_id"},
			&core.TextField{Name: "bundle_id"},
			&core.TextField{Name: "payment_provider"},
			&core.BoolField{Name: "is_trial"},
			&core.DateField{Name: "expires_at"},
			&core.DateField{Name: "starts_at"},
			&core.TextField{Name: "tenant_id"},
		)
		c.CreateRule = types.Pointer("")
		c.ListRule = types.Pointer("")
		c.ViewRule = types.Pointer("")
		if err := app.Save(c); err != nil {
			t.Fatalf("create license_keys: %v", err)
		}
	case "subscriptions":
		c := core.NewBaseCollection("subscriptions")
		c.Fields.Add(
			&core.TextField{Name: "tenant_id", Required: true},
			&core.SelectField{Name: "tier_key", Required: true, Values: []string{"free", "plus", "pro", "premium", "enterprise"}},
			&core.SelectField{Name: "status", Required: true, Values: []string{"active", "expired", "grace_period", "revoked", "paused"}},
			&core.DateField{Name: "starts_at", Required: true},
			&core.DateField{Name: "expires_at", Required: true},
			&core.DateField{Name: "grace_until"},
			&core.TextField{Name: "signed_payload", Required: true},
			&core.TextField{Name: "signature", Required: true},
			&core.TextField{Name: "paddle_sub_id"},
			&core.TextField{Name: "midtrans_sub_id"},
			&core.TextField{Name: "payment_provider"},
			&core.TextField{Name: "bundle_id"},
			&core.BoolField{Name: "is_trial"},
			&core.NumberField{Name: "max_stores"},
			&core.NumberField{Name: "max_pos_instances"},
		)
		c.CreateRule = types.Pointer("")
		c.ListRule = types.Pointer("")
		c.ViewRule = types.Pointer("")
		if err := app.Save(c); err != nil {
			t.Fatalf("create subscriptions: %v", err)
		}
	case "tenant_machines":
		c := core.NewBaseCollection("tenant_machines")
		c.Fields.Add(
			&core.TextField{Name: "tenant_id", Required: true},
			&core.TextField{Name: "machine_id", Required: true},
		)
		c.CreateRule = types.Pointer("")
		c.ListRule = types.Pointer("")
		c.ViewRule = types.Pointer("")
		if err := app.Save(c); err != nil {
			t.Fatalf("create tenant_machines: %v", err)
		}
	case "trial_registrations":
		c := core.NewBaseCollection("trial_registrations")
		c.Fields.Add(
			&core.TextField{Name: "claim_hash", Required: true},
			&core.TextField{Name: "tenant_id"},
			&core.TextField{Name: "email"},
			&core.NumberField{Name: "claim_count"},
		)
		c.CreateRule = types.Pointer("")
		c.ListRule = types.Pointer("")
		c.ViewRule = types.Pointer("")
		if err := app.Save(c); err != nil {
			t.Fatalf("create trial_registrations: %v", err)
		}
	case "trial_claims":
		c := core.NewBaseCollection("trial_claims")
		c.Fields.Add(
			&core.TextField{Name: "claim_hash", Required: true},
			&core.TextField{Name: "tenant_id"},
			&core.TextField{Name: "email"},
			&core.NumberField{Name: "claim_count"},
		)
		c.CreateRule = types.Pointer("")
		c.ListRule = types.Pointer("")
		c.ViewRule = types.Pointer("")
		if err := app.Save(c); err != nil {
			t.Fatalf("create trial_claims: %v", err)
		}
	case "trial_email_log":
		c := core.NewBaseCollection("trial_email_log")
		c.Fields.Add(
			&core.TextField{Name: "subscription", Required: true, Max: 15},
			&core.NumberField{Name: "day_offset", Required: true},
			&core.DateField{Name: "sent_at", Required: true},
		)
		c.CreateRule = types.Pointer("")
		c.ListRule = types.Pointer("")
		c.ViewRule = types.Pointer("")
		if err := app.Save(c); err != nil {
			t.Fatalf("create trial_email_log: %v", err)
		}
	}
}

// seedTrialTenantAndSub creates a tenant + subscription for testing.
func seedTrialTenantAndSub(t *testing.T, app *tests.TestApp, email, tier string, isTrial bool, startsAt, expiresAt time.Time) string {
	t.Helper()

	// Create tenant.
	tenants, _ := app.FindCollectionByNameOrId("tenants")
	tenant := core.NewRecord(tenants)
	tenant.Set("email", email)
	tenant.Set("api_key", "test-key-"+email)
	tenant.Set("status", "active")
	if err := app.Save(tenant); err != nil {
		t.Fatalf("seed tenant: %v", err)
	}

	// Create subscription.
	subs, _ := app.FindCollectionByNameOrId("subscriptions")
	sub := core.NewRecord(subs)
	sub.Set("tenant_id", tenant.Id)
	sub.Set("tier_key", tier)
	sub.Set("status", "active")
	sub.Set("starts_at", startsAt.Format(time.RFC3339))
	sub.Set("expires_at", expiresAt.Format(time.RFC3339))
	sub.Set("signed_payload", "{}")
	sub.Set("signature", "test-sig")
	sub.Set("is_trial", isTrial)
	if err := app.Save(sub); err != nil {
		t.Fatalf("seed subscription: %v", err)
	}

	return sub.Id
}

func TestBuildTrialEmail(t *testing.T) {
	msg := buildTrialEmail("no-reply@oz-pos.com", "user@example.com", "Test Subject", "Hello body")
	s := string(msg)

	if !strings.Contains(s, "From: OZ-POS <no-reply@oz-pos.com>") {
		t.Error("missing From header")
	}
	if !strings.Contains(s, "To: user@example.com") {
		t.Error("missing To header")
	}
	if !strings.Contains(s, "Subject: Test Subject") {
		t.Error("missing Subject header")
	}
	if !strings.Contains(s, "Hello body") {
		t.Error("missing body")
	}
	if !strings.Contains(s, "Content-Type: text/plain; charset=utf-8") {
		t.Error("missing MIME type")
	}
}

func TestTrialMilestones_AllSegmentsHaveBothDays(t *testing.T) {
	seen := map[string]map[int]bool{
		"plus": {7: false, 14: false},
		"pro":  {7: false, 14: false},
	}
	for _, m := range trialMilestones {
		if _, ok := seen[m.Segment]; !ok {
			t.Errorf("unexpected segment %q", m.Segment)
			continue
		}
		seen[m.Segment][m.DayOffset] = true
	}
	for seg, days := range seen {
		for day, found := range days {
			if !found {
				t.Errorf("missing milestone for segment=%s day=%d", seg, day)
			}
		}
	}
}

func TestDetectLocale_PhonePrefix(t *testing.T) {
	collection := core.NewBaseCollection("tenants")

	tenant := core.NewRecord(collection)
	tenant.Set("phone", "+628123456789")
	if got := detectLocale(tenant); got != "id" {
		t.Errorf("expected id for +62 phone, got %q", got)
	}

	tenant2 := core.NewRecord(collection)
	tenant2.Set("phone", "+14155551234")
	if got := detectLocale(tenant2); got != "en" {
		t.Errorf("expected en for +1 phone, got %q", got)
	}

	tenant3 := core.NewRecord(collection)
	tenant3.Set("phone", "")
	if got := detectLocale(tenant3); got != "en" {
		t.Errorf("expected en for empty phone, got %q", got)
	}
}

func TestSendTrialEmail_MissingSMTPHost(t *testing.T) {
	os.Unsetenv("OZ_SMTP_HOST")
	err := sendTrialEmail("to@example.com", "Subject", "Body")
	if err == nil {
		t.Error("expected error when OZ_SMTP_HOST is unset")
	}
	if !strings.Contains(err.Error(), "OZ_SMTP_HOST is not configured") {
		t.Errorf("unexpected error: %v", err)
	}
}

func TestEnsureTrialEmailLogCollection(t *testing.T) {
	app, _ := tests.NewTestApp()

	// First call should create it.
	if err := ensureTrialEmailLogCollection(app); err != nil {
		t.Fatalf("first call failed: %v", err)
	}

	// Second call should be idempotent.
	if err := ensureTrialEmailLogCollection(app); err != nil {
		t.Fatalf("second call failed: %v", err)
	}

	// Verify collection exists.
	collection, err := app.FindCollectionByNameOrId("trial_email_log")
	if err != nil {
		t.Fatalf("collection not found: %v", err)
	}
	if collection.Name != "trial_email_log" {
		t.Errorf("wrong collection name: %q", collection.Name)
	}
}

func TestEmailAlreadySent(t *testing.T) {
	app, _ := tests.NewTestApp()
	ensureTestCollection(t, app, "trial_email_log")

	// Create a log entry.
	collection, _ := app.FindCollectionByNameOrId("trial_email_log")
	record := core.NewRecord(collection)
	record.Set("subscription", "sub123")
	record.Set("day_offset", 7)
	record.Set("sent_at", time.Now().UTC().Format(time.RFC3339))
	if err := app.Save(record); err != nil {
		t.Fatalf("save log: %v", err)
	}

	// Should return true for existing entry.
	if !emailAlreadySent(app, "sub123", 7) {
		t.Error("expected emailAlreadySent to return true for existing entry")
	}

	// Should return false for different day.
	if emailAlreadySent(app, "sub123", 14) {
		t.Error("expected emailAlreadySent to return false for different day")
	}

	// Should return false for different subscription.
	if emailAlreadySent(app, "sub456", 7) {
		t.Error("expected emailAlreadySent to return false for different subscription")
	}
}

func TestGetTrialUsageSummary(t *testing.T) {
	collection := core.NewBaseCollection("subscriptions")
	sub := core.NewRecord(collection)
	salesCount, revenue := getTrialUsageSummary(sub)
	if salesCount == "" || revenue == "" {
		t.Error("expected non-empty usage summary")
	}
}

func TestWinBackMilestones_HaveBothDays(t *testing.T) {
	seen := map[int]bool{7: false, 30: false}
	for _, m := range winBackMilestones {
		seen[m.DayOffset] = true
	}
	for day, found := range seen {
		if !found {
			t.Errorf("missing win-back milestone for day %d", day)
		}
	}
}

func TestHashLogKey_Deterministic(t *testing.T) {
	key1 := hashLogKey("winback_7d")
	key2 := hashLogKey("winback_7d")
	if key1 != key2 {
		t.Errorf("hashLogKey not deterministic: %d != %d", key1, key2)
	}
	if key1 >= 0 {
		t.Errorf("expected negative hash, got %d", key1)
	}
}

func TestHashLogKey_DifferentKeysDifferentHashes(t *testing.T) {
	h1 := hashLogKey("winback_7d")
	h2 := hashLogKey("winback_30d")
	if h1 == h2 {
		t.Errorf("different keys produced same hash: %d", h1)
	}
}

package main

// Regression tests for the LSE-5 audit fix: in-code collection migrations
// must create SUPERUSER-ONLY API rules (nil), never empty-string rules.
// PocketBase treats a nil rule as superuser-only and an EMPTY STRING rule
// as PUBLIC (everybody, including unauthenticated guests) on the generic
// /api/collections/{name}/records endpoints. Older migrations used
// types.Pointer("") under the mistaken assumption that it meant
// "server-only", which publicly exposed trial_registrations, trial_claims,
// enterprise_approvals, trial_email_log, and password_rotation_state rows
// on deployments where those migrations created the collections.

import (
	"testing"

	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
	"github.com/pocketbase/pocketbase/tools/types"
)

// newLSE5LegacyCollection mimics a collection created by the pre-LSE-5
// migrations: empty-string rules (PUBLIC) instead of nil (superuser-only).
func newLSE5LegacyCollection(t *testing.T, app *tests.TestApp, name string) {
	t.Helper()
	coll := core.NewBaseCollection(name)
	coll.ListRule = types.Pointer("")
	coll.ViewRule = types.Pointer("")
	coll.CreateRule = types.Pointer("")
	coll.UpdateRule = types.Pointer("")
	coll.DeleteRule = types.Pointer("")
	if err := app.Save(coll); err != nil {
		t.Fatalf("failed to create legacy %s collection: %v", name, err)
	}
}

func assertSuperuserOnlyRules(t *testing.T, app *tests.TestApp, name string) {
	t.Helper()
	coll, err := app.FindCollectionByNameOrId(name)
	if err != nil {
		t.Fatalf("collection %s not found after ensure: %v", name, err)
	}
	for _, rule := range []struct {
		name string
		val  *string
	}{{"listRule", coll.ListRule}, {"viewRule", coll.ViewRule}, {"createRule", coll.CreateRule}, {"updateRule", coll.UpdateRule}, {"deleteRule", coll.DeleteRule}} {
		if rule.val != nil {
			t.Errorf("%s.%s = %q, want nil (superuser-only); empty-string rules are PUBLIC in PocketBase", name, rule.name, *rule.val)
		}
	}
}

// TestLSE5RepairRepairsLegacyPublicRules feeds the ensure functions a
// legacy-migrated collection (empty-string rules) and asserts the repair
// normalizes every rule to superuser-only.
func TestLSE5RepairRepairsLegacyPublicRules(t *testing.T) {
	app, err := tests.NewTestApp()
	if err != nil {
		t.Fatalf("failed to create test app: %v", err)
	}
	defer app.Cleanup()

	cases := []struct {
		name   string
		ensure func(core.App) error
	}{
		{"trial_registrations", ensureTrialRegistrations},
		{"trial_claims", ensureTrialClaims},
		{"enterprise_approvals", ensureEnterpriseApprovals},
		{"trial_email_log", ensureTrialEmailLogCollection},
		{"password_rotation_state", ensurePasswordRotationStateCollection},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			newLSE5LegacyCollection(t, app, tc.name)
			if err := tc.ensure(app); err != nil {
				t.Fatalf("ensure failed: %v", err)
			}
			assertSuperuserOnlyRules(t, app, tc.name)
		})
	}
}

// TestLSE5FreshCreatesAreSuperuserOnly runs each ensure against a fresh app
// (no pre-existing collection) and asserts the created collections are born
// superuser-only.
func TestLSE5FreshCreatesAreSuperuserOnly(t *testing.T) {
	app, err := tests.NewTestApp()
	if err != nil {
		t.Fatalf("failed to create test app: %v", err)
	}
	defer app.Cleanup()

	// The trial collections carry a tenant_id relation, whose Save-time
	// validation needs the target collection to exist (production boots
	// ensureCollections — the embedded pb_schema.json import — first).
	tenants := core.NewBaseCollection("tenants")
	if err := app.Save(tenants); err != nil {
		t.Fatalf("failed to create bare tenants collection: %v", err)
	}

	cases := []struct {
		name   string
		ensure func(core.App) error
	}{
		{"trial_registrations", ensureTrialRegistrations},
		{"trial_claims", ensureTrialClaims},
		{"enterprise_approvals", ensureEnterpriseApprovals},
		{"trial_email_log", ensureTrialEmailLogCollection},
		{"password_rotation_state", ensurePasswordRotationStateCollection},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if err := tc.ensure(app); err != nil {
				t.Fatalf("ensure failed: %v", err)
			}
			assertSuperuserOnlyRules(t, app, tc.name)
			// Idempotency: a second run must be a clean no-op.
			if err := tc.ensure(app); err != nil {
				t.Fatalf("second ensure failed: %v", err)
			}
			assertSuperuserOnlyRules(t, app, tc.name)
		})
	}
}

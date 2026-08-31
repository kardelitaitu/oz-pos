package main

// Robustness tests for the admin add-on endpoints
// (/api/v1/admin/license-addons) — bug hunt round 10.
//
// B44: addon_id had no length cap and the addons array had no size
//      check, while the license_keys.addons column is Max:1024. A single
//      oversized addon_id — or enough small ones — therefore reached
//      Save and failed PocketBase's field validation, surfacing a 500
//      for plainly bad admin input (same class as B42's enterprise
//      fields and B30's unknown tier_key).

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"testing"
)

func TestAddonB44_OversizedAddonIDIsBadRequestNot500(t *testing.T) {
	app, mux := dashboardMux(t)
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")
	seedLicenseKeyWithAddons(t, app, "OZ-B44-LONG-01", "plus", "[]")

	body := `{"license_key":"OZ-B44-LONG-01","addon_id":"` + strings.Repeat("x", 1200) + `"}`
	rec := doJSON(mux, http.MethodPost, "/api/v1/admin/license-addons",
		"Bearer secret-admin-key", body)
	if rec.Code == http.StatusInternalServerError {
		t.Fatalf("got 500 (validation leak), want 400; body=%s", rec.Body.String())
	}
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("got %d, want 400; body=%s", rec.Code, rec.Body.String())
	}
}

func TestAddonB44_AddonListOverflowIsBadRequestNot500(t *testing.T) {
	app, mux := dashboardMux(t)
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	// Seed a record just UNDER the 1024-rune cap so that appending one
	// more addon crosses it — the request itself is well-formed (a
	// 13-char id passes any sane per-id cap), only the resulting column
	// value is too big. 56 ids = 1009 chars; +1 comma +15 quoted = 1025.
	var ids []string
	for i := 0; i < 56; i++ {
		ids = append(ids, fmt.Sprintf("addon_number_%02d", i))
	}
	seedJSON, err := json.Marshal(ids)
	if err != nil {
		t.Fatalf("marshal seed addons: %v", err)
	}
	if len(seedJSON) >= 1024 {
		t.Fatalf("seed array must be valid on its own, got %d chars", len(seedJSON))
	}
	seedLicenseKeyWithAddons(t, app, "OZ-B44-OVER-01", "plus", string(seedJSON))

	rec := doJSON(mux, http.MethodPost, "/api/v1/admin/license-addons",
		"Bearer secret-admin-key",
		`{"license_key":"OZ-B44-OVER-01","addon_id":"extra_addon_x"}`)
	if rec.Code == http.StatusInternalServerError {
		t.Fatalf("got 500 (validation leak), want 400; body=%s", rec.Body.String())
	}
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("got %d, want 400; body=%s", rec.Code, rec.Body.String())
	}
}

// Guard: the normal path must keep working after the caps are added.
func TestAddonB44_NormalAddonPurchaseStillSucceeds(t *testing.T) {
	app, mux := dashboardMux(t)
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")
	seedLicenseKeyWithAddons(t, app, "OZ-B44-OK-0001", "plus", `["priority_support"]`)

	rec := doJSON(mux, http.MethodPost, "/api/v1/admin/license-addons",
		"Bearer secret-admin-key",
		`{"license_key":"OZ-B44-OK-0001","addon_id":"advanced_analytics"}`)
	if rec.Code != http.StatusOK {
		t.Fatalf("got %d, want 200; body=%s", rec.Code, rec.Body.String())
	}
}

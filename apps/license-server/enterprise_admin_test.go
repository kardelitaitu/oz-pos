package main

// Tests for the admin enterprise-approval-code endpoints
// (/api/v1/admin/enterprise-codes) — bug hunt round 9.
//
// B42: the enterprise_approvals schema caps code at Max:64, email at
//      Max:254 and prospect_name at Max:256, but the generate handler
//      only checks len(code) < 8. Oversized fields therefore pass
//      handler validation and fail PocketBase's field validation at
//      Save time — a 500 for plainly bad client input (same class as
//      B30's unknown-tier_key 500).
// B43: the schema carries a created_by field for attribution, but the
//      handler never populates it, so every privileged code is minted
//      anonymously — no audit trail for "who created this".

import (
	"net/http"
	"strings"
	"testing"
)

func TestEnterpriseCodeB42_OversizedFieldsAreBadRequestNot500(t *testing.T) {
	_, mux := dashboardMux(t)
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	cases := []struct {
		name string
		body string
	}{
		{"custom_code over Max:64", `{"custom_code":"` + strings.Repeat("A", 100) + `"}`},
		{"email over Max:254", `{"email":"` + strings.Repeat("a", 300) + `@example.com"}`},
		{"prospect_name over Max:256", `{"prospect_name":"` + strings.Repeat("p", 300) + `"}`},
	}
	for _, tc := range cases {
		rec := doJSON(mux, http.MethodPost, "/api/v1/admin/enterprise-codes",
			"Bearer secret-admin-key", tc.body)
		if rec.Code == http.StatusInternalServerError {
			t.Errorf("%s: got 500 (validation leak), want 400; body=%s",
				tc.name, rec.Body.String())
			continue
		}
		if rec.Code != http.StatusBadRequest {
			t.Errorf("%s: got %d, want 400; body=%s", tc.name, rec.Code, rec.Body.String())
		}
	}
}

func TestEnterpriseCodeB43_RecordsWhoMintedIt(t *testing.T) {
	app, mux := dashboardMux(t)
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	rec := doJSON(mux, http.MethodPost, "/api/v1/admin/enterprise-codes",
		"Bearer secret-admin-key",
		`{"custom_code":"ATTRIB-TEST-1","email":"lead@example.com"}`)
	if rec.Code != http.StatusOK {
		t.Fatalf("generate: got %d body=%s", rec.Code, rec.Body.String())
	}

	row, err := app.FindFirstRecordByData("enterprise_approvals", "code", "ATTRIB-TEST-1")
	if err != nil || row == nil {
		t.Fatalf("minted record not found: %v", err)
	}
	if got := row.GetString("created_by"); got == "" {
		t.Error("created_by is empty — a privileged admin action left no attribution")
	}
}

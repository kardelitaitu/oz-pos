package main

// Concurrency tests for the enterprise self-serve trial redemption
// (POST /api/v1/license/enterprise-trial) — bug hunt round 10.
//
// B45: handleEnterpriseTrial checks code status == "unused" up front and
//      only marks the code "redeemed" at the very END, after creating a
//      tenant and minting a license key. The approval code is a
//      one-time entitlement, so two parallel requests carrying the same
//      code both pass the check and both get an enterprise trial — one
//      minted code grants N tenants.

import (
	"net/http"
	"sync"
	"testing"
)

func redeemWith(t *testing.T, mux http.Handler, code, email string) int {
	t.Helper()
	body := `{"approval_code":"` + code + `","email":"` + email + `"}`
	rec := doJSON(mux, http.MethodPost, "/api/v1/license/enterprise-trial", "", body)
	return rec.Code
}

func TestEnterpriseTrialB45_CodeIsRedeemableOnlyOnceUnderConcurrency(t *testing.T) {
	app, mux := dashboardMux(t)
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")

	// Mint one code through the admin endpoint (also proves the mint and
	// redeem paths agree on the code format).
	mintRec := doJSON(mux, http.MethodPost, "/api/v1/admin/enterprise-codes",
		"Bearer secret-admin-key", `{"custom_code":"RACE-TEST-0001","email":"lead@example.com"}`)
	if mintRec.Code != http.StatusOK {
		t.Fatalf("mint: got %d body=%s", mintRec.Code, mintRec.Body.String())
	}

	const racers = 2
	var wg sync.WaitGroup
	start := make(chan struct{})
	codes := make([]int, racers)
	for i := 0; i < racers; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			<-start // align the racers as tightly as possible
			codes[i] = redeemWith(t, mux, "RACE-TEST-0001", "racer"+string(rune('a'+i))+"@example.com")
		}(i)
	}
	close(start)
	wg.Wait()

	successes := 0
	for _, c := range codes {
		if c == http.StatusOK || c == http.StatusCreated {
			successes++
		}
	}
	if successes > 1 {
		t.Errorf("%d of %d concurrent redemptions of ONE code succeeded — a one-time "+
			"enterprise entitlement granted multiple trials (statuses %v)",
			successes, racers, codes)
	}

	// The code must end up redeemed exactly once.
	row, err := app.FindFirstRecordByData("enterprise_approvals", "code", "RACE-TEST-0001")
	if err != nil || row == nil {
		t.Fatalf("code record missing: %v", err)
	}
	if got := row.GetString("status"); got != "redeemed" {
		t.Errorf("status = %q, want redeemed", got)
	}
}

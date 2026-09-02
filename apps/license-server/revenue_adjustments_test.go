package main

// Revenue adjustment pipeline tests: saveRevenueAdjustment, Midtrans/Paddle
// refund capture, and stats aggregation.

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// ── saveRevenueAdjustment unit tests ──────────────────────────────

func TestRevenueAdjustment_SaveAndDedup(t *testing.T) {
	resetRateLimiters()
	app, _ := dashboardMux(t)
	defer app.Cleanup()
	tenantID, _ := seedDashboardTenant(t, app, "adj-test@test.com")

	saved, err := saveRevenueAdjustment(app, revenueAdjustment{
		Provider:          "paddle",
		EventID:           "evt-adj-001",
		TenantID:          tenantID,
		Kind:              "refund",
		NativeAmountMinor: 999, // 9.99 USD
		NativeCurrency:    "USD",
		Notes:             "test refund",
	})
	if err != nil {
		t.Fatalf("first save: %v", err)
	}
	if !saved {
		t.Fatal("expected first save to return saved=true")
	}

	// Duplicate event_id → skip.
	saved, err = saveRevenueAdjustment(app, revenueAdjustment{
		Provider:          "paddle",
		EventID:           "evt-adj-001",
		NativeAmountMinor: 999,
		NativeCurrency:    "USD",
	})
	if err != nil {
		t.Fatalf("duplicate save: %v", err)
	}
	if saved {
		t.Fatal("expected duplicate save to return saved=false")
	}

	// Check the record.
	rec, err := app.FindFirstRecordByData("revenue_adjustments", "event_id", "evt-adj-001")
	if err != nil {
		t.Fatalf("find record: %v", err)
	}
	if rec.GetString("kind") != "refund" {
		t.Errorf("kind = %q, want refund", rec.GetString("kind"))
	}
	if rec.GetString("provider") != "paddle" {
		t.Errorf("provider = %q, want paddle", rec.GetString("provider"))
	}
}

func TestRevenueAdjustment_EmptyEventID(t *testing.T) {
	saved, err := saveRevenueAdjustment(nil, revenueAdjustment{EventID: ""})
	if saved || err != nil {
		t.Errorf("empty event_id: saved=%v err=%v", saved, err)
	}
}

func TestMidtransAdjustmentEventID(t *testing.T) {
	id := midtransAdjustmentEventID("refund", "txn-123")
	if id != "midtrans-refund:txn-123" {
		t.Errorf("got %q, want midtrans-refund:txn-123", id)
	}
	id2 := midtransAdjustmentEventID("partial_refund", "txn-456")
	if id2 != "midtrans-partial_refund:txn-456" {
		t.Errorf("got %q", id2)
	}
}

// ── Midtrans webhook refund capture ───────────────────────────────

func TestMidtransWebhook_RefundCapture(t *testing.T) {
	resetMidtransDedup()
	resetRateLimiters()
	setMidtransEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// First provision a tenant with a settled charge so there is a tenant record.
	email := "refund-test@test.com"
	txnID := "txn-refund-001"
	orderID := "OZ-REFUND-001"
	body := midtransSignedBody("test-midtrans-server-key", txnID, orderID, "sub_refund_001", "settlement", "200", "149000", "plus", email)
	serveMidtrans(t, se, body)

	// Verify revenue_events was created.
	if _, err := app.FindFirstRecordByData("revenue_events", "event_id", txnID); err != nil {
		t.Fatalf("expected revenue_events to exist: %v", err)
	}

	// Now send a refund notification (same transaction_id, different status).
	// Midtrans refund notifications reuse the original transaction_id.
	refundBody := midtransSignedBody("test-midtrans-server-key", txnID, orderID, "sub_refund_001", "refund", "200", "149000", "plus", email)
	rec := serveMidtrans(t, se, refundBody)
	if rec.Code != http.StatusOK {
		t.Fatalf("refund webhook: expected 200, got %d", rec.Code)
	}

	// revenue_adjustments should have a record for this refund.
	adjID := midtransAdjustmentEventID("refund", txnID)
	adj, err := app.FindFirstRecordByData("revenue_adjustments", "event_id", adjID)
	if err != nil {
		t.Fatalf("expected adjustment record: %v", err)
	}
	if adj.GetString("kind") != "refund" {
		t.Errorf("kind = %q, want refund", adj.GetString("kind"))
	}
	if adj.GetInt("amount_idr") <= 0 {
		t.Errorf("amount_idr = %d, want > 0", adj.GetInt("amount_idr"))
	}
}

func TestMidtransWebhook_PartialRefundCapture(t *testing.T) {
	resetMidtransDedup()
	resetRateLimiters()
	setMidtransEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Provision tenant.
	email := "partial-refund@test.com"
	txnID := "txn-partial-001"
	orderID := "OZ-PARTIAL-001"
	body := midtransSignedBody("test-midtrans-server-key", txnID, orderID, "sub_partial_001", "settlement", "200", "149000", "plus", email)
	serveMidtrans(t, se, body)

	// Send a partial_refund notification WITH refund_amount.
	raw := fmt.Sprintf(`{
	  "transaction_id": %q,"order_id": %q,"transaction_status": "partial_refund","status_code": "200",
	  "gross_amount": "149000","refund_amount": "50000","fraud_status": "accept","payment_type": "qris",
	  "signature_key": "SIGNATURE","settlement_time": "2026-08-18 10:00:00"
	}`, txnID, orderID)
	sig := signMidtrans("test-midtrans-server-key", orderID, "200", "149000")
	body2 := strings.Replace(raw, `"signature_key": "SIGNATURE"`, fmt.Sprintf(`"signature_key": %q`, sig), 1)
	rec := serveMidtrans(t, se, body2)
	if rec.Code != http.StatusOK {
		t.Fatalf("partial_refund: expected 200, got %d", rec.Code)
	}

	adjID := midtransAdjustmentEventID("partial_refund", txnID)
	adj, err := app.FindFirstRecordByData("revenue_adjustments", "event_id", adjID)
	if err != nil {
		t.Fatalf("expected adjustment record: %v", err)
	}
	if adj.GetString("kind") != "partial_refund" {
		t.Errorf("kind = %q, want partial_refund", adj.GetString("kind"))
	}
	// refund_amount was 50000 IDR.
	if adj.GetInt("amount_idr") != 50000 {
		t.Errorf("amount_idr = %d, want 50000", adj.GetInt("amount_idr"))
	}
}

func TestMidtransWebhook_RefundSkipForNonClawback(t *testing.T) {
	// cancel / expire / deny must NOT create adjustments.
	resetMidtransDedup()
	resetRateLimiters()
	setMidtransEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	for _, status := range []string{"cancel", "expire", "deny"} {
		txnID := "txn-" + status + "-001"
		orderID := "OZ-" + strings.ToUpper(status) + "-001"
		body := midtransSignedBody("test-midtrans-server-key", txnID, orderID, "", status, "200", "149000", "", "")
		_ = serveMidtrans(t, se, body)
		if n := countRecords(t, app, "revenue_adjustments"); n > 0 {
			t.Errorf("%s created %d adjustment records, want 0", status, n)
		}
	}
}

// ── Paddle refund capture ─────────────────────────────────────────

func TestPaddleWebhook_TransactionRevokedCapture(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	// Provision a tenant + subscription first.
	provisionForEvents(t, app, se, "sub_revoked_001")

	// Send a transaction.revoked event.
	body := `{"event_id":"evt_rev_001","event_type":"transaction.revoked","data":{"id":"txn_rev_001","status":"revoked","currency_code":"USD","items":[{"price":{"id":"mock"},"totals":{"total":999}}],"totals":{"total":999,"grand_total":999}}}`
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rec.Code)
	}

	adj, err := app.FindFirstRecordByData("revenue_adjustments", "event_id", "evt_rev_001")
	if err != nil {
		t.Fatalf("expected adjustment record: %v", err)
	}
	if adj.GetString("kind") != "refund" {
		t.Errorf("kind = %q, want refund", adj.GetString("kind"))
	}
	if adj.GetString("provider") != "paddle" {
		t.Errorf("provider = %q, want paddle", adj.GetString("provider"))
	}
}

// ── Stats aggregation ─────────────────────────────────────────────

func TestAdminStats_RefundAggregation(t *testing.T) {
	resetProviderRevenueCache()
	resetRateLimiters()
	app, mux := dashboardMux(t)
	defer app.Cleanup()

	tenantID, _ := seedDashboardTenant(t, app, "agg-refund@test.com")
	t.Setenv("OZ_ADMIN_KEY", "secret-admin-key")
	now := time.Now().UTC()
	curKey := now.Format("2006-01")
	col, _ := app.FindCollectionByNameOrId("revenue_events")

	// Seed a revenue_events record.
	rev := core.NewRecord(col)
	rev.Set("event_id", "evt-agg-001")
	rev.Set("provider", "paddle")
	rev.Set("tenant_id", tenantID)
	rev.Set("currency", "USD")
	rev.Set("amount_usd", 100.00)
	rev.Set("amount_idr", 1600000)
	rev.Set("created", now.Format(time.RFC3339))
	if err := app.Save(rev); err != nil {
		t.Fatalf("save revenue event: %v", err)
	}

	adjCol, _ := app.FindCollectionByNameOrId("revenue_adjustments")
	adj := core.NewRecord(adjCol)
	adj.Set("event_id", "evt-adj-agg-001")
	adj.Set("provider", "paddle")
	adj.Set("kind", "refund")
	adj.Set("currency", "USD")
	adj.Set("amount_usd", 10.00)
	adj.Set("amount_idr", 160000)
	adj.Set("created", now.Format(time.RFC3339))
	if err := app.Save(adj); err != nil {
		t.Fatalf("save adjustment: %v", err)
	}

	// Fetch stats with ?refresh=1 to bypass cache.
	rec := doJSON(mux, http.MethodGet, "/api/v1/admin/stats?refresh=1", "Bearer secret-admin-key", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body struct {
		KPIs struct {
			LifetimeRefundUsd float64 `json:"lifetimeRefundUsd"`
			LifetimeRefundIdr float64 `json:"lifetimeRefundIdr"`
			MonthlyRefundUsd  float64 `json:"monthlyRefundUsd"`
			MonthlyRefundIdr  float64 `json:"monthlyRefundIdr"`
		} `json:"kpis"`
		RevenueTrend []struct {
			Month     string  `json:"month"`
			RefundUsd float64 `json:"refundUsd,omitempty"`
			RefundIdr float64 `json:"refundIdr,omitempty"`
		} `json:"revenueTrend"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("bad JSON: %v", err)
	}
	if body.KPIs.LifetimeRefundUsd != 10.00 {
		t.Errorf("lifetimeRefundUsd = %v, want 10.00", body.KPIs.LifetimeRefundUsd)
	}
	if body.KPIs.LifetimeRefundIdr != 160000 {
		t.Errorf("lifetimeRefundIdr = %v, want 160000", body.KPIs.LifetimeRefundIdr)
	}
	if body.KPIs.MonthlyRefundUsd != 10.00 {
		t.Errorf("monthlyRefundUsd = %v, want 10.00", body.KPIs.MonthlyRefundUsd)
	}
	if body.KPIs.MonthlyRefundIdr != 160000 {
		t.Errorf("monthlyRefundIdr = %v, want 160000", body.KPIs.MonthlyRefundIdr)
	}
	// The current month trend row should carry the refund.
	var found bool
	for _, rb := range body.RevenueTrend {
		if rb.Month == curKey {
			found = true
			if rb.RefundUsd != 10.00 {
				t.Errorf("trend row refundUsd = %v, want 10.00", rb.RefundUsd)
			}
		}
	}
	if !found {
		t.Errorf("current month %q not found in revenueTrend", curKey)
	}
}

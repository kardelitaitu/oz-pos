package main

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/pocketbase/pocketbase/core"
	"github.com/pocketbase/pocketbase/tests"
)

// ── Test helpers ────────────────────────────────────────────────────

// signPaddle computes a Paddle-Signature header value for a raw body.
func signPaddle(secret, body string, ts int64) string {
	signed := fmt.Sprintf("%d:%s", ts, body)
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write([]byte(signed))
	return fmt.Sprintf("ts=%d;h1=%s", ts, hex.EncodeToString(mac.Sum(nil)))
}

// stubReceiptEmail captures the license key passed to sendReceiptEmail.
func stubReceiptEmail(t *testing.T, captured *string) func() {
	t.Helper()
	t.Setenv("OZ_SMTP_HOST", "test.local")
	orig := sendReceiptEmail
	sendReceiptEmail = func(to, key, tier, expires string) error {
		*captured = key
		return nil
	}
	return func() { sendReceiptEmail = orig }
}

// setPaddleEnv configures the webhook env vars shared by most tests.
func setPaddleEnv(t *testing.T) {
	t.Helper()
	t.Setenv("PADDLE_WEBHOOK_SECRET", "test-webhook-secret")
	t.Setenv("PADDLE_PRICE_TIERS", "pri_test_pro:pro,pri_test_premium:premium")
}

// paddleCreatedBody renders a subscription.created payload. email "" omits
// custom_data (forcing the API-fetch path).
func paddleCreatedBody(eventID, subID, customerID, email, priceID string) string {
	custom := ""
	if email != "" {
		custom = fmt.Sprintf(`,"custom_data": {"email": %q}`, email)
	}
	return fmt.Sprintf(`{
  "event_id": %q,
  "event_type": "subscription.created",
  "data": {
    "id": %q,
    "status": "active",
    "customer_id": %q%s,
    "items": [{"price": {"id": %q, "product_id": "pro_test"}, "quantity": 1}],
    "current_billing_period": {"starts_at": %q, "ends_at": %q}
  }
}`, eventID, subID, customerID, custom, priceID,
		time.Now().UTC().AddDate(0, 0, -1).Format(time.RFC3339),
		time.Now().UTC().AddDate(1, 0, 0).Format(time.RFC3339))
}

// seedPaddleTenant creates a tenant in the state the Paddle webhook leaves
// behind: an active tenant whose api_key is a placeholder hash nobody knows
// (the customer's real key is minted at first activation).
func seedPaddleTenant(t *testing.T, app *tests.TestApp, email string) string {
	t.Helper()
	col, err := app.FindCollectionByNameOrId("tenants")
	if err != nil {
		t.Fatalf("tenants collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("email", email)
	rec.Set("phone", "-")
	placeholder := generateAPIKey()
	h, l, hashErr := hashAPIKey(placeholder)
	if hashErr != nil {
		t.Fatalf("hashAPIKey failed: %v", hashErr)
	}
	rec.Set("api_key", h)
	rec.Set("api_key_lookup", l)
	rec.Set("status", "active")
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed paddle tenant %q: %v", email, err)
	}
	return rec.Id
}

// seedLicenseKeyPaddle creates a webhook-issued license key (paddle_sub_id set).
func seedLicenseKeyPaddle(t *testing.T, app *tests.TestApp, key, tierKey, status, expiresAt, paddleSubID string) {
	t.Helper()
	col, err := app.FindCollectionByNameOrId("license_keys")
	if err != nil {
		t.Fatalf("license_keys collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("key", key)
	rec.Set("tier_key", tierKey)
	rec.Set("max_stores", 0)
	rec.Set("max_pos_instances", 0)
	rec.Set("allowed_types", `["restaurant-pos","store-pos","inventory","warehouse","admin","kds"]`)
	rec.Set("status", status)
	rec.Set("expires_at", expiresAt)
	rec.Set("paddle_sub_id", paddleSubID)
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed paddle license key %q: %v", key, err)
	}
}

func countRecords(t *testing.T, app *tests.TestApp, collection string) int {
	t.Helper()
	recs, err := app.FindRecordsByFilter(collection, "", "", 0, 0, nil)
	if err != nil {
		t.Fatalf("count %s failed: %v", collection, err)
	}
	return len(recs)
}

// ── Signature verification ──────────────────────────────────────────

func TestPaddleWebhook_ValidSignature_ProvisioningFlow(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	var emailedKey string
	restore := stubReceiptEmail(t, &emailedKey)
	defer restore()

	body := paddleCreatedBody("evt_created_001", "sub_created_001", "cus_001", "buyer@example.com", "pri_test_pro")
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())

	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	// Tenant upserted by email.
	tenant, err := app.FindFirstRecordByData("tenants", "email", "buyer@example.com")
	if err != nil {
		t.Fatalf("tenant not created: %v", err)
	}
	if tenant.GetString("phone") != "-" {
		t.Errorf("expected phone default '-', got %q", tenant.GetString("phone"))
	}

	// License key minted with the tier-derived prefix.
	keyRec, err := app.FindFirstRecordByData("license_keys", "paddle_sub_id", "sub_created_001")
	if err != nil {
		t.Fatalf("license key not created: %v", err)
	}
	key := keyRec.GetString("key")
	if !strings.HasPrefix(key, "OZ-PRO-") {
		t.Errorf("expected key prefix OZ-PRO-, got %q", key)
	}
	if keyRec.GetString("status") != "unused" {
		t.Errorf("expected key status unused, got %q", keyRec.GetString("status"))
	}
	if emailedKey != key {
		t.Errorf("expected receipt email with key %q, got %q", key, emailedKey)
	}

	// Subscription created with an RSA-signed payload.
	subRec, err := app.FindFirstRecordByData("subscriptions", "paddle_sub_id", "sub_created_001")
	if err != nil {
		t.Fatalf("subscription not created: %v", err)
	}
	payload := subRec.GetString("signed_payload")
	if !strings.Contains(payload, `"tier_key":"pro"`) {
		t.Errorf("expected signed payload to carry tier pro, got: %s", payload)
	}
	if subRec.GetString("signature") == "" {
		t.Error("expected a non-empty signature")
	}
	if subRec.GetString("status") != "active" {
		t.Errorf("expected sub status active, got %q", subRec.GetString("status"))
	}

	// The tier quota block must be persisted on the subscription record so
	// /status and subscription.updated / canceled re-signs read real values
	// instead of zero values.
	if subRec.GetInt("max_stores") != 0 || subRec.GetInt("max_pos_instances") != 0 {
		t.Errorf("expected pro tier to persist unlimited quotas, got max_stores=%d max_pos_instances=%d",
			subRec.GetInt("max_stores"), subRec.GetInt("max_pos_instances"))
	}
	if got := subRec.GetString("allowed_types"); !strings.Contains(got, "restaurant-pos") {
		t.Errorf("expected allowed_types to persist workspace types, got %q", got)
	}
}

func TestPaddleWebhook_InvalidSignature_401(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := paddleCreatedBody("evt_bad_001", "sub_bad_001", "cus_001", "x@example.com", "pri_test_pro")
	// Sign the original body, then send a tampered one (trailing space).
	header := signPaddle("test-webhook-secret", body+" ", time.Now().Unix())

	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401, got %d: %s", rec.Code, rec.Body.String())
	}
	if countRecords(t, app, "tenants") != 0 {
		t.Error("no tenant should be created on a bad signature")
	}
}

func TestPaddleWebhook_MissingSignature_401(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	rec := webRequest(t, se, http.MethodPost, paddleWebhookPath,
		paddleCreatedBody("evt_nosig_001", "sub_nosig_001", "cus_001", "x@example.com", "pri_test_pro"), "", "")
	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestPaddleWebhook_StaleTimestamp_401(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := paddleCreatedBody("evt_stale_001", "sub_stale_001", "cus_001", "x@example.com", "pri_test_pro")
	// 10 minutes in the past — outside the 5-minute replay window.
	header := signPaddle("test-webhook-secret", body, time.Now().Unix()-600)

	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401 for stale ts, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestPaddleWebhook_KeyRotation_AcceptsNewSecret(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := paddleCreatedBody("evt_rot_001", "sub_rot_001", "cus_001", "x@example.com", "pri_test_pro")
	ts := time.Now().Unix()
	oldSig := signPaddle("old-secret", body, ts)
	newSig := signPaddle("test-webhook-secret", body, ts)
	header := oldSig + " " + newSig

	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 with key-rotation header, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestPaddleWebhook_NoSecret_503(t *testing.T) {
	resetPaddleDedup()
	t.Setenv("PADDLE_WEBHOOK_SECRET", "")
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := paddleCreatedBody("evt_nosecret_001", "sub_nosecret_001", "cus_001", "x@example.com", "pri_test_pro")
	header := signPaddle("whatever", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusServiceUnavailable {
		t.Fatalf("expected 503, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestPaddleWebhook_NotBlockedByBrowserOrigin(t *testing.T) {
	// The webhook is server-to-server and must NOT be behind the web CORS
	// allowlist — a signature-valid request from any Origin processes.
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := paddleCreatedBody("evt_origin_001", "sub_origin_001", "cus_001", "x@example.com", "pri_test_pro")
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	req.Header.Set("Origin", "http://evil.example")
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 (no CORS gate), got %d: %s", rec.Code, rec.Body.String())
	}
}

// ── Provisioning / event handling ───────────────────────────────────

func TestPaddleWebhook_DuplicateEventID_Noop(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := paddleCreatedBody("evt_dup_001", "sub_dup_001", "cus_001", "dup@example.com", "pri_test_pro")
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := func() *http.Request {
		r := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
		r.Header.Set(paddleSignatureHeader, header)
		return r
	}
	mux, _ := se.Router.BuildMux()

	rec1 := httptest.NewRecorder()
	mux.ServeHTTP(rec1, req())
	if rec1.Code != http.StatusOK {
		t.Fatalf("first delivery expected 200, got %d", rec1.Code)
	}
	rec2 := httptest.NewRecorder()
	mux.ServeHTTP(rec2, req())
	if rec2.Code != http.StatusOK {
		t.Fatalf("replay expected 200, got %d", rec2.Code)
	}
	if !strings.Contains(rec2.Body.String(), "duplicate") {
		t.Errorf("expected duplicate marker, got %s", rec2.Body.String())
	}

	// Exactly one key + one subscription + one tenant.
	if n := countRecords(t, app, "license_keys"); n != 1 {
		t.Errorf("expected 1 license key, got %d", n)
	}
	if n := countRecords(t, app, "subscriptions"); n != 1 {
		t.Errorf("expected 1 subscription, got %d", n)
	}
}

func TestPaddleWebhook_RedeliveryAfterRestart_UpsertsNotDuplicates(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := paddleCreatedBody("evt_again_001", "sub_again_001", "cus_001", "again@example.com", "pri_test_pro")
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	send := func() {
		req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
		req.Header.Set(paddleSignatureHeader, header)
		rec := httptest.NewRecorder()
		mux, _ := se.Router.BuildMux()
		mux.ServeHTTP(rec, req)
		if rec.Code != http.StatusOK {
			t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
		}
	}
	send()
	// Simulate a restart losing the in-memory dedup map.
	resetPaddleDedup()
	send()

	if n := countRecords(t, app, "license_keys"); n != 1 {
		t.Errorf("expected 1 license key after re-delivery, got %d", n)
	}
	if n := countRecords(t, app, "subscriptions"); n != 1 {
		t.Errorf("expected 1 subscription after re-delivery, got %d", n)
	}
}

func TestPaddleWebhook_UnmappedPrice_500(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := paddleCreatedBody("evt_unmapped_001", "sub_unmapped_001", "cus_001", "x@example.com", "pri_unknown_plan")
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusInternalServerError {
		t.Fatalf("expected 500 (Paddle retries), got %d: %s", rec.Code, rec.Body.String())
	}
	if n := countRecords(t, app, "tenants"); n != 0 {
		t.Error("no tenant should be created for an unmapped price")
	}
}

func TestPaddleWebhook_EmailFromCustomData(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := paddleCreatedBody("evt_cd_001", "sub_cd_001", "cus_001", "customdata@example.com", "pri_test_pro")
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	if _, err := app.FindFirstRecordByData("tenants", "email", "customdata@example.com"); err != nil {
		t.Fatalf("tenant should be created from custom_data.email: %v", err)
	}
}

func TestPaddleWebhook_EmailFromAPIFetch(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	t.Setenv("PADDLE_API_KEY", "test-api-key")
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	orig := fetchPaddleCustomer
	fetchPaddleCustomer = func(customerID string) string {
		if customerID == "cus_api_001" {
			return "api@example.com"
		}
		return ""
	}
	defer func() { fetchPaddleCustomer = orig }()

	// No custom_data email → must fall back to the API fetch.
	body := paddleCreatedBody("evt_api_001", "sub_api_001", "cus_api_001", "", "pri_test_pro")
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	if _, err := app.FindFirstRecordByData("tenants", "email", "api@example.com"); err != nil {
		t.Fatalf("tenant should be created from API-fetched email: %v", err)
	}
}

func TestPaddleWebhook_UnresolvableEmail_500(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	// No PADDLE_API_KEY, no custom_data email, no embedded customer.
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := paddleCreatedBody("evt_noemail_001", "sub_noemail_001", "cus_noemail_001", "", "pri_test_pro")
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusInternalServerError {
		t.Fatalf("expected 500 (Paddle retries until email is resolvable), got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestPaddleWebhook_TransactionCompleted_Acknowledged(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := `{"event_id":"evt_tx_001","event_type":"transaction.completed","data":{"id":"txn_001","status":"completed"}}`
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	if n := countRecords(t, app, "tenants"); n != 0 {
		t.Error("transaction.completed must not provision (no lifetime tier yet)")
	}
}

func TestPaddleWebhook_UnknownEventType_Acknowledged(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	body := `{"event_id":"evt_cust_001","event_type":"customer.created","data":{"id":"cus_001"}}`
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 ack, got %d: %s", rec.Code, rec.Body.String())
	}
}

// subscriptionLifecycle provisions a sub first, then returns its records.
func provisionForEvents(t *testing.T, app *tests.TestApp, se *core.ServeEvent, subID string) {
	t.Helper()
	body := paddleCreatedBody("evt_lc_"+subID, subID, "cus_lc_001", "lc@example.com", "pri_test_pro")
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("provision failed: %d: %s", rec.Code, rec.Body.String())
	}
}

func TestPaddleWebhook_SubscriptionUpdated_SyncsTierAndExpiry(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	provisionForEvents(t, app, se, "sub_upd_001")

	// Upgrade pro → premium and extend the period.
	body := fmt.Sprintf(`{
  "event_id": "evt_upd_001",
  "event_type": "subscription.updated",
  "data": {
    "id": "sub_upd_001",
    "status": "active",
    "customer_id": "cus_lc_001",
    "items": [{"price": {"id": "pri_test_premium", "product_id": "pro_test"}, "quantity": 1}],
    "current_billing_period": {"starts_at": %q, "ends_at": %q}
  }
}`, time.Now().UTC().Format(time.RFC3339),
		time.Now().UTC().AddDate(2, 0, 0).Format(time.RFC3339))
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	subRec, err := app.FindFirstRecordByData("subscriptions", "paddle_sub_id", "sub_upd_001")
	if err != nil {
		t.Fatalf("subscription not found: %v", err)
	}
	if subRec.GetString("tier_key") != "premium" {
		t.Errorf("expected tier premium after update, got %q", subRec.GetString("tier_key"))
	}
	if !strings.Contains(subRec.GetString("signed_payload"), `"tier_key":"premium"`) {
		t.Errorf("signed payload should reflect the new tier, got: %s", subRec.GetString("signed_payload"))
	}

	// The license key must be kept in sync (tier + expiry).
	keyRec, err := app.FindFirstRecordByData("license_keys", "paddle_sub_id", "sub_upd_001")
	if err != nil {
		t.Fatalf("license key not found: %v", err)
	}
	if keyRec.GetString("tier_key") != "premium" {
		t.Errorf("expected key tier premium after update, got %q", keyRec.GetString("tier_key"))
	}
	if keyRec.GetString("expires_at") != subRec.GetString("expires_at") {
		t.Errorf("expected key expiry synced to sub expiry (%s), got %s",
			subRec.GetString("expires_at"), keyRec.GetString("expires_at"))
	}
}

func TestPaddleWebhook_SubscriptionCanceled_GracePeriod(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	provisionForEvents(t, app, se, "sub_can_001")

	graceUntil := time.Now().UTC().AddDate(0, 1, 0).Format(time.RFC3339)
	body := fmt.Sprintf(`{
  "event_id": "evt_can_001",
  "event_type": "subscription.canceled",
  "data": {
    "id": "sub_can_001",
    "status": "canceled",
    "customer_id": "cus_lc_001",
    "items": [{"price": {"id": "pri_test_pro", "product_id": "pro_test"}, "quantity": 1}],
    "scheduled_change": {"effective_at": %q, "status": "active"}
  }
}`, graceUntil)
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, _ := se.Router.BuildMux()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	subRec, err := app.FindFirstRecordByData("subscriptions", "paddle_sub_id", "sub_can_001")
	if err != nil {
		t.Fatalf("subscription not found: %v", err)
	}
	if subRec.GetString("status") != "grace_period" {
		t.Errorf("expected status grace_period, got %q", subRec.GetString("status"))
	}
	// PocketBase normalizes date storage ("2026-09-15T23:10:49Z" →
	// "2026-09-15 23:10:49.000Z") — compare via the DateTime accessor.
	wantT, err1 := time.Parse(time.RFC3339, graceUntil)
	gotT := subRec.GetDateTime("grace_until").Time()
	if err1 != nil || !wantT.Equal(gotT) {
		t.Errorf("expected grace_until %s, got %s", graceUntil, gotT.Format(time.RFC3339))
	}
	if !strings.Contains(subRec.GetString("signed_payload"), `"status":"grace_period"`) {
		t.Errorf("signed payload should reflect grace_period, got: %s", subRec.GetString("signed_payload"))
	}
}

// ── Activation integration ──────────────────────────────────────────

func TestActivate_PaddleKeyWithoutAPIKey_MintsKeyAndReusesSubscription(t *testing.T) {
	resetPaddleDedup()
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	tenantID := seedPaddleTenant(t, app, "paddle-activate@example.com")
	expiresAt := time.Now().UTC().AddDate(1, 0, 0).Format(time.RFC3339)
	seedLicenseKeyPaddle(t, app, "OZ-PRO-PADDLE-0001", "pro", "unused", expiresAt, "sub_act_001")
	// The webhook-created active subscription (signed_payload "{}" marks it).
	seedSubscription(t, app, tenantID, "pro", "active")

	body := fmt.Sprintf(`{"key":"OZ-PRO-PADDLE-0001","machine_id":"aaaaaaaaaaaaaaa","email":"paddle-activate@example.com"}`)
	rec := webRequest(t, se, http.MethodPost, "/api/v1/license/activate", body, "", "")

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var resp map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("response not JSON: %v", err)
	}
	apiKey, _ := resp["api_key"].(string)
	if apiKey == "" {
		t.Fatal("expected a freshly minted api_key in the response")
	}
	if resp["signed_payload"] != "{}" {
		t.Errorf("expected the webhook-created subscription to be reused, got signed_payload %v", resp["signed_payload"])
	}

	// The minted api_key must actually authenticate /status (tenant rotated).
	if _, err := findTenantByAPIKey(app, apiKey); err != nil {
		t.Errorf("minted api_key should resolve the tenant: %v", err)
	}

	// The key must be marked activated and NO duplicate subscription created.
	keyRec, err := app.FindFirstRecordByData("license_keys", "key", "OZ-PRO-PADDLE-0001")
	if err != nil {
		t.Fatalf("key not found: %v", err)
	}
	if keyRec.GetString("status") != "activated" {
		t.Errorf("expected key activated, got %q", keyRec.GetString("status"))
	}
	subs, err := app.FindRecordsByFilter("subscriptions", "tenant_id = {:tid}", "", 0, 0,
		map[string]any{"tid": tenantID})
	if err != nil {
		t.Fatalf("subscription query failed: %v", err)
	}
	if len(subs) != 1 {
		t.Errorf("expected exactly 1 subscription (no duplicate), got %d", len(subs))
	}
}

func TestActivate_ManualKeyExistingTenantStillRequiresAPIKey(t *testing.T) {
	// Regression: the paddle-key exception must NOT weaken manual keys —
	// activating an unused manual key onto an existing tenant still requires
	// the tenant's api_key.
	resetPaddleDedup()
	resetRateLimiters()
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "manualtenant001", "manualkey000001", "active")
	expiresAt := time.Now().UTC().AddDate(1, 0, 0).Format(time.RFC3339)
	seedLicenseKey(t, app, "OZ-PRO-MANUAL-0001", "pro", "unused", expiresAt)

	body := fmt.Sprintf(`{"key":"OZ-PRO-MANUAL-0001","machine_id":"aaaaaaaaaaaaaaa","email":"MANUALTENANT001@example.com"}`)
	rec := webRequest(t, se, http.MethodPost, "/api/v1/license/activate", body, "", "")

	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401 (api_key required for manual keys), got %d: %s", rec.Code, rec.Body.String())
	}
}

// ── Remaining lifecycle + robustness paths ─────────────────────────

// sendPaddleEvent signs and delivers a raw webhook body to the router.
func sendPaddleEvent(t *testing.T, se *core.ServeEvent, body string) *httptest.ResponseRecorder {
	t.Helper()
	header := signPaddle("test-webhook-secret", body, time.Now().Unix())
	req := httptest.NewRequest(http.MethodPost, paddleWebhookPath, strings.NewReader(body))
	req.Header.Set(paddleSignatureHeader, header)
	rec := httptest.NewRecorder()
	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux failed: %v", err)
	}
	mux.ServeHTTP(rec, req)
	return rec
}

// subEventBody renders a minimal subscription.<event_type> payload.
func subEventBody(eventType, eventID, subID, status string) string {
	return fmt.Sprintf(`{
  "event_id": %q,
  "event_type": %q,
  "data": {
    "id": %q,
    "status": %q,
    "customer_id": "cus_lc_001",
    "items": [{"price": {"id": "pri_test_pro", "product_id": "pro_test"}, "quantity": 1}]
  }
}`, eventID, eventType, subID, status)
}

func TestPaddleWebhook_SubscriptionPaused_GracePeriod(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	provisionForEvents(t, app, se, "sub_pause_001")

	rec := sendPaddleEvent(t, se, subEventBody("subscription.paused", "evt_pause_001", "sub_pause_001", "paused"))
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	subRec, err := app.FindFirstRecordByData("subscriptions", "paddle_sub_id", "sub_pause_001")
	if err != nil {
		t.Fatalf("subscription not found: %v", err)
	}
	if subRec.GetString("status") != "grace_period" {
		t.Errorf("expected paused -> grace_period, got %q", subRec.GetString("status"))
	}
}

func TestPaddleWebhook_SubscriptionResumed_BackToActive(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	provisionForEvents(t, app, se, "sub_res_001")
	// Pause first, then resume.
	sendPaddleEvent(t, se, subEventBody("subscription.paused", "evt_res_pause_001", "sub_res_001", "paused"))
	rec := sendPaddleEvent(t, se, subEventBody("subscription.resumed", "evt_res_001", "sub_res_001", "active"))
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	subRec, err := app.FindFirstRecordByData("subscriptions", "paddle_sub_id", "sub_res_001")
	if err != nil {
		t.Fatalf("subscription not found: %v", err)
	}
	if subRec.GetString("status") != "active" {
		t.Errorf("expected resumed -> active, got %q", subRec.GetString("status"))
	}
	if !strings.Contains(subRec.GetString("signed_payload"), `"status":"active"`) {
		t.Errorf("resumed subscription must be re-signed as active, got: %s", subRec.GetString("signed_payload"))
	}
}

func TestPaddleWebhook_SubscriptionPastDue_StaysActive(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	provisionForEvents(t, app, se, "sub_due_001")

	rec := sendPaddleEvent(t, se, subEventBody("subscription.past_due", "evt_due_001", "sub_due_001", "past_due"))
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	// past_due is acknowledged without touching the subscription (Paddle is
	// still retrying the payment) — it must remain active.
	subRec, err := app.FindFirstRecordByData("subscriptions", "paddle_sub_id", "sub_due_001")
	if err != nil {
		t.Fatalf("subscription not found: %v", err)
	}
	if subRec.GetString("status") != "active" {
		t.Errorf("expected past_due to leave subscription active, got %q", subRec.GetString("status"))
	}
}

func TestPaddleWebhook_ReceiptEmailFailure_NonFatal(t *testing.T) {
	// Provisioning must succeed even when the receipt email fails — the key
	// is also visible in the dashboard, so a mail hiccup must not block it.
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	orig := sendReceiptEmail
	sendReceiptEmail = func(to, key, tier, expires string) error {
		return fmt.Errorf("relay down")
	}
	defer func() { sendReceiptEmail = orig }()

	body := paddleCreatedBody("evt_rcpt_fail_001", "sub_rcpt_fail_001", "cus_001", "buyer@example.com", "pri_test_pro")
	rec := sendPaddleEvent(t, se, body)
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 despite email failure, got %d: %s", rec.Code, rec.Body.String())
	}
	if _, err := app.FindFirstRecordByData("license_keys", "paddle_sub_id", "sub_rcpt_fail_001"); err != nil {
		t.Fatalf("license key must still be minted when the receipt fails: %v", err)
	}
}

func TestPaddleWebhook_UpdateUnknownSubscription_Acknowledged(t *testing.T) {
	// subscription.updated for a paddle_sub_id we have no local record for
	// (e.g. created before the webhook shipped) must acknowledge, not 500 —
	// otherwise Paddle retries forever.
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	rec := sendPaddleEvent(t, se, subEventBody("subscription.updated", "evt_unknown_upd_001", "sub_unknown_001", "active"))
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 ack, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestPaddleWebhook_CancelUnknownSubscription_Acknowledged(t *testing.T) {
	resetPaddleDedup()
	setPaddleEnv(t)
	app, se := setupDirectApp(t)
	defer app.Cleanup()

	rec := sendPaddleEvent(t, se, subEventBody("subscription.canceled", "evt_unknown_can_001", "sub_unknown_001", "canceled"))
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 ack, got %d: %s", rec.Code, rec.Body.String())
	}
}

package main

// Revenue event persistence (revenue-data-pipeline-plan.md, Phase A).
//
// saveRevenueEvent records a completed payment from either provider into
// the revenue_events collection so the admin dashboard can show REAL
// transaction revenue (not the price-map estimate):
//
//	Paddle   → event_id = Paddle event_id,  currency = USD
//	Midtrans → event_id = Midtrans transaction_id, currency = IDR
//
// Both the native amount and its FX-converted counterpart are stored, so
// the stats endpoint can sum in either currency without re-fetching the
// rate per record. The event_id is the dedup key: webhook retries (Paddle
// retries non-2xx; Midtrans may resend) are idempotent.

import (
	"log"
	"strconv"
	"strings"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// revenueEvent is the data needed to persist one payment record.
type revenueEvent struct {
	Provider       string  // "paddle" | "midtrans"
	EventID        string  // provider event/transaction id (dedup key)
	TenantID       string  // tenants record id
	TierKey        string  // plus/pro/premium/enterprise ("" if unknown)
	NativeAmount   float64 // amount in the native currency
	NativeCurrency string  // "USD" | "IDR"
	SubscriptionID string  // paddle_sub_id or midtrans_sub_id (optional)
	Notes          string  // metadata (payment_type, bundle, etc.)
}

// saveRevenueEvent persists a completed payment idempotently. Returns
// (saved bool, err) — saved=false when the event_id already exists.
func saveRevenueEvent(app core.App, ev revenueEvent) (bool, error) {
	if ev.EventID == "" {
		return false, nil
	}

	// Idempotency: skip if the event is already recorded (webhook retry).
	existing, err := app.FindFirstRecordByData("revenue_events", "event_id", ev.EventID)
	if err == nil && existing != nil {
		return false, nil
	}
	if err != nil && !isNoRowsError(err) {
		log.Printf("revenue_events: lookup failed for event=%s: %v", ev.EventID, err)
		// Fall through to insert; the unique index will reject duplicates.
	}

	// Normalize amounts into both currencies. Midtrans gross_amount is an
	// integer string in IDR; Paddle totals are decimal in USD.
	var amountUsd, amountIdr float64
	switch strings.ToUpper(ev.NativeCurrency) {
	case "USD":
		amountUsd = ev.NativeAmount
		fx, _, _ := getFxRate()
		amountIdr = amountUsd * fx
	case "IDR":
		amountIdr = ev.NativeAmount
		fx, _, _ := getFxRate()
		if fx > 0 {
			amountUsd = amountIdr / fx
		}
	}

	coll, err := app.FindCollectionByNameOrId("revenue_events")
	if err != nil {
		return false, err
	}
	rec := core.NewRecord(coll)
	rec.Set("event_id", ev.EventID)
	rec.Set("provider", ev.Provider)
	rec.Set("tenant_id", ev.TenantID)
	rec.Set("currency", strings.ToUpper(ev.NativeCurrency))
	rec.Set("amount_usd", round2(amountUsd))
	rec.Set("amount_idr", int64(amountIdr))
	if ev.TierKey != "" {
		rec.Set("tier_key", ev.TierKey)
	}
	if ev.SubscriptionID != "" {
		rec.Set("subscription_id", ev.SubscriptionID)
	}
	if ev.Notes != "" {
		rec.Set("notes", ev.Notes)
	}
	if err := app.Save(rec); err != nil {
		return false, err
	}
	log.Printf("revenue_events: recorded %s event=%s amount=%s %.2f",
		ev.Provider, ev.EventID, ev.NativeCurrency, ev.NativeAmount)
	return true, nil
}

// parseMidtransGrossAmount converts a Midtrans gross_amount string
// (integer IDR, e.g. "199000") to a float64. Returns 0 on parse failure.
func parseMidtransGrossAmount(gross string) float64 {
	g := strings.TrimSpace(gross)
	if g == "" {
		return 0
	}
	v, err := strconv.ParseFloat(g, 64)
	if err != nil {
		log.Printf("midtrans: unparseable gross_amount=%q", gross)
		return 0
	}
	return v
}

// revenueEventCreatedAt returns the settlement time if parseable, else now.
func revenueEventCreatedAt(ts string) time.Time {
	if t, err := time.Parse(time.RFC3339, ts); err == nil {
		return t
	}
	return time.Now().UTC()
}

// round2 rounds to 2 decimal places for money values.
func round2(v float64) float64 {
	return float64(int64(v*100+0.5)) / 100
}

// isNoRowsError reports whether a PocketBase query error means "no match"
// (which we treat as a fresh event, not a failure).
func isNoRowsError(err error) bool {
	if err == nil {
		return false
	}
	msg := err.Error()
	return strings.Contains(msg, "no rows") ||
		strings.Contains(msg, "not found") ||
		strings.Contains(msg, "sql: no rows")
}

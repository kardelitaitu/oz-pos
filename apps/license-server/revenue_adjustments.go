package main

// Refund / adjustment persistence (revenue-data-pipeline-plan.md, Phase D).
//
// saveRevenueAdjustment records a negative revenue event — a refund,
// partial refund, or chargeback from either provider — into the
// revenue_adjustments collection so the admin dashboard can show the
// operator how much gross is NOT keepable:
//
//	Paddle   → transaction.revoked / refunded events (event_id = Paddle event id)
//	Midtrans → transaction_status refund / partial_refund notifications
//
// The ledger mirrors revenue_events: amounts are stored in BOTH
// currencies (native + write-time FX), the event_id is the idempotency
// key, and the row is signed-proof-only (written by the verified
// webhook handler, never by admin DB edits).

import (
	"log"
	"strings"

	"github.com/pocketbase/pocketbase/core"
)

// revenueAdjustment is the data needed to persist one refund/adjustment.
type revenueAdjustment struct {
	Provider string // "paddle" | "midtrans"
	EventID  string // dedup key (see note per provider above)
	TenantID string // tenants record id ("" when unresolvable)
	Kind     string // refund | partial_refund | chargeback
	// NativeAmountMinor: refund amount in native minor units (USD cents /
	// whole rupiah).  Kept positive; stats subtract it from gross.
	NativeAmountMinor int64
	NativeCurrency    string // "USD" | "IDR"
	Notes             string
}

// saveRevenueAdjustment persists a refund/adjustment idempotently.
// Returns (saved bool, err) — saved=false when the event_id already exists.
func saveRevenueAdjustment(app core.App, adj revenueAdjustment) (bool, error) {
	if adj.EventID == "" {
		return false, nil
	}

	// Idempotency: skip if already recorded (webhook retry).
	existing, err := app.FindFirstRecordByData("revenue_adjustments", "event_id", adj.EventID)
	if err == nil && existing != nil {
		return false, nil
	}
	if err != nil && !isNoRowsError(err) {
		log.Printf("revenue_adjustments: lookup failed for event=%s: %v", adj.EventID, err)
	}

	// Normalize amounts into both currencies (mirror saveRevenueEvent).
	var amountUsd, amountIdr float64
	switch strings.ToUpper(adj.NativeCurrency) {
	case "USD":
		amountUsd = float64(adj.NativeAmountMinor) / 100.0
		fx, _, _ := getFxRate()
		amountIdr = amountUsd * fx
	case "IDR":
		amountIdr = float64(adj.NativeAmountMinor)
		fx, _, _ := getFxRate()
		if fx > 0 {
			amountUsd = amountIdr / fx
		}
	}

	coll, err := app.FindCollectionByNameOrId("revenue_adjustments")
	if err != nil {
		return false, err
	}
	rec := core.NewRecord(coll)
	rec.Set("event_id", adj.EventID)
	rec.Set("provider", adj.Provider)
	rec.Set("kind", adj.Kind)
	rec.Set("currency", strings.ToUpper(adj.NativeCurrency))
	rec.Set("amount_usd", round2(amountUsd))
	rec.Set("amount_idr", int64(amountIdr))
	if adj.TenantID != "" {
		rec.Set("tenant_id", adj.TenantID)
	}
	if adj.Notes != "" {
		rec.Set("notes", adj.Notes)
	}
	if err := app.Save(rec); err != nil {
		return false, err
	}
	log.Printf("revenue_adjustments: recorded %s %s event=%s amount=%d %s (minor units)",
		adj.Provider, adj.Kind, adj.EventID, adj.NativeAmountMinor, adj.NativeCurrency)
	return true, nil
}

// midtransAdjustmentEventID builds a stable dedup key for a Midtrans
// refund notification.  Midtrans reuses the ORIGINAL transaction_id on
// refund/partial_refund notifications, so the key must incorporate the
// kind — otherwise the first charge's idempotency check would collide.
func midtransAdjustmentEventID(kind, transactionID string) string {
	return "midtrans-" + kind + ":" + transactionID
}

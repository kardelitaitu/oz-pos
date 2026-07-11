package main

import (
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// RenewRequest is the JSON body for POST /api/v1/license/renew.
type RenewRequest struct {
	TenantID string `json:"tenant_id"`
	APIKey   string `json:"api_key"`
}

func handleRenew(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		var req RenewRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "invalid request body",
			})
		}

		// ── Validate required fields ──────────────────────────────
		if req.TenantID == "" || req.APIKey == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "tenant_id and api_key are required",
			})
		}

		// ── Authenticate tenant by api_key ────────────────────────
		tenant, err := app.FindFirstRecordByData("tenants", "api_key", req.APIKey)
		if err != nil || tenant.GetString("status") != "active" {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid api_key or tenant is not active",
			})
		}
		if tenant.GetString("id") != req.TenantID {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "tenant_id does not match api_key",
			})
		}

		// ── Find the latest active subscription ───────────────────
		subs, err := app.FindRecordsByFilter(
			"subscriptions",
			"tenant_id = {:tenant_id} && status = 'active'",
			"-starts_at", 1, 0,
			map[string]any{"tenant_id": req.TenantID},
		)
		if err != nil || len(subs) == 0 {
			return e.JSON(http.StatusNotFound, map[string]any{
				"error": "no active subscription found",
			})
		}
		currentSub := subs[0]

		// ── Parse current subscription data ───────────────────────
		tierKey := currentSub.GetString("tier_key")
		expiresAt := calculateExpiry(tierKey)
		sub := SubscriptionPayload{
			TenantID:   req.TenantID,
			TierKey:    tierKey,
			Status:     "active",
			StartsAt:   time.Now().UTC().Format(time.RFC3339),
			ExpiresAt:  expiresAt.Format(time.RFC3339),
			GraceUntil: calculateGraceUntil(expiresAt).Format(time.RFC3339),
			IssuedAt:   time.Now().UTC().Format(time.RFC3339),
		}

		payloadStr, signature, err := signSubscription(sub)
		if err != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "signing failed",
			})
		}

		// ── Mark old subscription as expired ──────────────────────
		currentSub.Set("status", "expired")
		if err := app.Save(currentSub); err != nil {
			log.Printf("WARNING: failed to mark old subscription as expired: %v", err)
		}

		// ── Save new subscription ─────────────────────────────────
		subColl, err := app.FindCollectionByNameOrId("subscriptions")
		if err != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "server misconfiguration: subscriptions collection not found",
			})
		}
		newSub := core.NewRecord(subColl)
		newSub.Set("tenant_id", req.TenantID)
		newSub.Set("tier_key", tierKey)
		newSub.Set("max_stores", currentSub.GetInt("max_stores"))
		newSub.Set("max_pos_instances", currentSub.GetInt("max_pos_instances"))
		newSub.Set("allowed_types", currentSub.GetString("allowed_types"))
		newSub.Set("status", "active")
		newSub.Set("starts_at", sub.StartsAt)
		newSub.Set("expires_at", sub.ExpiresAt)
		newSub.Set("grace_until", sub.GraceUntil)
		newSub.Set("signed_payload", payloadStr)
		newSub.Set("signature", signature)
		if err := app.Save(newSub); err != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to save subscription",
			})
		}

		return e.JSON(http.StatusOK, map[string]any{
			"signed_payload": payloadStr,
			"signature":      signature,
		})
	}
}

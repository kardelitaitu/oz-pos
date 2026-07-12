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
	Key      string `json:"key"` // newly purchased key to extend subscription
}

func handleRenew(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// Cap request body at 64KB to prevent OOM via oversized JSON payloads (M4 audit).
		e.Request.Body = http.MaxBytesReader(e.Response, e.Request.Body, 64*1024)
		var req RenewRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "invalid request body",
			})
		}

		// ── Validate required fields ──────────────────────────────
		if req.TenantID == "" || req.APIKey == "" || req.Key == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "tenant_id, api_key, and key are required",
			})
		}

		clientIP := e.RealIP()
		if !ipRateLimiter.allow(clientIP) {
			return e.JSON(http.StatusTooManyRequests, map[string]any{
				"error": "rate limit exceeded, try again later",
			})
		}

		if blocked, waitDuration := keyFailTracker.isBlocked(req.Key); blocked {
			waitStr := waitDuration.Round(time.Second).String()
			return e.JSON(http.StatusTooManyRequests, map[string]any{
				"error": "too many attempts for this key, try again in " + waitStr,
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

		// ── Per-key activation lock (C2/C3 audit fix) ──────────────
		// Serialise requests for the same license key to prevent a
		// TOCTOU race: without this lock, two concurrent renewals for
		// the same key can both read keyRecord.GetString("status") ==
		// "unused" before either saves "activated", silently granting
		// the customer an extra renewed subscription for one key
		// purchase. Mirrors the same lock pattern used in activate.go
		// so activation and renewal share the per-key mutex.
		unlock := activationLocks.lock(req.Key)
		defer unlock()

		// ── Validate the NEW license key ──────────────────────────
		keyRecord, err := app.FindFirstRecordByData("license_keys", "key", req.Key)
		if err != nil || keyRecord.GetString("status") != "unused" {
			keyFailTracker.recordFailure(req.Key)
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid or already used license key",
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
				"error": "no active subscription found to renew",
			})
		}
		currentSub := subs[0]

		// ── Parse current subscription data & extend expiry ───────
		tierKey := keyRecord.GetString("tier_key")
		currentExpiresAt := currentSub.GetDateTime("expires_at").Time()
		
		// If the current subscription has already expired, start from time.Now()
		// If it's still active, append the new duration to the current expiry
		baseTime := currentExpiresAt
		if baseTime.Before(time.Now().UTC()) {
			baseTime = time.Now().UTC()
		}

		// Calculate new expiry manually based on tier
		var newExpiresAt time.Time
		switch tierKey {
		case "free":
			newExpiresAt = baseTime.AddDate(100, 0, 0)
		case "pro", "premium":
			newExpiresAt = baseTime.AddDate(1, 0, 0)
		case "enterprise":
			newExpiresAt = baseTime.AddDate(3, 0, 0)
		default:
			newExpiresAt = baseTime.AddDate(1, 0, 0)
		}

	var allowedTypes []string
	if err := json.Unmarshal([]byte(keyRecord.GetString("allowed_types")), &allowedTypes); err != nil {
		allowedTypes = []string{}
	}

	sub := SubscriptionPayload{
		TenantID:        req.TenantID,
		TierKey:         tierKey,
		Status:          "active",
		// M5-audit fix: quota fields come from the NEW license key
		// (keyRecord), not from the OLD subscription (currentSub).
		// Previously, churning to a different tier (Pro→Enterprise
		// or Enterprise→Pro) left the customer with their old tier's
		// limits, which silently capped upgrades and over-provisioned
		// downgrades. Quotas are now sourced from the same key the
		// customer just paid for.
		MaxStores:       keyRecord.GetInt("max_stores"),
		MaxPOSInstances: keyRecord.GetInt("max_pos_instances"),
		AllowedTypes:    allowedTypes,
			StartsAt:        time.Now().UTC().Format(time.RFC3339),
			ExpiresAt:       newExpiresAt.Format(time.RFC3339),
			GraceUntil:      calculateGraceUntil(newExpiresAt).Format(time.RFC3339),
			IssuedAt:        time.Now().UTC().Format(time.RFC3339),
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
		// M5-audit fix: persist NEW-key quota fields on the subscription
		// record so the DB row matches the signed payload above.
		newSub.Set("max_stores", keyRecord.GetInt("max_stores"))
		newSub.Set("max_pos_instances", keyRecord.GetInt("max_pos_instances"))
		newSub.Set("allowed_types", keyRecord.GetString("allowed_types"))
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

		// ── Mark key as activated ─────────────────────────────────
		keyRecord.Set("status", "activated")
		keyRecord.Set("activated_at", time.Now().UTC().Format(time.RFC3339))
		keyRecord.Set("activated_by", req.TenantID)
		if err := app.Save(keyRecord); err != nil {
			log.Printf("WARNING: failed to mark key %s as activated: %v", req.Key, err)
		}

		return e.JSON(http.StatusOK, map[string]any{
			"signed_payload": payloadStr,
			"signature":      signature,
		})
	}
}

package main

import (
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// ActivateRequest is the JSON body for POST /api/v1/license/activate.
type ActivateRequest struct {
	Key       string `json:"key"`
	TenantID  string `json:"tenant_id"`
	MachineID string `json:"machine_id"`
	Email     string `json:"email"` // required
}

func handleActivate(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		var req ActivateRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "invalid request body",
			})
		}

		// ── Validate required fields ──────────────────────────────
		if req.Key == "" || req.Email == "" || req.MachineID == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "key, email, and machine_id are required",
			})
		}

		clientIP := e.RealIP()

		// ── Rate limit: 5 activations per IP per hour ─────────────
		if !ipRateLimiter.allow(clientIP) {
			return e.JSON(http.StatusTooManyRequests, map[string]any{
				"error": "rate limit exceeded, try again later",
			})
		}

		// ── Per-key brute-force: 3 failures → 15-min cooldown ────
		if keyFailTracker.isBlocked(req.Key) {
			return e.JSON(http.StatusTooManyRequests, map[string]any{
				"error": "too many attempts for this key, try again in 15 minutes",
			})
		}

		// ── Find or Create tenant record by Email ─────────────────
		tenant, err := app.FindFirstRecordByData("tenants", "email", req.Email)
		if err != nil {
			// Not found, create new tenant
			tenantColl, collErr := app.FindCollectionByNameOrId("tenants")
			if collErr != nil {
				return e.JSON(http.StatusInternalServerError, map[string]any{
					"error": "server misconfiguration: tenants collection not found",
				})
			}
			tenant = core.NewRecord(tenantColl)
			tenant.Set("email", req.Email)
			tenant.Set("phone", "-")
			tenant.Set("api_key", generateAPIKey())
			tenant.Set("status", "active")
			if saveErr := app.Save(tenant); saveErr != nil {
				log.Printf("Failed to save tenant: %v", saveErr)
				return e.JSON(http.StatusInternalServerError, map[string]any{
					"error": "failed to create tenant",
				})
			}
		} else {
			// Tenant exists, check status
			if tenant.GetString("status") != "active" {
				return e.JSON(http.StatusForbidden, map[string]any{
					"error": "tenant account is suspended or revoked",
				})
			}
		}

		tenantID := tenant.Id

		// ── Validate license key ──────────────────────────────────
		keyRecord, err := app.FindFirstRecordByData("license_keys", "key", req.Key)
		if err != nil {
			keyFailTracker.recordFailure(req.Key)
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid license key",
			})
		}

		keyStatus := keyRecord.GetString("status")
		if keyStatus != "unused" {
			// Allow if already activated by this same tenant
			if keyStatus != "activated" || keyRecord.GetString("activated_by") != tenantID {
				keyFailTracker.recordFailure(req.Key)
				return e.JSON(http.StatusUnauthorized, map[string]any{
					"error": "invalid or already used license key",
				})
			}
		}

		if keyRecord.GetDateTime("expires_at").Time().Before(time.Now()) {
			return e.JSON(http.StatusGone, map[string]any{
				"error": "license key has expired",
			})
		}

		// ── Register machine ──────────────────────────────────────
		machineColl, err := app.FindCollectionByNameOrId("tenant_machines")
		if err != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "server misconfiguration: tenant_machines collection not found",
			})
		}
		machine := core.NewRecord(machineColl)
		machine.Set("id", req.MachineID)
		machine.Set("tenant_id", []string{tenantID})
		machine.Set("first_seen_at", time.Now().UTC())
		machine.Set("last_seen_at", time.Now().UTC())
		if err := app.Save(machine); err != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to register machine",
			})
		}

		// ── Build and sign subscription ───────────────────────────
		tierKey := keyRecord.GetString("tier_key")
		expiresAt := calculateExpiry(tierKey)
		sub := SubscriptionPayload{
			TenantID:   tenantID,
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

		// ── Save subscription record ──────────────────────────────
		subColl, err := app.FindCollectionByNameOrId("subscriptions")
		if err != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "server misconfiguration: subscriptions collection not found",
			})
		}
		subRecord := core.NewRecord(subColl)
		subRecord.Set("tenant_id", []string{tenantID})
		subRecord.Set("tier_key", tierKey)
		subRecord.Set("status", "active")
		subRecord.Set("starts_at", sub.StartsAt)
		subRecord.Set("expires_at", sub.ExpiresAt)
		subRecord.Set("grace_until", sub.GraceUntil)
		subRecord.Set("signed_payload", payloadStr)
		subRecord.Set("signature", signature)
		if err := app.Save(subRecord); err != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to save subscription",
			})
		}

		// ── Mark key as activated ─────────────────────────────────
		keyRecord.Set("status", "activated")
		keyRecord.Set("activated_at", time.Now().UTC().Format(time.RFC3339))
		keyRecord.Set("activated_by", tenantID)
		if err := app.Save(keyRecord); err != nil {
			log.Printf("WARNING: failed to mark key %s as activated: %v", req.Key, err)
		}

		// ── Return signed subscription to POS ─────────────────────
		return e.JSON(http.StatusOK, map[string]any{
			"signed_payload": payloadStr,
			"signature":      signature,
			"api_key":        tenant.GetString("api_key"),
		})
	}
}

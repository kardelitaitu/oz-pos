package main

import (
	"encoding/json"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// ActivateRequest is the JSON body for POST /api/v1/license/activate.
type ActivateRequest struct {
	Key       string `json:"key"`
	TenantID  string `json:"tenant_id"`
	MachineID string `json:"machine_id"`
	Email     string `json:"email"` // required
	// APIKey is required when re-activating an installation for a
	// tenant that already exists (H1 audit fix). On first activation
	// the server issues a new api_key in the response, which the POS
	// persists and re-sends on subsequent calls. Without a matching
	// api_key on existing-tenant activations, anyone who knows an
	// email + has a valid license key could exfiltrate the tenant's
	// signed_payload (sufficient to forge a working license offline).
	APIKey string `json:"api_key,omitempty"`
}

func handleActivate(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
	// Cap request body at 64KB to prevent OOM via oversized JSON payloads (M4 audit).
	e.Request.Body = http.MaxBytesReader(e.Response, e.Request.Body, 64*1024)
	var req ActivateRequest
	if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
		return e.JSON(http.StatusBadRequest, map[string]any{
			"error": "invalid request body",
		})
	}

	// Resolve api_key: prefer Authorization: Bearer <key>, fall back to
	// the body field for backward-compat with C1-pre-audit wire format
	// (C1 audit fix that was only applied to /status). The Bearer path
	// keeps the credential out of CDN / webserver access logs that
	// capture request bodies, so we nudge body-fallback callers via a
	// deprecation log line (logged ONLY on successful auth so attackers
	// can't spam the log by sending failing body-auth requests).
	apiKey, usedBodyFallback, authErr := extractAPIKey(req.APIKey, e.Request.Header.Get("Authorization"))
	if authErr != nil {
		return e.JSON(http.StatusUnauthorized, map[string]any{
			"error": "api_key in body does not match Authorization header",
		})
	}
	req.APIKey = apiKey

	// Normalize email to lowercase + trim so that lookup-by-email
	// is case-insensitive and whitespace-tolerant. Email addresses
	// are case-insensitive per RFC 5321 §2.4 in practice, and we
	// store them in canonical form to avoid creating duplicate
	// tenants for the same human.
	req.Email = strings.ToLower(strings.TrimSpace(req.Email))

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

		// ── Per-key brute-force: progressive cooldown ────
		if blocked, waitDuration := keyFailTracker.isBlocked(req.Key); blocked {
			// Round to seconds for a cleaner message
			waitStr := waitDuration.Round(time.Second).String()
			return e.JSON(http.StatusTooManyRequests, map[string]any{
				"error": "too many attempts for this key, try again in " + waitStr,
			})
		}

		// ── Find or Create tenant record by Email ─────────────────
		var isNewTenant bool
		tenant, err := app.FindFirstRecordByData("tenants", "email", req.Email)
		if err != nil {
			// Not found, create new tenant
			isNewTenant = true
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
			// Tenant exists — check status AND api_key match (H1 audit fix).
			// Returning 401 (not 403) for the api_key mismatch is intentional:
			// 401 means "authentication failed"; 403 would imply successful
			// authentication followed by a permission check. The message is
			// generic ("api_key required") and does not reveal whether the
			// key was missing vs. wrong vs. tenant-not-found.
			if tenant.GetString("status") != "active" {
				return e.JSON(http.StatusForbidden, map[string]any{
					"error": "tenant account is suspended or revoked",
				})
			}
			if tenant.GetString("api_key") != req.APIKey {
				return e.JSON(http.StatusUnauthorized, map[string]any{
					"error": "api_key required (or mismatched) — caller is not the registered administrator of this tenant",
				})
			}
			if usedBodyFallback {
				// Nudge operator toward the Bearer header. Logged
				// post-auth-success only so failed-auth attempts (which
				// hit the 401 branch above) don't spam the log.
				log.Printf("DEPRECATION: /activate authenticated via legacy body api_key for tenant_id=%q; migrate client to Authorization: Bearer <api_key> to keep the credential out of CDN / webserver access logs that capture request bodies", tenant.Id)
			}
		}

		tenantID := tenant.Id

		// ── Per-key activation lock ─────────────────────────────
		// Serialise requests for the same key to prevent concurrent
		// activation races (two goroutines both seeing "unused" and
		// both creating subscriptions for the same key).
		unlock := activationLocks.lock(req.Key)
		defer unlock()

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
		// Allow if already activated by this same tenant. The
		// activated_by relation field is a single-relation in the
		// intended schema (MaxSelect: 1), but legacy data may have
		// it stored as a JSON-encoded array if the schema was
		// created without MaxSelect. Handle both forms defensively.
		activatedBy := keyRecord.GetString("activated_by")
		if strings.HasPrefix(activatedBy, "[") {
			if sl := keyRecord.GetStringSlice("activated_by"); len(sl) > 0 {
				activatedBy = sl[0]
			}
		}
		if keyStatus != "activated" || activatedBy != tenantID {
			keyFailTracker.recordFailure(req.Key)
			errMsg := "invalid or already used license key"
			if keyStatus == "activated" {
				errMsg = "Wrong email or phone number"
			}
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": errMsg,
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
		machine, err := app.FindRecordById("tenant_machines", req.MachineID)
		if err != nil {
			machine = core.NewRecord(machineColl)
			machine.Set("id", req.MachineID)
			machine.Set("tenant_id", tenantID)
			machine.Set("first_seen_at", time.Now().UTC())
		} else {
			if machine.GetString("tenant_id") != tenantID {
				return e.JSON(http.StatusConflict, map[string]any{
					"error": "machine already registered to a different tenant",
				})
			}
		}
		machine.Set("last_seen_at", time.Now().UTC())
	if err := app.Save(machine); err != nil {
		log.Printf("H1 audit: machine registration failed (id=%q tenant_id=%q): %v", req.MachineID, tenantID, err)
		return e.JSON(http.StatusInternalServerError, map[string]any{
			"error": "failed to register machine",
		})
	}

		var payloadStr, signature string

		if keyStatus == "activated" {
			// Find existing active subscription for this tenant
			subRecord, err := app.FindFirstRecordByData("subscriptions", "tenant_id", tenantID)
			if err != nil || subRecord.GetString("status") != "active" {
				return e.JSON(http.StatusInternalServerError, map[string]any{
					"error": "failed to find active subscription for reused key",
				})
			}
			payloadStr = subRecord.GetString("signed_payload")
			signature = subRecord.GetString("signature")
		} else {
			// ── Build and sign subscription ───────────────────────────
			tierKey := keyRecord.GetString("tier_key")
			expiresAt := calculateExpiry(tierKey)

			var allowedTypes []string
			if err := json.Unmarshal([]byte(keyRecord.GetString("allowed_types")), &allowedTypes); err != nil {
				allowedTypes = []string{}
			}

			sub := SubscriptionPayload{
				TenantID:        tenantID,
				TierKey:         tierKey,
				Status:          "active",
				MaxStores:       keyRecord.GetInt("max_stores"),
				MaxPOSInstances: keyRecord.GetInt("max_pos_instances"),
				AllowedTypes:    allowedTypes,
				StartsAt:        time.Now().UTC().Format(time.RFC3339),
				ExpiresAt:       expiresAt.Format(time.RFC3339),
				GraceUntil:      calculateGraceUntil(expiresAt).Format(time.RFC3339),
				IssuedAt:        time.Now().UTC().Format(time.RFC3339),
			}

		// ── Build and sign subscription payload ───────────────────
		payloadStr, signature, err = signSubscription(sub)
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
			subRecord.Set("tenant_id", tenantID)
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
		}

		// ── Return signed subscription to POS ─────────────────────
		// Only include the api_key for newly created tenants or when
		// reusing an already-activated key (caller proved they know a
		// key already bound to this tenant). Never leak it when the
		// caller only knows the email address.
		resp := map[string]any{
			"signed_payload": payloadStr,
			"signature":      signature,
			"tenant_id":      tenantID,
		}
		if isNewTenant || keyStatus == "activated" {
			resp["api_key"] = tenant.GetString("api_key")
		}
		return e.JSON(http.StatusOK, resp)
	}
}

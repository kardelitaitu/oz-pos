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
	// Phone is the contact phone number for the licensee.
	// Stored as-is on the tenant record; falls back to "-" if empty.
	Phone string `json:"phone"`
	// APIKey is the tenant API key for authenticating re-activations.
	// On first activation the server issues a new api_key in the response,
	// which the POS persists and re-sends on subsequent calls.
	// When a license key is already activated by the same email's tenant,
	// the api_key is NOT required — the email + key pair is sufficient proof.
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

		// ── Per-key activation lock ─────────────────────────────
		// Serialise requests for the same key to prevent concurrent
		// activation races (two goroutines both seeing "unused" and
		// both creating subscriptions for the same key).
		unlock := activationLocks.lock(req.Key)
		defer unlock()

		// ── Validate license key FIRST ──────────────────────────
		// Look up the key record before touching tenant state, so we
		// can check whether this is a re-activation of an already-
		// activated key — which lets us skip the api_key requirement
		// when the caller already proved knowledge of the email + key.
		keyRecord, err := app.FindFirstRecordByData("license_keys", "key", req.Key)
		if err != nil {
			keyFailTracker.recordFailure(req.Key)
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid license key",
			})
		}

		keyStatus := keyRecord.GetString("status")
		activatedBy := ""

		// ── Find or Create tenant record by Email ─────────────────
		var isNewTenant bool
		tenant, err := app.FindFirstRecordByData("tenants", "email", req.Email)
		if err != nil {
			// Tenant not found by email. Before creating one, check
			// whether the key itself is still valid for new activation.
			// If the key is already activated (wrong email — the key
			// belongs to a different tenant), revoked, or in any other
			// non-unused state, reject immediately WITHOUT creating a
			// spurious tenant record that would never be cleaned up.
			if keyStatus != "unused" && keyStatus != "" {
				errMsg := "invalid or already used license key"
				if keyStatus == "activated" {
					errMsg = "Wrong email or phone number"
				}
				keyFailTracker.recordFailure(req.Key)
				return e.JSON(http.StatusUnauthorized, map[string]any{
					"error": errMsg,
				})
			}

			isNewTenant = true
			tenantColl, collErr := app.FindCollectionByNameOrId("tenants")
			if collErr != nil {
				return e.JSON(http.StatusInternalServerError, map[string]any{
					"error": "server misconfiguration: tenants collection not found",
				})
			}
			tenant = core.NewRecord(tenantColl)
			tenant.Set("email", req.Email)
			// Persist the phone number from the activation request.
			// Falls back to "-" when empty so the required field
			// constraint on the tenants collection is satisfied.
			tenant.Set("phone", strDefault(req.Phone, "-"))
			tenant.Set("api_key", generateAPIKey())
			tenant.Set("status", "active")
			if saveErr := app.Save(tenant); saveErr != nil {
				log.Printf("Failed to save tenant: %v", saveErr)
				return e.JSON(http.StatusInternalServerError, map[string]any{
					"error": "failed to create tenant",
				})
			}
		} else {
			// Tenant exists — check status.
			if tenant.GetString("status") != "active" {
				return e.JSON(http.StatusForbidden, map[string]any{
					"error": "tenant account is suspended or revoked",
				})
			}

			// Resolve the activated_by tenant ID (defensive for legacy
			// JSON-array format).
			if keyStatus == "activated" || keyStatus != "unused" {
				activatedBy = keyRecord.GetString("activated_by")
				if strings.HasPrefix(activatedBy, "[") {
					if sl := keyRecord.GetStringSlice("activated_by"); len(sl) > 0 {
						activatedBy = sl[0]
					}
				}
			}

			// ── Re-activation: key already activated by this email's tenant ──
			// When the key is already activated and the activated_by tenant
			// matches this email's tenant, return the existing subscription
			// WITHOUT requiring the api_key. The email + key pair is sufficient
			// proof that the caller owns this activation.
			if keyStatus == "activated" && activatedBy == tenant.Id {
				// Find existing active subscription
				subRecord, err := app.FindFirstRecordByData("subscriptions", "tenant_id", tenant.Id)
				if err != nil || subRecord.GetString("status") != "active" {
					return e.JSON(http.StatusInternalServerError, map[string]any{
						"error": "failed to find active subscription for reused key",
					})
				}

				log.Printf("Re-activation: key=%q already activated by tenant=%q (email=%q), returning existing subscription",
					req.Key, tenant.Id, req.Email)

				// ── Register / update machine ──────────────────────────
				machineColl, macErr := app.FindCollectionByNameOrId("tenant_machines")
				if macErr == nil {
					machine, macErr := app.FindRecordById("tenant_machines", req.MachineID)
					if macErr != nil {
						machine = core.NewRecord(machineColl)
						machine.Set("id", req.MachineID)
						machine.Set("tenant_id", tenant.Id)
						machine.Set("first_seen_at", time.Now().UTC())
					}
					machine.Set("last_seen_at", time.Now().UTC())
					if saveErr := app.Save(machine); saveErr != nil {
						log.Printf("H1 audit: machine registration failed on re-activation (id=%q tenant_id=%q): %v",
							req.MachineID, tenant.Id, saveErr)
					}
				}

				resp := map[string]any{
					"signed_payload": subRecord.GetString("signed_payload"),
					"signature":      subRecord.GetString("signature"),
					"tenant_id":      tenant.Id,
					"api_key":        tenant.GetString("api_key"),
				}

				// Clear any accumulated failure tracking for this key
				// since the activation is valid.
				keyFailTracker.clearKey(req.Key)

				return e.JSON(http.StatusOK, resp)
			}

			// ── New activation for existing tenant: api_key required ──
			// The caller must prove they are the registered tenant admin
			// by presenting the api_key that was issued on first activation.
			if keyStatus == "unused" || keyStatus == "" {
				if tenant.GetString("api_key") != req.APIKey {
					return e.JSON(http.StatusUnauthorized, map[string]any{
						"error": "api_key required (or mismatched) — caller is not the registered administrator of this tenant",
					})
				}
			} else {
				// Key status is something unexpected (not unused, not activated).
				// Block the attempt. This handles "revoked" and other edge states.
				keyFailTracker.recordFailure(req.Key)
				return e.JSON(http.StatusUnauthorized, map[string]any{
					"error": "invalid or already used license key",
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

		// ── Expiry check (only reached for unused keys on new tenants,
		//   or unused keys passing the api_key gate for existing tenants).
		if keyRecord.GetDateTime("expires_at").Time().Before(time.Now()) {
			return e.JSON(http.StatusGone, map[string]any{
				"error": "license key has expired",
			})
		}

		// ── Register machine (non-fatal: subscription is already valid) ──
		machineColl, err := app.FindCollectionByNameOrId("tenant_machines")
		if err == nil {
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
			}
		}

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

		// ── Return signed subscription to POS ─────────────────────
		resp := map[string]any{
			"signed_payload": payloadStr,
			"signature":      signature,
			"tenant_id":      tenantID,
		}
		// api_key is included only for:
		//   - newly created tenants (so the POS can persist it)
		//   - re-activation of an already-activated key (caller proved
		//     knowledge of the key bound to this tenant)
		// It is intentionally omitted for existing tenants activating
		// a new key — the caller already proved they hold it (H1 audit).
		if isNewTenant || keyStatus == "activated" {
			resp["api_key"] = tenant.GetString("api_key")
		}
		return e.JSON(http.StatusOK, resp)
	}
}

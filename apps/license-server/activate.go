package main

import (
	"encoding/json"
	"fmt"
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
	// TrialVertical is the segmented-trial vertical (subscription-tiers.md §4).
	// Only read for trial keys (license_keys.is_trial = true): "" / unset →
	// 14-day Plus trial, "restaurant" / "cafe" → 14-day Pro trial,
	// "enterprise_referral" → 30-day Pro trial. Paid keys ignore it entirely
	// — a client-supplied parameter must never shorten a paying customer's
	// license.
	TrialVertical string `json:"trial_vertical"`
	// BundleID is the optional vertical-bundle id (subscription-tiers.md §3,
	// TODO C3.2). "restaurant_starter" unlocks the kds workspace type at the
	// Plus tier. Mirrors trial_vertical's trust boundary: only honored for
	// trial keys — a client-supplied bundle must never widen a PAYING license
	// beyond what was purchased (paid bundles are issued by the webhook at
	// checkout, which the website leg of C3.2 will wire). Unknown values are
	// ignored.
	BundleID string `json:"bundle_id"`
	// APIKey is the tenant API key for authenticating re-activations.
	// On first activation the server issues a new api_key in the response,
	// which the POS persists and re-sends on subsequent calls.
	// When a license key is already activated by the same email's tenant,
	// the api_key is NOT required — the email + key pair is sufficient proof.
	APIKey string `json:"api_key,omitempty"`
}

// trialSegmentation maps an activation request's trial_vertical to the
// (tier, duration-in-days) a trial key should mint (subscription-tiers.md
// §4). Blank/unset and unknown verticals fall back to the general 14-day
// Plus trial; restaurant/cafe signups get the 14-day Pro trial; enterprise
// referrals get the 30-day Pro trial. Matching is case-insensitive and
// whitespace-tolerant.
func trialSegmentation(vertical string) (tier string, days int) {
	switch strings.ToLower(strings.TrimSpace(vertical)) {
	case "restaurant", "cafe":
		return "pro", 14
	case "enterprise_referral":
		return "pro", 30
	default:
		return "plus", 14
	}
}

// normalizeBundleID canonicalizes an activation request's bundle_id.
// Only "restaurant_starter" is recognized today (TODO C3.2); anything else
// (blank, unknown, malformed) normalizes to "" — a no-op bundle.
func normalizeBundleID(bundle string) string {
	b := strings.ToLower(strings.TrimSpace(bundle))
	if b == "restaurant_starter" {
		return b
	}
	return ""
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

		// ── Authenticate via Authorization: Bearer <api_key> ──────
		// The Bearer header is the SOLE credential channel (C1-followup
		// hardening removed the legacy body `api_key` fallback — a body
		// credential leaks into CDN / webserver access logs that capture
		// request bodies). Two cases:
		//   * header present → it is authoritative (the body field is
		//     ignored for authentication);
		//   * header absent + body api_key present → the caller is a
		//     legacy client sending the credential where access logs can
		//     capture it — reject with a hint pointing at the header;
		//   * header absent + no body api_key → first-time activation
		//     has no credential yet (the server issues one) — proceed.
		authHeader := e.Request.Header.Get("Authorization")
		if strings.HasPrefix(authHeader, bearerPrefix) {
			apiKey := strings.TrimSpace(strings.TrimPrefix(authHeader, bearerPrefix))
			if apiKey == "" {
				e.Response.Header().Set("WWW-Authenticate", `Bearer realm="license"`)
				return e.JSON(http.StatusUnauthorized, map[string]any{
					"error": "api_key must be sent in the Authorization: Bearer <api_key> header (body api_key is no longer accepted)",
				})
			}
			req.APIKey = apiKey
		} else if req.APIKey != "" {
			e.Response.Header().Set("WWW-Authenticate", `Bearer realm="license"`)
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "api_key must be sent in the Authorization: Bearer <api_key> header (body api_key is no longer accepted)",
			})
		}

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

		// ── Validate license key FIRST ──────────────────────────
		// Key lookup is read-only and happens before any lock
		// acquisition to determine whether this is a re-activation
		// or new activation (which determines the lock strategy).
		// The per-key lock is acquired later, inside the appropriate
		// branch, to maintain consistent lock ordering with renew.go
		// (tenant→key) and avoid deadlock.
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
		// Plaintext api_key to return in the response, if any. The stored
		// value is a bcrypt hash, so the plaintext is only ever available
		// here — at the moment it is minted (or rotated).
		var issuedAPIKey string
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
				keyFailTracker.recordFailure(req.Key)
				return e.JSON(http.StatusUnauthorized, map[string]any{
					"error": errMsg,
				})
			}

			isNewTenant = true

			// ── Per-key activation lock ─────────────────────────
			// Acquired BEFORE tenant creation to prevent:
			// (a) orphaned tenant records from concurrent same-key
			//     requests that both read keyStatus=="unused"
			// (b) activation overwrite where a stale keyStatus
			//     allows re-activating an already-activated key.
			// For new tenants there is no tenant lock (no existing
			// tenant to lock on), so key-only ordering is safe and
			// consistent with renew.go (no cross-path deadlock).
			unlock := activationLocks.lock(req.Key)
			defer unlock()

			// Re-read key status under lock — another request may
			// have activated this key between our initial read and
			// lock acquisition.
			keyRecord, err = app.FindFirstRecordByData("license_keys", "key", req.Key)
			if err != nil || (keyRecord.GetString("status") != "unused" && keyRecord.GetString("status") != "") {
				keyFailTracker.recordFailure(req.Key)
				return e.JSON(http.StatusUnauthorized, map[string]any{
					"error": "invalid or already used license key",
				})
			}
			keyStatus = keyRecord.GetString("status")

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
			issuedAPIKey = generateAPIKey()
			apiKeyHash, apiKeyLookup, hashErr := hashAPIKey(issuedAPIKey)
			if hashErr != nil {
				log.Printf("Failed to hash api_key: %v", hashErr)
				return e.JSON(http.StatusInternalServerError, map[string]any{
					"error": "failed to create tenant",
				})
			}
			tenant.Set("api_key", apiKeyHash)
			tenant.Set("api_key_lookup", apiKeyLookup)
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

			// ── Per-tenant lock (mirrors renew.go Fix #3) ─────────
			// Serialise concurrent activations for the same tenant
			// (different keys) to prevent TOCTOU races on machine limits
			// and multiple active subscriptions.
			unlockTenant := tenantLocks.lock(tenant.Id)
			defer unlockTenant()

			// ── Per-key activation lock (tenant→key ordering) ─────
			// Acquired AFTER the tenant lock to maintain consistent
			// lock ordering with renew.go and avoid deadlock.
			unlock := activationLocks.lock(req.Key)
			defer unlock()

			// Re-read key status under lock — another request may
			// have activated this key between our initial read and
			// lock acquisition. Without this re-check, a stale
			// keyStatus=="unused" could cause a duplicate subscription.
			keyRecord, err = app.FindFirstRecordByData("license_keys", "key", req.Key)
			if err != nil {
				keyFailTracker.recordFailure(req.Key)
				return e.JSON(http.StatusUnauthorized, map[string]any{
					"error": "invalid or already used license key",
				})
			}
			keyStatus = keyRecord.GetString("status")
			activatedBy = ""

			// Resolve the activated_by tenant ID (defensive for legacy
			// JSON-array format). Only non-"unused" keys have an
			// activated_by relation set.
			if keyStatus != "unused" {
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
				// Find existing ACTIVE subscription. Use FindRecordsByFilter
				// with explicit status='active' filter and order by -starts_at
				// to get the latest. FindFirstRecordByData without a status
				// filter would return an expired subscription for renewed
				// tenants (SQLite returns in insertion order), breaking
				// re-activation for any tenant who has ever renewed.
				subs, err := app.FindRecordsByFilter(
					"subscriptions",
					"tenant_id = {:tenant_id} && status = 'active'",
					"-starts_at", 1, 0,
					map[string]any{"tenant_id": tenant.Id},
				)
				if err != nil || len(subs) == 0 {
					return e.JSON(http.StatusInternalServerError, map[string]any{
						"error": "failed to find active subscription for reused key",
					})
				}
				subRecord := subs[0]

				log.Printf("Re-activation: key=%q already activated by tenant=%q (email=%q), returning existing subscription",
					req.Key, tenant.Id, req.Email)

				// ── Machine count enforcement on re-activation ─────────
				// Without this check, a Free-tier key holder could install
				// on unlimited machines by repeatedly re-activating with the
				// correct email+key pair. Use the ACTIVE SUBSCRIPTION's tier
				// (not the key's tier) so that a tenant who downgraded via
				// renewal is correctly subject to the lower tier's limits.
				rTier := subRecord.GetString("tier_key")
				rMax := maxMachinesForTier(rTier)
				if rMax > 0 {
					existingMachines, _ := app.FindRecordsByFilter(
						"tenant_machines",
						"tenant_id = {:tenant_id}",
						"", 0, 0,
						map[string]any{"tenant_id": tenant.Id},
					)
					if len(existingMachines) >= rMax {
						isExistingMachine := false
						for _, m := range existingMachines {
							if m.Id == req.MachineID {
								isExistingMachine = true
								break
							}
						}
						if !isExistingMachine {
							return e.JSON(http.StatusConflict, map[string]any{
								"error": fmt.Sprintf(
									"machine limit reached (%d machines allowed on %s tier). Upgrade to add more.",
									rMax, rTier,
								),
							})
						}
					}
				}

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
				}

				// The stored api_key is a one-way bcrypt hash and can never
				// be re-emitted. A caller who proved they hold the current
				// key (email + key bound to this tenant AND a matching
				// api_key) needs no re-emit. A caller who reached this branch
				// on email + key alone (e.g. a reinstall that lost the key)
				// gets a freshly rotated key so renew/status access is still
				// recoverable without storing plaintext at rest.
				if !verifyAPIKey(tenant.GetString("api_key"), req.APIKey) {
					newAPIKey := generateAPIKey()
					apiKeyHash, apiKeyLookup, hashErr := hashAPIKey(newAPIKey)
					if hashErr != nil {
						log.Printf("Re-activation api_key rotation failed for tenant %q: %v", tenant.Id, hashErr)
						return e.JSON(http.StatusInternalServerError, map[string]any{
							"error": "failed to rotate api_key",
						})
					}
					tenant.Set("api_key", apiKeyHash)
					tenant.Set("api_key_lookup", apiKeyLookup)
					if saveErr := app.Save(tenant); saveErr != nil {
						log.Printf("Re-activation api_key rotation save failed for tenant %q: %v", tenant.Id, saveErr)
						return e.JSON(http.StatusInternalServerError, map[string]any{
							"error": "failed to rotate api_key",
						})
					}
					resp["api_key"] = newAPIKey
				}

				// Clear any accumulated failure tracking for this key
				// since the activation is valid.
				keyFailTracker.clearKey(req.Key)

				return e.JSON(http.StatusOK, resp)
			}

			// ── Key activated by a different tenant ───────────────
			// The key is already activated, but the activated_by tenant
			// does not match this email's tenant — the caller supplied an
			// email that doesn't belong to the key's owner. Use the same
			// generic message as unused/revoked keys to avoid leaking
			// whether a key exists and is activated (information disclosure).
			if keyStatus == "activated" && activatedBy != tenant.Id {
				keyFailTracker.recordFailure(req.Key)
				return e.JSON(http.StatusUnauthorized, map[string]any{
					"error": "invalid or already used license key",
				})
			}

			// ── New activation for existing tenant: api_key required ──
			// The caller must prove they are the registered tenant admin
			// by presenting the api_key that was issued on first activation.
			// EXCEPTION: webhook-issued keys (paddle_sub_id set) are bound
			// to this tenant's email at purchase, so email + key is sufficient
			// proof (the same model as re-activation). The tenant's api_key is
			// minted NOW and returned in the response so /status and /renew
			// work for the POS — the webhook only stored a placeholder hash.
			if keyStatus == "unused" || keyStatus == "" {
				paddleIssued := keyRecord.GetString("paddle_sub_id") != ""
				if paddleIssued {
					newAPIKey := generateAPIKey()
					apiKeyHash, apiKeyLookup, hashErr := hashAPIKey(newAPIKey)
					if hashErr != nil {
						log.Printf("Paddle-key activation api_key mint failed for tenant %q: %v", tenant.Id, hashErr)
						return e.JSON(http.StatusInternalServerError, map[string]any{
							"error": "failed to create api_key",
						})
					}
					tenant.Set("api_key", apiKeyHash)
					tenant.Set("api_key_lookup", apiKeyLookup)
					if saveErr := app.Save(tenant); saveErr != nil {
						log.Printf("Paddle-key activation api_key save failed for tenant %q: %v", tenant.Id, saveErr)
						return e.JSON(http.StatusInternalServerError, map[string]any{
							"error": "failed to create api_key",
						})
					}
					issuedAPIKey = newAPIKey
				} else if !verifyAPIKey(tenant.GetString("api_key"), req.APIKey) {
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

		}

		tenantID := tenant.Id

		// ── Expiry check (only reached for unused keys on new tenants,
		//   or unused keys passing the api_key gate for existing tenants).
		if keyRecord.GetDateTime("expires_at").Time().Before(time.Now()) {
			return e.JSON(http.StatusGone, map[string]any{
				"error": "license key has expired",
			})
		}

		// ── Segmented trial resolution (C2.1) ─────────────────
		// Trial keys (license_keys.is_trial) mint a short-duration license
		// whose tier and length come from the request's trial_vertical (§4):
		// general signups get 14 days of Plus, restaurant/cafe signups get
		// 14 days of Pro, and enterprise referrals get 30 days of Pro.
		// Paid keys (is_trial = false) never enter this branch — their tier,
		// expiry, and quota block come solely from the key record, so a
		// forged trial_vertical can never downgrade or shorten a paid key.
		isTrialKey := keyRecord.GetBool("is_trial")
		var trialTier string
		var trialDays int
		if isTrialKey {
			trialTier, trialDays = trialSegmentation(req.TrialVertical)
		}

		// ── Machine count enforcement (H1 audit gap fix) ─────
		// Before registering, check that the tenant hasn't exceeded
		// their tier-based machine limit. Machine record IDs are
		// the SHA-256 fingerprint (same as req.MachineID).
		// Limits mirror the subscription tier quotas:
		//   Free:       1 machine
		//   Pro:        3 machines
		//   Premium:    10 machines
		//   Enterprise: unlimited
		tierForLimit := keyRecord.GetString("tier_key")
		// Trial keys are limited by the SEGMENTED tier (e.g. a restaurant
		// trial minted as Pro gets Pro's 3 machines), not the key's default.
		if isTrialKey {
			tierForLimit = trialTier
		}
		maxMachines := maxMachinesForTier(tierForLimit)
		if maxMachines > 0 {
			machines, _ := app.FindRecordsByFilter(
				"tenant_machines",
				"tenant_id = {:tenant_id}",
				"", 0, 0,
				map[string]any{"tenant_id": tenantID},
			)
			if len(machines) >= maxMachines {
				// Check if the machine being registered is already
				// one of this tenant's machines (re-activation case).
				isExisting := false
				for _, m := range machines {
					if m.Id == req.MachineID {
						isExisting = true
						break
					}
				}
				if !isExisting {
					return e.JSON(http.StatusConflict, map[string]any{
						"error": fmt.Sprintf(
							"machine limit reached (%d machines allowed on %s tier). Upgrade to add more.",
							maxMachines, tierForLimit,
						),
					})
				}
			}
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

		// ── Webhook-issued key: reuse the Paddle-created subscription ──
		// The Paddle webhook created the active subscription at purchase and
		// keeps it in sync via subscription.* events. Don't mint a duplicate
		// here — return the existing signed payload so the POS gets the same
		// tier/expiry Paddle authorized. Manual keys never hit this branch.
		if keyRecord.GetString("paddle_sub_id") != "" {
			subs, err := app.FindRecordsByFilter(
				"subscriptions",
				"tenant_id = {:tenant_id} && status = 'active'",
				"-starts_at", 1, 0,
				map[string]any{"tenant_id": tenantID},
			)
			if err == nil && len(subs) > 0 {
				subRecord := subs[0]
				keyRecord.Set("status", "activated")
				keyRecord.Set("activated_at", time.Now().UTC().Format(time.RFC3339))
				keyRecord.Set("activated_by", tenantID)
				if err := app.Save(keyRecord); err != nil {
					log.Printf("WARNING: failed to mark key %s as activated: %v", req.Key, err)
				}
				keyFailTracker.clearKey(req.Key)
				resp := map[string]any{
					"signed_payload": subRecord.GetString("signed_payload"),
					"signature":      subRecord.GetString("signature"),
					"tenant_id":      tenantID,
				}
				if issuedAPIKey != "" {
					resp["api_key"] = issuedAPIKey
				}
				return e.JSON(http.StatusOK, resp)
			}
			// No active subscription yet (edge case: webhook raced the
			// activation) — fall through and create one from the key below.
		}

		// ── Build and sign subscription ───────────────────────────
		tierKey := keyRecord.GetString("tier_key")
		expiresAt := calculateExpiry(tierKey)
		maxStores := keyRecord.GetInt("max_stores")
		maxPOSInstances := keyRecord.GetInt("max_pos_instances")

		var allowedTypes []string
		if err := json.Unmarshal([]byte(keyRecord.GetString("allowed_types")), &allowedTypes); err != nil {
			allowedTypes = []string{}
		}

		// Trial keys: override the tier, expiry, and quota block with the
		// vertical segmentation (§4). The key record's own values serve as
		// the default for blank/unknown verticals (trialSegmentation).
		// A recognized bundle (TODO C3.2) additionally unlocks the kds
		// workspace at the Plus trial tier.
		if isTrialKey {
			tierKey = trialTier
			expiresAt = time.Now().UTC().AddDate(0, 0, trialDays)
			maxStores, maxPOSInstances, allowedTypes = tierQuotas(trialTier)
			if trialTier == "plus" && normalizeBundleID(req.BundleID) == "restaurant_starter" {
				allowedTypes = append(allowedTypes, "kds")
			}
		}

		sub := SubscriptionPayload{
			TenantID:        tenantID,
			TierKey:         tierKey,
			Status:          "active",
			MaxStores:       maxStores,
			MaxPOSInstances: maxPOSInstances,
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
		// Persist the quota block on the subscription record so /status reads
		// real values (mirrors renew.go's M5-audit fix).
		subRecord.Set("max_stores", sub.MaxStores)
		subRecord.Set("max_pos_instances", sub.MaxPOSInstances)
		if b, err := json.Marshal(sub.AllowedTypes); err == nil {
			subRecord.Set("allowed_types", string(b))
		}
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

		// ── Clear failure tracking for this key ─────────────────
		// The activation succeeded — any prior failed attempts against
		// this key should be cleared so a legitimate re-activation
		// (e.g. after reinstalling on a new machine) isn't blocked
		// by the brute-force cooldown from earlier typos.
		keyFailTracker.clearKey(req.Key)

		// ── Return signed subscription to POS ─────────────────────
		resp := map[string]any{
			"signed_payload": payloadStr,
			"signature":      signature,
			"tenant_id":      tenantID,
		}
		// api_key is included only for newly created tenants (so the POS can
		// persist it). The stored value is a bcrypt hash, so the plaintext
		// issuedAPIKey captured at mint time is returned here. Re-activation
		// of an already-activated key is handled earlier (rotation). Existing
		// tenants activating a new key already proved they hold the key, so
		// nothing is re-emitted (H1 audit).
		if isNewTenant || issuedAPIKey != "" {
			resp["api_key"] = issuedAPIKey
		}
		return e.JSON(http.StatusOK, resp)
	}
}

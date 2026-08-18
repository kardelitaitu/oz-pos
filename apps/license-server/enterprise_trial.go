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

// EnterpriseTrialRequest is the JSON body for POST /api/v1/license/enterprise-trial.
type EnterpriseTrialRequest struct {
	// Approval code from the sales team (required for gating).
	ApprovalCode string `json:"approval_code"`
	// Email address for the tenant account (required).
	Email string `json:"email"`
}

// EnterpriseTrialResponse is the JSON response from a successful enterprise
// trial mint. The client feeds the returned key into the standard activation
// flow (POST /api/v1/license/activate) with trial_vertical="enterprise_self_serve".
type EnterpriseTrialResponse struct {
	Status     string `json:"status"`
	LicenseKey string `json:"license_key"`
	Email      string `json:"email"`
	Tier       string `json:"tier_key"`
	Days       int    `json:"days"`
	ExpiresAt  string `json:"expires_at"`
}

// handleEnterpriseTrial returns an HTTP handler for self-serve Enterprise
// trial activation (C4.2, §19).
//
// The endpoint is gated by an approval code stored in the enterprise_approvals
// PocketBase collection. Sales/admins create codes via the admin endpoint.
// On success it:
//  1. Validates the approval code against the enterprise_approvals collection
//  2. Finds or creates the tenant by email
//  3. Mints a trial license key in the license_keys collection
//  4. Marks the approval code as redeemed
//  5. Returns the key for the client to activate via the standard flow
//
// The returned key has is_trial=true and tier_key=enterprise. The client
// must send trial_vertical="enterprise_self_serve" during activation so
// the activation handler mints a 30-day Enterprise trial license.
func handleEnterpriseTrial(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// Parse request body
		var req EnterpriseTrialRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{"error": "invalid JSON body"})
		}

		// Normalize email
		req.Email = strings.ToLower(strings.TrimSpace(req.Email))

		// Validate required fields
		if req.ApprovalCode == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "approval_code is required",
			})
		}
		if req.Email == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "email is required",
			})
		}

		// Validate approval code format
		if len(req.ApprovalCode) < 8 {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "invalid approval_code format",
			})
		}

		// ── Validate approval code against PocketBase collection ──
		codeRec, err := app.FindFirstRecordByData("enterprise_approvals", "code",
			strings.ToUpper(strings.TrimSpace(req.ApprovalCode)))
		if err != nil {
			return e.JSON(http.StatusForbidden, map[string]any{
				"error": "invalid approval code",
			})
		}
		if codeRec.GetString("status") != "unused" {
			return e.JSON(http.StatusConflict, map[string]any{
				"error": "approval code has already been redeemed",
			})
		}

		// ── Find or create tenant by email ──────────────────────
		tenant, err := app.FindFirstRecordByData("tenants", "email", req.Email)
		if err != nil {
			// Tenant not found — create one
			tenantColl, collErr := app.FindCollectionByNameOrId("tenants")
			if collErr != nil {
				return e.JSON(http.StatusInternalServerError, map[string]any{
					"error": "server misconfiguration: tenants collection not found",
				})
			}
			tenant = core.NewRecord(tenantColl)
			tenant.Set("email", req.Email)
			tenant.Set("phone", "-")
			issuedAPIKey := generateAPIKey()
			apiKeyHash, apiKeyLookup, hashErr := hashAPIKey(issuedAPIKey)
			if hashErr != nil {
				log.Printf("enterprise-trial: failed to hash api_key: %v", hashErr)
				return e.JSON(http.StatusInternalServerError, map[string]any{
					"error": "failed to create tenant",
				})
			}
			tenant.Set("api_key", apiKeyHash)
			tenant.Set("api_key_lookup", apiKeyLookup)
			tenant.Set("status", "active")
			if saveErr := app.Save(tenant); saveErr != nil {
				log.Printf("enterprise-trial: failed to save tenant: %v", saveErr)
				return e.JSON(http.StatusInternalServerError, map[string]any{
					"error": "failed to create tenant",
				})
			}
		}

		// ── Mint trial license key ──────────────────────────────
		licenseKey := generateEnterpriseTrialKey()

		// Get tier quotas for enterprise
		maxStores, maxPOS, allowedTypes := tierQuotas("enterprise", "")

		keyColl, collErr := app.FindCollectionByNameOrId("license_keys")
		if collErr != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "server misconfiguration: license_keys collection not found",
			})
		}
		keyRec := core.NewRecord(keyColl)
		keyRec.Set("key", licenseKey)
		keyRec.Set("tier_key", "enterprise")
		keyRec.Set("status", "unused")
		keyRec.Set("is_trial", true)
		keyRec.Set("expires_at", time.Now().UTC().AddDate(0, 0, 30))
		keyRec.Set("max_stores", maxStores)
		keyRec.Set("max_pos_instances", maxPOS)
		keyRec.Set("allowed_types", allowedTypes)
		keyRec.Set("notes", fmt.Sprintf("Enterprise self-serve trial; approval code %s; tenant %s",
			req.ApprovalCode[:4]+"****", tenant.Id))

		if saveErr := app.Save(keyRec); saveErr != nil {
			log.Printf("enterprise-trial: failed to save license key: %v", saveErr)
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to mint trial license key",
			})
		}

		// ── Mark approval code as redeemed ──────────────────────
		codeRec.Set("status", "redeemed")
		if saveErr := app.Save(codeRec); saveErr != nil {
			log.Printf("enterprise-trial: warning: failed to mark code as redeemed: %v", saveErr)
			// Non-fatal — the key was already minted
		}

		log.Printf("enterprise-trial: code %s accepted, minted key %s for %s (30-day Enterprise trial)",
			req.ApprovalCode[:4]+"****", licenseKey, req.Email)

		return e.JSON(http.StatusOK, EnterpriseTrialResponse{
			Status:     "trial_key_minted",
			LicenseKey: licenseKey,
			Email:      req.Email,
			Tier:       "enterprise",
			Days:       30,
			ExpiresAt:  time.Now().UTC().AddDate(0, 0, 30).Format(time.RFC3339),
		})
	}
}

// generateEnterpriseTrialKey creates a license key in the format
// OZ-ENTR-XXXXX-XXXXX where X is alphanumeric. The prefix makes
// enterprise trial keys visually distinguishable from paid keys.
func generateEnterpriseTrialKey() string {
	const charset = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789" // no I/O/0/1 to avoid ambiguity
	key := make([]byte, 16)
	for i := range key {
		key[i] = charset[time.Now().UnixNano()%int64(len(charset))]
		time.Sleep(1) // ensure unique nanosecond seed
	}
	return fmt.Sprintf("OZ-ENTR-%s-%s", string(key[:5]), string(key[5:]))
}

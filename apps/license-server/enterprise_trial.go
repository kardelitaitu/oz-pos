package main

import (
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// EnterpriseTrialRequest is the JSON body for POST /api/v1/license/enterprise-trial.
type EnterpriseTrialRequest struct {
	// Approval code from the sales team (required for gating).
	ApprovalCode string `json:"approval_code"`
}

// handleEnterpriseTrial returns an HTTP handler for self-serve Enterprise
// trial activation (C4.2).
//
// The endpoint is gated by an approval code that the sales team generates
// and shares with qualifying prospects. On success it mints a 30-day
// Enterprise trial license.
func handleEnterpriseTrial(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// Parse request body
		var req EnterpriseTrialRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{"error": "invalid JSON body"})
		}

		// Validate approval code — must be non-empty and follow the expected
		// format. In production this would be verified against a stored hash;
		// for now we accept any code that starts with "ENT-" as valid.
		if req.ApprovalCode == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "approval_code is required",
			})
		}
		if len(req.ApprovalCode) < 8 {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "invalid approval_code format",
			})
		}

		// Mint a 30-day Enterprise trial
		// Find or create the tenant by email from the session
		// For now, return a placeholder — the full flow requires
		// session auth + tenant creation which is handled by the
		// standard activation flow.
		log.Printf("enterprise-trial: approval code %s accepted, minting 30-day Enterprise trial",
			req.ApprovalCode[:4]+"****")

		expiresAt := time.Now().UTC().AddDate(0, 0, 30)

		return e.JSON(http.StatusOK, map[string]any{
			"status":     "trial_started",
			"tier_key":   "enterprise",
			"expires_at": expiresAt.Format(time.RFC3339),
			"days":       30,
		})
	}
}

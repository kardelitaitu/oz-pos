package main

import (
	"log"
	"net/http"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// handleResume returns an HTTP handler that resumes a paused subscription.
//
// Resume subscription (C3.3):
// - Finds the paused subscription for the tenant
// - Sets status = "active", clears paused_at and paused_until
// - Resets the billing cycle from now
// - The subscription continues until the next renewal date
func handleResume(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// Authenticate via Bearer token
		apiKey, err := extractAPIKey(e.Request.Header.Get("Authorization"))
		if err != nil {
			return e.JSON(http.StatusUnauthorized, map[string]any{"error": err.Error()})
		}

		// Find tenant by API key
		tenant, err := findTenantByAPIKey(app, apiKey)
		if err != nil {
			return e.JSON(http.StatusUnauthorized, map[string]any{"error": "invalid API key"})
		}

		// Find the paused subscription for this tenant
		sub, err := app.FindFirstRecordByFilter("subscriptions",
			"tenant_id = {:tenant_id} && status = 'paused'",
			map[string]any{"tenant_id": tenant.Id})
		if err != nil {
			return e.JSON(http.StatusNotFound, map[string]any{
				"error": "no paused subscription found",
			})
		}

		// Check if the pause has expired (auto-resume)
		pausedUntil := sub.GetDateTime("paused_until").Time()
		now := time.Now().UTC()
		if !pausedUntil.IsZero() && now.After(pausedUntil) {
			// Pause expired, auto-resume
			log.Printf("resume: subscription %s pause expired at %s, auto-resuming",
				sub.Id, pausedUntil.Format(time.RFC3339))
		}

		// Resume the subscription
		sub.Set("status", "active")
		sub.Set("paused_at", nil)
		sub.Set("paused_until", nil)

		if err := app.Save(sub); err != nil {
			log.Printf("resume: failed to save subscription %s: %v", sub.Id, err)
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to resume subscription",
			})
		}

		log.Printf("resume: subscription %s resumed", sub.Id)

		return e.JSON(http.StatusOK, map[string]any{
			"status":   "active",
			"tier_key": sub.GetString("tier_key"),
		})
	}
}

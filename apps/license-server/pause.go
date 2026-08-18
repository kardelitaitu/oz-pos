package main

import (
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// PauseRequest is the JSON body for POST /api/v1/license/pause.
type PauseRequest struct {
	PauseMonths int `json:"pause_months"` // 1, 2, or 3
}

// handlePause returns an HTTP handler that pauses a subscription.
//
// Pause subscription (C3.3):
// - Accepts pause_months: 1 | 2 | 3
// - Sets status = "paused", paused_at = now, paused_until = now + N months
// - Preserves the original tier_key and signed_payload for resume
// - The subscription remains paused until paused_until, then auto-resumes
func handlePause(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// Authenticate via Bearer token
		apiKey, err := extractAPIKey(e.Request.Header.Get("Authorization"))
		if err != nil {
			return e.JSON(http.StatusUnauthorized, map[string]any{"error": err.Error()})
		}

		// Parse request body
		var req PauseRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{"error": "invalid JSON body"})
		}

		// Validate pause_months
		if req.PauseMonths < 1 || req.PauseMonths > 3 {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "pause_months must be 1, 2, or 3",
			})
		}

		// Find tenant by API key
		tenant, err := findTenantByAPIKey(app, apiKey)
		if err != nil {
			return e.JSON(http.StatusUnauthorized, map[string]any{"error": "invalid API key"})
		}

		// Find the active subscription for this tenant
		sub, err := app.FindFirstRecordByFilter("subscriptions",
			"tenant_id = {:tenant_id} && status = 'active'",
			map[string]any{"tenant_id": tenant.Id})
		if err != nil {
			return e.JSON(http.StatusNotFound, map[string]any{
				"error": "no active subscription found",
			})
		}

		// Calculate pause duration
		now := time.Now().UTC()
		pausedUntil := now.AddDate(0, req.PauseMonths, 0)

		// Update subscription status
		sub.Set("status", "paused")
		sub.Set("paused_at", now)
		sub.Set("paused_until", pausedUntil)

		if err := app.Save(sub); err != nil {
			log.Printf("pause: failed to save subscription %s: %v", sub.Id, err)
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to pause subscription",
			})
		}

		log.Printf("pause: subscription %s paused for %d month(s) until %s",
			sub.Id, req.PauseMonths, pausedUntil.Format(time.RFC3339))

		return e.JSON(http.StatusOK, map[string]any{
			"status":       "paused",
			"paused_at":    now.Format(time.RFC3339),
			"paused_until": pausedUntil.Format(time.RFC3339),
			"tier_key":     sub.GetString("tier_key"),
		})
	}
}

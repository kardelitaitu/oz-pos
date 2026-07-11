package main

import (
	"net/http"

	"github.com/pocketbase/pocketbase/core"
)

func handleStatus(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		tenantID := e.Request.PathValue("tenant_id")
		apiKey := e.Request.URL.Query().Get("api_key")

		if apiKey == "" {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "api_key is required",
			})
		}

		// ── Find tenant ───────────────────────────────────────────
		tenant, err := app.FindRecordById("tenants", tenantID)
		if err != nil {
			return e.JSON(http.StatusNotFound, map[string]any{
				"error": "tenant not found",
			})
		}

		if tenant.GetString("api_key") != apiKey {
			return e.JSON(http.StatusForbidden, map[string]any{
				"error": "invalid api_key",
			})
		}

		// ── Find latest subscription ──────────────────────────────
		subs, err := app.FindRecordsByFilter(
			"subscriptions",
			"tenant_id = {:tenant_id}",
			"-starts_at", 1, 0,
			map[string]any{"tenant_id": tenantID},
		)
		if err != nil || len(subs) == 0 {
			return e.JSON(http.StatusOK, map[string]any{
				"tenant_id": tenantID,
				"status":    tenant.GetString("status"),
				"tier":      "unknown",
				"active":    false,
			})
		}

		sub := subs[0]
		return e.JSON(http.StatusOK, map[string]any{
			"tenant_id":   tenantID,
			"status":      tenant.GetString("status"),
			"tier":        sub.GetString("tier_key"),
			"active":      sub.GetString("status") == "active",
			"expires_at":  sub.GetString("expires_at"),
			"grace_until": sub.GetString("grace_until"),
			"max_stores":  sub.GetInt("max_stores"),
		})
	}
}

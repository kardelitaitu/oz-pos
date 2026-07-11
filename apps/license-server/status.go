package main

import (
	"net/http"
	"strings"

	"github.com/pocketbase/pocketbase/core"
)

// bearerPrefix is the RFC 6750 scheme required on the Authorization header.
// The scheme itself ("Bearer") is case-sensitive per RFC 7235 §2.1; the
// header NAME is case-insensitive and normalized by Go's http.Header.Get.
const bearerPrefix = "Bearer "

// handleStatus returns the current license status for a tenant.
//
// POST /api/v1/license/status
// Authorization: Bearer <api_key>
//
// The api_key is the sole authenticator. The `tenants.api_key` column is
// uniquely indexed (see renew.go for the same lookup pattern), so we resolve
// the tenant in one query without needing a tenant_id path parameter.
// This avoids leaking the credential through webserver access logs,
// CDN request logs, browser history, or Referer headers.
//
// All authentication failures (missing/malformed header, unknown api_key,
// non-active tenant) return 401 with a generic message — never 403, which
// would imply successful authentication — and never reveal whether the
// failure was an unknown key vs. a suspended tenant.
func handleStatus(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// ── Authenticate via Authorization: Bearer header ────────
		auth := e.Request.Header.Get("Authorization")
		if !strings.HasPrefix(auth, bearerPrefix) {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "missing or malformed Authorization header (expected: Bearer <api_key>)",
			})
		}
		apiKey := strings.TrimPrefix(auth, bearerPrefix)
		if apiKey == "" {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "missing or malformed Authorization header (expected: Bearer <api_key>)",
			})
		}

		// ── Look up tenant by api_key (uniquely indexed) ─────────
		tenant, err := app.FindFirstRecordByData("tenants", "api_key", apiKey)
		if err != nil || tenant.GetString("status") != "active" {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid api_key or tenant is not active",
			})
		}
		tenantID := tenant.Id

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

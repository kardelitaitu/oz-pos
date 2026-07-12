package main

import (
	"log"
	"net/http"
	"strings"

	"github.com/pocketbase/pocketbase/core"
)

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
		// Cap request body at 64KB to prevent OOM via oversized JSON payloads (M4 audit).
		e.Request.Body = http.MaxBytesReader(e.Response, e.Request.Body, 64*1024)
		// ── Authenticate via Authorization: Bearer header ────────
		auth := e.Request.Header.Get("Authorization")
		if !strings.HasPrefix(auth, bearerPrefix) {
			log.Printf("/status: missing or malformed Authorization header")
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "missing or malformed Authorization header (expected: Bearer <api_key>)",
			})
		}
		apiKey := strings.TrimPrefix(auth, bearerPrefix)
		if apiKey == "" {
			log.Printf("/status: empty api_key after Bearer prefix")
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "missing or malformed Authorization header (expected: Bearer <api_key>)",
			})
		}

		// ── Look up tenant by api_key (uniquely indexed) ─────────
		tenant, err := app.FindFirstRecordByData("tenants", "api_key", apiKey)
		if err != nil || tenant.GetString("status") != "active" {
			if err != nil {
				log.Printf("/status: unknown api_key (tenant lookup failed): %v", err)
			} else {
				log.Printf("/status: tenant %q status is %q, not active",
					tenant.Id, tenant.GetString("status"))
			}
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid api_key or tenant is not active",
			})
		}
		tenantID := tenant.Id

		// ── Find latest ACTIVE subscription ─────────────────────
		// Only return active subscriptions — expired/revoked/grace_period
		// subscriptions are not the current license state. Without the
		// status filter, a more-recently-created "expired" subscription
		// would shadow an older "active" one (when both share the same
		// starts_at, or the expired one has a later starts_at from a
		// churn that was subsequently reversed).
		subs, err := app.FindRecordsByFilter(
			"subscriptions",
			"tenant_id = {:tenant_id} && status = 'active'",
			"-starts_at", 1, 0,
			map[string]any{"tenant_id": tenantID},
		)
		if err != nil || len(subs) == 0 {
			if err != nil {
				log.Printf("/status: subscription query failed for tenant=%q: %v", tenantID, err)
			} else {
				log.Printf("/status: tenant=%q has no active subscription", tenantID)
			}
			return e.JSON(http.StatusOK, map[string]any{
				"tenant_id": tenantID,
				"status":    tenant.GetString("status"),
				"tier":      "unknown",
				"active":    false,
			})
		}

		sub := subs[0]
		tierKey := sub.GetString("tier_key")
		subStatus := sub.GetString("status")
		log.Printf("/status: tenant=%q tier=%s status=%s active=%v",
			tenantID, tierKey, subStatus, subStatus == "active")
		return e.JSON(http.StatusOK, map[string]any{
			"tenant_id":   tenantID,
			"status":      tenant.GetString("status"),
			"tier":        tierKey,
			"active":      subStatus == "active",
			"expires_at":  sub.GetString("expires_at"),
			"grace_until": sub.GetString("grace_until"),
			"max_stores":  sub.GetInt("max_stores"),
		})
	}
}

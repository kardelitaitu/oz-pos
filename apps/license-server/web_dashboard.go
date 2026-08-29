package main

// User dashboard API endpoints (ADR #42 Phase 2) — read-only resources
// scoped to the authenticated tenant. All endpoints require a Bearer
// session token (same as /me). The session is resolved via extractBearerToken
// + webOtpStore.getSession, exactly like the existing web handlers.
//
// Endpoints:
//
//	GET  /api/v1/web/usage    — tenant usage stats (device/terminal count + limits)
//	GET  /api/v1/web/devices  — registered devices for the tenant
//	POST /api/v1/web/devices/{machine_id}/revoke — revoke one device (self-service)
//	PATCH /api/v1/web/settings — update tenant preferences (region, notifications)

import (
	"log"
	"net/http"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// ── Shared session resolver ─────────────────────────────────────────

// resolveWebSession extracts the Bearer token, resolves the tenant, and
// returns the tenant record or writes a 401 JSON response. Callers must
// check the returned bool.
func resolveWebSession(app core.App, e *core.RequestEvent) (*core.Record, bool) {
	if !webOriginAllowed(e) {
		e.JSON(http.StatusForbidden, map[string]any{"error": "origin not allowed"})
		return nil, false
	}
	token, err := extractBearerToken(e)
	if err != nil {
		e.Response.Header().Set("WWW-Authenticate", `Bearer realm="web"`)
		e.JSON(http.StatusUnauthorized, map[string]any{"error": "missing or invalid session token"})
		return nil, false
	}
	tenantID := webOtpStore.getSession(hashWebToken(token))
	if tenantID == "" {
		e.Response.Header().Set("WWW-Authenticate", `Bearer realm="web"`)
		e.JSON(http.StatusUnauthorized, map[string]any{"error": "invalid or expired session"})
		return nil, false
	}
	tenant, err := app.FindRecordById("tenants", tenantID)
	if err != nil {
		webOtpStore.deleteSession(hashWebToken(token))
		e.Response.Header().Set("WWW-Authenticate", `Bearer realm="web"`)
		e.JSON(http.StatusUnauthorized, map[string]any{"error": "invalid or expired session"})
		return nil, false
	}
	return tenant, true
}

// ── GET /api/v1/web/usage ──────────────────────────────────────────

// handleWebUsage returns usage stats for the authenticated tenant.
func handleWebUsage(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		tenant, ok := resolveWebSession(app, e)
		if !ok {
			return nil // response already sent
		}

		// Count tenant_machines (devices).
		machines, _ := app.FindRecordsByFilter("tenant_machines",
			"tenant_id = {:tid}", "-created", 0, 0,
			map[string]any{"tid": tenant.Id})
		deviceCount := len(machines)

		// Count subscriptions for this tenant.
		subs, _ := app.FindRecordsByFilter("subscriptions",
			"tenant_id = {:tid}", "-created", 0, 0,
			map[string]any{"tid": tenant.Id})
		subCount := len(subs)

		// Pull entitlement limits from the latest subscription.
		maxStores := int64(0)
		maxPos := int64(0)
		if len(subs) > 0 {
			maxStores = int64(subs[0].GetInt("max_stores"))
			maxPos = int64(subs[0].GetInt("max_pos_instances"))
		}

		return e.JSON(http.StatusOK, map[string]any{
			"device_count":       deviceCount,
			"subscription_count": subCount,
			"max_stores":         maxStores,
			"max_pos_instances":  maxPos,
		})
	}
}

// ── GET /api/v1/web/devices ────────────────────────────────────────

// handleWebDevices returns the list of registered machines for the tenant.
func handleWebDevices(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		tenant, ok := resolveWebSession(app, e)
		if !ok {
			return nil
		}

		records, err := app.FindRecordsByFilter("tenant_machines",
			"tenant_id = {:tid}",
			"-created", 0, 0,
			map[string]any{"tid": tenant.Id})
		if err != nil {
			log.Printf("/web/devices: query failed for tenant %q: %v", tenant.Id, err)
			return e.JSON(http.StatusOK, map[string]any{"devices": []any{}})
		}

		devices := make([]map[string]any, 0, len(records))
		for _, rec := range records {
			devices = append(devices, map[string]any{
				"id":         rec.Id,
				"machine_id": rec.GetString("machine_id"),
				"device_id":  rec.GetString("device_id"),
				"created":    rec.GetDateTime("created").Time().Format(time.RFC3339),
				"revoked_at": rec.GetString("revoked_at"),
			})
		}

		return e.JSON(http.StatusOK, map[string]any{"devices": devices})
	}
}

// ── POST /api/v1/web/devices/{id}/revoke ─────────────────────────

// handleWebRevokeDevice lets a tenant revoke their own device (self-service).
// Sets the revoked_at timestamp on the tenant_machine record. The device
// must belong to the authenticated tenant. Idempotent: already-revoked
// devices return 200 with the existing revoked_at.
func handleWebRevokeDevice(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		tenant, ok := resolveWebSession(app, e)
		if !ok {
			return nil
		}

		deviceID := e.Request.PathValue("id")
		if deviceID == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{"error": "device id is required"})
		}

		// Load the machine record.
		machine, err := app.FindRecordById("tenant_machines", deviceID)
		if err != nil {
			return e.JSON(http.StatusNotFound, map[string]any{"error": "device not found"})
		}

		// Ownership check — only the device's tenant can revoke it.
		if machine.GetString("tenant_id") != tenant.Id {
			return e.JSON(http.StatusForbidden, map[string]any{"error": "device does not belong to this account"})
		}

		// Idempotent: if already revoked, return the existing timestamp.
		existing := machine.GetString("revoked_at")
		if existing != "" {
			return e.JSON(http.StatusOK, map[string]any{"status": "revoked", "revoked_at": existing})
		}

		now := time.Now().UTC().Format(time.RFC3339)
		machine.Set("revoked_at", now)
		if err := app.Save(machine); err != nil {
			log.Printf("/web/devices/%s/revoke: save failed: %v", deviceID, err)
			return e.JSON(http.StatusInternalServerError, map[string]any{"error": "revoke failed"})
		}

		log.Printf("/web/devices/%s/revoke: device %q revoked by tenant %q", deviceID, machine.GetString("machine_id"), tenant.GetString("email"))
		return e.JSON(http.StatusOK, map[string]any{"status": "revoked", "revoked_at": now})
	}
}

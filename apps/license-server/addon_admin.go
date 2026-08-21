package main

import (
	"encoding/json"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// AddonPurchaseRequest is the JSON body for POST /api/v1/admin/license-addons.
type AddonPurchaseRequest struct {
	// License key to add the addon to.
	LicenseKey string `json:"license_key"`
	// Addon identifier (e.g. "advanced_analytics", "priority_support").
	AddonID string `json:"addon_id"`
}

// AddonRemoveRequest is the JSON body for DELETE /api/v1/admin/license-addons.
type AddonRemoveRequest struct {
	// License key to remove the addon from.
	LicenseKey string `json:"license_key"`
	// Addon identifier to remove.
	AddonID string `json:"addon_id"`
}

// handleAddLicenseAddon returns an HTTP handler for adding an addon to a
// license key (admin-only). This is called after a successful Paddle
// addon purchase webhook.
//
// POST /api/v1/admin/license-addons
func handleAddLicenseAddon(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// ── Authenticate ──────────────────────────────────────
		if !authenticateAdmin(app, e) {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "Authorization: Bearer <api_key> header required",
			})
		}

		// ── Parse request ─────────────────────────────────────
		var req AddonPurchaseRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{"error": "invalid JSON body"})
		}
		req.LicenseKey = strings.TrimSpace(req.LicenseKey)
		req.AddonID = strings.TrimSpace(strings.ToLower(req.AddonID))

		if req.LicenseKey == "" || req.AddonID == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "license_key and addon_id are required",
			})
		}

		// ── Find the license key ──────────────────────────────
		keyRec, err := app.FindFirstRecordByData("license_keys", "key", req.LicenseKey)
		if err != nil {
			return e.JSON(http.StatusNotFound, map[string]any{
				"error": "license key not found",
			})
		}

		// ── Check if addon already present ────────────────────
		existingAddons := parseAddonsFromRecord(keyRec)
		for _, a := range existingAddons {
			if strings.EqualFold(a, req.AddonID) {
				return e.JSON(http.StatusConflict, map[string]any{
					"error": "addon already active on this license key",
				})
			}
		}

		// ── Add the addon ─────────────────────────────────────
		existingAddons = append(existingAddons, req.AddonID)
		addonsJSON, _ := json.Marshal(existingAddons)
		keyRec.Set("addons", string(addonsJSON))

		if saveErr := app.Save(keyRec); saveErr != nil {
			log.Printf("addon-admin: failed to save addon: %v", saveErr)
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to add addon to license key",
			})
		}

		log.Printf("addon-admin: added addon %s to key %s", req.AddonID, req.LicenseKey[:8]+"****")

		return e.JSON(http.StatusOK, map[string]any{
			"status":      "addon_added",
			"license_key": req.LicenseKey,
			"addon_id":    req.AddonID,
			"addons":      existingAddons,
		})
	}
}

// handleRemoveLicenseAddon returns an HTTP handler for removing an addon
// from a license key (admin-only).
//
// DELETE /api/v1/admin/license-addons
func handleRemoveLicenseAddon(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// ── Authenticate ──────────────────────────────────────
		if !authenticateAdmin(app, e) {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "Authorization: Bearer <api_key> header required",
			})
		}

		// ── Parse request ─────────────────────────────────────
		var req AddonRemoveRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{"error": "invalid JSON body"})
		}
		req.LicenseKey = strings.TrimSpace(req.LicenseKey)
		req.AddonID = strings.TrimSpace(strings.ToLower(req.AddonID))

		if req.LicenseKey == "" || req.AddonID == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "license_key and addon_id are required",
			})
		}

		// ── Find the license key ──────────────────────────────
		keyRec, err := app.FindFirstRecordByData("license_keys", "key", req.LicenseKey)
		if err != nil {
			return e.JSON(http.StatusNotFound, map[string]any{
				"error": "license key not found",
			})
		}

		// ── Remove the addon ──────────────────────────────────
		existingAddons := parseAddonsFromRecord(keyRec)
		newAddons := make([]string, 0, len(existingAddons))
		found := false
		for _, a := range existingAddons {
			if strings.EqualFold(a, req.AddonID) {
				found = true
				continue
			}
			newAddons = append(newAddons, a)
		}
		if !found {
			return e.JSON(http.StatusNotFound, map[string]any{
				"error": "addon not found on this license key",
			})
		}

		addonsJSON, _ := json.Marshal(newAddons)
		keyRec.Set("addons", string(addonsJSON))

		if saveErr := app.Save(keyRec); saveErr != nil {
			log.Printf("addon-admin: failed to save addon removal: %v", saveErr)
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to remove addon from license key",
			})
		}

		log.Printf("addon-admin: removed addon %s from key %s", req.AddonID, req.LicenseKey[:8]+"****")

		return e.JSON(http.StatusOK, map[string]any{
			"status":      "addon_removed",
			"license_key": req.LicenseKey,
			"addon_id":    req.AddonID,
			"addons":      newAddons,
		})
	}
}

// handleListLicenseAddons returns the addons on a specific license key.
//
// GET /api/v1/admin/license-addons?key=OZ-XXXXX
func handleListLicenseAddons(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// ── Authenticate ──────────────────────────────────────
		if !authenticateAdmin(app, e) {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "Authorization: Bearer <api_key> header required",
			})
		}

		key := strings.TrimSpace(e.Request.URL.Query().Get("key"))
		if key == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "key query parameter is required",
			})
		}

		keyRec, err := app.FindFirstRecordByData("license_keys", "key", key)
		if err != nil {
			return e.JSON(http.StatusNotFound, map[string]any{
				"error": "license key not found",
			})
		}

		addons := parseAddonsFromRecord(keyRec)

		return e.JSON(http.StatusOK, map[string]any{
			"license_key": key,
			"tier":        keyRec.GetString("tier_key"),
			"addons":      addons,
			"updated_at":  time.Now().UTC().Format(time.RFC3339),
		})
	}
}

// parseAddonsFromRecord extracts the addons array from a license key record.
// The addons are stored as a JSON string in the "addons" field.
func parseAddonsFromRecord(rec *core.Record) []string {
	raw := rec.GetString("addons")
	if raw == "" {
		return []string{}
	}
	var addons []string
	if err := json.Unmarshal([]byte(raw), &addons); err != nil {
		return []string{}
	}
	return addons
}

// authenticateAdmin checks that the request has a valid Bearer token.
func authenticateAdmin(app core.App, e *core.RequestEvent) bool {
	authHeader := e.Request.Header.Get("Authorization")
	if !strings.HasPrefix(authHeader, bearerPrefix) {
		return false
	}
	apiKey := strings.TrimSpace(strings.TrimPrefix(authHeader, bearerPrefix))
	if apiKey == "" {
		return false
	}
	lookup := apiKeyLookup(apiKey)
	_, err := app.FindFirstRecordByData("tenants", "api_key_lookup", lookup)
	return err == nil
}

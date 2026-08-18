package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// GenerateCodeRequest is the JSON body for POST /api/v1/admin/enterprise-codes.
type GenerateCodeRequest struct {
	// Prospect email (optional but recommended for tracking).
	Email string `json:"email"`
	// Prospect or company name (optional).
	ProspectName string `json:"prospect_name"`
	// Optional custom code — if empty, a random code is generated.
	CustomCode string `json:"custom_code"`
}

// GenerateCodeResponse is the JSON response from the code generation endpoint.
type GenerateCodeResponse struct {
	Code        string `json:"code"`
	Email       string `json:"email,omitempty"`
	ProspectName string `json:"prospect_name,omitempty"`
	Status      string `json:"status"`
	CreatedAt   string `json:"created_at"`
}

// handleGenerateEnterpriseCode returns an HTTP handler for creating enterprise
// trial approval codes (admin-only).
//
// Requires Authorization: Bearer <admin_api_key> header. In production,
// the admin key should be validated against a known admin set. For now,
// any valid tenant API key is accepted (the admin panel authenticates
// separately).
//
// POST /api/v1/admin/enterprise-codes
func handleGenerateEnterpriseCode(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// ── Authenticate ──────────────────────────────────────
		authHeader := e.Request.Header.Get("Authorization")
		if !strings.HasPrefix(authHeader, bearerPrefix) {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "Authorization: Bearer <api_key> header required",
			})
		}
		apiKey := strings.TrimSpace(strings.TrimPrefix(authHeader, bearerPrefix))
		if apiKey == "" {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "api_key must not be empty",
			})
		}

		// Verify the API key belongs to a valid tenant
		lookup := apiKeyLookup(apiKey)
		_, err := app.FindFirstRecordByData("tenants", "api_key_lookup", lookup)
		if err != nil {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid api_key",
			})
		}

		// ── Parse request body ─────────────────────────────────
		var req GenerateCodeRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			// Allow empty body — all fields are optional
			req = GenerateCodeRequest{}
		}

		// ── Generate or validate code ──────────────────────────
		code := strings.ToUpper(strings.TrimSpace(req.CustomCode))
		if code == "" {
			code = generateApprovalCode()
		}
		if len(code) < 8 {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "custom_code must be at least 8 characters",
			})
		}

		// Check for uniqueness
		existing, _ := app.FindFirstRecordByData("enterprise_approvals", "code", code)
		if existing != nil {
			return e.JSON(http.StatusConflict, map[string]any{
				"error": "code already exists",
			})
		}

		// ── Create the approval code record ────────────────────
		coll, collErr := app.FindCollectionByNameOrId("enterprise_approvals")
		if collErr != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "server misconfiguration: enterprise_approvals collection not found",
			})
		}
		rec := core.NewRecord(coll)
		rec.Set("code", code)
		rec.Set("email", strings.ToLower(strings.TrimSpace(req.Email)))
		rec.Set("prospect_name", req.ProspectName)
		rec.Set("status", "unused")

		if saveErr := app.Save(rec); saveErr != nil {
			log.Printf("enterprise-admin: failed to save approval code: %v", saveErr)
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to create approval code",
			})
		}

		log.Printf("enterprise-admin: generated approval code %s for %s", code[:4]+"****",
			strings.TrimSpace(req.Email))

		return e.JSON(http.StatusOK, GenerateCodeResponse{
			Code:         code,
			Email:        req.Email,
			ProspectName: req.ProspectName,
			Status:       "unused",
			CreatedAt:    time.Now().UTC().Format(time.RFC3339),
		})
	}
}

// handleListEnterpriseCodes returns a list of all enterprise approval codes
// (admin-only). Supports filtering by status via query parameter.
//
// GET /api/v1/admin/enterprise-codes?status=unused
func handleListEnterpriseCodes(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// ── Authenticate ──────────────────────────────────────
		authHeader := e.Request.Header.Get("Authorization")
		if !strings.HasPrefix(authHeader, bearerPrefix) {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "Authorization: Bearer <api_key> header required",
			})
		}
		apiKey := strings.TrimSpace(strings.TrimPrefix(authHeader, bearerPrefix))
		if apiKey == "" {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "api_key must not be empty",
			})
		}
		lookup := apiKeyLookup(apiKey)
		_, err := app.FindFirstRecordByData("tenants", "api_key_lookup", lookup)
		if err != nil {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid api_key",
			})
		}

		// ── List codes (optionally filtered by status) ─────────
		statusFilter := e.Request.URL.Query().Get("status")
		var records []*core.Record
		if statusFilter != "" {
			records, err = app.FindRecordsByFilter("enterprise_approvals",
				"status = {:status}", "-created", 100, 0,
				map[string]any{"status": statusFilter})
		} else {
			records, err = app.FindRecordsByFilter("enterprise_approvals",
				"", "-created", 100, 0, nil)
		}
		if err != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to list approval codes",
			})
		}

		type codeEntry struct {
			Code         string `json:"code"`
			Email        string `json:"email,omitempty"`
			ProspectName string `json:"prospect_name,omitempty"`
			Status       string `json:"status"`
			CreatedAt    string `json:"created_at"`
		}
		var entries []codeEntry
		for _, r := range records {
			entries = append(entries, codeEntry{
				Code:         r.GetString("code"),
				Email:        r.GetString("email"),
				ProspectName: r.GetString("prospect_name"),
				Status:       r.GetString("status"),
				CreatedAt:    r.GetDateTime("created").Time().UTC().Format(time.RFC3339),
			})
		}

		return e.JSON(http.StatusOK, map[string]any{
			"codes": entries,
			"total": len(entries),
		})
	}
}

// generateApprovalCode creates a random alphanumeric code in the format
// ENT-XXXXXXXX (8 hex chars after the prefix). The prefix makes enterprise
// codes visually distinguishable from license keys.
func generateApprovalCode() string {
	b := make([]byte, 4)
	_, _ = rand.Read(b)
	return "ENT-" + strings.ToUpper(hex.EncodeToString(b))
}



package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"log"
	"net/http"
	"strings"
	"time"
	"unicode/utf8"

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
	Code         string `json:"code"`
	Email        string `json:"email,omitempty"`
	ProspectName string `json:"prospect_name,omitempty"`
	Status       string `json:"status"`
	CreatedAt    string `json:"created_at"`
}

// handleGenerateEnterpriseCode returns an HTTP handler for creating enterprise
// trial approval codes (admin-only).
//
// Requires Authorization: Bearer <admin_key> (OZ_ADMIN_KEY) or a web session
// belonging to the admin tenant (OZ_ADMIN_EMAIL) — the same gate as the
// admin dashboard endpoints. LSE-9 fix: this previously accepted ANY valid
// tenant api_key, letting any activated customer mint approval codes.
//
// POST /api/v1/admin/enterprise-codes
func handleGenerateEnterpriseCode(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		// ── Authenticate ──────────────────────────────────────
		if !authenticateAdmin(app, e) {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "Authorization: Bearer <admin_key> header required",
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

		// B42: mirror the schema's field caps (ensureEnterpriseApprovals:
		// code Max:64, email Max:254, prospect_name Max:256). PocketBase
		// validates by RUNE count ("casted to []rune to count multi-byte
		// chars as one"), so the pre-check must too — otherwise oversized
		// input sails past the handler and fails at Save, surfacing a 500
		// for plainly bad client data (same class as B30's unknown
		// tier_key).
		if utf8.RuneCountInString(code) > 64 {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "custom_code must be at most 64 characters",
			})
		}
		email := strings.ToLower(strings.TrimSpace(req.Email))
		if utf8.RuneCountInString(email) > 254 {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "email must be at most 254 characters",
			})
		}
		prospect := strings.TrimSpace(req.ProspectName)
		if utf8.RuneCountInString(prospect) > 256 {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "prospect_name must be at most 256 characters",
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
		rec.Set("email", email)
		rec.Set("prospect_name", prospect)
		rec.Set("status", "unused")
		// B43: the schema has created_by for attribution; the handler
		// never filled it, so every privileged code was minted
		// anonymously. Record which admin credential created it.
		who := adminIdentity(app, e)
		rec.Set("created_by", who)

		if saveErr := app.Save(rec); saveErr != nil {
			log.Printf("enterprise-admin: failed to save approval code: %v", saveErr)
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to create approval code",
			})
		}

		log.Printf("enterprise-admin: generated approval code %s for %s (by %s)", code[:4]+"****",
			email, who)

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
		if !authenticateAdmin(app, e) {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "Authorization: Bearer <admin_key> header required",
			})
		}

		// ── List codes (optionally filtered by status) ─────────
		statusFilter := e.Request.URL.Query().Get("status")
		var records []*core.Record
		var err error
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

// adminIdentity labels WHICH admin credential authorized the current
// request, for audit fields like enterprise_approvals.created_by (B43).
// It mirrors authenticateAdmin's two accepted forms (addon_admin.go)
// without re-deciding authorization — callers must already have passed
// that gate, so "unknown" only labels an unexpected shape.
func adminIdentity(app core.App, e *core.RequestEvent) string {
	if adminKeyOK(e) {
		return "admin_key"
	}
	token, err := extractBearerToken(e)
	if err != nil {
		return "unknown"
	}
	tenantID := webOtpStore.getSession(hashWebToken(token))
	if tenantID == "" {
		return "unknown"
	}
	tenant, err := app.FindRecordById("tenants", tenantID)
	if err != nil {
		return "unknown"
	}
	return tenant.GetString("email")
}

package main

// One-time session exchange (security hardening F1): lets the login flow
// hand a session to the Worker WITHOUT putting the session token in a URL.
//
// Flow:
//
//	1. Login page authenticates → POST /api/v1/web/exchange-issue
//	   (Bearer <session token>) → returns a short-lived one-time code.
//	2. The page redirects to dashboard/admin with ?code=<code> — only the
//	   one-time code ever appears in a URL, never the session token.
//	3. The Worker POSTs /api/v1/web/exchange-consume with { code } and
//	   receives a fresh session token, sets the httpOnly cookie, and
//	   redirects to a clean URL.
//
// The code is single-use and expires in 30s, so even if it leaks from a
// URL (history, access logs) it cannot be replayed for another session.

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"log"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

const (
	// exchangeTTL is how long a minted exchange code stays valid.
	exchangeTTL = 30 * time.Second
	// exchangeSweepInterval is how often expired codes are cleaned up.
	exchangeSweepInterval = 1 * time.Minute
)

// exchangeEntry is a minted one-time code bound to a tenant.
type exchangeEntry struct {
	tenantID  string
	expiresAt time.Time
}

// exchangeStore holds pending one-time exchange codes (in-memory, like the
// OTP/session stores — short-lived and single-use).
type exchangeStore struct {
	mu      sync.Mutex
	entries map[string]*exchangeEntry
}

var webExchangeStore = &exchangeStore{
	entries: make(map[string]*exchangeEntry),
}

// mint creates a one-time code for the tenant and returns it.
func (s *exchangeStore) mint(tenantID string) (string, error) {
	b := make([]byte, 24)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	code := hex.EncodeToString(b)
	s.mu.Lock()
	defer s.mu.Unlock()
	s.entries[code] = &exchangeEntry{tenantID: tenantID, expiresAt: time.Now().Add(exchangeTTL)}
	return code, nil
}

// consume atomically reads and deletes the code. Returns the tenant id,
// or "" when the code is unknown/expired (both treated identically).
func (s *exchangeStore) consume(code string) string {
	s.mu.Lock()
	defer s.mu.Unlock()
	entry, ok := s.entries[code]
	if !ok || time.Now().After(entry.expiresAt) {
		delete(s.entries, code)
		return ""
	}
	delete(s.entries, code)
	return entry.tenantID
}

// sweep removes expired codes.
func (s *exchangeStore) sweep() {
	s.mu.Lock()
	defer s.mu.Unlock()
	now := time.Now()
	for code, entry := range s.entries {
		if now.After(entry.expiresAt) {
			delete(s.entries, code)
		}
	}
}

// exchangeSweepLoop is the background cleanup goroutine.
func exchangeSweepLoop() {
	ticker := time.NewTicker(exchangeSweepInterval)
	defer ticker.Stop()
	for range ticker.C {
		webExchangeStore.sweep()
	}
}

// ── POST /api/v1/web/exchange-issue ────────────────────────────────

// handleExchangeIssue mints a one-time code for the authenticated tenant.
// Requires the same Bearer session as /me.
func handleExchangeIssue(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		if !webOriginAllowed(e) {
			return e.JSON(http.StatusForbidden, map[string]any{"error": "origin not allowed"})
		}
		token, err := extractBearerToken(e)
		if err != nil {
			e.Response.Header().Set("WWW-Authenticate", `Bearer realm="web"`)
			return e.JSON(http.StatusUnauthorized, map[string]any{"error": "missing or invalid session token"})
		}
		tenantID := webOtpStore.getSession(hashWebToken(token))
		if tenantID == "" {
			e.Response.Header().Set("WWW-Authenticate", `Bearer realm="web"`)
			return e.JSON(http.StatusUnauthorized, map[string]any{"error": "invalid or expired session"})
		}
		// Active-use refresh, consistent with /me.
		webOtpStore.touchSession(hashWebToken(token))

		code, err := webExchangeStore.mint(tenantID)
		if err != nil {
			log.Printf("/web/exchange-issue: code mint failed: %v", err)
			return e.JSON(http.StatusInternalServerError, map[string]any{"error": "could not mint exchange code"})
		}
		return e.JSON(http.StatusOK, map[string]any{
			"code":       code,
			"expires_in": int(exchangeTTL.Seconds()),
		})
	}
}

// ── POST /api/v1/web/exchange-consume ──────────────────────────────

// exchangeConsumeRequest is the body for exchange-consume.
type exchangeConsumeRequest struct {
	Code string `json:"code"`
}

// handleExchangeConsume consumes a one-time code and issues a fresh
// session for the bound tenant. Single-use: the code is deleted on read.
func handleExchangeConsume(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		e.Request.Body = http.MaxBytesReader(e.Response, e.Request.Body, webMaxBodyBytes)
		if !webOriginAllowed(e) {
			return e.JSON(http.StatusForbidden, map[string]any{"error": "origin not allowed"})
		}
		var req exchangeConsumeRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{"error": "invalid JSON body"})
		}
		code := strings.TrimSpace(req.Code)
		if code == "" || len(code) < 32 {
			return e.JSON(http.StatusBadRequest, map[string]any{"error": "code is required"})
		}
		tenantID := webExchangeStore.consume(code)
		if tenantID == "" {
			return e.JSON(http.StatusUnauthorized, map[string]any{"error": "invalid or expired code"})
		}
		tenant, err := app.FindRecordById("tenants", tenantID)
		if err != nil {
			return e.JSON(http.StatusUnauthorized, map[string]any{"error": "invalid or expired code"})
		}
		token, expiresAt, err := issueWebSession(tenant.Id)
		if err != nil {
			log.Printf("/web/exchange-consume: session issuance failed for tenant %q: %v", tenant.Id, err)
			return e.JSON(http.StatusInternalServerError, map[string]any{"error": "could not start a session"})
		}
		log.Printf("/web/exchange-consume: session issued for tenant %q via one-time code", tenant.Id)
		return e.JSON(http.StatusOK, webSessionResponse(app, tenant, token, expiresAt))
	}
}

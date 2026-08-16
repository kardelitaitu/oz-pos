package main

// Password login for the marketing site's account flow (website-plan.md
// §5/§11), the complement to the OTP flow in web_otp.go.
//
// Two new endpoints:
//
//	POST /api/v1/web/login        — {email, password} → session token
//	POST /api/v1/web/set-password — {password} (Bearer session) → sets bcrypt hash
//
// The password is OPTIONAL: tenants are register-first via request-otp
// (email proof), and set-password is only reachable with an authenticated
// session, so an account can never be claimed by knowing an email alone —
// the same anti-claim property as the OTP flow. Tenants without a
// password_hash get the same generic 401 on login as a wrong password, so
// the endpoint never reveals whether an account exists or has a password
// set (matching verify-otp's no-enumeration posture).
//
// Passwords are stored as bcrypt hashes only (same golang.org/x/crypto
// dependency as api_key hashing) — never plaintext. Login shares the OTP
// session store (webOtpStore), so a password session and an OTP session
// are interchangeable Bearer tokens for /me.

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/pocketbase/pocketbase/core"
	"golang.org/x/crypto/bcrypt"
)

const (
	// webPasswordMinLen is the minimum password length (bytes AND runes).
	webPasswordMinLen = 8
	// webPasswordMaxBytes caps the length at bcrypt's 72-byte input limit
	// (bcrypt silently truncates beyond it, which would let a 100-char
	// password authenticate with only its first 72 bytes).
	webPasswordMaxBytes = 72
	// webLoginMax is the login-attempt budget per email per window —
	// deliberately tight (5/15min) so brute-forcing a password hash costs
	// more than the bcrypt work factor alone. The per-IP backstop is
	// shared with the OTP flow (otpIPLimiter).
	webLoginMax    = 5
	webLoginWindow = 15 * time.Minute
)

// webLoginLimiter bounds password-login attempts per email (5/15min).
var webLoginLimiter = &windowLimiter{
	entries: make(map[string]*windowEntry),
	limit:   webLoginMax,
	window:  webLoginWindow,
}

// hashPassword derives the at-rest representation of a web password
// (bcrypt, DefaultCost — same strength as api_key hashing).
func hashPassword(password string) (string, error) {
	h, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		return "", fmt.Errorf("failed to hash password: %w", err)
	}
	return string(h), nil
}

// isValidPassword enforces the password policy: 8–72 bytes, no leading or
// trailing whitespace (copy-paste surprises), and at least 8 runes so
// multi-byte (e.g. emoji) passwords can't sneak under the minimum by byte
// count alone.
func isValidPassword(password string) bool {
	if password == "" {
		return false
	}
	if len(password) < webPasswordMinLen || len(password) > webPasswordMaxBytes {
		return false
	}
	if len([]rune(password)) < webPasswordMinLen {
		return false
	}
	if strings.TrimSpace(password) != password {
		return false
	}
	return true
}

// ── POST /api/v1/web/login ─────────────────────────────────────────

// handleLoginPassword implements POST /api/v1/web/login — the password
// alternative to request-otp.
//
//	{ "email": "owner@example.com", "password": "..." }
//
// Success → the SAME shape as verify-otp: { token, expires_at, tenant,
// license, subscription }. Failure → a generic 401 "invalid email or
// password" for every failure mode (unknown email, no password set, wrong
// password, non-active tenant) so the endpoint never reveals account
// state. Login does NOT touch email_verified — only verify-otp proves
// inbox ownership; a password proves knowledge of a credential the owner
// set while already authenticated.
//
//   - 429: rate limited (5/email/15min, 10/IP/15min).
//   - 403: browser Origin outside the OZ_WEB_ALLOWED_ORIGINS allowlist.
//   - 400: malformed body / invalid email.
func handleLoginPassword(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		e.Request.Body = http.MaxBytesReader(e.Response, e.Request.Body, webMaxBodyBytes)

		if !webOriginAllowed(e) {
			return e.JSON(http.StatusForbidden, map[string]any{
				"error": "origin not allowed",
			})
		}

		clientIP := e.RealIP()
		var req struct {
			Email    string `json:"email"`
			Password string `json:"password"`
		}
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			otpIPLimiter.allow(clientIP)
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "invalid JSON body",
			})
		}

		email := normalizeEmail(req.Email)
		if email == "" {
			otpIPLimiter.allow(clientIP)
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "email is required",
			})
		}
		if !isValidEmail(email) {
			otpIPLimiter.allow(clientIP)
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "email must be a valid address",
			})
		}

		// ── Rate limit (per email + per IP), BEFORE tenant lookup ──
		if !webLoginLimiter.allow(email) || !otpIPLimiter.allow(clientIP) {
			return e.JSON(http.StatusTooManyRequests, map[string]any{
				"error": "rate limit exceeded, try again later",
			})
		}

		// ── Resolve tenant ────────────────────────────────────────
		tenant, err := app.FindFirstRecordByData("tenants", "email", email)
		if tenant == nil || err != nil {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid email or password",
			})
		}

		// Non-active tenants get the same generic 401 (their passwords
		// are never honored, but the response must not differ).
		if tenant.GetString("status") != "active" {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid email or password",
			})
		}

		// ── Verify the bcrypt hash ───────────────────────────────
		// A missing password_hash (OTP-only account) takes the same path
		// as a wrong password: the UI hints at the email-code alternative,
		// but the API never says which case occurred.
		storedHash := tenant.GetString("password_hash")
		if storedHash == "" ||
			bcrypt.CompareHashAndPassword([]byte(storedHash), []byte(req.Password)) != nil {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid email or password",
			})
		}

		// ── Issue session token (same store as verify-otp) ───────
		token, expiresAt, err := issueWebSession(tenant.Id)
		if err != nil {
			log.Printf("/web/login: session issuance failed for tenant %q: %v", tenant.Id, err)
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "could not start a session, please try again",
			})
		}

		log.Printf("/web/login: session issued for tenant %q", tenant.Id)
		return e.JSON(http.StatusOK, webSessionResponse(app, tenant, token, expiresAt))
	}
}

// ── POST /api/v1/web/set-password ──────────────────────────────────

// handleSetPassword implements POST /api/v1/web/set-password — the only
// way a password is created or changed.
//
//	{ "password": "..." }        Authorization: Bearer <session token>
//
// The session (issued only by verify-otp or login) identifies the account
// — the body carries no email, so the request can never set a password for
// a different tenant even if the JSON is mangled. First-time callers set
// their initial password; repeat callers rotate it. Success → 200
// {"status":"ok"}; the new hash takes effect on the next login.
//
//   - 401: missing/unknown/expired session token.
//   - 400: password fails the policy (8–72 chars, no edge whitespace).
//   - 403: browser Origin outside the allowlist.
func handleSetPassword(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		e.Request.Body = http.MaxBytesReader(e.Response, e.Request.Body, webMaxBodyBytes)

		if !webOriginAllowed(e) {
			return e.JSON(http.StatusForbidden, map[string]any{
				"error": "origin not allowed",
			})
		}

		token, err := extractBearerToken(e)
		if err != nil {
			e.Response.Header().Set("WWW-Authenticate", `Bearer realm="web"`)
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "missing or invalid session token",
			})
		}
		tenantID := webOtpStore.getSession(hashWebToken(token))
		if tenantID == "" {
			e.Response.Header().Set("WWW-Authenticate", `Bearer realm="web"`)
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid or expired session",
			})
		}

		var req struct {
			Password string `json:"password"`
		}
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "invalid JSON body",
			})
		}
		if !isValidPassword(req.Password) {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "password must be 8–72 characters",
			})
		}

		tenant, err := app.FindRecordById("tenants", tenantID)
		if err != nil {
			// Tenant deleted mid-session — treat as expired, not a leak.
			webOtpStore.deleteSession(hashWebToken(token))
			e.Response.Header().Set("WWW-Authenticate", `Bearer realm="web"`)
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid or expired session",
			})
		}
		if tenant.GetString("status") != "active" {
			return e.JSON(http.StatusUnauthorized, map[string]any{
				"error": "invalid or expired session",
			})
		}

		hash, err := hashPassword(req.Password)
		if err != nil {
			log.Printf("/web/set-password: hashing failed for tenant %q: %v", tenant.Id, err)
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "could not save the password, please try again",
			})
		}
		tenant.Set("password_hash", hash)
		if err := app.Save(tenant); err != nil {
			log.Printf("/web/set-password: failed to persist password for tenant %q: %v", tenant.Id, err)
			return e.JSON(http.StatusInternalServerError, map[string]any{
				"error": "could not save the password, please try again",
			})
		}

		log.Printf("/web/set-password: password set for tenant %q", tenant.Id)
		return e.JSON(http.StatusOK, map[string]any{"status": "ok"})
	}
}

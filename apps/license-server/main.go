// Package main is the entry point for the OZ-POS license server.
// It extends PocketBase with custom Go hooks for license activation,
// renewal, and status checks with RSA-2048 signing.
//
// Windows manifest: the committed rsrc_windows_amd64.syso embeds
// app.manifest (asInvoker, numeric RT_MANIFEST type 24) into the Windows
// build so UAC never raises an elevation consent prompt. Regenerate it with:
//
//	go generate ./...          (runs: go-winres make --arch amd64)
//
// The .syso is committed so `go build` on Windows needs no extra tooling.
//
//go:generate go run github.com/tc-hib/go-winres@v0.3.3 make --arch amd64
package main

import (
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/x509"
	_ "embed"
	"encoding/base64"
	"encoding/pem"
	"fmt"
	"log"
	"os"
	"strings"

	"github.com/pocketbase/pocketbase"
	"github.com/pocketbase/pocketbase/core"
)

// pbSchemaJSON is the PocketBase collections schema embedded at build time.
// A fresh deployment (empty pb_data volume) boots with only the default
// system collections; the business collections (license_keys, tenants,
// subscriptions, tenant_machines) are imported idempotently on first boot
// from this file so the server never starts "healthy" with activation
// endpoints failing with "collections not found".
//
//go:embed pb_schema.json
var pbSchemaJSON []byte

// requiredCollections are the business collections the license API depends
// on. If any is missing at serve time, the embedded pb_schema.json is
// imported (see ensureCollections).
var requiredCollections = []string{
	"license_keys",
	"tenants",
	"subscriptions",
	"tenant_machines",
}

// privateKey is the RSA-2048 private key loaded from the
// OZ_LICENSE_PRIVATE_KEY environment variable at startup.
var privateKey *rsa.PrivateKey

func main() {
	app := pocketbase.New()

	// ── Bootstrap: load RSA private key ──────────────────────────
	keyPEM := os.Getenv("OZ_LICENSE_PRIVATE_KEY")
	if keyPEM == "" {
		log.Fatal("OZ_LICENSE_PRIVATE_KEY environment variable is required")
	}

	block, _ := pem.Decode([]byte(normalizePEM(keyPEM)))
	if block == nil {
		log.Fatalf("failed to decode PEM block from OZ_LICENSE_PRIVATE_KEY (key length: %d bytes, starts with: %q)",
			len(keyPEM), safePrefix(keyPEM, 40))
	}

	var err error
	privateKey, err = x509.ParsePKCS1PrivateKey(block.Bytes)
	if err != nil {
		// Try PKCS8 format (more common with modern tools)
		pkcs8Key, err2 := x509.ParsePKCS8PrivateKey(block.Bytes)
		if err2 != nil {
			log.Fatalf("failed to parse RSA private key (PKCS1: %v, PKCS8: %v)", err, err2)
		}
		var ok bool
		privateKey, ok = pkcs8Key.(*rsa.PrivateKey)
		if !ok {
			log.Fatal("key is not an RSA private key")
		}
	}
	log.Println("RSA private key loaded successfully")

	// ── Bootstrap: SMTP sender identity ──────────────────────────
	// Fail fast when email delivery is configured but OZ_SMTP_FROM is
	// unset or rejected by the relay: signup codes and purchase receipts
	// would silently fail in production (see verifySMTPConfig). Skipped
	// when OZ_SMTP_HOST is unset — request-otp answers 503 by design then.
	if err := verifySMTPConfig(); err != nil {
		log.Fatal(err)
	}

	// ── Bootstrap: Paddle webhook config ─────────────────────────
	// Fail fast when the webhook would answer 503/500 on every event
	// (missing secret or price→tier map): purchases would provision
	// nothing and Paddle would retry forever (see verifyPaddleConfig).
	if err := verifyPaddleConfig(); err != nil {
		log.Fatal(err)
	}

	// ── Register custom license API routes ───────────────────────
	app.OnServe().BindFunc(func(se *core.ServeEvent) error {
		// First boot on an empty pb_data volume: import the embedded
		// collections schema so /activate, /renew, and /status find their
		// collections instead of a fresh-but-broken deployment.
		// Idempotent: no-op once all required collections exist.
		if err := ensureCollections(app); err != nil {
			return err
		}
		// Add the api_key_lookup field to existing deployments that predate
		// api_key hashing. Fresh boots get it from the embedded pb_schema.json;
		// this is the idempotent in-place upgrade for already-provisioned
		// pb_data volumes, so the SHA-256 lookup index used by
		// findTenantByAPIKey exists on every boot.
		if err := ensureAPIKeyLookupField(app); err != nil {
			return err
		}
		// Idempotent in-place upgrade for deployments that predate the
		// email_verified field (added with the register-first dashboard):
		// fresh boots get it from the embedded pb_schema.json; existing
		// pb_data volumes get it added without reimporting the schema.
		if err := ensureEmailVerifiedField(app); err != nil {
			return err
		}
		// Idempotent in-place upgrade for deployments that predate the
		// password_hash field (added with password login): fresh boots get
		// it from the embedded pb_schema.json; existing pb_data volumes get
		// it added without reimporting the schema. Existing tenants keep an
		// empty password_hash — OTP remains their only login until they set
		// one from the dashboard.
		if err := ensurePasswordHashField(app); err != nil {
			return err
		}
		// Idempotent in-place upgrade for deployments that predate the
		// password_reset_at field (added with the forgot-password flow):
		// fresh boots get it from the embedded pb_schema.json; existing
		// pb_data volumes get it added without reimporting the schema.
		// Existing records keep a zero value — no cooldown, resets allowed.
		if err := ensurePasswordResetAtField(app); err != nil {
			return err
		}
		// Wire rate-limiter persistence to SQLite (H2 audit). Idempotent
		// and logs-and-returns on schema/hydrate failure so the server can
		// still boot in degraded in-memory-only mode if SQLite is unavailable.
		// Runs BEFORE route registration — once routes are mounted, /activate
		// and /renew requests immediately call allow()/recordFailure() which
		// need the persistence handle.
		ipRateLimiter.attachPersistence(app)
		keyFailTracker.attachPersistence(app)

		se.Router.POST("/api/v1/license/activate", handleActivate(app))
		se.Router.POST("/api/v1/license/renew", handleRenew(app))
		// /status uses POST + Authorization: Bearer <api_key> to keep the
		// credential out of URLs (which would otherwise leak it to webserver
		// access logs, CDN logs, browser history, and Referer headers).
		se.Router.POST("/api/v1/license/status", handleStatus(app))
		// Public website support form → Discord channel (see contact.go).
		se.Router.POST("/api/v1/web/contact", handleContact(app))
		// Website tenant-email OTP auth + account dashboard (see web_otp.go).
		// request-otp / verify-otp are the login flow; /me reads the session;
		// logout invalidates it. All four enforce the CORS allowlist from
		// OZ_WEB_ALLOWED_ORIGINS and per-email/IP rate limits in-handler.
		se.Router.POST("/api/v1/web/request-otp", handleRequestOTP(app))
		se.Router.POST("/api/v1/web/verify-otp", handleVerifyOTP(app))
		// Password login + set-password (see web_password.go). login is the
		// email+password alternative to request-otp; set-password is
		// session-authenticated (the account sets its own password from the
		// dashboard). Both enforce the same CORS allowlist.
		se.Router.POST("/api/v1/web/login", handleLoginPassword(app))
		se.Router.POST("/api/v1/web/set-password", handleSetPassword(app))
		// Signup + forgot-password (see web_password.go). register pairs
		// email+password and emails a confirmation code (verify-otp
		// completes it); request-password-reset / reset-password implement
		// the OTP-proved password reset with a 7-day cooldown. All enforce
		// the same CORS allowlist + per-email/IP rate limits.
		se.Router.POST("/api/v1/web/register", handleRegister(app))
		se.Router.POST("/api/v1/web/request-password-reset", handleRequestPasswordReset(app))
		se.Router.POST("/api/v1/web/reset-password", handleResetPassword(app))
		se.Router.GET("/api/v1/web/me", handleMe(app))
		se.Router.POST("/api/v1/web/logout", handleLogout(app))
		// Paddle Billing webhook — signature-verified, server-to-server (see
		// paddle_webhook.go). NOT behind the web CORS allowlist: Paddle sends
		// no Origin, and the Paddle-Signature header is the gate.
		se.Router.POST(paddleWebhookPath, handlePaddleWebhook(app))
		// P8-2: Machine-level revocation is integrated into the /status
		// endpoint (send revoke:true with machine_id in the request body).
		// /api/health: PocketBase's built-in endpoint registers before this
		// hook, so it can't be replaced by re-registering the route; a root
		// middleware short-circuits it with our extended payload (health.go).
		bindHealthOverride(app, se)
		return se.Next()
	})

	if err := app.Start(); err != nil {
		log.Fatal(err)
	}
}

// ensureCollections verifies that every business collection the license API
// depends on exists, importing the embedded pb_schema.json on first boot
// when any is missing (idempotent). Without this, a fresh deployment boots
// "healthy" but every /activate, /renew, and /status call fails with
// "collections not found" until an operator manually imports the schema.
//
// ImportCollectionsByMarshaledJSON runs in a single transaction; deleteMissing
// is false so existing collections are never dropped.
func ensureCollections(app core.App) error {
	for _, name := range requiredCollections {
		if _, err := app.FindCollectionByNameOrId(name); err == nil {
			continue // collection already exists
		}
		// At least one required collection is missing — import the full
		// embedded schema (idempotent for the collections that exist).
		log.Printf("missing required collection %q — importing pb_schema.json", name)
		if err := app.ImportCollectionsByMarshaledJSON(pbSchemaJSON, false); err != nil {
			return fmt.Errorf("failed to auto-import pb_schema.json: %w", err)
		}
		return nil
	}
	return nil
}

// ensureAPIKeyLookupField adds the `api_key_lookup` field (and its unique
// partial index) to the tenants collection if it doesn't exist yet.
//
// Fresh deployments receive the field via the embedded pb_schema.json. This
// migration covers pb_data volumes created before api_key hashing, so the
// deterministic lookup column used by findTenantByAPIKey is always present.
// The index is partial (excluding empty values) so legacy rows that haven't
// been lazily migrated yet don't collide on the empty string.
func ensureAPIKeyLookupField(app core.App) error {
	collection, err := app.FindCollectionByNameOrId("tenants")
	if err != nil {
		return fmt.Errorf("tenants collection not found: %w", err)
	}
	if collection.Fields.GetByName("api_key_lookup") != nil {
		return nil
	}
	collection.Fields.Add(&core.TextField{Name: "api_key_lookup", Hidden: true})
	collection.Indexes = append(collection.Indexes,
		"CREATE UNIQUE INDEX idx_tenants_api_key_lookup ON tenants (api_key_lookup) WHERE api_key_lookup IS NOT NULL AND api_key_lookup != ''")
	if err := app.Save(collection); err != nil {
		return fmt.Errorf("failed to add api_key_lookup field: %w", err)
	}
	log.Println("migrated tenants collection: added api_key_lookup field + unique partial index")
	return nil
}

// ensureEmailVerifiedField adds the tenants.email_verified bool to existing
// deployments that predate it (fresh boots get it from the embedded
// pb_schema.json). Idempotent: no-op once the field exists. Existing
// records default to false — which is the correct semantics (only
// verify-otp flips it to true).
func ensureEmailVerifiedField(app core.App) error {
	collection, err := app.FindCollectionByNameOrId("tenants")
	if err != nil {
		return fmt.Errorf("tenants collection not found: %w", err)
	}
	if collection.Fields.GetByName("email_verified") != nil {
		return nil
	}
	collection.Fields.Add(&core.BoolField{
		Name: "email_verified",
		Help: "True once the tenant has completed OTP verification (verify-otp). Set false on self-signup and on webhook-created tenants; the dashboard shows this state.",
	})
	if err := app.Save(collection); err != nil {
		return fmt.Errorf("failed to add email_verified field: %w", err)
	}
	log.Println("migrated tenants collection: added email_verified field")
	return nil
}

// ensurePasswordHashField adds the hidden tenants.password_hash text field
// to existing deployments that predate password login (fresh boots get it
// from the embedded pb_schema.json). Idempotent: no-op once the field
// exists. Existing records keep an empty password_hash — the correct
// semantics, since only the account holder (via an authenticated session)
// can set one.
func ensurePasswordHashField(app core.App) error {
	collection, err := app.FindCollectionByNameOrId("tenants")
	if err != nil {
		return fmt.Errorf("tenants collection not found: %w", err)
	}
	if collection.Fields.GetByName("password_hash") != nil {
		return nil
	}
	collection.Fields.Add(&core.TextField{
		Name:   "password_hash",
		Hidden: true,
		Help:   "Bcrypt hash of the optional web login password (set via the account dashboard). Empty for OTP-only accounts; login-with-password requires this field.",
	})
	if err := app.Save(collection); err != nil {
		return fmt.Errorf("failed to add password_hash field: %w", err)
	}
	log.Println("migrated tenants collection: added password_hash field")
	return nil
}

// ensurePasswordResetAtField adds the tenants.password_reset_at date field
// to existing deployments that predate the forgot-password flow (fresh
// boots get it from the embedded pb_schema.json). Idempotent: no-op once
// the field exists. Existing records keep a zero value — the correct
// semantics, since the 7-day reset cooldown only starts after a completed
// reset (see web_password.go).
func ensurePasswordResetAtField(app core.App) error {
	collection, err := app.FindCollectionByNameOrId("tenants")
	if err != nil {
		return fmt.Errorf("tenants collection not found: %w", err)
	}
	if collection.Fields.GetByName("password_reset_at") != nil {
		return nil
	}
	collection.Fields.Add(&core.DateField{Name: "password_reset_at"})
	if err := app.Save(collection); err != nil {
		return fmt.Errorf("failed to add password_reset_at field: %w", err)
	}
	log.Println("migrated tenants collection: added password_reset_at field")
	return nil
}

// normalizePEM attempts to repair common formatting issues that occur when
// a multi-line PEM key is stored as an environment variable (e.g. in
// Northflank, Docker secrets, or CI/CD variables). It handles:
//   - The entire PEM on a single line (newlines stripped by the platform)
//   - Literal "\\n" escape sequences (double-escaped in JSON/YAML)
//   - Surrounding whitespace and quotes
func normalizePEM(raw string) string {
	// Strip surrounding whitespace.
	raw = strings.TrimSpace(raw)
	// Strip surrounding quotes, then re-trim in case quotes hid whitespace.
	raw = strings.TrimSpace(strings.Trim(raw, "\"'"))

	// Replace literal backslash-n sequences with real newlines.
	raw = strings.ReplaceAll(raw, "\\n", "\n")

	// If the PEM already has newlines in the expected places, return as-is.
	if strings.Contains(raw, "-----\n") || strings.Contains(raw, "-----\r\n") {
		return raw
	}

	// If there are no PEM markers at all, the user may have pasted only
	// the raw base64 body. Wrap it in a PKCS#8 PEM envelope.
	if !strings.Contains(raw, "-----BEGIN") && !strings.Contains(raw, "-----END") {
		return wrapPEM(raw, "PRIVATE KEY")
	}

	// The PEM is on a single line. Find the BEGIN and END marker boundaries.
	// Format: -----BEGIN <TYPE>-----<base64>-----END <TYPE>-----
	// The header line is everything from the first "-----" through the next "-----".
	beginMarker := strings.Index(raw, "-----BEGIN ")
	if beginMarker == -1 {
		return raw // not a recognizable PEM, let pem.Decode fail naturally
	}

	// The header closes with "-----" after the type name.
	// Skip past "-----BEGIN " (11 chars) to find the closing "-----".
	afterType := raw[beginMarker+11:]
	headerClose := strings.Index(afterType, "-----")
	if headerClose == -1 {
		return raw
	}
	headerClose += beginMarker + 11 + 5
	header := raw[beginMarker:headerClose]

	// Find the footer: "-----END " through its closing "-----".
	endMarker := strings.LastIndex(raw, "-----END ")
	if endMarker == -1 || endMarker < headerClose {
		return raw
	}
	afterEndType := raw[endMarker+9:] // skip "-----END "
	footerClose := strings.Index(afterEndType, "-----")
	if footerClose == -1 {
		return raw
	}
	footerClose += endMarker + 9 + 5
	footer := raw[endMarker:footerClose]

	base64data := raw[headerClose:endMarker]

	// Reconstruct with proper line breaks (64-char base64 lines).
	var sb strings.Builder
	sb.WriteString(header)
	sb.WriteByte('\n')
	for i := 0; i < len(base64data); i += 64 {
		end := i + 64
		if end > len(base64data) {
			end = len(base64data)
		}
		sb.WriteString(base64data[i:end])
		sb.WriteByte('\n')
	}
	sb.WriteString(footer)
	sb.WriteByte('\n')
	return sb.String()
}

// wrapPEM wraps raw base64 data in a PEM envelope with the given type label
// and standard 64-character line width.
func wrapPEM(base64data, label string) string {
	var sb strings.Builder
	sb.WriteString("-----BEGIN ")
	sb.WriteString(label)
	sb.WriteString("-----\n")
	for i := 0; i < len(base64data); i += 64 {
		end := i + 64
		if end > len(base64data) {
			end = len(base64data)
		}
		sb.WriteString(base64data[i:end])
		sb.WriteByte('\n')
	}
	sb.WriteString("-----END ")
	sb.WriteString(label)
	sb.WriteString("-----\n")
	return sb.String()
}

// safePrefix returns the first n bytes of s, escaping non-printable chars
// for safe inclusion in log messages.
func safePrefix(s string, n int) string {
	if len(s) > n {
		s = s[:n]
	}
	return strings.ReplaceAll(s, "\n", "\\n")
}

// SubscriptionPayload is the JSON structure signed by the license server.
// This is the payload the POS stores locally and verifies against the
// embedded public key. Must stay in sync with Rust SignedSubscriptionPayload
// in crates/oz-core/src/license_verification.rs.
type SubscriptionPayload struct {
	TenantID        string   `json:"tenant_id"`
	TierKey         string   `json:"tier_key"`
	Status          string   `json:"status"`
	MaxStores       int      `json:"max_stores"`
	MaxPOSInstances int      `json:"max_pos_instances"`
	AllowedTypes    []string `json:"allowed_types"`
	StartsAt        string   `json:"starts_at"`
	ExpiresAt       string   `json:"expires_at"`
	GraceUntil      string   `json:"grace_until"`
	IssuedAt        string   `json:"issued_at"`
}

// signSubscription marshals the payload to JSON, SHA-256 hashes it,
// and signs it with the RSA-2048 private key using PKCS1v15.
func signSubscription(sub SubscriptionPayload) (payload string, signature string, err error) {
	payloadBytes, err := jsonMarshal(sub)
	if err != nil {
		return "", "", err
	}
	hash := sha256.Sum256(payloadBytes)
	sig, err := rsa.SignPKCS1v15(rand.Reader, privateKey, crypto.SHA256, hash[:])
	if err != nil {
		return "", "", err
	}
	return string(payloadBytes), base64.StdEncoding.EncodeToString(sig), nil
}

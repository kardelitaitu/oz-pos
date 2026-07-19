// Package main is the entry point for the OZ-POS license server.
// It extends PocketBase with custom Go hooks for license activation,
// renewal, and status checks with RSA-2048 signing.
package main

import (
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/x509"
	"encoding/base64"
	"encoding/pem"
	"log"
	"os"
	"strings"

	"github.com/pocketbase/pocketbase"
	"github.com/pocketbase/pocketbase/core"
)

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

	// ── Register custom license API routes ───────────────────────
	app.OnServe().BindFunc(func(se *core.ServeEvent) error {
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
		// P8-2: Machine-level revocation is integrated into the /status
		// endpoint (send revoke:true with machine_id in the request body).
		// P8-4: Public health endpoint for Docker healthcheck and monitoring.
		se.Router.GET("/api/health", handleHealth(app))
		return se.Next()
	})

	if err := app.Start(); err != nil {
		log.Fatal(err)
	}
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

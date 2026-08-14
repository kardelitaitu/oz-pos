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
//go:generate go run github.com/tc-hib/go-winres@v0.3.3 make --arch amd64
package main

import (
	_ "embed"
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/x509"
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
		// P8-4: /api/health is now served by PocketBase's built-in endpoint (v0.39.6+).
		// The custom handler (health.go) is retained for reference but NOT registered
		// to avoid a route-conflict panic with PocketBase's own /api/health route.
		// se.Router.GET("/api/health", handleHealth(app))
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

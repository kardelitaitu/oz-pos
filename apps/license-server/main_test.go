package main

import (
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/x509"
	"encoding/base64"
	"encoding/pem"
	"strings"
	"testing"
	"time"
)

// ── Tests: Rate Limiting ──────────────────────────────────────────

func TestRateLimiter_AllowsThenBlocks(t *testing.T) {
	rl := &rateLimiter{buckets: make(map[string]*tokenBucket), maxPerHr: 3}
	ip := "192.168.1.1"
	for i := 0; i < 3; i++ {
		if !rl.allow(ip) {
			t.Errorf("request %d should be allowed", i+1)
		}
	}
	if rl.allow(ip) {
		t.Error("4th request should be blocked")
	}
}

func TestRateLimiter_RefillsAfterHour(t *testing.T) {
	rl := &rateLimiter{buckets: make(map[string]*tokenBucket), maxPerHr: 3}
	ip := "192.168.1.2"
	for i := 0; i < 3; i++ {
		rl.allow(ip)
	}
	if rl.allow(ip) {
		t.Error("should be blocked after exhausting tokens")
	}
	rl.mu.Lock()
	rl.buckets[ip].lastFill = time.Now().Add(-2 * time.Hour)
	rl.mu.Unlock()
	if !rl.allow(ip) {
		t.Error("should be allowed after refill period")
	}
}

func TestKeyFailureTracker_BlocksAfterLimit(t *testing.T) {
	kf := &keyFailureTracker{failures: make(map[string]*keyFailures), maxAttempts: 3, cooldown: time.Hour}
	key := "OZ-TEST-BRUTE"
	for i := 0; i < 3; i++ {
		if kf.isBlocked(key) {
			t.Errorf("should not be blocked after %d failures", i)
		}
		kf.recordFailure(key)
	}
	if !kf.isBlocked(key) {
		t.Error("should be blocked after 3 failures")
	}
}

func TestKeyFailureTracker_CooldownExpires(t *testing.T) {
	kf := &keyFailureTracker{failures: make(map[string]*keyFailures), maxAttempts: 3, cooldown: 1 * time.Millisecond}
	key := "OZ-TEST-COOLDOWN"
	for i := 0; i < 3; i++ {
		kf.recordFailure(key)
	}
	if !kf.isBlocked(key) {
		t.Error("should be blocked after 3 failures")
	}
	time.Sleep(10 * time.Millisecond)
	if kf.isBlocked(key) {
		t.Error("should not be blocked after cooldown expires")
	}
}

// ── Tests: Expiry ─────────────────────────────────────────────────

func TestCalculateExpiry_Pro(t *testing.T) {
	exp := calculateExpiry("pro")
	diff := exp.Sub(time.Now().UTC().AddDate(1, 0, 0))
	if diff > time.Hour || diff < -time.Hour {
		t.Errorf("pro expiry should be ~1 year, got diff %v", diff)
	}
}

func TestCalculateExpiry_Free(t *testing.T) {
	exp := calculateExpiry("free")
	diff := exp.Sub(time.Now().UTC().AddDate(100, 0, 0))
	if diff > time.Hour || diff < -time.Hour {
		t.Errorf("free expiry should be ~100 years, got diff %v", diff)
	}
}

func TestCalculateExpiry_Enterprise(t *testing.T) {
	exp := calculateExpiry("enterprise")
	diff := exp.Sub(time.Now().UTC().AddDate(3, 0, 0))
	if diff > time.Hour || diff < -time.Hour {
		t.Errorf("enterprise expiry should be ~3 years, got diff %v", diff)
	}
}

func TestCalculateGraceUntil(t *testing.T) {
	expiresAt := time.Date(2027, 1, 1, 0, 0, 0, 0, time.UTC)
	grace := calculateGraceUntil(expiresAt)
	if !grace.Equal(time.Date(2027, 1, 15, 0, 0, 0, 0, time.UTC)) {
		t.Errorf("grace_until should be 2027-01-15, got %v", grace)
	}
}

// ── Tests: Signature Round-Trip ───────────────────────────────────

func TestSignAndVerify_RoundTrip(t *testing.T) {
	initPrivateKey(t)

	sub := SubscriptionPayload{
		TenantID: "tenant-roundtrip", TierKey: "pro", Status: "active",
		MaxStores: 2, MaxPOSInstances: 3,
		AllowedTypes: []string{"restaurant-pos", "store-pos", "admin"},
		StartsAt: time.Now().UTC().Format(time.RFC3339),
		ExpiresAt: time.Now().UTC().AddDate(1, 0, 0).Format(time.RFC3339),
		GraceUntil: time.Now().UTC().AddDate(1, 0, 14).Format(time.RFC3339),
		IssuedAt: time.Now().UTC().Format(time.RFC3339),
	}
	payload, sig, err := signSubscription(sub)
	if err != nil {
		t.Fatalf("signing failed: %v", err)
	}
	verifySignature(t, payload, sig)
}

func TestSignAndVerify_TamperedPayload(t *testing.T) {
	initPrivateKey(t)

	sub := SubscriptionPayload{
		TenantID: "tenant-tamper", TierKey: "pro", Status: "active",
		MaxStores: 2, MaxPOSInstances: 3,
		AllowedTypes: []string{"restaurant-pos"},
		StartsAt: time.Now().UTC().Format(time.RFC3339),
		ExpiresAt: time.Now().UTC().AddDate(1, 0, 0).Format(time.RFC3339),
		GraceUntil: time.Now().UTC().AddDate(1, 0, 14).Format(time.RFC3339),
		IssuedAt: time.Now().UTC().Format(time.RFC3339),
	}
	payload, sig, err := signSubscription(sub)
	if err != nil {
		t.Fatalf("signing failed: %v", err)
	}
	tampered := strings.Replace(payload, `"pro"`, `"enterprise"`, 1)
	if err := verifySignatureHelper(tampered, sig); err == nil {
		t.Error("verification should fail on tampered payload")
	}
}

func verifySignature(t *testing.T, payload, sigBase64 string) {
	t.Helper()
	if err := verifySignatureHelper(payload, sigBase64); err != nil {
		t.Fatalf("signature verification failed: %v", err)
	}
}

func verifySignatureHelper(payload, sigBase64 string) error {
	sig, err := base64.StdEncoding.DecodeString(sigBase64)
	if err != nil {
		return err
	}
	hash := sha256.Sum256([]byte(payload))
	return rsa.VerifyPKCS1v15(&privateKey.PublicKey, crypto.SHA256, hash[:], sig)
}

// ── Tests: API Key Generation ─────────────────────────────────────

func TestGenerateAPIKey_Unique(t *testing.T) {
	keys := make(map[string]bool)
	for i := 0; i < 100; i++ {
		k := generateAPIKey()
		if keys[k] {
			t.Errorf("duplicate API key generated: %s", k)
		}
		keys[k] = true
		if len(k) < 32 {
			t.Errorf("API key too short: %s (%d chars)", k, len(k))
		}
	}
}

// ── Tests: PEM Key Loading ────────────────────────────────────────

func TestLoadRSAKey_PKCS1(t *testing.T) {
	testKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	pkcs1DER := x509.MarshalPKCS1PrivateKey(testKey)
	pemBytes := pem.EncodeToMemory(&pem.Block{Type: "RSA PRIVATE KEY", Bytes: pkcs1DER})

	block, _ := pem.Decode(pemBytes)
	if block == nil {
		t.Fatal("should decode PEM block")
	}
	parsed, err := x509.ParsePKCS1PrivateKey(block.Bytes)
	if err != nil {
		t.Fatalf("should parse PKCS1 key: %v", err)
	}
	if parsed == nil {
		t.Fatal("parsed key should not be nil")
	}
}

func TestLoadRSAKey_PKCS8(t *testing.T) {
	testKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	pkcs8DER, err := x509.MarshalPKCS8PrivateKey(testKey)
	if err != nil {
		t.Fatalf("marshal PKCS8: %v", err)
	}
	pemBytes := pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: pkcs8DER})

	block, _ := pem.Decode(pemBytes)
	if block == nil {
		t.Fatal("should decode PEM block")
	}
	parsed, err := x509.ParsePKCS8PrivateKey(block.Bytes)
	if err != nil {
		t.Fatalf("should parse PKCS8 key: %v", err)
	}
	rsaKey, ok := parsed.(*rsa.PrivateKey)
	if !ok {
		t.Fatal("parsed key should be RSA")
	}
	if rsaKey == nil {
		t.Fatal("rsa key should not be nil")
	}
}

// initPrivateKey ensures the package-level privateKey is set for unit tests
// that don't call newTestApp (e.g., signing round-trip tests).
func initPrivateKey(t testing.TB) {
	t.Helper()
	if privateKey == nil {
		key, err := rsa.GenerateKey(rand.Reader, 2048)
		if err != nil {
			t.Fatalf("failed to generate RSA key: %v", err)
		}
		privateKey = key
	}
}

// ── Tests: PEM Normalization ──────────────────────────────────────

func TestNormalizePEM_AlreadyValid(t *testing.T) {
	testKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	pkcs8DER, _ := x509.MarshalPKCS8PrivateKey(testKey)
	original := string(pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: pkcs8DER}))

	result := normalizePEM(original)
	// normalizePEM trims whitespace, so the result may differ from original
	// but must still decode as valid PEM.
	block, _ := pem.Decode([]byte(result))
	if block == nil {
		t.Fatal("already-valid PEM should decode successfully after normalization")
	}
	_, err := x509.ParsePKCS8PrivateKey(block.Bytes)
	if err != nil {
		t.Fatalf("already-valid PEM should parse after normalization: %v", err)
	}
}

func TestNormalizePEM_SingleLine(t *testing.T) {
	testKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	pkcs8DER, _ := x509.MarshalPKCS8PrivateKey(testKey)
	valid := string(pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: pkcs8DER}))

	// Strip all newlines to simulate Northflank-style single-line key.
	singleLine := strings.ReplaceAll(valid, "\n", "")

	result := normalizePEM(singleLine)
	if !strings.Contains(result, "-----\n") {
		t.Error("single-line PEM should be re-wrapped with line breaks")
	}

	// The result must be decodable.
	block, _ := pem.Decode([]byte(result))
	if block == nil {
		t.Fatal("normalized single-line PEM should decode successfully")
	}
	_, err := x509.ParsePKCS8PrivateKey(block.Bytes)
	if err != nil {
		t.Fatalf("normalized PEM should parse as PKCS8: %v", err)
	}
}

func TestNormalizePEM_LiteralBackslashN(t *testing.T) {
	testKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	pkcs8DER, _ := x509.MarshalPKCS8PrivateKey(testKey)
	valid := string(pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: pkcs8DER}))

	// Replace real newlines with literal \n (double-escaped in JSON/YAML env vars).
	literal := strings.ReplaceAll(valid, "\n", "\\n")

	result := normalizePEM(literal)
	if strings.Contains(result, "\\n") {
		t.Error("literal \\n should be converted to real newlines")
	}
	block, _ := pem.Decode([]byte(result))
	if block == nil {
		t.Fatal("PEM with literal \\n should decode after normalization")
	}
}

func TestNormalizePEM_SurroundingQuotes(t *testing.T) {
	testKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	pkcs8DER, _ := x509.MarshalPKCS8PrivateKey(testKey)
	valid := string(pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: pkcs8DER}))

	quoted := "\"  " + valid + "  \""
	result := normalizePEM(quoted)
	if strings.HasPrefix(result, "\"") {
		t.Error("surrounding quotes should be stripped")
	}
	block, _ := pem.Decode([]byte(result))
	if block == nil {
		t.Fatal("quoted PEM should decode after normalization")
	}
}

func TestNormalizePEM_SurroundingWhitespace(t *testing.T) {
	testKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	pkcs8DER, _ := x509.MarshalPKCS8PrivateKey(testKey)
	valid := string(pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: pkcs8DER}))

	spaced := "  \n  " + valid + "  \n  "
	result := normalizePEM(spaced)
	block, _ := pem.Decode([]byte(result))
	if block == nil {
		t.Fatal("whitespace-padded PEM should decode after normalization")
	}
}

// ── Tests: wrapPEM (raw base64) ───────────────────────────────────

func TestWrapPEM_RawBase64(t *testing.T) {
	testKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	pkcs8DER, _ := x509.MarshalPKCS8PrivateKey(testKey)
	valid := string(pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: pkcs8DER}))

	// Extract just the base64 body (strip PEM headers).
	lines := strings.Split(strings.TrimSpace(valid), "\n")
	var rawBase64 strings.Builder
	for _, line := range lines {
		if strings.HasPrefix(line, "-----") {
			continue
		}
		rawBase64.WriteString(line)
	}

	result := normalizePEM(rawBase64.String())
	if !strings.HasPrefix(result, "-----BEGIN PRIVATE KEY-----") {
		t.Error("raw base64 should be wrapped in PKCS#8 PEM envelope")
	}
	block, _ := pem.Decode([]byte(result))
	if block == nil {
		t.Fatal("wrapped base64 should produce valid PEM")
	}
}

func TestWrapPEM_StripWhitespace(t *testing.T) {
	testKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	pkcs8DER, _ := x509.MarshalPKCS8PrivateKey(testKey)
	valid := string(pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: pkcs8DER}))

	// Extract base64 with embedded newlines (64-char wrapped, no headers).
	lines := strings.Split(strings.TrimSpace(valid), "\n")
	var rawWithNewlines strings.Builder
	for _, line := range lines {
		if strings.HasPrefix(line, "-----") {
			continue
		}
		rawWithNewlines.WriteString(line)
		rawWithNewlines.WriteString("\n")
	}

	result := normalizePEM(rawWithNewlines.String())
	// Result should have exactly 64-char lines (not doubled newlines).
	block, _ := pem.Decode([]byte(result))
	if block == nil {
		t.Fatal("base64 with embedded newlines should produce valid PEM")
	}
}

func TestWrapPEM_RSAKeyType(t *testing.T) {
	testKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	pkcs1DER := x509.MarshalPKCS1PrivateKey(testKey)
	valid := string(pem.EncodeToMemory(&pem.Block{Type: "RSA PRIVATE KEY", Bytes: pkcs1DER}))

	// Extract base64 from PKCS1 PEM, wrap in PKCS8 envelope.
	lines := strings.Split(strings.TrimSpace(valid), "\n")
	var rawBase64 strings.Builder
	for _, line := range lines {
		if strings.HasPrefix(line, "-----") {
			continue
		}
		rawBase64.WriteString(line)
	}

	// normalizePEM wraps raw base64 in PKCS8 format by default.
	result := normalizePEM(rawBase64.String())
	block, _ := pem.Decode([]byte(result))
	if block == nil {
		t.Fatal("wrapped raw base64 should produce valid PEM")
	}
	// The DER bytes are PKCS1, so ParsePKCS1PrivateKey should succeed
	// regardless of the PEM "PRIVATE KEY" type label.
	parsed, err := x509.ParsePKCS1PrivateKey(block.Bytes)
	if err != nil {
		t.Fatalf("PKCS1 DER should parse with ParsePKCS1PrivateKey: %v", err)
	}
	if parsed == nil {
		t.Fatal("parsed key should not be nil")
	}
}

// ── Tests: Helpers ────────────────────────────────────────────────

func TestStrDefault_NonEmpty(t *testing.T) {
	if strDefault("hello", "world") != "hello" {
		t.Error("strDefault should return s when non-empty")
	}
}

func TestStrDefault_Empty(t *testing.T) {
	if strDefault("", "default") != "default" {
		t.Error("strDefault should return d when s is empty")
	}
}

func TestStrDefault_BothEmpty(t *testing.T) {
	if strDefault("", "") != "" {
		t.Error("strDefault should return empty string when both are empty")
	}
}

func TestGenerateAPIKey_HasPrefix(t *testing.T) {
	k := generateAPIKey()
	if !strings.HasPrefix(k, "oz_") {
		t.Errorf("API key should have 'oz_' prefix, got: %s", k)
	}
}

// ── Tests: Rate Limiter Edge Cases ────────────────────────────────

func TestRateLimiter_IsolatedByIP(t *testing.T) {
	rl := &rateLimiter{buckets: make(map[string]*tokenBucket), maxPerHr: 3}
	ip1 := "192.168.1.1"
	ip2 := "192.168.1.2"

	// Exhaust ip1.
	for i := 0; i < 3; i++ {
		rl.allow(ip1)
	}
	if rl.allow(ip1) {
		t.Error("ip1 should be blocked after exhausting tokens")
	}
	// ip2 should still have full quota.
	for i := 0; i < 3; i++ {
		if !rl.allow(ip2) {
			t.Errorf("ip2 request %d should be allowed (not affected by ip1)", i+1)
		}
	}
}

func TestRateLimiter_DefaultMaxPerHr(t *testing.T) {
	if ipRateLimiter.maxPerHr != 5 {
		t.Errorf("global ipRateLimiter should default to 5 per hour, got %d", ipRateLimiter.maxPerHr)
	}
}

// ── Tests: Key Failure Tracker Edge Cases ─────────────────────────

func TestKeyFailureTracker_IsolatedByKey(t *testing.T) {
	kf := &keyFailureTracker{failures: make(map[string]*keyFailures), maxAttempts: 3, cooldown: time.Hour}
	key1 := "OZ-KEY-1"
	key2 := "OZ-KEY-2"

	// Exhaust key1.
	for i := 0; i < 3; i++ {
		kf.recordFailure(key1)
	}
	if !kf.isBlocked(key1) {
		t.Error("key1 should be blocked after 3 failures")
	}
	// key2 should not be affected.
	if kf.isBlocked(key2) {
		t.Error("key2 should not be blocked (isolated from key1)")
	}
}

func TestKeyFailureTracker_PartialFailures(t *testing.T) {
	kf := &keyFailureTracker{failures: make(map[string]*keyFailures), maxAttempts: 3, cooldown: time.Hour}
	key := "OZ-PARTIAL"

	// 2 failures should not block.
	kf.recordFailure(key)
	kf.recordFailure(key)
	if kf.isBlocked(key) {
		t.Error("should not be blocked after only 2 failures")
	}
	// 3rd failure should block.
	kf.recordFailure(key)
	if !kf.isBlocked(key) {
		t.Error("should be blocked after 3rd failure")
	}
}

func TestKeyFailureTracker_CleanupAfterCooldown(t *testing.T) {
	kf := &keyFailureTracker{failures: make(map[string]*keyFailures), maxAttempts: 3, cooldown: 100 * time.Millisecond}
	key := "OZ-CLEANUP"

	for i := 0; i < 3; i++ {
		kf.recordFailure(key)
	}
	if !kf.isBlocked(key) {
		t.Error("should be blocked after 3 failures")
	}

	time.Sleep(150 * time.Millisecond)

	// After cooldown, the entry should be cleaned up on next check.
	if kf.isBlocked(key) {
		t.Error("should not be blocked after cooldown expires")
	}
	// New failures should start fresh.
	kf.recordFailure(key)
	if kf.isBlocked(key) {
		t.Error("should not be blocked after single fresh failure")
	}
}

// ── Tests: Expiry Edge Cases ──────────────────────────────────────

func TestCalculateExpiry_Premium(t *testing.T) {
	exp := calculateExpiry("premium")
	diff := exp.Sub(time.Now().UTC().AddDate(1, 0, 0))
	if diff > time.Hour || diff < -time.Hour {
		t.Errorf("premium expiry should be ~1 year, got diff %v", diff)
	}
}

func TestCalculateExpiry_Unknown(t *testing.T) {
	exp := calculateExpiry("unknown-tier")
	diff := exp.Sub(time.Now().UTC().AddDate(1, 0, 0))
	if diff > time.Hour || diff < -time.Hour {
		t.Errorf("unknown tier should default to ~1 year, got diff %v", diff)
	}
}

func TestCalculateExpiry_SameLength(t *testing.T) {
	pro := calculateExpiry("pro")
	premium := calculateExpiry("premium")
	diff := pro.Sub(premium)
	if diff > time.Hour || diff < -time.Hour {
		t.Errorf("pro and premium should have same expiry, got diff %v", diff)
	}
}

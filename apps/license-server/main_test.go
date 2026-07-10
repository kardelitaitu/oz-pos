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

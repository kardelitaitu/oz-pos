package main

import (
	"strings"
	"testing"
)

// ── normalizeBundleID ────────────────────────────────────────────────

func TestNormalizeBundleID_RestaurantStarter(t *testing.T) {
	if got := normalizeBundleID("restaurant_starter"); got != "restaurant_starter" {
		t.Errorf("expected restaurant_starter, got %q", got)
	}
}

func TestNormalizeBundleID_CaseInsensitive(t *testing.T) {
	for _, input := range []string{"Restaurant_Starter", "RESTAURANT_STARTER", "ReStAuRaNt_StArTeR"} {
		if got := normalizeBundleID(input); got != "restaurant_starter" {
			t.Errorf("normalizeBundleID(%q) = %q, want restaurant_starter", input, got)
		}
	}
}

func TestNormalizeBundleID_WhitespaceTrimmed(t *testing.T) {
	if got := normalizeBundleID("  restaurant_starter  "); got != "restaurant_starter" {
		t.Errorf("expected restaurant_starter with whitespace trimmed, got %q", got)
	}
}

func TestNormalizeBundleID_UnknownReturnsEmpty(t *testing.T) {
	for _, input := range []string{"", "unknown", "restaurant", "premium_kds", "  "} {
		if got := normalizeBundleID(input); got != "" {
			t.Errorf("normalizeBundleID(%q) = %q, want empty string", input, got)
		}
	}
}

// ── isBcryptHash ─────────────────────────────────────────────────────

func TestIsBcryptHash_Prefixes(t *testing.T) {
	for _, prefix := range []string{"$2a$", "$2b$", "$2y$"} {
		hash := prefix + "10$salt_and_hash_here_that_is_long_enough_for_bcrypt"
		if !isBcryptHash(hash) {
			t.Errorf("isBcryptHash(%q...) = false, want true", prefix)
		}
	}
}

func TestIsBcryptHash_PlaintextRejected(t *testing.T) {
	for _, input := range []string{
		"",
		"my-api-key",
		"not-a-hash",
		"$2x$invalid", // invalid bcrypt variant
		"plaintext-key",
	} {
		if isBcryptHash(input) {
			t.Errorf("isBcryptHash(%q) = true, want false", input)
		}
	}
}

// isBcryptHash is a prefix check — short strings like "$2a$" pass the
// prefix gate but bcrypt.CompareHashAndPassword would reject them later.
// This is the intended contract: the function tells you the FORMAT,
// not whether the hash is valid.
func TestIsBcryptHash_ShortBcryptStringPassesPrefixCheck(t *testing.T) {
	if !isBcryptHash("$2a$") {
		t.Error("$2a$ should pass the prefix check (validation happens at compare time)")
	}
}

// ── extractAPIKey ────────────────────────────────────────────────────

func TestExtractAPIKey_HappyPath(t *testing.T) {
	key, err := extractAPIKey("Bearer my-api-key-12345")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if key != "my-api-key-12345" {
		t.Errorf("expected my-api-key-12345, got %q", key)
	}
}

func TestExtractAPIKey_MissingPrefix(t *testing.T) {
	_, err := extractAPIKey("Token my-key")
	if err == nil {
		t.Error("expected error for missing Bearer prefix")
	}
}

func TestExtractAPIKey_EmptyKey(t *testing.T) {
	_, err := extractAPIKey("Bearer ")
	if err == nil {
		t.Error("expected error for empty key")
	}
}

func TestExtractAPIKey_WhitespaceTrimmed(t *testing.T) {
	key, err := extractAPIKey("Bearer   my-key  ")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if key != "my-key" {
		t.Errorf("expected my-key, got %q", key)
	}
}

func TestExtractAPIKey_EmptyHeader(t *testing.T) {
	_, err := extractAPIKey("")
	if err == nil {
		t.Error("expected error for empty header")
	}
}

// ── normalizeBillingPeriod ───────────────────────────────────────────

func TestNormalizeBillingPeriod_CanonicalPassThrough(t *testing.T) {
	if got := normalizeBillingPeriod("month"); got != "month" {
		t.Errorf("month -> %q, want month", got)
	}
	if got := normalizeBillingPeriod("year"); got != "year" {
		t.Errorf("year -> %q, want year", got)
	}
}

func TestNormalizeBillingPeriod_AliasMapping(t *testing.T) {
	if got := normalizeBillingPeriod("monthly"); got != "month" {
		t.Errorf("monthly -> %q, want month", got)
	}
	if got := normalizeBillingPeriod("yearly"); got != "year" {
		t.Errorf("yearly -> %q, want year", got)
	}
}

func TestNormalizeBillingPeriod_CaseInsensitive(t *testing.T) {
	if got := normalizeBillingPeriod("MONTHLY"); got != "month" {
		t.Errorf("MONTHLY -> %q, want month", got)
	}
	if got := normalizeBillingPeriod("Yearly"); got != "year" {
		t.Errorf("Yearly -> %q, want year", got)
	}
}

func TestNormalizeBillingPeriod_UnknownPassThrough(t *testing.T) {
	if got := normalizeBillingPeriod("weekly"); got != "weekly" {
		t.Errorf("weekly -> %q, want weekly (passthrough)", got)
	}
	if got := normalizeBillingPeriod(""); got != "" {
		t.Errorf("empty -> %q, want empty", got)
	}
}

func TestNormalizeBillingPeriod_WhitespaceTrimmed(t *testing.T) {
	if got := normalizeBillingPeriod("  monthly  "); got != "month" {
		t.Errorf("'  monthly  ' -> %q, want month", got)
	}
}

// ── redactRequestBody ────────────────────────────────────────────────

func TestRedactRequestBody_MasksAPIKey(t *testing.T) {
	input := []byte(`{"email":"user@test.com","api_key":"sk_live_abc123"}`)
	got := redactRequestBody(input)
	if strings.Contains(got, "sk_live_abc123") {
		t.Errorf("api_key must be redacted, got: %s", got)
	}
	if !strings.Contains(got, "[REDACTED]") {
		t.Errorf("expected [REDACTED] in output, got: %s", got)
	}
	if !strings.Contains(got, "user@test.com") {
		t.Errorf("non-sensitive fields must be preserved, got: %s", got)
	}
}

func TestRedactRequestBody_NoAPIKeyUnchanged(t *testing.T) {
	input := []byte(`{"email":"user@test.com","name":"Test"}`)
	got := redactRequestBody(input)
	if got != string(input) {
		t.Errorf("input without api_key must pass through unchanged, got: %s", got)
	}
}

func TestRedactRequestBody_InvalidJSONPassThrough(t *testing.T) {
	input := []byte(`not json at all`)
	got := redactRequestBody(input)
	if got != string(input) {
		t.Errorf("invalid JSON must pass through unchanged, got: %s", got)
	}
}

// Empty api_key is NOT redacted — an empty string is not a credential.
// This is the documented contract: "Only redact STRING api_key values"
// with the explicit `str != ""` guard.
func TestRedactRequestBody_EmptyAPIKeyPreserved(t *testing.T) {
	input := []byte(`{"api_key":""}`)
	got := redactRequestBody(input)
	if strings.Contains(got, "[REDACTED]") {
		t.Errorf("empty api_key must NOT be redacted, got: %s", got)
	}
}

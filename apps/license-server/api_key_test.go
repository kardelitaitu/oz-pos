package main

import (
	"testing"

	"github.com/pocketbase/pocketbase/core"
)

func TestHashAPIKey_StoresBcryptAndDeterministicLookup(t *testing.T) {
	key := generateAPIKey()
	hash, lookup, err := hashAPIKey(key)
	if err != nil {
		t.Fatalf("hashAPIKey failed: %v", err)
	}
	if !isBcryptHash(hash) {
		t.Fatalf("expected bcrypt hash, got %q", hash)
	}
	if hash == key {
		t.Fatal("hash must not equal the plaintext key")
	}
	if lookup != apiKeyLookup(key) {
		t.Fatal("lookup must be the deterministic SHA-256 of the key")
	}
}

func TestHashAPIKey_UniqueLookups(t *testing.T) {
	a := generateAPIKey()
	b := generateAPIKey()
	if apiKeyLookup(a) == apiKeyLookup(b) {
		t.Fatal("distinct keys must have distinct lookup hashes")
	}
}

func TestVerifyAPIKey_Bcrypt(t *testing.T) {
	key := generateAPIKey()
	hash, _, err := hashAPIKey(key)
	if err != nil {
		t.Fatalf("hashAPIKey failed: %v", err)
	}
	if !verifyAPIKey(hash, key) {
		t.Fatal("correct key must verify against its bcrypt hash")
	}
	if verifyAPIKey(hash, "wrong-"+key) {
		t.Fatal("wrong key must not verify")
	}
	if verifyAPIKey(hash, "") {
		t.Fatal("empty key must not verify")
	}
}

func TestVerifyAPIKey_LegacyPlaintext(t *testing.T) {
	legacy := "legacy-plaintext-key-0001"
	if isBcryptHash(legacy) {
		t.Fatal("test fixture must be a non-bcrypt plaintext key")
	}
	if !verifyAPIKey(legacy, legacy) {
		t.Fatal("legacy plaintext key must verify (constant-time)")
	}
	if verifyAPIKey(legacy, "other") {
		t.Fatal("mismatched legacy key must not verify")
	}
}

func TestFindTenantByAPIKey_ResolvesHashedTenant(t *testing.T) {
	resetRateLimiters()
	app, _ := setupDirectApp(t)
	defer app.Cleanup()

	seedTenant(t, app, "findhashed00001", "findhashedkey001", "active")

	rec, err := findTenantByAPIKey(app, "findhashedkey001")
	if err != nil {
		t.Fatalf("findTenantByAPIKey should resolve hashed tenant: %v", err)
	}
	if rec.Id != "findhashed00001" {
		t.Errorf("expected tenant findhashed00001, got %s", rec.Id)
	}
}

func TestFindTenantByAPIKey_UnknownKey(t *testing.T) {
	resetRateLimiters()
	app, _ := setupDirectApp(t)
	defer app.Cleanup()

	if rec, err := findTenantByAPIKey(app, "notarealkey00001"); err == nil {
		t.Fatalf("unknown key must not resolve a tenant, got %q", rec.Id)
	}
}

func TestFindTenantByAPIKey_LazilyMigratesLegacyPlaintext(t *testing.T) {
	resetRateLimiters()
	app, _ := setupDirectApp(t)
	defer app.Cleanup()

	// Seed a legacy plaintext tenant directly (bypassing seedTenant's
	// hashing) to simulate a tenant created before api_key hashing.
	col, err := app.FindCollectionByNameOrId("tenants")
	if err != nil {
		t.Fatalf("tenants collection not found: %v", err)
	}
	rec := core.NewRecord(col)
	rec.Set("id", "legacyplain0001")
	rec.Set("email", "legacyplain0001@example.com")
	rec.Set("phone", "-")
	rec.Set("api_key", "legacyplainkey01")
	rec.Set("status", "active")
	if err := app.Save(rec); err != nil {
		t.Fatalf("failed to seed legacy tenant: %v", err)
	}

	got, err := findTenantByAPIKey(app, "legacyplainkey01")
	if err != nil {
		t.Fatalf("legacy plaintext tenant must authenticate: %v", err)
	}
	if got.Id != "legacyplain0001" {
		t.Errorf("expected tenant legacyplain0001, got %s", got.Id)
	}

	// After the lookup, the row must be upgraded to a bcrypt hash + lookup.
	upgraded, err := app.FindRecordById("tenants", "legacyplain0001")
	if err != nil {
		t.Fatalf("failed to re-read migrated tenant: %v", err)
	}
	if !isBcryptHash(upgraded.GetString("api_key")) {
		t.Errorf("expected migrated bcrypt hash, got %q", upgraded.GetString("api_key"))
	}
	if upgraded.GetString("api_key_lookup") != apiKeyLookup("legacyplainkey01") {
		t.Error("expected api_key_lookup to be set after migration")
	}
}

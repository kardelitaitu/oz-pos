package main

import (
	"crypto/sha256"
	"crypto/subtle"
	"encoding/hex"
	"errors"
	"log"
	"strings"

	"github.com/pocketbase/pocketbase/core"
	"golang.org/x/crypto/bcrypt"
)

// errTenantNotFound is returned by findTenantByAPIKey when no tenant record
// matches the presented api_key (or when the bcrypt verification fails).
var errTenantNotFound = errors.New("tenant not found by api_key")

// apiKeyLookup returns the deterministic lookup hash (hex SHA-256) of an
// api_key. The lookup hash is the only indexed representation of the key:
// bcrypt hashes are salted and therefore cannot drive an equality lookup,
// but the api_key is a 256-bit CSPRNG secret, so a fast SHA-256 of it is
// un-invertible and safe to store and index.
func apiKeyLookup(apiKey string) string {
	sum := sha256.Sum256([]byte(apiKey))
	return hex.EncodeToString(sum[:])
}

// hashAPIKey derives the at-rest representation of an api_key: a bcrypt
// hash for verification plus the SHA-256 lookup hash for O(1) tenant
// resolution. The plaintext key is never stored.
func hashAPIKey(apiKey string) (hash string, lookup string, err error) {
	h, err := bcrypt.GenerateFromPassword([]byte(apiKey), bcrypt.DefaultCost)
	if err != nil {
		return "", "", err
	}
	return string(h), apiKeyLookup(apiKey), nil
}

// isBcryptHash reports whether stored looks like a bcrypt hash (the
// "$2a$", "$2b$", "$2y$" prefixes) rather than a legacy plaintext api_key.
func isBcryptHash(stored string) bool {
	return strings.HasPrefix(stored, "$2a$") ||
		strings.HasPrefix(stored, "$2b$") ||
		strings.HasPrefix(stored, "$2y$")
}

// verifyAPIKey checks a presented api_key against the stored at-rest value.
// It transparently handles legacy plaintext rows (constant-time compare) so
// tenants created before hashing continue to authenticate.
func verifyAPIKey(stored, presented string) bool {
	if stored == "" {
		return false
	}
	if isBcryptHash(stored) {
		return bcrypt.CompareHashAndPassword([]byte(stored), []byte(presented)) == nil
	}
	// Legacy plaintext: constant-time compare so the key comparison doesn't
	// leak timing information.
	return subtle.ConstantTimeCompare([]byte(stored), []byte(presented)) == 1
}

// findTenantByAPIKey resolves a tenant by its api_key, verifying the key
// against the stored bcrypt hash. It first tries the indexed lookup hash;
// if that misses (a legacy tenant created before the lookup column was
// added), it scans tenants once and upgrades any plaintext match in place.
func findTenantByAPIKey(app core.App, apiKey string) (*core.Record, error) {
	lookup := apiKeyLookup(apiKey)

	// Fast path: deterministic indexed lookup.
	if rec, err := app.FindFirstRecordByData("tenants", "api_key_lookup", lookup); err == nil {
		if verifyAPIKey(rec.GetString("api_key"), apiKey) {
			return rec, nil
		}
		// The lookup hash matched but bcrypt verification failed. Treat as
		// unknown rather than scanning — a SHA-256 collision is not realistic
		// for a CSPRNG key, and falling through would mask corruption.
		return nil, errTenantNotFound
	}

	// Slow path: legacy tenants predate the api_key_lookup column. Scan once
	// and upgrade any matching plaintext row to its hashed form.
	records, err := app.FindAllRecords("tenants")
	if err != nil {
		return nil, err
	}
	for _, rec := range records {
		stored := rec.GetString("api_key")
		if stored == "" {
			continue
		}
		if isBcryptHash(stored) {
			// Already hashed but the lookup hash was missing (stale row);
			// still verify so the tenant can authenticate.
			if bcrypt.CompareHashAndPassword([]byte(stored), []byte(apiKey)) == nil {
				return rec, nil
			}
			continue
		}
		// Legacy plaintext row.
		if subtle.ConstantTimeCompare([]byte(stored), []byte(apiKey)) == 1 {
			if migrateErr := migrateTenantAPIKey(app, rec, apiKey, lookup); migrateErr != nil {
				// Best-effort upgrade: the plaintext compare already succeeded,
				// so authenticate even if the migration write fails.
				log.Printf("api_key migration failed for tenant %q: %v", rec.Id, migrateErr)
			}
			return rec, nil
		}
	}
	return nil, errTenantNotFound
}

// migrateTenantAPIKey upgrades a legacy plaintext api_key to its hashed,
// at-rest form: a bcrypt hash plus the SHA-256 lookup hash.
func migrateTenantAPIKey(app core.App, rec *core.Record, apiKey, lookup string) error {
	hash, err := bcrypt.GenerateFromPassword([]byte(apiKey), bcrypt.DefaultCost)
	if err != nil {
		return err
	}
	rec.Set("api_key", string(hash))
	rec.Set("api_key_lookup", lookup)
	return app.Save(rec)
}

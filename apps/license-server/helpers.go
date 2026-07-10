package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"time"
)

// jsonMarshal is a thin wrapper around json.Marshal for readability.
func jsonMarshal(v any) ([]byte, error) {
	return json.Marshal(v)
}

// generateAPIKey returns a 32-byte hex-encoded random string used as the
// tenant's API key for renew and status endpoints.
func generateAPIKey() string {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		// Fallback: use timestamp-based key if CSPRNG fails
		return hex.EncodeToString([]byte(time.Now().UTC().Format(time.RFC3339Nano)))
	}
	return "oz_" + hex.EncodeToString(b)
}

// strDefault returns s if non-empty, otherwise returns d.
func strDefault(s, d string) string {
	if s != "" {
		return s
	}
	return d
}

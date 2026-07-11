package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"log"
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
		// CSPRNG failure means the OS entropy source is broken.
		// A predictable fallback is worse than crashing — the system
		// is in an unsafe state and should not generate API keys.
		log.Fatalf("crypto/rand.Read failed: %v — cannot generate secure API key", err)
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

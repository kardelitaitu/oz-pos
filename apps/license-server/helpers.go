package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"log"
	"strings"
)

// bearerPrefix is the RFC 6750 scheme required on the Authorization header.
// The scheme itself ("Bearer") is case-sensitive per RFC 7235 §2.1; the
// header NAME is case-insensitive and normalized by Go's http.Header.Get.
// Declared once here so activate/renew/status all share the same prefix
// check (avoids drift if a future handler forgets to use "Bearer ").
const bearerPrefix = "Bearer "

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

// extractAPIKey resolves the api_key from the Authorization: Bearer
// header (preferred) or the legacy JSON body field (backward-compat
// for C1-pre-audit wire format). Returns:
//
//   - apiKey: the resolved api_key, or "" if neither source provided one
//   - usedBodyFallback: true iff the body field was the actual auth
//     source (no Bearer header was present). Callers use this to decide
//     whether to log a deprecation warning nudging the operator toward
//     the Bearer header — which keeps the credential out of CDN /
//     webserver access logs that capture request bodies.
//   - err: non-nil iff both sources were present and disagreed (ambiguous
//     credential parsing must be rejected; we refuse to guess which the
//     client intended).
//
// The Bearer path trims surrounding whitespace so trailing spaces after
// the token (a common copy-paste quirk) don't silently invalidate auth.
func extractAPIKey(reqAPIKey, authHeader string) (apiKey string, usedBodyFallback bool, err error) {
	if strings.HasPrefix(authHeader, bearerPrefix) {
		bearerKey := strings.TrimSpace(strings.TrimPrefix(authHeader, bearerPrefix))
		if reqAPIKey != "" && reqAPIKey != bearerKey {
			return "", false, errors.New("api_key in body does not match Authorization header")
		}
		// Header present (even if empty) — body fallback was NOT used.
		return bearerKey, false, nil
	}
	// No Authorization: Bearer header — body is the only auth source.
	// usedBodyFallback is true only when the body actually had a value;
	// an empty body with no header is a missing-credential case, not a
	// deprecation case.
	return reqAPIKey, reqAPIKey != "", nil
}

// redactRequestBody returns a JSON-string copy of body with the "api_key"
// field masked as "[REDACTED]". Used by handlers that want to log the
// request payload for debugging without leaking the credential into log
// files (which are typically retained longer and shared more broadly than
// request bodies, and may be scraped by log-aggregation tools).
//
// If body is not valid JSON, or has no api_key field, the original bytes
// are returned unchanged. We deliberately fall back to the raw bytes
// rather than an error string so a malformed-body log line is still
// useful for debugging the parse failure itself.
func redactRequestBody(body []byte) string {
	var payload map[string]any
	if err := json.Unmarshal(body, &payload); err != nil {
		return string(body)
	}
	// Only redact STRING api_key values. The wire format is always a
	// string, but if a malformed request sent null/numeric/object, the
	// unmarshal target would be a non-string Go type — trying to assign
	// "[REDACTED]" to it would silently coerce at json.Marshal (or, for
	// json.RawMessage, write the literal "[REDACTED]" inside a quoted
	// string anyway, but the type assertion is the clearer contract).
	// The type assertion confines the redaction to the case we know how
	// to handle safely; non-string api_key is preserved as-is so a
	// malformed-body log line still shows the original payload.
	if val, ok := payload["api_key"]; ok {
		if str, ok := val.(string); ok && str != "" {
			payload["api_key"] = "[REDACTED]"
		}
	}
	redacted, err := json.Marshal(payload)
	if err != nil {
		return string(body)
	}
	return string(redacted)
}

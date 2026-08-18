package main

// Hardware-fingerprint trial lock (SPEC-2026-TRIAL-LOCK, ADR #23 re-scope).
//
// The Free tier is free forever, but paid trials are segmented by signup
// vertical (subscription-tiers.md §4): 14-day Plus (general), 14-day Pro
// (restaurant/cafe), 30-day Pro (enterprise referral). The trial lock's
// job is to make those trials one-per-physical-device — reinstalling the
// app or using a fresh email must not reset the clock.
//
// POST /api/v1/license/trial registers a hardware fingerprint the first
// time it is seen and answers 403 TRIAL_ALREADY_CLAIMED on every later
// attempt, permanently — the fingerprint stays claimed even after the
// trial expires, so the lock cannot be beaten by waiting. The same gate is
// enforced inside /api/v1/license/activate for trial keys (see
// enforceTrialLock in activate.go), so a client that skips this endpoint
// is still locked at mint time.
//
// The fingerprint is the client's derived hardware identifier — the
// 15-char machine_id (SHA-256 of the motherboard/Windows MachineGuid /
// /etc/machine-id anchor) or the spec's "hw_<hex>" form of the same
// anchor. Both are stable across app reinstalls and DB wipes.

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// trialPath is the route registered in main.go.
const trialPath = "/api/v1/license/trial"

// trialClaimRequest is the JSON body of POST /api/v1/license/trial.
type trialClaimRequest struct {
	// HardwareFingerprint is the client-derived device identifier
	// (15-char machine_id or "hw_<hex>"). Required.
	HardwareFingerprint string `json:"hardware_fingerprint"`
	// Platform is the OS the trial is claimed from ("windows",
	// "android", "linux", "macos"). Required.
	Platform string `json:"platform"`
	// AppVersion is the client app version. Required.
	AppVersion string `json:"app_version"`
	// TrialVertical is the optional segmented-trial vertical
	// (subscription-tiers.md §4): "restaurant"/"cafe" → 14-day Pro,
	// "enterprise_referral" → 30-day Pro, default → 14-day Plus. It
	// decides the trial_expires_at recorded on the claim; the actual
	// subscription is still minted at activation.
	TrialVertical string `json:"trial_vertical"`
	// Email is the buyer's account email, associated with the claim so a
	// re-install by the SAME account is a re-activation (allowed) while a
	// fresh claim by a DIFFERENT account on the same device is rejected.
	// Optional at claim time — activation fills it in when missing.
	Email string `json:"email"`
}

// normalizeHardwareFingerprint validates + canonicalizes a claimed
// fingerprint: either the 15-char lowercase-hex machine_id the desktop
// sends to /activate, or the spec's "hw_<64 hex>" form. Returns "" for
// anything else (the endpoint then answers 400).
func normalizeHardwareFingerprint(fp string) string {
	fp = strings.ToLower(strings.TrimSpace(fp))
	if len(fp) == 15 {
		valid := true
		for _, c := range fp {
			if !((c >= 'a' && c <= 'f') || (c >= '0' && c <= '9')) {
				valid = false
				break
			}
		}
		if valid {
			return fp
		}
		return ""
	}
	if strings.HasPrefix(fp, "hw_") {
		rest := fp[3:]
		if len(rest) == 64 {
			valid := true
			for _, c := range rest {
				if !((c >= 'a' && c <= 'f') || (c >= '0' && c <= '9')) {
					valid = false
					break
				}
			}
			if valid {
				return fp
			}
		}
	}
	return ""
}

// handleTrial implements POST /api/v1/license/trial: register the hardware
// fingerprint for a trial claim. 200 on first claim (with the trial
// window); 403 TRIAL_ALREADY_CLAIMED when the device has already claimed
// a trial — the anti-reset gate.
func handleTrial(app core.App) func(e *core.RequestEvent) error {
	return func(e *core.RequestEvent) error {
		e.Request.Body = http.MaxBytesReader(e.Response, e.Request.Body, 16*1024)
		var req trialClaimRequest
		if err := json.NewDecoder(e.Request.Body).Decode(&req); err != nil {
			return e.JSON(http.StatusBadRequest, map[string]any{"error": "malformed JSON"})
		}
		fp := normalizeHardwareFingerprint(req.HardwareFingerprint)
		if fp == "" {
			return e.JSON(http.StatusBadRequest, map[string]any{
				"error": "hardware_fingerprint must be the 15-char machine_id or the hw_<hex> form",
			})
		}
		platform := strings.ToLower(strings.TrimSpace(req.Platform))
		if platform == "" {
			platform = "unknown"
		}
		appVersion := strings.TrimSpace(req.AppVersion)
		if appVersion == "" {
			appVersion = "unknown"
		}

		// ── The gate: one trial per physical device, permanently. ──
		if existing, err := app.FindFirstRecordByData("trial_registrations", "hardware_fingerprint", fp); err == nil {
			return e.JSON(http.StatusForbidden, map[string]any{
				"code":                 "TRIAL_ALREADY_CLAIMED",
				"message":              "A trial has already been claimed for this device. Purchase a license to continue.",
				"expired_at":           existing.GetDateTime("trial_expires_at").Time().UTC().Format(time.RFC3339),
				"hardware_fingerprint": fp,
			})
		}

		// ── First claim: record it. The subscription itself is minted
		//    at activation (the same path paid keys take), which re-checks
		//    this table — so a race between claim and activate converges. ──
		_, trialDays := trialSegmentation(req.TrialVertical)
		trialExpires := time.Now().UTC().AddDate(0, 0, trialDays)

		coll, err := app.FindCollectionByNameOrId("trial_registrations")
		if err != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{"error": "server misconfiguration: trial_registrations collection not found"})
		}
		rec := core.NewRecord(coll)
		rec.Set("hardware_fingerprint", fp)
		rec.Set("first_seen_at", time.Now().UTC())
		rec.Set("trial_expires_at", trialExpires)
		rec.Set("platform", platform)
		rec.Set("app_version", appVersion)
		rec.Set("ip_address", e.RealIP())
		// Associate the claim with the buyer's tenant when an email is
		// provided (activation fills this in when the claim came first).
		if email := strings.ToLower(strings.TrimSpace(req.Email)); email != "" {
			if tenant, err := upsertTenantByEmail(app, email, "", "trial"); err == nil {
				rec.Set("tenant_id", []string{tenant.Id})
			}
		}
		if err := app.Save(rec); err != nil {
			return e.JSON(http.StatusInternalServerError, map[string]any{"error": "failed to register trial"})
		}
		log.Printf("trial: registered hardware fingerprint %q (platform=%s, expires=%s)", fp, platform, trialExpires.Format(time.RFC3339))

		// Lightweight repeat-email detector (NOT the trial-lock gate):
		// record the (email, device) fingerprint so a repeat claim by the
		// same email on the same device is observable even before
		// activation. Errors are non-fatal — the claim itself already
		// succeeded.
		if email := strings.ToLower(strings.TrimSpace(req.Email)); email != "" {
			tenantID := ""
			if sl := rec.GetStringSlice("tenant_id"); len(sl) > 0 {
				tenantID = sl[0]
			}
			recordTrialClaim(app, email, fp, tenantID, "")
		}

		return e.JSON(http.StatusOK, map[string]any{
			"status":               "active",
			"hardware_fingerprint": fp,
			"trial_expires_at":     trialExpires.Format(time.RFC3339),
			"days_remaining":       trialDays,
		})
	}
}

// trialClaimHash is the lightweight repeat-claim fingerprint: SHA-256 of
// the normalized email + "|" + the device id (machine_id at activation,
// the hardware fingerprint at the claim endpoint). Deterministic, so the
// same (email, device) pair always yields the same 64-char hex hash — and
// email normalization (lowercase + trim) keeps "Store@Example.COM " and
// "store@example.com" on the same row.
func trialClaimHash(email, deviceID string) string {
	sum := sha256.Sum256([]byte(strings.ToLower(strings.TrimSpace(email)) + "|" + deviceID))
	return hex.EncodeToString(sum[:])
}

// recordTrialClaim is the lightweight repeat-email detector. It is
// deliberately NOT the SPEC-2026-TRIAL-LOCK gate (enforceTrialLock) — it
// never blocks and never answers 403; the full lock only fires across
// tenants, while a same-tenant reinstall (the same email claiming a fresh
// trial key on the same device) is allowed by design. This function makes
// that exact case observable: every successful trial activation (and every
// /trial claim that carries an email) upserts a row keyed by the (email,
// device) hash; a second claim bumps claim_count and is logged as a repeat.
//
// Returns the claim count (1 = first claim) and the first-claim timestamp
// (RFC 3339) so callers can surface repeat_claim to the client without the
// full lock's machinery. Failures are non-fatal (logged, claim proceeds) —
// detection must never break trial activation.
func recordTrialClaim(app core.App, email, deviceID, tenantID, key string) (int, string) {
	email = strings.ToLower(strings.TrimSpace(email))
	if email == "" || deviceID == "" {
		return 0, ""
	}
	hash := trialClaimHash(email, deviceID)
	now := time.Now().UTC()

	existing, err := app.FindFirstRecordByData("trial_claims", "claim_hash", hash)
	if err == nil {
		// Repeat claim: bump the count, extend the last-seen stamp, and
		// append the key to the audit trail.
		count := existing.GetInt("claim_count") + 1
		existing.Set("claim_count", count)
		existing.Set("last_claimed_at", now)
		if tenantID != "" && existing.GetString("tenant_id") == "" {
			existing.Set("tenant_id", []string{tenantID})
		}
		if key != "" {
			var keys []string
			if raw := existing.GetString("trial_keys"); raw != "" {
				_ = json.Unmarshal([]byte(raw), &keys)
			}
			keys = append(keys, key)
			if b, err := json.Marshal(keys); err == nil {
				existing.Set("trial_keys", string(b))
			}
		}
		if saveErr := app.Save(existing); saveErr != nil {
			log.Printf("trial claim: failed to update repeat row for %q: %v", email, saveErr)
			return count, existing.GetDateTime("first_claimed_at").Time().UTC().Format(time.RFC3339)
		}
		if count > 1 {
			log.Printf("trial claim repeat detected: email=%q device=%q count=%d (full trial-lock gate not involved)", email, deviceID, count)
		}
		return count, existing.GetDateTime("first_claimed_at").Time().UTC().Format(time.RFC3339)
	}

	// First claim for this (email, device) pair.
	coll, collErr := app.FindCollectionByNameOrId("trial_claims")
	if collErr != nil {
		log.Printf("trial claim: trial_claims collection not found: %v", collErr)
		return 1, now.Format(time.RFC3339)
	}
	rec := core.NewRecord(coll)
	rec.Set("claim_hash", hash)
	rec.Set("email", email)
	rec.Set("device_id", deviceID)
	rec.Set("claim_count", 1)
	rec.Set("first_claimed_at", now)
	rec.Set("last_claimed_at", now)
	if tenantID != "" {
		rec.Set("tenant_id", []string{tenantID})
	}
	if key != "" {
		rec.Set("trial_keys", `["`+key+`"]`)
	}
	if saveErr := app.Save(rec); saveErr != nil {
		log.Printf("trial claim: failed to record first claim for %q: %v", email, saveErr)
		return 1, now.Format(time.RFC3339)
	}
	return 1, now.Format(time.RFC3339)
}

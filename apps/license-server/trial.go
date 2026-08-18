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

		return e.JSON(http.StatusOK, map[string]any{
			"status":               "active",
			"hardware_fingerprint": fp,
			"trial_expires_at":     trialExpires.Format(time.RFC3339),
			"days_remaining":       trialDays,
		})
	}
}

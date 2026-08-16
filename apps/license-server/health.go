package main

import (
	"log"
	"net/http"
	"os"
	"runtime"
	"strings"
	"sync"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// handleHealth returns the server health status.
//
// GET /api/health — public, no auth required.
//
// PocketBase v0.39.6 registers its own /api/health before our OnServe hook
// runs, so this handler cannot be registered as a plain route (route
// conflict). Instead it is mounted via bindHealthOverride as a root-group
// middleware that short-circuits the built-in endpoint — see main.go.
//
// Used by Docker healthcheck and monitoring systems.
func handleHealth(app core.App) func(e *core.RequestEvent) error {
	startTime := time.Now()

	return func(e *core.RequestEvent) error {
		uptime := time.Since(startTime).Seconds()

		// Quick DB connectivity check via PocketBase's internal DB.
		dbConnected := true
		dbErr := ""
		if _, err := app.DB().NewQuery("SELECT 1").Execute(); err != nil {
			dbConnected = false
			dbErr = err.Error()
			log.Printf("/health: DB ping failed: %v", err)
		}

		status := http.StatusOK
		statusText := "ok"
		if !dbConnected {
			status = http.StatusServiceUnavailable
			statusText = "degraded"
		}

		return e.JSON(status, map[string]any{
			"status":       statusText,
			"db_connected": dbConnected,
			"db_error":     dbErr,
			"smtp":         smtpHealthSnapshot(),
			"uptime_secs":  int(uptime),
			"go_version":   runtime.Version(),
			"go_os":        runtime.GOOS,
			"go_arch":      runtime.GOARCH,
		})
	}
}

// bindHealthOverride intercepts GET /api/health (PocketBase's built-in
// endpoint is registered before the OnServe hook, so it cannot be replaced
// by re-registering the route) and serves the extended handleHealth payload
// instead. All other requests pass through to their normal handlers.
func bindHealthOverride(app core.App, se *core.ServeEvent) {
	se.Router.BindFunc(func(e *core.RequestEvent) error {
		if strings.TrimSuffix(e.Request.URL.Path, "/") == "/api/health" {
			return handleHealth(app)(e)
		}
		return e.Next()
	})
}

// ── SMTP sender-identity health status ───────────────────────────────

// smtpHealthRefreshInterval bounds how often /api/health re-runs the
// sender-identity probe. Docker polls every 15s; re-probing the relay on
// every poll would hammer it, so the result is cached for 60s.
const smtpHealthRefreshInterval = 60 * time.Second

// smtpHealthState caches the last probe result so health checks are cheap.
type smtpHealthState struct {
	mu        sync.Mutex
	checkedAt time.Time
	snapshot  map[string]any
}

var smtpHealth smtpHealthState

// smtpHealthSnapshot returns the cached sender-identity status, re-running
// the probe (auth + MAIL FROM only — nothing is ever queued) when the
// cache is stale. Mirrors verifySMTPConfig's classification: permanent
// rejection surfaces as verified=false with the relay's error; transient
// failures also report verified=false but with a warning-style error, so a
// relay hiccup is visible in the health payload without failing the check.
func smtpHealthSnapshot() map[string]any {
	smtpHealth.mu.Lock()
	defer smtpHealth.mu.Unlock()
	if time.Since(smtpHealth.checkedAt) < smtpHealthRefreshInterval && smtpHealth.snapshot != nil {
		return smtpHealth.snapshot
	}
	smtpHealth.snapshot = runSMTPHealthProbe()
	smtpHealth.checkedAt = time.Now()
	return smtpHealth.snapshot
}

// runSMTPHealthProbe executes one sender-identity probe and shapes the
// result for the health payload. Env is read per call, matching the
// senders (a redeploy with fixed env is picked up without a restart).
func runSMTPHealthProbe() map[string]any {
	host := strings.TrimSpace(os.Getenv("OZ_SMTP_HOST"))
	if host == "" {
		return map[string]any{
			"configured": false,
			"verified":   false,
			"error":      "",
		}
	}
	port := strings.TrimSpace(os.Getenv("OZ_SMTP_PORT"))
	if port == "" {
		port = "587"
	}
	user := os.Getenv("OZ_SMTP_USER")
	password := os.Getenv("OZ_SMTP_PASSWORD")
	from := strings.TrimSpace(os.Getenv("OZ_SMTP_FROM"))

	res := map[string]any{
		"configured": true,
		"verified":   false,
		"error":      "",
	}
	if from == "" || from == smtpDefaultFrom {
		res["error"] = "OZ_SMTP_FROM is unset or still the unowned default " + smtpDefaultFrom
		return res
	}
	if err := probeSMTPFrom(host, port, user, password, from); err != nil {
		res["error"] = err.Error()
		return res
	}
	res["verified"] = true
	return res
}

// resetSMTPHealthCache clears the cached probe result so tests can re-run
// the probe with different env without waiting out the refresh interval.
func resetSMTPHealthCache() {
	smtpHealth.mu.Lock()
	defer smtpHealth.mu.Unlock()
	smtpHealth.snapshot = nil
	smtpHealth.checkedAt = time.Time{}
}

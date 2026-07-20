package main

import (
	"log"
	"net/http"
	"runtime"
	"time"

	"github.com/pocketbase/pocketbase/core"
)

// handleHealth returns the server health status.
//
// GET /api/health — public, no auth required.
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
			"uptime_secs":  int(uptime),
			"go_version":   runtime.Version(),
			"go_os":        runtime.GOOS,
			"go_arch":      runtime.GOARCH,
		})
	}
}

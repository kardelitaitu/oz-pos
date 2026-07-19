//go:build ignore

package main

import (
	"fmt"
	"net/http"
	"os"
	"time"
)

// Small Go binary used as Docker HEALTHCHECK command.
// Pings GET /api/health and exits with 0 on success, 1 on failure.
// This replaces the curl-based healthcheck, eliminating curl from
// the runtime image and providing more precise health information.
//
// Usage:
//
//	go run apps/license-server/healthcheck.go [url]
//
// Default url: http://localhost:8080/api/health
func main() {
	url := "http://localhost:8080/api/health"
	if len(os.Args) > 1 {
		url = os.Args[1]
	}

	client := &http.Client{
		Timeout: 5 * time.Second,
	}

	resp, err := client.Get(url)
	if err != nil {
		fmt.Fprintf(os.Stderr, "HEALTHCHECK FAIL: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		fmt.Fprintf(os.Stderr, "HEALTHCHECK FAIL: status %d\n", resp.StatusCode)
		os.Exit(1)
	}

	fmt.Fprintf(os.Stderr, "HEALTHCHECK OK: status %d\n", resp.StatusCode)
	os.Exit(0)
}

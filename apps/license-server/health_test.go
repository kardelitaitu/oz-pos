package main

// Tests for the /api/health override (bindHealthOverride) and its SMTP
// sender-identity status field. They bind the same middleware main.go
// uses, then hit the real built-in /api/health path through the router, so
// both the interception and the payload are exercised.

import (
	"encoding/json"
	"net"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// getHealth builds the mux with the health override bound and issues a GET
// to the given path, returning the recorder.
func getHealth(t *testing.T, path string) *httptest.ResponseRecorder {
	t.Helper()
	app, se := setupDirectApp(t)
	defer app.Cleanup()
	bindHealthOverride(app, se)

	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux: %v", err)
	}
	req := httptest.NewRequest(http.MethodGet, path, nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	return rec
}

func TestHealth_SMTPNotConfigured(t *testing.T) {
	resetSMTPHealthCache()
	t.Setenv("OZ_SMTP_HOST", "")
	t.Setenv("OZ_SMTP_FROM", "")

	rec := getHealth(t, "/api/health")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var body struct {
		Status string `json:"status"`
		SMTP   struct {
			Configured bool   `json:"configured"`
			Verified   bool   `json:"verified"`
			Error      string `json:"error"`
		} `json:"smtp"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("unmarshal: %v; body: %s", err, rec.Body.String())
	}
	if body.Status != "ok" {
		t.Errorf("status = %q, want ok", body.Status)
	}
	if body.SMTP.Configured {
		t.Error("smtp.configured should be false when OZ_SMTP_HOST is unset")
	}
	if body.SMTP.Verified {
		t.Error("smtp.verified should be false when SMTP is not configured")
	}
}

func TestHealth_SMTPVerified(t *testing.T) {
	resetSMTPHealthCache()
	addr, _ := runSMTPServer(t, nil, false) // accepts MAIL FROM
	host, port, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split: %v", err)
	}
	t.Setenv("OZ_SMTP_HOST", host)
	t.Setenv("OZ_SMTP_PORT", port)
	t.Setenv("OZ_SMTP_USER", "")
	t.Setenv("OZ_SMTP_PASSWORD", "")
	t.Setenv("OZ_SMTP_FROM", "verified@example.com")

	rec := getHealth(t, "/api/health")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), `"verified":true`) {
		t.Errorf("expected smtp.verified=true in %s", rec.Body.String())
	}
}

func TestHealth_SMTPRejected(t *testing.T) {
	resetSMTPHealthCache()
	addr, _ := runSMTPServer(t, nil, true) // 550 on MAIL FROM
	host, port, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split: %v", err)
	}
	t.Setenv("OZ_SMTP_HOST", host)
	t.Setenv("OZ_SMTP_PORT", port)
	t.Setenv("OZ_SMTP_USER", "")
	t.Setenv("OZ_SMTP_PASSWORD", "")
	t.Setenv("OZ_SMTP_FROM", "unverified@example.com")

	rec := getHealth(t, "/api/health")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	body := rec.Body.String()
	if strings.Contains(body, `"verified":true`) {
		t.Errorf("smtp.verified should be false for a rejected sender: %s", body)
	}
	if !strings.Contains(body, "unverified") && !strings.Contains(body, "not verified") {
		t.Errorf("expected the relay rejection to surface in smtp.error: %s", body)
	}
}

func TestHealth_SMTPUnsetFrom(t *testing.T) {
	resetSMTPHealthCache()
	t.Setenv("OZ_SMTP_HOST", "smtp-relay.brevo.com")
	t.Setenv("OZ_SMTP_PORT", "587")
	t.Setenv("OZ_SMTP_USER", "")
	t.Setenv("OZ_SMTP_PASSWORD", "")
	t.Setenv("OZ_SMTP_FROM", "")

	rec := getHealth(t, "/api/health")
	body := rec.Body.String()
	if !strings.Contains(body, `"configured":true`) || strings.Contains(body, `"verified":true`) {
		t.Errorf("expected configured=true, verified=false: %s", body)
	}
	if !strings.Contains(body, "OZ_SMTP_FROM") {
		t.Errorf("expected an OZ_SMTP_FROM error hint: %s", body)
	}
}

func TestHealth_OtherRoutesPassThrough(t *testing.T) {
	// The override middleware must not swallow non-health routes: the
	// built-in /api/collections (auth-required) should still answer 401,
	// proving the mux chain is intact.
	app, se := setupDirectApp(t)
	defer app.Cleanup()
	bindHealthOverride(app, se)

	mux, err := se.Router.BuildMux()
	if err != nil {
		t.Fatalf("BuildMux: %v", err)
	}
	req := httptest.NewRequest(http.MethodGet, "/api/collections", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Fatalf("expected 401 from unauthenticated /api/collections (built-in route intact), got %d: %s", rec.Code, rec.Body.String())
	}
}

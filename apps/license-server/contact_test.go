package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// postContact issues a POST to /api/v1/web/contact against the given mux.
func postContact(t *testing.T, mux http.Handler, body string, remoteAddr string) *httptest.ResponseRecorder {
	t.Helper()
	req := httptest.NewRequest(http.MethodPost, "/api/v1/web/contact", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	if remoteAddr != "" {
		req.RemoteAddr = remoteAddr
	}
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	return rec
}

func TestContactHandler_Validation(t *testing.T) {
	t.Run("empty body rejected", func(t *testing.T) {
		resetRateLimiters()
		app, se := setupDirectApp(t)
		defer app.Cleanup()
		mux, err := se.Router.BuildMux()
		if err != nil {
			t.Fatalf("BuildMux failed: %v", err)
		}
		rec := postContact(t, mux, `{}`, "")
		if rec.Code != http.StatusBadRequest {
			t.Fatalf("expected 400 for empty body, got %d: %s", rec.Code, rec.Body.String())
		}
	})

	t.Run("missing name", func(t *testing.T) {
		resetRateLimiters()
		app, se := setupDirectApp(t)
		defer app.Cleanup()
		mux, _ := se.Router.BuildMux()
		rec := postContact(t, mux, `{"email":"a@b.com","message":"this is a long enough message"}`, "")
		if rec.Code != http.StatusBadRequest {
			t.Fatalf("expected 400 for missing name, got %d: %s", rec.Code, rec.Body.String())
		}
	})

	t.Run("invalid email", func(t *testing.T) {
		resetRateLimiters()
		app, se := setupDirectApp(t)
		defer app.Cleanup()
		mux, _ := se.Router.BuildMux()
		rec := postContact(t, mux, `{"name":"Budi","email":"not-an-email","message":"this is a long enough message"}`, "")
		if rec.Code != http.StatusBadRequest {
			t.Fatalf("expected 400 for invalid email, got %d: %s", rec.Code, rec.Body.String())
		}
	})

	t.Run("message too short", func(t *testing.T) {
		resetRateLimiters()
		app, se := setupDirectApp(t)
		defer app.Cleanup()
		mux, _ := se.Router.BuildMux()
		rec := postContact(t, mux, `{"name":"Budi","email":"a@b.com","message":"short"}`, "")
		if rec.Code != http.StatusBadRequest {
			t.Fatalf("expected 400 for short message, got %d: %s", rec.Code, rec.Body.String())
		}
	})

	t.Run("name too long", func(t *testing.T) {
		resetRateLimiters()
		app, se := setupDirectApp(t)
		defer app.Cleanup()
		mux, _ := se.Router.BuildMux()
		long := strings.Repeat("x", contactNameMax+1)
		rec := postContact(t, mux, `{"name":"`+long+`","email":"a@b.com","message":"this is a long enough message"}`, "")
		if rec.Code != http.StatusBadRequest {
			t.Fatalf("expected 400 for over-long name, got %d: %s", rec.Code, rec.Body.String())
		}
	})

	t.Run("message too long", func(t *testing.T) {
		resetRateLimiters()
		app, se := setupDirectApp(t)
		defer app.Cleanup()
		mux, _ := se.Router.BuildMux()
		long := strings.Repeat("m", contactMessageMax+1)
		rec := postContact(t, mux, `{"name":"Budi","email":"a@b.com","message":"`+long+`"}`, "")
		if rec.Code != http.StatusBadRequest {
			t.Fatalf("expected 400 for over-long message, got %d: %s", rec.Code, rec.Body.String())
		}
	})
}

func TestContactHandler_NotConfigured(t *testing.T) {
	resetRateLimiters()
	// Ensure the webhook env is unset so the handler returns 503.
	t.Setenv("OZ_DISCORD_WEBHOOK", "")

	app, se := setupDirectApp(t)
	defer app.Cleanup()
	mux, _ := se.Router.BuildMux()

	rec := postContact(t, mux, `{"name":"Budi","email":"a@b.com","message":"this is a long enough message"}`, "")
	if rec.Code != http.StatusServiceUnavailable {
		t.Fatalf("expected 503 when webhook unset, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestContactHandler_HoneypotSilentlyAccepted(t *testing.T) {
	resetRateLimiters()
	// A live webhook that fails the test if it is ever called.
	hit := false
	webhook := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		hit = true
		w.WriteHeader(http.StatusOK)
	}))
	defer webhook.Close()
	t.Setenv("OZ_DISCORD_WEBHOOK", webhook.URL)

	app, se := setupDirectApp(t)
	defer app.Cleanup()
	mux, _ := se.Router.BuildMux()

	rec := postContact(t, mux,
		`{"name":"Bot","email":"bot@spam.example","message":"buy my product please","website":"http://spam.example"}`, "")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200 for honeypot submission, got %d: %s", rec.Code, rec.Body.String())
	}
	if hit {
		t.Fatal("honeypot submission must not be forwarded to the webhook")
	}
}

func TestContactHandler_ForwardsToWebhook(t *testing.T) {
	resetRateLimiters()
	var gotPayload string
	webhook := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		buf := make([]byte, r.ContentLength)
		_, _ = r.Body.Read(buf)
		gotPayload = string(buf)
		w.WriteHeader(http.StatusOK)
	}))
	defer webhook.Close()
	t.Setenv("OZ_DISCORD_WEBHOOK", webhook.URL)

	app, se := setupDirectApp(t)
	defer app.Cleanup()
	mux, _ := se.Router.BuildMux()

	rec := postContact(t, mux,
		`{"name":"Budi Santoso","email":"budi@example.com","message":"QRIS is not showing on my register, please help."}`, "")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var payload map[string]string
	if err := json.Unmarshal([]byte(gotPayload), &payload); err != nil {
		t.Fatalf("webhook payload is not JSON: %v", err)
	}
	content := payload["content"]
	for _, want := range []string{"Budi Santoso", "budi@example.com", "QRIS is not showing"} {
		if !strings.Contains(content, want) {
			t.Errorf("webhook content missing %q: %s", want, content)
		}
	}
	if len(content) > discordContentMax {
		t.Errorf("webhook content %d chars exceeds Discord limit", len(content))
	}
}

func TestContactHandler_WebhookFailure(t *testing.T) {
	resetRateLimiters()
	webhook := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
	}))
	defer webhook.Close()
	t.Setenv("OZ_DISCORD_WEBHOOK", webhook.URL)

	app, se := setupDirectApp(t)
	defer app.Cleanup()
	mux, _ := se.Router.BuildMux()

	rec := postContact(t, mux,
		`{"name":"Budi","email":"a@b.com","message":"this is a long enough message"}`, "")
	if rec.Code != http.StatusBadGateway {
		t.Fatalf("expected 502 when Discord fails, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestContactHandler_RateLimited(t *testing.T) {
	resetRateLimiters()
	t.Setenv("OZ_DISCORD_WEBHOOK", "") // 503 path never reached — limiter fires first

	app, se := setupDirectApp(t)
	defer app.Cleanup()
	mux, _ := se.Router.BuildMux()

	const ip = "10.9.9.9"
	// Exhaust the contact bucket for this IP.
	for i := 0; i < contactRateLimiter.maxPerHr; i++ {
		contactRateLimiter.allow(ip)
	}

	rec := postContact(t, mux,
		`{"name":"Budi","email":"a@b.com","message":"this is a long enough message"}`, ip+":1234")
	if rec.Code != http.StatusTooManyRequests {
		t.Fatalf("expected 429, got %d: %s", rec.Code, rec.Body.String())
	}
}

// TestContactHandler_LongMessageTruncatedForDiscord verifies that an
// over-long (but valid, ≤2000) message is truncated to fit Discord's
// content limit before forwarding.
func TestContactHandler_LongMessageTruncatedForDiscord(t *testing.T) {
	resetRateLimiters()
	var contentLen int
	webhook := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		var payload map[string]string
		_ = json.NewDecoder(r.Body).Decode(&payload)
		contentLen = len(payload["content"])
		w.WriteHeader(http.StatusOK)
	}))
	defer webhook.Close()
	t.Setenv("OZ_DISCORD_WEBHOOK", webhook.URL)

	app, se := setupDirectApp(t)
	defer app.Cleanup()
	mux, _ := se.Router.BuildMux()

	long := strings.Repeat("m", contactMessageMax)
	rec := postContact(t, mux,
		`{"name":"Budi","email":"a@b.com","message":"`+long+`"}`, "")
	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}
	if contentLen > discordContentMax {
		t.Errorf("webhook content %d chars exceeds Discord limit %d", contentLen, discordContentMax)
	}
}

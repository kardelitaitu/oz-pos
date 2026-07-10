// Package main is the entry point for the OZ-POS license server.
// It extends PocketBase with custom Go hooks for license activation,
// renewal, and status checks with RSA-2048 signing.
package main

import (
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/x509"
	"encoding/base64"
	"encoding/pem"
	"log"
	"os"

	"github.com/pocketbase/pocketbase"
	"github.com/pocketbase/pocketbase/core"
)

// privateKey is the RSA-2048 private key loaded from the
// OZ_LICENSE_PRIVATE_KEY environment variable at startup.
var privateKey *rsa.PrivateKey

func main() {
	app := pocketbase.New()

	// ── Bootstrap: load RSA private key ──────────────────────────
	keyPEM := os.Getenv("OZ_LICENSE_PRIVATE_KEY")
	if keyPEM == "" {
		log.Fatal("OZ_LICENSE_PRIVATE_KEY environment variable is required")
	}

	block, _ := pem.Decode([]byte(keyPEM))
	if block == nil {
		log.Fatal("failed to decode PEM block from OZ_LICENSE_PRIVATE_KEY")
	}

	var err error
	privateKey, err = x509.ParsePKCS1PrivateKey(block.Bytes)
	if err != nil {
		// Try PKCS8 format (more common with modern tools)
		pkcs8Key, err2 := x509.ParsePKCS8PrivateKey(block.Bytes)
		if err2 != nil {
			log.Fatalf("failed to parse RSA private key (PKCS1: %v, PKCS8: %v)", err, err2)
		}
		var ok bool
		privateKey, ok = pkcs8Key.(*rsa.PrivateKey)
		if !ok {
			log.Fatal("key is not an RSA private key")
		}
	}
	log.Println("RSA private key loaded successfully")

	// ── Register custom license API routes ───────────────────────
	app.OnServe().BindFunc(func(se *core.ServeEvent) error {
		se.Router.POST("/api/v1/license/activate", handleActivate(app))
		se.Router.POST("/api/v1/license/renew", handleRenew(app))
		se.Router.GET("/api/v1/license/status/{tenant_id}", handleStatus(app))
		return se.Next()
	})

	if err := app.Start(); err != nil {
		log.Fatal(err)
	}
}

// SubscriptionPayload is the JSON structure signed by the license server.
// This is the payload the POS stores locally and verifies against the
// embedded public key.
type SubscriptionPayload struct {
	TenantID        string   `json:"tenant_id"`
	TierKey         string   `json:"tier_key"`
	Status          string   `json:"status"`
	MaxStores       int      `json:"max_stores"`
	MaxPOSInstances int      `json:"max_pos_instances"`
	AllowedTypes    []string `json:"allowed_types"`
	StartsAt        string   `json:"starts_at"`
	ExpiresAt       string   `json:"expires_at"`
	GraceUntil      string   `json:"grace_until"`
	IssuedAt        string   `json:"issued_at"`
}

// signSubscription marshals the payload to JSON, SHA-256 hashes it,
// and signs it with the RSA-2048 private key using PKCS1v15.
func signSubscription(sub SubscriptionPayload) (payload string, signature string, err error) {
	payloadBytes, err := jsonMarshal(sub)
	if err != nil {
		return "", "", err
	}
	hash := sha256.Sum256(payloadBytes)
	sig, err := rsa.SignPKCS1v15(rand.Reader, privateKey, crypto.SHA256, hash[:])
	if err != nil {
		return "", "", err
	}
	return string(payloadBytes), base64.StdEncoding.EncodeToString(sig), nil
}

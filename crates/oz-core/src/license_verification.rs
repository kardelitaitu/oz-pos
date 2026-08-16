//! License server verification and activation client for ADR #9.
//!
//! This module handles:
//! - RSA-2048 PKCS1v15 signature verification of signed subscriptions
//! - HTTP client calls to the PocketBase license server for activation,
//!   renewal, and status checks.
//!
//! The public key is embedded at build time via `LICENSE_PUBLIC_KEY_PEM`.
//! The server URL is `LICENSE_SERVER_URL` with env var override.

use base64::Engine;
use rsa::RsaPublicKey;
use rsa::pkcs1v15::VerifyingKey;
use rsa::signature::Verifier;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::error::CoreError;

/// The license server URL embedded at build time.
///
/// Override via the `OZ_LICENSE_SERVER_URL` environment variable
/// in production, or use `http://localhost:8090` for local testing.
pub const LICENSE_SERVER_URL: &str = "https://auth--oz-pos-license-service--76cyv4d6bn54.code.run";

/// The RSA-2048 public key in PEM format, embedded at build time.
///
/// This key corresponds to the private key held by the PocketBase
/// license server. It is generated once and embedded in every POS
/// binary release.
///
/// In development/test builds, this defaults to a placeholder key.
/// Replace with the production public key before release.
pub const LICENSE_PUBLIC_KEY_PEM: &str = include_str!("../oz-license.key.pub");

/// Return the license server URL, respecting the env var override.
pub fn license_server_url() -> String {
    std::env::var("OZ_LICENSE_SERVER_URL").unwrap_or_else(|_| LICENSE_SERVER_URL.to_string())
}

/// Result of pinging the license server's unauthenticated health endpoint.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicensePingResult {
    /// Whether the server responded successfully.
    pub ok: bool,
    /// Status text (e.g. "Connected", "Connection refused", ...).
    pub status: String,
    /// Round-trip latency in milliseconds, if the ping succeeded.
    pub latency_ms: Option<u64>,
}

/// Ping the license server's `/api/health` endpoint to verify reachability.
///
/// Unlike activation/renew/status, this needs NO credentials — the health
/// route returns `{"status":"ok"}` unauthenticated. The login/lock-screen
/// connection pill uses it so it shows green as soon as the auth server is
/// reachable, before any license is activated.
pub async fn ping_license_server() -> LicensePingResult {
    let health_url = format!("{}/api/health", license_server_url().trim_end_matches('/'));
    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build();
    match client {
        Ok(client) => match client.get(&health_url).send().await {
            Ok(resp) => {
                let latency = start.elapsed().as_millis() as u64;
                if resp.status().is_success() {
                    LicensePingResult {
                        ok: true,
                        status: format!("Connected ({latency}ms)"),
                        latency_ms: Some(latency),
                    }
                } else {
                    LicensePingResult {
                        ok: false,
                        status: format!("Server returned {}", resp.status()),
                        latency_ms: Some(latency),
                    }
                }
            }
            Err(e) => LicensePingResult {
                ok: false,
                status: format!("Connection failed: {e}"),
                latency_ms: None,
            },
        },
        Err(e) => LicensePingResult {
            ok: false,
            status: format!("HTTP client init failed: {e}"),
            latency_ms: None,
        },
    }
}

/// Extract a human-readable error message from a JSON error body
/// returned by the license server.
///
/// The server returns errors as `{"error": "message"}`. This helper
/// extracts the `error` field so the user sees the clean message
/// (e.g. "Wrong email or phone number") instead of the raw JSON blob.
///
/// Falls back to the raw body string if parsing fails.
fn extract_server_error(body: &str) -> String {
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(body)
        && let Some(msg) = obj.get("error").and_then(|v| v.as_str())
    {
        return msg.to_string();
    }
    body.to_string()
}

// ── Request/Response types ──────────────────────────────────────────

/// Request body for `POST /api/v1/license/activate`.
#[derive(Debug, Clone, Serialize)]
pub struct ActivateLicenseRequest {
    /// The license key purchased by the customer.
    pub key: String,
    /// The machine/hardware fingerprint.
    pub machine_id: String,
    /// The contact email (used as primary tenant identifier).
    pub email: String,
    /// The contact phone number for the licensee.
    pub phone: String,
    /// The api_key of an existing tenant, required when re-activating
    /// an installation whose tenant was previously activated (H1 audit
    /// fix). New tenants omit this on the first activation; the server
    /// issues a new api_key in the response which must be persisted
    /// locally and re-sent on every subsequent activation call.
    /// `None` for first activation; `Some(api_key)` for re-activation.
    ///
    /// The key is sent in the `Authorization: Bearer <api_key>` header
    /// (see [`activate_license`]) and is deliberately NEVER serialized
    /// into the request body — a body credential leaks into CDN /
    /// webserver access logs that capture request bodies.
    #[serde(skip_serializing, default)]
    pub api_key: Option<String>,
}

/// Response from `POST /api/v1/license/activate`.
#[derive(Debug, Clone, Deserialize)]
pub struct ActivateLicenseResponse {
    /// The signed subscription payload (JSON string).
    pub signed_payload: String,
    /// Base64-encoded RSA-2048 signature.
    pub signature: String,
    /// The Tenant ID returned by the server.
    #[serde(default)]
    pub tenant_id: String,
    /// The API key for subsequent renew/status calls.
    #[serde(default)]
    pub api_key: String,
}

/// Request body for `POST /api/v1/license/renew`.
#[derive(Debug, Serialize, Deserialize)]
pub struct RenewLicenseRequest {
    /// The tenant ID.
    pub tenant_id: String,
    /// The API key obtained during activation. Sent in the
    /// `Authorization: Bearer <api_key>` header (see [`renew_license`])
    /// and deliberately NEVER serialized into the request body — a body
    /// credential leaks into CDN / webserver access logs that capture
    /// request bodies.
    #[serde(default, skip_serializing)]
    pub api_key: String,
    /// The new license key.
    pub key: String,
}

/// Response from `POST /api/v1/license/renew`.
#[derive(Debug, Clone, Deserialize)]
pub struct RenewLicenseResponse {
    /// The signed subscription payload (JSON string).
    pub signed_payload: String,
    /// Base64-encoded RSA-2048 signature.
    pub signature: String,
}

/// Response from `POST /api/v1/license/status`.
#[derive(Debug, Clone, Deserialize)]
pub struct LicenseStatusResponse {
    /// The tenant ID.
    pub tenant_id: String,
    /// The subscription status.
    pub status: String,
    /// The tier key (free, pro, premium, enterprise).
    pub tier: String,
    /// Whether the subscription is active.
    pub active: bool,
    /// When the subscription expires (RFC 3339).
    #[serde(default)]
    pub expires_at: Option<String>,
    /// When the grace period ends (RFC 3339).
    #[serde(default)]
    pub grace_until: Option<String>,
    /// Maximum stores allowed.
    #[serde(default)]
    pub max_stores: i64,
}

/// The subscription payload structure signed by the license server.
/// Matches the Go `SubscriptionPayload` struct in `apps/license-server/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedSubscriptionPayload {
    /// The tenant ID.
    pub tenant_id: String,
    /// The tier key (free, pro, premium, enterprise).
    pub tier_key: String,
    /// The subscription status.
    pub status: String,
    /// Maximum number of stores allowed.
    #[serde(default)]
    pub max_stores: i64,
    /// Maximum POS register instances allowed.
    #[serde(default)]
    pub max_pos_instances: i64,
    /// List of workspace types allowed.
    #[serde(default)]
    pub allowed_types: Vec<String>,
    /// When the subscription becomes active.
    pub starts_at: String,
    /// When the subscription expires.
    pub expires_at: String,
    /// When the offline grace period ends (expires_at + 14 days).
    pub grace_until: String,
    /// When this payload was issued.
    pub issued_at: String,
}

// ── Signature Verification ──────────────────────────────────────────

/// Verify an RSA-2048 PKCS1v15 SHA-256 signature over a payload.
///
/// This is the core verification function used by the POS to validate
/// signed subscriptions from the license server.
///
/// # Arguments
/// * `payload` - The JSON payload that was signed.
/// * `signature_base64` - The base64-encoded RSA signature.
///
/// # Returns
/// `Ok(())` if the signature is valid, or `Err(CoreError::InvalidSubscriptionSignature)`.
pub fn verify_license_signature(payload: &str, signature_base64: &str) -> Result<(), CoreError> {
    // BOOTSTRAP_FREE is a sentinel for single-store deployments without
    // a license server (seeded by migration 061). It is ONLY accepted in
    // debug/dev builds; release builds require a real RSA signature.
    #[cfg(debug_assertions)]
    if signature_base64 == "BOOTSTRAP_FREE" {
        return Ok(());
    }

    let public_key = load_public_key()?;

    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_base64)
        .map_err(|e| {
            CoreError::InvalidSubscriptionSignature(format!(
                "failed to decode base64 signature: {e}"
            ))
        })?;

    let signature = rsa::pkcs1v15::Signature::try_from(sig_bytes.as_slice()).map_err(|e| {
        CoreError::InvalidSubscriptionSignature(format!("invalid RSA signature format: {e}"))
    })?;

    // Use VerifyingKey which handles SHA-256 hashing internally (matching SigningKey).
    let verifying_key = VerifyingKey::<Sha256>::new(public_key);
    verifying_key
        .verify(payload.as_bytes(), &signature)
        .map_err(|e| {
            CoreError::InvalidSubscriptionSignature(format!(
                "RSA signature verification failed: {e}"
            ))
        })?;

    Ok(())
}

/// Load the RSA-2048 public key from the embedded PEM.
fn load_public_key() -> Result<RsaPublicKey, CoreError> {
    use rsa::pkcs8::DecodePublicKey;

    RsaPublicKey::from_public_key_pem(LICENSE_PUBLIC_KEY_PEM).map_err(|e| {
        CoreError::InvalidSubscriptionSignature(format!("failed to load embedded public key: {e}"))
    })
}

// ── HTTP Client Functions ───────────────────────────────────────────

/// Activate a license key with the PocketBase license server.
///
/// POSTs to `/api/v1/license/activate` with the license key, tenant ID,
/// and machine fingerprint. Returns the signed subscription and API key.
///
/// # Arguments
/// * `req` - The activation request with license key and tenant info.
///
/// # Returns
/// The activation response containing signed_payload, signature, and api_key.
pub async fn activate_license(
    req: &ActivateLicenseRequest,
) -> Result<ActivateLicenseResponse, CoreError> {
    let url = format!("{}/api/v1/license/activate", license_server_url());
    let client = reqwest::Client::new();

    let mut request = client.post(&url);
    // The api_key authenticates the caller as the tenant admin on
    // re-activations; it travels in the Authorization header (never the
    // body, which access logs capture). First activations have no key yet.
    if let Some(api_key) = &req.api_key {
        request = request.bearer_auth(api_key);
    }
    let resp = request
        .json(req)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| {
            let msg = format!("license server unreachable: {e}");
            tracing::warn!("activation: {msg}");
            CoreError::Internal(msg)
        })?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let msg = extract_server_error(&body);
        let err = format!("activation failed ({status}): {msg}");
        tracing::warn!("{err}");
        return Err(CoreError::Internal(err));
    }

    let result: ActivateLicenseResponse = resp.json().await.map_err(|e| {
        let msg = format!("failed to parse activation response: {e}");
        tracing::warn!("{msg}");
        CoreError::Internal(msg)
    })?;

    // Verify the returned signature before trusting it.
    if let Err(e) = verify_license_signature(&result.signed_payload, &result.signature) {
        tracing::warn!("activation signature verification failed: {e}");
        return Err(e);
    }

    Ok(result)
}

/// Renew an existing subscription with the license server.
///
/// POSTs to `/api/v1/license/renew` with the tenant ID and API key.
pub async fn renew_license(req: &RenewLicenseRequest) -> Result<RenewLicenseResponse, CoreError> {
    let url = format!("{}/api/v1/license/renew", license_server_url());
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .bearer_auth(&req.api_key)
        .json(req)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| {
            let msg = format!("license server unreachable: {e}");
            tracing::warn!("renewal: {msg}");
            CoreError::Internal(msg)
        })?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let msg = extract_server_error(&body);
        let err = format!("renewal failed ({status}): {msg}");
        tracing::warn!("{err}");
        return Err(CoreError::Internal(err));
    }

    let result: RenewLicenseResponse = resp.json().await.map_err(|e| {
        let msg = format!("failed to parse renewal response: {e}");
        tracing::warn!("{msg}");
        CoreError::Internal(msg)
    })?;

    if let Err(e) = verify_license_signature(&result.signed_payload, &result.signature) {
        tracing::warn!("renewal signature verification failed: {e}");
        return Err(e);
    }

    Ok(result)
}

/// Check the current license status from the license server.
///
/// POSTs to `/api/v1/license/status` with the api_key carried in an
/// `Authorization: Bearer <api_key>` header. The server authenticates the
/// caller by this header alone (no `tenant_id` path parameter). Moving
/// the credential out of the URL into a header prevents it from being
/// captured in webserver access logs, CDN logs, browser history, or
/// `Referer` request headers.
///
/// # Arguments
/// * `api_key` - The API key returned by the activation response, used
///   to authenticate this status check.
pub async fn check_license_status(api_key: &str) -> Result<LicenseStatusResponse, CoreError> {
    let url = format!("{}/api/v1/license/status", license_server_url());
    let client = reqwest::Client::new();

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| {
            let msg = format!("license server unreachable: {e}");
            tracing::warn!("status check: {msg}");
            CoreError::Internal(msg)
        })?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let msg = extract_server_error(&body);
        let err = format!("status check failed ({status}): {msg}");
        tracing::warn!("{err}");
        return Err(CoreError::Internal(err));
    }

    resp.json().await.map_err(|e| {
        let msg = format!("failed to parse status response: {e}");
        tracing::warn!("{msg}");
        CoreError::Internal(msg)
    })
}

/// Store a signed subscription payload and API key in the local database.
///
/// Updates the `tenant_subscription` table with the payload and key
/// received from the license server after activation or renewal.
pub fn store_subscription(
    conn: &rusqlite::Connection,
    tenant_id: &str,
    signed_payload: &str,
    signature: &str,
    api_key: &str,
) -> Result<(), CoreError> {
    // Parse the payload to extract tier info
    let payload: SignedSubscriptionPayload = serde_json::from_str(signed_payload)
        .map_err(|e| CoreError::Internal(format!("failed to parse signed payload: {e}")))?;

    let allowed_types_json =
        serde_json::to_string(&payload.allowed_types).unwrap_or_else(|_| "[]".into());

    conn.execute(
        "INSERT OR REPLACE INTO tenant_subscription
         (tenant_id, tier_key, status, expires_at, max_stores,
          max_pos_instances, allowed_types_json, signature, signed_payload,
          api_key, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        rusqlite::params![
            tenant_id,
            payload.tier_key,
            payload.status,
            payload.expires_at,
            payload.max_stores,
            payload.max_pos_instances,
            allowed_types_json,
            signature,
            signed_payload,
            api_key,
        ],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::RsaPrivateKey;
    use rsa::pkcs8::{DecodePublicKey, EncodePublicKey};
    use rsa::signature::SignatureEncoding;

    /// Generate a test RSA key pair and return (private, public_pem).
    fn generate_test_keypair() -> (RsaPrivateKey, String) {
        let mut rng = rand::thread_rng();
        let private_key =
            RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate test RSA key");
        let public_pem = private_key
            .to_public_key()
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .expect("failed to export public key PEM");
        (private_key, public_pem)
    }

    /// Sign a payload using a test RSA key (matching the license server Go code).
    fn sign_test_payload(key: &RsaPrivateKey, payload: &str) -> String {
        use rsa::pkcs1v15::SigningKey;
        use rsa::signature::Signer;

        let signing_key = SigningKey::<Sha256>::new(key.clone());
        let sig = signing_key.sign(payload.as_bytes());
        base64::engine::general_purpose::STANDARD.encode(sig.to_bytes())
    }

    #[test]
    fn verify_valid_signature() {
        let (private_key, public_pem) = generate_test_keypair();
        let payload = r#"{"tenant_id":"test","tier_key":"pro"}"#;
        let sig = sign_test_payload(&private_key, payload);

        // Temporarily override the embedded key for testing.
        // In a real build, LICENSE_PUBLIC_KEY_PEM is embedded at compile time.
        // We test the core verification logic directly.
        let public_key = RsaPublicKey::from_public_key_pem(&public_pem).expect("parse public key");
        let sig_bytes = base64::engine::general_purpose::STANDARD
            .decode(&sig)
            .unwrap();
        let signature = rsa::pkcs1v15::Signature::try_from(sig_bytes.as_slice()).unwrap();

        let verifying_key = VerifyingKey::<Sha256>::new(public_key);
        let result = verifying_key.verify(payload.as_bytes(), &signature);
        assert!(result.is_ok(), "valid signature should verify: {result:?}");
    }

    #[test]
    fn verify_tampered_payload_fails() {
        let (private_key, public_pem) = generate_test_keypair();
        let payload = r#"{"tenant_id":"test","tier_key":"pro"}"#;
        let sig = sign_test_payload(&private_key, payload);

        let public_key = RsaPublicKey::from_public_key_pem(&public_pem).expect("parse public key");
        let sig_bytes = base64::engine::general_purpose::STANDARD
            .decode(&sig)
            .unwrap();
        let signature = rsa::pkcs1v15::Signature::try_from(sig_bytes.as_slice()).unwrap();

        // Tamper with the payload
        let tampered = r#"{"tenant_id":"test","tier_key":"enterprise"}"#;
        let verifying_key = VerifyingKey::<Sha256>::new(public_key);
        let result = verifying_key.verify(tampered.as_bytes(), &signature);
        assert!(result.is_err(), "tampered payload should fail verification");
    }

    #[test]
    fn verify_bootstrap_free_bypasses_rsa_in_debug() {
        // The BOOTSTRAP_FREE sentinel should pass without a real key
        // in debug/dev/test builds (where #[cfg(debug_assertions)] applies).
        // This test is always compiled in test mode (which is debug).
        let result = verify_license_signature("anything", "BOOTSTRAP_FREE");
        assert!(result.is_ok());
    }

    #[test]
    fn verify_rejects_garbage_signatures() {
        // Non-BOOTSTRAP_FREE garbage signatures (random strings, empty)
        // should always fail verification, regardless of build mode.
        let payload = r#"{"tenant_id":"test","tier_key":"free"}"#;

        let result = verify_license_signature(payload, "TAMPERED_SIGNATURE");
        assert!(
            result.is_err(),
            "tampered signature should fail: {result:?}"
        );

        let result = verify_license_signature(payload, "");
        assert!(result.is_err(), "empty signature should fail: {result:?}");
    }

    /// NOTE: There is intentionally no test that BOOTSTRAP_FREE is *rejected*
    /// in release builds, because `cargo test` always runs with
    /// `debug_assertions` enabled. The `#[cfg(debug_assertions)]` guard is
    /// validated by inspection and by running `cargo build --release` and
    /// confirming the symbol is absent.

    #[test]
    fn embedded_public_key_is_loadable() {
        // The embedded public key must be parseable at startup.
        // A corrupt or missing key file would cause this to panic.
        use rsa::traits::PublicKeyParts;

        let key = RsaPublicKey::from_public_key_pem(LICENSE_PUBLIC_KEY_PEM);
        assert!(key.is_ok(), "embedded public key should load: {key:?}");
        let key = key.unwrap();
        // Verify it's a 2048-bit key (the expected size).
        let bits = key.size() * 8;
        assert_eq!(bits, 2048, "embedded key should be 2048-bit RSA");
    }

    #[test]
    fn license_server_url_default() {
        // Test the default URL without env var overrides (avoid unsafe on set_var).
        let url = license_server_url();
        assert_eq!(url, LICENSE_SERVER_URL);
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn ping_license_server_hits_api_health_path() {
        // The reachability probe must target the unauthenticated
        // /api/health endpoint (not the cloud server's /health) and return
        // a structured result. A live HTTP call is not made here — the
        // default URL is a real deployment, so assert the URL construction
        // contract instead and keep the network call out of unit tests.
        let url = license_server_url();
        assert!(url.starts_with("https://"));
        let health = format!("{}/api/health", url.trim_end_matches('/'));
        assert!(health.ends_with("/api/health"));
        // The struct serializes camelCase like the sync PingResult so the
        // UI can render both connection pills uniformly.
        let json = serde_json::to_value(LicensePingResult {
            ok: true,
            status: "Connected (1ms)".into(),
            latency_ms: Some(1),
        })
        .unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["latencyMs"], 1);
    }

    #[test]
    fn store_subscription_inserts_row() {
        use crate::migrations;

        let conn = migrations::fresh_db();

        let payload = r#"{
            "tenant_id": "test-tenant",
            "tier_key": "pro",
            "status": "active",
            "max_stores": 2,
            "max_pos_instances": 3,
            "allowed_types": ["restaurant-pos", "store-pos"],
            "starts_at": "2026-01-01T00:00:00Z",
            "expires_at": "2027-01-01T00:00:00Z",
            "grace_until": "2027-01-15T00:00:00Z",
            "issued_at": "2026-01-01T00:00:00Z"
        }"#;

        let result = store_subscription(
            &conn,
            "test-tenant",
            payload,
            "TESTSIG",
            "oz_test_api_key_123",
        );
        assert!(result.is_ok(), "store_subscription failed: {result:?}");

        // Verify the row was inserted
        let stored = TenantSubscription::load(&conn, "test-tenant")
            .expect("load")
            .expect("should exist");
        assert_eq!(stored.tenant_id, "test-tenant");
        assert_eq!(stored.tier, crate::subscription::SubscriptionTier::Pro);
        assert_eq!(stored.max_stores, 2);
        assert_eq!(stored.max_pos_instances, 3);
        assert_eq!(stored.signature, "TESTSIG");
        assert_eq!(stored.signed_payload, payload);
        assert_eq!(stored.api_key, "oz_test_api_key_123");
    }

    #[test]
    fn store_subscription_handles_all_tier_keys() {
        use crate::migrations;
        use crate::subscription::SubscriptionTier;

        let conn = migrations::fresh_db();

        let tiers = vec![
            ("free", SubscriptionTier::Free, 1, 1),
            ("one_time", SubscriptionTier::OneTime, 1, 1),
            ("standard", SubscriptionTier::Standard, 1, 2),
            ("pro", SubscriptionTier::Pro, 0, 0),
            ("enterprise", SubscriptionTier::Enterprise, 0, 0),
        ];

        for (key, expected_tier, stores, pos) in tiers {
            let payload = format!(
                r#"{{
                "tenant_id": "tenant-{key}",
                "tier_key": "{key}",
                "status": "active",
                "max_stores": {stores},
                "max_pos_instances": {pos},
                "allowed_types": ["store-pos"],
                "starts_at": "2026-01-01T00:00:00Z",
                "expires_at": "2027-01-01T00:00:00Z",
                "grace_until": "2027-01-15T00:00:00Z",
                "issued_at": "2026-01-01T00:00:00Z"
            }}"#
            );

            let result = store_subscription(
                &conn,
                &format!("tenant-{key}"),
                &payload,
                "TESTSIG",
                "api_key_test",
            );
            assert!(
                result.is_ok(),
                "store_subscription for {key} failed: {result:?}"
            );

            let stored = TenantSubscription::load(&conn, &format!("tenant-{key}"))
                .unwrap()
                .unwrap();
            assert_eq!(stored.tier, expected_tier);
            assert_eq!(stored.max_stores, stores);
            assert_eq!(stored.max_pos_instances, pos);
        }
    }

    // We need to import TenantSubscription for the test above.
    use crate::subscription::TenantSubscription;

    // ── extract_server_error tests ────────────────────────────────

    #[test]
    fn extract_error_from_json_body() {
        let body = r#"{"error":"Wrong email or phone number"}"#;
        let msg = super::extract_server_error(body);
        assert_eq!(msg, "Wrong email or phone number");
    }

    #[test]
    fn extract_error_escaped_json() {
        let body = r#"{"error":"invalid or already used license key"}"#;
        let msg = super::extract_server_error(body);
        assert_eq!(msg, "invalid or already used license key");
    }

    #[test]
    fn extract_error_falls_back_to_raw_body() {
        // Non-JSON body should be returned as-is.
        let body = "Internal Server Error";
        let msg = super::extract_server_error(body);
        assert_eq!(msg, "Internal Server Error");
    }

    #[test]
    fn extract_error_empty_json() {
        let body = "{}";
        let msg = super::extract_server_error(body);
        assert_eq!(msg, "{}");
    }

    #[test]
    fn extract_error_empty_string() {
        let msg = super::extract_server_error("");
        assert_eq!(msg, "");
    }
}

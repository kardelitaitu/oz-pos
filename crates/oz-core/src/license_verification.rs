//! License server verification and activation client for ADR #9.
//!
//! This module handles:
//! - RSA-2048 PKCS1v15 signature verification of signed subscriptions
//! - HTTP client calls to the PocketBase license server for activation,
//!   renewal, and status checks.
//!
//! The public key is embedded at build time via [`LICENSE_PUBLIC_KEY_PEM`].
//! The server URL is [`LICENSE_SERVER_URL`] with env var override.

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

// ── Request/Response types ──────────────────────────────────────────

/// Request body for `POST /api/v1/license/activate`.
#[derive(Debug, Clone, Serialize)]
pub struct ActivateLicenseRequest {
    /// The license key purchased by the customer.
    pub key: String,
    /// The tenant ID (client-generated UUID).
    pub tenant_id: String,
    /// The machine/hardware fingerprint.
    pub machine_id: String,
    /// Optional business name from setup wizard.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_name: Option<String>,
    /// Optional contact name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_name: Option<String>,
    /// Optional contact email.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// Response from `POST /api/v1/license/activate`.
#[derive(Debug, Clone, Deserialize)]
pub struct ActivateLicenseResponse {
    /// The signed subscription payload (JSON string).
    pub signed_payload: String,
    /// Base64-encoded RSA-2048 signature.
    pub signature: String,
    /// The API key for subsequent renew/status calls.
    pub api_key: String,
}

/// Request body for `POST /api/v1/license/renew`.
#[derive(Debug, Clone, Serialize)]
pub struct RenewLicenseRequest {
    /// The tenant ID.
    pub tenant_id: String,
    /// The API key obtained during activation.
    pub api_key: String,
}

/// Response from `POST /api/v1/license/renew`.
#[derive(Debug, Clone, Deserialize)]
pub struct RenewLicenseResponse {
    /// The signed subscription payload (JSON string).
    pub signed_payload: String,
    /// Base64-encoded RSA-2048 signature.
    pub signature: String,
}

/// Response from `GET /api/v1/license/status/:tenant_id`.
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
    pub max_stores: i64,
    /// Maximum POS register instances allowed.
    pub max_pos_instances: i64,
    /// List of workspace types allowed.
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
    // a license server (seeded by migration 061).
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

    let resp = client
        .post(&url)
        .json(req)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| CoreError::Internal(format!("license server unreachable: {e}")))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(CoreError::Internal(format!(
            "activation failed ({status}): {body}"
        )));
    }

    let result: ActivateLicenseResponse = resp
        .json()
        .await
        .map_err(|e| CoreError::Internal(format!("failed to parse activation response: {e}")))?;

    // Verify the returned signature before trusting it.
    verify_license_signature(&result.signed_payload, &result.signature)?;

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
        .json(req)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| CoreError::Internal(format!("license server unreachable: {e}")))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(CoreError::Internal(format!(
            "renewal failed ({status}): {body}"
        )));
    }

    let result: RenewLicenseResponse = resp
        .json()
        .await
        .map_err(|e| CoreError::Internal(format!("failed to parse renewal response: {e}")))?;

    verify_license_signature(&result.signed_payload, &result.signature)?;

    Ok(result)
}

/// Check the current license status from the license server.
///
/// GETs `/api/v1/license/status/:tenant_id`. This is a public endpoint
/// (no auth required).
pub async fn check_license_status(tenant_id: &str) -> Result<LicenseStatusResponse, CoreError> {
    let url = format!("{}/api/v1/license/status/{tenant_id}", license_server_url());
    let client = reqwest::Client::new();

    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| CoreError::Internal(format!("license server unreachable: {e}")))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(CoreError::Internal(format!(
            "status check failed ({status}): {body}"
        )));
    }

    resp.json()
        .await
        .map_err(|e| CoreError::Internal(format!("failed to parse status response: {e}")))
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
    fn verify_bootstrap_free_bypasses_rsa() {
        // The BOOTSTRAP_FREE sentinel should pass without a real key.
        let result = verify_license_signature("anything", "BOOTSTRAP_FREE");
        assert!(result.is_ok());
    }

    #[test]
    fn license_server_url_default() {
        // Test the default URL without env var overrides (avoid unsafe on set_var).
        let url = license_server_url();
        assert_eq!(url, LICENSE_SERVER_URL);
        assert!(url.contains("license.oz-pos.com"));
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

    // We need to import TenantSubscription for the test above.
    use crate::subscription::TenantSubscription;
}

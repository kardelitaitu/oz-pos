//! AES-256-GCM encryption helpers for encrypting sensitive data at rest.
//!
//! Uses AES-256-GCM with a key derived from a domain prefix and either
//! the machine's hardware fingerprint (machine-bound) or a static
//! derivation (portable across machines).
//!
//! Ciphertext format: `base64(nonce || ciphertext || tag)` where
//! `nonce` is 12 bytes (random), `ciphertext` is the encrypted
//! plaintext, and `tag` is the 16-byte GCM authentication tag
//! (appended automatically by `aes-gcm`).

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead, aead::generic_array::GenericArray};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Error type for cryptographic operations.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// An internal cryptographic error occurred.
    #[error("crypto error: {0}")]
    Internal(String),
}

// ── Key derivation ───────────────────────────────────────────────────

/// Derive a 256-bit AES key from a domain prefix and machine ID via SHA-256.
///
/// The key is deterministic for the same domain + machine ID — this is by
/// design: machine-bound encryption binds the ciphertext to the hardware
/// that owns it.
fn derive_key(domain: &[u8], machine_id: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(machine_id.as_bytes());
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

/// Derive a static 256-bit key from a domain prefix (no machine binding).
fn derive_static_key(domain: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

// ── Domain-separation prefixes ───────────────────────────────────────

/// SMTP password domain-separation prefix.
const SMTP_DOMAIN: &[u8] = b"oz-pos.smtp-password.v1:";

/// API key domain-separation prefix.
const API_KEY_DOMAIN: &[u8] = b"oz-pos.api-key.v1:";

/// Sync API key domain-separation prefix.
const SYNC_API_KEY_DOMAIN: &[u8] = b"oz-pos.sync-api-key.v1:";

/// Sync terminal secret domain-separation prefix.
const SYNC_TERMINAL_SECRET_DOMAIN: &[u8] = b"oz-pos.sync-terminal-secret.v1:";

/// PG sync password domain-separation prefix.
const PG_SYNC_PASSWORD_DOMAIN: &[u8] = b"oz-pos.pg-sync-password.v1:";

/// Rate sync API key domain-separation prefix.
const RATE_API_KEY_DOMAIN: &[u8] = b"oz-pos.rate-api-key.v1:";

/// LAN server PSK domain-separation prefix.
const LAN_PSK_DOMAIN: &[u8] = b"oz-pos.lan-psk.v1:";

/// SMTP at-rest domain-separation prefix.
const SMTP_AT_REST_DOMAIN: &[u8] = b"oz-pos.smtp-at-rest.v1:";

/// User-profile at-rest domain-separation prefix.
const PROFILE_AT_REST_DOMAIN: &[u8] = b"oz-pos.user-profile-at-rest.v1:";

// ── Machine-bound (API key / SMTP password) ──────────────────────────

/// Encrypt an API key with a machine-bound key.
pub fn encrypt_api_key(plaintext: &str, machine_id: &str) -> Result<String, CryptoError> {
    let key = derive_key(API_KEY_DOMAIN, machine_id);
    encrypt(plaintext, &key)
}

/// Decrypt an API key previously produced by [`encrypt_api_key`].
pub fn decrypt_api_key(encrypted_b64: &str, machine_id: &str) -> Result<String, CryptoError> {
    let key = derive_key(API_KEY_DOMAIN, machine_id);
    decrypt(encrypted_b64, &key)
}

/// Encrypt an SMTP password with a machine-bound key.
pub fn encrypt_smtp_password(plaintext: &str, machine_id: &str) -> Result<String, CryptoError> {
    let key = derive_key(SMTP_DOMAIN, machine_id);
    encrypt(plaintext, &key)
}

/// Decrypt an SMTP password previously encrypted with [`encrypt_smtp_password`].
///
/// If decryption fails (e.g. legacy plaintext), returns the original string
/// unchanged so that existing configurations continue to work.
pub fn decrypt_smtp_password(encrypted_b64: &str, machine_id: &str) -> Result<String, CryptoError> {
    let key = derive_key(SMTP_DOMAIN, machine_id);
    match decrypt(encrypted_b64, &key) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => Ok(encrypted_b64.to_string()),
    }
}

// ── Static-key (portable across machines) ────────────────────────────

/// Encrypt an SMTP password for at-rest storage using a static app-level key.
///
/// Unlike [`encrypt_smtp_password`], this does NOT bind to the machine
/// fingerprint — the database can be copied between machines without
/// losing access to the SMTP password.
pub fn encrypt_smtp_at_rest(password: &str) -> String {
    let key = derive_static_key(SMTP_AT_REST_DOMAIN);
    encrypt(password, &key).unwrap_or_else(|_| password.to_string())
}

/// Decrypt an SMTP password stored with [`encrypt_smtp_at_rest`].
///
/// If the value is legacy plaintext (decryption fails), returns it unchanged.
pub fn decrypt_smtp_at_rest(encrypted: &str) -> String {
    let key = derive_static_key(SMTP_AT_REST_DOMAIN);
    decrypt(encrypted, &key).unwrap_or_else(|_| encrypted.to_string())
}

/// Encrypt a sync API key for at-rest storage (static key, portable).
pub fn encrypt_sync_api_key(plaintext: &str) -> Result<String, CryptoError> {
    let key = derive_key(SYNC_API_KEY_DOMAIN, "static");
    encrypt(plaintext, &key)
}

/// Decrypt a sync API key previously encrypted with [`encrypt_sync_api_key`].
pub fn decrypt_sync_api_key(encrypted_b64: &str) -> Result<String, CryptoError> {
    let key = derive_key(SYNC_API_KEY_DOMAIN, "static");
    decrypt(encrypted_b64, &key)
}

/// Encrypt a sync terminal secret for at-rest storage (static key, portable).
pub fn encrypt_sync_terminal_secret(plaintext: &str) -> Result<String, CryptoError> {
    let key = derive_key(SYNC_TERMINAL_SECRET_DOMAIN, "static");
    encrypt(plaintext, &key)
}

/// Decrypt a sync terminal secret previously encrypted with [`encrypt_sync_terminal_secret`].
pub fn decrypt_sync_terminal_secret(encrypted_b64: &str) -> Result<String, CryptoError> {
    let key = derive_key(SYNC_TERMINAL_SECRET_DOMAIN, "static");
    decrypt(encrypted_b64, &key)
}

/// Encrypt a PG sync password for at-rest storage (static key, portable).
pub fn encrypt_pg_sync_password(plaintext: &str) -> Result<String, CryptoError> {
    let key = derive_key(PG_SYNC_PASSWORD_DOMAIN, "static");
    encrypt(plaintext, &key)
}

/// Decrypt a PG sync password previously encrypted with [`encrypt_pg_sync_password`].
pub fn decrypt_pg_sync_password(encrypted_b64: &str) -> Result<String, CryptoError> {
    let key = derive_key(PG_SYNC_PASSWORD_DOMAIN, "static");
    decrypt(encrypted_b64, &key)
}

/// Encrypt a rate sync API key for at-rest storage (static key, portable).
pub fn encrypt_rate_api_key(plaintext: &str) -> Result<String, CryptoError> {
    let key = derive_key(RATE_API_KEY_DOMAIN, "static");
    encrypt(plaintext, &key)
}

/// Decrypt a rate sync API key previously encrypted with [`encrypt_rate_api_key`].
pub fn decrypt_rate_api_key(encrypted_b64: &str) -> Result<String, CryptoError> {
    let key = derive_key(RATE_API_KEY_DOMAIN, "static");
    decrypt(encrypted_b64, &key)
}

/// Encrypt a LAN server PSK for at-rest storage (static key, portable).
pub fn encrypt_lan_psk(plaintext: &str) -> Result<String, CryptoError> {
    let key = derive_key(LAN_PSK_DOMAIN, "static");
    encrypt(plaintext, &key)
}

/// Decrypt a LAN server PSK previously encrypted with [`encrypt_lan_psk`].
pub fn decrypt_lan_psk(encrypted_b64: &str) -> Result<String, CryptoError> {
    let key = derive_key(LAN_PSK_DOMAIN, "static");
    decrypt(encrypted_b64, &key)
}

/// Encrypt a user-profile sensitive field for at-rest storage (static key).
pub fn encrypt_profile_field(plaintext: &str) -> Result<String, CryptoError> {
    let key = derive_static_key(PROFILE_AT_REST_DOMAIN);
    encrypt(plaintext, &key)
}

/// Decrypt a user-profile sensitive field previously encrypted with
/// [`encrypt_profile_field`].
///
/// Fails closed: corrupted, truncated, or cross-domain ciphertext returns
/// an error — never plaintext.
pub fn decrypt_profile_field(encrypted_b64: &str) -> Result<String, CryptoError> {
    let key = derive_static_key(PROFILE_AT_REST_DOMAIN);
    decrypt(encrypted_b64, &key)
}

// ── Internal encrypt / decrypt ───────────────────────────────────────

/// Internal: encrypt plaintext with a pre-derived key.
fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, CryptoError> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = GenericArray::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError::Internal(format!("encryption failed: {e}")))?;

    // Format: nonce (12) + ciphertext+tag (variable)
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(base64_encode(&combined))
}

/// Internal: decrypt a base64-encoded ciphertext with a pre-derived key.
fn decrypt(encrypted_b64: &str, key: &[u8; 32]) -> Result<String, CryptoError> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    let combined = base64_decode(encrypted_b64)?;

    if combined.len() < 12 + 16 {
        return Err(CryptoError::Internal(
            "encrypted data too short: corrupted or tampered".into(),
        ));
    }

    let nonce = GenericArray::from_slice(&combined[..12]);
    let ciphertext = &combined[12..];

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::Internal(format!("decryption failed: {e}")))?;

    String::from_utf8(plaintext)
        .map_err(|e| CryptoError::Internal(format!("decrypted data is not valid UTF-8: {e}")))
}

// ── Base64 helpers ───────────────────────────────────────────────────

/// Encode bytes as URL-safe base64 (no padding).
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// Decode URL-safe base64 (with or without padding).
fn base64_decode(encoded: &str) -> Result<Vec<u8>, CryptoError> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(encoded))
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(encoded))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(encoded))
        .map_err(|e| CryptoError::Internal(format!("failed to decode base64 ciphertext: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let plaintext = "my-secret-api-key-12345";
        let machine_id = "test-machine-uuid";
        let encrypted = encrypt_api_key(plaintext, machine_id).unwrap();
        assert_ne!(encrypted, plaintext);
        let decrypted = decrypt_api_key(&encrypted, machine_id).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_machine_id_fails() {
        let plaintext = "secret";
        let encrypted = encrypt_api_key(plaintext, "machine-a").unwrap();
        let result = decrypt_api_key(&encrypted, "machine-b");
        assert!(result.is_err());
    }

    #[test]
    fn static_key_roundtrip() {
        let plaintext = "sync-api-key-value";
        let encrypted = encrypt_sync_api_key(plaintext).unwrap();
        assert_ne!(encrypted, plaintext);
        let decrypted = decrypt_sync_api_key(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn smtp_at_rest_roundtrip() {
        let password = "smtp-password-123";
        let encrypted = encrypt_smtp_at_rest(password);
        assert_ne!(encrypted, password);
        let decrypted = decrypt_smtp_at_rest(&encrypted);
        assert_eq!(decrypted, password);
    }

    #[test]
    fn smtp_password_legacy_plaintext_passthrough() {
        // Legacy plaintext (not encrypted) should pass through unchanged.
        let legacy = "plaintext-password";
        let result = decrypt_smtp_password(legacy, "machine-id").unwrap();
        assert_eq!(result, legacy);
    }

    #[test]
    fn profile_field_roundtrip() {
        let field = "sensitive-national-id";
        let encrypted = encrypt_profile_field(field).unwrap();
        let decrypted = decrypt_profile_field(&encrypted).unwrap();
        assert_eq!(decrypted, field);
    }

    #[test]
    fn corrupted_ciphertext_fails() {
        let result = decrypt_api_key("not-valid-base64!!!", "machine-id");
        assert!(result.is_err());
    }

    #[test]
    fn too_short_ciphertext_fails() {
        // base64 of 5 bytes (< 12 + 16 minimum)
        let short = base64_encode(&[1, 2, 3, 4, 5]);
        let result = decrypt_api_key(&short, "machine-id");
        assert!(result.is_err());
    }

    #[test]
    fn domain_separation() {
        let plaintext = "shared-value";
        let machine_id = "same-machine";
        let api_encrypted = encrypt_api_key(plaintext, machine_id).unwrap();
        let smtp_encrypted = encrypt_smtp_password(plaintext, machine_id).unwrap();
        // Different domains produce different ciphertexts
        assert_ne!(api_encrypted, smtp_encrypted);
        // Each decrypts with its own key only
        assert_eq!(
            decrypt_api_key(&api_encrypted, machine_id).unwrap(),
            plaintext
        );
        assert_eq!(
            decrypt_smtp_password(&smtp_encrypted, machine_id).unwrap(),
            plaintext
        );
    }

    #[test]
    fn all_secret_types_roundtrip() {
        let val = "test-value";
        let machine = "test-machine";

        // Machine-bound
        assert_eq!(
            decrypt_api_key(&encrypt_api_key(val, machine).unwrap(), machine).unwrap(),
            val
        );
        assert_eq!(
            decrypt_smtp_password(&encrypt_smtp_password(val, machine).unwrap(), machine).unwrap(),
            val
        );

        // Static key
        assert_eq!(
            decrypt_sync_api_key(&encrypt_sync_api_key(val).unwrap()).unwrap(),
            val
        );
        assert_eq!(
            decrypt_sync_terminal_secret(&encrypt_sync_terminal_secret(val).unwrap()).unwrap(),
            val
        );
        assert_eq!(
            decrypt_pg_sync_password(&encrypt_pg_sync_password(val).unwrap()).unwrap(),
            val
        );
        assert_eq!(
            decrypt_rate_api_key(&encrypt_rate_api_key(val).unwrap()).unwrap(),
            val
        );
        assert_eq!(
            decrypt_lan_psk(&encrypt_lan_psk(val).unwrap()).unwrap(),
            val
        );
        assert_eq!(
            decrypt_profile_field(&encrypt_profile_field(val).unwrap()).unwrap(),
            val
        );
    }
}

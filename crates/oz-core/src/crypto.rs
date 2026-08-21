//! Cryptographic helpers for encrypting sensitive data at rest.
//!
//! Uses AES-256-GCM with a key derived from the machine's hardware
//! fingerprint (system UUID).  This binds encrypted data to the
//! specific machine — copying the SQLite database to another host
//! makes decryption impossible without also copying the system UUID.
//!
//! Ciphertext format: `base64(nonce || ciphertext || tag)` where
//! `nonce` is 12 bytes (random), `ciphertext` is the encrypted
//! plaintext, and `tag` is the 16-byte GCM authentication tag
//! (appended automatically by `aes-gcm`).

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead, aead::generic_array::GenericArray};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::CoreError;

/// Derive a 256-bit AES key from the machine fingerprint via SHA-256.
///
/// The key is deterministic for the same machine ID — this is by design:
/// encryption binds the ciphertext to the hardware that owns it.
fn derive_key(domain: &[u8], machine_id: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(machine_id.as_bytes());
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

/// SMTP password domain-separation prefix.
const SMTP_DOMAIN: &[u8] = b"oz-pos.smtp-password.v1:";

/// API key domain-separation prefix.
const API_KEY_DOMAIN: &[u8] = b"oz-pos.api-key.v1:";

/// Encrypt `plaintext` with a key derived from `machine_id`.
///
/// Returns a base64-encoded ciphertext containing the nonce,
/// encrypted data, and GCM authentication tag.
pub fn encrypt_api_key(plaintext: &str, machine_id: &str) -> Result<String, CoreError> {
    let key = derive_key(API_KEY_DOMAIN, machine_id);
    encrypt(plaintext, &key)
}

/// Decrypt a ciphertext previously produced by [`encrypt_api_key`].
pub fn decrypt_api_key(encrypted_b64: &str, machine_id: &str) -> Result<String, CoreError> {
    let key = derive_key(API_KEY_DOMAIN, machine_id);
    decrypt(encrypted_b64, &key)
}

/// Encrypt an SMTP password with a machine-bound key.
///
/// Uses a different domain-separation prefix than API keys so that
/// ciphertexts cannot be confused across purposes.
pub fn encrypt_smtp_password(plaintext: &str, machine_id: &str) -> Result<String, CoreError> {
    let key = derive_key(SMTP_DOMAIN, machine_id);
    encrypt(plaintext, &key)
}

/// Decrypt an SMTP password previously encrypted with [`encrypt_smtp_password`].
///
/// If decryption fails (e.g. because the stored value is legacy plaintext),
/// returns the original string unchanged so that existing configurations
/// continue to work.
pub fn decrypt_smtp_password(encrypted_b64: &str, machine_id: &str) -> Result<String, CoreError> {
    let key = derive_key(SMTP_DOMAIN, machine_id);
    match decrypt(encrypted_b64, &key) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => {
            // Legacy plaintext — return as-is so existing configs aren't broken.
            Ok(encrypted_b64.to_string())
        }
    }
}

/// Encrypt an SMTP password for at-rest storage using a static app-level key.
///
/// Unlike [`encrypt_smtp_password`], this does NOT bind to the machine
/// fingerprint — it uses a hardcoded domain key so that the database can
/// be copied between machines without losing access to the SMTP password.
/// Provides defence against casual database inspection.
pub fn encrypt_smtp_at_rest(password: &str) -> String {
    let key = derive_static_key();
    encrypt(password, &key).unwrap_or_else(|_| password.to_string())
}

/// Decrypt an SMTP password stored with [`encrypt_smtp_at_rest`].
///
/// If the value is legacy plaintext (decryption fails), returns it unchanged.
pub fn decrypt_smtp_at_rest(encrypted: &str) -> String {
    let key = derive_static_key();
    decrypt(encrypted, &key).unwrap_or_else(|_| encrypted.to_string())
}

/// Derive a static 256-bit key for SMTP at-rest encryption.
fn derive_static_key() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"oz-pos.smtp-at-rest.v1:");
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

/// Derive a static 256-bit key for user-profile at-rest encryption.
///
/// Profile fields (national id, monthly take-home pay) are encrypted at
/// rest with a domain-separated static key (the [`encrypt_smtp_at_rest`]
/// precedent) so the data stays readable after a database restore on a
/// different machine — profile data must survive device migration, unlike
/// machine-bound API keys. Provides defence against casual database
/// inspection, not against an attacker holding the app binary.
fn derive_profile_static_key() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"oz-pos.user-profile-at-rest.v1:");
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

/// Encrypt a user-profile sensitive field (ADR #35 D6 / spec 0049) for
/// at-rest storage. Uses a fresh random nonce per call.
pub fn encrypt_profile_field(plaintext: &str) -> Result<String, CoreError> {
    let key = derive_profile_static_key();
    encrypt(plaintext, &key)
}

/// Decrypt a user-profile sensitive field previously encrypted with
/// [`encrypt_profile_field`].
///
/// Fails closed: corrupted, truncated, or cross-domain ciphertext returns
/// an error — never plaintext.
pub fn decrypt_profile_field(encrypted_b64: &str) -> Result<String, CoreError> {
    let key = derive_profile_static_key();
    decrypt(encrypted_b64, &key)
}

/// Internal: encrypt plaintext with a pre-derived key.
fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, CoreError> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = GenericArray::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| CoreError::Internal(format!("encryption failed: {e}")))?;

    // Format: nonce (12) + ciphertext+tag (variable)
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(base64_encode(&combined))
}

/// Internal: decrypt a base64-encoded ciphertext with a pre-derived key.
fn decrypt(encrypted_b64: &str, key: &[u8; 32]) -> Result<String, CoreError> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    let combined = base64_decode(encrypted_b64)?;

    if combined.len() < 12 + 16 {
        // Minimum: nonce (12) + at least one GCM block (16 for tag)
        return Err(CoreError::Internal(
            "encrypted data too short: corrupted or tampered".into(),
        ));
    }

    let nonce = GenericArray::from_slice(&combined[..12]);
    let ciphertext = &combined[12..];

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CoreError::Internal(format!("decryption failed: {e}")))?;

    String::from_utf8(plaintext)
        .map_err(|e| CoreError::Internal(format!("decrypted data is not valid UTF-8: {e}")))
}

// ── Base64 helpers ────────────────────────────────────────────────────

/// Encode bytes as URL-safe base64 (no padding).
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// Decode URL-safe base64 (with or without padding).
fn base64_decode(encoded: &str) -> Result<Vec<u8>, CoreError> {
    use base64::Engine;
    // Accept both standard and URL-safe, with or without padding.
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(encoded))
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(encoded))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(encoded))
        .map_err(|e| CoreError::Internal(format!("failed to decode base64 ciphertext: {e}")))
}

#[cfg(test)]
#[path = "crypto_tests.rs"]
mod tests;

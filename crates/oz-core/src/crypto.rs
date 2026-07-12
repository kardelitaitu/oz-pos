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
fn derive_key(machine_id: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    // Domain-separation prefix prevents key reuse across different
    // encryption purposes within the same codebase.
    hasher.update(b"oz-pos.api-key.v1:");
    hasher.update(machine_id.as_bytes());
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

/// Encrypt `plaintext` with a key derived from `machine_id`.
///
/// Returns a base64-encoded ciphertext containing the nonce,
/// encrypted data, and GCM authentication tag.
pub fn encrypt_api_key(plaintext: &str, machine_id: &str) -> Result<String, CoreError> {
    let key = derive_key(machine_id);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));

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

/// Decrypt a ciphertext previously produced by [`encrypt_api_key`].
///
/// Returns the original plaintext, or an error if the ciphertext is
/// malformed, the key is wrong, or the authentication tag doesn't
/// verify.
pub fn decrypt_api_key(encrypted_b64: &str, machine_id: &str) -> Result<String, CoreError> {
    let key = derive_key(machine_id);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));

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
    base64::engine::general_purpose::URL_SAFE
        .decode(encoded)
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(encoded))
        .map_err(|e| CoreError::Internal(format!("failed to decode base64 ciphertext: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_api_key() {
        let machine_id = "abc123def456789";
        let original = "oz_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

        let encrypted = encrypt_api_key(original, machine_id).unwrap();
        let decrypted = decrypt_api_key(&encrypted, machine_id).unwrap();

        assert_eq!(decrypted, original);
    }

    #[test]
    fn roundtrip_empty_string() {
        let encrypted = encrypt_api_key("", "machine-1").unwrap();
        let decrypted = decrypt_api_key(&encrypted, "machine-1").unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn different_machine_id_fails() {
        let encrypted = encrypt_api_key("secret", "machine-a").unwrap();
        let result = decrypt_api_key(&encrypted, "machine-b");
        assert!(
            result.is_err(),
            "decryption with wrong machine ID should fail"
        );
    }

    #[test]
    fn same_plaintext_produces_different_ciphertext() {
        // Each encryption uses a fresh random nonce, so ciphertext
        // should differ across calls.
        let c1 = encrypt_api_key("secret", "machine-1").unwrap();
        let c2 = encrypt_api_key("secret", "machine-1").unwrap();
        assert_ne!(c1, c2, "nonce should produce distinct ciphertexts");
    }

    #[test]
    fn corrupted_ciphertext_fails() {
        let encrypted = encrypt_api_key("secret", "machine-1").unwrap();
        // Flip a byte in the base64 string
        let mut chars: Vec<char> = encrypted.chars().collect();
        chars[5] = if chars[5] == 'A' { 'B' } else { 'A' };
        let corrupted: String = chars.into_iter().collect();

        let result = decrypt_api_key(&corrupted, "machine-1");
        assert!(
            result.is_err(),
            "corrupted ciphertext should fail decryption"
        );
    }

    #[test]
    fn empty_ciphertext_fails() {
        let result = decrypt_api_key("", "machine-1");
        assert!(result.is_err());
    }

    #[test]
    fn too_short_ciphertext_fails() {
        // 11 bytes of base64 (too short for 12-byte nonce + 16-byte tag)
        let result = decrypt_api_key("YWJj", "machine-1");
        assert!(result.is_err());
    }

    #[test]
    fn key_is_deterministic() {
        let k1 = derive_key("machine-1");
        let k2 = derive_key("machine-1");
        assert_eq!(k1, k2);
    }

    #[test]
    fn different_machine_ids_produce_different_keys() {
        let k1 = derive_key("machine-a");
        let k2 = derive_key("machine-b");
        assert_ne!(k1, k2);
    }
}

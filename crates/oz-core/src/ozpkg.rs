//! Encrypted OZ-POS data export/import format (`.ozpkg`).
//!
//! # Format
//!
//! An `.ozpkg` file is a binary envelope:
//!
//! 1. **Plaintext JSON header** (256 bytes, space-padded) — contains
//!    format version, feature flag metadata, data types included, and
//!    the encryption parameters (salt, nonce). No sensitive data here.
//! 2. **Compressed + encrypted payload** — the actual data rows are
//!    serialized to JSON, compressed with zstd, then encrypted with
//!    AES-256-GCM using a key derived from the user's password via
//!    Argon2id.
//!
//! # Security properties
//!
//! - Password is never stored — only the Argon2id salt is in the header.
//! - AES-256-GCM provides authenticated encryption (integrity + secrecy).
//! - zstd compression runs before encryption (optimal compression ratio).
//! - Each export uses a fresh random salt and random nonce.

use std::collections::HashMap;

use aead::{Aead, KeyInit, OsRng};
use aes_gcm::Aes256Gcm;
use aes_gcm::Nonce;
use argon2::Argon2;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::CoreError;

// ── Constants ──────────────────────────────────────────────────────────

/// Current `.ozpkg` format version.
const FORMAT_VERSION: u32 = 1;

/// Length of the plaintext header in bytes (space-padded).
const HEADER_LEN: usize = 512;

/// Argon2id parameters (tuned for < 1s on modern hardware).
const ARGON_MEMORY: u32 = 19456; // 19 MB
const ARGON_ITERATIONS: u32 = 2;
const ARGON_PARALLELISM: u32 = 1;

/// Salt length in bytes (16 = 128 bits).
const SALT_LEN: usize = 16;

/// AES-GCM nonce length (96 bits = 12 bytes).
const NONCE_LEN: usize = 12;

/// AES-256 key length (256 bits = 32 bytes).
const KEY_LEN: usize = 32;

// ── Header types ──────────────────────────────────────────────────────

/// Plaintext metadata written at the start of every `.ozpkg` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzpkgHeader {
    /// Format version (currently 1).
    pub version: u32,
    /// Store name (from settings).
    pub store_name: String,
    /// OZ-POS version that created this export.
    pub app_version: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Data types included (e.g. `["products", "categories"]`).
    pub data_types: Vec<String>,
    /// Argon2id salt (hex-encoded, 32 hex chars).
    pub salt: String,
    /// AES-GCM nonce (hex-encoded, 24 hex chars).
    pub nonce: String,
    /// Feature flags embedded as plaintext metadata.
    pub features: HashMap<String, String>,
}

// ── Payload types ─────────────────────────────────────────────────────

/// All data that can be exported from an OZ-POS store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzpkgPayload {
    /// Product records.
    pub products: Vec<serde_json::Value>,
    /// Category records.
    pub categories: Vec<serde_json::Value>,
    /// Sale records (header only, no lines for privacy). Optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sales: Option<Vec<serde_json::Value>>,
    /// Customer records.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customers: Option<Vec<serde_json::Value>>,
    /// User records (no PIN hashes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users: Option<Vec<serde_json::Value>>,
    /// Settings rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Vec<serde_json::Value>>,
}

// ── Export / Import functions ─────────────────────────────────────────

/// Export data into an encrypted `.ozpkg` byte vector.
///
/// `password` is the user-chosen encryption password. `data_types` lists
/// the types of data included (for the plaintext header). `payload` is
/// the actual data to encrypt.
///
/// # Errors
///
/// Returns `CoreError::Internal` if encryption setup fails.
pub fn export_ozpkg(
    password: &str,
    store_name: &str,
    app_version: &str,
    data_types: Vec<String>,
    features: HashMap<String, String>,
    payload: &OzpkgPayload,
) -> Result<Vec<u8>, CoreError> {
    // 1. Generate random salt and nonce.
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);

    // 2. Derive AES-256 key via Argon2id.
    let mut key = [0u8; KEY_LEN];
    Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(
            ARGON_MEMORY,
            ARGON_ITERATIONS,
            ARGON_PARALLELISM,
            Some(KEY_LEN),
        )
        .map_err(|e| CoreError::Internal(format!("Argon2 params: {e}")))?,
    )
    .hash_password_into(password.as_bytes(), &salt, &mut key)
    .map_err(|e| CoreError::Internal(format!("Argon2 key derivation: {e}")))?;

    // 3. Serialize payload to JSON.
    let payload_json = serde_json::to_vec(payload)
        .map_err(|e| CoreError::Internal(format!("JSON serialize: {e}")))?;

    // 4. Compress with zstd.
    let compressed = zstd::encode_all(std::io::Cursor::new(&payload_json), 3)
        .map_err(|e| CoreError::Internal(format!("zstd compress: {e}")))?;

    // 5. Encrypt with AES-256-GCM.
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CoreError::Internal(format!("AES-GCM init: {e}")))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, compressed.as_ref())
        .map_err(|e| CoreError::Internal(format!("AES-GCM encrypt: {e}")))?;

    // 6. Build the plaintext header.
    let header = OzpkgHeader {
        version: FORMAT_VERSION,
        store_name: store_name.to_owned(),
        app_version: app_version.to_owned(),
        created_at: chrono::Utc::now().to_rfc3339(),
        data_types,
        salt: hex::encode(salt),
        nonce: hex::encode(nonce_bytes),
        features,
    };

    let header_json = serde_json::to_vec(&header)
        .map_err(|e| CoreError::Internal(format!("header JSON: {e}")))?;

    // Pad header to HEADER_LEN bytes.
    let mut header_padded = vec![b' '; HEADER_LEN];
    let header_len = header_json.len().min(HEADER_LEN);
    header_padded[..header_len].copy_from_slice(&header_json[..header_len]);

    // 7. Concatenate header + ciphertext.
    let mut result = header_padded;
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Import data from an encrypted `.ozpkg` byte slice.
///
/// Returns the header (plaintext metadata) and the decrypted payload.
///
/// # Errors
///
/// Returns `CoreError::Internal` if decryption fails (wrong password or
/// corrupt data).
pub fn import_ozpkg(data: &[u8], password: &str) -> Result<(OzpkgHeader, OzpkgPayload), CoreError> {
    if data.len() < HEADER_LEN {
        return Err(CoreError::Internal("file too short: missing header".into()));
    }

    // 1. Parse header from first HEADER_LEN bytes.
    // Trim trailing spaces (padding) while preserving spaces inside JSON.
    let header_bytes = &data[..HEADER_LEN];
    let trimmed_len = header_bytes
        .iter()
        .rposition(|&b| b != b' ')
        .map(|pos| pos + 1)
        .unwrap_or(0);
    let header: OzpkgHeader = serde_json::from_slice(&header_bytes[..trimmed_len])
        .map_err(|e| CoreError::Internal(format!("invalid header: {e}")))?;

    if header.version != FORMAT_VERSION {
        return Err(CoreError::Internal(format!(
            "unsupported format version: {} (expected {FORMAT_VERSION})",
            header.version
        )));
    }

    // 2. Decode salt and nonce.
    let salt = hex::decode(&header.salt)
        .map_err(|e| CoreError::Internal(format!("invalid salt hex: {e}")))?;
    let nonce_bytes = hex::decode(&header.nonce)
        .map_err(|e| CoreError::Internal(format!("invalid nonce hex: {e}")))?;

    if salt.len() != SALT_LEN || nonce_bytes.len() != NONCE_LEN {
        return Err(CoreError::Internal("invalid salt or nonce length".into()));
    }

    // 3. Derive AES-256 key via Argon2id.
    let mut key = [0u8; KEY_LEN];
    Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(
            ARGON_MEMORY,
            ARGON_ITERATIONS,
            ARGON_PARALLELISM,
            Some(KEY_LEN),
        )
        .map_err(|e| CoreError::Internal(format!("Argon2 params: {e}")))?,
    )
    .hash_password_into(password.as_bytes(), &salt, &mut key)
    .map_err(|e| CoreError::Internal(format!("Argon2 key derivation: {e}")))?;

    // 4. Decrypt with AES-256-GCM.
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CoreError::Internal(format!("AES-GCM init: {e}")))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let compressed = cipher.decrypt(nonce, &data[HEADER_LEN..]).map_err(|_| {
        CoreError::Internal("decryption failed: wrong password or corrupt data".into())
    })?;

    // 5. Decompress with zstd.
    let decompressed = zstd::decode_all(std::io::Cursor::new(&compressed))
        .map_err(|e| CoreError::Internal(format!("zstd decompress: {e}")))?;

    // 6. Deserialize payload.
    let payload: OzpkgPayload = serde_json::from_slice(&decompressed)
        .map_err(|e| CoreError::Internal(format!("JSON deserialize: {e}")))?;

    Ok((header, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_minimal() {
        let payload = OzpkgPayload {
            products: vec![],
            categories: vec![],
            sales: None,
            customers: None,
            users: None,
            settings: None,
        };

        let mut features = HashMap::new();
        features.insert("simple-retail".into(), "1".into());

        let exported = export_ozpkg(
            "test-password-123",
            "My Store",
            "0.0.1",
            vec!["products".into()],
            features.clone(),
            &payload,
        )
        .unwrap();

        let (header, imported) = import_ozpkg(&exported, "test-password-123").unwrap();

        assert_eq!(header.version, FORMAT_VERSION);
        assert_eq!(header.store_name, "My Store");
        assert_eq!(header.app_version, "0.0.1");
        assert_eq!(header.data_types, vec!["products"]);
        assert_eq!(header.features, features);
        assert!(imported.products.is_empty());
        assert!(imported.categories.is_empty());
        assert!(imported.sales.is_none());
    }

    #[test]
    fn roundtrip_with_data() {
        let payload = OzpkgPayload {
            products: vec![serde_json::json!({"sku": "LATTE", "name": "Latte", "price": 450})],
            categories: vec![
                serde_json::json!({"id": "cat-drinks", "name": "Drinks", "colour": "#06b6d4"}),
            ],
            sales: Some(vec![]),
            customers: Some(vec![serde_json::json!({"id": "cust-1", "name": "Alice"})]),
            users: None,
            settings: Some(vec![
                serde_json::json!({"key": "store.name", "value": "My Store"}),
            ]),
        };

        let exported = export_ozpkg(
            "strong-password-here!",
            "My Store",
            "0.0.1",
            vec!["products".into(), "categories".into(), "customers".into()],
            HashMap::new(),
            &payload,
        )
        .unwrap();

        let (_header, imported) = import_ozpkg(&exported, "strong-password-here!").unwrap();

        assert_eq!(imported.products.len(), 1);
        assert_eq!(imported.categories.len(), 1);
        assert_eq!(imported.customers.as_ref().unwrap().len(), 1);
        assert_eq!(imported.settings.as_ref().unwrap().len(), 1);
        assert!(imported.users.is_none());
    }

    #[test]
    fn wrong_password_fails() {
        let payload = OzpkgPayload {
            products: vec![],
            categories: vec![],
            sales: None,
            customers: None,
            users: None,
            settings: None,
        };

        let exported = export_ozpkg(
            "correct-password",
            "Store",
            "0.0.1",
            vec![],
            HashMap::new(),
            &payload,
        )
        .unwrap();

        let result = import_ozpkg(&exported, "wrong-password");
        assert!(
            result.is_err(),
            "decryption should fail with wrong password"
        );
    }

    #[test]
    fn corrupted_data_fails() {
        let payload = OzpkgPayload {
            products: vec![],
            categories: vec![],
            sales: None,
            customers: None,
            users: None,
            settings: None,
        };

        let mut exported = export_ozpkg(
            "password",
            "Store",
            "0.0.1",
            vec![],
            HashMap::new(),
            &payload,
        )
        .unwrap();

        // Corrupt a byte in the ciphertext.
        let last = exported.len() - 1;
        exported[last] ^= 0x01;

        let result = import_ozpkg(&exported, "password");
        assert!(
            result.is_err(),
            "decryption should fail with corrupted data"
        );
    }

    #[test]
    fn empty_password_allowed() {
        let payload = OzpkgPayload {
            products: vec![],
            categories: vec![],
            sales: None,
            customers: None,
            users: None,
            settings: None,
        };

        let exported =
            export_ozpkg("", "Store", "0.0.1", vec![], HashMap::new(), &payload).unwrap();

        let result = import_ozpkg(&exported, "");
        assert!(
            result.is_ok(),
            "empty password should work (though not recommended)"
        );
    }

    #[test]
    fn header_metadata_preserved() {
        let payload = OzpkgPayload {
            products: vec![],
            categories: vec![],
            sales: None,
            customers: None,
            users: None,
            settings: None,
        };

        let mut features = HashMap::new();
        features.insert("cash-payment".into(), "1".into());
        features.insert("barcode-scanning".into(), "1".into());

        let exported = export_ozpkg(
            "password",
            "Test Store",
            "0.1.0",
            vec!["products".into(), "settings".into()],
            features.clone(),
            &payload,
        )
        .unwrap();

        let (header, _) = import_ozpkg(&exported, "password").unwrap();

        assert_eq!(header.store_name, "Test Store");
        assert_eq!(header.app_version, "0.1.0");
        assert_eq!(header.data_types, vec!["products", "settings"]);
        assert_eq!(header.features, features);
        assert!(!header.created_at.is_empty());
        assert!(!header.salt.is_empty());
        assert!(!header.nonce.is_empty());
    }

    #[test]
    fn large_payload_roundtrip() {
        let products: Vec<serde_json::Value> = (0..100)
            .map(|i| serde_json::json!({"sku": format!("SKU-{i:04}"), "name": format!("Product {i}"), "price": 100 + i}))
            .collect();

        let payload = OzpkgPayload {
            products: products.clone(),
            categories: vec![],
            sales: None,
            customers: None,
            users: None,
            settings: None,
        };

        let exported = export_ozpkg(
            "large-payload-password",
            "Big Store",
            "0.0.1",
            vec!["products".into()],
            HashMap::new(),
            &payload,
        )
        .unwrap();

        let (_header, imported) = import_ozpkg(&exported, "large-payload-password").unwrap();

        assert_eq!(imported.products.len(), 100);
        assert_eq!(imported.products[0]["sku"], "SKU-0000");
        assert_eq!(imported.products[99]["sku"], "SKU-0099");
    }
}

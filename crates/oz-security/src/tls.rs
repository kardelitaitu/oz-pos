//! TLS configuration helpers for secure cloud sync connections.
//!
//! This module provides helper types for loading TLS certificates and
//! private keys from PEM files, building a TLS connector for use with
//! `tokio`-based networking crates.
//!
//! # Example
//!
//! ```no_run
//! use oz_security::tls::TlsConfig;
//!
//! let tls = TlsConfig::builder()
//!     .cert_path("/etc/oz-pos/certs/cert.pem")
//!     .key_path("/etc/oz-pos/certs/key.pem")
//!     .ca_path("/etc/oz-pos/certs/ca.pem")
//!     .build()?;
//! # Ok::<_, oz_security::SecurityError>(())
//! ```

use std::path::{Path, PathBuf};

use crate::SecurityError;
use serde::{Deserialize, Serialize};

/// TLS configuration for outbound connections.
///
/// Supports optional client certificate authentication and custom CA
/// bundles for self-signed or internal certificates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to the client certificate (PEM).
    pub cert_path: Option<PathBuf>,
    /// Path to the client private key (PEM).
    pub key_path: Option<PathBuf>,
    /// Path to the CA certificate bundle (PEM).
    pub ca_path: Option<PathBuf>,
    /// Whether to skip TLS verification (development only).
    #[serde(default)]
    pub insecure_skip_verify: bool,
    /// Optional ALPN protocols (e.g. "h2", "http/1.1").
    #[serde(default)]
    pub alpn_protocols: Vec<String>,
}

impl TlsConfig {
    /// Create a new `TlsConfigBuilder`.
    pub fn builder() -> TlsConfigBuilder {
        TlsConfigBuilder::default()
    }

    /// Validate the configuration.
    ///
    /// Checks that:
    /// - If `cert_path` is set, `key_path` must also be set (and vice versa).
    /// - All specified paths exist.
    pub fn validate(&self) -> Result<(), SecurityError> {
        // Cert and key must be provided together.
        match (&self.cert_path, &self.key_path) {
            (Some(_), None) => {
                return Err(SecurityError::KeyUnavailable(
                    "cert_path set but key_path is missing".into(),
                ));
            }
            (None, Some(_)) => {
                return Err(SecurityError::KeyUnavailable(
                    "key_path set but cert_path is missing".into(),
                ));
            }
            _ => {}
        }

        // Verify paths exist.
        for path in self
            .cert_path
            .iter()
            .chain(self.key_path.iter())
            .chain(self.ca_path.iter())
        {
            if !path.exists() {
                return Err(SecurityError::KeyUnavailable(format!(
                    "TLS file not found: {}",
                    path.display()
                )));
            }
        }

        Ok(())
    }

    /// Load the client certificate (if configured).
    ///
    /// Returns the PEM-encoded certificate bytes.
    pub fn load_cert(&self) -> Result<Option<Vec<u8>>, SecurityError> {
        match &self.cert_path {
            Some(path) => {
                let data = std::fs::read(path)
                    .map_err(|e| SecurityError::KeyUnavailable(format!("reading cert: {e}")))?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Load the client private key (if configured).
    ///
    /// Returns the PEM-encoded key bytes.
    pub fn load_key(&self) -> Result<Option<Vec<u8>>, SecurityError> {
        match &self.key_path {
            Some(path) => {
                let data = std::fs::read(path)
                    .map_err(|e| SecurityError::KeyUnavailable(format!("reading key: {e}")))?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Load the CA certificate bundle (if configured).
    pub fn load_ca(&self) -> Result<Option<Vec<u8>>, SecurityError> {
        match &self.ca_path {
            Some(path) => {
                let data = std::fs::read(path)
                    .map_err(|e| SecurityError::KeyUnavailable(format!("reading CA: {e}")))?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }
}

/// Builder for [`TlsConfig`].
#[derive(Debug, Default)]
pub struct TlsConfigBuilder {
    cert_path: Option<PathBuf>,
    key_path: Option<PathBuf>,
    ca_path: Option<PathBuf>,
    insecure_skip_verify: bool,
    alpn_protocols: Vec<String>,
}

impl TlsConfigBuilder {
    /// Set the client certificate path (PEM).
    pub fn cert_path(mut self, path: impl AsRef<Path>) -> Self {
        self.cert_path = Some(path.as_ref().to_owned());
        self
    }

    /// Set the client private key path (PEM).
    pub fn key_path(mut self, path: impl AsRef<Path>) -> Self {
        self.key_path = Some(path.as_ref().to_owned());
        self
    }

    /// Set the CA certificate bundle path (PEM).
    pub fn ca_path(mut self, path: impl AsRef<Path>) -> Self {
        self.ca_path = Some(path.as_ref().to_owned());
        self
    }

    /// Skip TLS certificate verification (development only).
    pub fn insecure_skip_verify(mut self, skip: bool) -> Self {
        self.insecure_skip_verify = skip;
        self
    }

    /// Add an ALPN protocol.
    pub fn alpn_protocol(mut self, protocol: impl Into<String>) -> Self {
        self.alpn_protocols.push(protocol.into());
        self
    }

    /// Build the `TlsConfig`.
    pub fn build(self) -> Result<TlsConfig, SecurityError> {
        let config = TlsConfig {
            cert_path: self.cert_path,
            key_path: self.key_path,
            ca_path: self.ca_path,
            insecure_skip_verify: self.insecure_skip_verify,
            alpn_protocols: self.alpn_protocols,
        };
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn tls_config_builder_minimal() {
        let config = TlsConfig::builder().build().unwrap();
        assert!(config.cert_path.is_none());
        assert!(config.key_path.is_none());
        assert!(config.ca_path.is_none());
        assert!(!config.insecure_skip_verify);
    }

    #[test]
    fn tls_config_builder_with_paths() {
        let dir = std::env::temp_dir();
        let cert_path = dir.join("oz-pos-test-cert.pem");
        let key_path = dir.join("oz-pos-test-key.pem");
        let ca_path = dir.join("oz-pos-test-ca.pem");
        fs::write(&cert_path, b"cert").unwrap();
        fs::write(&key_path, b"key").unwrap();
        fs::write(&ca_path, b"ca").unwrap();

        let config = TlsConfig::builder()
            .cert_path(cert_path.to_str().unwrap())
            .key_path(key_path.to_str().unwrap())
            .ca_path(ca_path.to_str().unwrap())
            .insecure_skip_verify(true)
            .build()
            .unwrap();
        assert_eq!(config.cert_path.unwrap(), cert_path);

        fs::remove_file(&cert_path).ok();
        fs::remove_file(&key_path).ok();
        fs::remove_file(&ca_path).ok();
    }

    #[test]
    fn tls_config_validates_both_or_neither() {
        // Only cert, no key — should fail.
        let err = TlsConfig::builder()
            .cert_path("/tmp/cert.pem")
            .build()
            .unwrap_err();
        assert!(matches!(err, SecurityError::KeyUnavailable(_)));

        // Only key, no cert — should fail.
        let err = TlsConfig::builder()
            .key_path("/tmp/key.pem")
            .build()
            .unwrap_err();
        assert!(matches!(err, SecurityError::KeyUnavailable(_)));
    }

    #[test]
    fn tls_config_validates_file_exists() {
        let err = TlsConfig::builder()
            .cert_path("/tmp/nonexistent-cert.pem")
            .key_path("/tmp/nonexistent-key.pem")
            .build()
            .unwrap_err();
        assert!(matches!(err, SecurityError::KeyUnavailable(_)));
    }

    #[test]
    fn tls_config_load_files_roundtrip() {
        let dir = std::env::temp_dir().join(format!("oz-tls-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let cert_path = dir.join("cert.pem");
        let key_path = dir.join("key.pem");
        let ca_path = dir.join("ca.pem");

        let mut f = fs::File::create(&cert_path).unwrap();
        f.write_all(b"-----BEGIN CERTIFICATE-----\nfake-cert\n-----END CERTIFICATE-----\n")
            .unwrap();
        let mut f = fs::File::create(&key_path).unwrap();
        f.write_all(b"-----BEGIN PRIVATE KEY-----\nfake-key\n-----END PRIVATE KEY-----\n")
            .unwrap();
        let mut f = fs::File::create(&ca_path).unwrap();
        f.write_all(b"-----BEGIN CERTIFICATE-----\nfake-ca\n-----END CERTIFICATE-----\n")
            .unwrap();

        let config = TlsConfig::builder()
            .cert_path(&cert_path)
            .key_path(&key_path)
            .ca_path(&ca_path)
            .build()
            .unwrap();

        assert!(config.load_cert().unwrap().is_some());
        assert!(config.load_key().unwrap().is_some());
        assert!(config.load_ca().unwrap().is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tls_config_builder_insecure() {
        let config = TlsConfig::builder()
            .insecure_skip_verify(true)
            .build()
            .unwrap();
        assert!(config.insecure_skip_verify);
    }

    #[test]
    fn tls_config_alpn() {
        let config = TlsConfig::builder()
            .alpn_protocol("h2")
            .alpn_protocol("http/1.1")
            .build()
            .unwrap();
        assert_eq!(config.alpn_protocols, vec!["h2", "http/1.1"]);
    }

    // ── Boundary / invariant tests for tls.rs ────────────────────────

    /// `load_cert` must return `Ok(None)` (not `Err`) when `cert_path`
    /// is `None`. Symmetric with `load_key` and `load_ca`. This pins
    /// the API contract that a missing path is NOT a load failure —
    /// it's simply "not configured".
    #[test]
    fn tls_config_load_cert_returns_none_when_unset() {
        let config = TlsConfig::builder().build().unwrap();
        assert!(matches!(config.load_cert(), Ok(None)));
        assert!(matches!(config.load_key(), Ok(None)));
        assert!(matches!(config.load_ca(), Ok(None)));
    }

    /// CA-only configuration (no cert, no key) is valid: TLS clients
    /// that only need to verify a server certificate use this shape.
    /// `validate()` should accept it as long as the CA file exists.
    #[test]
    fn tls_config_validates_ca_only() {
        let dir = std::env::temp_dir().join(format!("oz-tls-ca-only-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let ca_path = dir.join("ca.pem");
        fs::write(&ca_path, b"fake-ca").unwrap();

        let config = TlsConfig::builder().ca_path(&ca_path).build().unwrap();
        assert!(config.ca_path.is_some());
        assert!(config.cert_path.is_none());
        assert!(config.key_path.is_none());

        fs::remove_dir_all(&dir).ok();
    }

    /// Validate-time success is NOT load-time success: if the file is
    /// deleted between `validate()` and `load_cert()`, the load MUST
    /// return an error. Pins the TOCTOU behavior — security-relevant,
    /// because a config that passes validation at startup should not
    /// be assumed valid at the moment of use.
    #[test]
    fn tls_config_load_fails_when_file_deleted_after_validate() {
        let dir = std::env::temp_dir().join(format!("oz-tls-toctou-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let cert_path = dir.join("cert.pem");
        let key_path = dir.join("key.pem");
        fs::write(&cert_path, b"cert-bytes").unwrap();
        fs::write(&key_path, b"key-bytes").unwrap();

        let config = TlsConfig::builder()
            .cert_path(&cert_path)
            .key_path(&key_path)
            .build()
            .unwrap();

        // Delete cert AFTER validate succeeds.
        fs::remove_file(&cert_path).unwrap();

        let err = config.load_cert().unwrap_err();
        assert!(matches!(err, SecurityError::KeyUnavailable(_)));

        fs::remove_dir_all(&dir).ok();
    }

    /// ALPN protocols with unusual contents (empty strings, NUL bytes,
    /// emoji) must be stored verbatim. The builder does not validate
    /// ALPN strings — that's the rustls layer. We pin that contract.
    #[test]
    fn tls_config_alpn_preserves_arbitrary_strings() {
        let config = TlsConfig::builder()
            .alpn_protocol("")
            .alpn_protocol("h2")
            .alpn_protocol("\u{0000}weird\u{1F60A}")
            .alpn_protocol("h2") // duplicate
            .build()
            .unwrap();
        // Order is preserved; duplicates are NOT deduplicated by the builder.
        assert_eq!(
            config.alpn_protocols,
            vec!["", "h2", "\u{0000}weird\u{1F60A}", "h2"]
        );
    }

    /// `insecure_skip_verify` is a boolean — toggling it true then
    /// false then true must round-trip correctly through `build()`.
    /// Pins that the builder has no implicit state retention.
    #[test]
    fn tls_config_insecure_skip_verify_state_cycle() {
        let config_on = TlsConfig::builder()
            .insecure_skip_verify(true)
            .build()
            .unwrap();
        let config_off = TlsConfig::builder()
            .insecure_skip_verify(false)
            .build()
            .unwrap();
        let config_on_again = TlsConfig::builder()
            .insecure_skip_verify(true)
            .build()
            .unwrap();
        assert!(config_on.insecure_skip_verify);
        assert!(!config_off.insecure_skip_verify);
        assert!(config_on_again.insecure_skip_verify);
    }

    /// `TlsConfig` derives `Serialize`/`Deserialize`. A roundtrip
    /// through `serde_json` must preserve all fields exactly,
    /// including the `Option<PathBuf>` and `Vec<String>`. This pins
    /// the on-disk shape used by the sync-config persistence layer.
    #[test]
    fn tls_config_json_roundtrip() {
        let dir = std::env::temp_dir().join(format!("oz-tls-json-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let cert_path = dir.join("cert.pem");
        let key_path = dir.join("key.pem");
        fs::write(&cert_path, b"cert").unwrap();
        fs::write(&key_path, b"key").unwrap();

        let original = TlsConfig::builder()
            .cert_path(&cert_path)
            .key_path(&key_path)
            .ca_path(&cert_path)
            .insecure_skip_verify(true)
            .alpn_protocol("h2")
            .alpn_protocol("http/1.1")
            .build()
            .unwrap();

        let json = serde_json::to_string(&original).unwrap();
        let restored: TlsConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.cert_path, original.cert_path);
        assert_eq!(restored.key_path, original.key_path);
        assert_eq!(restored.ca_path, original.ca_path);
        assert_eq!(restored.insecure_skip_verify, original.insecure_skip_verify);
        assert_eq!(restored.alpn_protocols, original.alpn_protocols);

        fs::remove_dir_all(&dir).ok();
    }

    /// `Default::default()` for `TlsConfig` (triggered by serde
    /// `#[serde(default)]` on the fields) must produce a config
    /// equivalent to the minimal builder. Pins the deserialization
    /// fallback for partial configs (e.g. legacy saved configs that
    /// omitted a field).
    #[test]
    fn tls_config_json_default_resolution() {
        // Empty JSON object: all fields fall back to defaults.
        let restored: TlsConfig = serde_json::from_str("{}").unwrap();
        assert!(restored.cert_path.is_none());
        assert!(restored.key_path.is_none());
        assert!(restored.ca_path.is_none());
        assert!(!restored.insecure_skip_verify);
        assert!(restored.alpn_protocols.is_empty());

        // Partial JSON with only one field set: validate must succeed
        // because the others default to None.
        let partial = r#"{"insecure_skip_verify": true}"#;
        let restored: TlsConfig = serde_json::from_str(partial).unwrap();
        assert!(restored.insecure_skip_verify);
        assert!(restored.cert_path.is_none());
        assert!(restored.key_path.is_none());
        restored.validate().unwrap();
    }

    /// JSON deserialization that sets cert WITHOUT key must produce a
    /// config that fails `validate()`. Pins that the on-disk format
    /// cannot bypass the cert/key coupling guard.
    #[test]
    fn tls_config_json_cert_without_key_fails_validate() {
        let json = r#"{"cert_path": "/tmp/cert.pem"}"#;
        let config: TlsConfig = serde_json::from_str(json).unwrap();
        let err = config.validate().unwrap_err();
        assert!(matches!(err, SecurityError::KeyUnavailable(_)));
        assert!(err.to_string().contains("key_path"));
    }
}

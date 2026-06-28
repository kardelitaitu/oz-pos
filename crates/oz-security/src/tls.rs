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
        for opt in [&self.cert_path, &self.key_path, &self.ca_path] {
            if let Some(path) = opt {
                if !path.exists() {
                    return Err(SecurityError::KeyUnavailable(format!(
                        "TLS file not found: {}",
                        path.display()
                    )));
                }
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
        let config = TlsConfig::builder()
            .cert_path("/tmp/test-cert.pem")
            .key_path("/tmp/test-key.pem")
            .ca_path("/tmp/test-ca.pem")
            .insecure_skip_verify(true)
            .build()
            .unwrap();
        assert_eq!(
            config.cert_path.unwrap(),
            PathBuf::from("/tmp/test-cert.pem")
        );
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
        f.write_all(b"-----BEGIN CERTIFICATE-----\nfake-cert\n-----END CERTIFICATE-----\n").unwrap();
        let mut f = fs::File::create(&key_path).unwrap();
        f.write_all(b"-----BEGIN PRIVATE KEY-----\nfake-key\n-----END PRIVATE KEY-----\n").unwrap();
        let mut f = fs::File::create(&ca_path).unwrap();
        f.write_all(b"-----BEGIN CERTIFICATE-----\nfake-ca\n-----END CERTIFICATE-----\n").unwrap();

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
}

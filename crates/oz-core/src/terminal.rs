//! Terminal domain type — registered POS device.

use serde::{Deserialize, Serialize};

/// A registered POS terminal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Terminal {
    /// Internal row id (UUID v4).
    pub id: String,
    /// Human-readable terminal name (e.g. "Front Counter", "Drive-Thru").
    pub name: String,
    /// Unique device identifier (machine hostname, MAC, or hardware ID).
    pub device_id: String,
    /// Optional shared secret for authenticating sync requests.
    pub terminal_secret: Option<String>,
    /// Whether this terminal is active and allowed to operate.
    pub is_active: bool,
    /// ISO-8601 timestamp of last communication from this terminal.
    pub last_seen_at: Option<String>,
    /// JSON blob for extra metadata (OS version, app version, etc.).
    pub metadata: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl Terminal {
    /// Create a new terminal with generated id.
    pub fn new(name: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            device_id: device_id.into(),
            terminal_secret: None,
            is_active: true,
            last_seen_at: None,
            metadata: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    /// Set the terminal secret (builder-style).
    #[must_use]
    pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.terminal_secret = Some(secret.into());
        self
    }

    /// Set metadata JSON (builder-style).
    #[must_use]
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_terminal_has_generated_id() {
        let t = Terminal::new("Front Counter", "host-01");
        assert!(!t.id.is_empty(), "id should be generated");
        assert_eq!(t.name, "Front Counter");
        assert_eq!(t.device_id, "host-01");
        assert!(t.terminal_secret.is_none());
        assert!(t.is_active);
        assert!(t.last_seen_at.is_none());
        assert!(t.metadata.is_none());
        assert!(t.created_at.is_empty());
        assert!(t.updated_at.is_empty());
    }

    #[test]
    fn new_terminal_sets_fields() {
        let t = Terminal::new("Drive-Thru", "host-02");
        assert_eq!(t.name, "Drive-Thru");
        assert_eq!(t.device_id, "host-02");
    }

    #[test]
    fn builder_methods() {
        let t = Terminal::new("Back Office", "host-03")
            .with_secret("s3cr3t")
            .with_metadata(r#"{"os":"windows"}"#);
        assert_eq!(t.terminal_secret, Some("s3cr3t".into()));
        assert_eq!(t.metadata, Some(r#"{"os":"windows"}"#.into()));
    }

    #[test]
    fn serde_roundtrip() {
        let t = Terminal::new("Front Counter", "host-01")
            .with_secret("s3cr3t")
            .with_metadata(r#"{"version":"1.0"}"#);
        let json = serde_json::to_string(&t).unwrap();
        let back: Terminal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }
}

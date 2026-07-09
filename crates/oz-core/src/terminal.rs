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

    // ── Construction ─────────────────────────────────────────────

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
    fn new_terminal_long_name() {
        let long_name = "A".repeat(255);
        let t = Terminal::new(&long_name, "host-01");
        assert_eq!(t.name, long_name);
    }

    #[test]
    fn new_terminal_empty_device_id() {
        let t = Terminal::new("Terminal", "");
        assert_eq!(t.device_id, "");
    }

    #[test]
    fn terminal_id_is_uuid_v4() {
        let t = Terminal::new("Test", "host-01");
        assert_eq!(t.id.len(), 36);
        assert_eq!(t.id.chars().filter(|&c| c == '-').count(), 4);
    }

    // ── Builder methods ──────────────────────────────────────────

    #[test]
    fn builder_methods() {
        let t = Terminal::new("Back Office", "host-03")
            .with_secret("s3cr3t")
            .with_metadata(r#"{"os":"windows"}"#);
        assert_eq!(t.terminal_secret, Some("s3cr3t".into()));
        assert_eq!(t.metadata, Some(r#"{"os":"windows"}"#.into()));
    }

    #[test]
    fn builder_chains_all_methods() {
        let t = Terminal::new("Register 1", "host-01")
            .with_secret("secret-key")
            .with_metadata(r#"{"app":"1.0","os":"linux"}"#);
        assert_eq!(t.name, "Register 1");
        assert_eq!(t.device_id, "host-01");
        assert_eq!(t.terminal_secret, Some("secret-key".into()));
        assert_eq!(t.metadata, Some(r#"{"app":"1.0","os":"linux"}"#.into()));
    }

    #[test]
    fn builder_empty_secret() {
        let t = Terminal::new("Test", "host-01").with_secret("");
        assert_eq!(t.terminal_secret, Some("".into()));
    }

    #[test]
    fn builder_empty_metadata() {
        let t = Terminal::new("Test", "host-01").with_metadata("");
        assert_eq!(t.metadata, Some("".into()));
    }

    // ── Active/inactive ──────────────────────────────────────────

    #[test]
    fn terminal_default_active() {
        let t = Terminal::new("Front Counter", "host-01");
        assert!(t.is_active);
    }

    #[test]
    fn terminal_can_be_deactivated() {
        let mut t = Terminal::new("Decommissioned", "host-old");
        t.is_active = false;
        assert!(!t.is_active);
    }

    // ── last_seen_at ─────────────────────────────────────────────

    #[test]
    fn terminal_last_seen_at_defaults_none() {
        let t = Terminal::new("Front Counter", "host-01");
        assert!(t.last_seen_at.is_none());
    }

    #[test]
    fn terminal_last_seen_at_can_be_set() {
        let mut t = Terminal::new("Front Counter", "host-01");
        t.last_seen_at = Some("2026-07-06T12:00:00.000Z".into());
        assert_eq!(t.last_seen_at.as_deref(), Some("2026-07-06T12:00:00.000Z"));
    }

    // ── Serde ────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let t = Terminal::new("Front Counter", "host-01")
            .with_secret("s3cr3t")
            .with_metadata(r#"{"version":"1.0"}"#);
        let json = serde_json::to_string(&t).unwrap();
        let back: Terminal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn serde_roundtrip_inactive_terminal() {
        let mut t = Terminal::new("Old Terminal", "host-old");
        t.is_active = false;
        let json = serde_json::to_string(&t).unwrap();
        let back: Terminal = serde_json::from_str(&json).unwrap();
        assert!(!back.is_active);
    }

    #[test]
    fn serde_roundtrip_with_last_seen() {
        let mut t = Terminal::new("Terminal A", "host-a");
        t.last_seen_at = Some("2026-07-06T12:00:00.000Z".into());
        let json = serde_json::to_string(&t).unwrap();
        let back: Terminal = serde_json::from_str(&json).unwrap();
        assert_eq!(back.last_seen_at, t.last_seen_at);
    }

    #[test]
    fn serde_roundtrip_minimal() {
        let t = Terminal::new("Minimal", "host-min");
        let json = serde_json::to_string(&t).unwrap();
        let back: Terminal = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Minimal");
        assert!(back.terminal_secret.is_none());
        assert!(back.metadata.is_none());
        assert!(back.last_seen_at.is_none());
        assert!(back.is_active);
    }

    #[test]
    fn serde_json_field_names() {
        let t = Terminal::new("Test", "host-01").with_secret("key");
        let json = serde_json::to_value(&t).unwrap();
        assert_eq!(json["name"], "Test");
        assert_eq!(json["device_id"], "host-01");
        assert_eq!(json["terminal_secret"], "key");
        assert_eq!(json["is_active"], true);
        assert!(json.get("last_seen_at").unwrap().is_null());
    }

    // ── Clone + equality ─────────────────────────────────────────

    #[test]
    fn terminal_clone_eq() {
        let a = Terminal::new("Front Counter", "host-01");
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn terminal_neq_when_field_differs() {
        let a = Terminal::new("Counter 1", "host-01");
        let b = Terminal::new("Counter 2", "host-02");
        assert_ne!(a, b);
    }

    #[test]
    fn terminal_debug_output() {
        let t = Terminal::new("Front Counter", "host-01");
        let debug = format!("{:?}", t);
        assert!(debug.contains("Front Counter"));
        assert!(debug.contains("host-01"));
    }
}

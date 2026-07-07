//! Domain type for per-terminal feature overrides.
//!
//! A [`TerminalFeatureOverride`] lets a specific terminal deviate from
//! the global feature flag set. This is useful when a subset of terminals
//! should have different capabilities (e.g. a kiosk terminal that should
//! not accept card payments).

use serde::{Deserialize, Serialize};

/// A per-terminal feature override.
///
/// When set, this overrides the global feature flag for a specific terminal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalFeatureOverride {
    /// The terminal this override applies to.
    pub terminal_id: String,
    /// The feature key (kebab-case, matching [`crate::feature_key`]).
    pub feature: String,
    /// Whether the feature is enabled for this terminal.
    pub enabled: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_feature_override_serde_roundtrip() {
        let tfo = TerminalFeatureOverride {
            terminal_id: "t1".into(),
            feature: "gift-cards".into(),
            enabled: true,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-07-01T12:00:00Z".into(),
        };
        let json = serde_json::to_string(&tfo).unwrap();
        let back: TerminalFeatureOverride = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tfo);
    }

    #[test]
    fn terminal_feature_override_disabled() {
        let tfo = TerminalFeatureOverride {
            terminal_id: "t2".into(),
            feature: "kds".into(),
            enabled: false,
            created_at: "2026-06-01T00:00:00Z".into(),
            updated_at: "2026-06-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&tfo).unwrap();
        assert!(json.contains("\"enabled\":false"));
        let back: TerminalFeatureOverride = serde_json::from_str(&json).unwrap();
        assert!(!back.enabled);
        assert_eq!(back.terminal_id, "t2");
        assert_eq!(back.feature, "kds");
    }

    #[test]
    fn terminal_feature_override_inequality() {
        let a = TerminalFeatureOverride {
            terminal_id: "t1".into(),
            feature: "gift-cards".into(),
            enabled: true,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let mut b = a.clone();
        b.enabled = false;
        assert_ne!(a, b);
    }

    #[test]
    fn terminal_feature_override_different_terminal() {
        let a = TerminalFeatureOverride {
            terminal_id: "t1".into(),
            feature: "gift-cards".into(),
            enabled: true,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let mut b = a.clone();
        b.terminal_id = "t2".into();
        assert_ne!(a, b);
    }
}

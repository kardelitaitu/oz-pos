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

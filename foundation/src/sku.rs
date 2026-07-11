//! Stock-Keeping Unit (SKU) — a string identifier for a product.
//!
//! `Sku` is `#[serde(transparent)]` so it serializes as its inner
//! `String`. `LineId` is a fresh UUID per line item.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A SKU string. Trimmed, must be non-empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sku(String);

impl Sku {
    /// Construct a SKU from any string-like value, trimming and panicking
    /// if empty.
    ///
    /// # Panics
    /// Panics if the trimmed input is empty.
    pub fn new(s: impl Into<String>) -> Self {
        let trimmed = s.into().trim().to_owned();
        assert!(!trimmed.is_empty(), "SKU cannot be empty");
        Self(trimmed)
    }

    /// Try-constructor returning `None` for empty input.
    #[must_use]
    pub fn try_new(s: impl Into<String>) -> Option<Self> {
        let trimmed = s.into().trim().to_owned();
        if trimmed.is_empty() {
            None
        } else {
            Some(Self(trimmed))
        }
    }

    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Sku {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Sku {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// A `LineId` is a fresh UUID per line item within a cart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LineId(pub Uuid);

impl LineId {
    /// Create a new line identifier backed by a UUID v7.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for LineId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sku_trims_whitespace() {
        let s = Sku::new("  ABC  ");
        assert_eq!(s.as_str(), "ABC");
    }

    #[test]
    #[should_panic(expected = "SKU cannot be empty")]
    fn empty_sku_panics() {
        Sku::new("   ");
    }

    #[test]
    fn try_new_returns_none_for_empty() {
        assert!(Sku::try_new("").is_none());
        assert!(Sku::try_new("ABC").is_some());
    }

    #[test]
    fn line_ids_are_unique() {
        let a = LineId::new();
        let b = LineId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn sku_serializes_as_bare_string() {
        let s = Sku::new("COFFEE");
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"COFFEE\"");
    }

    #[test]
    fn line_id_serializes_as_bare_string() {
        let id = LineId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"') && json.ends_with('"'));
        let back: LineId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    // ── Display / From / Clone / Eq ──

    #[test]
    fn sku_display_formats_as_inner_string() {
        let s = Sku::new("COFFEE");
        assert_eq!(s.to_string(), "COFFEE");
    }

    #[test]
    fn sku_from_str_trait() {
        let s: Sku = "TEA".into();
        assert_eq!(s.as_str(), "TEA");
    }

    #[test]
    fn sku_clone_preserves_value() {
        let s = Sku::new("LATTE");
        let c = s.clone();
        assert_eq!(s, c);
        assert_eq!(c.as_str(), "LATTE");
    }

    #[test]
    fn sku_equality_compares_inner_value() {
        let a = Sku::new("COFFEE");
        let b = Sku::new("COFFEE");
        let c = Sku::new("TEA");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn sku_try_new_trims_whitespace() {
        let s = Sku::try_new("  MOCHA  ").unwrap();
        assert_eq!(s.as_str(), "MOCHA");
    }

    #[test]
    fn sku_try_new_whitespace_only_returns_none() {
        assert!(Sku::try_new("   ").is_none());
        assert!(Sku::try_new("\t\n").is_none());
    }

    // ── LineId ──

    #[test]
    fn line_id_default_creates_new() {
        let a = LineId::default();
        let b = LineId::default();
        assert_ne!(a, b, "each default() should produce a unique UUIDv7");
    }

    #[test]
    fn line_id_display_formats_as_uuid() {
        let id = LineId::new();
        let s = id.to_string();
        assert_eq!(s.len(), 36, "UUIDv7 string should be 36 chars");
        assert!(s.contains('-'), "UUID string should contain hyphens");
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn line_id_clone_and_copy() {
        let a = LineId::new();
        let b = a; // Copy
        let c = a.clone();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }
}

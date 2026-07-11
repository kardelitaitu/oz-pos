//! Stock-Keeping Unit (SKU) — a string identifier for a product.
//!
//! `Sku` is `#[serde(transparent)]` so it serializes as its inner
//! `String`. `LineId` is a fresh UUID per line item.

#![allow(missing_docs)]

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
    fn line_id_clone_and_copy() {
        let a = LineId::new();
        let b = a; // Copy
        let c = a.clone();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    // ── Sku Debug / Hash ─────────────────────────────────────────

    #[test]
    fn sku_debug_format_contains_value() {
        let s = Sku::new("ESPRESSO");
        let debug = format!("{:?}", s);
        assert!(debug.contains("ESPRESSO"));
    }

    #[test]
    fn sku_hash_consistent_with_eq() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let a = Sku::new("COFFEE");
        let b = Sku::new("COFFEE");
        let mut ha = DefaultHasher::new();
        let mut hb = DefaultHasher::new();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }

    // ── Sku serde deserialization ────────────────────────────────

    #[test]
    fn sku_deserialization_roundtrip() {
        let json = "\"MATCHA\"";
        let s: Sku = serde_json::from_str(json).unwrap();
        assert_eq!(s.as_str(), "MATCHA");
    }

    #[test]
    fn sku_deserialize_from_non_string_fails() {
        let json = "42";
        let result: Result<Sku, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn sku_deserialize_from_empty_string_succeeds_via_transparent() {
        // #[serde(transparent)] deserializes the inner String directly,
        // bypassing Sku::new's empty-string guard. This test documents
        // that deserialization accepts "" even though Sku::new("") panics.
        let json = "\"\"";
        let result: Result<Sku, _> = serde_json::from_str(json);
        assert!(
            result.is_ok(),
            "empty string deserializes to Sku with empty inner"
        );
    }

    // ── LineId Debug / Hash / Eq ─────────────────────────────────

    #[test]
    fn line_id_debug_format_contains_uuid() {
        let id = LineId::new();
        let debug = format!("{:?}", id);
        assert!(
            debug.contains('-'),
            "Debug should contain UUID with hyphens"
        );
    }

    #[test]
    fn line_id_hash_consistent_with_eq() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let a = LineId::new();
        let b = a; // Copy
        let mut ha = DefaultHasher::new();
        let mut hb = DefaultHasher::new();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn line_id_equality_with_different_values() {
        let a = LineId::new();
        let b = LineId::new();
        let c = LineId::new();
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);
        // All copies of the same value should be equal
        assert_eq!(a, a);
        let a2 = a;
        assert_eq!(a, a2);
    }
}

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
        if trimmed.is_empty() { None } else { Some(Self(trimmed)) }
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
        Self(Uuid::new_v4())
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
}

//! Unit tests for `sku` — sibling test file per AGENTS.md
//! (tests must never live inside production `.rs` files; COR-33).
//!
//! Wired from `sku.rs` via `#[cfg(test)] #[path = "sku_tests.rs"]
//! mod tests;`.

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

// ── Sku serde validation ──

#[test]
fn sku_serde_rejects_empty_string() {
    let result: Result<Sku, _> = serde_json::from_str("\"\"");
    assert!(result.is_err());
}

#[test]
fn sku_serde_rejects_whitespace_only() {
    let result: Result<Sku, _> = serde_json::from_str("\"   \"");
    assert!(result.is_err());
}

#[test]
fn sku_serde_trims_whitespace() {
    let sku: Sku = serde_json::from_str("\"  COFFEE  \"").unwrap();
    assert_eq!(sku.as_str(), "COFFEE");
}

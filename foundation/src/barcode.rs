//! Barcode — a validated product barcode string.
//!
//! `Barcode` is a `#[serde(transparent)]` newtype validated to be
//! non-empty after trimming. It accepts any barcode format (EAN-13,
//! UPC-A, Code-128, etc.) as long as it is non-empty.
//!
//! # Example
//!
//! ```rust
//! use foundation::barcode::Barcode;
//!
//! let bc = Barcode::new("5901234123457").unwrap();
//! assert_eq!(bc.as_str(), "5901234123457");
//! ```

use serde::{Deserialize, Serialize};

use crate::ValidationError;

/// A validated product barcode.
///
/// Guarantees:
/// - Non-empty (after trimming)
///
/// # Serialization
///
/// Serializes as a bare string via `#[serde(transparent)]`.
///
/// ```rust
/// # use foundation::barcode::Barcode;
/// let bc = Barcode::new("4901234567890").unwrap();
/// assert_eq!(serde_json::to_string(&bc).unwrap(), "\"4901234567890\"");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Barcode(String);

impl Barcode {
    /// Construct a `Barcode`, trimming whitespace and validating non-empty.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError`] when the input is empty or whitespace-only.
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let trimmed = s.into().trim().to_owned();
        if trimmed.is_empty() {
            return Err(ValidationError {
                field: "barcode",
                message: "barcode must not be empty".into(),
            });
        }
        Ok(Self(trimmed))
    }

    /// Borrow the underlying barcode string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for Barcode {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl std::fmt::Display for Barcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ─────────────────────────────────────────────

    #[test]
    fn valid_ean13() {
        let bc = Barcode::new("5901234123457").unwrap();
        assert_eq!(bc.as_str(), "5901234123457");
    }

    #[test]
    fn valid_upc_a() {
        let bc = Barcode::new("012345678905").unwrap();
        assert_eq!(bc.as_str(), "012345678905");
    }

    #[test]
    fn valid_alphanumeric() {
        let bc = Barcode::new("ABC123XYZ").unwrap();
        assert_eq!(bc.as_str(), "ABC123XYZ");
    }

    #[test]
    fn trims_whitespace() {
        let bc = Barcode::new("  4901234567890  ").unwrap();
        assert_eq!(bc.as_str(), "4901234567890");
    }

    #[test]
    fn rejects_empty() {
        let err = Barcode::new("").unwrap_err();
        assert_eq!(err.field, "barcode");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn rejects_whitespace_only() {
        let err = Barcode::new("   ").unwrap_err();
        assert!(err.message.contains("must not be empty"));
    }

    // ── FromStr ──────────────────────────────────────────────────

    #[test]
    fn from_str_works() {
        let bc: Barcode = "5901234123457".parse().unwrap();
        assert_eq!(bc.to_string(), "5901234123457");
    }

    // ── Serde ────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let bc = Barcode::new("4901234567890").unwrap();
        let json = serde_json::to_string(&bc).unwrap();
        assert_eq!(json, "\"4901234567890\"");
        let back: Barcode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, bc);
    }

    #[test]
    fn display_and_as_str_match() {
        let bc = Barcode::new("012345678905").unwrap();
        assert_eq!(bc.as_str(), bc.to_string());
    }
}

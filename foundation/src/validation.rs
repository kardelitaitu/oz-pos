//! Shared validation utilities for OZ-POS.
//!
//! These functions provide consistent, reusable validation for common
//! constraints: non-empty strings, numeric ranges, and string lengths.
//! Every function returns `Result<(), ValidationError>` with a
//! descriptive message that includes the field name, making it easy to
//! chain with `?` in command handlers and domain logic.
//!
//! # Example
//!
//! ```rust
//! use foundation::validation::{validate_not_empty, validate_range};
//!
//! fn update_product(name: &str, price: i64) -> Result<(), foundation::ValidationError> {
//!     validate_not_empty("name", name)?;
//!     validate_range("price", price, 0, 1_000_000)?;
//!     Ok(())
//! }
//! ```

use crate::ValidationError;

/// Validate that a trimmed string is non-empty.
///
/// Returns `Err(ValidationError)` when the trimmed value has zero length.
///
/// # Example
///
/// ```
/// # use foundation::validation::validate_not_empty;
/// assert!(validate_not_empty("name", "Coffee").is_ok());
/// assert!(validate_not_empty("name", "  ").is_err());
/// assert!(validate_not_empty("name", "").is_err());
/// ```
pub fn validate_not_empty(field: &'static str, value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        Err(ValidationError {
            field,
            message: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

/// Validate that a value falls within the inclusive range `[min, max]`.
///
/// Works with any type that implements `PartialOrd + Display` (integers,
/// floats, etc.).
///
/// # Example
///
/// ```
/// # use foundation::validation::validate_range;
/// assert!(validate_range("discount", 10u8, 0u8, 100u8).is_ok());
/// assert!(validate_range("discount", 101u8, 0u8, 100u8).is_err());
/// ```
pub fn validate_range<T>(
    field: &'static str,
    value: T,
    min: T,
    max: T,
) -> Result<(), ValidationError>
where
    T: PartialOrd + std::fmt::Display,
{
    if value < min || value > max {
        Err(ValidationError {
            field,
            message: format!("{field} must be between {min} and {max}, got {value}"),
        })
    } else {
        Ok(())
    }
}

/// Validate that a trimmed string meets a minimum length.
///
/// # Example
///
/// ```
/// # use foundation::validation::validate_min_length;
/// assert!(validate_min_length("pin", "1234", 4).is_ok());
/// assert!(validate_min_length("pin", "12", 4).is_err());
/// ```
pub fn validate_min_length(
    field: &'static str,
    value: &str,
    min: usize,
) -> Result<(), ValidationError> {
    let len = value.trim().len();
    if len < min {
        Err(ValidationError {
            field,
            message: format!("{field} must be at least {min} characters (got {len})"),
        })
    } else {
        Ok(())
    }
}

/// Validate that a string does not exceed a maximum length.
///
/// # Example
///
/// ```
/// # use foundation::validation::validate_max_length;
/// assert!(validate_max_length("sku", "COFFEE", 20).is_ok());
/// assert!(validate_max_length("sku", &"X".repeat(21), 20).is_err());
/// ```
pub fn validate_max_length(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), ValidationError> {
    let len = value.len();
    if len > max {
        Err(ValidationError {
            field,
            message: format!("{field} must be at most {max} characters (got {len})"),
        })
    } else {
        Ok(())
    }
}

/// Validate that a trimmed string contains only Unicode alphanumeric
/// characters (letters and digits from any script).
///
/// Returns `Err(ValidationError)` when the trimmed value is empty or
/// contains non-alphanumeric characters (spaces, punctuation, symbols).
///
/// # Example
///
/// ```
/// # use foundation::validation::validate_alphanumeric;
/// assert!(validate_alphanumeric("username", "alice_123").is_err());  // underscore
/// assert!(validate_alphanumeric("username", "alice123").is_ok());
/// assert!(validate_alphanumeric("username", "café").is_ok());  // Unicode é
/// assert!(validate_alphanumeric("username", "  alice  ").is_ok());  // trimmed
/// ```
pub fn validate_alphanumeric(field: &'static str, value: &str) -> Result<(), ValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ValidationError {
            field,
            message: format!("{field} must not be empty"),
        });
    }
    if let Some(bad) = trimmed.chars().find(|c| !c.is_alphanumeric()) {
        return Err(ValidationError {
            field,
            message: format!("{field} must be alphanumeric (found invalid character '{bad}')"),
        });
    }
    Ok(())
}

/// Validate that a trimmed string contains only ASCII alphanumeric
/// characters (`a-z`, `A-Z`, `0-9`).
///
/// Returns `Err(ValidationError)` when the trimmed value is empty or
/// contains any non-ASCII-alphanumeric character (spaces, punctuation,
/// symbols, accented letters, etc.).
///
/// # Example
///
/// ```
/// # use foundation::validation::validate_ascii_alphanumeric;
/// assert!(validate_ascii_alphanumeric("sku", "COFFEE123").is_ok());
/// assert!(validate_ascii_alphanumeric("sku", "COFFEE-123").is_err());  // hyphen
/// assert!(validate_ascii_alphanumeric("sku", "café").is_err());  // é is not ASCII
/// ```
pub fn validate_ascii_alphanumeric(
    field: &'static str,
    value: &str,
) -> Result<(), ValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ValidationError {
            field,
            message: format!("{field} must not be empty"),
        });
    }
    if let Some(bad) = trimmed.chars().find(|c| !c.is_ascii_alphanumeric()) {
        return Err(ValidationError {
            field,
            message: format!(
                "{field} must be ASCII alphanumeric (found invalid character '{bad}')"
            ),
        });
    }
    Ok(())
}

/// Validate that a trimmed string matches a regular expression pattern.
///
/// The pattern is applied via [`Regex::is_match`], which searches for a
/// match anywhere in the string. To validate the **entire** string, use
/// `^` and `$` anchors in your pattern.
///
/// Returns `Err(ValidationError)` when the trimmed value is empty or
/// does not match the pattern.
///
/// # Example
///
/// ```
/// # use foundation::validation::validate_regex;
/// # use regex::Regex;
/// let hex_color = Regex::new(r"^#[0-9a-fA-F]{6}$").unwrap();
/// assert!(validate_regex("color", "#ff6600", &hex_color).is_ok());
/// assert!(validate_regex("color", "#GGG", &hex_color).is_err());
/// ```
pub fn validate_regex(
    field: &'static str,
    value: &str,
    pattern: &regex::Regex,
) -> Result<(), ValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ValidationError {
            field,
            message: format!("{field} must not be empty"),
        });
    }
    if !pattern.is_match(trimmed) {
        return Err(ValidationError {
            field,
            message: format!("{field} does not match the required pattern"),
        });
    }
    Ok(())
}

/// Validate that a trimmed string is non-empty and within a length range.
///
/// Equivalent to calling `validate_not_empty` then `validate_min_length`
/// and `validate_max_length`, but with a single error message when the
/// value is empty (rather than a cryptic "must be at least N characters").
///
/// # Example
///
/// ```
/// # use foundation::validation::validate_non_empty_bounded;
/// assert!(validate_non_empty_bounded("name", "Coffee", 2, 50).is_ok());
/// assert!(validate_non_empty_bounded("name", "", 2, 50).is_err());
/// assert!(validate_non_empty_bounded("name", "X", 2, 50).is_err());
/// assert!(validate_non_empty_bounded("name", &"X".repeat(51), 2, 50).is_err());
/// ```
pub fn validate_non_empty_bounded(
    field: &'static str,
    value: &str,
    min: usize,
    max: usize,
) -> Result<(), ValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ValidationError {
            field,
            message: format!("{field} must not be empty"),
        });
    }
    let len = trimmed.len();
    if len < min {
        return Err(ValidationError {
            field,
            message: format!("{field} must be at least {min} characters (got {len})"),
        });
    }
    if len > max {
        return Err(ValidationError {
            field,
            message: format!("{field} must be at most {max} characters (got {len})"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_not_empty ───────────────────────────────────────

    #[test]
    fn not_empty_accepts_non_empty() {
        assert!(validate_not_empty("name", "Coffee").is_ok());
    }

    #[test]
    fn not_empty_rejects_empty() {
        let err = validate_not_empty("name", "").unwrap_err();
        assert_eq!(err.field, "name");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn not_empty_rejects_whitespace() {
        let err = validate_not_empty("sku", "   ").unwrap_err();
        assert_eq!(err.field, "sku");
    }

    #[test]
    fn not_empty_trims_input() {
        assert!(validate_not_empty("name", "  Coffee  ").is_ok());
    }

    // ── validate_range ───────────────────────────────────────────

    #[test]
    fn range_accepts_inclusive_bounds() {
        assert!(validate_range("x", 0u8, 0u8, 100u8).is_ok());
        assert!(validate_range("x", 100u8, 0u8, 100u8).is_ok());
    }

    #[test]
    fn range_rejects_below_min() {
        let err = validate_range("discount", -1i64, 0i64, 100i64).unwrap_err();
        assert_eq!(err.field, "discount");
        assert!(err.message.contains("must be between"));
    }

    #[test]
    fn range_rejects_above_max() {
        let err = validate_range("discount", 101u8, 0u8, 100u8).unwrap_err();
        assert_eq!(err.field, "discount");
    }

    #[test]
    fn range_works_with_different_types() {
        assert!(validate_range("price", 500, 0, 10_000).is_ok());
        let err = validate_range("price", -1, 0, 10_000).unwrap_err();
        assert_eq!(err.field, "price");
    }

    // ── validate_min_length ──────────────────────────────────────

    #[test]
    fn min_length_accepts_equal() {
        assert!(validate_min_length("pin", "1234", 4).is_ok());
    }

    #[test]
    fn min_length_accepts_longer() {
        assert!(validate_min_length("pin", "123456", 4).is_ok());
    }

    #[test]
    fn min_length_rejects_shorter() {
        let err = validate_min_length("pin", "12", 4).unwrap_err();
        assert_eq!(err.field, "pin");
        assert!(err.message.contains("at least 4"));
    }

    #[test]
    fn min_length_trims_before_counting() {
        assert!(validate_min_length("pin", "  1234  ", 4).is_ok());
    }

    // ── validate_max_length ──────────────────────────────────────

    #[test]
    fn max_length_accepts_equal() {
        assert!(validate_max_length("sku", "ABCD", 4).is_ok());
    }

    #[test]
    fn max_length_accepts_shorter() {
        assert!(validate_max_length("sku", "AB", 4).is_ok());
    }

    #[test]
    fn max_length_rejects_longer() {
        let err = validate_max_length("sku", "ABCDE", 4).unwrap_err();
        assert_eq!(err.field, "sku");
        assert!(err.message.contains("at most 4"));
    }

    #[test]
    fn max_length_does_not_trim() {
        // validate_max_length uses the original length, not trimmed.
        let err = validate_max_length("sku", "    ", 2).unwrap_err();
        assert_eq!(err.field, "sku");
    }

    // ── validate_alphanumeric ───────────────────────────────────

    #[test]
    fn alphanumeric_accepts_plain_text() {
        assert!(validate_alphanumeric("username", "alice").is_ok());
    }

    #[test]
    fn alphanumeric_accepts_digits() {
        assert!(validate_alphanumeric("sku", "COFFEE123").is_ok());
    }

    #[test]
    fn alphanumeric_accepts_unicode() {
        assert!(validate_alphanumeric("name", "café").is_ok());
        assert!(validate_alphanumeric("name", "用户").is_ok());
    }

    #[test]
    fn alphanumeric_rejects_empty() {
        let err = validate_alphanumeric("user", "").unwrap_err();
        assert_eq!(err.field, "user");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn alphanumeric_rejects_whitespace() {
        let err = validate_alphanumeric("user", "   ").unwrap_err();
        assert_eq!(err.field, "user");
    }

    #[test]
    fn alphanumeric_rejects_punctuation() {
        let err = validate_alphanumeric("user", "alice!").unwrap_err();
        assert!(err.message.contains("invalid character"));
    }

    #[test]
    fn alphanumeric_rejects_spaces() {
        let err = validate_alphanumeric("user", "alice bob").unwrap_err();
        assert!(err.message.contains("invalid character"));
    }

    #[test]
    fn alphanumeric_rejects_underscore() {
        let err = validate_alphanumeric("user", "alice_bob").unwrap_err();
        assert!(err.message.contains("invalid character"));
    }

    #[test]
    fn alphanumeric_trims_before_check() {
        assert!(validate_alphanumeric("user", "  alice  ").is_ok());
    }

    // ── validate_ascii_alphanumeric ──────────────────────────────

    #[test]
    fn ascii_alphanumeric_accepts_ascii_letters() {
        assert!(validate_ascii_alphanumeric("sku", "COFFEE").is_ok());
    }

    #[test]
    fn ascii_alphanumeric_accepts_digits() {
        assert!(validate_ascii_alphanumeric("sku", "BAGEL99").is_ok());
    }

    #[test]
    fn ascii_alphanumeric_rejects_empty() {
        let err = validate_ascii_alphanumeric("user", "").unwrap_err();
        assert_eq!(err.field, "user");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn ascii_alphanumeric_rejects_whitespace() {
        let err = validate_ascii_alphanumeric("sku", "   ").unwrap_err();
        assert_eq!(err.field, "sku");
    }

    #[test]
    fn ascii_alphanumeric_rejects_punctuation() {
        let err = validate_ascii_alphanumeric("sku", "COFFEE-1").unwrap_err();
        assert!(err.message.contains("invalid character"));
    }

    #[test]
    fn ascii_alphanumeric_rejects_unicode() {
        let err = validate_ascii_alphanumeric("name", "café").unwrap_err();
        assert!(err.message.contains("invalid character"));
    }

    #[test]
    fn ascii_alphanumeric_trims_before_check() {
        assert!(validate_ascii_alphanumeric("sku", "  COFFEE  ").is_ok());
    }

    #[test]
    fn ascii_alphanumeric_uppercase_lowercase() {
        assert!(validate_ascii_alphanumeric("user", "Alice99").is_ok());
    }

    // ── validate_regex ───────────────────────────────────────────

    #[test]
    fn regex_matches_pattern() {
        let hex = regex::Regex::new(r"^#[0-9a-fA-F]{6}$").unwrap();
        assert!(validate_regex("color", "#ff6600", &hex).is_ok());
    }

    #[test]
    fn regex_rejects_non_match() {
        let hex = regex::Regex::new(r"^#[0-9a-fA-F]{6}$").unwrap();
        let err = validate_regex("color", "#GGG", &hex).unwrap_err();
        assert_eq!(err.field, "color");
        assert!(err.message.contains("does not match"));
    }

    #[test]
    fn regex_rejects_empty() {
        let any = regex::Regex::new(r".*").unwrap();
        let err = validate_regex("x", "", &any).unwrap_err();
        assert_eq!(err.field, "x");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn regex_anchored_pattern_requires_full_match() {
        let digits = regex::Regex::new(r"^\d+$").unwrap();
        assert!(validate_regex("num", "123", &digits).is_ok());
        assert!(validate_regex("num", "12a3", &digits).is_err());
    }

    #[test]
    fn regex_unanchored_pattern_matches_substring() {
        // Without anchors, is_match finds digits anywhere in the string.
        let has_digit = regex::Regex::new(r"\d").unwrap();
        assert!(validate_regex("x", "abc123", &has_digit).is_ok());
        assert!(validate_regex("x", "abc", &has_digit).is_err());
    }

    #[test]
    fn regex_trims_before_matching() {
        let hex = regex::Regex::new(r"^#[0-9a-fA-F]{6}$").unwrap();
        assert!(validate_regex("color", "  #ff6600  ", &hex).is_ok());
    }

    // ── validate_non_empty_bounded ───────────────────────────────

    #[test]
    fn bounded_accepts_valid() {
        assert!(validate_non_empty_bounded("name", "Coffee", 2, 50).is_ok());
    }

    #[test]
    fn bounded_rejects_empty() {
        let err = validate_non_empty_bounded("name", "", 2, 50).unwrap_err();
        assert_eq!(err.field, "name");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn bounded_rejects_too_short() {
        let err = validate_non_empty_bounded("name", "X", 2, 50).unwrap_err();
        assert_eq!(err.field, "name");
        assert!(err.message.contains("at least 2"));
    }

    #[test]
    fn bounded_rejects_too_long() {
        let err = validate_non_empty_bounded("name", &"X".repeat(51), 2, 50).unwrap_err();
        assert_eq!(err.field, "name");
        assert!(err.message.contains("at most 50"));
    }

    #[test]
    fn bounded_trims_before_checks() {
        assert!(validate_non_empty_bounded("name", "  Coffee  ", 2, 50).is_ok());
    }

    #[test]
    fn bounded_with_zero_min_accepts_one_char() {
        // When min = 0, a single character is valid (after trimming).
        assert!(validate_non_empty_bounded("flag", "X", 0, 10).is_ok());
    }

    // ── Error traits ─────────────────────────────────────────────

    #[test]
    fn returned_error_implements_std_error() {
        let err = validate_not_empty("x", "").unwrap_err();
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn returned_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ValidationError>();
    }
}

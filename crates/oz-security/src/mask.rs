//! PCI-DSS helpers for cardholder data handling.
//!
//! This module provides functions for masking Primary Account Numbers
//! (PAN) in compliance with PCI-DSS requirement 3.3 — display only the
//! first six and last four digits.
//!
//! # Example
//!
//! ```
//! use oz_security::mask::mask_pan;
//!
//! assert_eq!(mask_pan("4111111111111111"), "411111******1111");
//! assert_eq!(mask_pan("4111-1111-1111-1111"), "411111******1111");
//! assert_eq!(mask_pan("411111111111"), "411111****1111");
//! assert_eq!(mask_pan("123"), "****");
//! ```

/// Mask a PAN (Primary Account Number) for PCI-DSS compliant display.
///
/// Shows only the first six and last four digits. All other digits are
/// replaced with `*`. Non-digit characters (spaces, hyphens) are
/// stripped before masking.
///
/// # Panics
///
/// Does not panic — returns a masked string even for short inputs.
///
/// # PCI-DSS Requirement 3.3
///
/// > Render PAN unreadable anywhere it is stored by using any of the
/// > following approaches: truncation, masking, hashing, or encryption.
/// > Mask PAN when displayed such that only the first six and last four
/// > digits are visible.
pub fn mask_pan(pan: &str) -> String {
    // Strip non-digit characters.
    let digits: String = pan.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.len() <= 6 {
        // Too short for meaningful masking — mask all.
        return "****".to_string();
    }

    let first_six = &digits[..6];
    let last_four = &digits[digits.len() - 4..];
    let masked_len = digits.len().saturating_sub(10);
    let masked = "*".repeat(masked_len.max(4));

    format!("{first_six}{masked}{last_four}")
}

/// Check whether a string contains a valid PAN format (at least 13
/// digits, at most 19 digits, passes Luhn check).
///
/// This is a basic validation — it does NOT verify that the PAN is
/// actually issued by a real financial institution.
///
/// # Example
///
/// ```
/// use oz_security::mask::is_valid_pan;
///
/// // Visa test number
/// assert!(is_valid_pan("4111111111111111"));
/// // Invalid Luhn
/// assert!(!is_valid_pan("4111111111111112"));
/// ```
pub fn is_valid_pan(pan: &str) -> bool {
    let digits: String = pan.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }

    // Luhn check.
    let mut sum = 0u32;
    let mut double = false;

    for c in digits.chars().rev() {
        let digit = c.to_digit(10).unwrap_or(0);
        if double {
            let doubled = digit * 2;
            sum += if doubled > 9 { doubled - 9 } else { doubled };
        } else {
            sum += digit;
        }
        double = !double;
    }

    sum.is_multiple_of(10)
}

/// Mask a cardholder name — show only the first and last letter of
/// each name part.
///
/// # Example
///
/// ```
/// use oz_security::mask::mask_name;
///
/// assert_eq!(mask_name("John A. Doe"), "J**n A. D*e");
/// ```
pub fn mask_name(name: &str) -> String {
    name.split_whitespace()
        .map(|part| {
            if part.len() <= 2 {
                part.to_string()
            } else {
                let first = part.chars().next().unwrap_or('*');
                let last = part.chars().last().unwrap_or('*');
                let masked_len = part.len().saturating_sub(2);
                let masked = "*".repeat(masked_len);
                format!("{first}{masked}{last}")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Mask a card verification value (CVV/CVC) — always returns `"***"`.
///
/// PCI-DSS prohibits storing CVV/CVC after authorization.
/// This function is a visual indicator that the value should NOT
/// be stored or displayed.
///
/// # Example
///
/// ```
/// use oz_security::mask::mask_cvv;
///
/// assert_eq!(mask_cvv("123"), "***");
/// ```
pub fn mask_cvv(_cvv: &str) -> String {
    "***".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_pan_standard() {
        assert_eq!(mask_pan("4111111111111111"), "411111******1111");
    }

    #[test]
    fn mask_pan_with_dashes() {
        assert_eq!(mask_pan("4111-1111-1111-1111"), "411111******1111");
    }

    #[test]
    fn mask_pan_with_spaces() {
        assert_eq!(mask_pan("4111 1111 1111 1111"), "411111******1111");
    }

    #[test]
    fn mask_pan_short() {
        assert_eq!(mask_pan("123"), "****");
    }

    #[test]
    fn mask_pan_exactly_10_digits() {
        assert_eq!(mask_pan("1234567890"), "123456****7890");
    }

    #[test]
    fn mask_pan_amex() {
        // Amex is 15 digits: 378282246310005
        assert_eq!(mask_pan("378282246310005"), "378282*****0005");
    }

    #[test]
    fn mask_pan_empty() {
        assert_eq!(mask_pan(""), "****");
    }

    #[test]
    fn is_valid_pan_visa() {
        assert!(is_valid_pan("4111111111111111"));
    }

    #[test]
    fn is_valid_pan_mastercard() {
        assert!(is_valid_pan("5555555555554444"));
    }

    #[test]
    fn is_valid_pan_amex() {
        assert!(is_valid_pan("378282246310005"));
    }

    #[test]
    fn is_valid_pan_invalid_luhn() {
        assert!(!is_valid_pan("4111111111111112"));
    }

    #[test]
    fn is_valid_pan_too_short() {
        assert!(!is_valid_pan("411111111111"));
    }

    #[test]
    fn is_valid_pan_too_long() {
        assert!(!is_valid_pan("411111111111111111111"));
    }

    #[test]
    fn is_valid_pan_with_formatting() {
        assert!(is_valid_pan("4111-1111-1111-1111"));
    }

    #[test]
    fn mask_name_standard() {
        // "John" → J**n, "A." → A. (len <= 2), "Doe" → D*e
        assert_eq!(mask_name("John A. Doe"), "J**n A. D*e");
    }

    #[test]
    fn mask_name_short() {
        assert_eq!(mask_name("Al"), "Al");
    }

    #[test]
    fn mask_name_single() {
        assert_eq!(mask_name("John"), "J**n");
    }

    #[test]
    fn mask_cvv_always_stars() {
        assert_eq!(mask_cvv("123"), "***");
        assert_eq!(mask_cvv(""), "***");
        assert_eq!(mask_cvv("9999"), "***");
    }
}

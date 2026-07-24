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

    // PCI-DSS 3.3: show first 6 + last 4 digits. But when the PAN is
    // ≤ 10 digits, first_six and last_four overlap (or cover the entire
    // PAN), so showing both exposes the full PAN. In that case, show
    // only the last 4 digits (the more sensitive part to hide is the
    // beginning, per PCI-DSS truncation guidance).
    if digits.len() <= 10 {
        let last_four = &digits[digits.len() - 4..];
        let masked_len = digits.len() - 4;
        let masked = "*".repeat(masked_len.max(4));
        return format!("{masked}{last_four}");
    }

    let first_six = &digits[..6];
    let last_four = &digits[digits.len() - 4..];
    let masked_len = digits.len() - 10;
    let masked = "*".repeat(masked_len);

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
        // 10-digit PAN: first_six + last_four would cover the entire
        // PAN, so only last 4 is shown (PCI-DSS 3.3 compliant).
        assert_eq!(mask_pan("1234567890"), "******7890");
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

    // ── PCI-DSS 3.3: mask_pan must not expose the full PAN (Bug #16) ────
    //
    // PCI-DSS Requirement 3.3: "Mask PAN when displayed such that only
    // the first six and last four digits are visible."
    //
    // For PANs of length 7-10, the current implementation shows BOTH
    // the first 6 and last 4 digits. When the PAN is <= 10 digits,
    // first_six + last_four covers (or exceeds) the entire PAN — so
    // the full PAN is reconstructable from the "masked" output.
    // The `.max(4)` on masked_len adds 4 stars, giving the *illusion*
    // of masking while exposing the full number. This is a PCI-DSS
    // violation and a security bug.

    #[test]
    fn mask_pan_10_digits_does_not_expose_full_pan() {
        // A 10-digit "PAN": first_six="123456", last_four="7890".
        // Together they cover ALL 10 digits — the full PAN is
        // reconstructable by concatenating the visible first 6 and
        // last 4. PCI-DSS 3.3 requires that the PAN NOT be fully
        // recoverable from the masked display.
        let pan = "1234567890";
        let masked = mask_pan(pan);
        // Extract all digit groups from the masked output (split on '*').
        let visible_digits: String = masked
            .split('*')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .concat();
        // The visible digits (first 6 + last 4 concatenated) must NOT
        // reconstruct the full original PAN.
        assert_ne!(
            visible_digits, pan,
            "mask_pan must not expose enough digits to reconstruct the \
             full 10-digit PAN — got '{masked}' (visible digits: '{visible_digits}' \
             reconstruct the full PAN '{pan}')"
        );
    }

    #[test]
    fn mask_pan_7_digits_does_not_leak_more_than_4() {
        // A 7-digit "PAN": first_six="123456", last_four="4567".
        // These OVERLAP (digits 4-6 appear in both), so the masked
        // output exposes all 7 digits — more than the 4 PCI-DSS allows
        // for the "last four" display of a short PAN. The fix: when
        // first_six and last_four overlap (PAN <= 10 digits), don't
        // show first_six (show only last 4 with the rest masked).
        let pan = "1234567";
        let masked = mask_pan(pan);
        // Extract all digit groups from the masked output.
        let visible_digits: String = masked
            .split('*')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .concat();
        // For a 7-digit PAN, at most 4 digits should be visible (last 4).
        assert!(
            visible_digits.len() <= 4,
            "mask_pan must show at most 4 digits for a 7-digit PAN (PCI-DSS \
             allows first 6 + last 4 only when they don't overlap) — got \
             '{masked}' (visible digits: '{visible_digits}', {} visible)",
            visible_digits.len()
        );
    }
}

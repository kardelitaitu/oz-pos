//! Payment processor abstraction for OZ-POS.
//!
//! `oz-payment` provides a single trait, [`PaymentProcessor`], with
//! vendor-specific implementations for Stripe, Square, and EMV
//! terminals. The cashier's flow uses the trait; switching processors
//! is a config change, not a code change.
//!
//! # Lifecycle
//!
//! 1. Build a [`PaymentRequest`](types::PaymentRequest)
//! 2. Call [`authorize`](PaymentProcessor::authorize) to hold funds
//! 3. Call [`capture`](PaymentProcessor::capture) to complete
//! 4. Optionally [`refund`](PaymentProcessor::refund) or [`void`](PaymentProcessor::void)
//!
//! For simple flows [`sale`](PaymentProcessor::sale) combines step 2 + 3.
//!
//! # Testing
//!
//! Use [`MockPaymentProcessor`](drivers::mock::MockPaymentProcessor) in
//! unit tests. It tracks call counts and can simulate declines and
//! timeouts.
//!
//! ```
//! use oz_payment::{PaymentProcessor, drivers::mock::MockPaymentProcessor};
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod drivers;
pub mod error;
pub mod processor;
pub mod types;

pub use error::PaymentError;
pub use processor::PaymentProcessor;
pub use types::{PaymentMethod, PaymentReceipt, PaymentRequest, PaymentResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declined_display() {
        let err = PaymentError::Declined("insufficient funds".into());
        assert_eq!(
            err.to_string(),
            "authorization declined: insufficient funds"
        );
    }

    #[test]
    fn timeout_display() {
        let err = PaymentError::Timeout(5000);
        assert_eq!(err.to_string(), "processor timed out after 5000 ms");
    }

    #[test]
    fn network_error_display() {
        let err = PaymentError::Network("connection refused".into());
        assert_eq!(err.to_string(), "network error: connection refused");
    }

    #[test]
    fn invalid_response_display() {
        let err = PaymentError::InvalidResponse("missing field".into());
        assert_eq!(err.to_string(), "invalid response: missing field");
    }

    #[test]
    fn error_is_debug() {
        let err = PaymentError::Declined("test".into());
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn payment_method_label_cash() {
        assert_eq!(PaymentMethod::Cash.label(), "Cash");
    }

    #[test]
    fn payment_method_label_card() {
        assert_eq!(PaymentMethod::Card.label(), "Card");
    }
}

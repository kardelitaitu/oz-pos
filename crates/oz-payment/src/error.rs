//! Error type for `oz-payment`.

use thiserror::Error;

/// Errors that can originate in a payment-processor call.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PaymentError {
    /// The processor rejected the authorization request.
    #[error("authorization declined: {0}")]
    Declined(String),

    /// The processor timed out before responding.
    #[error("processor timed out after {0} ms")]
    Timeout(u32),

    /// A network-level error prevented the call from completing.
    #[error("network error: {0}")]
    Network(String),

    /// The processor's API returned an unexpected response shape.
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

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
    fn network_display() {
        let err = PaymentError::Network("connection refused".into());
        assert_eq!(err.to_string(), "network error: connection refused");
    }

    #[test]
    fn invalid_response_display() {
        let err = PaymentError::InvalidResponse("missing field 'auth_code'".into());
        assert_eq!(
            err.to_string(),
            "invalid response: missing field 'auth_code'"
        );
    }

    #[test]
    fn debug_output() {
        let err = PaymentError::Declined("test".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("Declined"));
    }

    #[test]
    fn implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(PaymentError::Network("test".into()));
        let _ = err.to_string();
    }

    #[test]
    fn is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PaymentError>();
    }

    #[test]
    fn variants_are_distinct() {
        let a = format!("{:?}", PaymentError::Declined("x".into()));
        let b = format!("{:?}", PaymentError::Timeout(1));
        let c = format!("{:?}", PaymentError::Network("x".into()));
        let d = format!("{:?}", PaymentError::InvalidResponse("x".into()));
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_ne!(b, c);
        assert_ne!(b, d);
        assert_ne!(c, d);
    }
}

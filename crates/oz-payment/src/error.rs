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

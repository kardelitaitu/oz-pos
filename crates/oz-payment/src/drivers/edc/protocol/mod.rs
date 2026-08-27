//! EDC terminal protocol codec — PLANNED (stubs).
//!
//! Real EDC terminals (Ingenico, Verifone, PAX) do not speak REST. They
//! communicate over serial/USB/Bluetooth using vendor-specific binary
//! framing on top of ISO 8583. This module provides the [`ProtocolCodec`]
//! trait that isolates the wire format from the transport and the
//! terminal state machine.
//!
//! Vendors:
//!
//! * [`IngenicoCodec`] — Telium / Telium 2 protocol (iPP320, iPP350, Desk 3500).
//! * [`VerifoneCodec`] — Verifone SSL / Verix protocol (VX520, VX680, P400).
//! * [`PaxCodec`] — PAX DCC protocol (S80, S300, S920, A920).

pub mod ingenico;
pub mod pax;
pub mod verifone;

use crate::error::PaymentError;

/// A vendor-specific message that has been decoded from the wire.
#[derive(Debug, Clone)]
pub enum ProtocolMessage {
    /// Ready to accept a new transaction.
    Ready,
    /// Transaction was authorised.
    Authorised {
        /// Unique transaction identifier assigned by the terminal.
        transaction_id: String,
        /// Authorization code from the acquirer (e.g. "001234").
        auth_code: String,
        /// Card scheme name (e.g. "VISA", "MASTERCARD"), if reported.
        card_scheme: Option<String>,
        /// Last 4 digits of the card number, if reported.
        card_last4: Option<String>,
    },
    /// Transaction was declined.
    Declined {
        /// Human-readable reason for the decline (e.g. "Insufficient funds").
        reason: Option<String>,
    },
    /// Terminal error / hardware fault.
    Error {
        /// Vendor-specific error code.
        code: u32,
        /// Human-readable error description.
        message: String,
    },
    /// Raw response that the caller's state machine interprets.
    Raw(Vec<u8>),
}

/// Encodes and decodes a vendor-specific EDC protocol.
///
/// **PLANNED:** every method returns [`PaymentError::Unsupported`] until
/// the real vendor protocol is implemented.
pub trait ProtocolCodec: Send + Sync {
    /// The vendor name (e.g. `"ingenico"`, `"verifone"`, `"pax"`).
    fn vendor(&self) -> &'static str;

    /// Encode a *sale* command (amount + optional invoice reference) into
    /// the vendor's wire format for transmission.
    fn encode_sale(
        &self,
        amount: foundation::Money,
        reference: &str,
    ) -> Result<Vec<u8>, PaymentError>;

    /// Encode a *refund* command.
    fn encode_refund(
        &self,
        amount: foundation::Money,
        transaction_id: &str,
    ) -> Result<Vec<u8>, PaymentError>;

    /// Encode a *void / cancel* command.
    fn encode_void(&self, transaction_id: &str) -> Result<Vec<u8>, PaymentError>;

    /// Decode a response from the terminal into a structured message.
    fn decode(&self, wire_data: &[u8]) -> Result<ProtocolMessage, PaymentError>;
}

/// Shared helper: build a "not implemented" error for a vendor method.
///
/// Used by the stub codecs so every unimplemented method returns a
/// consistent error message.
pub fn stub_error(vendor: &str, method: &str) -> PaymentError {
    PaymentError::Unsupported(format!(
        "{vendor} protocol codec `{method}` — PLANNED, not implemented yet"
    ))
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;

//! Ingenico Telium / Telium 2 protocol codec — PLANNED (stub).
//!
//! Implements [`ProtocolCodec`] for Ingenico terminals (iPP320, iPP350,
//! Desk 3500, Lane 5000). Telium 2 uses a binary framing over serial
//! (RS-232) with a proprietary command set wrapped around ISO 8583
//! messages.

use super::{ProtocolCodec, ProtocolMessage};
use crate::error::PaymentError;

/// Ingenico-specific protocol codec.
///
/// **STUB:** every method returns [`PaymentError::Unsupported`] until
/// the real Telium/Telium2 protocol handler is implemented.
pub struct IngenicoCodec;

impl ProtocolCodec for IngenicoCodec {
    fn vendor(&self) -> &'static str {
        "ingenico"
    }

    fn encode_sale(
        &self,
        _amount: foundation::Money,
        _reference: &str,
    ) -> Result<Vec<u8>, PaymentError> {
        Err(super::stub_error("ingenico", "encode_sale"))
    }

    fn encode_refund(
        &self,
        _amount: foundation::Money,
        _transaction_id: &str,
    ) -> Result<Vec<u8>, PaymentError> {
        Err(super::stub_error("ingenico", "encode_refund"))
    }

    fn encode_void(&self, _transaction_id: &str) -> Result<Vec<u8>, PaymentError> {
        Err(super::stub_error("ingenico", "encode_void"))
    }

    fn decode(&self, _wire_data: &[u8]) -> Result<ProtocolMessage, PaymentError> {
        Err(super::stub_error("ingenico", "decode"))
    }
}

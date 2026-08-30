/*
last audited 25-07-26 by RSA-Agent
crate: oz-payment | status: SAFE | lint: CLEAN
findings: PLANNED stub — all codec methods fail closed via stub_error
next: none until Verix handler | perf: N/A
*/
//! Verifone SSL / Verix protocol codec — PLANNED (stub).
//!
//! Implements [`ProtocolCodec`] for Verifone terminals (VX520, VX680,
//! P400, VX 820). Verifone uses a custom SSL (Secure Socket Layer —
//! unrelated to TLS) protocol over serial or TCP, with Verix as the
//! application runtime.

use super::{ProtocolCodec, ProtocolMessage};
use crate::error::PaymentError;

/// Verifone-specific protocol codec.
///
/// **STUB:** every method returns [`PaymentError::Unsupported`] until
/// the real Verifone SSL/Verix protocol handler is implemented.
pub struct VerifoneCodec;

impl ProtocolCodec for VerifoneCodec {
    fn vendor(&self) -> &'static str {
        "verifone"
    }

    fn encode_sale(
        &self,
        _amount: foundation::Money,
        _reference: &str,
    ) -> Result<Vec<u8>, PaymentError> {
        Err(super::stub_error("verifone", "encode_sale"))
    }

    fn encode_refund(
        &self,
        _amount: foundation::Money,
        _transaction_id: &str,
    ) -> Result<Vec<u8>, PaymentError> {
        Err(super::stub_error("verifone", "encode_refund"))
    }

    fn encode_void(&self, _transaction_id: &str) -> Result<Vec<u8>, PaymentError> {
        Err(super::stub_error("verifone", "encode_void"))
    }

    fn decode(&self, _wire_data: &[u8]) -> Result<ProtocolMessage, PaymentError> {
        Err(super::stub_error("verifone", "decode"))
    }
}

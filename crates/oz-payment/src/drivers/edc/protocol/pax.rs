/*
last audited 25-07-26 by RSA-Agent
crate: oz-payment | status: SAFE | lint: CLEAN
findings: PLANNED stub — all codec methods fail closed via stub_error
next: none until PAX DCC handler | perf: N/A
*/
//! PAX DCC protocol codec — PLANNED (stub).
//!
//! Implements [`ProtocolCodec`] for PAX terminals (S80, S300, S920,
//! A920). PAX uses the DCC (Device Configuration and Communication)
//! protocol over serial, USB, or Bluetooth — a packet-based protocol
//! with a fixed header, command/response structure, and CRC.

use super::{ProtocolCodec, ProtocolMessage};
use crate::error::PaymentError;

/// PAX-specific protocol codec.
///
/// **STUB:** every method returns [`PaymentError::Unsupported`] until
/// the real PAX DCC protocol handler is implemented.
pub struct PaxCodec;

impl ProtocolCodec for PaxCodec {
    fn vendor(&self) -> &'static str {
        "pax"
    }

    fn encode_sale(
        &self,
        _amount: foundation::Money,
        _reference: &str,
    ) -> Result<Vec<u8>, PaymentError> {
        Err(super::stub_error("pax", "encode_sale"))
    }

    fn encode_refund(
        &self,
        _amount: foundation::Money,
        _transaction_id: &str,
    ) -> Result<Vec<u8>, PaymentError> {
        Err(super::stub_error("pax", "encode_refund"))
    }

    fn encode_void(&self, _transaction_id: &str) -> Result<Vec<u8>, PaymentError> {
        Err(super::stub_error("pax", "encode_void"))
    }

    fn decode(&self, _wire_data: &[u8]) -> Result<ProtocolMessage, PaymentError> {
        Err(super::stub_error("pax", "decode"))
    }
}

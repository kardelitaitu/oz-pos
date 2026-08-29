//! EDC protocol codecs — STUB test placeholder.
//!
//! Tests will be added when the real vendor protocols are implemented.
//! The vendor() accessor is functional now.

use crate::drivers::edc::protocol::ProtocolCodec;
use crate::drivers::edc::{IngenicoCodec, PaxCodec, VerifoneCodec};

#[test]
fn vendor_names_are_stable() {
    assert_eq!(IngenicoCodec.vendor(), "ingenico");
    assert_eq!(VerifoneCodec.vendor(), "verifone");
    assert_eq!(PaxCodec.vendor(), "pax");
}

#[test]
fn stubs_return_unsupported() {
    let codec = PaxCodec;
    let result = codec.encode_sale(
        foundation::Money::from_major(10, "USD".parse().unwrap()).unwrap(),
        "ref-1",
    );
    assert!(
        matches!(result, Err(crate::PaymentError::Unsupported(_))),
        "expected Unsupported error"
    );
}

//! Encryption, secrets, and PCI-DSS helpers for OZ-POS.
//!
//! `oz-security` is responsible for at-rest encryption, secret
//! management, key rotation, and the small set of PCI-DSS-related
//! utilities the cashier flow needs (masked PAN display, audit
//! logging, etc.).
//!
//! This crate is a scaffold. Production code lands in a follow-up
//! once the `oz-payment` crate defines the secret shape it needs.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;

pub use error::SecurityError;

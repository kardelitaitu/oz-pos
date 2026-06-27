//! Payment processor abstraction for OZ-POS.
//!
//! `oz-payment` provides a single trait, [`PaymentProcessor`], with
//! vendor-specific implementations for Stripe, Square, and EMV
//! terminals. The cashier's flow uses the trait; switching processors
//! is a config change, not a code change.
//!
//! This crate is a scaffold — the trait and adapters land in a
//! follow-up alongside the `oz-hal` `PaymentTerminal` driver.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;

pub use error::PaymentError;

//! Hardware drivers.
//!
//! Every real driver implements one of the traits in `crate::traits`.
//! Mocks live in `mock.rs` and are compiled unconditionally so tests
//! never need a `mock` feature flag.

pub mod mock;
pub mod serial_scanner;
pub mod usb_printer;
pub mod usb_scanner;

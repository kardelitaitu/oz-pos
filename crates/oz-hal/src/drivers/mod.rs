//! Hardware drivers.
//!
//! Every real driver implements one of the traits in `crate::traits`.
//! Mocks live in `mock.rs` and are compiled unconditionally so tests
//! never need a `mock` feature flag.

pub mod bt_printer;
pub mod escpos;
pub mod mock;
pub mod receipt;
pub mod serial_scanner;
pub mod tcp_printer;
pub mod usb_printer;
pub mod usb_scanner;

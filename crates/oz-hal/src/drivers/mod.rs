//! Hardware drivers.
//!
//! Every real driver implements one of the traits in `crate::traits`.
//! Mocks live in `mock.rs` and are compiled unconditionally so tests
//! never need a `mock` feature flag.

/// Bluetooth receipt printer driver.
pub mod bt_printer;
/// Bluetooth barcode scanner driver.
pub mod bt_scanner;
/// Cash drawer driver (serial / USB).
pub mod drawer;
/// ESC/POS command builder for receipt printers.
pub mod escpos;
/// Programmable mock implementations for all HAL traits.
pub mod mock;
/// Generic receipt printer abstraction.
pub mod receipt;
/// Weight scale driver.
pub mod scale;
/// Serial-attached customer display driver.
pub mod serial_display;
/// Serial-attached barcode scanner driver.
pub mod serial_scanner;
/// TCP/IP network receipt printer driver.
pub mod tcp_printer;
/// USB receipt printer driver.
pub mod usb_printer;
/// USB barcode scanner driver.
pub mod usb_scanner;

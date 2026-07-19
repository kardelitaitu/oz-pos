/*
last audited 19-07-26 by RSA-Agent
crate: oz-hal | status: SAFE | lint: CLEAN
findings: No actual unsafe blocks present. #![allow(unsafe_code)] is forward-looking for planned
  FFI drivers (barcode scanners, receipt printers). All traits have programmable mocks (drivers::mock).
  13 unit tests pass. DriverRegistry provides safe abstraction over device enumeration.
next: Add SAFETY comments when real FFI drivers are implemented | perf: Mock drivers are zero-alloc.
*/
#![allow(unsafe_code)]
#![warn(missing_docs)]

//! Hardware Abstraction Layer for OZ-POS.
//!
//! `oz-hal` is the seam between business logic and physical devices:
//! barcode scanners, receipt printers, cash drawers, NFC readers, and
//! payment terminals. Business code only ever sees the trait
//! ([`BarcodeScanner`], [`ReceiptPrinter`], [`CashDrawer`]) — it never
//! imports a specific driver.
//!
//! Every trait has a programmable mock in [`drivers::mock`]. Tests use
//! the mocks; production code uses real drivers registered through
//! [`DriverRegistry`] at startup.

pub mod drivers;
pub mod error;
pub mod registry;
pub mod traits;
/// USB transport helpers for device enumeration.
pub mod transport;
pub mod types;

pub use drivers::scale::WeightReading;
pub use drivers::scale::WeightScale;
pub use error::{HalError, HalErrorKind};
pub use registry::DriverRegistry;
pub use traits::barcode::BarcodeScanner;
pub use traits::cash_drawer::CashDrawer;
pub use traits::customer_display::CustomerDisplay;
pub use traits::customer_display::DisplayContent;
pub use traits::printer::ReceiptPrinter;
pub use types::{Barcode, BarcodeSymbology, DeviceInfo};

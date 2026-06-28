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

#![allow(unsafe_code)]
// HAL drivers may need unsafe for FFI
// Scaffold: a number of accessors and field docs are still TODO.
// The full doc pass is tracked as a followup in CHANGELOG.md
// "Known limitations"; for now allow the warnings so the scaffold
// compiles under `clippy -- -D warnings`.
#![allow(missing_docs)]

pub mod drivers;
pub mod error;
pub mod registry;
pub mod traits;
pub mod transport;
pub mod types;

pub use error::HalError;
pub use registry::DriverRegistry;
pub use traits::barcode::BarcodeScanner;
pub use traits::cash_drawer::CashDrawer;
pub use traits::printer::ReceiptPrinter;
pub use types::{Barcode, BarcodeSymbology, DeviceInfo};

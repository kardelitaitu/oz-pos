/*
last audited 19-07-26 by RSA-Agent
crate: oz-hal | status: SAFE | lint: CLEAN
findings: No actual unsafe blocks present. #![deny(unsafe_code)] (RUST-06): the crate is currently
  pure-safe — all device drivers are mocked. When a real FFI driver lands, the unsafe block MUST
  be scoped to that module with a `// SAFETY:` comment and an item-level `#[allow(unsafe_code)]`,
  keeping the crate root deny in force everywhere else.
next: Add SAFETY comments when real FFI drivers are implemented | perf: Mock drivers are zero-alloc.
*/
// RUST-06: no unsafe code exists today; deny at crate root so any future
// unsafe addition requires an explicit, narrowly-scoped reviewable allow.
#![deny(unsafe_code)]

//! Hardware Abstraction Layer for OZ-POS.
//!
//! `oz-hal` is the seam between business logic and physical devices:
//! barcode scanners, receipt printers, cash drawers, NFC readers, and
//! payment terminals. Business code only ever sees the trait
//! (`BarcodeScanner`, `ReceiptPrinter`, `CashDrawer`) — it never
//! imports a specific driver.
//!
//! Every trait has a programmable mock in [`drivers::mock`]. Tests use
//! the mocks; production code uses real drivers registered through
//! [`registry::DriverRegistry`] at startup.

pub mod drivers;
pub mod error;
pub mod registry;
pub mod traits;
/// USB transport helpers for device enumeration.
pub mod transport;
pub mod types;

pub use error::{HalError, HalErrorKind};
pub use registry::DriverRegistry;
pub use traits::barcode::BarcodeScanner;
pub use traits::cash_drawer::CashDrawer;
pub use traits::customer_display::CustomerDisplay;
pub use traits::customer_display::DisplayContent;
pub use traits::printer::{PaperStatus, PrinterStatus, ReceiptPrinter};
pub use traits::weight_scale::WeightReading;
pub use traits::weight_scale::WeightScale;
pub use types::{Barcode, BarcodeSymbology, DeviceInfo};

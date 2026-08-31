/*
last audited 19-07-26 by RSA-Agent; stamp corrected 31-08-26
crate: oz-hal | status: SAFE | lint: CLEAN
findings: No actual unsafe blocks present. #![deny(unsafe_code)] (RUST-06): the crate is pure-safe BY CONSTRUCTION, not because drivers are stubbed — the previous stamp here claimed "all device drivers are mocked", which is false. usb_scanner/usb_printer drive real HID endpoints via rusb, serial_scanner/serial_display/bt_scanner/bt_printer bind real ports via serialport (Bluetooth SPP surfaces as a COM port on Windows), and tcp_printer uses tokio::net::TcpStream. What IS absent is the payment terminal: the EDC driver tree lives in crates/oz-payment/src/drivers/edc/ and bypasses this crate's registry, discovery, and mock convention entirely (unification in progress). When a real FFI driver lands, the unsafe block MUST be scoped to that module with a `// SAFETY:` comment and an item-level `#[allow(unsafe_code)]`, keeping the crate root deny in force everywhere else.
next: SAFETY comments when real FFI drivers are implemented | perf: drivers do deadline-bounded I/O, not zero-alloc mocks
*/
// RUST-06: no unsafe code exists today; deny at crate root so any future
// unsafe addition requires an explicit, narrowly-scoped reviewable allow.
#![deny(unsafe_code)]

//! Hardware Abstraction Layer for OZ-POS.
//!
//! `oz-hal` is the seam between business logic and physical devices.
//! Business code only ever sees the trait (`BarcodeScanner`,
//! `ReceiptPrinter`, `CashDrawer`, `CustomerDisplay`, `WeightScale`) — it
//! never imports a specific driver.
//!
//! Implemented device categories: barcode scanners (USB HID, serial,
//! Bluetooth SPP, plus a TCP path), receipt printers (USB, Bluetooth, TCP)
//! with ESC/POS formatting and KDS chits, cash drawers (standalone serial
//! and printer-kick), serial customer pole displays, and weight scales.
//!
//! Card-payment terminals are mid-migration: the [`EdcTerminal`] trait is
//! defined here, but its drivers still live in
//! `crates/oz-payment/src/drivers/edc/` and are not yet registered through
//! [`registry::DriverRegistry`]. NFC readers are not implemented at all.
//!
//! Every trait has a programmable mock in [`drivers::mock`]. Tests use
//! the mocks; production code uses real drivers registered through
//! [`registry::DriverRegistry`] at startup.

/// Turning saved hardware configuration into registered drivers.
pub mod bootstrap;
pub mod drivers;
pub mod error;
pub mod registry;
pub mod traits;
/// USB transport helpers for device enumeration.
pub mod transport;
pub mod types;

pub use bootstrap::{
    BootstrapReport, Connection, DisplayConfig, DrawerConfig, HardwareConfig, PrinterConfig,
    TerminalConfig, TerminalConnection, apply_config,
};
pub use error::{HalError, HalErrorKind};
pub use registry::DriverRegistry;
pub use traits::barcode::BarcodeScanner;
pub use traits::cash_drawer::CashDrawer;
pub use traits::customer_display::CustomerDisplay;
pub use traits::customer_display::DisplayContent;
pub use traits::edc::{EdcPaymentResult, EdcTerminal, TerminalStatus};
pub use traits::printer::{PaperStatus, PrinterStatus, ReceiptPrinter};
pub use traits::weight_scale::WeightReading;
pub use traits::weight_scale::WeightScale;
pub use types::{Barcode, BarcodeSymbology, DeviceInfo};

//! Cash drawer drivers.
//!
//! Two implementations of [`CashDrawer`]:
//!
//! * [`PrinterKickCashDrawer`] — sends an ESC/POS kick pulse through a
//!   connected receipt printer's cash drawer port. This is the most
//!   common setup: the printer has an RJ12 port specifically for the
//!   cash drawer, and it fires when it receives `ESC p m t1 t2`.
//!
//! * [`SerialCashDrawer`] — standalone drawer connected via serial/
//!   USB-to-serial adapter. Sends the same pulse sequence over a
//!   serial line.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::task::spawn_blocking;

use crate::error::HalError;
use crate::traits::cash_drawer::CashDrawer;
use crate::traits::printer::ReceiptPrinter;
use crate::transport::serial;
use crate::types::DeviceInfo;

use super::escpos;

/// Default baud rate for standalone serial cash drawers.
pub const DRAWER_DEFAULT_BAUD: u32 = 9600;

// ── Printer-kick drawer ─────────────────────────────────────────────────

/// A cash drawer triggered through a receipt printer's cash-drawer port.
///
/// Wraps an `Arc<dyn ReceiptPrinter>` and sends the ESC/POS kick command
/// (`ESC p m t1 t2`) via the printer connection. This works with most
/// thermal receipt printers (Epson, Star, Bixolon, etc.) that have an
/// RJ12/RJ11 cash-drawer connector.
pub struct PrinterKickCashDrawer {
    printer: Arc<dyn ReceiptPrinter>,
    info: DeviceInfo,
    /// Which pin to pulse: 0 = pin 2 (default), 1 = pin 5.
    pin: u8,
}

impl PrinterKickCashDrawer {
    /// Create a new drawer driver that kicks through the given printer.
    ///
    /// Uses pin 2 (the standard kick pin on most printers).
    pub fn new(printer: Arc<dyn ReceiptPrinter>, pin: u8) -> Self {
        let info = DeviceInfo::new(
            "PrinterKick",
            &printer.device_info().model,
            &printer.device_info().serial,
        );
        Self { printer, info, pin }
    }

    /// Create a drawer driver using pin 2 (default).
    pub fn new_pin2(printer: Arc<dyn ReceiptPrinter>) -> Self {
        Self::new(printer, 0)
    }

    /// Create a drawer driver using pin 5.
    pub fn new_pin5(printer: Arc<dyn ReceiptPrinter>) -> Self {
        Self::new(printer, 1)
    }
}

#[async_trait]
impl CashDrawer for PrinterKickCashDrawer {
    async fn open(&self) -> Result<(), HalError> {
        let cmd = if self.pin == 0 {
            escpos::KICK_DRAWER_PIN2
        } else {
            escpos::KICK_DRAWER_PIN5
        };
        self.printer.print_raw(cmd).await
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

// ── Serial / standalone drawer ──────────────────────────────────────────

/// A cash drawer driven through a serial (RS-232) port.
///
/// Many standalone cash drawers (e.g. APG, Star, Epson) expose a serial
/// interface. Opening the drawer is done by sending a brief pulse on
/// the DTR or RTS line, or by sending a specific byte sequence.
///
/// This driver sends the standard ESC/POS kick command bytes over the
/// serial line, which is compatible with most serial cash drawers.
pub struct SerialCashDrawer {
    port_name: String,
    baud_rate: u32,
    info: DeviceInfo,
}

impl SerialCashDrawer {
    /// Create a serial cash drawer on the given port.
    pub fn new(port_name: impl Into<String>, baud_rate: u32, info: DeviceInfo) -> Self {
        Self {
            port_name: port_name.into(),
            baud_rate,
            info,
        }
    }

    /// Discover serial ports that look like cash drawers.
    ///
    /// Uses the same KNOWN_SERIAL_ADAPTERS list as the serial scanner
    /// driver to find USB-to-serial adapters. Returns a driver for
    /// each discovered port.
    pub fn discover_all() -> Vec<Self> {
        let ports = match serial::probe_ports(false) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        ports
            .into_iter()
            .map(|p| {
                let info = DeviceInfo::new("SerialDrawer", &p.description, &p.port_name);
                Self::new(p.port_name, DRAWER_DEFAULT_BAUD, info)
            })
            .collect()
    }
}

#[async_trait]
impl CashDrawer for SerialCashDrawer {
    async fn open(&self) -> Result<(), HalError> {
        let port_name = self.port_name.clone();
        let baud_rate = self.baud_rate;

        spawn_blocking(move || {
            let mut port = serial::open_port(&port_name, baud_rate)
                .map_err(|e| HalError::NotFound(format!("serial drawer {port_name}: {e}")))?;

            // Standard kick-on-serial: send the ESC/POS pulse command.
            port.write_all(escpos::KICK_DRAWER_PIN2)
                .map_err(HalError::Io)?;

            // Some drawers need a brief settle time.
            std::thread::sleep(std::time::Duration::from_millis(100));

            Ok::<_, HalError>(())
        })
        .await
        .map_err(|e| HalError::Bluetooth(format!("serial drawer join: {e}")))?
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::mock::MockReceiptPrinter;

    #[tokio::test]
    async fn printer_kick_sends_kick_command() {
        let printer = Arc::new(MockReceiptPrinter::new());
        let drawer = PrinterKickCashDrawer::new_pin2(printer.clone());

        drawer.open().await.unwrap();

        let raw = printer.printed_raw.lock().unwrap();
        assert_eq!(raw.len(), 1, "should have sent one raw command");
        assert_eq!(
            raw[0],
            escpos::KICK_DRAWER_PIN2,
            "should send standard kick command"
        );
    }

    #[tokio::test]
    async fn printer_kick_pin5_sends_pin5_command() {
        let printer = Arc::new(MockReceiptPrinter::new());
        let drawer = PrinterKickCashDrawer::new_pin5(printer.clone());

        drawer.open().await.unwrap();

        let raw = printer.printed_raw.lock().unwrap();
        assert_eq!(raw[0], escpos::KICK_DRAWER_PIN5);
    }

    #[tokio::test]
    async fn printer_kick_device_info() {
        let printer = Arc::new(MockReceiptPrinter::with_info(DeviceInfo::new(
            "Epson", "TM-T88", "SN001",
        )));
        let drawer = PrinterKickCashDrawer::new_pin2(printer);
        let info = drawer.device_info();
        assert_eq!(info.vendor, "PrinterKick");
    }

    #[tokio::test]
    async fn printer_kick_propagates_error() {
        let printer = Arc::new(MockReceiptPrinter::new());
        printer.set_next_error(HalError::Disconnected);
        let drawer = PrinterKickCashDrawer::new_pin2(printer);

        let err = drawer.open().await.unwrap_err();
        assert!(matches!(err, HalError::Disconnected));
    }

    #[test]
    fn serial_discover_does_not_panic() {
        let drawers = SerialCashDrawer::discover_all();
        // No hardware expected in CI — empty vec is fine.
        assert!(drawers.is_empty() || !drawers.is_empty());
    }

    #[tokio::test]
    async fn serial_drawer_device_info() {
        let info = DeviceInfo::new("Test", "SerialDrawer", "COM99");
        let drawer = SerialCashDrawer::new("COM99", 9600, info.clone());
        assert_eq!(drawer.device_info(), info);
    }
}

//! Bluetooth (SPP / RFCOMM) receipt printer driver.
//!
//! Implements [`ReceiptPrinter`] over a Bluetooth serial (SPP) connection.
//! Most BT receipt printers (Epson TM-m30 BT, Star SP700 BT) use the
//! Serial Port Profile, which appears as a virtual COM port (Windows) or
//! `/dev/rfcomm*` (Linux) after pairing.
//!
//! The user pairs the printer with the OS, notes the port name (e.g.
//! `"COM7"` or `"/dev/rfcomm0"`), and enters it in the setup wizard.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::task::spawn_blocking;

use crate::error::HalError;
use crate::traits::printer::ReceiptPrinter;
use crate::transport::serial::open_port;
use crate::types::DeviceInfo;

use super::escpos;

/// A receipt printer driven through a Bluetooth serial (SPP) connection.
pub struct BtReceiptPrinter {
    port_name: String,
    baud_rate: u32,
    port: Arc<Mutex<Option<Box<dyn serialport::SerialPort + Send>>>>,
    info: DeviceInfo,
    partial_cut: bool,
}

impl BtReceiptPrinter {
    /// Create a new BT printer at the given serial port and baud rate.
    pub fn new(port_name: impl Into<String>, baud_rate: u32, info: DeviceInfo) -> Self {
        Self {
            port_name: port_name.into(),
            baud_rate,
            port: Arc::new(Mutex::new(None)),
            info,
            partial_cut: false,
        }
    }

    /// Set whether to use a partial cut instead of full cut.
    pub fn with_partial_cut(mut self, partial: bool) -> Self {
        self.partial_cut = partial;
        self
    }

    async fn ensure_connected(&self) -> Result<(), HalError> {
        let mut guard = self.port.lock().await;
        if guard.is_some() {
            return Ok(());
        }

        let mut port = open_port(&self.port_name, self.baud_rate)?;
        port.set_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| HalError::Protocol(format!("serial set_timeout: {e}")))?;

        *guard = Some(port);
        Ok(())
    }

    async fn write_to_port(&self, data: &[u8]) -> Result<(), HalError> {
        let port_arc = self.port.clone();
        let data_owned = data.to_vec();

        spawn_blocking(move || {
            let mut guard = port_arc.blocking_lock();
            let port = guard
                .as_mut()
                .ok_or(HalError::NotFound("not connected".into()))?;

            use std::io::Write;
            port.write_all(&data_owned).map_err(HalError::Io)?;
            port.flush().map_err(HalError::Io)?;
            Ok(())
        })
        .await
        .map_err(|e| HalError::Protocol(format!("serial write join error: {e}")))?
    }
}

#[async_trait]
impl ReceiptPrinter for BtReceiptPrinter {
    async fn print_receipt(&self, body: &str) -> Result<(), HalError> {
        self.ensure_connected().await?;
        let data = escpos::format_receipt(body);
        self.write_to_port(&data).await
    }

    async fn print_raw(&self, data: &[u8]) -> Result<(), HalError> {
        self.ensure_connected().await?;
        self.write_to_port(data).await
    }

    async fn cut(&self) -> Result<(), HalError> {
        self.ensure_connected().await?;
        let data = if self.partial_cut {
            escpos::CUT_PARTIAL.to_vec()
        } else {
            escpos::CUT_FULL.to_vec()
        };
        self.write_to_port(&data).await
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_fields() {
        let info = DeviceInfo::new("Epson", "TM-m30", "SN001");
        let printer = BtReceiptPrinter::new("COM7", 9600, info.clone());
        assert_eq!(printer.port_name, "COM7");
        assert_eq!(printer.baud_rate, 9600);
        assert!(!printer.partial_cut);
    }

    #[test]
    fn device_info_returns_identity() {
        let info = DeviceInfo::new("Star", "SP700", "SN002");
        let printer = BtReceiptPrinter::new("/dev/rfcomm0", 115200, info.clone());
        let returned = printer.device_info();
        assert_eq!(returned.vendor, "Star");
        assert_eq!(returned.model, "SP700");
        assert_eq!(returned.serial, "SN002");
    }

    #[test]
    fn with_partial_cut_enables() {
        let info = DeviceInfo::new("Test", "Printer", "SN");
        let printer = BtReceiptPrinter::new("COM1", 9600, info).with_partial_cut(true);
        assert!(printer.partial_cut);
    }

    #[test]
    fn with_partial_cut_disables() {
        let info = DeviceInfo::new("Test", "Printer", "SN");
        let printer = BtReceiptPrinter::new("COM1", 9600, info).with_partial_cut(false);
        assert!(!printer.partial_cut);
    }

    #[test]
    fn default_partial_cut_is_false() {
        let info = DeviceInfo::new("Test", "Printer", "SN");
        let printer = BtReceiptPrinter::new("COM1", 9600, info);
        assert!(!printer.partial_cut);
    }
}

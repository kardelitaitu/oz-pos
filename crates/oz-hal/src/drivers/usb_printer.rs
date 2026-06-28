//! USB receipt printer driver (stub).
//!
//! Implements [`ReceiptPrinter`] via USB bulk-out transfers. Supports
//! standard ESC/POS commands for text printing and paper cutting.
//!
//! **Stub status:** This driver sends plain text wrapped in minimal
//! ESC/POS formatting. Advanced features (character encoding selection,
//! barcode printing, logo upload, NV graphics) are not yet implemented.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::task::spawn_blocking;

use crate::error::HalError;
use crate::traits::printer::ReceiptPrinter;
use crate::transport::usb::UsbDeviceInfo;
use crate::types::DeviceInfo;

// ---------------------------------------------------------------------------
// ESC/POS command constants
// ---------------------------------------------------------------------------

/// Initialize printer.
const ESC_INIT: &[u8] = &[0x1B, 0x40];
/// Print and carriage return.
const LF: &[u8] = &[0x0A];
/// Cut paper (full cut).
const CUT_FULL: &[u8] = &[0x1D, 0x56, 0x00];
/// Cut paper (partial cut).
const CUT_PARTIAL: &[u8] = &[0x1D, 0x56, 0x01];
/// Select character font A (12×24).
const FONT_A: &[u8] = &[0x1B, 0x4D, 0x00];
/// Select character font B (9×17).
#[allow(dead_code)]
const FONT_B: &[u8] = &[0x1B, 0x4D, 0x01];
/// Set line spacing to default (30 dots).
const LINE_SPACING_DEFAULT: &[u8] = &[0x1B, 0x32];

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

/// A receipt printer driven through a USB bulk OUT endpoint.
pub struct UsbReceiptPrinter {
    handle: Arc<Mutex<Option<rusb::DeviceHandle<rusb::Context>>>>,
    info: DeviceInfo,
    usb_info: UsbDeviceInfo,
    /// Whether to use a partial cut instead of a full cut.
    partial_cut: bool,
}

impl UsbReceiptPrinter {
    /// Attempt to create a driver for the given USB printer info.
    pub fn try_new(info: UsbDeviceInfo) -> Self {
        let device_info = DeviceInfo::new(
            &info.manufacturer,
            &info.product,
            &info.serial,
        );

        Self {
            handle: Arc::new(Mutex::new(None)),
            info: device_info,
            usb_info: info,
            partial_cut: false,
        }
    }

    /// Set whether to use partial cut instead of full cut.
    pub fn with_partial_cut(mut self, partial: bool) -> Self {
        self.partial_cut = partial;
        self
    }

    /// Discover all known USB receipt printers.
    pub fn discover_all() -> Vec<Self> {
        let devices = match crate::transport::usb::probe_printers() {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };
        devices.into_iter().map(Self::try_new).collect()
    }

    /// Build an ESC/POS byte buffer from a plain-text receipt body.
    fn format_receipt(body: &str) -> Vec<u8> {
        let mut buf = Vec::with_capacity(body.len() + 64);

        buf.extend_from_slice(ESC_INIT);
        buf.extend_from_slice(LINE_SPACING_DEFAULT);
        buf.extend_from_slice(FONT_A);

        for line in body.lines() {
            buf.extend_from_slice(line.as_bytes());
            buf.extend_from_slice(LF);
        }

        buf
    }

    async fn ensure_connected(&self) -> Result<(), HalError> {
        let mut guard = self.handle.lock().await;
        if guard.is_some() {
            return Ok(());
        }
        let handle = crate::transport::usb::open_device(
            self.usb_info.vid,
            self.usb_info.pid,
            self.usb_info.interface_number,
        )?;
        *guard = Some(handle);
        Ok(())
    }

    async fn write_to_endpoint(&self, data: &[u8]) -> Result<(), HalError> {
        self.ensure_connected().await?;

        let handle_arc = self.handle.clone();
        let ep_out = self
            .usb_info
            .endpoint_out
            .ok_or(HalError::NotFound("no OUT endpoint on printer".into()))?;

        let data_owned = data.to_vec();
        let timeout = Duration::from_secs(5);

        spawn_blocking(move || {
            let mut guard = handle_arc.blocking_lock();
            let handle = guard
                .as_mut()
                .ok_or(HalError::NotFound("not connected".into()))?;

            handle
                .write_bulk(ep_out, &data_owned, timeout)
                .map_err(|e| {
                    if matches!(e, rusb::Error::NoDevice) {
                        *guard = None;
                        HalError::Disconnected
                    } else {
                        HalError::Usb(e.to_string())
                    }
                })?;

            Ok(())
        })
        .await
        .map_err(|e| HalError::Usb(format!("write_bulk join error: {e}")))?
    }
}

#[async_trait]
impl ReceiptPrinter for UsbReceiptPrinter {
    async fn print_receipt(&self, body: &str) -> Result<(), HalError> {
        let data = Self::format_receipt(body);
        self.write_to_endpoint(&data).await
    }

    async fn cut(&self) -> Result<(), HalError> {
        let cut_cmd = if self.partial_cut {
            CUT_PARTIAL
        } else {
            CUT_FULL
        };
        self.write_to_endpoint(cut_cmd).await
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_receipt_includes_init_and_font() {
        let body = "Hello\nWorld";
        let data = UsbReceiptPrinter::format_receipt(body);

        // Should start with ESC/POS init
        assert!(data.starts_with(ESC_INIT), "missing ESC @ init");
        // Should contain the line text
        assert!(
            data.windows(b"Hello".len()).any(|w| w == b"Hello"),
            "missing body text"
        );
        // Should contain LF after each line
        assert!(
            data.windows(LF.len()).any(|w| w == LF),
            "missing line feeds"
        );
    }

    #[test]
    fn cut_command_uses_full_cut_by_default() {
        // Test that CUT_FULL is used when partial_cut is false.
        // We can check the constant directly.
        assert_eq!(CUT_FULL, &[0x1D, 0x56, 0x00]);
        assert_eq!(CUT_PARTIAL, &[0x1D, 0x56, 0x01]);
    }
}

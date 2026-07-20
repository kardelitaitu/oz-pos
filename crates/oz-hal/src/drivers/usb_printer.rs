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

use super::escpos;

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
        let device_info = DeviceInfo::new(&info.manufacturer, &info.product, &info.serial);

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
        let data = escpos::format_receipt(body);
        self.write_to_endpoint(&data).await
    }

    async fn print_raw(&self, data: &[u8]) -> Result<(), HalError> {
        self.write_to_endpoint(data).await
    }

    async fn cut(&self) -> Result<(), HalError> {
        self.ensure_connected().await?;
        let cut_cmd = if self.partial_cut {
            escpos::CUT_PARTIAL
        } else {
            escpos::CUT_FULL
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
    use crate::transport::usb::DeviceCategory;

    #[test]
    fn try_new_stores_info() {
        let usb_info = UsbDeviceInfo {
            vid: 0x0416,
            pid: 0x5011,
            manufacturer: "Epson".into(),
            product: "TM-T20".into(),
            serial: "SN007".into(),
            interface_number: 0,
            endpoint_in: 0x81,
            endpoint_out: Some(0x02),
            category: DeviceCategory::Printer,
            label: String::new(),
        };
        let printer = UsbReceiptPrinter::try_new(usb_info);
        let info = printer.device_info();
        assert_eq!(info.vendor, "Epson");
        assert_eq!(info.model, "TM-T20");
        assert_eq!(info.serial, "SN007");
    }

    #[test]
    fn device_info_reflects_manufacturer() {
        let usb_info = UsbDeviceInfo {
            vid: 0x0519,
            pid: 0x0301,
            manufacturer: "Star".into(),
            product: "TSP100".into(),
            serial: "SN008".into(),
            interface_number: 0,
            endpoint_in: 0x81,
            endpoint_out: Some(0x02),
            category: DeviceCategory::Printer,
            label: String::new(),
        };
        let printer = UsbReceiptPrinter::try_new(usb_info);
        assert_eq!(printer.device_info().vendor, "Star");
    }

    #[test]
    fn with_partial_cut_enables() {
        let usb_info = UsbDeviceInfo {
            vid: 0x0416,
            pid: 0x5011,
            manufacturer: "Epson".into(),
            product: "TM-T20".into(),
            serial: "SN".into(),
            interface_number: 0,
            endpoint_in: 0x81,
            endpoint_out: Some(0x02),
            category: DeviceCategory::Printer,
            label: String::new(),
        };
        let printer = UsbReceiptPrinter::try_new(usb_info).with_partial_cut(true);
        assert!(printer.partial_cut);
    }

    #[test]
    fn default_partial_cut_is_false() {
        let usb_info = UsbDeviceInfo {
            vid: 0x0416,
            pid: 0x5011,
            manufacturer: "Epson".into(),
            product: "TM-T20".into(),
            serial: "SN".into(),
            interface_number: 0,
            endpoint_in: 0x81,
            endpoint_out: Some(0x02),
            category: DeviceCategory::Printer,
            label: String::new(),
        };
        let printer = UsbReceiptPrinter::try_new(usb_info);
        assert!(!printer.partial_cut);
    }

    #[test]
    fn discover_all_does_not_panic() {
        let printers = UsbReceiptPrinter::discover_all();
        // No USB hardware expected in CI — empty vec is fine.
        assert!(printers.is_empty() || !printers.is_empty());
    }

    #[test]
    fn try_new_preserves_usb_info() {
        let usb_info = UsbDeviceInfo {
            vid: 0x0416,
            pid: 0x5011,
            manufacturer: "Epson".into(),
            product: "TM-T20".into(),
            serial: "SN009".into(),
            interface_number: 0,
            endpoint_in: 0x81,
            endpoint_out: Some(0x02),
            category: DeviceCategory::Printer,
            label: String::new(),
        };
        let printer = UsbReceiptPrinter::try_new(usb_info.clone());
        assert_eq!(printer.usb_info.vid, 0x0416);
        assert_eq!(printer.usb_info.pid, 0x5011);
        assert_eq!(printer.usb_info.serial, "SN009");
    }
}

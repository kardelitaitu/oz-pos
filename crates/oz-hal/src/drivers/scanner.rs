//! Sample "real" barcode driver.
//!
//! Real drivers (Honeywell, Datalogic, Zebra) live in their own files
//! and depend on `rusb`, `btleplug`, or `serialport`. This scaffold
//! ships a wrapper that delegates to [`MockBarcodeScanner`] so the
//! rest of the code can be exercised end-to-end without USB hardware
//! plugged in. The wrapper demonstrates the trait pattern: it implements
//! [`BarcodeScanner`] and is registered the same way a real driver will
//! be once one is added.
//!
//! When the first real driver lands, this file should become the
//! Honeywell / Datalogic / etc. driver and the mock import is removed.

use std::sync::Arc;

use async_trait::async_trait;

use crate::drivers::mock::MockBarcodeScanner;
use crate::error::HalError;
use crate::traits::barcode::BarcodeScanner;
use crate::types::{Barcode, DeviceInfo};

/// A barcode scanner driver placeholder. Replace with a real `rusb`
/// implementation once hardware is available.
pub struct UsbBarcodeScanner {
    inner: MockBarcodeScanner,
    info: DeviceInfo,
}

impl UsbBarcodeScanner {
    /// Construct a USB scanner mock with the given identity. In a real
    /// driver this would open the USB device and store the handle.
    #[must_use]
    pub fn new(info: DeviceInfo) -> Self {
        Self {
            inner: MockBarcodeScanner::with_info(info.clone()),
            info,
        }
    }
}

impl Default for UsbBarcodeScanner {
    fn default() -> Self {
        Self::new(DeviceInfo::new("OZ-POS", "UsbBarcodeScanner", "0001"))
    }
}

#[async_trait]
impl BarcodeScanner for UsbBarcodeScanner {
    async fn connect(&self) -> Result<Box<dyn BarcodeScanner>, HalError> {
        // A real driver would open a USB handle here. For the scaffold
        // we delegate to the mock so the rest of the stack works.
        self.inner.connect().await
    }

    async fn poll(&mut self, timeout_ms: u32) -> Result<Option<Barcode>, HalError> {
        self.inner.poll(timeout_ms).await
    }

    async fn cancel(&self) -> Result<(), HalError> {
        self.inner.cancel().await
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

/// Convenience: register a default `UsbBarcodeScanner` on the given
/// registry under the id `"default"`. Used by the setup wizard.
pub async fn register_default(registry: &super::super::DriverRegistry, info: DeviceInfo) {
    let scanner: Arc<dyn BarcodeScanner> = Arc::new(UsbBarcodeScanner::new(info));
    registry.register_scanner("default", scanner).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DriverRegistry;

    #[tokio::test]
    async fn registers_and_polls() {
        let reg = DriverRegistry::default();
        let info = DeviceInfo::new("test", "UsbBarcodeScanner", "x");
        register_default(&reg, info.clone()).await;
        let scanner = reg.scanner("default").await.unwrap();
        assert_eq!(scanner.device_info().vendor, info.vendor);
        assert_eq!(scanner.device_info().model, info.model);
    }
}

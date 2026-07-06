use serde::{Deserialize, Serialize};

use crate::error::HalError;

/// A single weight reading from the scale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightReading {
    /// Weight in grams.
    pub weight_grams: f64,
    /// Whether the scale reports the reading as stable.
    pub stable: bool,
}

/// Trait for USB HID weight scale drivers.
pub trait WeightScale: Send + Sync {
    /// Read the current weight from the scale.
    ///
    /// Returns a [`WeightReading`] on success, or a [`HalError`] if the
    /// device is disconnected, busy, or returns an invalid packet.
    fn read_weight(&self) -> Result<WeightReading, HalError>;

    /// Static device identity (vendor, model, serial).
    fn device_info(&self) -> crate::types::DeviceInfo;
}

/// A real USB HID weight scale driver.
///
/// Communicates with the scale over the HID POS usage page
/// (`0x0001:0x0011`). This is a basic implementation that reads
/// from a configured device path.
pub struct HidWeightScale {
    device_path: String,
    /// Placeholder for a future HID device handle.
    _vendor_id: u16,
    _product_id: u16,
}

impl HidWeightScale {
    /// Create a new `HidWeightScale`.
    ///
    /// `vendor_id` and `product_id` identify the USB device.
    /// `device_path` is the platform-specific path (e.g. `/dev/hidraw0`
    /// or `COM3`).
    pub fn new(vendor_id: u16, product_id: u16, device_path: String) -> Self {
        Self {
            device_path,
            _vendor_id: vendor_id,
            _product_id: product_id,
        }
    }

    /// The configured device path.
    pub fn device_path(&self) -> &str {
        &self.device_path
    }
}

impl WeightScale for HidWeightScale {
    fn read_weight(&self) -> Result<WeightReading, HalError> {
        // Stub implementation: In production this would:
        // 1. Open the HID device at self.device_path
        // 2. Send a GET_WEIGHT command (or listen for reports)
        // 3. Parse the HID POS scale report (Usage Page 0x0011)
        //    to extract weight value and stability flag
        //
        // For now, return a "not connected" error so the caller
        // knows the physical device has not been attached.
        Err(HalError::NotFound(format!(
            "scale at {} not available — stub implementation",
            self.device_path
        )))
    }

    fn device_info(&self) -> crate::types::DeviceInfo {
        crate::types::DeviceInfo::new(
            format!("{:04x}", self._vendor_id),
            format!("{:04x}", self._product_id),
            &self.device_path,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_device_path() {
        let scale = HidWeightScale::new(0x1234, 0x5678, "/dev/hidraw0".into());
        assert_eq!(scale.device_path(), "/dev/hidraw0");
    }

    #[test]
    fn read_weight_returns_not_found() {
        let scale = HidWeightScale::new(0x1234, 0x5678, "COM3".into());
        let result = scale.read_weight();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HalError::NotFound(_)));
        assert!(err.to_string().contains("not available"));
    }

    #[test]
    fn device_info_returns_vendor_and_product() {
        let scale = HidWeightScale::new(0x1234, 0x5678, "COM3".into());
        let info = scale.device_info();
        assert_eq!(info.vendor, "1234");
        assert_eq!(info.model, "5678");
    }

    #[test]
    fn device_info_includes_device_path() {
        let scale = HidWeightScale::new(0x0001, 0x0011, "/dev/hidraw0".into());
        let info = scale.device_info();
        assert_eq!(info.serial, "/dev/hidraw0");
    }
}

use serde::Serialize;
use tauri::{State, command};

use oz_hal::WeightReading;

use crate::error::AppError;
use crate::state::AppState;

/// Information about a detected scale device.
#[derive(Debug, Serialize)]
pub struct ScaleDeviceInfo {
    /// Vendor ID in hex (e.g. `"0x0922"`).
    pub vendor_id: String,
    /// Product ID in hex (e.g. `"0x8001"`).
    pub product_id: String,
    /// Platform device path.
    pub device_path: String,
}

/// Read the current weight from the registered weight scale.
///
/// Uses the default scale registered under the "default" key.
/// Returns `None` if no scale is registered.
#[command]
pub async fn read_scale_weight(
    state: State<'_, AppState>,
) -> Result<Option<WeightReading>, AppError> {
    let scale = state.registry.scale("default").await;
    match scale {
        Some(s) => {
            let reading = s.read_weight()?;
            Ok(Some(reading))
        }
        None => Ok(None),
    }
}

/// List all registered weight scales.
#[command]
pub async fn list_scale_devices(
    state: State<'_, AppState>,
) -> Result<Vec<ScaleDeviceInfo>, AppError> {
    let ids = state.registry.scale_ids().await;
    let mut devices = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(scale) = state.registry.scale(&id).await {
            let info = scale.device_info();
            devices.push(ScaleDeviceInfo {
                vendor_id: info.vendor,
                product_id: info.model,
                device_path: info.serial,
            });
        }
    }
    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_device_info_debug() {
        let info = ScaleDeviceInfo {
            vendor_id: "0x0922".into(),
            product_id: "0x8001".into(),
            device_path: "/dev/hidraw0".into(),
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("0x0922"));
    }

    #[test]
    fn scale_device_info_serialize() {
        let info = ScaleDeviceInfo {
            vendor_id: "0x1234".into(),
            product_id: "0x5678".into(),
            device_path: "/dev/ttyUSB0".into(),
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["vendor_id"], "0x1234");
        assert_eq!(json["product_id"], "0x5678");
    }

    #[test]
    fn scale_device_info_empty_fields() {
        let info = ScaleDeviceInfo {
            vendor_id: String::new(),
            product_id: String::new(),
            device_path: String::new(),
        };
        assert_eq!(info.vendor_id, "");
    }
}

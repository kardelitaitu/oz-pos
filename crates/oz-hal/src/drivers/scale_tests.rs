
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

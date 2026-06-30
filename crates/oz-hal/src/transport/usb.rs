//! USB device enumeration helpers for barcode scanners and receipt printers.
//!
//! Uses `rusb` (libusb wrapper) to probe for HID-class and printer-class
//! devices by known VID/PID pairs. The discovery functions in this module
//! are called by [`DriverRegistry::discover()`] at startup.

use rusb::{Context, UsbContext};

use crate::error::HalError;

/// USB interface class code for HID devices.
pub const CLASS_HID: u8 = 3;
/// USB interface class code for printer devices.
pub const CLASS_PRINTER: u8 = 7;
/// Vendor-specific class — some barcode scanners use this.
pub const CLASS_VENDOR_SPECIFIC: u8 = 0xFF;

/// Static metadata about a discovered USB device.
#[derive(Debug, Clone)]
pub struct UsbDeviceInfo {
    pub vid: u16,
    pub pid: u16,
    pub manufacturer: String,
    pub product: String,
    pub serial: String,
    pub interface_number: u8,
    pub endpoint_in: u8,
    pub endpoint_out: Option<u8>,
}

/// Known barcode scanner VID/PID pairs.
pub const KNOWN_SCANNERS: &[(u16, u16)] = &[
    // Honeywell
    (0x0C2E, 0x0A10), // Voyager 1450g
    (0x0C2E, 0x0A11), // Voyager 1452g
    (0x0C2E, 0x0B10), // Voyager 1900g
    (0x0C2E, 0x0B11), // Voyager 1902g
    // Datalogic
    (0x05F9, 0x2211), // Magellan 800i
    (0x05F9, 0x2212), // Magellan 900i
    (0x05F9, 0x2201), // Gryphon
    (0x05F9, 0x2203), // QuickScan
    // Zebra
    (0x06DA, 0x5001), // LI3678
    (0x06DA, 0x5002), // DS3678
    (0x06DA, 0x4001), // DS2208
    (0x06DA, 0x4002), // DS4608
    // Generic / other
    (0x045E, 0x0800), // Microsoft USB barcode scanner
    (0x055D, 0x2020), // Wasp WLR-8950
];

/// Known receipt printer VID/PID pairs.
pub const KNOWN_PRINTERS: &[(u16, u16)] = &[
    // Epson
    (0x0416, 0x5011), // TM-T20
    (0x0416, 0x5021), // TM-T88VI
    (0x0416, 0x5031), // TM-T70
    (0x0416, 0x5041), // TM-m30
    // Star
    (0x0519, 0x0201), // SP700
    (0x0519, 0x0301), // TSP100
    (0x0519, 0x0401), // mC-Print3
    // Bixolon
    (0x0525, 0xA800), // SRP-350
    (0x0525, 0xA900), // SRP-275
    // Generic
    (0x067B, 0x2305), // Prolific-based USB printer
];

/// Enumerate USB devices whose interface class matches `class`.
pub fn probe_by_class(class: u8) -> Result<Vec<UsbDeviceInfo>, HalError> {
    let context =
        Context::new().map_err(|e| HalError::Usb(format!("failed to create USB context: {e}")))?;
    let devices = context
        .devices()
        .map_err(|e| HalError::Usb(format!("failed to list USB devices: {e}")))?;

    let mut results = Vec::new();

    for device in devices.iter() {
        let desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };

        let config = match device.config_descriptor(0) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for interface in config.interfaces() {
            for setting in interface.descriptors() {
                if setting.class_code() != class {
                    continue;
                }

                let if_num = setting.interface_number();
                let mut ep_in = None;
                let mut ep_out = None;

                for ep in setting.endpoint_descriptors() {
                    if ep.direction() == rusb::Direction::In {
                        ep_in = Some(ep.address());
                    } else {
                        ep_out = Some(ep.address());
                    }
                }

                let Some(ep_in) = ep_in else { continue };

                let (manufacturer, product, serial) = match device.open() {
                    Ok(handle) => (
                        handle
                            .read_manufacturer_string_ascii(&desc)
                            .unwrap_or_default(),
                        handle.read_product_string_ascii(&desc).unwrap_or_default(),
                        handle
                            .read_serial_number_string_ascii(&desc)
                            .unwrap_or_default(),
                    ),
                    Err(_) => (String::new(), String::new(), String::new()),
                };

                results.push(UsbDeviceInfo {
                    vid: desc.vendor_id(),
                    pid: desc.product_id(),
                    manufacturer,
                    product,
                    serial,
                    interface_number: if_num,
                    endpoint_in: ep_in,
                    endpoint_out: ep_out,
                });
            }
        }
    }

    Ok(results)
}

/// Probe for USB HID barcode scanners matching [`KNOWN_SCANNERS`].
///
/// Returns an empty vec (not an error) when no hardware is found — this
/// lets the system fall back to the mock driver.
pub fn probe_scanners() -> Result<Vec<UsbDeviceInfo>, HalError> {
    let mut results = probe_by_class(CLASS_HID).unwrap_or_default();

    // Also check vendor-specific class for devices not exposing HID class
    if let Ok(vendor_devices) = probe_by_class(CLASS_VENDOR_SPECIFIC) {
        for dev in vendor_devices {
            if KNOWN_SCANNERS.contains(&(dev.vid, dev.pid))
                && !results.iter().any(|r| r.vid == dev.vid && r.pid == dev.pid)
            {
                results.push(dev);
            }
        }
    }

    results.retain(|d| KNOWN_SCANNERS.contains(&(d.vid, d.pid)));
    Ok(results)
}

/// Probe for USB receipt printers matching [`KNOWN_PRINTERS`].
pub fn probe_printers() -> Result<Vec<UsbDeviceInfo>, HalError> {
    let results = probe_by_class(CLASS_PRINTER).unwrap_or_default();
    let filtered: Vec<_> = results
        .into_iter()
        .filter(|d| KNOWN_PRINTERS.contains(&(d.vid, d.pid)))
        .collect();
    Ok(filtered)
}

/// Helper to open a USB device and claim an interface.
pub fn open_device(
    vid: u16,
    pid: u16,
    interface: u8,
) -> Result<rusb::DeviceHandle<rusb::Context>, HalError> {
    let context =
        Context::new().map_err(|e| HalError::Usb(format!("failed to create USB context: {e}")))?;

    let device = context
        .devices()
        .map_err(|e| HalError::Usb(format!("failed to list devices: {e}")))?
        .iter()
        .find(|d| {
            d.device_descriptor()
                .map(|desc| desc.vendor_id() == vid && desc.product_id() == pid)
                .unwrap_or(false)
        })
        .ok_or_else(|| HalError::NotFound(format!("USB device {vid:#06x}:{pid:#06x}")))?;

    let handle = device
        .open()
        .map_err(|e| HalError::Usb(format!("failed to open USB device: {e}")))?;

    handle
        .claim_interface(interface)
        .map_err(|e| HalError::Usb(format!("failed to claim interface {interface}: {e}")))?;

    // On Linux, detach the kernel driver if it's attached.
    if handle.kernel_driver_active(interface).unwrap_or(false) {
        let _ = handle.detach_kernel_driver(interface);
    }

    Ok(handle)
}

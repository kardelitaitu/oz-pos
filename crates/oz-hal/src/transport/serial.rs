//! Serial port enumeration and connection helpers for serial barcode
//! scanners and serial receipt printers.
//!
//! Many barcode scanners (e.g. Honeywell, Datalogic) and receipt printers
//! (e.g. Epson) offer a serial (RS-232) interface. On modern machines
//! this is usually accessed via a USB-to-serial adapter (FTDI, CH340,
//! CP210x, Prolific).

use std::time::Duration;

use serialport::{SerialPort, SerialPortBuilder, SerialPortType, UsbPortInfo, available_ports};

use crate::error::HalError;

/// Known VID/PID pairs for common USB-to-serial adapters used by POS
/// hardware. These are matched against the adapter, not the peripheral
/// itself (which might have a generic serial interface).
const KNOWN_SERIAL_ADAPTERS: &[(u16, u16)] = &[
    (0x0403, 0x6001), // FTDI FT232R
    (0x0403, 0x6015), // FTDI FT231X
    (0x0403, 0x6010), // FTDI FT2232C
    (0x0403, 0x6011), // FTDI FT4232
    (0x1A86, 0x7523), // CH340
    (0x1A86, 0x55D4), // CH34x (newer)
    (0x10C4, 0xEA60), // CP210x
    (0x10C4, 0xEA70), // CP210x (CP2102N)
    (0x067B, 0x2303), // Prolific PL2303
    (0x067B, 0x23C3), // Prolific PL2303GC
    (0x0403, 0xFA00), // Honeywell specific adapter
];

/// Static metadata about a discovered serial port.
#[derive(Debug, Clone)]
pub struct SerialPortInfo {
    /// OS port name (e.g. `"COM3"` on Windows, `"/dev/ttyUSB0"` on Linux).
    pub port_name: String,
    /// Human-readable description (if available).
    pub description: String,
    /// USB vendor ID if the port is behind a USB adapter.
    pub vid: Option<u16>,
    /// USB product ID if the port is behind a USB adapter.
    pub pid: Option<u16>,
}

/// Enumerate serial ports and return those connected via known POS
/// hardware adapters, or all ports when `only_known` is false.
///
/// Use [`probe_bluetooth()`] specifically to find Bluetooth SPP serial
/// ports that are typically used by BT receipt printers.
pub fn probe_ports(only_known: bool) -> Result<Vec<SerialPortInfo>, HalError> {
    let ports =
        available_ports().map_err(|e| HalError::Io(std::io::Error::other(e.to_string())))?;

    let mut results = Vec::new();

    for port in ports {
        let (vid, pid) = match &port.port_type {
            SerialPortType::UsbPort(UsbPortInfo { vid, pid, .. }) => (Some(*vid), Some(*pid)),
            SerialPortType::BluetoothPort => (None, None),
            SerialPortType::PciPort => (None, None),
            _ => (None, None),
        };

        if only_known {
            let adapter_matched = vid
                .zip(pid)
                .is_some_and(|(v, p)| KNOWN_SERIAL_ADAPTERS.contains(&(v, p)));

            // If vid/pid are available but not in our known list, skip.
            // If no vid/pid (e.g. PCI serial card), include it — the
            // user might have explicitly configured it.
            if vid.is_some() && pid.is_some() && !adapter_matched {
                continue;
            }
        }

        results.push(SerialPortInfo {
            port_name: port.port_name,
            description: description_for_type(&port.port_type),
            vid,
            pid,
        });
    }

    Ok(results)
}

/// Open a serial port at the given baud rate with typical POS device
/// settings (8 data bits, 1 stop bit, no parity, no flow control).
pub fn open_port(port_name: &str, baud_rate: u32) -> Result<Box<dyn SerialPort>, HalError> {
    serialport::new(port_name, baud_rate)
        .data_bits(serialport::DataBits::Eight)
        .stop_bits(serialport::StopBits::One)
        .parity(serialport::Parity::None)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(1000))
        .open()
        .map_err(|e| HalError::NotFound(format!("failed to open serial port {port_name}: {e}")))
}

/// Open a serial port with full custom settings.
pub fn open_port_with_settings(
    builder: SerialPortBuilder,
) -> Result<Box<dyn SerialPort>, HalError> {
    builder
        .open()
        .map_err(|e| HalError::NotFound(format!("failed to open serial port: {e}")))
}

/// Enumerate only Bluetooth SPP serial ports — used by BT receipt printers.
///
/// On Windows these appear as virtual COM ports after pairing; on Linux
/// as `/dev/rfcomm*` or `/dev/tty*` with Bluetooth port type.
pub fn probe_bluetooth() -> Result<Vec<SerialPortInfo>, HalError> {
    let ports =
        available_ports().map_err(|e| HalError::Io(std::io::Error::other(e.to_string())))?;

    Ok(ports
        .into_iter()
        .filter(|p| matches!(p.port_type, SerialPortType::BluetoothPort))
        .map(|p| SerialPortInfo {
            port_name: p.port_name,
            description: description_for_type(&p.port_type),
            vid: None,
            pid: None,
        })
        .collect())
}

fn description_for_type(port_type: &SerialPortType) -> String {
    match port_type {
        SerialPortType::UsbPort(info) => {
            format!(
                "{} {}",
                info.manufacturer.as_deref().unwrap_or("Unknown"),
                info.product.as_deref().unwrap_or("USB Serial"),
            )
        }
        SerialPortType::BluetoothPort => "Bluetooth Serial".into(),
        SerialPortType::PciPort => "PCI Serial".into(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_serial_does_not_panic() {
        // This test just verifies the function runs without panicking.
        // No hardware expected in CI — should return an empty vec or error.
        let result = probe_ports(true);
        assert!(result.is_ok() || result.is_err());
    }

    // ── SerialPortInfo struct tests ──────────────────────────────────

    #[test]
    fn serial_port_info_debug() {
        let info = SerialPortInfo {
            port_name: "COM3".into(),
            description: "FTDI FT232R USB UART".into(),
            vid: Some(0x0403),
            pid: Some(0x6001),
        };
        let debug = format!("{info:?}");
        assert!(debug.contains("COM3"));
        assert!(debug.contains("FTDI"));
        assert!(debug.contains("1027")); // 0x0403 in decimal
        assert!(debug.contains("24577")); // 0x6001 in decimal
    }

    #[test]
    fn serial_port_info_clone_eq() {
        let info = SerialPortInfo {
            port_name: "COM3".into(),
            description: "Test".into(),
            vid: Some(0x0403),
            pid: Some(0x6001),
        };
        let cloned = info.clone();
        assert_eq!(info.port_name, cloned.port_name);
        assert_eq!(info.description, cloned.description);
        assert_eq!(info.vid, cloned.vid);
        assert_eq!(info.pid, cloned.pid);
    }

    #[test]
    fn serial_port_info_no_vid_pid() {
        let info = SerialPortInfo {
            port_name: "COM1".into(),
            description: "Onboard Serial".into(),
            vid: None,
            pid: None,
        };
        assert_eq!(info.port_name, "COM1");
        assert!(info.vid.is_none());
        assert!(info.pid.is_none());
    }

    #[test]
    fn serial_port_info_vid_only() {
        let info = SerialPortInfo {
            port_name: "/dev/ttyS0".into(),
            description: "Legacy Serial".into(),
            vid: Some(0x0403),
            pid: None,
        };
        assert_eq!(info.vid, Some(0x0403));
        assert!(info.pid.is_none());
    }

    #[test]
    fn serial_port_info_empty_description() {
        let info = SerialPortInfo {
            port_name: "/dev/ttyUSB0".into(),
            description: String::new(),
            vid: None,
            pid: None,
        };
        assert_eq!(info.description, "");
    }

    // ── description_for_type tests ───────────────────────────────────

    #[test]
    fn description_for_usb_port_with_manufacturer() {
        let info = UsbPortInfo {
            vid: 0x0403,
            pid: 0x6001,
            serial_number: Some("A123".into()),
            manufacturer: Some("FTDI".into()),
            product: Some("FT232R".into()),
        };
        let desc = description_for_type(&SerialPortType::UsbPort(info));
        assert_eq!(desc, "FTDI FT232R");
    }

    #[test]
    fn description_for_usb_port_without_manufacturer() {
        let info = UsbPortInfo {
            vid: 0x1A86,
            pid: 0x7523,
            serial_number: None,
            manufacturer: None,
            product: None,
        };
        let desc = description_for_type(&SerialPortType::UsbPort(info));
        assert_eq!(desc, "Unknown USB Serial");
    }

    #[test]
    fn description_for_usb_port_product_only() {
        let info = UsbPortInfo {
            vid: 0x1A86,
            pid: 0x7523,
            serial_number: None,
            manufacturer: None,
            product: Some("CH340".into()),
        };
        let desc = description_for_type(&SerialPortType::UsbPort(info));
        assert_eq!(desc, "Unknown CH340");
    }

    #[test]
    fn description_for_bluetooth_port() {
        let desc = description_for_type(&SerialPortType::BluetoothPort);
        assert_eq!(desc, "Bluetooth Serial");
    }

    #[test]
    fn description_for_pci_port() {
        let desc = description_for_type(&SerialPortType::PciPort);
        assert_eq!(desc, "PCI Serial");
    }

    // ── Known serial adapters tests ──────────────────────────────────

    #[test]
    fn known_serial_adapters_non_empty() {
        assert!(!KNOWN_SERIAL_ADAPTERS.is_empty());
    }

    #[test]
    fn known_serial_adapters_no_duplicates() {
        let len = KNOWN_SERIAL_ADAPTERS.len();
        let mut unique: Vec<_> = KNOWN_SERIAL_ADAPTERS.to_vec();
        unique.sort();
        unique.dedup();
        assert_eq!(
            unique.len(),
            len,
            "KNOWN_SERIAL_ADAPTERS has duplicate entries"
        );
    }

    #[test]
    fn known_serial_adapters_contains_ftdi() {
        assert!(KNOWN_SERIAL_ADAPTERS.contains(&(0x0403, 0x6001)));
    }

    #[test]
    fn known_serial_adapters_contains_ch340() {
        assert!(KNOWN_SERIAL_ADAPTERS.contains(&(0x1A86, 0x7523)));
    }

    #[test]
    fn known_serial_adapters_count() {
        assert_eq!(KNOWN_SERIAL_ADAPTERS.len(), 11);
    }
}

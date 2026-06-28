//! `DriverRegistry` — the runtime's catalogue of available hardware.
//!
//! The registry holds `Arc<dyn Trait>` per device category, indexed by a
//! user-defined string id. Commands reach hardware through the registry
//! (`state.registry.scanner(id)`) and never construct a specific driver.
//!
//! Discovery (`DriverRegistry::discover()`) probes USB, Bluetooth, and
//! serial at startup and populates the registry. Failure of one driver
//! does not abort discovery; the rest still get registered.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::traits::barcode::BarcodeScanner;
use crate::traits::cash_drawer::CashDrawer;
use crate::traits::printer::ReceiptPrinter;
use crate::types::DeviceInfo;

/// Shared, mutable catalogue of HAL drivers.
#[derive(Default)]
pub struct DriverRegistry {
    scanners: RwLock<HashMap<String, Arc<dyn BarcodeScanner>>>,
    printers: RwLock<HashMap<String, Arc<dyn ReceiptPrinter>>>,
    drawers: RwLock<HashMap<String, Arc<dyn CashDrawer>>>,
}

impl DriverRegistry {
    /// Construct an empty registry. Use `register_*` to add devices.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a barcode scanner under `id`. Overwrites any previous
    /// entry with the same id.
    pub async fn register_scanner(&self, id: &str, driver: Arc<dyn BarcodeScanner>) {
        self.scanners.write().await.insert(id.to_owned(), driver);
    }

    /// Register a receipt printer under `id`. Overwrites any previous
    /// entry with the same id.
    pub async fn register_printer(&self, id: &str, driver: Arc<dyn ReceiptPrinter>) {
        self.printers.write().await.insert(id.to_owned(), driver);
    }

    /// Register a cash drawer under `id`. Overwrites any previous
    /// entry with the same id.
    pub async fn register_cash_drawer(&self, id: &str, driver: Arc<dyn CashDrawer>) {
        self.drawers.write().await.insert(id.to_owned(), driver);
    }

    /// Look up a scanner by id. Returns `None` if no scanner is registered.
    pub async fn scanner(&self, id: &str) -> Option<Arc<dyn BarcodeScanner>> {
        self.scanners.read().await.get(id).cloned()
    }

    /// Look up a printer by id. Returns `None` if no printer is registered.
    pub async fn printer(&self, id: &str) -> Option<Arc<dyn ReceiptPrinter>> {
        self.printers.read().await.get(id).cloned()
    }

    /// Look up a cash drawer by id. Returns `None` if no drawer is registered.
    pub async fn cash_drawer(&self, id: &str) -> Option<Arc<dyn CashDrawer>> {
        self.drawers.read().await.get(id).cloned()
    }

    /// Snapshot of registered scanner ids (for the setup wizard's "what's
    /// plugged in?" view).
    pub async fn scanner_ids(&self) -> Vec<String> {
        self.scanners.read().await.keys().cloned().collect()
    }

    /// Snapshot of registered printer ids.
    pub async fn printer_ids(&self) -> Vec<String> {
        self.printers.read().await.keys().cloned().collect()
    }

    /// Snapshot of registered cash drawer ids.
    pub async fn drawer_ids(&self) -> Vec<String> {
        self.drawers.read().await.keys().cloned().collect()
    }

    /// Discover and register available hardware. Failure of one driver
    /// does not abort the rest. Probes USB HID scanners, serial scanners,
    /// and USB receipt printers, then registers them all.
    pub async fn discover(&self) {
        // --- USB HID barcode scanners ---
        for scanner in crate::drivers::usb_scanner::UsbHidBarcodeScanner::discover_all() {
            let info = scanner.device_info();
            let id = if info.serial.is_empty() || info.serial == "0000" {
                format!("scanner:usb:{}:{}", info.vendor, info.model)
            } else {
                format!("scanner:usb:{}", info.serial)
            };
            self.register_scanner(&id, Arc::new(scanner)).await;
        }

        // --- Serial barcode scanners ---
        for scanner in crate::drivers::serial_scanner::SerialBarcodeScanner::discover_all() {
            let info = scanner.device_info();
            // Serial port name is used as the identity key.
            let id = format!("scanner:serial:{}", info.serial);
            self.register_scanner(&id, Arc::new(scanner)).await;
        }

        // --- USB receipt printers ---
        for printer in crate::drivers::usb_printer::UsbReceiptPrinter::discover_all() {
            let info = printer.device_info();
            let id = if info.serial.is_empty() {
                format!("printer:{}:{}", info.vendor, info.model)
            } else {
                format!("printer:{}", info.serial)
            };
            self.register_printer(&id, Arc::new(printer)).await;
        }

        // --- Bluetooth (SPP) receipt printers ---
        let bt_ports = crate::transport::serial::probe_bluetooth().unwrap_or_default();
        for port_info in bt_ports {
            let info = DeviceInfo::new("bluetooth", &port_info.description, &port_info.port_name);
            let printer = crate::drivers::bt_printer::BtReceiptPrinter::new(
                &port_info.port_name,
                9600,
                info,
            );
            let id = format!("printer:bt:{}", port_info.port_name);
            self.register_printer(&id, Arc::new(printer)).await;
        }
    }

    /// Register a TCP (network) printer under the given id. This is not
    /// auto-discovered; the setup wizard calls this when the user
    /// configures a printer by IP address or hostname.
    pub async fn register_tcp_printer(
        &self,
        id: &str,
        addr: &str,
        info: DeviceInfo,
    ) {
        let printer = crate::drivers::tcp_printer::TcpReceiptPrinter::new(addr, info);
        self.register_printer(id, Arc::new(printer)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::mock::MockBarcodeScanner;
    use crate::drivers::mock::MockCashDrawer;
    use crate::drivers::mock::MockReceiptPrinter;
    use crate::types::DeviceInfo;

    #[tokio::test]
    async fn register_and_lookup_scanner() {
        let reg = DriverRegistry::default();
        let scanner: Arc<dyn BarcodeScanner> = Arc::new(MockBarcodeScanner::with_info(
            DeviceInfo::new("test", "MockScanner", "0001"),
        ));
        reg.register_scanner("front", scanner).await;
        let got = reg.scanner("front").await.unwrap();
        assert_eq!(got.device_info().vendor, "test");
    }

    #[tokio::test]
    async fn missing_scanner_returns_none() {
        let reg = DriverRegistry::default();
        assert!(reg.scanner("nope").await.is_none());
    }

    #[tokio::test]
    async fn register_printer_and_drawer() {
        let reg = DriverRegistry::default();
        let printer: Arc<dyn ReceiptPrinter> = Arc::new(MockReceiptPrinter::with_info(
            DeviceInfo::new("test", "MockPrinter", "0002"),
        ));
        let drawer: Arc<dyn CashDrawer> = Arc::new(MockCashDrawer::with_info(DeviceInfo::new(
            "test",
            "MockDrawer",
            "0003",
        )));
        reg.register_printer("default", printer).await;
        reg.register_cash_drawer("default", drawer).await;
        assert!(reg.printer("default").await.is_some());
        assert!(reg.cash_drawer("default").await.is_some());
    }
}

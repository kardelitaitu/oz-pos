/*
last audited 25-07-26 by RSA-Agent (oz-hal slice A: registry deep read)
crate: oz-hal | status: SAFE | lint: CLEAN
findings: clean — per-category RwLock maps with documented overwrite semantics; discovery fail-open per driver (one failure never aborts the rest); deterministic device-id scheme with serial/model fallback; companion cash-drawer registration for every printer; register_mock_scale expect is a documented startup-only invariant
next: none | perf: short-lived read locks on lookup
*/
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

use crate::drivers::drawer::PrinterKickCashDrawer;
use crate::traits::barcode::BarcodeScanner;
use crate::traits::cash_drawer::CashDrawer;
use crate::traits::customer_display::CustomerDisplay;
use crate::traits::printer::ReceiptPrinter;
use crate::traits::weight_scale::WeightScale;
use crate::types::DeviceInfo;

/// Shared, mutable catalogue of HAL drivers.
#[derive(Default)]
pub struct DriverRegistry {
    scanners: RwLock<HashMap<String, Arc<dyn BarcodeScanner>>>,
    printers: RwLock<HashMap<String, Arc<dyn ReceiptPrinter>>>,
    drawers: RwLock<HashMap<String, Arc<dyn CashDrawer>>>,
    displays: RwLock<HashMap<String, Arc<dyn CustomerDisplay>>>,
    scales: RwLock<HashMap<String, Arc<dyn WeightScale>>>,
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

    /// Register a customer display under `id`. Overwrites any previous
    /// entry with the same id.
    pub async fn register_display(&self, id: &str, driver: Arc<dyn CustomerDisplay>) {
        self.displays.write().await.insert(id.to_owned(), driver);
    }

    /// Look up a customer display by id. Returns `None` if no display is registered.
    pub async fn display(&self, id: &str) -> Option<Arc<dyn CustomerDisplay>> {
        self.displays.read().await.get(id).cloned()
    }

    /// Snapshot of registered cash drawer ids.
    pub async fn drawer_ids(&self) -> Vec<String> {
        self.drawers.read().await.keys().cloned().collect()
    }

    /// Snapshot of registered customer display ids.
    pub async fn display_ids(&self) -> Vec<String> {
        self.displays.read().await.keys().cloned().collect()
    }

    /// Register a weight scale under `id`. Overwrites any previous
    /// entry with the same id.
    pub async fn register_scale(&self, id: &str, driver: Arc<dyn WeightScale>) {
        self.scales.write().await.insert(id.to_owned(), driver);
    }

    /// Look up a weight scale by id. Returns `None` if no scale is registered.
    pub async fn scale(&self, id: &str) -> Option<Arc<dyn WeightScale>> {
        self.scales.read().await.get(id).cloned()
    }

    /// Snapshot of registered scale ids.
    pub async fn scale_ids(&self) -> Vec<String> {
        self.scales.read().await.keys().cloned().collect()
    }

    /// Register a mock weight scale under `id` for testing.
    pub fn register_mock_scale(&self, id: &str) {
        let mock = Arc::new(crate::drivers::mock::MockWeightScale::new());
        // Synchronous insert — only used at startup/test time.
        self.scales
            .try_write()
            // SAFETY: synchronous startup/test-time insert — `try_write` only fails if a concurrent writer holds the lock.
            .expect("register_mock_scale called concurrently")
            .insert(id.to_owned(), mock);
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

        // --- USB receipt printers (and companion cash drawers) ---
        for printer in crate::drivers::usb_printer::UsbReceiptPrinter::discover_all() {
            let info = printer.device_info();
            let id = if info.serial.is_empty() {
                format!("printer:{}:{}", info.vendor, info.model)
            } else {
                format!("printer:{}", info.serial)
            };
            let printer_arc = Arc::new(printer);
            self.register_printer(&id, printer_arc.clone()).await;
            // Register a companion cash drawer that kicks through this printer.
            let drawer_id = format!("drawer:kick:{}", id);
            let drawer = Arc::new(PrinterKickCashDrawer::new_pin2(printer_arc));
            self.register_cash_drawer(&drawer_id, drawer).await;
        }

        // --- Bluetooth (SPP) barcode scanners ---
        for scanner in crate::drivers::bt_scanner::BtBarcodeScanner::discover_all() {
            let info = scanner.device_info();
            let id = format!("scanner:bt:{}", info.serial);
            self.register_scanner(&id, Arc::new(scanner)).await;
        }

        // --- Serial customer-facing pole displays ---
        for display in crate::drivers::serial_display::SerialCustomerDisplay::discover_all() {
            let info = display.device_info();
            let id = format!("display:serial:{}", info.serial);
            self.register_display(&id, Arc::new(display)).await;
        }

        // --- Bluetooth (SPP) receipt printers (and companion cash drawers) ---
        let bt_ports = crate::transport::serial::probe_bluetooth().unwrap_or_default();
        for port_info in bt_ports {
            let info = DeviceInfo::new("bluetooth", &port_info.description, &port_info.port_name);
            let printer =
                crate::drivers::bt_printer::BtReceiptPrinter::new(&port_info.port_name, 9600, info);
            let id = format!("printer:bt:{}", port_info.port_name);
            let printer_arc = Arc::new(printer);
            self.register_printer(&id, printer_arc.clone()).await;
            // Companion drawer for BT printers.
            let drawer_id = format!("drawer:kick:{}", id);
            let drawer = Arc::new(PrinterKickCashDrawer::new_pin2(printer_arc));
            self.register_cash_drawer(&drawer_id, drawer).await;
        }
    }

    /// Register a TCP (network) printer under the given id. Also registers
    /// a companion cash drawer that kicks through this printer. This is
    /// not auto-discovered; the setup wizard calls this when the user
    /// configures a printer by IP address or hostname.
    pub async fn register_tcp_printer(&self, id: &str, addr: &str, info: DeviceInfo) {
        let printer_arc = Arc::new(crate::drivers::tcp_printer::TcpReceiptPrinter::new(
            addr, info,
        ));
        self.register_printer(id, printer_arc.clone()).await;
        // Companion drawer for TCP printer.
        let drawer_id = format!("drawer:kick:{id}");
        let drawer = Arc::new(PrinterKickCashDrawer::new_pin2(printer_arc));
        self.register_cash_drawer(&drawer_id, drawer).await;
    }

    /// Register a serial customer display under the given id. The setup
    /// wizard calls this when the user configures a pole display by port name.
    pub async fn register_serial_display(&self, id: &str, port_name: &str, info: DeviceInfo) {
        let display = Arc::new(crate::drivers::serial_display::SerialCustomerDisplay::new(
            port_name,
            crate::drivers::serial_display::DISPLAY_DEFAULT_BAUD,
            info,
        ));
        self.register_display(id, display).await;
    }

    /// Register a serial cash drawer under the given id. The setup wizard
    /// calls this when the user configures a standalone drawer by port name.
    pub async fn register_serial_drawer(&self, id: &str, port_name: &str, info: DeviceInfo) {
        let drawer = Arc::new(crate::drivers::drawer::SerialCashDrawer::new(
            port_name, 9600, info,
        ));
        self.register_cash_drawer(id, drawer).await;
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;


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

#[tokio::test]
async fn register_overwrites_previous() {
    let reg = DriverRegistry::default();
    let old: Arc<dyn BarcodeScanner> = Arc::new(MockBarcodeScanner::with_info(
        DeviceInfo::new("v1", "MockScanner", "0001"),
    ));
    let new: Arc<dyn BarcodeScanner> = Arc::new(MockBarcodeScanner::with_info(
        DeviceInfo::new("v2", "MockScanner", "0002"),
    ));
    reg.register_scanner("main", old).await;
    reg.register_scanner("main", new).await;
    let got = reg.scanner("main").await.unwrap();
    assert_eq!(got.device_info().vendor, "v2");
}

#[tokio::test]
async fn scanner_ids_returns_registered_keys() {
    let reg = DriverRegistry::default();
    let s1: Arc<dyn BarcodeScanner> = Arc::new(MockBarcodeScanner::with_info(DeviceInfo::new(
        "t", "S1", "001",
    )));
    let s2: Arc<dyn BarcodeScanner> = Arc::new(MockBarcodeScanner::with_info(DeviceInfo::new(
        "t", "S2", "002",
    )));
    reg.register_scanner("front", s1).await;
    reg.register_scanner("back", s2).await;
    let ids = reg.scanner_ids().await;
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"front".to_owned()));
    assert!(ids.contains(&"back".to_owned()));
}

#[tokio::test]
async fn printer_ids_returns_registered_keys() {
    let reg = DriverRegistry::default();
    let p: Arc<dyn ReceiptPrinter> = Arc::new(MockReceiptPrinter::with_info(DeviceInfo::new(
        "t", "P", "001",
    )));
    reg.register_printer("default", p).await;
    let ids = reg.printer_ids().await;
    assert_eq!(ids, vec!["default".to_owned()]);
}

#[tokio::test]
async fn drawer_ids_returns_registered_keys() {
    let reg = DriverRegistry::default();
    let d: Arc<dyn CashDrawer> =
        Arc::new(MockCashDrawer::with_info(DeviceInfo::new("t", "D", "001")));
    reg.register_cash_drawer("main", d).await;
    let ids = reg.drawer_ids().await;
    assert_eq!(ids, vec!["main".to_owned()]);
}

#[tokio::test]
async fn empty_registry_ids_are_empty() {
    let reg = DriverRegistry::default();
    assert!(reg.scanner_ids().await.is_empty());
    assert!(reg.printer_ids().await.is_empty());
    assert!(reg.drawer_ids().await.is_empty());
}

#[tokio::test]
async fn register_tcp_printer_and_lookup() {
    let reg = DriverRegistry::default();
    reg.register_tcp_printer(
        "net-printer",
        "192.168.1.100:9100",
        DeviceInfo::new("epson", "TM-T88", "net-001"),
    )
    .await;
    let got = reg.printer("net-printer").await;
    assert!(got.is_some());
    assert_eq!(got.unwrap().device_info().vendor, "epson");
}

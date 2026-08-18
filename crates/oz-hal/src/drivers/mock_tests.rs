
use super::*;

#[tokio::test]
async fn barcode_mock_returns_pushed_codes() {
    let m = MockBarcodeScanner::new();
    m.push(Barcode::new("ABC"));
    m.push(Barcode::new("DEF"));
    let mut dyn_scanner: Box<dyn BarcodeScanner> = m.connect().await.unwrap();
    assert_eq!(dyn_scanner.poll(0).await.unwrap().unwrap().code, "ABC");
    assert_eq!(dyn_scanner.poll(0).await.unwrap().unwrap().code, "DEF");
    assert!(dyn_scanner.poll(0).await.unwrap().is_none());
}

#[tokio::test]
async fn receipt_mock_captures_bodies() {
    let p = MockReceiptPrinter::new();
    p.print_receipt("hello\n").await.unwrap();
    p.print_receipt("world\n").await.unwrap();
    assert_eq!(p.printed.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn drawer_mock_counts_opens() {
    let d = MockCashDrawer::new();
    d.open().await.unwrap();
    d.open().await.unwrap();
    assert_eq!(d.open_calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn printer_returns_programmed_error() {
    let p = MockReceiptPrinter::new();
    p.set_next_error(HalError::Disconnected);
    assert!(matches!(
        p.print_receipt("x").await,
        Err(HalError::Disconnected)
    ));
    // After the error is consumed, subsequent calls succeed.
    p.print_receipt("y").await.unwrap();
}

#[tokio::test]
async fn barcode_mock_queue_len() {
    let m = MockBarcodeScanner::new();
    assert_eq!(m.queue_len(), 0);
    m.push(Barcode::new("A"));
    m.push(Barcode::new("B"));
    assert_eq!(m.queue_len(), 2);
    // Poll consumes one.
    let mut dyn_scanner: Box<dyn BarcodeScanner> = m.connect().await.unwrap();
    dyn_scanner.poll(0).await.unwrap();
    assert_eq!(m.queue_len(), 1);
}

#[tokio::test]
async fn barcode_mock_cancel() {
    let m = MockBarcodeScanner::new();
    let dyn_scanner = m.connect().await.unwrap();
    dyn_scanner.cancel().await.unwrap();
    assert_eq!(m.cancel_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn printer_mock_captures_raw_bytes() {
    let p = MockReceiptPrinter::new();
    p.print_raw(b"\x1b@\x0a").await.unwrap();
    p.print_raw(b"hello").await.unwrap();
    assert_eq!(p.printed_raw.lock().unwrap().len(), 2);
    assert_eq!(p.printed_raw.lock().unwrap()[0], b"\x1b@\x0a");
}

#[tokio::test]
async fn printer_mock_error_affects_raw_too() {
    let p = MockReceiptPrinter::new();
    p.set_next_error(HalError::Busy);
    assert!(matches!(p.print_raw(b"x").await, Err(HalError::Busy)));
    // Error consumed, next call succeeds.
    p.print_raw(b"y").await.unwrap();
}

#[tokio::test]
async fn drawer_error_is_returned() {
    let d = MockCashDrawer::new();
    d.set_next_error(HalError::Timeout(100));
    assert!(matches!(d.open().await, Err(HalError::Timeout(100))));
    // After error is consumed, subsequent open succeeds.
    d.open().await.unwrap();
    assert_eq!(d.open_calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn drawer_is_open_defaults_to_disconnected() {
    // When no response is programmed, is_open returns Disconnected.
    let d = MockCashDrawer::new();
    let result = d.is_open().await;
    assert!(matches!(result, Err(HalError::Disconnected)));
}

#[tokio::test]
async fn drawer_is_open_programmable_closed() {
    let d = MockCashDrawer::new();
    d.set_is_open(Some(Ok(false)));
    assert!(!d.is_open().await.unwrap());
}

#[tokio::test]
async fn drawer_is_open_programmable_open() {
    let d = MockCashDrawer::new();
    d.set_is_open(Some(Ok(true)));
    assert!(d.is_open().await.unwrap());
}

#[tokio::test]
async fn drawer_is_open_programmable_error() {
    let d = MockCashDrawer::new();
    d.set_is_open(Some(Err(HalError::Timeout(50))));
    assert!(matches!(d.is_open().await, Err(HalError::Timeout(50))));
}

#[tokio::test]
async fn drawer_is_open_reverts_to_disconnected() {
    let d = MockCashDrawer::new();
    d.set_is_open(Some(Ok(true)));
    assert!(d.is_open().await.unwrap());
    // Revert to default.
    d.set_is_open(None);
    assert!(matches!(d.is_open().await, Err(HalError::Disconnected)));
}

#[tokio::test]
async fn mock_get_status_returns_default_ok() {
    let p = MockReceiptPrinter::new();
    let status = p.get_status().await.unwrap();
    assert_eq!(status.paper, PaperStatus::Ok);
    assert!(!status.cover_open);
    assert!(!status.drawer_open);
    assert!(status.is_ready());
    assert!(!status.has_fault());
}

#[tokio::test]
async fn mock_get_status_programmable() {
    let p = MockReceiptPrinter::new();
    p.set_status(PrinterStatus {
        paper: PaperStatus::Low,
        cover_open: false,
        drawer_open: true,
    });
    let status = p.get_status().await.unwrap();
    assert_eq!(status.paper, PaperStatus::Low);
    assert!(!status.cover_open);
    assert!(status.drawer_open);
}

#[tokio::test]
async fn mock_status_fault_detection() {
    let p = MockReceiptPrinter::new();
    // Empty paper + cover open = fault
    p.set_status(PrinterStatus {
        paper: PaperStatus::Empty,
        cover_open: true,
        drawer_open: false,
    });
    let status = p.get_status().await.unwrap();
    assert!(status.has_fault());
    assert!(!status.is_ready());
}

#[tokio::test]
async fn mock_status_is_ready_requires_ok_paper_and_closed_cover() {
    let p = MockReceiptPrinter::new();
    // Low paper + closed cover = should still be NOT ready
    p.set_status(PrinterStatus {
        paper: PaperStatus::Low,
        cover_open: false,
        drawer_open: false,
    });
    assert!(!p.get_status().await.unwrap().is_ready());
}

#[tokio::test]
async fn printer_mock_cut_counts_calls() {
    let p = MockReceiptPrinter::new();
    p.cut().await.unwrap();
    assert_eq!(p.cut_calls.load(Ordering::SeqCst), 1);
}

// ── Weight scale mock tests ──────────────────────────────────

#[test]
fn scale_mock_read_weight_returns_stable_zero() {
    let s = MockWeightScale::new();
    let reading = s.read_weight().unwrap();
    assert_eq!(reading.weight_grams, 0.0);
    assert!(reading.stable);
}

#[test]
fn scale_mock_counts_read_calls() {
    let s = MockWeightScale::new();
    s.read_weight().unwrap();
    s.read_weight().unwrap();
    s.read_weight().unwrap();
    assert_eq!(s.read_calls.load(Ordering::SeqCst), 3);
}

#[test]
fn scale_mock_device_info_is_accessible() {
    let s = MockWeightScale::new();
    let info = s.device_info();
    assert!(!info.vendor.is_empty());
    assert!(!info.model.is_empty());
}

#[test]
fn scale_mock_custom_device_info() {
    let info = DeviceInfo::new("acme", "ACME-SCALE", "usb-001");
    let s = MockWeightScale::with_info(info.clone());
    assert_eq!(s.device_info().vendor, info.vendor);
    assert_eq!(s.device_info().model, info.model);
}

#[test]
fn scale_mock_default_implements_weight_scale_trait() {
    // Verify the mock can be used as a trait object through the trait.
    fn accept_scale(_s: &dyn WeightScale) {}
    let s = MockWeightScale::new();
    accept_scale(&s);
}

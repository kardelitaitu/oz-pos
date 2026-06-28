//! Mock implementations of every HAL trait.
//!
//! Mocks are **stateful** and **programmable**: tests push inputs, then
//! assert what the rest of the system did with them. Call counters
//! (`connect_calls`, `poll_calls`, …) make assertions trivial.
//!
//! Mocks implement the same trait as the real driver — no special
//! "mock mode" in business code.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::error::HalError;
use crate::traits::barcode::BarcodeScanner;
use crate::traits::cash_drawer::CashDrawer;
use crate::traits::printer::ReceiptPrinter;
use crate::types::{Barcode, DeviceInfo};

// --- Barcode scanner mock -----------------------------------------------

/// Programmable mock for `BarcodeScanner`. Tests push scans into the
/// queue; the mock returns them in order.
#[derive(Clone)]
pub struct MockBarcodeScanner {
    queue: Arc<Mutex<VecDeque<Barcode>>>,
    pub connect_calls: Arc<AtomicUsize>,
    pub poll_calls: Arc<AtomicUsize>,
    pub cancel_calls: Arc<AtomicUsize>,
    pub info: DeviceInfo,
}

impl MockBarcodeScanner {
    /// Construct a mock with default identity `("mock", "MockBarcode", "0000")`.
    #[must_use]
    pub fn new() -> Self {
        Self::with_info(DeviceInfo::new("mock", "MockBarcode", "0000"))
    }

    /// Construct a mock with custom identity.
    #[must_use]
    pub fn with_info(info: DeviceInfo) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            connect_calls: Arc::new(AtomicUsize::new(0)),
            poll_calls: Arc::new(AtomicUsize::new(0)),
            cancel_calls: Arc::new(AtomicUsize::new(0)),
            info,
        }
    }

    /// Queue a barcode to be returned by the next `poll`.
    pub fn push(&self, code: Barcode) {
        self.queue
            .lock()
            .expect("mock queue poisoned")
            .push_back(code);
    }

    /// Number of queued scans (for assertions).
    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.queue.lock().expect("mock queue poisoned").len()
    }
}

impl Default for MockBarcodeScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BarcodeScanner for MockBarcodeScanner {
    async fn connect(&self) -> Result<Box<dyn BarcodeScanner>, HalError> {
        self.connect_calls.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(self.clone()))
    }

    async fn poll(&mut self, timeout_ms: u32) -> Result<Option<Barcode>, HalError> {
        self.poll_calls.fetch_add(1, Ordering::SeqCst);
        // Honour timeout by short-circuiting when the queue is empty.
        // A real driver would block on a USB/BT channel.
        if self.queue.lock().expect("mock queue poisoned").is_empty() {
            if timeout_ms == 0 {
                return Ok(None);
            }
            let sleep_ms = u64::from(timeout_ms.min(50));
            tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
        }
        Ok(self.queue.lock().expect("mock queue poisoned").pop_front())
    }

    async fn cancel(&self) -> Result<(), HalError> {
        self.cancel_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

// --- Receipt printer mock -----------------------------------------------

/// Programmable mock for `ReceiptPrinter`. Captures every printed body
/// so tests can assert what the system tried to print.
#[derive(Clone)]
pub struct MockReceiptPrinter {
    pub printed: Arc<Mutex<Vec<String>>>,
    /// Captured raw bytes from `print_raw` calls.
    pub printed_raw: Arc<Mutex<Vec<Vec<u8>>>>,
    pub cut_calls: Arc<AtomicUsize>,
    pub info: DeviceInfo,
    /// If set, every `print_receipt` returns this error instead of Ok.
    pub fail_with: Arc<Mutex<Option<HalError>>>,
}

impl MockReceiptPrinter {
    #[must_use]
    pub fn new() -> Self {
        Self::with_info(DeviceInfo::new("mock", "MockPrinter", "0000"))
    }

    #[must_use]
    pub fn with_info(info: DeviceInfo) -> Self {
        Self {
            printed: Arc::new(Mutex::new(Vec::new())),
            printed_raw: Arc::new(Mutex::new(Vec::new())),
            cut_calls: Arc::new(AtomicUsize::new(0)),
            info,
            fail_with: Arc::new(Mutex::new(None)),
        }
    }

    /// Program the next `print_receipt` to return `err` (and any
    /// subsequent calls until cleared).
    pub fn set_next_error(&self, err: HalError) {
        *self.fail_with.lock().expect("poisoned") = Some(err);
    }
}

impl Default for MockReceiptPrinter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReceiptPrinter for MockReceiptPrinter {
    async fn print_receipt(&self, body: &str) -> Result<(), HalError> {
        if let Some(err) = self.fail_with.lock().expect("poisoned").take() {
            return Err(err);
        }
        self.printed.lock().expect("poisoned").push(body.to_owned());
        Ok(())
    }

    async fn print_raw(&self, data: &[u8]) -> Result<(), HalError> {
        if let Some(err) = self.fail_with.lock().expect("poisoned").take() {
            return Err(err);
        }
        self.printed_raw.lock().expect("poisoned").push(data.to_vec());
        Ok(())
    }

    async fn cut(&self) -> Result<(), HalError> {
        self.cut_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

// --- Cash drawer mock ---------------------------------------------------

/// Programmable mock for `CashDrawer`. Counts `open` calls; can be
/// programmed to fail.
#[derive(Clone)]
pub struct MockCashDrawer {
    pub open_calls: Arc<AtomicUsize>,
    pub info: DeviceInfo,
    pub fail_with: Arc<Mutex<Option<HalError>>>,
}

impl MockCashDrawer {
    #[must_use]
    pub fn new() -> Self {
        Self::with_info(DeviceInfo::new("mock", "MockDrawer", "0000"))
    }

    #[must_use]
    pub fn with_info(info: DeviceInfo) -> Self {
        Self {
            open_calls: Arc::new(AtomicUsize::new(0)),
            info,
            fail_with: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_next_error(&self, err: HalError) {
        *self.fail_with.lock().expect("poisoned") = Some(err);
    }
}

impl Default for MockCashDrawer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CashDrawer for MockCashDrawer {
    async fn open(&self) -> Result<(), HalError> {
        self.open_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(err) = self.fail_with.lock().expect("poisoned").take() {
            return Err(err);
        }
        Ok(())
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

#[cfg(test)]
mod tests {
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
}

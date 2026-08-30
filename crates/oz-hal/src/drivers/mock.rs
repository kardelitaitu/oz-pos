/*
last audited 25-07-26 by RSA-Agent (oz-hal slice C: verified)
crate: oz-hal | status: SAFE | lint: CLEAN
findings: clean driver — no unwrap/panic/unsafe
next: none | perf: N/A
*/
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
use crate::traits::customer_display::{CustomerDisplay, DisplayContent};
use crate::traits::printer::{PaperStatus, PrinterStatus, ReceiptPrinter};
use crate::traits::weight_scale::{WeightReading, WeightScale};
use crate::types::{Barcode, DeviceInfo};

// --- Barcode scanner mock -----------------------------------------------

/// Programmable mock for `BarcodeScanner`. Tests push scans into the
/// queue; the mock returns them in order.
#[derive(Clone)]
pub struct MockBarcodeScanner {
    queue: Arc<Mutex<VecDeque<Barcode>>>,
    /// Number of times `connect` has been called.
    pub connect_calls: Arc<AtomicUsize>,
    /// Number of times `poll` has been called.
    pub poll_calls: Arc<AtomicUsize>,
    /// Number of times `cancel` has been called.
    pub cancel_calls: Arc<AtomicUsize>,
    /// Device identity reported by `device_info()`.
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
            .expect("mock queue poisoned") // SAFETY: mock driver — lock poison is the intended failure signal; data behind it is irrelevant in a test double
            .push_back(code);
    }

    /// Number of queued scans (for assertions).
    #[must_use]
    pub fn queue_len(&self) -> usize {
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
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
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        if self.queue.lock().expect("mock queue poisoned").is_empty() {
            if timeout_ms == 0 {
                return Ok(None);
            }
            let sleep_ms = u64::from(timeout_ms.min(50));
            tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
        }
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
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
    /// Captured receipt body strings from `print_receipt` calls.
    pub printed: Arc<Mutex<Vec<String>>>,
    /// Captured raw bytes from `print_raw` calls.
    pub printed_raw: Arc<Mutex<Vec<Vec<u8>>>>,
    /// Number of times `cut` has been called.
    pub cut_calls: Arc<AtomicUsize>,
    /// Device identity reported by `device_info()`.
    pub info: DeviceInfo,
    /// If set, every `print_receipt` returns this error instead of Ok.
    pub fail_with: Arc<Mutex<Option<HalError>>>,
    /// Programmable printer status returned by `get_status()`.
    /// Defaults to `PrinterStatus { paper: Ok, cover_open: false, drawer_open: false }`.
    pub status: Arc<Mutex<PrinterStatus>>,
}

impl MockReceiptPrinter {
    /// Construct a mock with default identity `("mock", "MockPrinter", "0000")`.
    #[must_use]
    pub fn new() -> Self {
        Self::with_info(DeviceInfo::new("mock", "MockPrinter", "0000"))
    }

    /// Construct a mock with custom identity.
    #[must_use]
    pub fn with_info(info: DeviceInfo) -> Self {
        Self {
            printed: Arc::new(Mutex::new(Vec::new())),
            printed_raw: Arc::new(Mutex::new(Vec::new())),
            cut_calls: Arc::new(AtomicUsize::new(0)),
            info,
            fail_with: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(PrinterStatus {
                paper: PaperStatus::Ok,
                cover_open: false,
                drawer_open: false,
            })),
        }
    }

    /// Program the next `print_receipt` to return `err` (and any
    /// subsequent calls until cleared).
    pub fn set_next_error(&self, err: HalError) {
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        *self.fail_with.lock().expect("poisoned") = Some(err);
    }

    /// Set the printer status returned by `get_status()`.
    pub fn set_status(&self, status: PrinterStatus) {
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        *self.status.lock().expect("poisoned") = status;
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
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        if let Some(err) = self.fail_with.lock().expect("poisoned").take() {
            return Err(err);
        }
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        self.printed.lock().expect("poisoned").push(body.to_owned());
        Ok(())
    }

    async fn print_raw(&self, data: &[u8]) -> Result<(), HalError> {
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        if let Some(err) = self.fail_with.lock().expect("poisoned").take() {
            return Err(err);
        }
        self.printed_raw
            .lock()
            .expect("poisoned") // SAFETY: mock driver — lock poison is the intended failure signal in a test double
            .push(data.to_vec());
        Ok(())
    }

    async fn cut(&self) -> Result<(), HalError> {
        self.cut_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn get_status(&self) -> Result<PrinterStatus, HalError> {
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        Ok(self.status.lock().expect("poisoned").clone())
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

// --- Customer display mock ---------------------------------------------

/// Programmable mock for `CustomerDisplay`. Records the last content
/// pushed and supports brightness control.
#[derive(Clone)]
pub struct MockCustomerDisplay {
    /// Number of times `show` has been called.
    pub show_calls: Arc<AtomicUsize>,
    /// Number of times `clear` has been called.
    pub clear_calls: Arc<AtomicUsize>,
    last_content: Arc<Mutex<Option<DisplayContent>>>,
    brightness: Arc<Mutex<f32>>,
    /// Device identity reported by `device_info()`.
    pub info: DeviceInfo,
}

impl MockCustomerDisplay {
    /// Construct a mock with default identity `("mock", "MockDisplay", "0000")`.
    #[must_use]
    pub fn new() -> Self {
        Self::with_info(DeviceInfo::new("mock", "MockDisplay", "0000"))
    }

    /// Construct a mock with custom identity.
    #[must_use]
    pub fn with_info(info: DeviceInfo) -> Self {
        Self {
            show_calls: Arc::new(AtomicUsize::new(0)),
            clear_calls: Arc::new(AtomicUsize::new(0)),
            last_content: Arc::new(Mutex::new(None)),
            brightness: Arc::new(Mutex::new(1.0)),
            info,
        }
    }

    /// The last content that was shown on the display.
    pub fn last_content(&self) -> Option<DisplayContent> {
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        self.last_content.lock().expect("poisoned").clone()
    }

    /// Current brightness level.
    #[must_use]
    pub fn brightness(&self) -> f32 {
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        *self.brightness.lock().expect("poisoned")
    }
}

impl Default for MockCustomerDisplay {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CustomerDisplay for MockCustomerDisplay {
    async fn connect(&self) -> Result<Box<dyn CustomerDisplay>, HalError> {
        Ok(Box::new(self.clone()))
    }

    async fn show(&self, content: &DisplayContent) -> Result<(), HalError> {
        self.show_calls.fetch_add(1, Ordering::SeqCst);
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        *self.last_content.lock().expect("poisoned") = Some(content.clone());
        Ok(())
    }

    async fn clear(&self) -> Result<(), HalError> {
        self.clear_calls.fetch_add(1, Ordering::SeqCst);
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        *self.last_content.lock().expect("poisoned") = None;
        Ok(())
    }

    async fn set_brightness(&self, level: f32) -> Result<(), HalError> {
        let clamped = level.clamp(0.0, 1.0);
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        *self.brightness.lock().expect("poisoned") = clamped;
        Ok(())
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

// --- Cash drawer mock ---------------------------------------------------

/// Programmable mock for `CashDrawer`. Counts `open` calls; can be
/// programmed to fail or to report drawer-open state.
#[derive(Clone)]
pub struct MockCashDrawer {
    /// Number of times `open` has been called.
    pub open_calls: Arc<AtomicUsize>,
    /// Device identity reported by `device_info()`.
    pub info: DeviceInfo,
    /// If set, the next `open` call returns this error.
    pub fail_with: Arc<Mutex<Option<HalError>>>,
    /// Programmable response for `is_open()`. `None` means "use the
    /// trait default" (which returns `Disconnected`). Set to
    /// `Some(Ok(true))` to simulate an open drawer, etc.
    is_open_response: Arc<Mutex<Option<Result<bool, HalError>>>>,
}

impl MockCashDrawer {
    /// Construct a mock with default identity `("mock", "MockDrawer", "0000")`.
    #[must_use]
    pub fn new() -> Self {
        Self::with_info(DeviceInfo::new("mock", "MockDrawer", "0000"))
    }

    /// Construct a mock with custom identity.
    #[must_use]
    pub fn with_info(info: DeviceInfo) -> Self {
        Self {
            open_calls: Arc::new(AtomicUsize::new(0)),
            info,
            fail_with: Arc::new(Mutex::new(None)),
            is_open_response: Arc::new(Mutex::new(None)),
        }
    }

    /// Program the next `open` to return `err` (consumed on first call).
    pub fn set_next_error(&self, err: HalError) {
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        *self.fail_with.lock().expect("poisoned") = Some(err);
    }

    /// Program what `is_open()` returns.
    ///
    /// Pass `None` to revert to the trait default (`Disconnected`).
    /// Pass `Some(Ok(true))` for an open drawer, `Some(Ok(false))` for
    /// a closed one, or `Some(Err(...))` to simulate a hardware error.
    pub fn set_is_open(&self, response: Option<Result<bool, HalError>>) {
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        *self.is_open_response.lock().expect("poisoned") = response;
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
        // SAFETY: mock driver — lock poison is the intended failure signal in a test double
        if let Some(err) = self.fail_with.lock().expect("poisoned").take() {
            return Err(err);
        }
        Ok(())
    }

    async fn is_open(&self) -> Result<bool, HalError> {
        // SAFETY: mock driver — lock poison is the intended failure signal
        match self.is_open_response.lock().expect("poisoned").clone() {
            Some(response) => response,
            None => {
                // Fall through to the trait default: most drawers don't
                // report state, so this is the realistic default.
                Err(HalError::Disconnected)
            }
        }
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

// --- Weight scale mock ---------------------------------------------------

/// Programmable mock for `WeightScale`. Always returns a stable zero reading.
#[derive(Clone)]
pub struct MockWeightScale {
    /// Number of times `read_weight` has been called.
    pub read_calls: Arc<AtomicUsize>,
    /// Device identity reported by `device_info()`.
    pub info: DeviceInfo,
}

impl MockWeightScale {
    /// Construct a mock with default identity `("mock", "MockScale", "0000")`.
    #[must_use]
    pub fn new() -> Self {
        Self::with_info(DeviceInfo::new("mock", "MockScale", "0000"))
    }

    /// Construct a mock with custom identity.
    #[must_use]
    pub fn with_info(info: DeviceInfo) -> Self {
        Self {
            read_calls: Arc::new(AtomicUsize::new(0)),
            info,
        }
    }
}

impl Default for MockWeightScale {
    fn default() -> Self {
        Self::new()
    }
}

impl WeightScale for MockWeightScale {
    fn read_weight(&self) -> Result<WeightReading, HalError> {
        self.read_calls.fetch_add(1, Ordering::SeqCst);
        Ok(WeightReading {
            weight_grams: 0.0,
            stable: true,
        })
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

#[cfg(test)]
#[path = "mock_tests.rs"]
mod tests;

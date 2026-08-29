/*
last audited 25-07-26 by RSA-Agent
crate: oz-payment | status: SAFE | lint: CLEAN
findings: programmable mock fails closed until set_success — good default; SeqCst counters fine for test double
next: none | perf: N/A
*/
//! Mock EDC terminal for testing.
//!
//! Programmable behaviour mirroring [`crate::drivers::mock::MockPaymentProcessor`].
//! Tests configure the mock before passing it to code under test, so the
//! EDC terminal feature can be tested without physical hardware.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use async_trait::async_trait;
use foundation::Money;
use oz_hal::types::DeviceInfo;

use crate::error::PaymentError;
use crate::types::PaymentReceipt;

use super::{EdcTerminal, PaymentResult, TerminalStatus};

/// A programmable mock EDC terminal.
///
/// Usage:
/// ```ignore
/// let terminal = MockEdcTerminal::new();
/// let result = terminal.sale(Money::from_major(10, usd()).unwrap()).await;
/// assert!(result.is_err()); // Unsupported by default — call .set_authorise(true) to enable.
/// ```
pub struct MockEdcTerminal {
    /// Whether the next `authorize` succeeds. Default `false`.
    success: AtomicBool,
    /// Number of times `authorize` has been called.
    authorize_calls: AtomicUsize,
    /// Number of times `capture` has been called.
    capture_calls: AtomicUsize,
    /// Number of times `sale` has been called.
    sale_calls: AtomicUsize,
    /// Number of times `refund` has been called.
    refund_calls: AtomicUsize,
    /// Number of times `void` has been called.
    void_calls: AtomicUsize,
}

impl MockEdcTerminal {
    /// Create a mock that returns [`PaymentError::Unsupported`] until
    /// [`Self::set_success`] is called.
    #[must_use]
    pub fn new() -> Self {
        Self {
            success: AtomicBool::new(false),
            authorize_calls: AtomicUsize::new(0),
            capture_calls: AtomicUsize::new(0),
            sale_calls: AtomicUsize::new(0),
            refund_calls: AtomicUsize::new(0),
            void_calls: AtomicUsize::new(0),
        }
    }

    /// Make the next authorise return success (a mock transaction id).
    pub fn set_success(&self) {
        self.success.store(true, Ordering::SeqCst);
    }

    /// Number of `authorize` calls.
    pub fn authorize_calls(&self) -> usize {
        self.authorize_calls.load(Ordering::SeqCst)
    }
    /// Number of `capture` calls.
    pub fn capture_calls(&self) -> usize {
        self.capture_calls.load(Ordering::SeqCst)
    }
    /// Number of `sale` calls.
    pub fn sale_calls(&self) -> usize {
        self.sale_calls.load(Ordering::SeqCst)
    }
    /// Number of `refund` calls.
    pub fn refund_calls(&self) -> usize {
        self.refund_calls.load(Ordering::SeqCst)
    }
    /// Number of `void` calls.
    pub fn void_calls(&self) -> usize {
        self.void_calls.load(Ordering::SeqCst)
    }
}

impl Default for MockEdcTerminal {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EdcTerminal for MockEdcTerminal {
    async fn status(&self) -> Result<TerminalStatus, PaymentError> {
        if self.success.load(Ordering::SeqCst) {
            Ok(TerminalStatus::Ready)
        } else {
            Err(PaymentError::Unsupported(
                "mock EDC status — not configured".into(),
            ))
        }
    }

    async fn authorize(&self, _amount: Money) -> Result<String, PaymentError> {
        self.authorize_calls.fetch_add(1, Ordering::SeqCst);
        if self.success.load(Ordering::SeqCst) {
            Ok("mock-txn-001".into())
        } else {
            Err(PaymentError::Unsupported(
                "mock EDC authorize — not configured".into(),
            ))
        }
    }

    async fn capture(&self, _transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        self.capture_calls.fetch_add(1, Ordering::SeqCst);
        if self.success.load(Ordering::SeqCst) {
            Ok(PaymentResult {
                success: true,
                transaction_id: Some("mock-txn-001".into()),
                auth_code: Some("MOCKAUTH".into()),
                card_scheme: Some("Visa".into()),
                card_last4: Some("1111".into()),
                message: "approved".into(),
            })
        } else {
            Err(PaymentError::Unsupported(
                "mock EDC capture — not configured".into(),
            ))
        }
    }

    async fn sale(&self, amount: Money) -> Result<PaymentResult, PaymentError> {
        self.sale_calls.fetch_add(1, Ordering::SeqCst);
        let txn_id = self.authorize(amount).await?;
        self.capture(&txn_id).await
    }

    async fn refund(
        &self,
        _transaction_id: &str,
        _amount: Option<Money>,
    ) -> Result<PaymentResult, PaymentError> {
        self.refund_calls.fetch_add(1, Ordering::SeqCst);
        if self.success.load(Ordering::SeqCst) {
            Ok(PaymentResult {
                success: true,
                transaction_id: Some("mock-refund-001".into()),
                auth_code: Some("MOCKREF".into()),
                card_scheme: None,
                card_last4: None,
                message: "refund approved".into(),
            })
        } else {
            Err(PaymentError::Unsupported(
                "mock EDC refund — not configured".into(),
            ))
        }
    }

    async fn void(&self, _transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        self.void_calls.fetch_add(1, Ordering::SeqCst);
        if self.success.load(Ordering::SeqCst) {
            Ok(PaymentResult {
                success: true,
                transaction_id: Some("mock-void-001".into()),
                auth_code: None,
                card_scheme: None,
                card_last4: None,
                message: "void approved".into(),
            })
        } else {
            Err(PaymentError::Unsupported(
                "mock EDC void — not configured".into(),
            ))
        }
    }

    async fn print_receipt(&self, _transaction_id: &str) -> Result<PaymentReceipt, PaymentError> {
        Err(PaymentError::Unsupported(
            "mock EDC print_receipt — not implemented".into(),
        ))
    }

    fn device_info(&self) -> DeviceInfo {
        DeviceInfo::new("MockEDC", "MockEdcTerminal", "mock-edc-001")
    }
}

#[cfg(test)]
#[path = "mock_tests.rs"]
mod tests;

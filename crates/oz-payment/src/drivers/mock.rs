//! Programmable mock for the [`PaymentProcessor`] trait.
//!
//! Use in unit tests to simulate approvals, declines, and network
//! errors without touching any payment gateway.

use async_trait::async_trait;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use foundation::{Currency, Money};
use oz_hal::types::DeviceInfo;

use crate::PaymentProcessor;
use crate::error::PaymentError;
use crate::types::{PaymentMethod, PaymentReceipt, PaymentRequest, PaymentResult};

/// A builder for [`MockPaymentProcessor`].
///
/// # Example
///
/// ```
/// # use oz_payment::drivers::mock::MockPaymentProcessor;
/// let proc = MockPaymentProcessor::builder()
///     .decline_next(true)
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct MockPaymentProcessorBuilder {
    decline_next: bool,
    simulate_timeout: bool,
}

impl MockPaymentProcessorBuilder {
    /// If `true`, the next `authorize` call will return `Declined`.
    pub fn decline_next(mut self, decline: bool) -> Self {
        self.decline_next = decline;
        self
    }

    /// If `true`, the next `authorize` call will return `Timeout`.
    pub fn simulate_timeout(mut self, timeout: bool) -> Self {
        self.simulate_timeout = timeout;
        self
    }

    /// Build the [`MockPaymentProcessor`].
    pub fn build(self) -> MockPaymentProcessor {
        MockPaymentProcessor {
            authorize_calls: AtomicUsize::new(0),
            capture_calls: AtomicUsize::new(0),
            refund_calls: AtomicUsize::new(0),
            void_calls: AtomicUsize::new(0),
            receipt_calls: AtomicUsize::new(0),
            decline_next: Mutex::new(self.decline_next),
            simulate_timeout: Mutex::new(self.simulate_timeout),
        }
    }
}

/// A programmable mock payment processor for testing.
///
/// Tracks call counts for every method. Can be configured to simulate
/// declines and timeouts via [`MockPaymentProcessor::builder`].
#[derive(Debug)]
pub struct MockPaymentProcessor {
    authorize_calls: AtomicUsize,
    capture_calls: AtomicUsize,
    refund_calls: AtomicUsize,
    void_calls: AtomicUsize,
    receipt_calls: AtomicUsize,
    decline_next: Mutex<bool>,
    simulate_timeout: Mutex<bool>,
}

impl MockPaymentProcessor {
    /// Create a new `MockPaymentProcessor` that approves every request.
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Obtain a builder for configuring mock behaviour.
    pub fn builder() -> MockPaymentProcessorBuilder {
        MockPaymentProcessorBuilder::default()
    }

    /// Number of times `authorize` was called.
    pub fn authorize_calls(&self) -> usize {
        self.authorize_calls.load(Ordering::Relaxed)
    }

    /// Number of times `capture` was called.
    pub fn capture_calls(&self) -> usize {
        self.capture_calls.load(Ordering::Relaxed)
    }

    /// Number of times `refund` was called.
    pub fn refund_calls(&self) -> usize {
        self.refund_calls.load(Ordering::Relaxed)
    }

    /// Number of times `void` was called.
    pub fn void_calls(&self) -> usize {
        self.void_calls.load(Ordering::Relaxed)
    }

    fn check_decline(&self) -> Result<(), PaymentError> {
        let mut decline = self.decline_next.lock().unwrap();
        if *decline {
            *decline = false; // one-shot
            return Err(PaymentError::Declined("mock decline".into()));
        }
        Ok(())
    }

    fn check_timeout(&self) -> Result<(), PaymentError> {
        let mut timeout = self.simulate_timeout.lock().unwrap();
        if *timeout {
            *timeout = false;
            return Err(PaymentError::Timeout(5000));
        }
        Ok(())
    }
}

impl Default for MockPaymentProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PaymentProcessor for MockPaymentProcessor {
    async fn authorize(&self, request: &PaymentRequest) -> Result<PaymentResult, PaymentError> {
        self.authorize_calls.fetch_add(1, Ordering::Relaxed);
        self.check_timeout()?;
        self.check_decline()?;

        Ok(PaymentResult {
            success: true,
            transaction_id: Some(format!("mock_txn_{:09}", 1)),
            auth_code: Some("MOCKAUTH".into()),
            amount_charged: request.amount,
            message: Some("approved".into()),
        })
    }

    async fn capture(&self, _transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        self.capture_calls.fetch_add(1, Ordering::Relaxed);
        self.check_timeout()?;

        Ok(PaymentResult {
            success: true,
            transaction_id: Some("mock_capture_001".into()),
            auth_code: Some("MOCKCAPTURE".into()),
            amount_charged: Money::zero(Currency(*b"USD")),
            message: Some("captured".into()),
        })
    }

    async fn sale(&self, request: &PaymentRequest) -> Result<PaymentResult, PaymentError> {
        // Sale uses the default trait implementation which calls
        // authorize + capture. We override here for atomic mock
        // behaviour (decline/timeout checked once, not twice).
        self.authorize_calls.fetch_add(1, Ordering::Relaxed);
        self.check_timeout()?;
        self.check_decline()?;

        self.capture_calls.fetch_add(1, Ordering::Relaxed);

        Ok(PaymentResult {
            success: true,
            transaction_id: Some(format!("mock_sale_{:09}", 1)),
            auth_code: Some("MOCKSALE".into()),
            amount_charged: request.amount,
            message: Some("approved".into()),
        })
    }

    async fn refund(
        &self,
        _transaction_id: &str,
        _amount: Option<foundation::Money>,
    ) -> Result<PaymentResult, PaymentError> {
        self.refund_calls.fetch_add(1, Ordering::Relaxed);

        Ok(PaymentResult {
            success: true,
            transaction_id: Some("mock_refund_001".into()),
            auth_code: None,
            amount_charged: Money::zero(Currency(*b"USD")),
            message: Some("refunded".into()),
        })
    }

    async fn void(&self, _transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        self.void_calls.fetch_add(1, Ordering::Relaxed);

        Ok(PaymentResult {
            success: true,
            transaction_id: Some("mock_void_001".into()),
            auth_code: None,
            amount_charged: Money::zero(Currency(*b"USD")),
            message: Some("voided".into()),
        })
    }

    async fn receipt(&self, transaction_id: &str) -> Result<PaymentReceipt, PaymentError> {
        self.receipt_calls.fetch_add(1, Ordering::Relaxed);

        Ok(PaymentReceipt {
            transaction_id: transaction_id.to_owned(),
            method: PaymentMethod::Card,
            amount: Money::zero(Currency(*b"USD")),
            timestamp: "2026-06-30T12:00:00Z".into(),
            raw_data: None,
        })
    }

    fn device_info(&self) -> DeviceInfo {
        DeviceInfo::new("OZ-POS", "Mock Payment Processor", "0000-0000")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::Currency;

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    fn make_req() -> PaymentRequest {
        PaymentRequest {
            amount: Money::from_major(10, usd()).unwrap(),
            reference: None,
            description: None,
            idempotency_key: None,
        }
    }

    #[tokio::test]
    async fn mock_approves_by_default() {
        let p = MockPaymentProcessor::new();
        let result = p.authorize(&make_req()).await.unwrap();
        assert!(result.success);
        assert_eq!(p.authorize_calls(), 1);
    }

    #[tokio::test]
    async fn mock_decline() {
        let p = MockPaymentProcessor::builder().decline_next(true).build();
        let result = p.authorize(&make_req()).await;
        assert!(matches!(result, Err(PaymentError::Declined(_))));
    }

    #[tokio::test]
    async fn mock_decline_is_one_shot() {
        let p = MockPaymentProcessor::builder().decline_next(true).build();
        // First call is declined.
        assert!(p.authorize(&make_req()).await.is_err());
        // Second call is approved.
        let result = p.authorize(&make_req()).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn mock_timeout() {
        let p = MockPaymentProcessor::builder()
            .simulate_timeout(true)
            .build();
        let result = p.authorize(&make_req()).await;
        assert!(matches!(result, Err(PaymentError::Timeout(5000))));
    }

    #[tokio::test]
    async fn mock_sale_approves() {
        let p = MockPaymentProcessor::new();
        let result = p.sale(&make_req()).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn mock_sale_declines() {
        let p = MockPaymentProcessor::builder().decline_next(true).build();
        let result = p.sale(&make_req()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_refund() {
        let p = MockPaymentProcessor::new();
        let result = p.refund("txn_001", None).await.unwrap();
        assert!(result.success);
        assert_eq!(p.refund_calls(), 1);
    }

    #[tokio::test]
    async fn mock_void() {
        let p = MockPaymentProcessor::new();
        let result = p.void("txn_001").await.unwrap();
        assert!(result.success);
        assert_eq!(p.void_calls(), 1);
    }

    #[tokio::test]
    async fn mock_receipt() {
        let p = MockPaymentProcessor::new();
        let receipt = p.receipt("txn_001").await.unwrap();
        assert_eq!(receipt.transaction_id, "txn_001");
    }

    #[tokio::test]
    async fn mock_device_info() {
        let p = MockPaymentProcessor::new();
        let info = p.device_info();
        assert_eq!(info.vendor, "OZ-POS");
    }

    #[tokio::test]
    async fn mock_tracks_calls() {
        let p = MockPaymentProcessor::new();
        p.authorize(&make_req()).await.unwrap();
        p.authorize(&make_req()).await.unwrap();
        p.authorize(&make_req()).await.unwrap();
        assert_eq!(p.authorize_calls(), 3);
    }
}

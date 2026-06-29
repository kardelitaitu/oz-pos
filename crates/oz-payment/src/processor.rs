//! [`PaymentProcessor`] trait — the interface every payment gateway
//! (Stripe, Square, EMV terminal) implements.
//!
//! # Lifecycle
//!
//! ```text
//! authorize(request)
//!     │
//!     ▼
//!   success? ──no──→ void(authorization)
//!     │
//!    yes
//!     │
//!     ▼
//!   capture(transaction_id)
//!     │
//!     ▼
//!   success? ──no──→ (manual reconciliation)
//!     │
//!    yes
//!     │
//!     ▼
//!   refund(transaction_id, amount)  ←── optional later
//! ```

use async_trait::async_trait;

use crate::error::PaymentError;
use crate::types::{PaymentReceipt, PaymentRequest, PaymentResult};
use oz_hal::types::DeviceInfo;

/// A processor that can authorize, capture, refund, and void payments.
///
/// Every method is async so that network calls or hardware I/O never
/// block the main thread.
#[async_trait]
pub trait PaymentProcessor: Send + Sync {
    /// Authorize a payment (hold funds without capturing them yet).
    ///
    /// Returns a [`PaymentResult`] with a `transaction_id` on success.
    async fn authorize(&self, request: &PaymentRequest) -> Result<PaymentResult, PaymentError>;

    /// Capture an authorized payment that was previously held.
    ///
    /// `transaction_id` is the value returned by [`authorize`](Self::authorize).
    async fn capture(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError>;

    /// Execute an immediate sale (authorize + capture in one call).
    ///
    /// The default implementation calls [`authorize`](Self::authorize) followed
    /// by [`capture`](Self::capture) with the returned transaction ID.
    async fn sale(&self, request: &PaymentRequest) -> Result<PaymentResult, PaymentError> {
        let auth = self.authorize(request).await?;
        if !auth.success {
            return Ok(auth);
        }
        if let Some(ref txn_id) = auth.transaction_id {
            self.capture(txn_id).await
        } else {
            Ok(auth)
        }
    }

    /// Refund a previously captured payment.
    ///
    /// If `amount` is `None` the full amount is refunded.
    async fn refund(
        &self,
        transaction_id: &str,
        amount: Option<foundation::Money>,
    ) -> Result<PaymentResult, PaymentError>;

    /// Void / reverse a pending authorization (before capture).
    async fn void(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError>;

    /// Return a receipt for a completed transaction.
    async fn receipt(&self, transaction_id: &str) -> Result<PaymentReceipt, PaymentError>;

    /// Static device / processor identity (used in logs and the setup wizard).
    fn device_info(&self) -> DeviceInfo;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::mock::MockPaymentProcessor;
    use foundation::{Currency, Money};

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    #[tokio::test]
    async fn default_sale_calls_authorize_then_capture() {
        let proc = MockPaymentProcessor::new();
        let req = PaymentRequest {
            amount: Money::from_major(10, usd()).unwrap(),
            reference: None,
            description: None,
        };

        let result = proc.sale(&req).await.unwrap();
        assert!(result.success);
        assert_eq!(proc.authorize_calls(), 1);
        assert_eq!(proc.capture_calls(), 1);
    }

    #[tokio::test]
    async fn default_sale_returns_auth_failure() {
        let proc = MockPaymentProcessor::builder()
            .decline_next(true)
            .build();
        let req = PaymentRequest {
            amount: Money::from_major(10, usd()).unwrap(),
            reference: None,
            description: None,
        };

        let result = proc.sale(&req).await;
        assert!(result.is_err(), "sale should return Err when authorize declines");
        // capture should not have been called because authorize failed.
        assert_eq!(proc.authorize_calls(), 1);
        assert_eq!(proc.capture_calls(), 0);
    }

    #[tokio::test]
    async fn authorize_happy_path() {
        let proc = MockPaymentProcessor::new();
        let req = PaymentRequest {
            amount: Money::from_major(25, usd()).unwrap(),
            reference: Some("inv-001".into()),
            description: None,
        };

        let result = proc.authorize(&req).await.unwrap();
        assert!(result.success);
        assert!(result.transaction_id.is_some());
        assert_eq!(result.amount_charged.minor_units, 2500);
    }

    #[tokio::test]
    async fn capture_happy_path() {
        let proc = MockPaymentProcessor::new();
        let result = proc.capture("txn_test_001").await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn refund_happy_path() {
        let proc = MockPaymentProcessor::new();
        let result = proc
            .refund("txn_test_001", None)
            .await
            .unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn void_happy_path() {
        let proc = MockPaymentProcessor::new();
        let result = proc.void("txn_test_001").await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn receipt_returns_data() {
        let proc = MockPaymentProcessor::new();
        let receipt = proc.receipt("txn_test_001").await.unwrap();
        assert_eq!(receipt.transaction_id, "txn_test_001");
    }

    #[tokio::test]
    async fn device_info_returns_mock_identity() {
        let proc = MockPaymentProcessor::new();
        let info = proc.device_info();
        assert_eq!(info.vendor, "OZ-POS");
        assert_eq!(info.model, "Mock Payment Processor");
    }
}

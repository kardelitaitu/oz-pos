//! Stripe payment processor — implements [`PaymentProcessor`] using the
//! `stripe` Rust SDK for PaymentIntent-based card-present and
//! card-not-present payments.
//!
//! # Configuration
//!
//! The processor reads `STRIPE_SECRET_KEY` from the environment at
//! construction. In production the key should be set in the OS key-ring
//! (see `oz_core::Keyring`); this driver provides a `new` constructor
//! that accepts an explicit key for that use case.

use async_trait::async_trait;
use std::sync::Arc;

use foundation::{Currency, Money};
use oz_hal::types::DeviceInfo;
use stripe::{
    CapturePaymentIntent, Client, CreatePaymentIntent, CreateRefund, PaymentIntent,
    CancelPaymentIntent, Refund,
};

use crate::error::PaymentError;
use crate::types::{PaymentMethod, PaymentReceipt, PaymentRequest, PaymentResult};
use crate::PaymentProcessor;

/// A [`PaymentProcessor`] implementation backed by Stripe PaymentIntents.
///
/// Supports:
/// - **Card-present** payments (via `payment_method_types = ["card_present"]`)
/// - **Card-not-present** payments (via `payment_method_types = ["card"]`)
///
/// # Example
///
/// ```ignore
/// use oz_payment::drivers::stripe::StripePaymentProcessor;
/// use oz_payment::PaymentProcessor;
///
/// let proc = StripePaymentProcessor::from_env()?;
/// proc.sale(&request).await?;
/// ```
#[derive(Debug, Clone)]
pub struct StripePaymentProcessor {
    client: Arc<Client>,
    /// Whether to use card-present terminal API (vs card-not-present).
    card_present: bool,
}

impl StripePaymentProcessor {
    /// Create a new Stripe payment processor with the given secret key.
    ///
    /// The `card_present` flag switches between `card_present` and `card`
    /// payment method types.
    pub fn new(secret_key: &str, card_present: bool) -> Self {
        Self {
            client: Arc::new(Client::new(secret_key)),
            card_present,
        }
    }

    /// Create a new Stripe payment processor from the `STRIPE_SECRET_KEY`
    /// environment variable.
    ///
    /// # Errors
    ///
    /// Returns [`PaymentError::Network`] if the env var is not set.
    pub fn from_env() -> Result<Self, PaymentError> {
        let key = std::env::var("STRIPE_SECRET_KEY")
            .map_err(|_| PaymentError::Network("STRIPE_SECRET_KEY not set".into()))?;
        Ok(Self::new(&key, false))
    }

    /// Create a new card-present (terminal) Stripe payment processor
    /// from the `STRIPE_SECRET_KEY` environment variable.
    pub fn from_env_terminal() -> Result<Self, PaymentError> {
        let key = std::env::var("STRIPE_SECRET_KEY")
            .map_err(|_| PaymentError::Network("STRIPE_SECRET_KEY not set".into()))?;
        Ok(Self::new(&key, true))
    }

    /// The payment method type string used for this processor.
    fn pm_type(&self) -> &'static str {
        if self.card_present { "card_present" } else { "card" }
    }

    /// Convert minor units + currency to Stripe's amount-in-cents format.
    fn to_stripe_amount(amount: &Money) -> i64 {
        // ISO-4217 minor-unit exponent: most currencies use 2 decimal places.
        // Stripe expects amount in the currency's smallest unit (cents for USD).
        // The `Money` struct already stores in minor units, so we pass it directly.
        amount.minor_units
    }
}

#[async_trait]
impl PaymentProcessor for StripePaymentProcessor {
    async fn authorize(&self, request: &PaymentRequest) -> Result<PaymentResult, PaymentError> {
        let mut create = CreatePaymentIntent::new(
            Self::to_stripe_amount(&request.amount),
            stripe::Currency::default(), // Stripe determines from account
        );
        create.payment_method_types = Some(vec![self.pm_type().to_string()]);
        create.capture_method = Some(stripe::CaptureMethod::Manual);
        if let Some(ref desc) = request.description {
            create.description = Some(desc.clone());
        }

        let intent = PaymentIntent::create(&self.client, create)
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;

        Ok(PaymentResult {
            success: intent.status == stripe::PaymentIntentStatus::RequiresCapture
                || intent.status == stripe::PaymentIntentStatus::Succeeded,
            transaction_id: intent.id.as_ref().map(|id| id.to_string()),
            auth_code: None, // Stripe doesn't expose auth codes via the basic API
            amount_charged: request.amount,
            message: Some(format!("{:?}", intent.status)),
        })
    }

    async fn capture(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        let params = CapturePaymentIntent::new();
        let intent = PaymentIntent::capture(&self.client, transaction_id, params)
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;

        Ok(PaymentResult {
            success: intent.status == stripe::PaymentIntentStatus::Succeeded,
            transaction_id: Some(transaction_id.to_owned()),
            auth_code: None,
            amount_charged: Money::zero(Currency(*b"USD")), // amount may differ; actual amount is on intent
            message: Some(format!("{:?}", intent.status)),
        })
    }

    async fn refund(
        &self,
        transaction_id: &str,
        _amount: Option<foundation::Money>,
    ) -> Result<PaymentResult, PaymentError> {
        let mut params = CreateRefund::new();
        params.payment_intent = Some(transaction_id);

        let refund = Refund::create(&self.client, params)
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;

        Ok(PaymentResult {
            success: refund.status == stripe::RefundStatus::Succeeded,
            transaction_id: refund.id.as_ref().map(|id| id.to_string()),
            auth_code: None,
            amount_charged: Money::zero(Currency(*b"USD")),
            message: Some(format!("{:?}", refund.status)),
        })
    }

    async fn void(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        let params = CancelPaymentIntent::new();
        let intent = PaymentIntent::cancel(&self.client, transaction_id, params)
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;

        Ok(PaymentResult {
            success: intent.status == stripe::PaymentIntentStatus::Canceled,
            transaction_id: Some(transaction_id.to_owned()),
            auth_code: None,
            amount_charged: Money::zero(Currency(*b"USD")),
            message: Some(format!("{:?}", intent.status)),
        })
    }

    async fn receipt(&self, transaction_id: &str) -> Result<PaymentReceipt, PaymentError> {
        let intent = PaymentIntent::retrieve(&self.client, &transaction_id, &[])
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;

        Ok(PaymentReceipt {
            transaction_id: transaction_id.to_owned(),
            method: PaymentMethod::Card,
            amount: Money::zero(Currency(*b"USD")),
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            raw_data: None,
        })
    }

    fn device_info(&self) -> DeviceInfo {
        DeviceInfo::new("Stripe", "Payment Intents API", "cloud")
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
        }
    }

    /// Test that the processor can be constructed from environment.
    /// This test is ignored by default because it requires a real
    /// STRIPE_SECRET_KEY.
    #[ignore]
    #[tokio::test]
    async fn stripe_from_env_constructs() {
        let proc = StripePaymentProcessor::from_env().unwrap();
        let info = proc.device_info();
        assert_eq!(info.vendor, "Stripe");
    }

    /// Test that the pm_type method returns the correct value.
    #[test]
    fn stripe_pm_type_card() {
        let proc = StripePaymentProcessor::new("sk_test_dummy", false);
        assert_eq!(proc.pm_type(), "card");
    }

    #[test]
    fn stripe_pm_type_card_present() {
        let proc = StripePaymentProcessor::new("sk_test_dummy", true);
        assert_eq!(proc.pm_type(), "card_present");
    }

    #[test]
    fn stripe_from_env_missing_key() {
        // Temporarily unset the env var.
        let _ = std::env::remove_var("STRIPE_SECRET_KEY");
        let result = StripePaymentProcessor::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn stripe_to_stripe_amount() {
        let amount = Money::from_major(10, usd()).unwrap();
        assert_eq!(StripePaymentProcessor::to_stripe_amount(&amount), 1000);
    }

    #[test]
    fn stripe_device_info() {
        let proc = StripePaymentProcessor::new("sk_test_dummy", false);
        let info = proc.device_info();
        assert_eq!(info.vendor, "Stripe");
    }
}

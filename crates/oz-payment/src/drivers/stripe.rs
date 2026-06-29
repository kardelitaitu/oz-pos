//! Stripe payment processor — implements [`PaymentProcessor`] using the
//! Stripe REST API directly via `reqwest`.
//!
//! # Configuration
//!
//! The processor reads `STRIPE_SECRET_KEY` from the environment at
//! construction. In production the key should be set in the OS key-ring
//! (see `oz_core::Keyring`); this driver provides a `new` constructor
//! that accepts an explicit key for that use case.

use async_trait::async_trait;
use std::fmt;
use std::sync::Arc;

use foundation::{Currency, Money};
use oz_hal::types::DeviceInfo;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};

use crate::PaymentProcessor;
use crate::error::PaymentError;
use crate::types::{PaymentMethod, PaymentReceipt, PaymentRequest, PaymentResult};

/// Base URL for the Stripe API.
const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

/// A [`PaymentProcessor`] implementation backed by the Stripe REST API.
///
/// Supports:
/// - **Card-not-present** payments (default)
/// - **Card-present** payments (when constructed with `card_present: true`)
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
pub struct StripePaymentProcessor {
    client: Arc<reqwest::Client>,
    /// Whether to use card-present terminal API (vs card-not-present).
    card_present: bool,
}

impl fmt::Debug for StripePaymentProcessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StripePaymentProcessor")
            .field("client", &self.client)
            .field("secret_key", &"***")
            .field("card_present", &self.card_present)
            .finish()
    }
}

impl Clone for StripePaymentProcessor {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            card_present: self.card_present,
        }
    }
}

/// Minimal response fields we extract from the Stripe PaymentIntent JSON.
#[derive(serde::Deserialize, Debug, Clone)]
struct PaymentIntentResponse {
    id: String,
    amount: i64,
    #[serde(default)]
    amount_received: Option<i64>,
    currency: String,
    status: String,
}

/// Minimal refund response fields.
#[derive(serde::Deserialize, Debug, Clone)]
struct RefundResponse {
    id: String,
    amount: i64,
    currency: String,
    status: String,
}

/// Stripe API error response body.
#[derive(serde::Deserialize, Debug)]
struct StripeErrorBody {
    error: StripeErrorDetail,
}

#[derive(serde::Deserialize, Debug)]
struct StripeErrorDetail {
    #[serde(rename = "type")]
    error_type: String,
    message: Option<String>,
    #[allow(dead_code)]
    code: Option<String>,
}

impl StripePaymentProcessor {
    /// Create a new Stripe payment processor with the given secret key.
    ///
    /// The `card_present` flag switches between `card_present` and `card`
    /// payment method types.
    pub fn new(secret_key: &str, card_present: bool) -> Self {
        let mut headers = HeaderMap::new();
        let mut auth_value =
            HeaderValue::from_str(&format!("Bearer {}", secret_key)).expect("valid header value");
        auth_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_value);
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("valid reqwest client");

        Self {
            client: Arc::new(client),
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
        Ok(Self::new(&Self::secret_key_from_env()?, false))
    }

    /// Create a new card-present (terminal) Stripe payment processor
    /// from the `STRIPE_SECRET_KEY` environment variable.
    pub fn from_env_terminal() -> Result<Self, PaymentError> {
        Ok(Self::new(&Self::secret_key_from_env()?, true))
    }

    /// Read `STRIPE_SECRET_KEY` from the environment.
    fn secret_key_from_env() -> Result<String, PaymentError> {
        std::env::var("STRIPE_SECRET_KEY")
            .map_err(|_| PaymentError::Network("STRIPE_SECRET_KEY not set".into()))
    }

    /// The payment method type string used for this processor.
    fn pm_type(&self) -> &'static str {
        if self.card_present {
            "card_present"
        } else {
            "card"
        }
    }

    /// Convert a `Money` value to Stripe's amount-in-cents format.
    fn to_stripe_amount(amount: &Money) -> i64 {
        amount.minor_units
    }

    /// Convert a Stripe currency code (lowercase) to a [`foundation::Currency`].
    fn to_currency(code: &str) -> Currency {
        code.to_uppercase().parse().unwrap_or(Currency(*b"USD"))
    }

    /// Convert Stripe amount + currency code to [`Money`].
    fn to_money(minor_units: i64, currency: &str) -> Money {
        Money {
            minor_units,
            currency: Self::to_currency(currency),
        }
    }

    /// Perform an HTTP POST to the Stripe API and return (status, body).
    async fn post(
        &self,
        path: &str,
        form: Vec<(&str, &str)>,
    ) -> Result<(u16, String), PaymentError> {
        let url = format!("{}{}", STRIPE_API_BASE, path);
        let resp = self
            .client
            .post(&url)
            .form(&form)
            .send()
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;
        Ok((status, body))
    }

    /// Perform an HTTP GET to the Stripe API and return (status, body).
    async fn get(&self, path: &str) -> Result<(u16, String), PaymentError> {
        let url = format!("{}{}", STRIPE_API_BASE, path);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;
        Ok((status, body))
    }

    /// Parse a Stripe API error response body into a [`PaymentError`].
    fn parse_error(status: u16, body: &str) -> PaymentError {
        if let Ok(err) = serde_json::from_str::<StripeErrorBody>(body) {
            let msg = err
                .error
                .message
                .unwrap_or_else(|| format!("stripe_error: {}", err.error.error_type));
            PaymentError::Network(msg)
        } else {
            PaymentError::Network(format!("HTTP {}: {}", status, body))
        }
    }

    /// Parse a successful Stripe response body into a [`PaymentIntentResponse`].
    fn parse_intent(body: &str) -> Result<PaymentIntentResponse, PaymentError> {
        serde_json::from_str(body).map_err(|e| {
            PaymentError::Network(format!(
                "failed to parse PaymentIntent: {} — body: {}",
                e, body
            ))
        })
    }

    /// Parse a successful Stripe response body into a [`RefundResponse`].
    fn parse_refund(body: &str) -> Result<RefundResponse, PaymentError> {
        serde_json::from_str(body).map_err(|e| {
            PaymentError::Network(format!("failed to parse Refund: {} — body: {}", e, body))
        })
    }

    /// Extract success status and amount from an intent response.
    fn intent_result(intent: &PaymentIntentResponse) -> (bool, Money) {
        let success = intent.status == "succeeded" || intent.status == "requires_capture";
        let amount = Self::to_money(
            intent.amount_received.unwrap_or(intent.amount),
            &intent.currency,
        );
        (success, amount)
    }
}

#[async_trait]
impl PaymentProcessor for StripePaymentProcessor {
    async fn authorize(&self, request: &PaymentRequest) -> Result<PaymentResult, PaymentError> {
        let amount_str = Self::to_stripe_amount(&request.amount).to_string();
        let mut form = vec![
            ("amount", amount_str.as_str()),
            ("currency", "usd"),
            ("payment_method_types[]", self.pm_type()),
            ("capture_method", "manual"),
        ];
        if let Some(ref desc) = request.description {
            form.push(("description", desc.as_str()));
        }

        let (status, body) = self.post("/payment_intents", form).await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &body));
        }

        let intent = Self::parse_intent(&body)?;
        let (success, amount) = Self::intent_result(&intent);

        Ok(PaymentResult {
            success,
            transaction_id: Some(intent.id),
            auth_code: None,
            amount_charged: amount,
            message: Some(intent.status),
        })
    }

    async fn capture(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        let (status, body) = self
            .post(
                &format!("/payment_intents/{}/capture", transaction_id),
                vec![],
            )
            .await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &body));
        }

        let intent = Self::parse_intent(&body)?;
        let (success, amount) = Self::intent_result(&intent);

        Ok(PaymentResult {
            success,
            transaction_id: Some(intent.id),
            auth_code: None,
            amount_charged: amount,
            message: Some(intent.status),
        })
    }

    async fn refund(
        &self,
        transaction_id: &str,
        _amount: Option<Money>,
    ) -> Result<PaymentResult, PaymentError> {
        let form = vec![("payment_intent", transaction_id)];
        let (status, body) = self.post("/refunds", form).await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &body));
        }

        let refund = Self::parse_refund(&body)?;
        let amount = Self::to_money(refund.amount, &refund.currency);

        Ok(PaymentResult {
            success: refund.status == "succeeded",
            transaction_id: Some(refund.id),
            auth_code: None,
            amount_charged: amount,
            message: Some(refund.status),
        })
    }

    async fn void(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        let (status, body) = self
            .post(
                &format!("/payment_intents/{}/cancel", transaction_id),
                vec![],
            )
            .await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &body));
        }

        let intent = Self::parse_intent(&body)?;
        let (success, amount) = Self::intent_result(&intent);

        Ok(PaymentResult {
            success,
            transaction_id: Some(intent.id),
            auth_code: None,
            amount_charged: amount,
            message: Some(intent.status),
        })
    }

    async fn receipt(&self, transaction_id: &str) -> Result<PaymentReceipt, PaymentError> {
        let (status, body) = self
            .get(&format!("/payment_intents/{}", transaction_id))
            .await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &body));
        }

        let intent = Self::parse_intent(&body)?;
        let (_, amount) = Self::intent_result(&intent);

        Ok(PaymentReceipt {
            transaction_id: intent.id,
            method: PaymentMethod::Card,
            amount,
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            raw_data: None,
        })
    }

    fn device_info(&self) -> DeviceInfo {
        DeviceInfo::new("Stripe", "REST API", "cloud")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    fn test_key() -> String {
        "sk_test_dummy_key_1234567890".to_string()
    }

    fn currency_code(c: &Currency) -> &str {
        str::from_utf8(&c.0).unwrap_or("???")
    }

    #[test]
    fn stripe_constructs() {
        let proc = StripePaymentProcessor::new(&test_key(), false);
        let info = proc.device_info();
        assert_eq!(info.vendor, "Stripe");
        assert_eq!(info.model, "REST API");
    }

    #[test]
    fn stripe_pm_type_card() {
        let proc = StripePaymentProcessor::new(&test_key(), false);
        assert_eq!(proc.pm_type(), "card");
    }

    #[test]
    fn stripe_pm_type_card_present() {
        let proc = StripePaymentProcessor::new(&test_key(), true);
        assert_eq!(proc.pm_type(), "card_present");
    }

    #[test]
    fn stripe_to_stripe_amount() {
        let usd: Currency = "USD".parse().unwrap();
        let amount = Money::from_major(10, usd).unwrap();
        assert_eq!(StripePaymentProcessor::to_stripe_amount(&amount), 1000);
    }

    #[test]
    fn stripe_to_currency_usd() {
        let c = StripePaymentProcessor::to_currency("usd");
        assert_eq!(currency_code(&c), "USD");
    }

    #[test]
    fn stripe_to_currency_eur() {
        let c = StripePaymentProcessor::to_currency("eur");
        assert_eq!(currency_code(&c), "EUR");
    }

    #[test]
    fn stripe_to_money_constructs() {
        let m = StripePaymentProcessor::to_money(1000, "usd");
        assert_eq!(m.minor_units, 1000);
        assert_eq!(currency_code(&m.currency), "USD");
    }

    #[test]
    fn stripe_parse_intent_success() {
        let json = r#"{"id":"pi_test_123","amount":1000,"amount_received":1000,"currency":"usd","status":"succeeded"}"#;
        let intent = StripePaymentProcessor::parse_intent(json).unwrap();
        assert_eq!(intent.id, "pi_test_123");
        assert_eq!(intent.amount, 1000);
        assert_eq!(intent.amount_received, Some(1000));
        assert_eq!(intent.currency, "usd");
        assert_eq!(intent.status, "succeeded");
    }

    #[test]
    fn stripe_parse_intent_no_amount_received() {
        let json =
            r#"{"id":"pi_test_456","amount":2000,"currency":"usd","status":"requires_capture"}"#;
        let intent = StripePaymentProcessor::parse_intent(json).unwrap();
        assert_eq!(intent.amount, 2000);
        assert_eq!(intent.amount_received, None);
    }

    #[test]
    fn stripe_parse_refund_success() {
        let json = r#"{"id":"re_test_789","amount":500,"currency":"usd","status":"succeeded"}"#;
        let refund = StripePaymentProcessor::parse_refund(json).unwrap();
        assert_eq!(refund.id, "re_test_789");
        assert_eq!(refund.amount, 500);
        assert_eq!(refund.status, "succeeded");
    }

    #[test]
    fn stripe_parse_error_body() {
        let json = r#"{"error":{"type":"card_error","message":"Your card was declined.","code":"card_declined"}}"#;
        let body: StripeErrorBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.error.error_type, "card_error");
        assert_eq!(body.error.message.unwrap(), "Your card was declined.");
    }

    #[test]
    fn stripe_parse_error_no_message() {
        let json = r#"{"error":{"type":"api_error"}}"#;
        let body: StripeErrorBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.error.error_type, "api_error");
        assert!(body.error.message.is_none());
    }

    #[test]
    fn stripe_secret_key_from_env_error_check() {
        // Verify the error path of `secret_key_from_env` by checking
        // that calling `from_env` works when STRIPE_SECRET_KEY is set
        // (in CI) or fails gracefully when it's not set (local dev).
        let result = StripePaymentProcessor::from_env();
        match std::env::var("STRIPE_SECRET_KEY") {
            Ok(_) => assert!(result.is_ok()),
            Err(_) => {
                assert!(result.is_err());
                let msg = result.unwrap_err().to_string();
                assert!(msg.contains("not set"), "error: {}", msg);
            }
        }
    }

    #[test]
    fn stripe_intent_result_succeeded() {
        let intent = PaymentIntentResponse {
            id: "pi_1".into(),
            amount: 1000,
            amount_received: Some(1000),
            currency: "usd".into(),
            status: "succeeded".into(),
        };
        let (success, money) = StripePaymentProcessor::intent_result(&intent);
        assert!(success);
        assert_eq!(money.minor_units, 1000);
    }

    #[test]
    fn stripe_intent_result_requires_capture() {
        let intent = PaymentIntentResponse {
            id: "pi_2".into(),
            amount: 2000,
            amount_received: None,
            currency: "usd".into(),
            status: "requires_capture".into(),
        };
        let (success, money) = StripePaymentProcessor::intent_result(&intent);
        assert!(success);
        assert_eq!(money.minor_units, 2000);
    }

    #[test]
    fn stripe_intent_result_canceled() {
        let intent = PaymentIntentResponse {
            id: "pi_3".into(),
            amount: 500,
            amount_received: None,
            currency: "usd".into(),
            status: "canceled".into(),
        };
        let (success, _) = StripePaymentProcessor::intent_result(&intent);
        assert!(!success);
    }

    #[test]
    fn stripe_parse_error_formats() {
        let err = StripePaymentProcessor::parse_error(
            402,
            r#"{"error":{"type":"card_error","message":"declined"}}"#,
        );
        let msg = err.to_string();
        assert!(msg.contains("declined"));
    }

    #[test]
    fn stripe_parse_error_non_json() {
        let err = StripePaymentProcessor::parse_error(500, "Internal Server Error");
        let msg = err.to_string();
        assert!(msg.contains("500"));
        assert!(msg.contains("Internal Server Error"));
    }

    #[test]
    fn stripe_debug_masks_key() {
        let proc = StripePaymentProcessor::new(&test_key(), false);
        let debug = format!("{:?}", proc);
        assert!(!debug.contains("sk_test"));
        assert!(!debug.contains("dummy_key"));
        assert!(debug.contains("***"));
    }

    #[test]
    fn stripe_clone_preserves_config() {
        let proc = StripePaymentProcessor::new(&test_key(), true);
        let cloned = proc.clone();
        assert_eq!(cloned.pm_type(), "card_present");
        let info = cloned.device_info();
        assert_eq!(info.vendor, "Stripe");
    }
}

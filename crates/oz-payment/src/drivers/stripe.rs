//! Stripe payment processor — implements [`PaymentProcessor`] using the
//! Stripe REST API directly via `reqwest`.
//!
//! # Configuration
//!
//! The processor reads `STRIPE_SECRET_KEY` from the environment at
//! construction. In production the key should be set in the OS key-ring
//! (see `oz_core::Keyring`); this driver provides a `new` constructor
//! that accepts an explicit key for that use case.
//!
//! # Testing
//!
//! Use `new_with_endpoint` to direct requests
//! to a local mock server (e.g. `wiremock`) during integration tests.

use async_trait::async_trait;
use std::fmt;
use std::sync::Arc;

use foundation::{Currency, Money};
use oz_hal::types::DeviceInfo;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};

use crate::PaymentProcessor;
use crate::error::PaymentError;
use crate::types::{PaymentMethod, PaymentReceipt, PaymentRequest, PaymentResult};

/// Default base URL for the Stripe API.
const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

/// A [`PaymentProcessor`] implementation backed by the Stripe REST API.
///
/// Supports:
/// - **Card-not-present** payments (default)
/// - **Card-present** payments (when constructed with `card_present: true`)
///
/// # Example
///
/// ```no_run
/// # use oz_payment::drivers::stripe::StripePaymentProcessor;
/// # use oz_payment::PaymentProcessor;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let request = unimplemented!();
/// let proc = StripePaymentProcessor::from_env()?;
/// proc.sale(&request).await?;
/// # Ok(())
/// # }
/// ```
pub struct StripePaymentProcessor {
    client: Arc<reqwest::Client>,
    /// Whether to use card-present terminal API (vs card-not-present).
    card_present: bool,
    /// Base URL for the Stripe API (configurable for testing).
    api_base: String,
}

impl fmt::Debug for StripePaymentProcessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StripePaymentProcessor")
            .field("client", &self.client)
            .field("secret_key", &"***")
            .field("card_present", &self.card_present)
            .field("api_base", &self.api_base)
            .finish()
    }
}

impl Clone for StripePaymentProcessor {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            card_present: self.card_present,
            api_base: self.api_base.clone(),
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
    /// payment method types. Requests are sent to the live Stripe API at
    /// `https://api.stripe.com/v1`.
    pub fn new(secret_key: &str, card_present: bool) -> Self {
        Self::new_with_endpoint(secret_key, STRIPE_API_BASE, card_present)
    }

    /// Create a new Stripe payment processor with a custom API endpoint.
    ///
    /// This constructor is useful for integration tests where requests
    /// should be directed to a mock server (e.g. `wiremock`).
    pub fn new_with_endpoint(secret_key: &str, api_base: &str, card_present: bool) -> Self {
        let mut headers = HeaderMap::new();
        let mut auth_value = HeaderValue::from_str(&format!("Bearer {}", secret_key))
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "invalid Stripe auth header — using placeholder");
                HeaderValue::from_static("Bearer placeholder")
            });
        auth_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_value);
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .no_proxy()
            .build()
            .unwrap_or_else(|e| {
                tracing::error!(
                    error = %e,
                    "failed to build HTTP client for Stripe — using default"
                );
                reqwest::Client::new()
            });

        Self {
            client: Arc::new(client),
            card_present,
            api_base: api_base.to_owned(),
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
    ///
    /// PA-02: unknown codes are a hard error, not a silent USD fallback —
    /// a malformed gateway currency would otherwise mislabel the recorded
    /// amount.
    fn to_currency(code: &str) -> Result<Currency, PaymentError> {
        code.parse::<Currency>()
            .map_err(|_| PaymentError::Network(format!("unknown gateway currency code: {code}")))
    }

    /// Convert Stripe amount + currency code to [`Money`].
    fn to_money(minor_units: i64, currency: &str) -> Result<Money, PaymentError> {
        Ok(Money {
            minor_units,
            currency: Self::to_currency(currency)?,
        })
    }

    /// Perform an HTTP POST to the Stripe API and return (status, body).
    async fn post(
        &self,
        path: &str,
        form: Vec<(&str, &str)>,
    ) -> Result<(u16, String), PaymentError> {
        let url = format!("{}{}", self.api_base, path);
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
        let url = format!("{}{}", self.api_base, path);
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

    /// Classify a Stripe error type into a specific PaymentError variant.
    fn classify_stripe_error(
        error_type: &str,
        message: Option<&str>,
        code: Option<&str>,
    ) -> PaymentError {
        let msg = message.unwrap_or(error_type).to_string();
        match error_type {
            "card_error" | "invalid_request_error"
                if code == Some("card_declined")
                    || code == Some("processing_error")
                    || code == Some("incorrect_number")
                    || code == Some("expired_card")
                    || code == Some("incorrect_cvc")
                    || code == Some("incorrect_zip")
                    || message.is_some_and(|m| m.contains("card")) =>
            {
                PaymentError::InvalidCard(msg)
            }
            "card_error" => PaymentError::Declined(msg),
            "idempotency_error" => PaymentError::Duplicate(msg),
            "invalid_request_error"
            | "api_error"
            | "api_connection_error"
            | "authentication_error"
            | "rate_limit_error" => PaymentError::Network(msg),
            _ => PaymentError::Network(format!("stripe_error: {}", msg)),
        }
    }

    /// Parse a Stripe API error response body into a [`PaymentError`].
    fn parse_error(status: u16, body: &str) -> PaymentError {
        if let Ok(err) = serde_json::from_str::<StripeErrorBody>(body) {
            Self::classify_stripe_error(
                &err.error.error_type,
                err.error.message.as_deref(),
                err.error.code.as_deref(),
            )
        } else {
            PaymentError::Network(format!("HTTP {}: {}", status, body))
        }
    }

    /// Parse a successful Stripe response body into a [`PaymentIntentResponse`].
    fn parse_intent(body: &str) -> Result<PaymentIntentResponse, PaymentError> {
        serde_json::from_str(body).map_err(|e| {
            PaymentError::Network(format!(
                "failed to parse PaymentIntent: {} -- body: {}",
                e, body
            ))
        })
    }

    /// Parse a successful Stripe response body into a [`RefundResponse`].
    fn parse_refund(body: &str) -> Result<RefundResponse, PaymentError> {
        serde_json::from_str(body).map_err(|e| {
            PaymentError::Network(format!("failed to parse Refund: {} -- body: {}", e, body))
        })
    }

    /// Extract success status and amount from an intent response.
    fn intent_result(intent: &PaymentIntentResponse) -> Result<(bool, Money), PaymentError> {
        let success = intent.status == "succeeded" || intent.status == "requires_capture";
        let amount = Self::to_money(
            intent.amount_received.unwrap_or(intent.amount),
            &intent.currency,
        )?;
        Ok((success, amount))
    }
}

#[async_trait]
impl PaymentProcessor for StripePaymentProcessor {
    async fn authorize(&self, request: &PaymentRequest) -> Result<PaymentResult, PaymentError> {
        let amount_str = Self::to_stripe_amount(&request.amount).to_string();
        let currency_str = String::from_utf8_lossy(&request.amount.currency.0).into_owned();
        let mut form = vec![
            ("amount", amount_str.as_str()),
            ("currency", currency_str.as_str()),
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
        let (success, amount) = Self::intent_result(&intent)?;

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
        let (success, amount) = Self::intent_result(&intent)?;

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
        let amount = Self::to_money(refund.amount, &refund.currency)?;

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
        let (success, amount) = Self::intent_result(&intent)?;

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
        let (_, amount) = Self::intent_result(&intent)?;

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
#[path = "stripe_tests.rs"]
mod tests;

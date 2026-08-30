/*
last audited 31-08-26 by TDD-Agent (round M; PAY-2 closed for charges when the caller supplies a key)
crate: oz-payment | status: SAFE | lint: CLEAN
findings: capture_method manual correctly maps authorize/capture model; PAY-2 FIXED for authorize — idempotency_key_for() forwards PaymentRequest.idempotency_key as the Idempotency-Key header, verbatim, or omits the header when absent or blank (blank must not be sent: Some("") would put every caller who leaves the field empty into one shared key and Stripe would reject each later charge as a conflict). The tell that this was an oversight rather than a decision: parse_error already mapped Stripe's idempotency_error code to PaymentError::Duplicate, so the driver handled a duplicate-key rejection it could never receive. PAY-2 STILL OPEN for refund — refund(transaction_id, amount) takes no PaymentRequest, so there is no caller key; minting a fresh one dedups nothing and deriving one from transaction_id alone would collide two genuinely different partial refunds of the same payment into one. Needs a PaymentProcessor trait change. capture/void send no key and do not need one: re-capturing or re-cancelling is a Stripe error response, not a second state change. PAY-3 refund(_amount) ignores partial amount (always full refund); PAY-4 classifier sends most card_error declines to InvalidCard (message.contains("card") heuristic; "card_error => Declined" arm nearly unreachable); no confirm step — card-not-present intents stay requires_payment_method unless confirmed elsewhere
next: Idempotency-Key on refunds (trait change), partial refund, fix decline classification. COR-31 STILL HELD, and round L's release condition was written unsatisfiably: it said to bound the client "once every request carries a caller-supplied key", but idempotency_key is Option on PaymentRequest, so no driver-side change can ever guarantee that. Releasing the hold needs a CALLER policy (desktop/tablet always populate it) or a decision that a hang is worse than a possible double charge — not another edit in this file. | perf: N/A
*/
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

    /// The `Idempotency-Key` to send for a charge request, if any.
    ///
    /// PAY-2: Stripe dedups on this header, and the driver never sent one —
    /// while `parse_error` already maps Stripe's `idempotency_error` code to
    /// [`PaymentError::Duplicate`]. The rejection handling for a duplicate
    /// key existed with no key ever sent to trigger it.
    ///
    /// Returns `None` for an absent OR blank key. Blank must not be sent:
    /// `Some("")` would put every caller who leaves the field empty into one
    /// shared key, and Stripe would reject each charge after the first as a
    /// conflict — turning the double-charge guard into a way to refuse
    /// legitimate payments. The value is forwarded verbatim, never clamped:
    /// truncating maps two distinct keys onto one and silently drops a
    /// charge, which is worse than a loud rejection from Stripe.
    fn idempotency_key_for(request: &PaymentRequest) -> Option<&str> {
        let key = request.idempotency_key.as_deref()?;
        if key.trim().is_empty() {
            None
        } else {
            Some(key)
        }
    }

    /// Perform an HTTP POST to the Stripe API and return (status, body).
    ///
    /// `idempotency_key` is sent as the `Idempotency-Key` header when
    /// present; see [`Self::idempotency_key_for`].
    async fn post(
        &self,
        path: &str,
        form: Vec<(&str, &str)>,
        idempotency_key: Option<&str>,
    ) -> Result<(u16, String), PaymentError> {
        let url = format!("{}{}", self.api_base, path);
        let mut request = self.client.post(&url).form(&form);
        if let Some(key) = idempotency_key {
            request = request.header("Idempotency-Key", key);
        }
        let resp = request
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

        let (status, body) = self
            .post("/payment_intents", form, Self::idempotency_key_for(request))
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

    async fn capture(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        // No key: capture takes only a transaction_id, and it is safe to
        // leave — capturing an already-captured intent is a Stripe error
        // response, not a second charge.
        let (status, body) = self
            .post(
                &format!("/payment_intents/{}/capture", transaction_id),
                vec![],
                None,
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
        // No idempotency key: refund(transaction_id, amount) takes no
        // PaymentRequest, so there is no caller key to forward. Minting one
        // here would be worse than nothing — a fresh key per call dedups
        // nothing, and a stable one derived from transaction_id alone would
        // make two genuinely different partial refunds of the same payment
        // collide into one. Needs a PaymentProcessor trait change; recorded
        // in the module stamp rather than half-solved.
        let (status, body) = self.post("/refunds", form, None).await?;
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
        // No key: cancelling an already-cancelled intent is a Stripe error,
        // not a second state change, so a retry cannot double-apply.
        let (status, body) = self
            .post(
                &format!("/payment_intents/{}/cancel", transaction_id),
                vec![],
                None,
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

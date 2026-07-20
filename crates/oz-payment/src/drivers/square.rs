//! Square payment processor — implements [`PaymentProcessor`] using the
//! Square REST API directly via `reqwest`.
//!
//! # Configuration
//!
//! The processor is constructed with an API key and a location ID. In
//! production the key should be set in the OS key-ring (see
//! `oz_core::Keyring`); this driver provides a `new` constructor that
//! accepts an explicit key for that use case.

use async_trait::async_trait;
use std::fmt;

use foundation::{Currency, Money};
use oz_hal::types::DeviceInfo;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Serialize;
use uuid::Uuid;

use crate::PaymentProcessor;
use crate::error::PaymentError;
use crate::types::{PaymentMethod, PaymentReceipt, PaymentRequest, PaymentResult};

/// Default base URL for the Square API.
const SQUARE_API_BASE: &str = "https://connect.squareup.com/v2";

/// A [`PaymentProcessor`] implementation backed by the Square REST API.
///
/// Supports card-not-present payments via the Square Payments API.
///
/// # Example
///
/// ```no_run
/// # use oz_payment::drivers::square::SquarePaymentProcessor;
/// # use oz_payment::PaymentProcessor;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let request = unimplemented!();
/// let proc = SquarePaymentProcessor::new("api_key", "location_id");
/// proc.sale(&request).await?;
/// # Ok(())
/// # }
/// ```
pub struct SquarePaymentProcessor {
    #[allow(dead_code)]
    api_key: String,
    location_id: String,
    client: reqwest::Client,
    /// Base URL for the Square API (configurable for testing).
    api_base: String,
}

impl fmt::Debug for SquarePaymentProcessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SquarePaymentProcessor")
            .field("api_key", &"***")
            .field("location_id", &self.location_id)
            .field("client", &self.client)
            .field("api_base", &self.api_base)
            .finish()
    }
}

/// Request body for Square Create Payment.
#[derive(Serialize, Debug)]
struct CreatePaymentRequest {
    idempotency_key: String,
    amount_money: MoneyAmount,
    source_id: String,
    location_id: String,
    reference_id: Option<String>,
    note: Option<String>,
}

/// Request body for Square Refund.
#[derive(Serialize, Debug)]
struct CreateRefundRequest {
    idempotency_key: String,
    payment_id: String,
    amount_money: MoneyAmount,
}

/// Amount + currency used in Square API requests.
#[derive(Serialize, Debug)]
struct MoneyAmount {
    amount: i64,
    currency: String,
}

/// Minimal response fields we extract from the Square Payment JSON.
#[derive(serde::Deserialize, Debug, Clone)]
struct PaymentResponse {
    payment: PaymentData,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct PaymentData {
    id: String,
    status: String,
    amount_money: MoneyAmountResponse,
    created_at: Option<String>,
}

/// Minimal refund response fields.
#[derive(serde::Deserialize, Debug, Clone)]
struct RefundResponse {
    refund: RefundData,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct RefundData {
    id: String,
    status: String,
    amount_money: MoneyAmountResponse,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct MoneyAmountResponse {
    amount: i64,
    currency: String,
}

/// Square API error response body.
#[derive(serde::Deserialize, Debug)]
struct SquareErrorBody {
    errors: Vec<SquareErrorDetail>,
}

#[derive(serde::Deserialize, Debug)]
struct SquareErrorDetail {
    #[allow(dead_code)]
    code: Option<String>,
    detail: Option<String>,
}

impl SquarePaymentProcessor {
    /// Create a new Square payment processor with the given API key and location ID.
    ///
    /// Requests are sent to the live Square API at `https://connect.squareup.com/v2`.
    pub fn new(api_key: &str, location_id: &str) -> Self {
        Self::new_with_endpoint(api_key, location_id, SQUARE_API_BASE)
    }

    /// Create a new Square payment processor with a custom API endpoint.
    ///
    /// This constructor is useful for integration tests where requests
    /// should be directed to a mock server (e.g. `wiremock`).
    pub fn new_with_endpoint(api_key: &str, location_id: &str, api_base: &str) -> Self {
        let mut headers = HeaderMap::new();
        let mut auth_value =
            HeaderValue::from_str(&format!("Bearer {}", api_key)).expect("valid header value");
        auth_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_value);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .no_proxy()
            .build()
            .expect("valid reqwest client");

        Self {
            api_key: api_key.to_owned(),
            location_id: location_id.to_owned(),
            client,
            api_base: api_base.to_owned(),
        }
    }

    /// Convert a `Money` value to Square's amount-minor-units format.
    fn to_square_amount(amount: &Money) -> i64 {
        amount.minor_units
    }

    /// Convert a Square currency code (uppercase) to a [`foundation::Currency`].
    fn to_currency(code: &str) -> Currency {
        code.to_uppercase().parse().unwrap_or(Currency(*b"USD"))
    }

    /// Convert Square amount + currency code to [`Money`].
    fn to_money(minor_units: i64, currency: &str) -> Money {
        Money {
            minor_units,
            currency: Self::to_currency(currency),
        }
    }

    /// Perform an HTTP POST to the Square API and return (status, body).
    async fn post<T: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<(u16, String), PaymentError> {
        let url = format!("{}{}", self.api_base, path);
        let resp = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;
        Ok((status, body_text))
    }

    /// Perform an HTTP GET to the Square API and return (status, body).
    async fn get(&self, path: &str) -> Result<(u16, String), PaymentError> {
        let url = format!("{}{}", self.api_base, path);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(|e| PaymentError::Network(e.to_string()))?;
        Ok((status, body_text))
    }

    /// Classify a Square error code into a specific PaymentError variant.
    fn classify_square_error(code: Option<&str>, detail: Option<&str>) -> PaymentError {
        let msg = detail.unwrap_or(code.unwrap_or("unknown")).to_string();
        match code {
            Some(c)
                if c == "CARD_DECLINED"
                    || c == "VERIFY_CVV_FAILURE"
                    || c == "VERIFY_AVS_FAILURE"
                    || c == "INVALID_EXPIRATION"
                    || c == "INVALID_CARD"
                    || c == "CARD_EXPIRED"
                    || c == "INVALID_PIN"
                    || c == "INVALID_ACCOUNT"
                    || c == "CARDHOLDER_VERIFICATION_REQUIRED" =>
            {
                PaymentError::Declined(msg)
            }
            Some(c)
                if c == "INVALID_LOCATION"
                    || c == "MISSING_REQUIRED_PARAMETER"
                    || c == "UNSUPPORTED_CARD_BRAND"
                    || c == "CARD_NOT_SUPPORTED"
                    || c == "UNSUPPORTED_ENTRY_METHOD" =>
            {
                PaymentError::InvalidCard(msg)
            }
            Some(c) if c == "DUPLICATE_CARD" || c == "IDEMPOTENCY_KEY_REUSED" => {
                PaymentError::Duplicate(msg)
            }
            Some(c) if c == "TIMEOUT" || c == "GATEWAY_TIMEOUT" => PaymentError::Timeout(30000),
            _ => PaymentError::Network(msg),
        }
    }

    /// Parse a Square API error response body into a [`PaymentError`].
    fn parse_error(status: u16, body: &str) -> PaymentError {
        if let Ok(err) = serde_json::from_str::<SquareErrorBody>(body) {
            let mut errors = err.errors;
            if errors.is_empty() {
                PaymentError::Network(format!("square_error: HTTP {}", status))
            } else {
                // Classify the most specific error from Square's error list
                let first = errors.remove(0);
                Self::classify_square_error(first.code.as_deref(), first.detail.as_deref())
            }
        } else {
            PaymentError::Network(format!("HTTP {}: {}", status, body))
        }
    }

    /// Parse a successful Square response body into a [`PaymentData`].
    fn parse_payment(body: &str) -> Result<PaymentData, PaymentError> {
        let resp: PaymentResponse = serde_json::from_str(body).map_err(|e| {
            PaymentError::Network(format!("failed to parse Payment: {} — body: {}", e, body))
        })?;
        Ok(resp.payment)
    }

    /// Parse a successful Square response body into a [`RefundData`].
    fn parse_refund(body: &str) -> Result<RefundData, PaymentError> {
        let resp: RefundResponse = serde_json::from_str(body).map_err(|e| {
            PaymentError::Network(format!("failed to parse Refund: {} — body: {}", e, body))
        })?;
        Ok(resp.refund)
    }

    /// Extract success status and amount from payment data.
    fn payment_result(data: &PaymentData) -> (bool, Money) {
        let success = data.status == "COMPLETED" || data.status == "APPROVED";
        let amount = Self::to_money(data.amount_money.amount, &data.amount_money.currency);
        (success, amount)
    }
}

#[async_trait]
impl PaymentProcessor for SquarePaymentProcessor {
    async fn authorize(&self, request: &PaymentRequest) -> Result<PaymentResult, PaymentError> {
        let body = CreatePaymentRequest {
            idempotency_key: Uuid::now_v7().to_string(),
            amount_money: MoneyAmount {
                amount: Self::to_square_amount(&request.amount),
                currency: "USD".to_string(),
            },
            source_id: "EXTERNAL".to_string(),
            location_id: self.location_id.clone(),
            reference_id: request.reference.clone(),
            note: request.description.clone(),
        };

        let (status, body_text) = self.post("/payments", &body).await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &body_text));
        }

        let payment = Self::parse_payment(&body_text)?;
        let (success, amount) = Self::payment_result(&payment);

        Ok(PaymentResult {
            success,
            transaction_id: Some(payment.id),
            auth_code: None,
            amount_charged: amount,
            message: Some(payment.status),
        })
    }

    async fn capture(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        let path = format!("/payments/{}/complete", transaction_id);
        let (status, body_text) = self.post(&path, &serde_json::json!({})).await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &body_text));
        }

        let payment = Self::parse_payment(&body_text)?;
        let (success, amount) = Self::payment_result(&payment);

        Ok(PaymentResult {
            success,
            transaction_id: Some(payment.id),
            auth_code: None,
            amount_charged: amount,
            message: Some(payment.status),
        })
    }

    async fn refund(
        &self,
        transaction_id: &str,
        amount: Option<Money>,
    ) -> Result<PaymentResult, PaymentError> {
        let charged_amount = amount.unwrap_or(Money::zero(Currency(*b"USD")));
        let body = CreateRefundRequest {
            idempotency_key: Uuid::now_v7().to_string(),
            payment_id: transaction_id.to_string(),
            amount_money: MoneyAmount {
                amount: Self::to_square_amount(&charged_amount),
                currency: "USD".to_string(),
            },
        };

        let (status, body_text) = self.post("/refunds", &body).await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &body_text));
        }

        let refund = Self::parse_refund(&body_text)?;
        let refund_amount =
            Self::to_money(refund.amount_money.amount, &refund.amount_money.currency);

        Ok(PaymentResult {
            success: refund.status == "COMPLETED"
                || refund.status == "APPROVED"
                || refund.status == "PENDING",
            transaction_id: Some(refund.id),
            auth_code: None,
            amount_charged: refund_amount,
            message: Some(refund.status),
        })
    }

    async fn void(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        let path = format!("/payments/{}/cancel", transaction_id);
        let (status, body_text) = self.post(&path, &serde_json::json!({})).await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &body_text));
        }

        let payment = Self::parse_payment(&body_text)?;
        let (success, amount) = Self::payment_result(&payment);

        Ok(PaymentResult {
            success,
            transaction_id: Some(payment.id),
            auth_code: None,
            amount_charged: amount,
            message: Some(payment.status),
        })
    }

    async fn receipt(&self, transaction_id: &str) -> Result<PaymentReceipt, PaymentError> {
        let path = format!("/payments/{}", transaction_id);
        let (status, body_text) = self.get(&path).await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &body_text));
        }

        let payment = Self::parse_payment(&body_text)?;
        let (_, amount) = Self::payment_result(&payment);

        Ok(PaymentReceipt {
            transaction_id: payment.id,
            method: PaymentMethod::Card,
            amount,
            timestamp: payment.created_at.unwrap_or_else(|| {
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            }),
            raw_data: None,
        })
    }

    fn device_info(&self) -> DeviceInfo {
        DeviceInfo::new("Square", "Square API", "cloud")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_api_key() -> String {
        "EAAA_test_square_api_key_123456".to_string()
    }

    fn test_location_id() -> String {
        "L_ABC123".to_string()
    }

    #[test]
    fn square_processor_construction() {
        let proc = SquarePaymentProcessor::new(&test_api_key(), &test_location_id());
        assert_eq!(proc.api_key, test_api_key());
        assert_eq!(proc.location_id, test_location_id());
    }

    #[test]
    fn square_device_info() {
        let proc = SquarePaymentProcessor::new(&test_api_key(), &test_location_id());
        let info = proc.device_info();
        assert_eq!(info.vendor, "Square");
        assert_eq!(info.model, "Square API");
    }

    #[test]
    fn square_debug_masks_key() {
        let proc = SquarePaymentProcessor::new(&test_api_key(), &test_location_id());
        let debug = format!("{:?}", proc);
        assert!(!debug.contains("EAAA_test"));
        assert!(debug.contains("***"));
    }

    #[test]
    fn square_parse_payment_success() {
        let json = r#"{
            "payment": {
                "id": "sq_payment_123",
                "status": "COMPLETED",
                "amount_money": {"amount": 1000, "currency": "USD"},
                "created_at": "2026-06-30T12:00:00Z"
            }
        }"#;
        let payment = SquarePaymentProcessor::parse_payment(json).unwrap();
        assert_eq!(payment.id, "sq_payment_123");
        assert_eq!(payment.status, "COMPLETED");
        assert_eq!(payment.amount_money.amount, 1000);
        assert_eq!(payment.created_at.unwrap(), "2026-06-30T12:00:00Z");
    }

    #[test]
    fn square_parse_refund_success() {
        let json = r#"{
            "refund": {
                "id": "sq_refund_456",
                "status": "COMPLETED",
                "amount_money": {"amount": 500, "currency": "USD"}
            }
        }"#;
        let refund = SquarePaymentProcessor::parse_refund(json).unwrap();
        assert_eq!(refund.id, "sq_refund_456");
        assert_eq!(refund.status, "COMPLETED");
        assert_eq!(refund.amount_money.amount, 500);
    }

    #[test]
    fn square_parse_error_body() {
        let json = r#"{
            "errors": [
                {"code": "CARD_DECLINED", "detail": "The card was declined."}
            ]
        }"#;
        let body: SquareErrorBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.errors[0].code.as_deref(), Some("CARD_DECLINED"));
        assert_eq!(
            body.errors[0].detail.as_deref(),
            Some("The card was declined.")
        );
    }

    #[test]
    fn square_parse_error_no_detail() {
        let json = r#"{"errors": [{"code": "NOT_FOUND"}]}"#;
        let body: SquareErrorBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.errors[0].detail, None);
    }

    #[test]
    fn square_parse_error_formats() {
        let err = SquarePaymentProcessor::parse_error(
            402,
            r#"{"errors":[{"code":"CARD_DECLINED","detail":"declined"}]}"#,
        );
        let msg = err.to_string();
        assert!(msg.contains("declined"));
    }

    #[test]
    fn square_parse_error_non_json() {
        let err = SquarePaymentProcessor::parse_error(500, "Internal Server Error");
        let msg = err.to_string();
        assert!(msg.contains("500"));
        assert!(msg.contains("Internal Server Error"));
    }

    #[test]
    fn square_payment_result_completed() {
        let data = PaymentData {
            id: "sq_1".into(),
            status: "COMPLETED".into(),
            amount_money: MoneyAmountResponse {
                amount: 1000,
                currency: "USD".into(),
            },
            created_at: None,
        };
        let (success, money) = SquarePaymentProcessor::payment_result(&data);
        assert!(success);
        assert_eq!(money.minor_units, 1000);
    }

    #[test]
    fn square_payment_result_approved() {
        let data = PaymentData {
            id: "sq_2".into(),
            status: "APPROVED".into(),
            amount_money: MoneyAmountResponse {
                amount: 2000,
                currency: "USD".into(),
            },
            created_at: None,
        };
        let (success, _) = SquarePaymentProcessor::payment_result(&data);
        assert!(success);
    }

    #[test]
    fn square_payment_result_failed() {
        let data = PaymentData {
            id: "sq_3".into(),
            status: "FAILED".into(),
            amount_money: MoneyAmountResponse {
                amount: 500,
                currency: "USD".into(),
            },
            created_at: None,
        };
        let (success, _) = SquarePaymentProcessor::payment_result(&data);
        assert!(!success);
    }

    #[test]
    fn square_to_money_constructs() {
        let m = SquarePaymentProcessor::to_money(1000, "USD");
        assert_eq!(m.minor_units, 1000);
    }

    #[test]
    fn square_to_square_amount() {
        let usd: Currency = "USD".parse().unwrap();
        let amount = Money::from_major(10, usd).unwrap();
        assert_eq!(SquarePaymentProcessor::to_square_amount(&amount), 1000);
    }
}

//! QRIS payment processor — implements [`PaymentProcessor`] using the
//! Midtrans REST API for Indonesian QRIS (Quick Response Code Indonesian
//! Standard) payments.
//!
//! QRIS is the standardized QR code payment system mandated by Bank
//! Indonesia. Customers scan the displayed QR code with any compatible
//! e-wallet (GoPay, OVO, DANA, LinkAja, etc.) and confirm the payment
//! on their phone.
//!
//! # Flow
//!
//! 1. **`authorize`** — Calls the Midtrans Charge API with
//!    `payment_type: "qris"` and returns a `transaction_id`.
//! 2. **`capture`** — Polls the transaction status until `settlement`
//!    or `expire`.
//! 3. **`sale`** — Overridden to authorize + poll immediately (single call).
//! 4. **`refund`** — Submits a refund via the Midtrans Refund API.
//! 5. **`void`** — Cancels a pending transaction via the Cancel API.
//!
//! # Configuration
//!
//! The processor reads `MIDTRANS_SERVER_KEY` from the environment at
//! construction. The server key is found in the Midtrans dashboard
//! under Settings → Access Keys.
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

/// Base URL for the Midtrans API (production).
const MIDTRANS_API_BASE: &str = "https://api.midtrans.com/v2";

/// Base URL for the Midtrans sandbox API.
const MIDTRANS_SANDBOX_BASE: &str = "https://api.sandbox.midtrans.com/v2";

/// Number of milliseconds to wait between status polls.
const POLL_INTERVAL_MS: u64 = 2000;

/// Maximum number of polling attempts before giving up.
const MAX_POLL_ATTEMPTS: u32 = 30;

/// How long a QRIS transaction is valid (in seconds).
const QRIS_EXPIRY_SECS: u64 = 300; // 5 minutes

/// A [`PaymentProcessor`] implementation backed by the Midtrans QRIS API.
///
/// # Example
///
/// ```no_run
/// # use oz_payment::drivers::qris::QrisPaymentProcessor;
/// # use oz_payment::PaymentProcessor;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let request = unimplemented!();
/// let proc = QrisPaymentProcessor::from_env()?;
/// proc.sale(&request).await?;
/// # Ok(())
/// # }
/// ```
pub struct QrisPaymentProcessor {
    client: Arc<reqwest::Client>,
    /// Whether to use the sandbox endpoint.
    sandbox: bool,
    /// Base URL for the Midtrans API (configurable for testing).
    api_base: String,
}

impl fmt::Debug for QrisPaymentProcessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QrisPaymentProcessor")
            .field("client", &self.client)
            .field("server_key", &"***")
            .field("sandbox", &self.sandbox)
            .field("api_base", &self.api_base)
            .finish()
    }
}

impl Clone for QrisPaymentProcessor {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            sandbox: self.sandbox,
            api_base: self.api_base.clone(),
        }
    }
}

/// QRIS charge response from Midtrans.
#[derive(serde::Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct QrisChargeResponse {
    #[serde(default)]
    transaction_id: String,
    #[serde(default)]
    order_id: String,
    #[serde(default)]
    gross_amount: String,
    #[serde(default)]
    transaction_status: String,
    #[serde(default)]
    status_code: String,
    #[serde(default)]
    status_message: String,
    /// The QR code content (URL or raw string to render).
    #[serde(default)]
    qr_code_url: Option<String>,
    /// Actions the POS can take (e.g., "deeplink_redirect").
    #[serde(default)]
    actions: Option<Vec<QrisAction>>,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct QrisAction {
    #[serde(default)]
    name: String,
    #[serde(default)]
    url: String,
}

/// Transaction status response from Midtrans.
#[derive(serde::Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct TransactionStatusResponse {
    #[serde(default)]
    transaction_id: String,
    #[serde(default)]
    order_id: String,
    #[serde(default)]
    gross_amount: String,
    #[serde(default)]
    transaction_status: String,
    #[serde(default)]
    status_code: String,
    #[serde(default)]
    status_message: String,
    #[serde(default)]
    currency: String,
    #[serde(default)]
    payment_type: String,
}

/// Midtrans API error response.
#[derive(serde::Deserialize, Debug)]
struct MidtransErrorResponse {
    #[serde(default)]
    status_code: String,
    #[serde(default)]
    status_message: String,
}

impl QrisPaymentProcessor {
    /// Create a new QRIS payment processor with the given server key.
    ///
    /// When `sandbox` is true, requests go to the Midtrans sandbox
    /// environment (`api.sandbox.midtrans.com`).
    pub fn new(server_key: &str, sandbox: bool) -> Self {
        let api_base = if sandbox {
            MIDTRANS_SANDBOX_BASE
        } else {
            MIDTRANS_API_BASE
        };
        Self::new_with_endpoint(server_key, api_base, sandbox)
    }

    /// Create a new QRIS payment processor with a custom API endpoint.
    ///
    /// This constructor is useful for integration tests where requests
    /// should be directed to a mock server (e.g. `wiremock`).
    pub fn new_with_endpoint(server_key: &str, api_base: &str, sandbox: bool) -> Self {
        let mut headers = HeaderMap::new();
        let encoded = base64_standard(&format!("{}:", server_key));
        let mut auth_value =
            HeaderValue::from_str(&format!("Basic {}", encoded)).unwrap_or_else(|e| {
                tracing::error!(
                    ?e,
                    "invalid Midtrans server key — using placeholder auth header"
                );
                HeaderValue::from_static("Basic placeholder")
            });
        auth_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_value);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .no_proxy()
            .build()
            .unwrap_or_else(|e| {
                tracing::error!(
                    ?e,
                    "failed to build reqwest Client for Midtrans — using default"
                );
                reqwest::Client::new()
            });

        Self {
            client: Arc::new(client),
            sandbox,
            api_base: api_base.to_owned(),
        }
    }

    /// Create a new QRIS processor from the `MIDTRANS_SERVER_KEY`
    /// environment variable.
    pub fn from_env() -> Result<Self, PaymentError> {
        Ok(Self::new(&Self::server_key_from_env()?, false))
    }

    /// Create a new QRIS processor in sandbox mode from the
    /// `MIDTRANS_SERVER_KEY` environment variable.
    pub fn from_env_sandbox() -> Result<Self, PaymentError> {
        Ok(Self::new(&Self::server_key_from_env()?, true))
    }

    /// Read `MIDTRANS_SERVER_KEY` from the environment.
    fn server_key_from_env() -> Result<String, PaymentError> {
        std::env::var("MIDTRANS_SERVER_KEY")
            .map_err(|_| PaymentError::Network("MIDTRANS_SERVER_KEY not set".into()))
    }

    /// The base URL for API calls.
    fn base_url(&self) -> &str {
        &self.api_base
    }

    /// Generate a unique order ID for a QRIS transaction.
    fn generate_order_id() -> String {
        format!(
            "QRIS-{}-{}",
            chrono::Utc::now().timestamp(),
            uuid::Uuid::now_v7().to_string().get(..8).unwrap_or("0000")
        )
    }

    /// Convert a `Money` value to Midtrans' amount format (IDR, no decimals).
    fn to_amount_string(amount: &Money) -> String {
        amount.minor_units.to_string()
    }

    /// Parse an amount string from Midtrans into minor units.
    fn parse_amount(s: &str) -> i64 {
        s.parse().unwrap_or(0)
    }

    /// Perform a JSON POST to the Midtrans API and return (status, body).
    async fn post_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<(u16, String), PaymentError> {
        let url = format!("{}{}", self.base_url(), path);
        let resp = self
            .client
            .post(&url)
            .json(&body)
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

    /// Perform a JSON GET to the Midtrans API and return (status, body).
    async fn get_json(&self, path: &str) -> Result<(u16, String), PaymentError> {
        let url = format!("{}{}", self.base_url(), path);
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

    /// Classify a Midtrans status code into a specific PaymentError variant.
    fn classify_midtrans_status(status_code: &str, status_message: &str) -> PaymentError {
        match status_code {
            "402" => PaymentError::InvalidCard(format!(
                "midtrans card error: {} (code: {})",
                status_message, status_code
            )),
            "406" => PaymentError::Duplicate(format!(
                "midtrans duplicate: {} (code: {})",
                status_message, status_code
            )),
            _ => {
                let msg = if status_message.is_empty() {
                    format!("midtrans_error: HTTP {}", status_code)
                } else {
                    format!("midtrans_error: {} (code: {})", status_message, status_code)
                };
                PaymentError::Network(msg)
            }
        }
    }

    /// Parse a Midtrans API error from the response body.
    fn parse_error(status: u16, body: &str) -> PaymentError {
        if let Ok(err) = serde_json::from_str::<MidtransErrorResponse>(body) {
            Self::classify_midtrans_status(&err.status_code, &err.status_message)
        } else {
            PaymentError::Network(format!("HTTP {}: {}", status, body))
        }
    }

    /// Charge a QRIS payment and return the charge response.
    async fn charge_qris(
        &self,
        request: &PaymentRequest,
        order_id: &str,
    ) -> Result<QrisChargeResponse, PaymentError> {
        let amount_str = Self::to_amount_string(&request.amount);
        let body = serde_json::json!({
            "payment_type": "qris",
            "transaction_details": {
                "order_id": order_id,
                "gross_amount": amount_str
            },
            "qris": {
                "acquirer": "airpay shopee"
            },
            "custom_expiry": {
                "expiry_duration": QRIS_EXPIRY_SECS,
                "unit": "second"
            }
        });

        let (status, text) = self.post_json("/charge", body).await?;
        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &text));
        }

        serde_json::from_str(&text).map_err(|e| {
            PaymentError::InvalidResponse(format!(
                "failed to parse QRIS charge response: {} — body: {}",
                e, text
            ))
        })
    }

    /// Poll the transaction status until settlement or failure.
    async fn poll_status(&self, order_id: &str) -> Result<TransactionStatusResponse, PaymentError> {
        for attempt in 1..=MAX_POLL_ATTEMPTS {
            tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)).await;

            let (status, text) = self.get_json(&format!("/{}/status", order_id)).await?;

            if !(200..300).contains(&status) {
                return Err(Self::parse_error(status, &text));
            }

            if let Ok(tx) = serde_json::from_str::<TransactionStatusResponse>(&text) {
                match tx.transaction_status.as_str() {
                    "settlement" | "capture" => return Ok(tx),
                    "deny" | "cancel" => {
                        let msg = tx.status_message.clone();
                        return Err(PaymentError::Declined(if msg.is_empty() {
                            format!("QRIS payment {}", tx.transaction_status)
                        } else {
                            msg
                        }));
                    }
                    "expire" => {
                        return Err(PaymentError::InvalidCard(tx.status_message.clone()));
                    }
                    _ => {
                        // Still pending — keep polling.
                        if attempt >= MAX_POLL_ATTEMPTS {
                            return Err(PaymentError::Timeout(attempt * POLL_INTERVAL_MS as u32));
                        }
                    }
                }
            }
        }

        Err(PaymentError::Timeout(
            MAX_POLL_ATTEMPTS * POLL_INTERVAL_MS as u32,
        ))
    }
}

#[async_trait]
impl PaymentProcessor for QrisPaymentProcessor {
    /// Generate a QRIS charge and return the transaction details.
    ///
    /// The returned [`PaymentResult`] contains the `transaction_id` and
    /// a message with the QR code URL/content that the POS should display.
    async fn authorize(&self, request: &PaymentRequest) -> Result<PaymentResult, PaymentError> {
        let order_id = Self::generate_order_id();
        let charge = self.charge_qris(request, &order_id).await?;

        let amount = Money {
            minor_units: Self::parse_amount(&charge.gross_amount),
            currency: Currency(*b"IDR"),
        };

        let msg = if let Some(ref qr_url) = charge.qr_code_url {
            format!("{}|{}", charge.transaction_status, qr_url)
        } else {
            charge.transaction_status.clone()
        };

        Ok(PaymentResult {
            success: charge.status_code == "201" || charge.status_code == "200",
            transaction_id: Some(charge.order_id),
            auth_code: None,
            amount_charged: amount,
            message: Some(msg),
        })
    }

    /// Poll for settlement of a QRIS transaction.
    async fn capture(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        let tx = self.poll_status(transaction_id).await?;

        let amount = Money {
            minor_units: Self::parse_amount(&tx.gross_amount),
            currency: Currency(*b"IDR"),
        };

        Ok(PaymentResult {
            success: tx.transaction_status == "settlement" || tx.transaction_status == "capture",
            transaction_id: Some(tx.transaction_id),
            auth_code: None,
            amount_charged: amount,
            message: Some(tx.transaction_status),
        })
    }

    /// Execute a complete QRIS sale: charge + poll for settlement.
    async fn sale(&self, request: &PaymentRequest) -> Result<PaymentResult, PaymentError> {
        let order_id = Self::generate_order_id();
        let charge = self.charge_qris(request, &order_id).await?;

        if charge.status_code != "201" && charge.status_code != "200" {
            return Err(Self::parse_error(
                400,
                &format!("charge failed: {}", charge.status_message),
            ));
        }

        // Return the QR info immediately so the UI can display it.
        // The UI should then call capture() with the order_id to poll.
        let amount = Money {
            minor_units: Self::parse_amount(&charge.gross_amount),
            currency: Currency(*b"IDR"),
        };

        let msg = if let Some(ref qr_url) = charge.qr_code_url {
            format!("SCAN_QR|{}|{}", charge.order_id, qr_url)
        } else {
            format!("SCAN_QR|{}", charge.order_id)
        };

        Ok(PaymentResult {
            success: true,
            transaction_id: Some(charge.order_id),
            auth_code: None,
            amount_charged: amount,
            message: Some(msg),
        })
    }

    /// Refund a settled QRIS transaction.
    async fn refund(
        &self,
        transaction_id: &str,
        _amount: Option<Money>,
    ) -> Result<PaymentResult, PaymentError> {
        let refund_body = serde_json::json!({
            "refund_key": format!("refund-{}-{}", transaction_id, uuid::Uuid::now_v7()),
            "amount": null, // full refund
            "reason": "requested_by_merchant"
        });

        let (status, text) = self
            .post_json(&format!("/{}/refund", transaction_id), refund_body)
            .await?;

        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &text));
        }

        #[derive(serde::Deserialize)]
        struct RefundResponse {
            #[serde(default)]
            transaction_id: String,
            #[serde(default)]
            refund_amount: String,
            #[serde(default)]
            status_code: String,
            #[serde(default)]
            status_message: String,
        }

        let refund: RefundResponse = serde_json::from_str(&text).map_err(|e| {
            PaymentError::InvalidResponse(format!("failed to parse refund: {} — body: {}", e, text))
        })?;

        Ok(PaymentResult {
            success: refund.status_code == "200",
            transaction_id: Some(refund.transaction_id),
            auth_code: None,
            amount_charged: Money {
                minor_units: refund.refund_amount.parse().unwrap_or(0),
                currency: Currency(*b"IDR"),
            },
            message: Some(refund.status_message),
        })
    }

    /// Cancel/void a pending QRIS transaction.
    async fn void(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        let (status, text) = self
            .post_json(
                &format!("/{}/cancel", transaction_id),
                serde_json::json!({}),
            )
            .await?;

        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &text));
        }

        #[derive(serde::Deserialize)]
        struct CancelResponse {
            #[serde(default)]
            transaction_id: String,
            #[serde(default)]
            status_code: String,
            #[serde(default)]
            status_message: String,
        }

        let cancel: CancelResponse = serde_json::from_str(&text).map_err(|e| {
            PaymentError::InvalidResponse(format!("failed to parse cancel: {} — body: {}", e, text))
        })?;

        Ok(PaymentResult {
            success: cancel.status_code == "200",
            transaction_id: Some(cancel.transaction_id),
            auth_code: None,
            amount_charged: Money::zero(Currency(*b"IDR")),
            message: Some(cancel.status_message),
        })
    }

    /// Return a receipt for a completed QRIS transaction.
    async fn receipt(&self, transaction_id: &str) -> Result<PaymentReceipt, PaymentError> {
        let (status, text) = self
            .get_json(&format!("/{}/status", transaction_id))
            .await?;

        if !(200..300).contains(&status) {
            return Err(Self::parse_error(status, &text));
        }

        let tx: TransactionStatusResponse = serde_json::from_str(&text).map_err(|e| {
            PaymentError::InvalidResponse(format!(
                "failed to parse transaction status: {} — body: {}",
                e, text
            ))
        })?;

        let amount = Money {
            minor_units: Self::parse_amount(&tx.gross_amount),
            currency: Currency(*b"IDR"),
        };

        Ok(PaymentReceipt {
            transaction_id: tx.transaction_id,
            method: PaymentMethod::Qr,
            amount,
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            raw_data: None,
        })
    }

    fn device_info(&self) -> DeviceInfo {
        DeviceInfo::new("Midtrans", "QRIS", "cloud")
    }
}

impl Default for QrisPaymentProcessor {
    fn default() -> Self {
        Self::new("", false)
    }
}

/// Standard Base64 encoding (no padding, no line breaks).
fn base64_standard(input: &str) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(input.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> String {
        "MID-server_test_key_123456".to_string()
    }

    #[test]
    fn qris_constructs() {
        let proc = QrisPaymentProcessor::new(&test_key(), false);
        let info = proc.device_info();
        assert_eq!(info.vendor, "Midtrans");
        assert_eq!(info.model, "QRIS");
    }

    #[test]
    fn qris_constructs_sandbox() {
        let proc = QrisPaymentProcessor::new(&test_key(), true);
        assert!(proc.sandbox);
    }

    #[test]
    fn qris_base_url_production() {
        let proc = QrisPaymentProcessor::new(&test_key(), false);
        assert_eq!(proc.base_url(), "https://api.midtrans.com/v2");
    }

    #[test]
    fn qris_base_url_sandbox() {
        let proc = QrisPaymentProcessor::new(&test_key(), true);
        assert_eq!(proc.base_url(), "https://api.sandbox.midtrans.com/v2");
    }

    #[test]
    fn qris_base_url_custom_endpoint() {
        let proc =
            QrisPaymentProcessor::new_with_endpoint("sk_test", "http://localhost:9999", false);
        assert_eq!(proc.base_url(), "http://localhost:9999");
    }

    #[test]
    fn qris_default_constructs() {
        let proc = QrisPaymentProcessor::default();
        assert!(!proc.sandbox);
    }

    #[test]
    fn qris_from_env_missing_key() {
        let result = QrisPaymentProcessor::from_env();
        match std::env::var("MIDTRANS_SERVER_KEY") {
            Ok(_) => assert!(result.is_ok()),
            Err(_) => {
                assert!(result.is_err());
                let msg = result.unwrap_err().to_string();
                assert!(msg.contains("not set"), "error: {}", msg);
            }
        }
    }

    #[test]
    fn qris_generate_order_id_format() {
        let id = QrisPaymentProcessor::generate_order_id();
        assert!(
            id.starts_with("QRIS-"),
            "order id should start with QRIS-: {}",
            id
        );
        assert!(id.len() > 10, "order id should have reasonable length");
    }

    #[test]
    fn qris_to_amount_string() {
        let idr = Currency(*b"IDR");
        let money = Money {
            minor_units: 50000,
            currency: idr,
        };
        assert_eq!(QrisPaymentProcessor::to_amount_string(&money), "50000");
    }

    #[test]
    fn qris_parse_amount() {
        assert_eq!(QrisPaymentProcessor::parse_amount("75000"), 75000);
        assert_eq!(QrisPaymentProcessor::parse_amount("0"), 0);
        assert_eq!(QrisPaymentProcessor::parse_amount("abc"), 0);
    }

    #[test]
    fn qris_parse_charge_response() {
        let json = r#"{
            "status_code": "201",
            "status_message": "QRIS transaction is created",
            "transaction_id": "txn_qris_001",
            "order_id": "QRIS-1234567890-abc",
            "gross_amount": "25000",
            "transaction_status": "pending",
            "qr_code_url": "https://api.midtrans.com/qris/qr-code-abc"
        }"#;
        let resp: QrisChargeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status_code, "201");
        assert_eq!(resp.transaction_id, "txn_qris_001");
        assert_eq!(resp.order_id, "QRIS-1234567890-abc");
        assert_eq!(resp.gross_amount, "25000");
        assert_eq!(resp.transaction_status, "pending");
        assert_eq!(
            resp.qr_code_url.unwrap(),
            "https://api.midtrans.com/qris/qr-code-abc"
        );
    }

    #[test]
    fn qris_parse_charge_response_minimal() {
        let json = r#"{
            "status_code": "201",
            "transaction_id": "txn_002",
            "order_id": "QRIS-xxx",
            "gross_amount": "10000",
            "transaction_status": "pending",
            "status_message": "OK"
        }"#;
        let resp: QrisChargeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.transaction_id, "txn_002");
        assert!(resp.qr_code_url.is_none());
    }

    #[test]
    fn qris_parse_transaction_status() {
        let json = r#"{
            "transaction_id": "txn_003",
            "order_id": "QRIS-abc",
            "gross_amount": "50000",
            "transaction_status": "settlement",
            "status_code": "200",
            "status_message": "Success",
            "currency": "IDR",
            "payment_type": "qris"
        }"#;
        let tx: TransactionStatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(tx.transaction_status, "settlement");
        assert_eq!(tx.gross_amount, "50000");
        assert_eq!(tx.currency, "IDR");
    }

    #[test]
    fn qris_parse_error_response() {
        let json =
            r#"{"status_code": "402", "status_message": "Transaction amount exceeds limit"}"#;
        let err: MidtransErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(err.status_code, "402");
        assert_eq!(err.status_message, "Transaction amount exceeds limit");
    }

    #[test]
    fn qris_parse_error_empty() {
        let json = r#"{"status_code": "500", "status_message": ""}"#;
        let err: MidtransErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(err.status_code, "500");
        assert!(err.status_message.is_empty());
    }

    #[test]
    fn qris_debug_masks_key() {
        let proc = QrisPaymentProcessor::new(&test_key(), false);
        let debug = format!("{:?}", proc);
        assert!(!debug.contains("test_key"));
        assert!(!debug.contains("MID-server"));
        assert!(debug.contains("***"));
    }

    #[test]
    fn qris_clone_preserves_config() {
        let proc = QrisPaymentProcessor::new(&test_key(), true);
        let cloned = proc.clone();
        assert!(cloned.sandbox);
        let info = cloned.device_info();
        assert_eq!(info.vendor, "Midtrans");
    }

    #[test]
    fn qris_base64_encoding() {
        let encoded = base64_standard("test:key");
        assert!(!encoded.is_empty());
        assert!(!encoded.contains('\n'));
    }
}

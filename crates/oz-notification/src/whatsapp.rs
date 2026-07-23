//! WhatsApp Cloud API client implementation.
//!
//! Uses the Meta Graph API v21.0+ to send template messages, text messages,
//! and verify webhook signatures via HMAC-SHA256.
//!
//! # Environment variables
//!
//! - `WHATSAPP_PHONE_NUMBER_ID` — The WhatsApp Business phone number ID
//! - `WHATSAPP_ACCESS_TOKEN` — Permanent or temporary access token
//! - `WHATSAPP_APP_SECRET` — App secret for webhook signature verification

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::{
    NotificationClient, NotificationError, NotificationResult, NotificationStatus,
    TemplateParameter,
};

type HmacSha256 = Hmac<Sha256>;

/// WhatsApp Cloud API client for sending messages via the Meta Graph API.
///
/// # Example
///
/// ```ignore
/// let client = WhatsAppClient::from_env()?;
/// client.send_template(
///     "+6281234567890",
///     "order_confirmed",
///     &[TemplateParameter::text("Order #42"), TemplateParameter::currency("IDR", 50000)],
///     Some("id"),
/// ).await?;
/// ```
#[derive(Debug)]
pub struct WhatsAppClient {
    /// WhatsApp Business phone number ID.
    phone_number_id: String,
    /// WhatsApp Cloud API access token.
    access_token: String,
    /// App secret for webhook verification (optional).
    app_secret: Option<String>,
    /// HTTP client for API requests.
    http_client: reqwest::Client,
    /// Base URL for the WhatsApp Cloud API.
    base_url: String,
}

impl WhatsAppClient {
    /// Create a new WhatsApp client with explicit credentials.
    pub fn new(phone_number_id: impl Into<String>, access_token: impl Into<String>) -> Self {
        Self {
            phone_number_id: phone_number_id.into(),
            access_token: access_token.into(),
            app_secret: None,
            http_client: reqwest::Client::new(),
            base_url: "https://graph.facebook.com/v21.0".into(),
        }
    }

    /// Create a WhatsApp client from environment variables.
    ///
    /// Reads `WHATSAPP_PHONE_NUMBER_ID`, `WHATSAPP_ACCESS_TOKEN`, and
    /// optionally `WHATSAPP_APP_SECRET`.
    pub fn from_env() -> NotificationResult<Self> {
        let phone_number_id = std::env::var("WHATSAPP_PHONE_NUMBER_ID").map_err(|_| {
            NotificationError::Config(
                "WHATSAPP_PHONE_NUMBER_ID environment variable not set".into(),
            )
        })?;
        let access_token = std::env::var("WHATSAPP_ACCESS_TOKEN").map_err(|_| {
            NotificationError::Config("WHATSAPP_ACCESS_TOKEN environment variable not set".into())
        })?;
        let app_secret = std::env::var("WHATSAPP_APP_SECRET").ok();

        Ok(Self {
            phone_number_id,
            access_token,
            app_secret,
            http_client: reqwest::Client::new(),
            base_url: "https://graph.facebook.com/v21.0".into(),
        })
    }

    /// Set the app secret for webhook signature verification.
    pub fn with_app_secret(mut self, secret: impl Into<String>) -> Self {
        self.app_secret = Some(secret.into());
        self
    }

    /// Set a custom HTTP client (useful for testing with wiremock).
    #[doc(hidden)]
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = client;
        self
    }

    /// Validate that a phone number is in international format.
    fn validate_phone(to: &str) -> NotificationResult<()> {
        if to.is_empty() || !to.starts_with('+') {
            return Err(NotificationError::InvalidPhoneNumber(format!(
                "phone number must be in international format (e.g., +6281234567890): {to}"
            )));
        }
        // Must have at least 10 digits after the +
        let digits: String = to.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() < 7 {
            return Err(NotificationError::InvalidPhoneNumber(format!(
                "phone number has too few digits: {to}"
            )));
        }
        Ok(())
    }

    /// Build the API URL for sending messages.
    fn messages_url(&self) -> String {
        format!("{}/{}/messages", self.base_url, self.phone_number_id)
    }

    /// Parse the WhatsApp API response into a NotificationStatus.
    fn parse_response(body: &serde_json::Value) -> NotificationStatus {
        let message_id = body["messages"][0]["id"].as_str().map(|s| s.to_string());
        let accepted = body["messages"][0]["message_status"].as_str() == Some("accepted");
        NotificationStatus {
            message_id,
            accepted,
            status: if accepted { "accepted" } else { "rejected" }.into(),
        }
    }
}

#[async_trait]
impl NotificationClient for WhatsAppClient {
    async fn send_template(
        &self,
        to: &str,
        template_name: &str,
        parameters: &[TemplateParameter],
        language: Option<&str>,
    ) -> NotificationResult<NotificationStatus> {
        Self::validate_phone(to)?;

        let lang_code = language.unwrap_or("id");

        let components = if parameters.is_empty() {
            serde_json::json!([{
                "type": "body",
                "parameters": []
            }])
        } else {
            serde_json::json!([{
                "type": "body",
                "parameters": parameters.iter().map(|p| {
                    match p.param_type.as_str() {
                        "text" => serde_json::json!({
                            "type": "text",
                            "text": p.text.as_deref().unwrap_or("")
                        }),
                        "currency" => serde_json::json!({
                            "type": "currency",
                            "currency": {
                                "fallback_value": p.text.as_deref().unwrap_or(""),
                                "code": "IDR",
                                "amount_1000": 0
                            }
                        }),
                        _ => serde_json::json!({
                            "type": "text",
                            "text": p.text.as_deref().unwrap_or("")
                        })
                    }
                }).collect::<Vec<_>>()
            }])
        };

        let payload = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "template",
            "template": {
                "name": template_name,
                "language": {
                    "code": lang_code
                },
                "components": components
            }
        });

        let response = self
            .http_client
            .post(self.messages_url())
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap_or_default();

        if status.is_success() {
            Ok(Self::parse_response(&body))
        } else {
            let error = body["error"]["message"].as_str().unwrap_or("unknown error");

            if status.as_u16() == 429 {
                Err(NotificationError::RateLimited {
                    retry_after_seconds: 60,
                    message: error.to_string(),
                })
            } else if body["error"]["code"].as_i64() == Some(100) {
                Err(NotificationError::TemplateNotFound(
                    template_name.to_string(),
                ))
            } else {
                Err(NotificationError::Api(error.to_string()))
            }
        }
    }

    async fn send_text(&self, to: &str, body: &str) -> NotificationResult<NotificationStatus> {
        Self::validate_phone(to)?;

        let payload = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": body
            }
        });

        let response = self
            .http_client
            .post(self.messages_url())
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap_or_default();

        if status.is_success() {
            Ok(Self::parse_response(&body))
        } else {
            let error = body["error"]["message"].as_str().unwrap_or("unknown error");
            Err(NotificationError::Api(error.to_string()))
        }
    }

    fn verify_webhook_signature(
        &self,
        payload: &[u8],
        signature_header: &str,
    ) -> NotificationResult<bool> {
        let app_secret = self.app_secret.as_ref().ok_or_else(|| {
            NotificationError::Config(
                "WHATSAPP_APP_SECRET not configured — cannot verify webhook signatures".into(),
            )
        })?;

        // The signature header format is: "sha256=<hex-encoded-hmac>"
        let expected_sig = signature_header
            .strip_prefix("sha256=")
            .unwrap_or(signature_header);

        let mut mac = HmacSha256::new_from_slice(app_secret.as_bytes())
            .map_err(|e| NotificationError::Config(format!("HMAC init failed: {e}")))?;
        mac.update(payload);

        let expected_bytes = hex::decode(expected_sig)
            .map_err(|e| NotificationError::Api(format!("invalid hex signature: {e}")))?;

        Ok(mac.verify_slice(&expected_bytes).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_phone_accepts_valid_number() {
        assert!(WhatsAppClient::validate_phone("+6281234567890").is_ok());
        assert!(WhatsAppClient::validate_phone("+1234567890").is_ok());
    }

    #[test]
    fn validate_phone_rejects_empty() {
        let err = WhatsAppClient::validate_phone("").unwrap_err();
        assert!(err.to_string().contains("international format"));
    }

    #[test]
    fn validate_phone_rejects_no_plus() {
        let err = WhatsAppClient::validate_phone("6281234567890").unwrap_err();
        assert!(err.to_string().contains("international format"));
    }

    #[test]
    fn validate_phone_rejects_too_short() {
        let err = WhatsAppClient::validate_phone("+123").unwrap_err();
        assert!(err.to_string().contains("too few digits"));
    }

    #[test]
    #[serial_test::serial]
    fn from_env_missing_vars_returns_config_error() {
        // Save original values so we can restore after the test.
        // SAFETY: serial_test::serial serializes access to this test,
        // preventing races with other tests that read these env vars.
        let saved_phone_id = std::env::var("WHATSAPP_PHONE_NUMBER_ID").ok();
        let saved_access_token = std::env::var("WHATSAPP_ACCESS_TOKEN").ok();
        let saved_app_secret = std::env::var("WHATSAPP_APP_SECRET").ok();

        // Clear env vars to simulate missing config
        unsafe {
            std::env::remove_var("WHATSAPP_PHONE_NUMBER_ID");
            std::env::remove_var("WHATSAPP_ACCESS_TOKEN");
            std::env::remove_var("WHATSAPP_APP_SECRET");
        }

        let result = WhatsAppClient::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("WHATSAPP_PHONE_NUMBER_ID"));

        // Restore env vars to their previous state.
        // SAFETY: serialize_test::serial ensures exclusive access;
        // restoring prevents leakage to other tests in the same process.
        unsafe {
            if let Some(val) = saved_phone_id {
                std::env::set_var("WHATSAPP_PHONE_NUMBER_ID", val);
            }
            if let Some(val) = saved_access_token {
                std::env::set_var("WHATSAPP_ACCESS_TOKEN", val);
            }
            if let Some(val) = saved_app_secret {
                std::env::set_var("WHATSAPP_APP_SECRET", val);
            }
        }
    }

    #[test]
    fn new_client_has_correct_base_url() {
        let client = WhatsAppClient::new("123456", "token");
        assert!(client.messages_url().contains("123456"));
        assert!(client.messages_url().contains("v21.0"));
    }

    #[test]
    fn webhook_verification_requires_app_secret() {
        let client = WhatsAppClient::new("123456", "token");
        let result = client.verify_webhook_signature(b"payload", "sha256=abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("APP_SECRET"));
    }

    #[test]
    fn webhook_verification_with_valid_signature() {
        let secret = "test-secret-key";
        let payload = b"{\"object\":\"whatsapp_business_account\"}";

        // Compute valid HMAC
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        let client = WhatsAppClient::new("123456", "token").with_app_secret(secret);
        assert!(
            client
                .verify_webhook_signature(payload, &signature)
                .unwrap()
        );
    }

    #[test]
    fn webhook_verification_with_invalid_signature() {
        let client = WhatsAppClient::new("123456", "token").with_app_secret("secret");

        assert!(
            !client
                .verify_webhook_signature(b"payload", "sha256=deadbeef")
                .unwrap()
        );
    }
}

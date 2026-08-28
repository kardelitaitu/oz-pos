//! WhatsApp Cloud API notification client for OZ-POS.
//!
//! Provides a notification abstraction with a mock driver for testing
//! and a real WhatsApp Cloud API client for production use.
//!
//! # Quick start
//!
//! ```ignore
//! use oz_notification::{NotificationClient, WhatsAppClient, MockNotificationClient};
//!
//! // Production
//! let client = WhatsAppClient::new("PHONE_NUMBER_ID", "ACCESS_TOKEN");
//! client.send_template("+1234567890", "order_confirmed", &json!({...})).await?;
//!
//! // Testing
//! let mock = MockNotificationClient::new();
//! mock.send_template("+1234567890", "order_confirmed", &json!({...})).await?;
//! assert_eq!(mock.sent_count(), 1);
//! ```

pub mod handlers;
pub mod mock;
pub mod whatsapp;

use async_trait::async_trait;
use serde::Serialize;
use std::fmt::Debug;

/// Error type for notification operations.
#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    /// The WhatsApp API returned an error response.
    #[error("whatsapp API error: {0}")]
    Api(String),
    /// Network-level failure (connection refused, timeout, DNS).
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    /// Invalid phone number format.
    #[error("invalid phone number: {0}")]
    InvalidPhoneNumber(String),
    /// Template name not found in the WhatsApp Business account.
    #[error("template not found: {0}")]
    TemplateNotFound(String),
    /// Rate limited by the WhatsApp API.
    #[error("rate limited (retry after {retry_after_seconds}s): {message}")]
    RateLimited {
        /// Seconds to wait before retrying.
        retry_after_seconds: u64,
        /// Human-readable message.
        message: String,
    },
    /// Configuration error (missing env vars, invalid credentials).
    #[error("configuration error: {0}")]
    Config(String),
}

/// Result type alias for notification operations.
pub type NotificationResult<T> = Result<T, NotificationError>;

/// Parameters for a WhatsApp template message.
///
/// Each parameter replaces a `{{N}}` placeholder in the template body,
/// header, or buttons. The type field determines how WhatsApp renders
/// the parameter.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateParameter {
    /// The parameter type: "text", "currency", "date_time", "image", "document", "video".
    #[serde(rename = "type")]
    pub param_type: String,
    /// The parameter value (for "text" type) or sub-object (for media types).
    pub text: Option<String>,
}

impl TemplateParameter {
    /// Create a text parameter.
    pub fn text(value: impl Into<String>) -> Self {
        Self {
            param_type: "text".into(),
            text: Some(value.into()),
        }
    }

    /// Create a currency parameter.
    pub fn currency(code: &str, amount: i64) -> Self {
        Self {
            param_type: "currency".into(),
            text: Some(format!("{} {}", amount, code)),
        }
    }
}

/// Notification delivery status returned after sending.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationStatus {
    /// WhatsApp message ID (wa_id) if successfully queued.
    pub message_id: Option<String>,
    /// Whether the message was accepted by the WhatsApp API.
    pub accepted: bool,
    /// Human-readable status description.
    pub status: String,
}

/// Trait for notification clients — WhatsApp, mock, and future providers.
///
/// All methods are async and return `NotificationResult<T>`.
#[async_trait]
pub trait NotificationClient: Debug + Send + Sync {
    /// Send a WhatsApp template message to a phone number.
    ///
    /// # Arguments
    /// - `to`: Recipient phone number in international format (e.g., "+6281234567890").
    /// - `template_name`: Name of the approved WhatsApp template (e.g., "order_confirmed").
    /// - `parameters`: Template body parameters (replaces `{{1}}`, `{{2}}`, etc.).
    /// - `language`: BCP 47 language code (default: "id" for Indonesian).
    async fn send_template(
        &self,
        to: &str,
        template_name: &str,
        parameters: &[TemplateParameter],
        language: Option<&str>,
    ) -> NotificationResult<NotificationStatus>;

    /// Send a free-form text message (requires user-initiated conversation).
    async fn send_text(&self, to: &str, body: &str) -> NotificationResult<NotificationStatus>;

    /// Verify a WhatsApp webhook signature.
    ///
    /// Uses HMAC-SHA256 with the app secret to verify that the webhook
    /// payload came from Meta's servers.
    fn verify_webhook_signature(
        &self,
        payload: &[u8],
        signature_header: &str,
    ) -> NotificationResult<bool>;
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

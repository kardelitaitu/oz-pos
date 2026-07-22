//! Mock notification client for testing.
//!
//! Records all sent messages in memory and returns configurable responses.
//! Use this in unit and integration tests to verify notification behavior
//! without hitting the real WhatsApp API.

use async_trait::async_trait;
use std::sync::Mutex;

use crate::{
    NotificationClient, NotificationError, NotificationResult, NotificationStatus,
    TemplateParameter,
};

/// A recorded notification that was "sent" through the mock client.
#[derive(Debug, Clone, PartialEq)]
pub struct MockNotification {
    /// Recipient phone number.
    pub to: String,
    /// Template name or "text" for free-form messages.
    pub template_name: String,
    /// JSON-serialized parameters.
    pub parameters_json: String,
    /// Language code (if any).
    pub language: Option<String>,
    /// Whether the send was accepted.
    pub accepted: bool,
}

/// Mock notification client that records all sends in memory.
///
/// By default all sends succeed. Call `set_should_fail(true)` to simulate
/// API failures, and `sent_messages()` to inspect recorded sends.
#[derive(Debug)]
pub struct MockNotificationClient {
    /// Recorded sent messages.
    messages: Mutex<Vec<MockNotification>>,
    /// If true, all sends return an error.
    should_fail: Mutex<bool>,
    /// Custom error message when `should_fail` is true.
    fail_message: Mutex<String>,
}

impl MockNotificationClient {
    /// Create a new mock client with all sends succeeding by default.
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
            should_fail: Mutex::new(false),
            fail_message: Mutex::new("mock failure".into()),
        }
    }

    /// Set whether subsequent sends should fail.
    pub fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().unwrap() = fail;
    }

    /// Set the error message returned when sends fail.
    pub fn set_fail_message(&self, msg: impl Into<String>) {
        *self.fail_message.lock().unwrap() = msg.into();
    }

    /// Get all recorded sent messages.
    pub fn sent_messages(&self) -> Vec<MockNotification> {
        self.messages.lock().unwrap().clone()
    }

    /// Get the count of sent messages.
    pub fn sent_count(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    /// Clear all recorded messages.
    pub fn clear(&self) {
        self.messages.lock().unwrap().clear();
    }
}

impl Default for MockNotificationClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NotificationClient for MockNotificationClient {
    async fn send_template(
        &self,
        to: &str,
        template_name: &str,
        parameters: &[TemplateParameter],
        language: Option<&str>,
    ) -> NotificationResult<NotificationStatus> {
        if *self.should_fail.lock().unwrap() {
            return Err(NotificationError::Api(
                self.fail_message.lock().unwrap().clone(),
            ));
        }

        let params_json = serde_json::to_string(parameters).unwrap_or_default();

        self.messages.lock().unwrap().push(MockNotification {
            to: to.to_string(),
            template_name: template_name.to_string(),
            parameters_json: params_json,
            language: language.map(|s| s.to_string()),
            accepted: true,
        });

        Ok(NotificationStatus {
            message_id: Some(format!("mock-msg-{}", self.sent_count())),
            accepted: true,
            status: "accepted".into(),
        })
    }

    async fn send_text(&self, to: &str, body: &str) -> NotificationResult<NotificationStatus> {
        if *self.should_fail.lock().unwrap() {
            return Err(NotificationError::Api(
                self.fail_message.lock().unwrap().clone(),
            ));
        }

        self.messages.lock().unwrap().push(MockNotification {
            to: to.to_string(),
            template_name: "text".into(),
            parameters_json: body.to_string(),
            language: None,
            accepted: true,
        });

        Ok(NotificationStatus {
            message_id: Some(format!("mock-msg-{}", self.sent_count())),
            accepted: true,
            status: "accepted".into(),
        })
    }

    fn verify_webhook_signature(
        &self,
        _payload: &[u8],
        _signature_header: &str,
    ) -> NotificationResult<bool> {
        // Mock always returns true for signature verification
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_sends_and_records() {
        let client = MockNotificationClient::new();
        assert_eq!(client.sent_count(), 0);

        let status = client
            .send_template(
                "+6281234567890",
                "order_confirmed",
                &[TemplateParameter::text("Coffee")],
                Some("id"),
            )
            .await
            .unwrap();

        assert!(status.accepted);
        assert!(status.message_id.is_some());
        assert_eq!(client.sent_count(), 1);

        let msgs = client.sent_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].to, "+6281234567890");
        assert_eq!(msgs[0].template_name, "order_confirmed");
        assert_eq!(msgs[0].language, Some("id".into()));
    }

    #[tokio::test]
    async fn mock_send_text_records_as_text() {
        let client = MockNotificationClient::new();
        client
            .send_text("+6281234567890", "Your order is ready!")
            .await
            .unwrap();

        let msgs = client.sent_messages();
        assert_eq!(msgs[0].template_name, "text");
        assert!(msgs[0].parameters_json.contains("order is ready"));
    }

    #[tokio::test]
    async fn mock_should_fail_returns_error() {
        let client = MockNotificationClient::new();
        client.set_should_fail(true);
        client.set_fail_message("invalid auth token");

        let result = client
            .send_template("+6281234567890", "order_confirmed", &[], None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid auth token"));
        assert_eq!(client.sent_count(), 0);
    }

    #[tokio::test]
    async fn mock_clear_removes_messages() {
        let client = MockNotificationClient::new();
        client.send_text("+621", "msg1").await.unwrap();
        client.send_text("+622", "msg2").await.unwrap();
        assert_eq!(client.sent_count(), 2);

        client.clear();
        assert_eq!(client.sent_count(), 0);
    }

    #[tokio::test]
    async fn mock_verify_webhook_always_ok() {
        let client = MockNotificationClient::new();
        assert!(
            client
                .verify_webhook_signature(b"payload", "sha256=abc")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn mock_multiple_templates() {
        let client = MockNotificationClient::new();
        client
            .send_template(
                "+621",
                "order_confirmed",
                &[
                    TemplateParameter::text("Order #1"),
                    TemplateParameter::currency("IDR", 50000),
                ],
                Some("id"),
            )
            .await
            .unwrap();

        client
            .send_template(
                "+622",
                "payment_receipt",
                &[TemplateParameter::text("Receipt #42")],
                None,
            )
            .await
            .unwrap();

        let msgs = client.sent_messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].template_name, "order_confirmed");
        assert_eq!(msgs[1].template_name, "payment_receipt");
    }
}

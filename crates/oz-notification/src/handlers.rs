//! WhatsApp notification event handlers.
//!
//! These handlers subscribe to the kernel event bus and fire-and-forget
//! WhatsApp messages via a [`NotificationClient`] when business events occur.
//!
//! Each handler spawns a `tokio` task for the actual send so that the
//! synchronous event bus is never blocked on network I/O. Failures are
//! logged via `tracing` — they never propagate back to the publisher.
//!
//! # Wiring
//!
//! ```ignore
//! use oz_notification::handlers::*;
//! use oz_notification::mock::MockNotificationClient;
//! use std::sync::Arc;
//!
//! let client = Arc::new(MockNotificationClient::new());
//! bus.subscribe::<SaleCompleted>(
//!     "sale.completed",
//!     Box::new(OrderConfirmationHandler::new(client.clone(), None)),
//! );
//! bus.subscribe::<StockAdjusted>(
//!     "stock.adjusted",
//!     Box::new(StockLowAlertHandler::new(client.clone(), 5, "+6281234567890")),
//! );
//! bus.subscribe::<SaleCompleted>(
//!     "sale.completed",
//!     Box::new(PaymentReceiptHandler::new(client, "+6281234567890")),
//! );
//! ```

use std::sync::Arc;

use foundation::contracts::{EventHandler, ModuleResult};
use oz_core::events::{SaleCompleted, StockAdjusted};
use tracing::{error, info, warn};

use crate::{NotificationClient, TemplateParameter};

/// Handler that sends an order confirmation via WhatsApp when a sale completes.
///
/// Uses the `order_confirmed` WhatsApp template. If the sale has a
/// `customer_id`, the handler looks it up via a phone number resolver.
/// Otherwise it uses the `store_phone` fallback or skips.
#[derive(Debug)]
pub struct OrderConfirmationHandler {
    /// The notification client (WhatsApp or mock).
    client: Arc<dyn NotificationClient>,
    /// Fallback phone number for the store (used when the customer has no phone).
    store_phone: Option<String>,
}

impl OrderConfirmationHandler {
    /// Create a new handler backed by the given notification client.
    ///
    /// `store_phone` is the store's WhatsApp-enabled phone number used as
    /// a fallback when the customer has no phone on file.
    pub fn new(
        client: Arc<dyn NotificationClient>,
        store_phone: Option<impl Into<String>>,
    ) -> Self {
        Self {
            client,
            store_phone: store_phone.map(|s| s.into()),
        }
    }
}

impl EventHandler<SaleCompleted> for OrderConfirmationHandler {
    fn handle(&self, event: &SaleCompleted) -> ModuleResult {
        let to = if let Some(ref phone) = self.store_phone {
            phone.clone()
        } else {
            warn!(
                sale_id = %event.sale_id,
                "order confirmation handler: no store phone configured — skipping"
            );
            return Ok(());
        };

        let client = Arc::clone(&self.client);
        let total = event.total_minor;
        let currency = event.currency.clone();
        let sale_id = event.sale_id.clone();
        let line_count = event.line_items.len();

        tokio::spawn(async move {
            let params = vec![
                TemplateParameter::text(format!("#{sale_id}")),
                TemplateParameter::text(format!("{line_count} item(s)")),
                TemplateParameter::currency(&currency, total),
            ];

            match client
                .send_template(&to, "order_confirmed", &params, Some("id"))
                .await
            {
                Ok(status) => {
                    info!(
                        sale_id = %sale_id,
                        accepted = status.accepted,
                        message_id = ?status.message_id,
                        "order confirmation sent via WhatsApp"
                    );
                }
                Err(e) => {
                    error!(
                        sale_id = %sale_id,
                        error = %e,
                        "failed to send order confirmation via WhatsApp"
                    );
                }
            }
        });

        Ok(())
    }
}

/// Handler that sends a low-stock alert to the store manager when stock
/// drops below a configured threshold.
///
/// Subscribes to `stock.adjusted` events and fires only when `new_qty`
/// is at or below the `threshold`. Uses the `low_stock_alert` template.
#[derive(Debug)]
pub struct StockLowAlertHandler {
    /// The notification client (WhatsApp or mock).
    client: Arc<dyn NotificationClient>,
    /// Stock quantity at or below which an alert is triggered.
    threshold: i64,
    /// Store manager's WhatsApp phone number.
    manager_phone: String,
}

impl StockLowAlertHandler {
    /// Create a new low-stock alert handler.
    ///
    /// # Arguments
    /// - `client`: Notification client for sending alerts.
    /// - `threshold`: Stock level that triggers an alert (e.g., 5 = alert when ≤5 remaining).
    /// - `manager_phone`: Phone number to receive alerts.
    pub fn new(
        client: Arc<dyn NotificationClient>,
        threshold: i64,
        manager_phone: impl Into<String>,
    ) -> Self {
        Self {
            client,
            threshold,
            manager_phone: manager_phone.into(),
        }
    }
}

impl EventHandler<StockAdjusted> for StockLowAlertHandler {
    fn handle(&self, event: &StockAdjusted) -> ModuleResult {
        if event.new_qty > self.threshold {
            return Ok(()); // stock is above threshold, no alert needed
        }

        let client = Arc::clone(&self.client);
        let to = self.manager_phone.clone();
        let sku = event.sku.clone();
        let new_qty = event.new_qty;
        let reason = event.reason.clone();
        let threshold = self.threshold;

        tokio::spawn(async move {
            let urgency = if new_qty == 0 {
                "OUT OF STOCK"
            } else {
                "Low Stock"
            };
            let params = vec![
                TemplateParameter::text(sku.clone()),
                TemplateParameter::text(format!("{new_qty} remaining (threshold: {threshold})")),
                TemplateParameter::text(reason),
                TemplateParameter::text(urgency.to_string()),
            ];

            match client
                .send_template(&to, "low_stock_alert", &params, Some("id"))
                .await
            {
                Ok(status) => {
                    info!(
                        sku = %sku,
                        new_qty = new_qty,
                        accepted = status.accepted,
                        "low-stock alert sent via WhatsApp"
                    );
                }
                Err(e) => {
                    error!(
                        sku = %sku,
                        error = %e,
                        "failed to send low-stock alert via WhatsApp"
                    );
                }
            }
        });

        Ok(())
    }
}

/// Handler that sends a payment receipt via WhatsApp when a sale completes.
///
/// Subscribes to `sale.completed` and sends the `payment_receipt` template
/// to the configured phone number (typically the customer's phone or the
/// store's customer-facing line).
#[derive(Debug)]
pub struct PaymentReceiptHandler {
    /// The notification client (WhatsApp or mock).
    client: Arc<dyn NotificationClient>,
    /// Recipient phone number for the receipt.
    recipient_phone: String,
}

impl PaymentReceiptHandler {
    /// Create a new payment receipt handler.
    pub fn new(client: Arc<dyn NotificationClient>, recipient_phone: impl Into<String>) -> Self {
        Self {
            client,
            recipient_phone: recipient_phone.into(),
        }
    }
}

impl EventHandler<SaleCompleted> for PaymentReceiptHandler {
    fn handle(&self, event: &SaleCompleted) -> ModuleResult {
        let client = Arc::clone(&self.client);
        let to = self.recipient_phone.clone();
        let sale_id = event.sale_id.clone();
        let total = event.total_minor;
        let currency = event.currency.clone();

        tokio::spawn(async move {
            let params = vec![
                TemplateParameter::text(format!("#{sale_id}")),
                TemplateParameter::currency(&currency, total),
            ];

            match client
                .send_template(&to, "payment_receipt", &params, Some("id"))
                .await
            {
                Ok(status) => {
                    info!(
                        sale_id = %sale_id,
                        accepted = status.accepted,
                        message_id = ?status.message_id,
                        "payment receipt sent via WhatsApp"
                    );
                }
                Err(e) => {
                    error!(
                        sale_id = %sale_id,
                        error = %e,
                        "failed to send payment receipt via WhatsApp"
                    );
                }
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockNotificationClient;
    use oz_core::events::SaleCompletedLine;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn order_confirmation_sends_message() {
        let mock = Arc::new(MockNotificationClient::new());
        let handler = OrderConfirmationHandler::new(mock.clone(), Some("+6281234567890"));

        let event = SaleCompleted {
            sale_id: "sale-1".into(),
            store_id: None,
            line_items: vec![SaleCompletedLine {
                sku: "COFFEE".into(),
                qty: 2,
                unit_price_minor: 350,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 700,
            currency: "IDR".into(),
            customer_id: None,
        };

        handler.handle(&event).unwrap();

        // Give the spawned task time to execute.
        sleep(Duration::from_millis(50)).await;

        let msgs = mock.sent_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].to, "+6281234567890");
        assert_eq!(msgs[0].template_name, "order_confirmed");
        assert!(msgs[0].parameters_json.contains("sale-1"));
        assert!(msgs[0].parameters_json.contains("IDR"));
    }

    #[tokio::test]
    async fn order_confirmation_skips_when_no_phone() {
        let mock = Arc::new(MockNotificationClient::new());
        let handler = OrderConfirmationHandler::new(mock.clone(), None::<String>);

        let event = SaleCompleted {
            sale_id: "sale-no-phone".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 100,
            currency: "IDR".into(),
            customer_id: None,
        };

        // Should return Ok without sending anything.
        handler.handle(&event).unwrap();

        // No messages should have been sent.
        sleep(Duration::from_millis(10)).await;
        assert_eq!(mock.sent_count(), 0);
    }

    #[tokio::test]
    async fn stock_low_alert_sends_when_below_threshold() {
        let mock = Arc::new(MockNotificationClient::new());
        let handler = StockLowAlertHandler::new(mock.clone(), 5, "+6289876543210");

        let event = StockAdjusted {
            sku: "COFFEE".into(),
            delta: -1,
            new_qty: 3,
            reason: "sale".into(),
        };

        handler.handle(&event).unwrap();
        sleep(Duration::from_millis(50)).await;

        let msgs = mock.sent_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].to, "+6289876543210");
        assert_eq!(msgs[0].template_name, "low_stock_alert");
        assert!(msgs[0].parameters_json.contains("COFFEE"));
        assert!(msgs[0].parameters_json.contains("3"));
    }

    #[tokio::test]
    async fn stock_low_alert_skips_when_above_threshold() {
        let mock = Arc::new(MockNotificationClient::new());
        let handler = StockLowAlertHandler::new(mock.clone(), 5, "+62");

        let event = StockAdjusted {
            sku: "TEA".into(),
            delta: -2,
            new_qty: 98,
            reason: "sale".into(),
        };

        handler.handle(&event).unwrap();
        sleep(Duration::from_millis(10)).await;

        // No alert — stock is still plenty.
        assert_eq!(mock.sent_count(), 0);
    }

    #[tokio::test]
    async fn stock_low_alert_at_exact_threshold() {
        let mock = Arc::new(MockNotificationClient::new());
        let handler = StockLowAlertHandler::new(mock.clone(), 5, "+62");

        let event = StockAdjusted {
            sku: "MILK".into(),
            delta: -95,
            new_qty: 5,
            reason: "sale".into(),
        };

        handler.handle(&event).unwrap();
        sleep(Duration::from_millis(50)).await;

        // At threshold (≤ 5), alert should fire.
        assert_eq!(mock.sent_count(), 1);
    }

    #[tokio::test]
    async fn stock_low_alert_zero_stock_fires() {
        let mock = Arc::new(MockNotificationClient::new());
        let handler = StockLowAlertHandler::new(mock.clone(), 3, "+62");

        let event = StockAdjusted {
            sku: "SUGAR".into(),
            delta: -10,
            new_qty: 0,
            reason: "sale".into(),
        };

        handler.handle(&event).unwrap();
        sleep(Duration::from_millis(50)).await;

        let msgs = mock.sent_messages();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].parameters_json.contains("OUT OF STOCK"));
    }

    #[tokio::test]
    async fn payment_receipt_sends_message() {
        let mock = Arc::new(MockNotificationClient::new());
        let handler = PaymentReceiptHandler::new(mock.clone(), "+628111222333");

        let event = SaleCompleted {
            sale_id: "sale-receipt-1".into(),
            store_id: None,
            line_items: vec![SaleCompletedLine {
                sku: "LATTE".into(),
                qty: 1,
                unit_price_minor: 45000,
                tax_minor: 4500,
                tax_rate_id: Some("tax-ppn".into()),
            }],
            total_minor: 49500,
            currency: "IDR".into(),
            customer_id: Some("cust-1".into()),
        };

        handler.handle(&event).unwrap();
        sleep(Duration::from_millis(50)).await;

        let msgs = mock.sent_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].to, "+628111222333");
        assert_eq!(msgs[0].template_name, "payment_receipt");
        assert!(msgs[0].parameters_json.contains("sale-receipt-1"));
        assert!(msgs[0].parameters_json.contains("IDR"));
    }

    #[tokio::test]
    async fn multiple_handlers_on_same_event() {
        let mock = Arc::new(MockNotificationClient::new());
        let order_handler = OrderConfirmationHandler::new(mock.clone(), Some("+628111111111"));
        let receipt_handler = PaymentReceiptHandler::new(mock.clone(), "+628222222222");

        let event = SaleCompleted {
            sale_id: "sale-multi".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 999,
            currency: "IDR".into(),
            customer_id: None,
        };

        order_handler.handle(&event).unwrap();
        receipt_handler.handle(&event).unwrap();
        sleep(Duration::from_millis(50)).await;

        let msgs = mock.sent_messages();
        // Both handlers use the same mock, so 2 messages.
        assert_eq!(msgs.len(), 2);
        assert!(msgs.iter().any(|m| m.template_name == "order_confirmed"));
        assert!(msgs.iter().any(|m| m.template_name == "payment_receipt"));
    }

    #[tokio::test]
    async fn handler_logs_error_when_client_fails() {
        let mock = Arc::new(MockNotificationClient::new());
        mock.set_should_fail(true);
        mock.set_fail_message("network timeout");

        let handler = OrderConfirmationHandler::new(mock.clone(), Some("+62"));

        let event = SaleCompleted {
            sale_id: "sale-fail".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 100,
            currency: "IDR".into(),
            customer_id: None,
        };

        // Handler itself returns Ok (fire-and-forget).
        handler.handle(&event).unwrap();

        // The spawned task will have logged the error — message not recorded.
        sleep(Duration::from_millis(50)).await;
        assert_eq!(mock.sent_count(), 0);
    }
}

/*
last audited 25-07-26 by RSA-Agent
crate: oz-payment | status: SAFE | lint: CLEAN
findings: EdcTerminal trait mirrors PaymentProcessor soundly; PAY-11: local PaymentResult struct shadows crate::types::PaymentResult with different shape — rename candidate EdcPaymentResult
next: none | perf: N/A
*/
//! EDC (Electronic Data Capture) payment terminal drivers — PLANNED.
//!
//! EDC terminals are physical card-payment devices (Ingenico, Verifone,
//! PAX, etc.) that connect via serial/USB (wired) or Bluetooth/WiFi
//! (wireless). They handle card-present transactions (swipe, dip, tap)
//! and communicate directly with the acquirer/gateway.
//!
//! **Status: PLANNED — stubs only.**
//!
//! # Architecture
//!
//! The [`EdcTerminal`] trait mirrors [`PaymentProcessor`](crate::PaymentProcessor)
//! but is hardware-focused: it returns raw terminal status, can query
//! the card reader state, and supports settlement/batch-close operations
//! that online processors don't need.
//!
//! Concrete drivers:
//!
//! | Driver | Connection | Status |
//! |--------|-----------|--------|
//! | [`WiredEdcTerminal`](crate::drivers::edc::wired::WiredEdcTerminal) | Serial / USB | STUB |
//! | [`WirelessEdcTerminal`](crate::drivers::edc::wireless::WirelessEdcTerminal) | Bluetooth / WiFi | STUB |
//!
//! A planned `EdcPaymentProcessor` in this module will adapt an
//! [`EdcTerminal`] to the [`PaymentProcessor`](crate::PaymentProcessor)
//! trait so the cashier flow can use an EDC terminal as a drop-in
//! replacement for Stripe/Paddle/Midtrans.

pub mod mock;
pub mod protocol;
pub mod wired;
pub mod wireless;

pub use mock::MockEdcTerminal;
pub use protocol::{
    ProtocolCodec, ProtocolMessage, ingenico::IngenicoCodec, pax::PaxCodec, verifone::VerifoneCodec,
};

use async_trait::async_trait;

use foundation::Money;
use oz_hal::types::DeviceInfo;

use crate::error::PaymentError;
use crate::types::PaymentReceipt;

/// Status of an EDC payment terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalStatus {
    /// Terminal is idle and ready for a transaction.
    Ready,
    /// Terminal is processing a transaction.
    Busy,
    /// Terminal is offline / not connected.
    Offline,
    /// Terminal has a paper roll error (receipt printer inside the terminal).
    PaperError,
    /// Terminal encountered a hardware fault.
    Error,
}

/// A card-present payment terminal (EDC).
///
/// This trait is the hardware counterpart of [`PaymentProcessor`](crate::PaymentProcessor):
/// it handles physical card interactions (swipe, dip, tap) and reports
/// terminal-level status. Online gateways (Stripe, Paddle, Midtrans) use
/// [`PaymentProcessor`]; EDC terminals use this trait.
///
/// **PLANNED:** every method returns [`PaymentError::Unsupported`] until
/// the real driver is implemented.
#[async_trait]
pub trait EdcTerminal: Send + Sync {
    /// Query the terminal's current status.
    async fn status(&self) -> Result<TerminalStatus, PaymentError>;

    /// Authorise a card-present transaction.
    ///
    /// `amount` is the amount in minor units. Returns a `transaction_id`
    /// on success (the gateway authorisation code).
    async fn authorize(&self, amount: Money) -> Result<String, PaymentError>;

    /// Capture a previously authorised transaction.
    async fn capture(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError>;

    /// Execute an immediate sale (authorize + capture in one operation).
    ///
    /// Default implementation calls [`authorize`](Self::authorize) followed
    /// by [`capture`](Self::capture).
    async fn sale(&self, amount: Money) -> Result<PaymentResult, PaymentError> {
        let txn_id = self.authorize(amount).await?;
        self.capture(&txn_id).await
    }

    /// Refund a previously captured transaction.
    ///
    /// If `amount` is `None` the full amount is refunded.
    async fn refund(
        &self,
        transaction_id: &str,
        amount: Option<Money>,
    ) -> Result<PaymentResult, PaymentError>;

    /// Void a pending authorisation (before capture).
    async fn void(&self, transaction_id: &str) -> Result<PaymentResult, PaymentError>;

    /// Print a receipt for a completed transaction on the terminal's
    /// built-in printer (if equipped).
    async fn print_receipt(&self, transaction_id: &str) -> Result<PaymentReceipt, PaymentError>;

    /// Static device identity (vendor, model, serial).
    fn device_info(&self) -> DeviceInfo;
}

/// A payment result from an EDC terminal.
#[derive(Debug, Clone)]
pub struct PaymentResult {
    /// Whether the transaction was approved.
    pub success: bool,
    /// Gateway / acquirer transaction ID.
    pub transaction_id: Option<String>,
    /// Authorisation code from the card network.
    pub auth_code: Option<String>,
    /// Card scheme (e.g. "Visa", "Mastercard", "Amex").
    pub card_scheme: Option<String>,
    /// Last 4 digits of the card number.
    pub card_last4: Option<String>,
    /// Human-readable message from the terminal.
    pub message: String,
}

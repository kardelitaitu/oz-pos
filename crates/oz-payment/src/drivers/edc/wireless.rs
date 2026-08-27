//! Wireless EDC payment terminal driver — STUB (PLANNED).
//!
//! A card-present payment terminal connected over Bluetooth (SPP/LE) or
//! WiFi (e.g. Ingenico APOS, PAX S920, Verifone P400 in wireless mode).
//!
//! **Status: PLANNED — stub only.** Construction succeeds and tracks the
//! connection target, but every terminal operation returns
//! [`PaymentError::Unsupported`] until the real Bluetooth/WiFi protocol
//! handler is implemented.

use async_trait::async_trait;

use foundation::Money;
use oz_hal::types::DeviceInfo;

use crate::error::PaymentError;
use crate::types::PaymentReceipt;

use super::{EdcTerminal, PaymentResult, TerminalStatus};

/// How the wireless EDC terminal is addressed.
#[derive(Debug, Clone)]
pub enum WirelessTarget {
    /// Bluetooth SPP / LE device address (e.g. MAC).
    Bluetooth(String),
    /// Network address (e.g. `192.168.1.50:9500`).
    Network(String),
}

/// A wireless (Bluetooth / WiFi) EDC payment terminal.
///
/// **STUB:** construction records the connection target, but no link is
/// established yet. See module docs for the planned implementation.
pub struct WirelessEdcTerminal {
    /// PLANNED: how to reach the terminal.
    #[allow(dead_code)]
    target: WirelessTarget,
    /// Device identity reported to the caller.
    info: DeviceInfo,
}

impl WirelessEdcTerminal {
    /// Create a new wireless EDC terminal at the given connection target.
    ///
    /// # STUB
    ///
    /// Construction succeeds and records the target, but does not connect.
    pub fn new(target: WirelessTarget, info: DeviceInfo) -> Self {
        Self { target, info }
    }

    /// Create a new wireless EDC terminal reached over Bluetooth.
    pub fn over_bluetooth(address: impl Into<String>, info: DeviceInfo) -> Self {
        Self::new(WirelessTarget::Bluetooth(address.into()), info)
    }

    /// Create a new wireless EDC terminal reached over the network.
    pub fn over_network(address: impl Into<String>, info: DeviceInfo) -> Self {
        Self::new(WirelessTarget::Network(address.into()), info)
    }
}

#[async_trait]
impl EdcTerminal for WirelessEdcTerminal {
    async fn status(&self) -> Result<TerminalStatus, PaymentError> {
        Err(PaymentError::Unsupported(
            "wireless EDC status — PLANNED, not implemented yet".into(),
        ))
    }

    async fn authorize(&self, _amount: Money) -> Result<String, PaymentError> {
        Err(PaymentError::Unsupported(
            "wireless EDC authorize — PLANNED, not implemented yet".into(),
        ))
    }

    async fn capture(&self, _transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        Err(PaymentError::Unsupported(
            "wireless EDC capture — PLANNED, not implemented yet".into(),
        ))
    }

    async fn refund(
        &self,
        _transaction_id: &str,
        _amount: Option<Money>,
    ) -> Result<PaymentResult, PaymentError> {
        Err(PaymentError::Unsupported(
            "wireless EDC refund — PLANNED, not implemented yet".into(),
        ))
    }

    async fn void(&self, _transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        Err(PaymentError::Unsupported(
            "wireless EDC void — PLANNED, not implemented yet".into(),
        ))
    }

    async fn print_receipt(&self, _transaction_id: &str) -> Result<PaymentReceipt, PaymentError> {
        Err(PaymentError::Unsupported(
            "wireless EDC print_receipt — PLANNED, not implemented yet".into(),
        ))
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

#[cfg(test)]
#[path = "wireless_tests.rs"]
mod tests;

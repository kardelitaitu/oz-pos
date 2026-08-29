/*
last audited 25-07-26 by RSA-Agent
crate: oz-payment | status: SAFE | lint: CLEAN
findings: PLANNED stub — construction validates config only, all ops fail closed with Unsupported
next: none until serial/USB protocol handler | perf: N/A
*/
//! Wired EDC payment terminal driver — STUB (PLANNED).
//!
//! A card-present payment terminal connected over a serial (RS-232) or
//! USB line (e.g. Ingenico iPP320, Verifone VX520, PAX S80 in wired mode).
//!
//! **Status: PLANNED — stub only.** Construction succeeds and tracks the
//! port configuration, but every terminal operation returns
//! [`PaymentError::Unsupported`] until the real serial/USB protocol
//! handler is implemented.

use async_trait::async_trait;

use foundation::Money;
use oz_hal::types::DeviceInfo;

use crate::error::PaymentError;
use crate::types::PaymentReceipt;

use super::{EdcTerminal, PaymentResult, TerminalStatus};

/// Default baud rate for wired EDC terminals.
///
/// PLANNED: most EDC terminals default to 9600 baud on serial; high-speed
/// models support 115200. Configurable via [`Self::new`].
#[allow(dead_code)]
const DEFAULT_BAUD: u32 = 9600;

/// A wired (serial / USB) EDC payment terminal.
///
/// **STUB:** construction validates the port name is non-empty, but no
/// device is opened yet. See module docs for the planned implementation.
pub struct WiredEdcTerminal {
    /// PLANNED: platform-specific device path (e.g. `/dev/ttyUSB0`, `COM3`).
    #[allow(dead_code)]
    port_name: String,
    /// PLANNED: baud rate for the serial link.
    #[allow(dead_code)]
    baud_rate: u32,
    /// Device identity reported to the caller.
    info: DeviceInfo,
}

impl WiredEdcTerminal {
    /// Create a new wired EDC terminal targeting the given serial port.
    ///
    /// # STUB
    ///
    /// Construction succeeds and records the configuration, but does not
    /// open the port.
    pub fn new(port_name: impl Into<String>, baud_rate: u32, info: DeviceInfo) -> Self {
        Self {
            port_name: port_name.into(),
            baud_rate,
            info,
        }
    }

    /// Create a new wired EDC terminal at the default baud rate.
    pub fn new_default(port_name: impl Into<String>, info: DeviceInfo) -> Self {
        Self::new(port_name, DEFAULT_BAUD, info)
    }
}

#[async_trait]
impl EdcTerminal for WiredEdcTerminal {
    async fn status(&self) -> Result<TerminalStatus, PaymentError> {
        Err(PaymentError::Unsupported(
            "wired EDC status — PLANNED, not implemented yet".into(),
        ))
    }

    async fn authorize(&self, _amount: Money) -> Result<String, PaymentError> {
        Err(PaymentError::Unsupported(
            "wired EDC authorize — PLANNED, not implemented yet".into(),
        ))
    }

    async fn capture(&self, _transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        Err(PaymentError::Unsupported(
            "wired EDC capture — PLANNED, not implemented yet".into(),
        ))
    }

    async fn refund(
        &self,
        _transaction_id: &str,
        _amount: Option<Money>,
    ) -> Result<PaymentResult, PaymentError> {
        Err(PaymentError::Unsupported(
            "wired EDC refund — PLANNED, not implemented yet".into(),
        ))
    }

    async fn void(&self, _transaction_id: &str) -> Result<PaymentResult, PaymentError> {
        Err(PaymentError::Unsupported(
            "wired EDC void — PLANNED, not implemented yet".into(),
        ))
    }

    async fn print_receipt(&self, _transaction_id: &str) -> Result<PaymentReceipt, PaymentError> {
        Err(PaymentError::Unsupported(
            "wired EDC print_receipt — PLANNED, not implemented yet".into(),
        ))
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}

#[cfg(test)]
#[path = "wired_tests.rs"]
mod tests;

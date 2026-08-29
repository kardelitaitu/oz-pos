//! Wireless EDC terminal — STUB test placeholder.
//!
//! Tests will be added when the driver is implemented.
//! See `wireless.rs` for the planned API surface.

use crate::drivers::edc::EdcTerminal;
use crate::drivers::edc::wireless::WirelessEdcTerminal;

/// Verify the wireless EDC stub returns unsupported for every operation.
#[tokio::test]
async fn stub_returns_unsupported() {
    let info = oz_hal::types::DeviceInfo::new("WirelessEDC", "STUB", "wlan-edc-001");
    let term = WirelessEdcTerminal::over_network("192.168.1.50:9500", info);

    let result = term.status().await;
    assert!(
        matches!(result, Err(crate::PaymentError::Unsupported(_))),
        "expected Unsupported error, got {result:?}"
    );
}

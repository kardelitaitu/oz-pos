//! Wired EDC terminal — STUB test placeholder.
//!
//! Tests will be added when the driver is implemented.
//! See `wired.rs` for the planned API surface.

use foundation::{Currency, Money};

use crate::drivers::edc::EdcTerminal;
use crate::drivers::edc::wired::WiredEdcTerminal;

fn usd() -> Currency {
    "USD".parse().unwrap()
}

/// Verify the wired EDC stub returns unsupported for every operation.
#[tokio::test]
async fn stub_returns_unsupported() {
    let info = oz_hal::types::DeviceInfo::new("WiredEDC", "STUB", "wired-edc-001");
    let term = WiredEdcTerminal::new_default("COM3", info);
    let amount = Money::from_major(10, usd()).unwrap();

    let result = term.status().await;
    assert!(
        matches!(result, Err(crate::PaymentError::Unsupported(_))),
        "expected Unsupported error, got {result:?}"
    );

    let result = term.sale(amount).await;
    assert!(
        matches!(result, Err(crate::PaymentError::Unsupported(_))),
        "expected Unsupported error, got {result:?}"
    );
}

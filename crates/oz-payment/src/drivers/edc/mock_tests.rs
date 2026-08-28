//! Mock EDC terminal — tests.

use foundation::{Currency, Money};

use super::MockEdcTerminal;
use crate::drivers::edc::EdcTerminal;

fn usd() -> Currency {
    "USD".parse().unwrap()
}

#[tokio::test]
async fn unconfigured_returns_unsupported() {
    let terminal = MockEdcTerminal::new();
    let amount = Money::from_major(10, usd()).unwrap();

    assert!(terminal.authorize(amount).await.is_err());
    assert!(terminal.sale(amount).await.is_err());
    assert!(terminal.status().await.is_err());
}

#[tokio::test]
async fn configured_succeeds_and_counts_calls() {
    let terminal = MockEdcTerminal::new();
    terminal.set_success();
    let amount = Money::from_major(10, usd()).unwrap();

    let txn = terminal.authorize(amount).await.unwrap();
    assert_eq!(txn, "mock-txn-001");
    assert_eq!(terminal.authorize_calls(), 1);

    let result = terminal.sale(amount).await.unwrap();
    assert!(result.success);
    assert!(terminal.sale_calls() >= 1);
}

#[tokio::test]
async fn default_sale_authorizes_then_captures() {
    let terminal = MockEdcTerminal::new();
    terminal.set_success();
    let amount = Money::from_major(5, usd()).unwrap();

    let result = terminal.sale(amount).await.unwrap();
    assert!(result.success);
    assert_eq!(terminal.authorize_calls(), 1);
    assert_eq!(terminal.capture_calls(), 1);
}

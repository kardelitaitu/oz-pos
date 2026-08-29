//! Webhook verifier — tests.
//!
//! The guard is functional (fails closed); per-gateway verifiers are stubs
//! until implemented.

use std::collections::HashMap;

use crate::webhook::{UnverifiedWebhookGuard, WebhookVerifier};

#[tokio::test]
async fn guard_fails_closed() {
    let guard = UnverifiedWebhookGuard;
    let headers = HashMap::new();
    let result = guard.verify(&headers, b"{}").await;
    assert!(
        matches!(result, Err(crate::PaymentError::Unsupported(_))),
        "expected Unsupported error, got {result:?}"
    );
}

//! Payment processor registry — tests.
//!
//! The registry itself is functional (register/lookup); only
//! `build_from_config` is a stub.

use std::sync::Arc;

use crate::PaymentProcessorRegistry;
use crate::drivers::mock::MockPaymentProcessor;

#[tokio::test]
async fn register_and_lookup_roundtrip() {
    let reg = PaymentProcessorRegistry::new();
    let proc: Arc<dyn crate::PaymentProcessor> = Arc::new(MockPaymentProcessor::new());
    reg.register("mock", proc.clone()).await;

    let found = reg.processor("mock").await.expect("registered");
    assert!(Arc::ptr_eq(&found, &proc));

    let names = reg.processor_names().await;
    assert_eq!(names, vec!["mock"]);
}

#[tokio::test]
async fn missing_processor_returns_none() {
    let reg = PaymentProcessorRegistry::new();
    assert!(reg.processor("stripe").await.is_none());
}

#[tokio::test]
async fn build_from_config_is_stub() {
    let reg = PaymentProcessorRegistry::new();
    let result = reg.build_from_config("stripe").await;
    assert!(
        matches!(result, Err(crate::PaymentError::Unsupported(_))),
        "expected Unsupported error"
    );
}

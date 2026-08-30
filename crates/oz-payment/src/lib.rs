/*
last audited 25-07-26 by RSA-Agent
crate: oz-payment | status: SAFE | lint: CLEAN
findings: re-verified 136 unit + 91 integration + 6 doctests pass; PAY-1 HIGH: qris parse_amount unwrap_or(0) zeroes Midtrans decimal amounts ("14500.00" fails i64 parse); PAY-2 idempotency_key ignored by all live drivers; stubs fail closed
next: fix qris amount parsing (PAY-1), honor idempotency keys (PAY-2) | perf: HTTP async/tokio; mock in-memory atomics
*/
#![deny(unsafe_code)]

//! Payment processor abstraction for OZ-POS.
//!
//! `oz-payment` provides a single trait, [`PaymentProcessor`], with
//! vendor-specific implementations for Stripe, Square, and EMV
//! terminals. The cashier's flow uses the trait; switching processors
//! is a config change, not a code change.
//!
//! # Lifecycle
//!
//! 1. Build a [`PaymentRequest`]
//! 2. Call [`authorize`](PaymentProcessor::authorize) to hold funds
//! 3. Call [`capture`](PaymentProcessor::capture) to complete
//! 4. Optionally [`refund`](PaymentProcessor::refund) or [`void`](PaymentProcessor::void)
//!
//! For simple flows [`sale`](PaymentProcessor::sale) combines step 2 + 3.
//!
//! # Testing
//!
//! Use [`MockPaymentProcessor`](drivers::mock::MockPaymentProcessor) in
//! unit tests. It tracks call counts and can simulate declines and
//! timeouts.
//!
//! ```
//! use oz_payment::{PaymentProcessor, drivers::mock::MockPaymentProcessor};
//! ```

pub mod drivers;
pub mod error;
pub mod processor;
pub mod registry;
pub mod types;
pub mod webhook;

pub use error::PaymentError;
pub use processor::PaymentProcessor;
pub use registry::PaymentProcessorRegistry;
pub use types::{PaymentMethod, PaymentReceipt, PaymentRequest, PaymentResult};
pub use webhook::{UnverifiedWebhookGuard, WebhookEvent, WebhookVerifier};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

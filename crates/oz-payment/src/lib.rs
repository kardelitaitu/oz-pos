/*
last audited 19-07-26 by RSA-Agent
crate: oz-payment | status: SAFE | lint: CLEAN
findings: #![deny(unsafe_code)] at crate root. All payment processors (Stripe, Square, QRIS, Mock)
  implemented via async traits with reqwest HTTP; no FFI or raw pointer manipulation.
  120 unit + 64 integration tests pass (184 total). Mock processor covers decline/timeout/approval paths.
next: None | perf: HTTP calls are async/tokio; mock is in-memory with AtomicUsize counters.
*/
#![deny(unsafe_code)]
#![warn(missing_docs)]

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
pub mod types;

pub use error::PaymentError;
pub use processor::PaymentProcessor;
pub use types::{PaymentMethod, PaymentReceipt, PaymentRequest, PaymentResult};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

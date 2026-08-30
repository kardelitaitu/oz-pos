/*
last audited 31-08-26 by TDD-Agent (round M; PAY-2 closed for charges, wiring status documented)
crate: oz-payment | status: SAFE | lint: CLEAN
findings: PAY-2 FIXED for charges — square.rs honors PaymentRequest.idempotency_key in the body (85b97f1d), stripe.rs forwards it as the Idempotency-Key header (788407e5). Still open for refunds in both: the trait gives refund() no PaymentRequest, so there is no caller key to forward. PAY-1 (qris parse_amount) fixed 25-07-26. Correcting the previous stamp's wording: it called stripe/square/qris the "live drivers", which implied they run in production. They do not — no app code constructs a PaymentRequest or calls authorize; the clients use drivers::edc only, and that is a documented mock. Severity of everything in this crate is prospective until the gateway wiring lands, and the module doc now says so where a reader will hit it first. Stubs fail closed.
next: give refund an idempotency key (trait change, touches every driver), qris PAY-2 is conditional on callers supplying keys, partial refund (PAY-3), Stripe decline classification (PAY-4) | perf: HTTP async/tokio; mock in-memory atomics
*/
#![deny(unsafe_code)]

//! Payment processor abstraction for OZ-POS.
//!
//! `oz-payment` provides a single trait, [`PaymentProcessor`], with
//! vendor-specific implementations for Stripe, Square, and EMV
//! terminals. Switching processors is a config change, not a code change.
//!
//! # Wiring status — read this before scoring a bug's severity
//!
//! Nothing in `apps/desktop-client` or `apps/tablet-client` constructs a
//! [`PaymentRequest`] or calls [`authorize`](PaymentProcessor::authorize).
//! An earlier version of this doc said "the cashier's flow uses the trait",
//! which is not true today — and a sentence like that is what makes a reader
//! grade a latent defect as a live money-path outage.
//!
//! What the clients actually take from this crate is `drivers::edc`, and
//! even that is wired to `MockEdcTerminal` in success mode: a documented
//! placeholder until physical hardware support lands
//! (`apps/desktop-client/src/state.rs:181-187`).
//!
//! Defects in the HTTP gateway drivers are still real and worth fixing — the
//! crate is compiled by CI, has a full wiremock suite, and the integration
//! point is plainly intended. But PAY-2 and COR-31 in those drivers have no
//! production caller yet, so they are prospective. Close them before the
//! wiring lands, not after.
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

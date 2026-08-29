//! Driver implementations for payment processors.
//!
//! - `mock` — in-memory mock for testing and offline demo
//! - `stripe` — live Stripe PaymentIntents integration
//! - `square` — live Square Payments API integration
//! - `qris` — Midtrans QRIS (Indonesian QR code standard)
//! - `paddle` — Paddle Billing integration (PLANNED — stub)
//! - `edc` — EDC payment terminal drivers (PLANNED — stubs)

pub mod mock;
pub mod qris;
pub mod square;
pub mod stripe;

/// Paddle Billing payment processor — PLANNED (stub).
#[cfg(feature = "paddle")]
pub mod paddle;

/// EDC (Electronic Data Capture) payment terminal drivers — PLANNED (stubs).
#[cfg(feature = "edc")]
pub mod edc;

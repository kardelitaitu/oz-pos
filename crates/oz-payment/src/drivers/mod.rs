//! Driver implementations for payment processors.
//!
//! - `mock` — in-memory mock for testing and offline demo
//! - `stripe` — live Stripe PaymentIntents integration
//! - `qris` — Midtrans QRIS (Indonesian QR code standard)

pub mod mock;
pub mod qris;
pub mod stripe;

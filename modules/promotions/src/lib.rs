/*
stub module — created as part of the growable-workspace plan
crate: modules-promotions | status: SAFE | lint: CLEAN
findings: No-op Module implementation. No unsafe code, no DB access yet.
next: Migrate the promotion rule engine and cart-time evaluation into this module.
*/

//! Promotions Module — discount rules and campaign evaluation.
//!
//! Owns promotion definitions (buy-X-get-Y, percentage off, fixed amount
//! off, time-limited campaigns) and the cart-time evaluation that turns a
//! matching rule into a discount line.
//!
//! Key types: [`PromotionsModule`] (kernel lifecycle), [`PromotionsError`].
//!
//! ## Stub status
//!
//! This is a **stub**: it registers with the kernel, declares its
//! dependency on `sales`, and logs its lifecycle transitions. The rule
//! engine still lives in `oz-core` and the management screen in
//! `ui/src/features/`. The `promotions-engine` feature flag already gates
//! the capability and itself depends on `discount-engine`.
//!
//! Promotion path — see `modules/README.md`:
//! 1. Move promotion tables and queries into `repository.rs`.
//! 2. Move rule matching and discount computation into `service.rs`,
//!    keeping every amount in `Money` minor units.
//! 3. Evaluate against the cart before tax, matching `foundation::Cart`
//!    ordering rules.

pub mod error;

pub use error::PromotionsError;

use foundation::contracts::{Module, ModuleId, ModuleResult};
use tracing::info;

/// Stable module id, matching the `id` field in `manifest.json`.
pub const MODULE_ID: ModuleId = "promotions";

/// The Promotions module.
///
/// Implements [`Module`] so the kernel can order it after `sales` during
/// load/start and before it during shutdown.
#[derive(Debug, Default)]
pub struct PromotionsModule;

impl PromotionsModule {
    /// Create a new `PromotionsModule`.
    pub fn new() -> Self {
        Self
    }
}

impl Module for PromotionsModule {
    fn id(&self) -> ModuleId {
        MODULE_ID
    }

    fn dependencies(&self) -> &'static [ModuleId] {
        // Promotions are evaluated against a cart owned by sales.
        &["sales"]
    }

    fn on_load(&mut self) -> ModuleResult {
        info!("promotions module: on_load (stub — no handlers registered yet)");
        Ok(())
    }

    fn on_start(&mut self) -> ModuleResult {
        info!("promotions module: on_start (stub)");
        Ok(())
    }

    fn on_stop(&mut self) -> ModuleResult {
        info!("promotions module: on_stop (stub)");
        Ok(())
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

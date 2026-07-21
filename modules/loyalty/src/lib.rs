/*
last audited 2026-07-22 by Buffy
crate: modules-loyalty | status: SAFE | lint: CLEAN
findings: New module implementing Module trait. Re-exports loyalty types from oz-core.
  No unsafe code. 8 unit tests verify lifecycle and kernel integration.
next: Migrate loyalty commands and DB layer into this module | perf: N/A.
*/
#![warn(missing_docs)]

//! Loyalty Module — customer loyalty program and point management.
//!
//! This module owns the loyalty vertical: tier definitions, customer
//! loyalty accounts, point earn/redeem transactions, and tier-based
//! earning multipliers.
//!
//! ## Current state
//!
//! The LoyaltyModule implements the [`Module`] trait and is registered
//! with the kernel during application startup. The underlying backend
//! (domain types, database access, Tauri commands) and frontend
//! (React screens, API calls, Fluent locale) still live in their
//! original locations:
//!
//! - Domain: `crates/oz-core/src/loyalty.rs`
//! - DB: `crates/oz-core/src/db/loyalty.rs`
//! - Commands: `apps/desktop-client/src/commands/` (TBD)
//! - Frontend: `ui/src/features/crm/` (LoyaltyPrograms)
//! - API: `ui/src/api/` (TBD)
//! - Locale: `ui/src/locales/` (TBD)
//!
//! In subsequent phases, these files will be physically moved into
//! `modules/loyalty/` as the module system matures.
//!
//! ## Module manifest
//!
//! See `modules/loyalty/manifest.json` for the module metadata.

//! # Re-exports
//!
//! This module re-exports loyalty domain types from `oz-core` so that
//! consumers can access all loyalty-related types through a single crate:
//!
//! ```
//! # use modules_loyalty::{LoyaltyModule, LoyaltyTier, LoyaltyAccount, LoyaltyTransaction, LoyaltyAccountWithDetails};
//! ```

// Re-export loyalty domain types from oz-core so consumers can
// access them through this module without importing oz-core directly.
pub use oz_core::{LoyaltyAccount, LoyaltyAccountWithDetails, LoyaltyTier, LoyaltyTransaction};

use std::fmt::Debug;

use foundation::contracts::{Module, ModuleResult};
use tracing::info;

/// The Loyalty module.
///
/// Implements the [`Module`] trait to participate in the kernel
/// lifecycle. Currently acts as a registration and configuration
/// layer; the actual loyalty logic lives in the existing codebase
/// and will be migrated into this module in upcoming phases.
#[derive(Debug)]
pub struct LoyaltyModule;

impl LoyaltyModule {
    /// Create a new LoyaltyModule instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoyaltyModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for LoyaltyModule {
    fn id(&self) -> &'static str {
        "loyalty"
    }

    fn on_load(&mut self) -> ModuleResult {
        info!("loyalty module: on_load — validating configuration");
        // In future phases, this will:
        // 1. Register event handlers (e.g., sale.completed → earn_points)
        // 2. Validate that loyalty tiers seed data exists
        // 3. Check that the CRM module is available
        Ok(())
    }

    fn on_start(&mut self) -> ModuleResult {
        info!("loyalty module: on_start — ready to process loyalty operations");
        // In future phases, this will:
        // 1. Start background point-expiry checker
        // 2. Cache tier definitions for fast lookup
        Ok(())
    }

    fn on_stop(&mut self) -> ModuleResult {
        info!("loyalty module: on_stop — cleaning up");
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use platform_kernel::Kernel;

    #[test]
    fn loyalty_module_id() {
        let module = LoyaltyModule::new();
        assert_eq!(module.id(), "loyalty");
    }

    #[test]
    fn loyalty_module_lifecycle() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(LoyaltyModule::new())).unwrap();
        assert!(kernel.is_registered("loyalty"));
        assert_eq!(kernel.module_count(), 1);

        kernel.load_all().unwrap();
        assert!(kernel.is_loaded());

        kernel.start_all().unwrap();
        assert!(kernel.is_started());

        kernel.stop_all().unwrap();
        assert!(!kernel.is_loaded());
        assert!(!kernel.is_started());
    }

    #[test]
    fn loyalty_module_duplicate_registration_fails() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(LoyaltyModule::new())).unwrap();
        let err = kernel.register(Box::new(LoyaltyModule::new()));
        assert!(err.is_err());
    }

    #[test]
    fn loyalty_module_on_load_succeeds() {
        let mut module = LoyaltyModule::new();
        assert!(module.on_load().is_ok());
    }

    #[test]
    fn loyalty_module_on_start_succeeds() {
        let mut module = LoyaltyModule::new();
        assert!(module.on_start().is_ok());
    }

    #[test]
    fn loyalty_module_on_stop_succeeds() {
        let mut module = LoyaltyModule::new();
        assert!(module.on_stop().is_ok());
    }

    #[test]
    fn loyalty_module_full_lifecycle_with_kernel() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(LoyaltyModule::new())).unwrap();

        kernel.load_all().unwrap();
        kernel.start_all().unwrap();
        kernel.stop_all().unwrap();

        assert!(kernel.is_registered("loyalty"));
    }

    #[test]
    fn multiple_modules_can_coexist() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(LoyaltyModule::new())).unwrap();
        kernel.register(Box::new(OtherModule)).unwrap();

        assert!(kernel.is_registered("loyalty"));
        assert!(kernel.is_registered("other"));
        assert_eq!(kernel.module_count(), 2);
    }

    #[derive(Debug)]
    struct OtherModule;

    impl Module for OtherModule {
        fn id(&self) -> &'static str {
            "other"
        }
        fn on_load(&mut self) -> ModuleResult {
            Ok(())
        }
        fn on_start(&mut self) -> ModuleResult {
            Ok(())
        }
        fn on_stop(&mut self) -> ModuleResult {
            Ok(())
        }
    }
}

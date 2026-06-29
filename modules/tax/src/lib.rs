//! Tax Module — tax rate configuration and management.
//!
//! This module owns the tax vertical: tax rate CRUD, product and
//! category tax assignments, and tax calculation helpers.
//!
//! ## Current state
//!
//! The TaxModule implements the [`Module`] trait and is registered
//! with the kernel during application startup. The underlying backend
//! (DB CRUD, Tauri commands) and frontend (React screens, API calls,
//! Fluent locale) still live in their original locations:
//!
//! - Backend: `crates/oz-core/src/db/tax.rs` + `src-tauri/src/commands/tax.rs`
//! - Frontend: `ui/src/features/tax/`
//! - API: `ui/src/api/tax.ts`
//! - Locale: `ui/src/locales/{en,fr,es,de,zh,ja}/tax.ftl`
//!
//! In subsequent phases, these files will be physically moved into
//! `modules/tax/` as the module system matures.
//!
//! ## Module manifest
//!
//! See `modules/tax/manifest.json` for the module metadata.

//! # Re-exports
//!
//! This module re-exports key tax domain types from `oz-core` so that
//! consumers can access all tax-related types through a single crate:
//!
//! ```ignore
//! use modules_tax::{TaxModule, TaxRate};
//! ```

// Re-export key tax domain types from oz-core so consumers can
// access tax types through this module without importing oz-core.
pub use oz_core::tax_rate::TaxRate;

use std::fmt::Debug;

use foundation::contracts::{Module, ModuleResult};
use tracing::info;

/// The Tax module.
///
/// Implements the [`Module`] trait to participate in the kernel
/// lifecycle. Currently acts as a registration and configuration
/// layer; the actual tax logic lives in the existing codebase
/// and will be migrated into this module in upcoming phases.
#[derive(Debug)]
pub struct TaxModule;

impl TaxModule {
    /// Create a new TaxModule instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaxModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for TaxModule {
    fn id(&self) -> &'static str {
        "tax"
    }

    fn on_load(&mut self) -> ModuleResult {
        info!("tax module: on_load — validating configuration");
        // In future phases, this will:
        // 1. Register event handlers with the event bus
        // 2. Validate that the database has the required tables
        Ok(())
    }

    fn on_start(&mut self) -> ModuleResult {
        info!("tax module: on_start — ready to manage tax rates");
        Ok(())
    }

    fn on_stop(&mut self) -> ModuleResult {
        info!("tax module: on_stop — cleaning up");
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use platform_kernel::Kernel;

    #[test]
    fn tax_module_id() {
        let module = TaxModule::new();
        assert_eq!(module.id(), "tax");
    }

    #[test]
    fn tax_module_lifecycle() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TaxModule::new())).unwrap();
        assert!(kernel.is_registered("tax"));
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
    fn tax_module_duplicate_registration_fails() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TaxModule::new())).unwrap();
        let err = kernel.register(Box::new(TaxModule::new()));
        assert!(err.is_err());
    }

    #[test]
    fn tax_module_on_load_succeeds() {
        let mut module = TaxModule::new();
        assert!(module.on_load().is_ok());
    }

    #[test]
    fn tax_module_on_start_succeeds() {
        let mut module = TaxModule::new();
        assert!(module.on_start().is_ok());
    }

    #[test]
    fn tax_module_on_stop_succeeds() {
        let mut module = TaxModule::new();
        assert!(module.on_stop().is_ok());
    }

    #[test]
    fn tax_module_full_lifecycle_with_kernel() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TaxModule::new())).unwrap();

        kernel.load_all().unwrap();
        kernel.start_all().unwrap();
        kernel.stop_all().unwrap();

        assert!(kernel.is_registered("tax"));
    }

    #[test]
    fn multiple_modules_can_coexist() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TaxModule::new())).unwrap();
        kernel.register(Box::new(OtherModule)).unwrap();

        assert!(kernel.is_registered("tax"));
        assert!(kernel.is_registered("other"));
        assert_eq!(kernel.module_count(), 2);
    }

    #[derive(Debug)]
    struct OtherModule;

    impl Module for OtherModule {
        fn id(&self) -> &'static str { "other" }
        fn on_load(&mut self) -> ModuleResult { Ok(()) }
        fn on_start(&mut self) -> ModuleResult { Ok(()) }
        fn on_stop(&mut self) -> ModuleResult { Ok(()) }
    }
}

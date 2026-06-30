//! Currency/Exchange Module — ISO-4217 currencies and exchange rates.
//!
//! This module owns the currency and exchange rate management
//! vertical: ISO-4217 currency table, default currency, and
//! exchange rate CRUD for multi-currency transactions.
//!
//! ## Current state
//!
//! The CurrencyModule implements the [`Module`] trait and is registered
//! with the kernel during application startup. The underlying backend
//! and frontend still live in their original locations:
//!
//! - Backend: `crates/oz-core/src/exchange_rate.rs` + `crates/oz-core/src/db/settings.rs` (exchange rate methods)
//! - Commands: `src-tauri/src/commands/currencies.rs` + `src-tauri/src/commands/exchange_rates.rs`
//! - Frontend: `ui/src/features/currency/`
//! - API: `ui/src/api/currency.ts`
//!
//! In subsequent phases, these files will be physically moved into
//! `modules/currency/` as the module system matures.
//!
//! ## Module manifest
//!
//! See `modules/currency/manifest.json` for the module metadata.

// Re-export key currency/exchange domain types from oz-core.
pub use foundation::money::Currency;
pub use oz_core::exchange_rate::ExchangeRateRow;

use std::fmt::Debug;

use foundation::contracts::{Module, ModuleResult};
use tracing::info;

/// The Currency/Exchange module.
///
/// Implements the [`Module`] trait to participate in the kernel
/// lifecycle. Currently acts as a registration and configuration
/// layer; the actual currency logic lives in the existing codebase.
#[derive(Debug)]
pub struct CurrencyModule;

impl CurrencyModule {
    /// Create a new CurrencyModule instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CurrencyModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for CurrencyModule {
    fn id(&self) -> &'static str {
        "currency"
    }

    fn on_load(&mut self) -> ModuleResult {
        info!("currency module: on_load — validating configuration");
        // In future phases, this will:
        // 1. Validate exchange rate configuration
        // 2. Register event handlers (e.g., cache latest rates)
        Ok(())
    }

    fn on_start(&mut self) -> ModuleResult {
        info!("currency module: on_start — ready for currency operations");
        // In future phases, this will:
        // 1. Initialize exchange rate cache
        // 2. Register scheduled updates for auto-sync rates
        Ok(())
    }

    fn on_stop(&mut self) -> ModuleResult {
        info!("currency module: on_stop — cleaning up");
        // In future phases, this will:
        // 1. Flush exchange rate cache
        // 2. Persist in-memory state
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use platform_kernel::Kernel;

    #[test]
    fn currency_module_id() {
        let module = CurrencyModule::new();
        assert_eq!(module.id(), "currency");
    }

    #[test]
    fn currency_module_lifecycle() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(CurrencyModule::new())).unwrap();
        assert!(kernel.is_registered("currency"));
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
    fn currency_module_duplicate_registration_fails() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(CurrencyModule::new())).unwrap();
        let err = kernel.register(Box::new(CurrencyModule::new()));
        assert!(err.is_err());
    }

    #[test]
    fn currency_module_on_load_succeeds() {
        let mut module = CurrencyModule::new();
        assert!(module.on_load().is_ok());
    }

    #[test]
    fn currency_module_on_start_succeeds() {
        let mut module = CurrencyModule::new();
        assert!(module.on_start().is_ok());
    }

    #[test]
    fn currency_module_on_stop_succeeds() {
        let mut module = CurrencyModule::new();
        assert!(module.on_stop().is_ok());
    }

    #[test]
    fn currency_module_full_lifecycle_with_kernel() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(CurrencyModule::new())).unwrap();

        // load → start → stop
        kernel.load_all().unwrap();
        kernel.start_all().unwrap();
        kernel.stop_all().unwrap();

        // Module is still registered after stop
        assert!(kernel.is_registered("currency"));
    }

    #[test]
    fn multiple_modules_can_coexist() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(CurrencyModule::new())).unwrap();
        kernel.register(Box::new(OtherModule)).unwrap();

        // Verify both are registered
        assert!(kernel.is_registered("currency"));
        assert!(kernel.is_registered("other"));
        assert_eq!(kernel.module_count(), 2);
    }

    /// Minimal module for coexistence test.
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

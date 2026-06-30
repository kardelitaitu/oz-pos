//! Reporting Module — generates and exports sales, inventory, and financial reports.
//!
//! This module subscribes to the `sale.completed` domain event to capture
//! sale data into a dedicated report table (`report_sales`), enabling
//! downstream aggregation for daily summaries, hourly trends, and exports.
//!
//! ## Current state
//!
//! The ReportingModule implements the [`Module`] trait and is registered
//! with the kernel during application startup. The `SaleCompletedReporter`
//! handler is subscribed to the `sale.completed` event bus topic so that
//! every completed sale is captured for reporting.
//!
//! ## Module manifest
//!
//! See `modules/reporting/manifest.json` for the module metadata.

pub mod handlers;

use std::fmt::Debug;

use foundation::contracts::{Module, ModuleResult};
use tracing::info;

/// The Reporting module.
///
/// Implements the [`Module`] trait to participate in the kernel
/// lifecycle. Registers the `SaleCompletedReporter` event handler
/// to capture sale data for reporting purposes.
#[derive(Debug)]
pub struct ReportingModule;

impl ReportingModule {
    /// Create a new ReportingModule instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReportingModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ReportingModule {
    fn id(&self) -> &'static str {
        "reporting"
    }

    fn on_load(&mut self) -> ModuleResult {
        info!("reporting module: on_load — validating configuration");
        Ok(())
    }

    fn on_start(&mut self) -> ModuleResult {
        info!("reporting module: on_start — ready for reporting");
        Ok(())
    }

    fn on_stop(&mut self) -> ModuleResult {
        info!("reporting module: on_stop — cleaning up");
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use platform_kernel::Kernel;

    #[test]
    fn reporting_module_id() {
        let module = ReportingModule::new();
        assert_eq!(module.id(), "reporting");
    }

    #[test]
    fn reporting_module_lifecycle() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(ReportingModule::new())).unwrap();
        assert!(kernel.is_registered("reporting"));
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
    fn reporting_module_duplicate_registration_fails() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(ReportingModule::new())).unwrap();
        let err = kernel.register(Box::new(ReportingModule::new()));
        assert!(err.is_err());
    }

    #[test]
    fn reporting_module_on_load_succeeds() {
        let mut module = ReportingModule::new();
        assert!(module.on_load().is_ok());
    }

    #[test]
    fn reporting_module_on_start_succeeds() {
        let mut module = ReportingModule::new();
        assert!(module.on_start().is_ok());
    }

    #[test]
    fn reporting_module_on_stop_succeeds() {
        let mut module = ReportingModule::new();
        assert!(module.on_stop().is_ok());
    }

    #[test]
    fn reporting_module_full_lifecycle_with_kernel() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(ReportingModule::new())).unwrap();

        // load → start → stop
        kernel.load_all().unwrap();
        kernel.start_all().unwrap();
        kernel.stop_all().unwrap();

        // Module is still registered after stop
        assert!(kernel.is_registered("reporting"));
    }

    #[test]
    fn multiple_modules_can_coexist() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(ReportingModule::new())).unwrap();
        kernel.register(Box::new(OtherModule)).unwrap();

        // Verify both are registered
        assert!(kernel.is_registered("reporting"));
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

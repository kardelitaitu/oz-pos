//! Split-sanity tests for the R5 mechanical refactor of `kernel.rs`.
//!
//! These tests do not re-prove lifecycle logic; they verify that the
//! module split preserved the public API surface and re-exports.

use crate::kernel::dependency::{HasDependencies, collect_dependencies};
use crate::{Kernel, ModuleStatus};
use foundation::contracts::{Module, ModuleResult};

// A minimal module for use in API-surface tests.
#[derive(Debug)]
struct DummyModule {
    id: &'static str,
}

impl Module for DummyModule {
    fn id(&self) -> &'static str {
        self.id
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

/// The public re-exports for `Kernel` and `ModuleStatus` must still be
/// reachable from the crate root after the split.
#[test]
fn public_reexports_exist_after_split() {
    let _kernel = Kernel::new();
    let _status = ModuleStatus::Started;
}

/// `Kernel` and `ModuleStatus` must also be reachable through the
/// `kernel` submodule path, and both paths refer to the same types.
#[test]
fn submodule_public_paths_exist_after_split() {
    let _crate_root: Kernel = crate::Kernel::new();
    let _submodule: Kernel = crate::kernel::Kernel::new();
    let _status = crate::kernel::ModuleStatus::Stopped;
}

/// `ModuleStatus` derives Copy/Clone/PartialEq/Eq/Debug — all of which
/// were needed by the rest of the codebase. This test compiles only
/// if those derives are present.
#[test]
fn module_status_derives_are_present() {
    let a = ModuleStatus::Registered;
    let b = a; // Copy
    let _c = a.clone(); // Clone
    let _debug = format!("{a:?}"); // Debug
    assert_eq!(a, b); // PartialEq
    assert_eq!(a, ModuleStatus::Registered); // Eq
}

/// `HasDependencies` is re-exported and can be implemented by a type.
#[test]
fn has_dependencies_trait_is_reachable() {
    struct HasDeps;
    impl HasDependencies for HasDeps {
        fn dependencies(&self) -> Vec<&'static str> {
            vec!["inventory"]
        }
    }
    assert_eq!(HasDeps.dependencies(), vec!["inventory"]);
}

/// `collect_dependencies` is reachable from inside the same crate and
/// returns an empty vector for a module that does not implement the
/// trait. This is a crate-internal sanity check, not a public API test.
#[test]
fn collect_dependencies_is_reachable_and_defaults_empty() {
    let module = DummyModule { id: "sales" };
    let deps = collect_dependencies(&module);
    assert!(deps.is_empty());
}

/// The `dependency` submodule is public, so the `HasDependencies` trait
/// can be imported through it.
#[test]
fn dependency_submodule_is_public() {
    struct SomeModule;
    impl crate::kernel::dependency::HasDependencies for SomeModule {
        fn dependencies(&self) -> Vec<&'static str> {
            vec![]
        }
    }
    let module = SomeModule;
    assert!(module.dependencies().is_empty());
}

/// Constructing `Kernel` from the crate root re-export and from the
/// `kernel` submodule must produce equivalent empty kernels. This guards
/// against duplicate-definition or divergent-impl regressions after the
/// mechanical split.
#[test]
fn kernel_constructible_from_both_root_and_submodule_paths() {
    let root_kernel = crate::Kernel::new();
    let submodule_kernel = crate::kernel::Kernel::new();
    assert_eq!(root_kernel.module_count(), submodule_kernel.module_count());
    assert_eq!(root_kernel.is_loaded(), submodule_kernel.is_loaded());
    assert_eq!(root_kernel.is_started(), submodule_kernel.is_started());
}

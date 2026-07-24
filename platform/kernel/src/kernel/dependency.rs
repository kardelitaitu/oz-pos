//! Module dependency declaration helpers.

use foundation::contracts::Module;

/// Trait for modules that declare dependencies on other modules.
///
/// Modules that depend on other modules should implement this trait
/// and return the IDs of their dependencies. The kernel uses this
/// to resolve the correct load/start/stop ordering.
///
/// Modules that do NOT implement this trait are assumed to have zero
/// dependencies.
pub trait HasDependencies {
    /// Module IDs that this module depends on.
    fn dependencies(&self) -> Vec<&'static str>;
}

/// Collect the dependency IDs declared by a module.
///
/// Modules declare dependencies through the [`HasDependencies`] trait.
/// If the module does not implement that trait, it is assumed to
/// have zero dependencies.
///
/// Note: in this phase, dependency declaration is not yet integrated
/// into the [`Module`] trait itself. The `HasDependencies` trait is
/// a separate opt-in mechanism. A future upgrade will add a
/// `dependencies()` method to `Module` directly.
pub(crate) fn collect_dependencies(_module: &dyn Module) -> Vec<&'static str> {
    // Downcasting from `&dyn Module` to `&dyn HasDependencies` is not
    // directly possible without `Any` bounds on `Module`. In the next
    // phase, `Module` will gain a `dependencies()` method that returns
    // a `Vec<&'static str>`, making this simpler.
    //
    // For now, all modules are assumed to have zero dependencies.
    Vec::new()
}

// DEPRECATED: HasDependencies trait and module_dependencies function
// will be added in Phase 2.2 when the Module trait gains a
// dependencies() method. Until then, all modules are assumed to
// have zero dependencies.

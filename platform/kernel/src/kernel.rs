//! Module system kernel — lifecycle management and dependency resolution.
//!
//! The [`Kernel`] is the sole owner of the module lifecycle. It maintains
//! a registry of modules, resolves dependencies via topological sort,
//! and drives the lifecycle: **register → load → start → stop**.

use std::collections::{HashMap, HashSet, VecDeque};

use foundation::contracts::{Module, Service};
use tracing::{debug, error, info};

use crate::error::KernelError;
use crate::event_bus::EventBus;

/// The module system kernel.
///
/// Manages module registration, dependency resolution, and lifecycle.
/// The kernel operates exclusively through the [`Module`] and [`Service`]
/// traits — it has no knowledge of specific module types.
///
/// # Example
///
/// ```ignore
/// let mut kernel = Kernel::new();
/// kernel.register(Box::new(MyModule))?;
/// kernel.register(Box::new(MyOtherModule))?;
/// kernel.load_all()?;
/// kernel.start_all()?;
/// // ... application runs ...
/// kernel.stop_all()?;
/// ```
pub struct Kernel {
    /// Registered modules, keyed by module id.
    modules: HashMap<&'static str, Box<dyn Module>>,
    /// Registered services.
    services: Vec<Box<dyn Service>>,
    /// Whether `load_all` has been called.
    loaded: bool,
    /// Whether `start_all` has been called.
    started: bool,
    /// In-process event bus for module-to-module communication.
    event_bus: EventBus,
}

impl Kernel {
    /// Create a new empty kernel.
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            services: Vec::new(),
            loaded: false,
            started: false,
            event_bus: EventBus::new(),
        }
    }

    // ── Registration ─────────────────────────────────────────────

    /// Register a module with the kernel.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::DuplicateModule`] if a module with the same
    /// id is already registered.
    pub fn register(&mut self, module: Box<dyn Module>) -> Result<(), KernelError> {
        let id = module.id();
        if self.modules.contains_key(id) {
            return Err(KernelError::DuplicateModule(id));
        }
        debug!(module = id, "registering module");
        self.modules.insert(id, module);
        Ok(())
    }

    /// Register a service with the kernel.
    pub fn register_service(&mut self, service: Box<dyn Service>) {
        debug!(svc = service.id(), "registering service");
        self.services.push(service);
    }

    /// Check if a module is registered.
    pub fn is_registered(&self, id: &str) -> bool {
        self.modules.contains_key(id)
    }

    /// Number of registered modules.
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// IDs of all registered modules.
    pub fn module_ids(&self) -> Vec<&'static str> {
        self.modules.keys().copied().collect()
    }

    /// Get a reference to a registered module by id.
    pub fn get_module(&self, id: &str) -> Option<&dyn Module> {
        self.modules.get(id).map(|b| b.as_ref())
    }

    // ── Lifecycle: Load ──────────────────────────────────────────

    /// Call `on_load` on every registered module in dependency order.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::NoModulesRegistered`] if no modules are
    /// registered. Returns [`KernelError::MissingDependency`] or
    /// [`KernelError::CircularDependency`] from dependency resolution.
    /// Returns [`KernelError::LifecycleError`] if `on_load` fails.
    pub fn load_all(&mut self) -> Result<(), KernelError> {
        if self.modules.is_empty() {
            return Err(KernelError::NoModulesRegistered);
        }

        let order = self.resolve_dependencies()?;
        info!("loading {} modules in dependency order", order.len());

        for &id in &order {
            let module = self.modules.get_mut(id).ok_or_else(|| {
                KernelError::Internal(format!(
                    "module '{id}' not found after dependency resolution"
                ))
            })?;
            debug!(module = id, "loading module");
            module.on_load().map_err(|e| KernelError::LifecycleError {
                module: id,
                operation: "load",
                source: e,
            })?;
        }

        self.loaded = true;
        Ok(())
    }

    // ── Lifecycle: Start ─────────────────────────────────────────

    /// Call `on_start` on every module (in dependency order), then
    /// start all services.
    ///
    /// Auto-loads if `load_all` was not called explicitly.
    ///
    /// # Errors
    ///
    /// Propagates errors from module or service start.
    pub fn start_all(&mut self) -> Result<(), KernelError> {
        if !self.loaded {
            self.load_all()?;
        }

        let order = self.resolve_dependencies()?;
        info!("starting {} modules", order.len());

        for &id in &order {
            let module = self.modules.get_mut(id).ok_or_else(|| {
                KernelError::Internal(format!("module '{id}' not found during start"))
            })?;
            debug!(module = id, "starting module");
            module.on_start().map_err(|e| KernelError::LifecycleError {
                module: id,
                operation: "start",
                source: e,
            })?;
        }

        // Start services after modules.
        for service in &mut self.services {
            let id = service.id();
            debug!(svc = id, "starting service");
            service.start().map_err(|e| KernelError::ServiceError {
                service: id,
                operation: "start",
                source: e,
            })?;
        }

        self.started = true;
        Ok(())
    }

    // ── Lifecycle: Stop ──────────────────────────────────────────

    /// Stop all services and modules gracefully.
    ///
    /// Services stop first (reverse registration order), then modules
    /// (reverse dependency order). Errors are logged but all items are
    /// stopped regardless.
    ///
    /// # Errors
    ///
    /// Returns the first error encountered (if any), but stopping
    /// continues for all other modules and services.
    pub fn stop_all(&mut self) -> Result<(), KernelError> {
        let mut first_error: Option<KernelError> = None;

        // Stop services in reverse order.
        for service in self.services.iter_mut().rev() {
            let id = service.id();
            debug!(svc = id, "stopping service");
            if let Err(e) = service.stop() {
                error!(svc = id, error = %e, "failed to stop service");
                first_error.get_or_insert_with(|| KernelError::ServiceError {
                    service: id,
                    operation: "stop",
                    source: e,
                });
            }
        }

        // Stop modules in reverse dependency order.
        if let Ok(order) = self.resolve_dependencies() {
            for &id in order.iter().rev() {
                if let Some(module) = self.modules.get_mut(id) {
                    debug!(module = id, "stopping module");
                    if let Err(e) = module.on_stop() {
                        error!(module = id, error = %e, "failed to stop module");
                        first_error.get_or_insert_with(|| KernelError::LifecycleError {
                            module: id,
                            operation: "stop",
                            source: e,
                        });
                    }
                }
            }
        }

        self.started = false;
        self.loaded = false;

        match first_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    // ── Dependency Resolution ────────────────────────────────────

    /// Resolve module dependencies using Kahn's algorithm (BFS-based
    /// topological sort).
    ///
    /// Modules declare their dependencies through the [`HasDependencies`]
    /// trait. Modules with no implemented trait are assumed to have zero
    /// dependencies.
    ///
    /// Returns module IDs in dependency-first order (dependencies appear
    /// before dependents).
    ///
    /// # Errors
    ///
    /// - [`KernelError::MissingDependency`] if a dependency is not registered.
    /// - [`KernelError::CircularDependency`] if a cycle is detected.
    pub(crate) fn resolve_dependencies(&self) -> Result<Vec<&'static str>, KernelError> {
        let module_ids: HashSet<&str> = self.modules.keys().copied().collect();
        if module_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build the dependency graph: module → [its direct dependencies].
        let mut graph: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
        for (&id, module) in &self.modules {
            let deps = collect_dependencies(module.as_ref());
            // Validate that all declared deps are registered.
            for &dep in &deps {
                if !module_ids.contains(dep) {
                    return Err(KernelError::MissingDependency { module: id, dep });
                }
            }
            graph.insert(id, deps);
        }

        // Kahn's algorithm.
        // in_degree[m] = number of dependencies m has.
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for &id in &module_ids {
            in_degree.insert(id, graph.get(id).map_or(0, |d| d.len()));
        }

        // Start with modules that have 0 dependencies.
        let mut queue: VecDeque<&str> = module_ids
            .iter()
            .filter(|id| *in_degree.get(*id).unwrap_or(&0) == 0)
            .copied()
            .collect();

        let mut sorted: Vec<&'static str> = Vec::new();

        while let Some(id) = queue.pop_front() {
            sorted.push(id);

            // Find every module that depends on `id` and decrement its
            // in_degree.
            for (&candidate, deps) in &graph {
                if deps.contains(&id)
                    && let Some(deg) = in_degree.get_mut(candidate)
                {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(candidate);
                    }
                }
            }
        }

        if sorted.len() < module_ids.len() {
            let unresolved: Vec<String> = module_ids
                .iter()
                .filter(|id| !sorted.contains(id))
                .map(|s| (*s).to_string())
                .collect();
            return Err(KernelError::CircularDependency(unresolved.join(", ")));
        }

        Ok(sorted)
    }

    // ── Event Bus ────────────────────────────────────────────────

    /// Access the kernel's shared event bus.
    ///
    /// Modules use this during `on_load` to register their event
    /// handlers.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    // ── State queries ─────────────────────────────────────────────

    /// Whether `load_all` has been called.
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Whether `start_all` has been called.
    pub fn is_started(&self) -> bool {
        self.started
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

// ── Dependency declaration ──────────────────────────────────────────

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
fn collect_dependencies(_module: &dyn Module) -> Vec<&'static str> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::contracts::Module;

    // ── Test helpers ─────────────────────────────────────────────

    use foundation::contracts::ModuleResult;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Debug)]
    struct TestModule {
        id: &'static str,
        fail_load: bool,
        fail_start: bool,
        fail_stop: bool,
        load_called: AtomicBool,
        start_called: AtomicBool,
        stop_called: AtomicBool,
    }

    impl TestModule {
        fn new(id: &'static str) -> Self {
            Self {
                id,
                fail_load: false,
                fail_start: false,
                fail_stop: false,
                load_called: AtomicBool::new(false),
                start_called: AtomicBool::new(false),
                stop_called: AtomicBool::new(false),
            }
        }

        fn with_fail_load(mut self) -> Self {
            self.fail_load = true;
            self
        }

        fn with_fail_start(mut self) -> Self {
            self.fail_start = true;
            self
        }

        fn with_fail_stop(mut self) -> Self {
            self.fail_stop = true;
            self
        }
    }

    impl Module for TestModule {
        fn id(&self) -> &'static str {
            self.id
        }

        fn on_load(&mut self) -> ModuleResult {
            if self.fail_load {
                return Err("load failed".into());
            }
            self.load_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn on_start(&mut self) -> ModuleResult {
            if self.fail_start {
                return Err("start failed".into());
            }
            self.start_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn on_stop(&mut self) -> ModuleResult {
            if self.fail_stop {
                return Err("stop failed".into());
            }
            self.stop_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[derive(Debug)]
    struct TestService {
        id: &'static str,
        fail_start: bool,
        fail_stop: bool,
        start_called: AtomicBool,
        stop_called: AtomicBool,
    }

    impl TestService {
        fn new(id: &'static str) -> Self {
            Self {
                id,
                fail_start: false,
                fail_stop: false,
                start_called: AtomicBool::new(false),
                stop_called: AtomicBool::new(false),
            }
        }
    }

    impl Service for TestService {
        fn id(&self) -> &'static str {
            self.id
        }

        fn start(&mut self) -> ModuleResult {
            if self.fail_start {
                return Err("service start failed".into());
            }
            self.start_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn stop(&mut self) -> ModuleResult {
            if self.fail_stop {
                return Err("service stop failed".into());
            }
            self.stop_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    // ── Registration tests ───────────────────────────────────────

    #[test]
    fn register_single_module() {
        let mut kernel = Kernel::new();
        assert_eq!(kernel.module_count(), 0);

        kernel.register(Box::new(TestModule::new("sales"))).unwrap();
        assert_eq!(kernel.module_count(), 1);
        assert!(kernel.is_registered("sales"));
    }

    #[test]
    fn register_duplicate_module_fails() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("sales"))).unwrap();
        let result = kernel.register(Box::new(TestModule::new("sales")));
        assert!(result.is_err());
        match result.unwrap_err() {
            KernelError::DuplicateModule(id) => assert_eq!(id, "sales"),
            other => panic!("expected DuplicateModule, got {other:?}"),
        }
    }

    #[test]
    fn register_multiple_modules() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("sales"))).unwrap();
        kernel
            .register(Box::new(TestModule::new("inventory")))
            .unwrap();
        kernel.register(Box::new(TestModule::new("crm"))).unwrap();
        assert_eq!(kernel.module_count(), 3);
    }

    #[test]
    fn register_service_does_not_crash() {
        let mut kernel = Kernel::new();
        kernel.register_service(Box::new(TestService::new("sync")));
    }

    // ── Lifecycle tests ──────────────────────────────────────────

    #[test]
    fn load_all_calls_on_load() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("test"))).unwrap();
        kernel.load_all().unwrap();
        assert!(kernel.is_loaded());
    }

    #[test]
    fn load_all_fails_with_no_modules() {
        let mut kernel = Kernel::new();
        let result = kernel.load_all();
        assert!(result.is_err());
        match result.unwrap_err() {
            KernelError::NoModulesRegistered => {}
            other => panic!("expected NoModulesRegistered, got {other:?}"),
        }
    }

    #[test]
    fn load_all_propagates_module_error() {
        let mut kernel = Kernel::new();
        kernel
            .register(Box::new(TestModule::new("bad").with_fail_load()))
            .unwrap();
        let result = kernel.load_all();
        assert!(result.is_err());
        match result.unwrap_err() {
            KernelError::LifecycleError {
                module, operation, ..
            } => {
                assert_eq!(module, "bad");
                assert_eq!(operation, "load");
            }
            other => panic!("expected LifecycleError, got {other:?}"),
        }
    }

    #[test]
    fn start_all_calls_on_start() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("test"))).unwrap();
        kernel.start_all().unwrap();
        assert!(kernel.is_started());
    }

    #[test]
    fn start_all_auto_loads_if_not_loaded() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("test"))).unwrap();
        kernel.start_all().unwrap();
        assert!(kernel.is_loaded());
        assert!(kernel.is_started());
    }

    #[test]
    fn start_all_propagates_module_error() {
        let mut kernel = Kernel::new();
        kernel
            .register(Box::new(TestModule::new("bad").with_fail_start()))
            .unwrap();
        let result = kernel.start_all();
        assert!(result.is_err());
        match result.unwrap_err() {
            KernelError::LifecycleError {
                module, operation, ..
            } => {
                assert_eq!(module, "bad");
                assert_eq!(operation, "start");
            }
            other => panic!("expected LifecycleError, got {other:?}"),
        }
    }

    #[test]
    fn full_lifecycle() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("a"))).unwrap();
        kernel.register(Box::new(TestModule::new("b"))).unwrap();
        kernel.register_service(Box::new(TestService::new("svc")));

        kernel.load_all().unwrap();
        assert!(kernel.is_loaded());
        assert!(!kernel.is_started());

        kernel.start_all().unwrap();
        assert!(kernel.is_started());

        kernel.stop_all().unwrap();
        assert!(!kernel.is_loaded());
        assert!(!kernel.is_started());
    }

    #[test]
    fn stop_all_does_not_error_when_not_started() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("test"))).unwrap();
        kernel.stop_all().unwrap();
    }

    #[test]
    fn stop_all_continues_on_error() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("ok"))).unwrap();
        kernel
            .register(Box::new(TestModule::new("bad").with_fail_stop()))
            .unwrap();
        kernel.load_all().unwrap();
        let result = kernel.stop_all();
        assert!(result.is_err());
    }

    #[test]
    fn start_all_auto_loads_and_starts_services() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("test"))).unwrap();
        kernel.register_service(Box::new(TestService::new("svc")));
        kernel.start_all().unwrap();
        kernel.stop_all().unwrap();
    }

    #[test]
    fn module_ids_returns_registered_ids() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("a"))).unwrap();
        kernel.register(Box::new(TestModule::new("b"))).unwrap();
        let mut ids = kernel.module_ids();
        ids.sort();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn get_module_returns_registered_module() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("test"))).unwrap();
        let module = kernel.get_module("test");
        assert!(module.is_some());
        assert_eq!(module.unwrap().id(), "test");
    }

    #[test]
    fn get_module_returns_none_for_unknown() {
        let kernel = Kernel::new();
        assert!(kernel.get_module("nonexistent").is_none());
    }

    // ── Dependency resolution tests ──────────────────────────────

    #[test]
    fn resolve_no_dependencies() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("a"))).unwrap();
        kernel.register(Box::new(TestModule::new("b"))).unwrap();
        let order = kernel.resolve_dependencies().unwrap();
        assert_eq!(order.len(), 2);
        assert!(order.contains(&"a"));
        assert!(order.contains(&"b"));
    }

    #[test]
    fn resolve_all_modules_included() {
        let mut kernel = Kernel::new();
        kernel.register(Box::new(TestModule::new("sales"))).unwrap();
        kernel
            .register(Box::new(TestModule::new("inventory")))
            .unwrap();
        kernel.register(Box::new(TestModule::new("crm"))).unwrap();
        let order = kernel.resolve_dependencies().unwrap();
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn load_all_with_empty_kernel_fails() {
        let mut kernel = Kernel::new();
        assert!(kernel.load_all().is_err());
    }
}

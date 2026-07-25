use super::*;
use crate::KernelError;
use foundation::contracts::{Module, ModuleResult, Service};
use std::sync::atomic::{AtomicBool, Ordering};

// ── Test helpers ─────────────────────────────────────────────

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
            return Err(anyhow::anyhow!("load failed"));
        }
        self.load_called.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn on_start(&mut self) -> ModuleResult {
        if self.fail_start {
            return Err(anyhow::anyhow!("start failed"));
        }
        self.start_called.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn on_stop(&mut self) -> ModuleResult {
        if self.fail_stop {
            return Err(anyhow::anyhow!("stop failed"));
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
            return Err(anyhow::anyhow!("service start failed"));
        }
        self.start_called.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&mut self) -> ModuleResult {
        if self.fail_stop {
            return Err(anyhow::anyhow!("service stop failed"));
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

/// Regression test for TDD Bug #1: when dependency resolution fails
/// during shutdown (e.g. circular dependency), `stop_all` must still
/// attempt to stop all modules rather than silently skipping shutdown.
///
/// With the current `collect_dependencies` stub (always empty),
/// `resolve_dependencies` never fails in practice. But the code path
/// is defensive — if a future version introduces real dependency
/// resolution, the fallback ensures modules are always stopped.
#[test]
fn stop_all_stops_modules_even_when_dep_resolution_would_fail() {
    let mut kernel = Kernel::new();
    let module = TestModule::new("survivor");
    kernel.register(Box::new(module)).unwrap();
    kernel.load_all().unwrap();

    // Even without starting, stop_all should update status and not panic.
    kernel.stop_all().unwrap();

    assert_eq!(
        kernel.module_status("survivor"),
        Some(ModuleStatus::Stopped),
        "module should be Stopped after stop_all"
    );
    assert!(!kernel.is_loaded());
    assert!(!kernel.is_started());
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

// ── Initial state / Default ─────────────────────────────────

#[test]
fn kernel_default_creates_empty_kernel() {
    let kernel = Kernel::default();
    assert_eq!(kernel.module_count(), 0);
    assert!(!kernel.is_loaded());
    assert!(!kernel.is_started());
}

#[test]
fn kernel_new_initial_state() {
    let kernel = Kernel::new();
    assert_eq!(kernel.module_count(), 0);
    assert!(!kernel.is_loaded());
    assert!(!kernel.is_started());
    assert!(kernel.module_ids().is_empty());
}

#[test]
fn is_registered_returns_false_for_unknown() {
    let kernel = Kernel::new();
    assert!(!kernel.is_registered("nonexistent"));
}

// ── ModuleStatus tests ──────────────────────────────────────

#[test]
fn register_sets_status_to_registered() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("test"))).unwrap();
    assert_eq!(kernel.module_status("test"), Some(ModuleStatus::Registered));
}

#[test]
fn load_all_updates_status_to_loaded() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("test"))).unwrap();
    kernel.load_all().unwrap();
    assert_eq!(kernel.module_status("test"), Some(ModuleStatus::Loaded));
}

#[test]
fn start_all_updates_status_to_started() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("test"))).unwrap();
    kernel.start_all().unwrap();
    assert_eq!(kernel.module_status("test"), Some(ModuleStatus::Started));
}

#[test]
fn stop_all_updates_status_to_stopped() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("test"))).unwrap();
    kernel.start_all().unwrap();
    kernel.stop_all().unwrap();
    assert_eq!(kernel.module_status("test"), Some(ModuleStatus::Stopped));
}

#[test]
fn module_status_unknown_for_unregistered() {
    let kernel = Kernel::new();
    assert_eq!(kernel.module_status("nonexistent"), None);
}

#[test]
fn all_statuses_returns_all_registered() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("a"))).unwrap();
    kernel.register(Box::new(TestModule::new("b"))).unwrap();
    let statuses = kernel.all_statuses();
    assert_eq!(statuses.len(), 2);
    assert_eq!(statuses.get("a"), Some(&ModuleStatus::Registered));
}

#[test]
fn status_transition_register_load_start_stop() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("m"))).unwrap();
    assert_eq!(kernel.module_status("m"), Some(ModuleStatus::Registered));

    kernel.load_all().unwrap();
    assert_eq!(kernel.module_status("m"), Some(ModuleStatus::Loaded));

    kernel.start_all().unwrap();
    assert_eq!(kernel.module_status("m"), Some(ModuleStatus::Started));

    kernel.stop_all().unwrap();
    assert_eq!(kernel.module_status("m"), Some(ModuleStatus::Stopped));
}

// ── Individual start/stop tests ──────────────────────────────

#[test]
fn start_module_starts_single_module() {
    let mut kernel = Kernel::new();
    let module = TestModule::new("test");
    kernel.register(Box::new(module)).unwrap();
    kernel.load_all().unwrap();

    kernel.start_module("test").unwrap();
    assert_eq!(kernel.module_status("test"), Some(ModuleStatus::Started));
}

#[test]
fn start_module_fails_if_not_registered() {
    let mut kernel = Kernel::new();
    let result = kernel.start_module("nonexistent");
    assert!(result.is_err());
}

#[test]
fn start_module_fails_if_registered_only() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("test"))).unwrap();
    let result = kernel.start_module("test");
    assert!(result.is_err());
    // Status should remain Registered.
    assert_eq!(kernel.module_status("test"), Some(ModuleStatus::Registered));
}

#[test]
fn start_module_fails_if_already_started() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("test"))).unwrap();
    kernel.start_all().unwrap();
    let result = kernel.start_module("test");
    assert!(result.is_err());
}

#[test]
fn stop_module_stops_single_module() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("test"))).unwrap();
    kernel.start_all().unwrap();

    kernel.stop_module("test").unwrap();
    assert_eq!(kernel.module_status("test"), Some(ModuleStatus::Stopped));
}

#[test]
fn stop_module_fails_if_not_registered() {
    let mut kernel = Kernel::new();
    let result = kernel.stop_module("nonexistent");
    assert!(result.is_err());
}

#[test]
fn stop_module_fails_if_not_started() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("test"))).unwrap();
    kernel.load_all().unwrap();
    let result = kernel.stop_module("test");
    assert!(result.is_err());
    // Status should remain Loaded.
    assert_eq!(kernel.module_status("test"), Some(ModuleStatus::Loaded));
}

#[test]
fn start_module_allows_restart_after_stop() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("test"))).unwrap();
    kernel.start_all().unwrap();
    kernel.stop_module("test").unwrap();
    assert_eq!(kernel.module_status("test"), Some(ModuleStatus::Stopped));

    // Can restart a stopped module.
    kernel.start_module("test").unwrap();
    assert_eq!(kernel.module_status("test"), Some(ModuleStatus::Started));
}

#[test]
fn start_module_propagates_lifecycle_error() {
    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(TestModule::new("bad").with_fail_start()))
        .unwrap();
    kernel.load_all().unwrap();

    let result = kernel.start_module("bad");
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
fn stop_module_propagates_lifecycle_error() {
    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(TestModule::new("bad").with_fail_stop()))
        .unwrap();
    kernel.start_all().unwrap();

    let result = kernel.stop_module("bad");
    assert!(result.is_err());
    match result.unwrap_err() {
        KernelError::LifecycleError {
            module, operation, ..
        } => {
            assert_eq!(module, "bad");
            assert_eq!(operation, "stop");
        }
        other => panic!("expected LifecycleError, got {other:?}"),
    }
}

#[test]
fn event_bus_returns_reference() {
    let kernel = Kernel::new();
    let _bus = kernel.event_bus();
}

#[test]
fn default_equals_new() {
    let k1 = Kernel::new();
    let k2 = Kernel::default();
    assert_eq!(k1.module_count(), k2.module_count());
    assert_eq!(k1.is_loaded(), k2.is_loaded());
    assert_eq!(k1.is_started(), k2.is_started());
}

// ── TDD: stop_all must unsubscribe module handlers from EventBus ─
//
// Bug: Kernel::stop_all() stops modules and services but never
// calls event_bus.unsubscribe_module(id) for each stopped module.
// Stopped modules continue to receive events — their handlers
// remain registered on the bus, potentially accessing freed
// resources or wasting cycles.
//
// This test subscribes a handler for a module, stops the kernel,
// then publishes an event and asserts the handler was NOT called.

#[test]
fn stop_all_unsubscribes_module_handlers_from_event_bus() {
    use foundation::contracts::{DomainEvent, EventHandler};

    #[derive(Debug, Clone)]
    struct TestBusEvent {
        _value: i32,
    }
    impl DomainEvent for TestBusEvent {
        fn event_name(&self) -> &'static str {
            "test.bus.event"
        }
    }

    struct BusHandler {
        called: AtomicBool,
    }
    impl EventHandler<TestBusEvent> for BusHandler {
        fn handle(&self, _event: &TestBusEvent) -> ModuleResult {
            self.called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }
    impl EventHandler<TestBusEvent> for std::sync::Arc<BusHandler> {
        fn handle(&self, event: &TestBusEvent) -> ModuleResult {
            (**self).handle(event)
        }
    }

    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(TestModule::new("test-mod")))
        .unwrap();
    kernel.start_all().unwrap();

    // Subscribe a handler owned by "test-mod".
    let handler = std::sync::Arc::new(BusHandler {
        called: AtomicBool::new(false),
    });
    kernel.event_bus().subscribe_for_module(
        "test-mod",
        "test.bus.event",
        Box::new(handler.clone()),
    );
    assert_eq!(
        kernel.event_bus().handler_count_for_module("test-mod"),
        1,
        "handler should be registered before stop"
    );

    // Stop the kernel — this must unsubscribe "test-mod" handlers.
    kernel.stop_all().unwrap();

    // The handler should have been removed from the bus.
    assert_eq!(
        kernel.event_bus().handler_count_for_module("test-mod"),
        0,
        "stop_all must unsubscribe module handlers from the event bus"
    );

    // Publish an event — the stopped module's handler must NOT fire.
    kernel
        .event_bus()
        .publish(&TestBusEvent { _value: 42 })
        .unwrap();
    assert!(
        !handler.called.load(Ordering::SeqCst),
        "stopped module's handler must not be called after stop_all"
    );
}

#[test]
fn stop_module_unsubscribes_that_modules_handlers() {
    use foundation::contracts::{DomainEvent, EventHandler};

    #[derive(Debug, Clone)]
    struct TestStopEvent {
        _value: i32,
    }
    impl DomainEvent for TestStopEvent {
        fn event_name(&self) -> &'static str {
            "test.stop.event"
        }
    }

    struct StopHandler {
        called: AtomicBool,
    }
    impl EventHandler<TestStopEvent> for StopHandler {
        fn handle(&self, _event: &TestStopEvent) -> ModuleResult {
            self.called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }
    impl EventHandler<TestStopEvent> for std::sync::Arc<StopHandler> {
        fn handle(&self, event: &TestStopEvent) -> ModuleResult {
            (**self).handle(event)
        }
    }

    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("mod-a"))).unwrap();
    kernel.register(Box::new(TestModule::new("mod-b"))).unwrap();
    kernel.start_all().unwrap();

    let handler_a = std::sync::Arc::new(StopHandler {
        called: AtomicBool::new(false),
    });
    kernel.event_bus().subscribe_for_module(
        "mod-a",
        "test.stop.event",
        Box::new(handler_a.clone()),
    );

    let handler_b = std::sync::Arc::new(StopHandler {
        called: AtomicBool::new(false),
    });
    kernel.event_bus().subscribe_for_module(
        "mod-b",
        "test.stop.event",
        Box::new(handler_b.clone()),
    );

    // Stop only mod-a — its handlers must be removed, mod-b's kept.
    kernel.stop_module("mod-a").unwrap();

    assert_eq!(
        kernel.event_bus().handler_count_for_module("mod-a"),
        0,
        "stop_module must unsubscribe that module's handlers"
    );
    assert_eq!(
        kernel.event_bus().handler_count_for_module("mod-b"),
        1,
        "other modules' handlers must remain"
    );

    // Publish — mod-a's handler must NOT fire, mod-b's must.
    kernel
        .event_bus()
        .publish(&TestStopEvent { _value: 1 })
        .unwrap();
    assert!(
        !handler_a.called.load(Ordering::SeqCst),
        "stopped module's handler must not fire"
    );
    assert!(
        handler_b.called.load(Ordering::SeqCst),
        "running module's handler must still fire"
    );
}

/// A stopped-then-restarted module must have its `on_load` handler
/// registration re-established, because the `Module` trait contract
/// says `on_load` is where event handlers are registered ("Use this
/// to validate configuration and register event handlers"), while
/// `on_start`/`on_stop` only manage runtime resources.
///
/// `stop_module` removes the module's handlers via
/// `unsubscribe_module` (Bug #6 fix). A restart via `start_module`
/// goes `Stopped → Started` and calls `on_start` but NOT `on_load`,
/// so the handlers `stop_module` removed are never re-registered —
/// the restarted module silently loses every event-bus subscription.
///
/// This test pins the root cause: `start_module` on a Stopped module
/// must re-run `on_load` (the only hook that registers handlers) to
/// restore the load-time invariant, mirroring how `start_all` after
/// `stop_all` already re-runs `load_all` → `on_load` (because
/// `stop_all` clears `self.loaded`).
#[test]
fn restart_via_start_module_re_runs_on_load_to_restore_handlers() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicI32;

    // A module that counts on_load invocations. on_load is the
    // documented hook for registering event handlers, so its call
    // count is a direct proxy for "were handlers re-registered?".
    #[derive(Debug)]
    struct CountingModule {
        id: &'static str,
        load_count: Arc<AtomicI32>,
    }
    impl Module for CountingModule {
        fn id(&self) -> &'static str {
            self.id
        }
        fn on_load(&mut self) -> ModuleResult {
            self.load_count.fetch_add(1, Ordering::SeqCst);
            // In a real module this is where subscribe_for_module
            // would register the module's event-bus handlers.
            Ok(())
        }
    }

    // ── Path A: stop_all → start_all re-runs on_load (works today).
    let mut kernel = Kernel::new();
    let load_count_a = Arc::new(AtomicI32::new(0));
    kernel
        .register(Box::new(CountingModule {
            id: "mod-a",
            load_count: load_count_a.clone(),
        }))
        .unwrap();
    kernel.start_all().unwrap();
    assert_eq!(
        load_count_a.load(Ordering::SeqCst),
        1,
        "initial start_all runs on_load once"
    );
    kernel.stop_all().unwrap();
    kernel.start_all().unwrap();
    assert_eq!(
        load_count_a.load(Ordering::SeqCst),
        2,
        "start_all after stop_all re-runs on_load (self.loaded was cleared)"
    );

    // ── Path B: stop_module → start_module does NOT re-run on_load (BUG).
    let mut kernel = Kernel::new();
    let load_count_b = Arc::new(AtomicI32::new(0));
    kernel
        .register(Box::new(CountingModule {
            id: "mod-b",
            load_count: load_count_b.clone(),
        }))
        .unwrap();
    kernel.start_all().unwrap();
    assert_eq!(
        load_count_b.load(Ordering::SeqCst),
        1,
        "initial start_all runs on_load once"
    );
    kernel.stop_module("mod-b").unwrap();
    // Restart the stopped module via start_module.
    kernel.start_module("mod-b").unwrap();
    // EXPECTED after fix: on_load re-runs on restart → count == 2.
    // CURRENT (buggy): on_load is NOT re-run → count stays 1, so any
    // handlers registered in on_load were permanently lost by the
    // stop_module cleanup (Bug #6) and never restored.
    assert_eq!(
        load_count_b.load(Ordering::SeqCst),
        2,
        "start_module on a Stopped module must re-run on_load to \
             restore the load-time invariant (handler registration), \
             mirroring start_all after stop_all"
    );
}

/// `load_all` must be idempotent for already-loaded modules: a retry
/// after a partial failure (module N's on_load fails) must NOT
/// re-run on_load for modules 1..N-1 that already loaded
/// successfully.
///
/// The `Module` contract documents on_load as the hook where event
/// handlers are registered ("Use this to validate configuration and
/// register event handlers"). Re-running on_load on an already-
/// loaded module double-registers its handlers (1 → 2 → 3 ... per
/// retry), so every published event fires the handler multiple
/// times. This is a real state-inconsistency bug (Axes 5 & 8):
/// partial-failure recovery silently corrupts the handler registry.
#[test]
fn load_all_retry_does_not_re_run_on_load_on_already_loaded_modules() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicI32;

    // A module that counts on_load invocations. Its on_load is the
    // proxy for "did the module re-register its event-bus handlers?".
    #[derive(Debug)]
    struct CountingModule {
        id: &'static str,
        load_count: Arc<AtomicI32>,
    }
    impl Module for CountingModule {
        fn id(&self) -> &'static str {
            self.id
        }
        fn on_load(&mut self) -> ModuleResult {
            self.load_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    let mut kernel = Kernel::new();
    // A single counting module. load_all is called twice in full
    // success (no failure). The second call must be a no-op for the
    // already-Loaded module — its on_load must NOT re-run, because
    // re-running would double-register any event-bus handlers it
    // registered the first time (Module contract: on_load is the
    // handler-registration hook).
    //
    // This is deterministic: no second module, no dependency-order
    // nondeterminism, no partial failure. It isolates the exact
    // invariant: load_all is idempotent for already-Loaded modules.
    let load_count = Arc::new(AtomicI32::new(0));
    kernel
        .register(Box::new(CountingModule {
            id: "counter",
            load_count: load_count.clone(),
        }))
        .unwrap();

    // First load_all: counter loads, on_load runs once.
    kernel.load_all().unwrap();
    assert_eq!(
        load_count.load(Ordering::SeqCst),
        1,
        "on_load runs once on first load_all"
    );

    // Second load_all: counter is already Loaded. BUG (before fix):
    // load_all re-runs on_load (count → 2), double-registering
    // handlers. After fix: on_load is skipped, count stays 1.
    kernel.load_all().unwrap();

    assert_eq!(
        load_count.load(Ordering::SeqCst),
        1,
        "load_all must NOT re-run on_load on already-Loaded modules \
             (prevents duplicate event-bus handler registration)"
    );
}

/// `stop_all` must NOT call `stop()` on services that were never
/// started. When `start_all` fails partway through the services
/// loop (service N fails start, services 1..N-1 already started),
/// the caller recovers with `stop_all`. But `stop_all` currently
/// calls `stop()` on EVERY service unconditionally — including
/// service N (never started) and services N+1..end (never started).
///
/// This violates the `Service` contract: `stop()` is only
/// meaningful after a successful `start()`. For services that
/// allocate `stop()` resources in `start()` (shutdown channels,
/// join handles), calling `stop()` without `start()` panics or
/// silently no-ops, masking the partial-start state (Axis 8:
/// silent failure / contract violation).
#[test]
fn stop_all_does_not_stop_services_that_were_never_started() {
    use std::sync::{Arc, Mutex};

    // A service that records whether start() and stop() were called,
    // and can be configured to fail start(). Shared via Arc<Mutex<>>,
    // because Service::start/stop take &mut self and the test must
    // retain access to the flags after registering the service.
    #[derive(Debug)]
    struct TrackingService {
        id: &'static str,
        fail_start: bool,
        start_called: bool,
        stop_called: bool,
    }
    impl TrackingService {
        fn new(id: &'static str) -> Self {
            Self {
                id,
                fail_start: false,
                start_called: false,
                stop_called: false,
            }
        }
        fn with_fail_start(mut self) -> Self {
            self.fail_start = true;
            self
        }
    }
    impl Service for TrackingService {
        fn id(&self) -> &'static str {
            self.id
        }
        fn start(&mut self) -> ModuleResult {
            if self.fail_start {
                return Err(anyhow::anyhow!("service start failed"));
            }
            self.start_called = true;
            Ok(())
        }
        fn stop(&mut self) -> ModuleResult {
            self.stop_called = true;
            Ok(())
        }
    }

    let mut kernel = Kernel::new();
    kernel.register(Box::new(TestModule::new("mod"))).unwrap();

    // svc-a starts successfully; svc-b fails to start; svc-c is
    // after the failure so never reached. Wrapped in Arc<Mutex<>> so
    // the test can inspect flags after start_all/stop_all run.
    let svc_a = Arc::new(Mutex::new(TrackingService::new("svc-a")));
    let svc_b = Arc::new(Mutex::new(TrackingService::new("svc-b").with_fail_start()));
    let svc_c = Arc::new(Mutex::new(TrackingService::new("svc-c")));

    // Thin Service wrapper that forwards to the shared Arc<Mutex<...>>.
    #[derive(Debug)]
    struct ServiceRef(Arc<Mutex<TrackingService>>);
    impl Service for ServiceRef {
        fn id(&self) -> &'static str {
            self.0.lock().unwrap().id
        }
        fn start(&mut self) -> ModuleResult {
            self.0.lock().unwrap().start()
        }
        fn stop(&mut self) -> ModuleResult {
            self.0.lock().unwrap().stop()
        }
    }

    kernel.register_service(Box::new(ServiceRef(svc_a.clone())));
    kernel.register_service(Box::new(ServiceRef(svc_b.clone())));
    kernel.register_service(Box::new(ServiceRef(svc_c.clone())));

    // start_all fails at svc-b. svc-a started; svc-b and svc-c did not.
    let result = kernel.start_all();
    assert!(result.is_err(), "start_all should fail at svc-b");
    assert!(
        svc_a.lock().unwrap().start_called,
        "svc-a should have been started"
    );
    assert!(
        !svc_b.lock().unwrap().start_called,
        "svc-b should NOT have completed start (it failed)"
    );
    assert!(
        !svc_c.lock().unwrap().start_called,
        "svc-c should NOT have been started (after the failure)"
    );

    // Caller recovers with stop_all. BUG: stop_all calls stop() on
    // svc-b and svc-c, which were never started.
    kernel.stop_all().unwrap();

    // svc-a (started) should be stopped.
    assert!(
        svc_a.lock().unwrap().stop_called,
        "svc-a (started) should be stopped"
    );
    // svc-b and svc-c were NEVER started — stop() must NOT be called
    // on them (Service contract: stop() only after a successful start()).
    assert!(
        !svc_b.lock().unwrap().stop_called,
        "stop_all must NOT call stop() on svc-b, which was never started"
    );
    assert!(
        !svc_c.lock().unwrap().stop_called,
        "stop_all must NOT call stop() on svc-c, which was never started"
    );
}

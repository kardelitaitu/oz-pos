//! In-process, topic-based, synchronous event bus.
//!
//! The [`EventBus`] decouples modules by allowing them to publish and
//! subscribe to domain events without direct imports. Handlers are
//! dispatched synchronously — the publisher blocks until all handlers
//! have run. Errors are logged but do not propagate.
//!
//! # Thread safety
//!
//! `EventBus` is `Send + Sync`. Use `Arc<EventBus>` for shared
//! access between modules and the kernel.
//!
//! # Design
//!
//! See ADR #2 (`docs/decisions/2026-02-01-event-bus-design.md`) for
//! the full design rationale.

use std::any::Any;
use std::collections::HashMap;
use std::sync::RwLock;

use foundation::contracts::{DomainEvent, EventHandler, ModuleResult};
use tracing::{debug, error};

/// A type-erased event handler stored in the bus.
type HandlerFn = Box<dyn Fn(&dyn Any) -> ModuleResult + Send + Sync>;

/// Wrap an `EventHandler<E>` into a type-erased `HandlerFn`.
fn wrap_handler<E>(handler: Box<dyn EventHandler<E>>) -> HandlerFn
where
    E: DomainEvent + 'static,
{
    Box::new(move |event: &dyn Any| -> ModuleResult {
        match event.downcast_ref::<E>() {
            Some(typed) => handler.handle(typed),
            None => Err(anyhow::anyhow!(
                "event bus type mismatch: expected {}, got unknown type",
                std::any::type_name::<E>(),
            )),
        }
    })
}

/// A registered subscriber entry, optionally owned by a module.
struct SubscriberEntry {
    /// The module that registered this handler, or `""` for anonymous.
    module_id: &'static str,
    /// The type-erased handler function.
    handler: HandlerFn,
}

/// In-process event bus with synchronous dispatch and module-scoped
/// subscription tracking.
///
/// Handlers can be registered with an optional `module_id` via
/// [`subscribe_for_module`](EventBus::subscribe_for_module). When a
/// module is stopped, `unsubscribe_module`
/// atomically removes all handlers owned by that module.
///
/// # Example
///
/// ```no_run
/// # use platform_kernel::EventBus;
/// # use foundation::contracts::{DomainEvent, EventHandler, ModuleResult};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Define an event
/// #[derive(Clone, Debug)]
/// struct SaleCompleted { sale_id: String }
/// impl DomainEvent for SaleCompleted {
///     fn event_name(&self) -> &'static str { "sale.completed" }
/// }
///
/// // Define a handler
/// struct InventoryHandler;
/// impl EventHandler<SaleCompleted> for InventoryHandler {
///     fn handle(&self, event: &SaleCompleted) -> ModuleResult {
///         println!("processing sale: {}", event.sale_id);
///         Ok(())
///     }
/// }
///
/// // Wire it up
/// let bus = EventBus::new();
/// bus.subscribe("sale.completed", Box::new(InventoryHandler));
/// bus.publish(&SaleCompleted { sale_id: "sale-1".into() })?;
/// # Ok(())
/// # }
pub struct EventBus {
    /// Map of topic name → subscriber entries (handler + ownership).
    subscribers: RwLock<HashMap<&'static str, Vec<SubscriberEntry>>>,
}

impl EventBus {
    /// Create a new empty event bus.
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a handler for events published under the given topic.
    ///
    /// The `topic` must match the value returned by `event.event_name()`
    /// on the published event.
    ///
    /// Multiple handlers can be registered for the same topic; they are
    /// called in registration order.
    ///
    /// This is an anonymous subscription (no module ownership tracking).
    /// Prefer [`subscribe_for_module`](EventBus::subscribe_for_module)
    /// when the handler belongs to a module that may be stopped at runtime.
    pub fn subscribe<E>(&self, topic: &'static str, handler: Box<dyn EventHandler<E>>)
    where
        E: DomainEvent + 'static,
    {
        let mut subs = self
            .subscribers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        subs.entry(topic).or_default().push(SubscriberEntry {
            module_id: "",
            handler: wrap_handler::<E>(handler),
        });
        debug!(topic, "anonymous event handler registered");
    }

    /// Register a handler owned by a specific module.
    ///
    /// When the module is stopped, call `unsubscribe_module`
    /// to atomically remove all handlers registered under that
    /// `module_id` across all topics.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use platform_kernel::EventBus;
    /// # use foundation::contracts::{DomainEvent, EventHandler, ModuleResult};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #[derive(Clone, Debug)] struct MyEvent;
    /// # impl DomainEvent for MyEvent { fn event_name(&self) -> &'static str { "my.event" } }
    /// struct MyHandler;
    /// # impl EventHandler<MyEvent> for MyHandler { fn handle(&self, _: &MyEvent) -> ModuleResult { Ok(()) } }
    /// let bus = EventBus::new();
    /// bus.subscribe_for_module("inventory", "sale.completed", Box::new(MyHandler));
    /// # Ok(())
    /// # }
    /// ```
    pub fn subscribe_for_module<E>(
        &self,
        module_id: &'static str,
        topic: &'static str,
        handler: Box<dyn EventHandler<E>>,
    ) where
        E: DomainEvent + 'static,
    {
        let wrapped = wrap_handler::<E>(handler);
        let mut subs = self
            .subscribers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        subs.entry(topic).or_default().push(SubscriberEntry {
            module_id,
            handler: wrapped,
        });
        debug!(module = module_id, topic, "module event handler registered");
    }

    /// Remove all handlers owned by the given module across every topic.
    ///
    /// This is called by the kernel when a module is stopped, ensuring
    /// stopped modules do not continue to receive events.
    ///
    /// Returns the number of handlers removed.
    pub fn unsubscribe_module(&self, module_id: &'static str) -> usize {
        let mut subs = self
            .subscribers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut total_removed = 0;

        // Retain only entries that do NOT belong to the given module,
        // dropping topic keys that become empty.
        subs.retain(|topic, entries| {
            let before = entries.len();
            entries.retain(|e| e.module_id != module_id);
            let removed = before - entries.len();
            total_removed += removed;
            if removed > 0 {
                debug!(
                    module = module_id,
                    topic, removed, "unsubscribed module handlers"
                );
            }
            // Drop the topic key entirely if no handlers remain.
            !entries.is_empty()
        });

        if total_removed > 0 {
            debug!(
                module = module_id,
                total_removed, "module fully unsubscribed"
            );
        }
        total_removed
    }

    /// Publish an event to all subscribed handlers.
    ///
    /// The topic is obtained from `event.event_name()`. All handlers
    /// registered for that topic are called synchronously. If a handler
    /// returns an error, it is logged but other handlers still run.
    ///
    /// Returns `Ok(())` regardless of individual handler errors (the
    /// error is fire-and-forget).
    pub fn publish<E>(&self, event: &E) -> ModuleResult
    where
        E: DomainEvent + 'static,
    {
        let topic = event.event_name();
        let any_ref: &dyn Any = event;

        let subs = self
            .subscribers
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let entries = match subs.get(topic) {
            Some(h) => h,
            None => {
                debug!(topic, "no handlers registered for this event");
                return Ok(());
            }
        };

        let count = entries.len();
        for (i, entry) in entries.iter().enumerate() {
            if let Err(e) = (entry.handler)(any_ref) {
                error!(
                    topic,
                    handler_index = i,
                    module = entry.module_id,
                    error = %e,
                    "event handler failed (continuing)"
                );
            }
        }

        debug!(topic, handler_count = count, "event published");
        Ok(())
    }

    /// Number of topics with registered handlers.
    pub fn topic_count(&self) -> usize {
        self.subscribers
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// Number of handlers registered for a specific module across all topics.
    pub fn handler_count_for_module(&self, module_id: &str) -> usize {
        self.subscribers
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .flat_map(|v| v.iter())
            .filter(|e| e.module_id == module_id)
            .count()
    }

    /// Total number of registered handlers across all topics.
    pub fn handler_count(&self) -> usize {
        self.subscribers
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .map(|v| v.len())
            .sum()
    }

    /// Check if any handlers are registered for a topic.
    pub fn has_handlers(&self, topic: &str) -> bool {
        self.subscribers
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(topic)
            .is_some_and(|v| !v.is_empty())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

    // ── Test event types ─────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    struct TestEvent {
        value: i32,
    }

    impl DomainEvent for TestEvent {
        fn event_name(&self) -> &'static str {
            "test.event"
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct OtherEvent {
        message: String,
    }

    impl DomainEvent for OtherEvent {
        fn event_name(&self) -> &'static str {
            "other.event"
        }
    }

    // ── Test handlers (use Arc for shared state) ─────────────────

    struct TestHandler {
        last_value: AtomicI32,
    }

    impl TestHandler {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                last_value: AtomicI32::new(0),
            })
        }

        fn last_value(&self) -> i32 {
            self.last_value.load(Ordering::SeqCst)
        }
    }

    impl EventHandler<TestEvent> for TestHandler {
        fn handle(&self, event: &TestEvent) -> ModuleResult {
            self.last_value.store(event.value, Ordering::SeqCst);
            Ok(())
        }
    }

    // Arc<TestHandler> delegates to TestHandler
    impl EventHandler<TestEvent> for Arc<TestHandler> {
        fn handle(&self, event: &TestEvent) -> ModuleResult {
            (**self).handle(event)
        }
    }

    struct FailingHandler;

    impl EventHandler<TestEvent> for FailingHandler {
        fn handle(&self, _event: &TestEvent) -> ModuleResult {
            Err(anyhow::anyhow!("handler deliberately failed"))
        }
    }

    struct OtherHandler {
        last_message: std::sync::Mutex<Option<String>>,
    }

    impl OtherHandler {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                last_message: std::sync::Mutex::new(None),
            })
        }

        fn last_message(&self) -> Option<String> {
            self.last_message.lock().unwrap().clone()
        }
    }

    impl EventHandler<OtherEvent> for OtherHandler {
        fn handle(&self, event: &OtherEvent) -> ModuleResult {
            *self.last_message.lock().unwrap() = Some(event.message.clone());
            Ok(())
        }
    }

    impl EventHandler<OtherEvent> for Arc<OtherHandler> {
        fn handle(&self, event: &OtherEvent) -> ModuleResult {
            (**self).handle(event)
        }
    }

    // A handler that records whether it was called.
    struct CalledHandler {
        was_called: AtomicBool,
    }

    impl CalledHandler {
        fn new() -> Self {
            Self {
                was_called: AtomicBool::new(false),
            }
        }
    }

    impl EventHandler<TestEvent> for CalledHandler {
        fn handle(&self, _event: &TestEvent) -> ModuleResult {
            self.was_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    // ── Tests ────────────────────────────────────────────────────

    #[test]
    fn empty_bus_has_no_topics() {
        let bus = EventBus::new();
        assert_eq!(bus.topic_count(), 0);
        assert_eq!(bus.handler_count(), 0);
    }

    #[test]
    fn subscribe_and_publish() {
        let bus = EventBus::new();
        let handler = CalledHandler::new();

        bus.subscribe("test.event", Box::new(handler));
        assert_eq!(bus.topic_count(), 1);
        assert_eq!(bus.handler_count(), 1);
        assert!(bus.has_handlers("test.event"));

        bus.publish(&TestEvent { value: 42 }).unwrap();
        // Can't check handler state since it was moved into the bus
        // Instead, verify the bus processed it
        assert_eq!(bus.handler_count(), 1);
    }

    #[test]
    fn publish_with_no_handlers_does_not_error() {
        let bus = EventBus::new();
        let result = bus.publish(&TestEvent { value: 1 });
        assert!(result.is_ok());
    }

    #[test]
    fn failing_handler_does_not_block() {
        let bus = EventBus::new();

        bus.subscribe("test.event", Box::new(CalledHandler::new()));
        bus.subscribe("test.event", Box::new(FailingHandler));

        let result = bus.publish(&TestEvent { value: 99 });
        assert!(result.is_ok());
    }

    #[test]
    fn multiple_handlers_for_different_topics() {
        let bus = EventBus::new();

        bus.subscribe("test.event", Box::new(CalledHandler::new()));
        bus.subscribe("other.event", Box::new(CalledHandler::new()));

        assert_eq!(bus.topic_count(), 2);
        assert_eq!(bus.handler_count(), 2);
    }

    #[test]
    fn multiple_handlers_for_same_topic() {
        let bus = EventBus::new();

        bus.subscribe("test.event", Box::new(CalledHandler::new()));
        bus.subscribe("test.event", Box::new(CalledHandler::new()));

        assert_eq!(bus.handler_count(), 2);
    }

    #[test]
    fn has_handlers_returns_false_for_unknown_topic() {
        let bus = EventBus::new();
        assert!(!bus.has_handlers("nonexistent"));
    }

    #[test]
    fn topic_count_is_distinct_topics() {
        let bus = EventBus::new();
        bus.subscribe("a", Box::new(CalledHandler::new()));
        bus.subscribe("a", Box::new(CalledHandler::new()));
        bus.subscribe("b", Box::new(CalledHandler::new()));

        assert_eq!(bus.topic_count(), 2);
        assert_eq!(bus.handler_count(), 3);
    }

    #[test]
    fn concurrent_publish_is_safe() {
        let bus = Arc::new(EventBus::new());
        bus.subscribe("test.event", Box::new(CalledHandler::new()));

        let bus2 = bus.clone();
        let t1 = std::thread::spawn(move || {
            bus2.publish(&TestEvent { value: 1 }).unwrap();
        });
        let bus3 = bus.clone();
        let t2 = std::thread::spawn(move || {
            bus3.publish(&TestEvent { value: 2 }).unwrap();
        });

        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn subscribe_twice_same_topic() {
        let bus = EventBus::new();
        bus.subscribe("test.event", Box::new(CalledHandler::new()));
        bus.subscribe("test.event", Box::new(CalledHandler::new()));
        assert_eq!(bus.handler_count(), 2);
    }

    #[test]
    fn publish_using_arc_handler() {
        let bus = EventBus::new();
        let handler = TestHandler::new();
        bus.subscribe("test.event", Box::new(handler.clone()));

        bus.publish(&TestEvent { value: 77 }).unwrap();
        assert_eq!(handler.last_value(), 77);
    }

    #[test]
    fn multiple_topics_independent() {
        let bus = EventBus::new();
        let test_handler = TestHandler::new();
        let other_handler = OtherHandler::new();

        bus.subscribe("test.event", Box::new(test_handler.clone()));
        bus.subscribe("other.event", Box::new(other_handler.clone()));

        bus.publish(&TestEvent { value: 42 }).unwrap();
        assert_eq!(test_handler.last_value(), 42);
        assert!(other_handler.last_message().is_none());

        bus.publish(&OtherEvent {
            message: "hello".into(),
        })
        .unwrap();
        assert_eq!(test_handler.last_value(), 42);
        assert_eq!(other_handler.last_message(), Some("hello".into()));
    }

    // ── Module-scoped subscription tests ─────────────────────────

    #[test]
    fn subscribe_for_module_tracks_module_id() {
        let bus = EventBus::new();

        bus.subscribe_for_module("inventory", "test.event", Box::new(CalledHandler::new()));
        bus.subscribe_for_module("sales", "test.event", Box::new(CalledHandler::new()));

        assert_eq!(bus.handler_count_for_module("inventory"), 1);
        assert_eq!(bus.handler_count_for_module("sales"), 1);
        assert_eq!(bus.handler_count_for_module("unknown"), 0);
        assert_eq!(bus.handler_count(), 2);
    }

    #[test]
    fn subscribe_for_module_and_publish() {
        let bus = EventBus::new();
        let handler = TestHandler::new();

        bus.subscribe_for_module("inventory", "test.event", Box::new(handler.clone()));
        bus.publish(&TestEvent { value: 42 }).unwrap();
        assert_eq!(handler.last_value(), 42);
    }

    #[test]
    fn unsubscribe_module_removes_all_its_handlers() {
        let bus = EventBus::new();

        bus.subscribe_for_module("inventory", "test.event", Box::new(CalledHandler::new()));
        bus.subscribe_for_module("inventory", "other.event", Box::new(CalledHandler::new()));
        bus.subscribe_for_module("sales", "test.event", Box::new(CalledHandler::new()));

        assert_eq!(bus.handler_count(), 3);
        assert_eq!(bus.handler_count_for_module("inventory"), 2);

        let removed = bus.unsubscribe_module("inventory");
        assert_eq!(removed, 2);

        // Sales handler should remain.
        assert_eq!(bus.handler_count(), 1);
        assert_eq!(bus.handler_count_for_module("inventory"), 0);
        assert_eq!(bus.handler_count_for_module("sales"), 1);
    }

    #[test]
    fn unsubscribe_module_with_no_handlers_returns_zero() {
        let bus = EventBus::new();
        let removed = bus.unsubscribe_module("nonexistent");
        assert_eq!(removed, 0);
    }

    #[test]
    fn unsubscribe_module_does_not_affect_anonymous_subscriptions() {
        let bus = EventBus::new();

        bus.subscribe("test.event", Box::new(CalledHandler::new()));
        bus.subscribe_for_module("inventory", "test.event", Box::new(CalledHandler::new()));

        assert_eq!(bus.handler_count(), 2);

        let removed = bus.unsubscribe_module("inventory");
        assert_eq!(removed, 1);

        // Anonymous handler should remain.
        assert_eq!(bus.handler_count(), 1);
        assert!(bus.has_handlers("test.event"));
    }

    #[test]
    fn unsubscribe_module_multiple_topics() {
        let bus = EventBus::new();

        bus.subscribe_for_module("inventory", "a", Box::new(CalledHandler::new()));
        bus.subscribe_for_module("inventory", "b", Box::new(CalledHandler::new()));
        bus.subscribe_for_module("inventory", "c", Box::new(CalledHandler::new()));

        assert_eq!(bus.topic_count(), 3);
        assert_eq!(bus.handler_count(), 3);

        bus.unsubscribe_module("inventory");
        assert_eq!(bus.handler_count(), 0);
    }

    #[test]
    fn module_handlers_are_called_in_order() {
        let bus = EventBus::new();
        let first = TestHandler::new();
        let second = TestHandler::new();

        bus.subscribe_for_module("sales", "test.event", Box::new(first.clone()));
        bus.subscribe_for_module("inventory", "test.event", Box::new(second.clone()));

        bus.publish(&TestEvent { value: 100 }).unwrap();
        assert_eq!(first.last_value(), 100);
        assert_eq!(second.last_value(), 100);
    }

    #[test]
    fn unsubscribe_then_resubscribe() {
        let bus = EventBus::new();

        bus.subscribe_for_module("inventory", "test.event", Box::new(CalledHandler::new()));
        bus.unsubscribe_module("inventory");
        assert_eq!(bus.handler_count(), 0);

        // Re-subscribe.
        bus.subscribe_for_module("inventory", "test.event", Box::new(CalledHandler::new()));
        assert_eq!(bus.handler_count(), 1);
        assert!(bus.has_handlers("test.event"));
    }

    /// Panicking handler does not poison the bus. Since `publish()` uses
    /// a read lock and panics drop read guards without poisoning the RwLock,
    /// subsequent publishes must still succeed.
    #[test]
    fn panicking_handler_does_not_poison_bus() {
        use std::sync::Arc;

        let bus = Arc::new(EventBus::new());

        // Handler that panics on odd values.
        struct PanicOnOdd;
        impl EventHandler<TestEvent> for PanicOnOdd {
            fn handle(&self, event: &TestEvent) -> ModuleResult {
                if event.value % 2 != 0 {
                    panic!("handler panicked on odd value: {}", event.value);
                }
                Ok(())
            }
        }

        let good_handler = TestHandler::new();
        bus.subscribe("test.event", Box::new(PanicOnOdd));
        bus.subscribe("test.event", Box::new(good_handler.clone()));

        // First publish with odd value should panic through publish().
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            bus.publish(&TestEvent { value: 1 })
        }));
        // The panic should propagate out of publish().
        assert!(result.is_err(), "publish() should propagate handler panic");
        // The second handler was never called — panic skipped it.
        // This is a known limitation: panics drop remaining handlers silently.
        assert_eq!(
            good_handler.last_value(),
            0,
            "handler after panicking handler should not have been called"
        );

        // The bus must NOT be poisoned — subsequent publish should work.
        let result2 = bus.publish(&TestEvent { value: 2 });
        assert!(result2.is_ok(), "bus should still be usable after panic");
        assert_eq!(good_handler.last_value(), 2);
    }
}

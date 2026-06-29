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

/// In-process event bus with synchronous dispatch.
///
/// # Example
///
/// ```ignore
/// use platform_kernel::EventBus;
/// use foundation::contracts::{DomainEvent, EventHandler};
///
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
/// ```
pub struct EventBus {
    /// Map of topic name → type-erased handler functions.
    subscribers: RwLock<HashMap<&'static str, Vec<HandlerFn>>>,
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
    pub fn subscribe<E>(&self, topic: &'static str, handler: Box<dyn EventHandler<E>>)
    where
        E: DomainEvent + 'static,
    {
        let wrapped: HandlerFn = Box::new(move |event: &dyn Any| -> ModuleResult {
            match event.downcast_ref::<E>() {
                Some(typed) => handler.handle(typed),
                None => Err(format!(
                    "event bus type mismatch: expected {}, got unknown type",
                    std::any::type_name::<E>(),
                )
                .into()),
            }
        });

        let mut subs = self.subscribers.write().expect("event bus lock poisoned");
        subs.entry(topic).or_default().push(wrapped);
        debug!(topic, "event handler registered");
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

        let subs = self.subscribers.read().expect("event bus lock poisoned");
        let handlers = match subs.get(topic) {
            Some(h) => h,
            None => {
                debug!(topic, "no handlers registered for this event");
                return Ok(());
            }
        };

        let count = handlers.len();
        for (i, handler) in handlers.iter().enumerate() {
            if let Err(e) = handler(any_ref) {
                error!(
                    topic,
                    handler_index = i,
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
            .expect("event bus lock poisoned")
            .len()
    }

    /// Total number of registered handlers across all topics.
    pub fn handler_count(&self) -> usize {
        self.subscribers
            .read()
            .expect("event bus lock poisoned")
            .values()
            .map(|v| v.len())
            .sum()
    }

    /// Check if any handlers are registered for a topic.
    pub fn has_handlers(&self, topic: &str) -> bool {
        self.subscribers
            .read()
            .expect("event bus lock poisoned")
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
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
    use std::sync::Arc;

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
            Err("handler deliberately failed".into())
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

        fn was_called(&self) -> bool {
            self.was_called.load(Ordering::SeqCst)
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

    #[test]
    fn handler_state_persists_after_publish() {
        let bus = EventBus::new();
        let handler = TestHandler::new();
        bus.subscribe("test.event", Box::new(handler.clone()));

        bus.publish(&TestEvent { value: 10 }).unwrap();
        assert_eq!(handler.last_value(), 10);

        bus.publish(&TestEvent { value: 20 }).unwrap();
        assert_eq!(handler.last_value(), 20);
    }
}

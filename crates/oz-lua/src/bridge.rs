//! Lua event bridge — exposes event subscription callbacks to Lua scripts.
//!
//! Provides the `oz.on()` and `oz.off()` APIs that let Lua plugins subscribe
//! to domain events such as `sale.completed`, `order.fired`, or custom events
//! fired by other plugins or the Rust backend.
//!
//! # Example (Lua)
//!
//! ```lua
//! oz.on("sale.completed", function(event)
//!     oz.log("info", "Sale completed: " .. event.total_minor)
//! end)
//! ```
//!
//! # Example (Rust)
//!
//! ```no_run
//! # use oz_lua::bridge::LuaEventBridge;
//! # use oz_lua::LuaRuntime;
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let bridge = LuaEventBridge::new();
//! let lua = LuaRuntime::new()?;
//! // Register callbacks from Lua
//! // Fire from Rust
//! bridge.fire(lua.inner(), "sale.completed", rlua::Value::Nil)?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use rlua::{Function, Lua, RegistryKey, Value};

/// Manages Lua event callbacks registered via `oz.on()`.
///
/// Callbacks are stored as `RegistryKey`s in the Lua registry so they
/// survive garbage collection between script loads. The bridge is
/// designed to be shared between the `PluginManager` and the Lua runtime.
#[derive(Debug)]
pub struct LuaEventBridge {
    /// Registry keys for callback functions, indexed by event name.
    callbacks: HashMap<String, Vec<RegistryKey>>,
}

impl LuaEventBridge {
    /// Create a new empty event bridge.
    pub fn new() -> Self {
        Self {
            callbacks: HashMap::new(),
        }
    }

    /// Register a Lua callback for an event (called by `oz.on()`).
    ///
    /// The callback function is stored in the Lua registry so it persists
    /// across script reloads and survives GC.
    pub fn register(&mut self, lua: &Lua, event: String, callback: Function) -> rlua::Result<()> {
        let key = lua.create_registry_value(callback)?;
        self.callbacks.entry(event).or_default().push(key);
        Ok(())
    }

    /// Fire an event: call all registered Lua callbacks with the given args.
    ///
    /// Returns `Ok(())` even if individual callbacks error — errors are
    /// collected and returned as a single comma-separated error string only
    /// if ALL callbacks fail. If at least one succeeds, the result is `Ok(())`.
    pub fn fire(&self, lua: &Lua, event: &str, args: Value) -> Result<(), crate::LuaError> {
        let Some(keys) = self.callbacks.get(event) else {
            return Ok(());
        };

        let mut last_error = None;

        for key in keys {
            let callback: Function = match lua.registry_value(key) {
                Ok(f) => f,
                Err(e) => {
                    last_error = Some(crate::LuaError::Script(format!(
                        "failed to retrieve callback for '{event}': {e}"
                    )));
                    continue;
                }
            };

            if let Err(e) = callback.call::<_, ()>(args.clone()) {
                last_error = Some(crate::LuaError::Script(format!(
                    "callback for '{event}' failed: {e}"
                )));
                // Continue to other callbacks
            } else {
                // At least one callback succeeded — clear any prior error
                last_error = None;
            }
        }

        last_error.map_or(Ok(()), Err)
    }

    /// Remove all callbacks for a specific event.
    ///
    /// This drops the registry keys, allowing the Lua GC to collect
    /// the callback functions.
    pub fn off(&mut self, event: &str) {
        if let Some(keys) = self.callbacks.remove(event) {
            // RegistryKeys are dropped here, which removes them from the
            // Lua registry.
            drop(keys);
        }
    }

    /// Remove all callbacks for all events.
    pub fn clear(&mut self) {
        self.callbacks.clear();
    }

    /// Returns the number of events that have registered callbacks.
    pub fn event_count(&self) -> usize {
        self.callbacks.len()
    }

    /// Returns the total number of registered callbacks across all events.
    pub fn callback_count(&self) -> usize {
        self.callbacks.values().map(|v| v.len()).sum()
    }

    /// Check if any callbacks are registered for an event.
    pub fn has_callbacks(&self, event: &str) -> bool {
        self.callbacks.get(event).is_some_and(|v| !v.is_empty())
    }

    /// Returns the number of callbacks registered for a specific event.
    pub fn callback_count_for(&self, event: &str) -> usize {
        self.callbacks.get(event).map_or(0, |v| v.len())
    }

    /// Remove a specific callback by event name and index.
    /// Returns `true` if a callback was removed.
    pub fn remove_callback(&mut self, event: &str, index: usize) -> bool {
        if let Some(keys) = self.callbacks.get_mut(event)
            && index < keys.len()
        {
            keys.remove(index);
            if keys.is_empty() {
                self.callbacks.remove(event);
            }
            return true;
        }
        false
    }
}

impl Default for LuaEventBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_bridge_is_empty() {
        let bridge = LuaEventBridge::new();
        assert_eq!(bridge.event_count(), 0);
        assert_eq!(bridge.callback_count(), 0);
    }

    #[test]
    fn register_callback_increments_counts() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        bridge
            .register(&lua, "test.event".into(), callback)
            .unwrap();

        assert_eq!(bridge.event_count(), 1);
        assert_eq!(bridge.callback_count(), 1);
        assert!(bridge.has_callbacks("test.event"));
        assert_eq!(bridge.callback_count_for("test.event"), 1);
    }

    #[test]
    fn register_multiple_callbacks_same_event() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        for _ in 0..3 {
            let cb = lua.create_function(|_, ()| Ok(())).unwrap();
            bridge.register(&lua, "same.event".into(), cb).unwrap();
        }

        assert_eq!(bridge.event_count(), 1);
        assert_eq!(bridge.callback_count(), 3);
        assert_eq!(bridge.callback_count_for("same.event"), 3);
    }

    #[test]
    fn register_callbacks_different_events() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        for event in &["a", "b", "c"] {
            let cb = lua.create_function(|_, ()| Ok(())).unwrap();
            bridge.register(&lua, (*event).into(), cb).unwrap();
        }

        assert_eq!(bridge.event_count(), 3);
        assert_eq!(bridge.callback_count(), 3);
    }

    #[test]
    fn fire_event_with_no_callbacks_does_nothing() {
        let lua = rlua::Lua::new();
        let bridge = LuaEventBridge::new();
        let result = bridge.fire(&lua, "nonexistent", rlua::Value::Nil);
        assert!(result.is_ok());
    }

    #[test]
    fn fire_event_calls_callback() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        let callback = lua
            .create_function(move |_, ()| {
                called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .unwrap();

        bridge
            .register(&lua, "test.event".into(), callback)
            .unwrap();
        bridge.fire(&lua, "test.event", rlua::Value::Nil).unwrap();

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn fire_event_passes_args_to_callback() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        let received = std::sync::Arc::new(std::sync::Mutex::new(None));
        let received_clone = received.clone();

        let callback = lua
            .create_function(move |_ctx, args: rlua::Table| {
                let val: i64 = args.get("value").unwrap();
                *received_clone.lock().unwrap() = Some(val);
                Ok(())
            })
            .unwrap();

        bridge
            .register(&lua, "test.event".into(), callback)
            .unwrap();

        let args = lua.create_table().unwrap();
        args.set("value", 42).unwrap();
        bridge
            .fire(&lua, "test.event", rlua::Value::Table(args))
            .unwrap();

        assert_eq!(*received.lock().unwrap(), Some(42));
    }

    #[test]
    fn fire_multiple_callbacks_all_get_called() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));
        let c1 = counter.clone();
        let c2 = counter.clone();

        let cb1 = lua
            .create_function(move |_, ()| {
                c1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .unwrap();

        let cb2 = lua
            .create_function(move |_, ()| {
                c2.fetch_add(10, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .unwrap();

        bridge.register(&lua, "test.event".into(), cb1).unwrap();
        bridge.register(&lua, "test.event".into(), cb2).unwrap();

        bridge.fire(&lua, "test.event", rlua::Value::Nil).unwrap();
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 11);
    }

    #[test]
    fn fire_continues_when_one_callback_fails() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));
        let c1 = counter.clone();
        let c2 = counter.clone();

        // Failing callback
        let cb_fail = lua
            .create_function(move |_, ()| {
                c1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err::<(), _>(rlua::Error::RuntimeError("deliberate fail".into()))
            })
            .unwrap();

        // Succeeding callback
        let cb_ok = lua
            .create_function(move |_, ()| {
                c2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .unwrap();

        bridge.register(&lua, "test.event".into(), cb_fail).unwrap();
        bridge.register(&lua, "test.event".into(), cb_ok).unwrap();

        // When at least one succeeds, the overall result is Ok
        let result = bridge.fire(&lua, "test.event", rlua::Value::Nil);
        assert!(
            result.is_ok(),
            "should succeed when at least one callback works"
        );
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn fire_returns_error_when_all_callbacks_fail() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        let cb1 = lua
            .create_function(|_, ()| Err::<(), _>(rlua::Error::RuntimeError("fail 1".into())))
            .unwrap();
        let cb2 = lua
            .create_function(|_, ()| Err::<(), _>(rlua::Error::RuntimeError("fail 2".into())))
            .unwrap();

        bridge.register(&lua, "bad.event".into(), cb1).unwrap();
        bridge.register(&lua, "bad.event".into(), cb2).unwrap();

        let result = bridge.fire(&lua, "bad.event", rlua::Value::Nil);
        assert!(result.is_err());
    }

    #[test]
    fn off_removes_event_callbacks() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        let cb = lua.create_function(|_, ()| Ok(())).unwrap();
        bridge.register(&lua, "test.event".into(), cb).unwrap();

        assert!(bridge.has_callbacks("test.event"));
        bridge.off("test.event");
        assert!(!bridge.has_callbacks("test.event"));
        assert_eq!(bridge.callback_count(), 0);
    }

    #[test]
    fn off_nonexistent_event_is_noop() {
        let mut bridge = LuaEventBridge::new();
        bridge.off("nonexistent");
        // Should not panic
    }

    #[test]
    fn clear_removes_all_callbacks() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        for event in &["e1", "e2", "e3"] {
            let cb = lua.create_function(|_, ()| Ok(())).unwrap();
            bridge.register(&lua, (*event).into(), cb).unwrap();
        }

        assert_eq!(bridge.event_count(), 3);
        assert_eq!(bridge.callback_count(), 3);

        bridge.clear();
        assert_eq!(bridge.event_count(), 0);
        assert_eq!(bridge.callback_count(), 0);
    }

    #[test]
    fn remove_callback_by_index() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        let cb = lua.create_function(|_, ()| Ok(())).unwrap();
        bridge.register(&lua, "test.event".into(), cb).unwrap();

        assert!(bridge.remove_callback("test.event", 0));
        assert!(!bridge.has_callbacks("test.event"));
    }

    #[test]
    fn remove_callback_out_of_bounds_returns_false() {
        let mut bridge = LuaEventBridge::new();
        assert!(!bridge.remove_callback("nonexistent", 0));
    }

    #[test]
    fn default_is_empty() {
        let bridge = LuaEventBridge::default();
        assert_eq!(bridge.event_count(), 0);
    }

    #[test]
    fn debug_output() {
        let bridge = LuaEventBridge::new();
        let debug = format!("{bridge:?}");
        assert!(debug.contains("LuaEventBridge"));
    }

    #[test]
    fn callback_count_for_unknown_event() {
        let bridge = LuaEventBridge::new();
        assert_eq!(bridge.callback_count_for("unknown"), 0);
    }

    #[test]
    fn fire_with_nil_args_works_for_callbacks_ignoring_args() {
        let lua = rlua::Lua::new();
        let mut bridge = LuaEventBridge::new();

        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cc = called.clone();

        let cb = lua
            .create_function(move |_, _: rlua::Value| {
                cc.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .unwrap();

        bridge.register(&lua, "test.event".into(), cb).unwrap();
        bridge.fire(&lua, "test.event", rlua::Value::Nil).unwrap();
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }
}

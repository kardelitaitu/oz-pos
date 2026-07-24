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
//! bridge.fire(lua.inner(), "sale.completed", mlua::Value::Nil)?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use mlua::{Function, Lua, RegistryKey, Value};

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

impl Default for LuaEventBridge {
    fn default() -> Self {
        Self::new()
    }
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
    pub fn register(&mut self, lua: &Lua, event: String, callback: Function) -> mlua::Result<()> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lua() -> Lua {
        Lua::new()
    }

    #[test]
    fn new_bridge_is_empty() {
        let bridge = LuaEventBridge::new();
        assert_eq!(bridge.event_count(), 0);
        assert_eq!(bridge.callback_count(), 0);
        assert!(!bridge.has_callbacks("sale.completed"));
    }

    #[test]
    fn default_bridge_is_empty() {
        let bridge = LuaEventBridge::default();
        assert_eq!(bridge.event_count(), 0);
    }

    #[test]
    fn register_single_callback() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let func = lua.create_function(|_, ()| Ok(())).unwrap();
        bridge.register(&lua, "test.event".into(), func).unwrap();

        assert_eq!(bridge.event_count(), 1);
        assert_eq!(bridge.callback_count(), 1);
        assert!(bridge.has_callbacks("test.event"));
        assert!(!bridge.has_callbacks("other.event"));
    }

    #[test]
    fn register_multiple_callbacks_same_event() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let f1 = lua.create_function(|_, ()| Ok(())).unwrap();
        let f2 = lua.create_function(|_, ()| Ok(())).unwrap();

        bridge.register(&lua, "evt".into(), f1).unwrap();
        bridge.register(&lua, "evt".into(), f2).unwrap();

        assert_eq!(bridge.event_count(), 1);
        assert_eq!(bridge.callback_count(), 2);
    }

    #[test]
    fn fire_unregistered_event_is_ok() {
        let lua = make_lua();
        let bridge = LuaEventBridge::new();

        let result = bridge.fire(&lua, "nonexistent", Value::Nil);
        assert!(result.is_ok());
    }

    #[test]
    fn fire_invokes_callback() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let globals = lua.globals();
        globals.set("called", false).unwrap();

        let func = lua
            .create_function(|lua, ()| {
                lua.globals().set("called", true)?;
                Ok(())
            })
            .unwrap();

        bridge.register(&lua, "test.event".into(), func).unwrap();
        bridge.fire(&lua, "test.event", Value::Nil).unwrap();

        let called: bool = globals.get("called").unwrap();
        assert!(called, "callback should have set 'called' to true");
    }

    #[test]
    fn fire_passes_arguments() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let globals = lua.globals();
        globals.set("received_val", 0i64).unwrap();

        let func = lua
            .create_function(move |lua, args: Value| {
                if let Value::Table(tbl) = args {
                    let val: i64 = tbl.get("amount")?;
                    lua.globals().set("received_val", val)?;
                }
                Ok(())
            })
            .unwrap();

        bridge.register(&lua, "test.event".into(), func).unwrap();

        let args = lua.create_table().unwrap();
        args.set("amount", 42i64).unwrap();

        bridge.fire(&lua, "test.event", Value::Table(args)).unwrap();

        let received: i64 = globals.get("received_val").unwrap();
        assert_eq!(received, 42);
    }

    #[test]
    fn fire_handles_callback_error() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let func = lua
            .create_function(|_, ()| {
                Err::<(), _>(mlua::Error::RuntimeError("deliberate fail".into()))
            })
            .unwrap();

        bridge.register(&lua, "test.event".into(), func).unwrap();

        let result = bridge.fire(&lua, "test.event", Value::Nil);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("deliberate fail"),
            "error should contain inner Lua error message, got: {err_msg}"
        );
    }

    #[test]
    fn fire_partial_success_clears_error() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let f_bad = lua
            .create_function(|_, ()| Err::<(), _>(mlua::Error::RuntimeError("fail 1".into())))
            .unwrap();
        let f_good = lua.create_function(|_, ()| Ok(())).unwrap();

        bridge.register(&lua, "bad.event".into(), f_bad).unwrap();
        bridge.register(&lua, "bad.event".into(), f_good).unwrap();

        let result = bridge.fire(&lua, "bad.event", Value::Nil);
        assert!(
            result.is_ok(),
            "if at least one callback succeeds, fire should return Ok"
        );
    }

    #[test]
    fn off_removes_callbacks_for_event() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let f1 = lua.create_function(|_, ()| Ok(())).unwrap();
        let f2 = lua.create_function(|_, ()| Ok(())).unwrap();

        bridge.register(&lua, "evt1".into(), f1).unwrap();
        bridge.register(&lua, "evt2".into(), f2).unwrap();

        assert_eq!(bridge.event_count(), 2);

        bridge.off("evt1");

        assert_eq!(bridge.event_count(), 1);
        assert!(!bridge.has_callbacks("evt1"));
        assert!(bridge.has_callbacks("evt2"));
    }

    #[test]
    fn clear_removes_all_callbacks() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let f1 = lua.create_function(|_, ()| Ok(())).unwrap();
        let f2 = lua.create_function(|_, ()| Ok(())).unwrap();

        bridge.register(&lua, "evt1".into(), f1).unwrap();
        bridge.register(&lua, "evt2".into(), f2).unwrap();

        bridge.clear();

        assert_eq!(bridge.event_count(), 0);
        assert_eq!(bridge.callback_count(), 0);
    }
}

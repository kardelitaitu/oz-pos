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

/// A single registered callback plus its owning plugin id.
#[derive(Debug)]
struct CallbackEntry {
    /// The plugin id that registered this callback (empty for system callbacks).
    owner: String,
    /// Registry key for the Lua callback function.
    key: RegistryKey,
}

/// Manages Lua event callbacks registered via `oz.on()`.
///
/// Callbacks are stored as `RegistryKey`s in the Lua registry so they
/// survive garbage collection between script loads. The bridge is
/// designed to be shared between the `PluginManager` and the Lua runtime.
#[derive(Debug)]
pub struct LuaEventBridge {
    /// Registry keys for callback functions, indexed by event name.
    callbacks: HashMap<String, Vec<CallbackEntry>>,
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
    /// across script reloads and survives GC. The callback is tagged with an
    /// empty owner (system-level); plugins should use
    /// [`register_for`](LuaEventBridge::register_for) so their callbacks can
    /// be removed when the plugin is disabled or reloaded.
    pub fn register(&mut self, lua: &Lua, event: String, callback: Function) -> mlua::Result<()> {
        self.register_for(String::new(), lua, event, callback)
    }

    /// Register a Lua callback for an event, tagged with the owning plugin id.
    ///
    /// All callbacks registered by one plugin can be removed with a single
    /// [`remove_owner`](LuaEventBridge::remove_owner) call when the plugin is
    /// disabled, reloaded, or fails to load.
    pub fn register_for(
        &mut self,
        owner: String,
        lua: &Lua,
        event: String,
        callback: Function,
    ) -> mlua::Result<()> {
        let key = lua.create_registry_value(callback)?;
        self.callbacks
            .entry(event)
            .or_default()
            .push(CallbackEntry { owner, key });
        Ok(())
    }

    /// Fire an event: call all registered Lua callbacks with the given args.
    ///
    /// Returns `Ok(())` even if individual callbacks error — errors are
    /// collected and returned as a single joined error string only if ALL
    /// callbacks fail. If at least one succeeds, the result is `Ok(())` and
    /// any partial failures are logged as warnings for observability.
    pub fn fire(&self, lua: &Lua, event: &str, args: Value) -> Result<(), crate::LuaError> {
        let Some(entries) = self.callbacks.get(event) else {
            return Ok(());
        };
        if entries.is_empty() {
            return Ok(());
        }

        let mut errors: Vec<String> = Vec::new();
        let mut success_count = 0usize;

        for entry in entries {
            let callback: Function = match lua.registry_value(&entry.key) {
                Ok(f) => f,
                Err(e) => {
                    errors.push(format!("failed to retrieve callback for '{event}': {e}"));
                    continue;
                }
            };

            match callback.call::<_, ()>(args.clone()) {
                Ok(()) => success_count += 1,
                Err(e) => errors.push(format!("callback for '{event}' failed: {e}")),
            }
        }

        // Documented contract: Ok unless EVERY callback failed.
        if success_count > 0 {
            if !errors.is_empty() {
                tracing::warn!(
                    event,
                    failed = errors.len(),
                    succeeded = success_count,
                    "partial callback failures for event"
                );
            }
            return Ok(());
        }

        if errors.is_empty() {
            return Ok(());
        }

        Err(crate::LuaError::Script(format!(
            "all {} callbacks for '{event}' failed: {}",
            errors.len(),
            errors.join("; ")
        )))
    }

    /// Remove all callbacks for a specific event.
    ///
    /// This drops the registry keys, allowing the Lua GC to collect
    /// the callback functions.
    pub fn off(&mut self, event: &str) {
        if let Some(entries) = self.callbacks.remove(event) {
            // RegistryKeys are dropped here, which removes them from the
            // Lua registry.
            drop(entries);
        }
    }

    /// Remove every callback registered by `owner` across all events.
    ///
    /// Used when a plugin is disabled or reloaded so stale callbacks from a
    /// removed plugin can never fire again.
    pub fn remove_owner(&mut self, owner: &str) {
        if owner.is_empty() {
            return;
        }
        self.callbacks.retain(|_, entries| {
            entries.retain(|e| e.owner != owner);
            !entries.is_empty()
        });
    }

    /// Remove every callback registered by `owner` for a single event.
    ///
    /// Backs the plugin-side `oz.off(event)` so one plugin can only ever
    /// unsubscribe its own callbacks, never another plugin's (PLG-04).
    pub fn off_for(&mut self, owner: &str, event: &str) {
        if owner.is_empty() {
            return;
        }
        if let Some(entries) = self.callbacks.get_mut(event) {
            entries.retain(|e| e.owner != owner);
            if entries.is_empty() {
                self.callbacks.remove(event);
            }
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
        // Failure BEFORE success must still return Ok (documented contract).
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
    fn fire_success_then_failure_is_still_ok() {
        // PLG-05 regression: success-then-failure previously returned Err
        // because the error was cleared on success and re-set on the later
        // failure — order-dependent and contrary to the documented contract.
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let f_good = lua.create_function(|_, ()| Ok(())).unwrap();
        let f_bad = lua
            .create_function(|_, ()| Err::<(), _>(mlua::Error::RuntimeError("fail 2".into())))
            .unwrap();

        bridge.register(&lua, "order.event".into(), f_good).unwrap();
        bridge.register(&lua, "order.event".into(), f_bad).unwrap();

        let result = bridge.fire(&lua, "order.event", Value::Nil);
        assert!(
            result.is_ok(),
            "success-then-failure must still be Ok (at least one succeeded)"
        );
    }

    #[test]
    fn fire_all_fail_returns_aggregated_error() {
        // PLG-05: when EVERY callback fails, fire must return Err with the
        // individual errors joined, regardless of order.
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let f1 = lua
            .create_function(|_, ()| Err::<(), _>(mlua::Error::RuntimeError("fail A".into())))
            .unwrap();
        let f2 = lua
            .create_function(|_, ()| Err::<(), _>(mlua::Error::RuntimeError("fail B".into())))
            .unwrap();

        bridge.register(&lua, "allfail.event".into(), f1).unwrap();
        bridge.register(&lua, "allfail.event".into(), f2).unwrap();

        let result = bridge.fire(&lua, "allfail.event", Value::Nil);
        assert!(result.is_err(), "all-failed must return Err");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("fail A") && err_msg.contains("fail B"),
            "aggregated error should contain every callback failure, got: {err_msg}"
        );
        assert!(
            err_msg.contains("all 2 callbacks"),
            "error should report the count, got: {err_msg}"
        );
    }

    #[test]
    fn fire_all_fail_returns_err_in_reverse_order_too() {
        // Order independence: all-failed must be Err in any registration order.
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let f1 = lua
            .create_function(|_, ()| Err::<(), _>(mlua::Error::RuntimeError("fail X".into())))
            .unwrap();
        let f2 = lua
            .create_function(|_, ()| Err::<(), _>(mlua::Error::RuntimeError("fail Y".into())))
            .unwrap();

        bridge.register(&lua, "rev.event".into(), f2).unwrap();
        bridge.register(&lua, "rev.event".into(), f1).unwrap();

        let result = bridge.fire(&lua, "rev.event", Value::Nil);
        assert!(
            result.is_err(),
            "all-failed must be Err regardless of order"
        );
    }

    #[test]
    fn fire_zero_callbacks_is_ok() {
        let lua = make_lua();
        let bridge = LuaEventBridge::new();
        let result = bridge.fire(&lua, "none.event", Value::Nil);
        assert!(result.is_ok());
    }

    #[test]
    fn remove_owner_removes_only_that_plugins_callbacks() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let f_a = lua.create_function(|_, ()| Ok(())).unwrap();
        let f_b = lua.create_function(|_, ()| Ok(())).unwrap();

        bridge
            .register_for("plugin-a".into(), &lua, "evt".into(), f_a)
            .unwrap();
        bridge
            .register_for("plugin-b".into(), &lua, "evt".into(), f_b)
            .unwrap();

        assert_eq!(bridge.callback_count(), 2);
        bridge.remove_owner("plugin-a");

        assert_eq!(bridge.callback_count(), 1);
        assert!(bridge.has_callbacks("evt"));
    }

    #[test]
    fn remove_owner_across_multiple_events() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let f1 = lua.create_function(|_, ()| Ok(())).unwrap();
        let f2 = lua.create_function(|_, ()| Ok(())).unwrap();
        let f3 = lua.create_function(|_, ()| Ok(())).unwrap();

        bridge
            .register_for("p1".into(), &lua, "e1".into(), f1)
            .unwrap();
        bridge
            .register_for("p1".into(), &lua, "e2".into(), f2)
            .unwrap();
        bridge
            .register_for("p2".into(), &lua, "e2".into(), f3)
            .unwrap();

        assert_eq!(bridge.event_count(), 2);
        bridge.remove_owner("p1");

        // e1 is now empty and should be dropped; e2 keeps only p2's callback.
        assert_eq!(bridge.event_count(), 1);
        assert!(!bridge.has_callbacks("e1"));
        assert!(bridge.has_callbacks("e2"));
        assert_eq!(bridge.callback_count(), 1);
    }

    #[test]
    fn remove_owner_with_empty_owner_is_noop() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();
        let f = lua.create_function(|_, ()| Ok(())).unwrap();
        bridge.register(&lua, "evt".into(), f).unwrap();

        // System callbacks (empty owner) are never removed by owner cleanup.
        bridge.remove_owner("");
        assert_eq!(bridge.callback_count(), 1);
    }

    #[test]
    fn register_for_and_fire_with_owner() {
        let lua = make_lua();
        let mut bridge = LuaEventBridge::new();

        let globals = lua.globals();
        globals.set("owner_called", false).unwrap();
        let f = lua
            .create_function(|lua, ()| {
                lua.globals().set("owner_called", true)?;
                Ok(())
            })
            .unwrap();
        bridge
            .register_for("plug".into(), &lua, "evt".into(), f)
            .unwrap();

        bridge.fire(&lua, "evt", Value::Nil).unwrap();
        let called: bool = globals.get("owner_called").unwrap();
        assert!(called);
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

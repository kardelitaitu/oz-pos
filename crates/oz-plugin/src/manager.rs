use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{Datelike, Timelike};
use oz_lua::{CartLineData, DiscountResult, LuaError, LuaEventBridge, LuaRuntime, TaxOverride};

use crate::error::PluginError;
use crate::loader::{PluginRegistry, load_plugins};
use crate::manifest::Permission;

/// A discount queued by a plugin script for later application.
#[derive(Debug, Clone)]
pub struct PendingDiscount {
    /// Discount target — `"cart"` or `"line:<SKU>"`.
    pub target: String,
    /// Discount percentage (0–100).
    pub percent: i64,
}

/// Runtime manager for Lua plugin scripts.
///
/// Manages the Lua sandbox, plugin lifecycle, hook registration,
/// discount accumulation, and event dispatching.
pub struct PluginManager {
    runtime: LuaRuntime,
    _registry: PluginRegistry,
    hook_names: Arc<Mutex<HashMap<String, Vec<String>>>>,
    pending_discounts: Arc<Mutex<Vec<PendingDiscount>>>,
    bridge: Arc<Mutex<LuaEventBridge>>,
}

impl std::fmt::Debug for PluginManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginManager").finish_non_exhaustive()
    }
}

impl PluginManager {
    /// Whitelist of all recognised plugin permissions.
    /// Plugins that declare permissions outside this set are rejected at load time.
    const ALLOWED_PERMISSIONS: &'static [Permission] = &[
        Permission::CartRead,
        Permission::CartWrite,
        Permission::TaxRead,
        Permission::InventoryRead,
        Permission::InventoryWrite,
        Permission::ReportingRead,
        Permission::SystemTime,
        Permission::LogWrite,
    ];

    /// Create a new `PluginManager`, loading all plugins from `plugins_dir`.
    pub fn new(plugins_dir: &Path) -> Result<Self, PluginError> {
        let registry = load_plugins(plugins_dir)?;

        // ── Enforce plugin permissions ──────────────────────────────
        for plugin in &registry.plugins {
            // Check that all declared permissions are in the whitelist.
            for perm in &plugin.manifest.permissions.required_permissions {
                if !Self::ALLOWED_PERMISSIONS.contains(perm) {
                    return Err(PluginError::Manifest(format!(
                        "plugin '{}' declares unknown permission '{}' — rejected",
                        plugin.manifest.plugin.name, perm,
                    )));
                }
            }
            // Require at minimum: at least one permission must be declared.
            // Plugins with zero declared permissions are rejected to force
            // explicit opt-in.
            if plugin.manifest.permissions.required_permissions.is_empty() {
                return Err(PluginError::Manifest(format!(
                    "plugin '{}' declares no required_permissions — \
                     at least one permission must be declared (e.g. [\"cart:read\"])",
                    plugin.manifest.plugin.name,
                )));
            }
        }

        let runtime = LuaRuntime::new().map_err(|e| PluginError::Lua(e.to_string()))?;

        let hook_names: Arc<Mutex<HashMap<String, Vec<String>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_discounts: Arc<Mutex<Vec<PendingDiscount>>> = Arc::new(Mutex::new(Vec::new()));
        let bridge: Arc<Mutex<LuaEventBridge>> = Arc::new(Mutex::new(LuaEventBridge::new()));

        // ── Register the `oz.*` API before loading any scripts ──
        {
            let lua = runtime.inner();
            let globals = lua.globals();

            let oz_table = lua
                .create_table()
                .map_err(|e| PluginError::Lua(e.to_string()))?;

            // oz.get_time() → { wday, hour, min, sec, month, day, year }
            let get_time_fn = lua
                .create_function(|ctx, ()| {
                    let now = chrono::Utc::now();
                    let tbl = ctx.create_table()?;
                    tbl.set(
                        "wday",
                        now.format("%u").to_string().parse::<u32>().unwrap_or(0),
                    )?;
                    tbl.set("hour", now.hour())?;
                    tbl.set("min", now.minute())?;
                    tbl.set("sec", now.second())?;
                    tbl.set("month", now.month())?;
                    tbl.set("day", now.day())?;
                    tbl.set("year", now.year())?;
                    Ok(tbl)
                })
                .map_err(|e| PluginError::Lua(e.to_string()))?;
            oz_table
                .set("get_time", get_time_fn)
                .map_err(|e| PluginError::Lua(e.to_string()))?;

            // oz.log(level, message)
            let log_fn = lua
                .create_function(|_, (level, message): (String, String)| {
                    match level.as_str() {
                        "error" => tracing::error!(target: "plugin", "{message}"),
                        "warn" => tracing::warn!(target: "plugin", "{message}"),
                        "info" => tracing::info!(target: "plugin", "{message}"),
                        "debug" => tracing::debug!(target: "plugin", "{message}"),
                        _ => tracing::info!(target: "plugin", "[{level}] {message}"),
                    }
                    Ok(())
                })
                .map_err(|e| PluginError::Lua(e.to_string()))?;
            oz_table
                .set("log", log_fn)
                .map_err(|e| PluginError::Lua(e.to_string()))?;

            // oz.apply_discount(target, percent) — captures Arc
            let pd = pending_discounts.clone();
            let apply_discount_fn = lua
                .create_function(move |_, (target, percent): (String, i64)| {
                    if let Ok(mut guard) = pd.lock() {
                        guard.push(PendingDiscount { target, percent });
                    }
                    Ok(())
                })
                .map_err(|e| PluginError::Lua(e.to_string()))?;
            oz_table
                .set("apply_discount", apply_discount_fn)
                .map_err(|e| PluginError::Lua(e.to_string()))?;

            // oz.register_hook(event, function_name) — captures Arc
            let hn = hook_names.clone();
            let register_hook_fn = lua
                .create_function(move |_, (event, func_name): (String, String)| {
                    if let Ok(mut guard) = hn.lock() {
                        guard.entry(event).or_default().push(func_name);
                    }
                    Ok(())
                })
                .map_err(|e| PluginError::Lua(e.to_string()))?;
            oz_table
                .set("register_hook", register_hook_fn)
                .map_err(|e| PluginError::Lua(e.to_string()))?;

            // oz.on(event, callback) — registers a Lua function callback
            let br = bridge.clone();
            let on_fn = lua
                .create_function(move |lua, (event, callback): (String, rlua::Function)| {
                    if let Ok(mut guard) = br.lock() {
                        guard
                            .register(lua, event, callback)
                            .map_err(|e| rlua::Error::RuntimeError(format!("oz.on error: {e}")))?;
                    }
                    Ok(())
                })
                .map_err(|e| PluginError::Lua(e.to_string()))?;
            oz_table
                .set("on", on_fn)
                .map_err(|e| PluginError::Lua(e.to_string()))?;

            // oz.off(event) — removes all callbacks for an event
            let br_off = bridge.clone();
            let off_fn = lua
                .create_function(move |_, event: String| {
                    if let Ok(mut guard) = br_off.lock() {
                        guard.off(&event);
                    }
                    Ok(())
                })
                .map_err(|e| PluginError::Lua(e.to_string()))?;
            oz_table
                .set("off", off_fn)
                .map_err(|e| PluginError::Lua(e.to_string()))?;

            globals
                .set("oz", oz_table)
                .map_err(|e| PluginError::Lua(e.to_string()))?;
        }

        // ── Load all plugin scripts ──────────────────────────────
        for plugin in &registry.plugins {
            for script in &plugin.scripts {
                runtime
                    .load_file(script)
                    .map_err(|e| PluginError::Lua(format!("{}: {e}", script.display())))?;
                tracing::info!(
                    plugin = %plugin.manifest.plugin.name,
                    script = %script.display(),
                    "plugin script loaded"
                );
            }
        }

        Ok(Self {
            runtime,
            _registry: registry,
            hook_names,
            pending_discounts,
            bridge,
        })
    }

    /// Delegate to `LuaRuntime::validate_order`.
    pub fn validate_order(
        &self,
        lines: &[CartLineData],
        total_minor: i64,
        currency: &str,
    ) -> Result<Vec<String>, LuaError> {
        self.runtime.validate_order(lines, total_minor, currency)
    }

    /// Delegate to `LuaRuntime::apply_discount`.
    pub fn apply_discount(
        &self,
        lines: &[CartLineData],
    ) -> Result<Option<DiscountResult>, LuaError> {
        self.runtime.apply_discount(lines)
    }

    /// Delegate to `LuaRuntime::calc_line_tax`.
    pub fn calc_line_tax(
        &self,
        sku: &str,
        qty: i64,
        unit_price_minor: i64,
        currency: &str,
    ) -> Result<Option<TaxOverride>, LuaError> {
        self.runtime
            .calc_line_tax(sku, qty, unit_price_minor, currency)
    }

    /// Drain all queued discounts, returning them and clearing the queue.
    pub fn drain_pending_discounts(&self) -> Vec<PendingDiscount> {
        self.pending_discounts
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default()
    }

    /// Build a sale table and fire the `sale.before_complete` event.
    ///
    /// The table passed to Lua hooks contains:
    /// ```lua
    /// { total_minor, currency, user_id, lines = { { sku, qty, unit_price_minor, currency }, ... } }
    /// ```
    pub fn fire_sale_before_complete(
        &self,
        lines: &[CartLineData],
        total_minor: i64,
        currency: &str,
        user_id: &str,
    ) -> Result<(), LuaError> {
        let lua = self.runtime.inner();
        let tbl = lua
            .create_table()
            .map_err(|e| LuaError::Script(e.to_string()))?;
        tbl.set("total_minor", total_minor)
            .map_err(|e| LuaError::Script(e.to_string()))?;
        tbl.set("currency", currency)
            .map_err(|e| LuaError::Script(e.to_string()))?;
        tbl.set("user_id", user_id)
            .map_err(|e| LuaError::Script(e.to_string()))?;

        let lines_tbl = lua
            .create_table()
            .map_err(|e| LuaError::Script(e.to_string()))?;
        for (i, line) in lines.iter().enumerate() {
            let row = lua
                .create_table()
                .map_err(|e| LuaError::Script(e.to_string()))?;
            row.set("sku", line.sku.as_str())
                .map_err(|e| LuaError::Script(e.to_string()))?;
            row.set("qty", line.qty)
                .map_err(|e| LuaError::Script(e.to_string()))?;
            row.set("unit_price_minor", line.unit_price_minor)
                .map_err(|e| LuaError::Script(e.to_string()))?;
            row.set("currency", line.currency.as_str())
                .map_err(|e| LuaError::Script(e.to_string()))?;
            lines_tbl
                .set(i + 1, row)
                .map_err(|e| LuaError::Script(e.to_string()))?;
        }
        tbl.set("lines", lines_tbl)
            .map_err(|e| LuaError::Script(e.to_string()))?;

        self.fire_event("sale.before_complete", rlua::Value::Table(tbl))
    }

    /// Fire an event to all Lua callbacks registered via `oz.on()`.
    ///
    /// This calls the `LuaEventBridge` to dispatch the event to all
    /// registered Lua function callbacks.
    pub fn fire_bridge_event(&self, event: &str, args: rlua::Value) -> Result<(), LuaError> {
        if let Ok(guard) = self.bridge.lock() {
            guard.fire(self.runtime.inner(), event, args)
        } else {
            Err(LuaError::Script("bridge lock poisoned".into()))
        }
    }

    /// Fire an event to all registered hook functions by name.
    pub fn fire_event(&self, event: &str, args: rlua::Value) -> Result<(), LuaError> {
        let func_names = self
            .hook_names
            .lock()
            .map(|g| g.get(event).cloned().unwrap_or_default())
            .unwrap_or_default();

        for func_name in &func_names {
            let func: rlua::Function = {
                let globals = self.runtime.inner().globals();
                match globals.get(func_name.as_str()) {
                    Ok(f) => f,
                    Err(_) => {
                        tracing::warn!(
                            event,
                            func = %func_name,
                            "hook function not found in globals"
                        );
                        continue;
                    }
                }
            };
            func.call::<_, ()>(args.clone())
                .map_err(|e| LuaError::Script(format!("hook {event}/{func_name}: {e}")))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── Helpers ────────────────────────────────────────────────────────

    /// Create a temp plugin directory with `plugin.toml` and `script.lua`.
    /// Returns (TempDir, plugins_root_dir) — the TempDir must be kept alive.
    fn create_plugin_dir(name: &str, lua_content: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join(name);
        std::fs::create_dir(&plugin_dir).unwrap();

        let manifest = format!(
            "[plugin]\nname = \"{}\"\nversion = \"1.0.0\"\n\n[capabilities]\nscripts = [\"script.lua\"]\n\n[permissions]\nrequired_permissions = [\"cart:read\"]\n",
            name
        );
        std::fs::write(plugin_dir.join("plugin.toml"), manifest).unwrap();
        std::fs::write(plugin_dir.join("script.lua"), lua_content).unwrap();

        let plugins_root = dir.path().to_path_buf();
        (dir, plugins_root)
    }

    /// Helper to build a single `CartLineData`.
    fn line(sku: &str, qty: i64, unit_price_minor: i64, currency: &str) -> CartLineData {
        CartLineData {
            sku: sku.into(),
            qty,
            unit_price_minor,
            currency: currency.into(),
        }
    }

    // ── PendingDiscount tests (existing) ───────────────────────────────

    #[test]
    fn pending_discount_new() {
        let d = PendingDiscount {
            target: "COFFEE".into(),
            percent: 10,
        };
        assert_eq!(d.target, "COFFEE");
        assert_eq!(d.percent, 10);
    }

    #[test]
    fn pending_discount_debug() {
        let d = PendingDiscount {
            target: "COFFEE".into(),
            percent: 10,
        };
        let debug = format!("{d:?}");
        assert!(debug.contains("COFFEE"));
        assert!(debug.contains("10"));
    }

    #[test]
    fn pending_discount_clone() {
        let d = PendingDiscount {
            target: "TEA".into(),
            percent: 25,
        };
        let cloned = d.clone();
        assert_eq!(d.target, cloned.target);
        assert_eq!(d.percent, cloned.percent);
    }

    #[test]
    fn pending_discount_zero_percent() {
        let d = PendingDiscount {
            target: "ITEM".into(),
            percent: 0,
        };
        assert_eq!(d.percent, 0);
    }

    #[test]
    fn pending_discount_large_percent() {
        let d = PendingDiscount {
            target: "ITEM".into(),
            percent: 100,
        };
        assert_eq!(d.percent, 100);
    }

    // ── PluginManager::new() tests ─────────────────────────────────────

    // ── Permission enforcement tests ─────────────────────────────────

    #[test]
    fn plugin_without_permissions_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("no-perms");
        std::fs::create_dir(&plugin_dir).unwrap();
        // No [permissions] section at all — should be rejected.
        std::fs::write(
            plugin_dir.join("plugin.toml"),
            "[plugin]\nname = \"no-perms\"\nversion = \"1.0.0\"\n\n[capabilities]\nscripts = []\n",
        )
        .unwrap();
        let result = PluginManager::new(dir.path());
        assert!(
            result.is_err(),
            "plugin without permissions should be rejected"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("required_permissions") || err.contains("no-perms"),
            "error should mention missing permissions, got: {err}"
        );
    }

    #[test]
    fn plugin_with_cart_read_permission_succeeds() {
        let (_dir, plugins_root) = create_plugin_dir("cart-only", "");
        // create_plugin_dir already includes required_permissions = ["cart:read"]
        let mgr = PluginManager::new(&plugins_root).unwrap();
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn multiple_plugins_with_valid_permissions_all_load() {
        let dir = tempfile::tempdir().unwrap();
        for (i, name) in ["plug-a", "plug-b"].iter().enumerate() {
            let plugin_dir = dir.path().join(name);
            std::fs::create_dir(&plugin_dir).unwrap();
            std::fs::write(
                plugin_dir.join("plugin.toml"),
                format!(
                    r#"[plugin]
name = "{name}"
version = "1.0.0"

[capabilities]
scripts = []

[permissions]
required_permissions = ["cart:read"]
"#
                ),
            )
            .unwrap();
            let _ = i; // suppress warning
        }
        let mgr = PluginManager::new(dir.path()).unwrap();
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn plugin_with_unknown_permission_silently_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("unknown-perm");
        std::fs::create_dir(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.toml"),
            r#"[plugin]
name = "unknown-perm"
version = "1.0.0"

[permissions]
required_permissions = ["cart:read", "super:admin"]
"#,
        )
        .unwrap();
        // "super:admin" is unrecognised — silently ignored for forward compat.
        // "cart:read" is valid, so this plugin should load.
        let result = PluginManager::new(dir.path());
        assert!(
            result.is_ok(),
            "unknown permission should be silently ignored (forward compat)"
        );
    }

    #[test]
    fn plugin_with_all_permission_types_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("all-perms");
        std::fs::create_dir(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.toml"),
            r#"[plugin]
name = "all-perms"
version = "1.0.0"

[permissions]
required_permissions = [
    "cart:read",
    "cart:write",
    "tax:read",
    "inventory:read",
    "inventory:write",
    "reporting:read",
    "system:time",
    "log:write",
]
"#,
        )
        .unwrap();
        let mgr = PluginManager::new(dir.path()).unwrap();
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn permission_display_format() {
        use crate::manifest::Permission;
        assert_eq!(Permission::CartRead.to_string(), "cart:read");
        assert_eq!(Permission::CartWrite.to_string(), "cart:write");
        assert_eq!(Permission::TaxRead.to_string(), "tax:read");
        assert_eq!(Permission::InventoryRead.to_string(), "inventory:read");
        assert_eq!(Permission::InventoryWrite.to_string(), "inventory:write");
        assert_eq!(Permission::ReportingRead.to_string(), "reporting:read");
        assert_eq!(Permission::SystemTime.to_string(), "system:time");
        assert_eq!(Permission::LogWrite.to_string(), "log:write");
    }

    // ── Existing tests ────────────────────────────────────────────────

    #[test]
    fn plugin_manager_new_with_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PluginManager::new(dir.path()).unwrap();
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn plugin_manager_new_with_nonexistent_dir() {
        let mgr = PluginManager::new(Path::new("/nonexistent/plugin/dir")).unwrap();
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn plugin_manager_new_with_empty_script() {
        let (_dir, plugins_root) = create_plugin_dir("empty-plugin", "");
        let mgr = PluginManager::new(&plugins_root).unwrap();
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn plugin_manager_new_with_hook_registration_and_no_side_effects() {
        let lua = r#"
function my_hook(sale)
    -- no-op hook
end
oz.register_hook("sale.before_complete", "my_hook")
"#;
        let (_dir, plugins_root) = create_plugin_dir("hook-plugin", lua);
        let mgr = PluginManager::new(&plugins_root).unwrap();
        // Hook was registered but not fired — no discounts pushed
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn plugin_manager_new_with_top_level_discount_push() {
        let lua = r#"
oz.apply_discount("cart", 10)
oz.apply_discount("line:SKU123", 20)
"#;
        let (_dir, plugins_root) = create_plugin_dir("discount-plugin", lua);
        let mgr = PluginManager::new(&plugins_root).unwrap();
        let discounts = mgr.drain_pending_discounts();
        assert_eq!(discounts.len(), 2);
        assert_eq!(discounts[0].target, "cart");
        assert_eq!(discounts[0].percent, 10);
        assert_eq!(discounts[1].target, "line:SKU123");
        assert_eq!(discounts[1].percent, 20);
    }

    #[test]
    fn plugin_manager_new_with_invalid_lua_syntax() {
        let lua = "function broken(syntax";
        let (_dir, plugins_root) = create_plugin_dir("broken-plugin", lua);
        let result = PluginManager::new(&plugins_root);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Lua") || err.contains("script"),
            "expected Lua error, got: {err}"
        );
    }

    #[test]
    fn plugin_manager_new_with_real_example_discount_plugin() {
        // Use the real example-discount plugin from the workspace.
        let plugins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins");
        let mgr = PluginManager::new(&plugins_dir).unwrap();
        // The example-discount registers a hook, no top-level discount push.
        let discounts = mgr.drain_pending_discounts();
        assert!(discounts.is_empty());
    }

    #[test]
    fn real_example_plugin_hook_executes_without_error() {
        // Regression test for P0-5: verify the real example-discount
        // plugin's hook fires without crashing or errors.
        // The plugin applies a 10% discount on Tuesdays (wday == 3),
        // so the discount may or may not be created depending on the
        // current day — this test verifies the hook machinery works.
        let plugins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins");
        let mgr = PluginManager::new(&plugins_dir).unwrap();

        let lines = [line("TEST", 1, 1000, "USD")];
        let result = mgr.fire_sale_before_complete(&lines, 1000, "USD", "user-1");
        assert!(result.is_ok(), "hook should execute without error");

        // Hook may or may not push a discount (depends on day of week),
        // but drain must succeed without panic.
        let _ = mgr.drain_pending_discounts();
    }

    // ── drain_pending_discounts tests ──────────────────────────────────

    #[test]
    fn drain_pending_discounts_empty_after_fresh_init() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PluginManager::new(dir.path()).unwrap();
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn drain_pending_discounts_drains_after_top_level_push() {
        let lua = r#"
oz.apply_discount("cart", 5)
oz.apply_discount("cart", 15)
oz.apply_discount("line:A", 25)
"#;
        let (_dir, plugins_root) = create_plugin_dir("multi-discount", lua);
        let mgr = PluginManager::new(&plugins_root).unwrap();

        let discounts = mgr.drain_pending_discounts();
        assert_eq!(discounts.len(), 3);

        // Second drain is empty (already drained)
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn drain_pending_discounts_after_hook_fire() {
        let lua = r#"
function my_hook(sale)
    oz.apply_discount("cart", 15)
end
oz.register_hook("sale.before_complete", "my_hook")
"#;
        let (_dir, plugins_root) = create_plugin_dir("hook-discount", lua);
        let mgr = PluginManager::new(&plugins_root).unwrap();

        // Fire the hook — it should push a discount
        let lines = [line("ITEM", 1, 1000, "USD")];
        mgr.fire_sale_before_complete(&lines, 1000, "USD", "user-1")
            .unwrap();

        let discounts = mgr.drain_pending_discounts();
        assert_eq!(discounts.len(), 1);
        assert_eq!(discounts[0].target, "cart");
        assert_eq!(discounts[0].percent, 15);
    }

    // ── fire_sale_before_complete tests ────────────────────────────────

    #[test]
    fn fire_sale_before_complete_no_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PluginManager::new(dir.path()).unwrap();

        let result = mgr.fire_sale_before_complete(&[], 0, "USD", "anon");
        assert!(result.is_ok());
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn fire_sale_before_complete_with_hook_discounts() {
        let lua = r#"
function on_sale(sale)
    if sale.total_minor >= 5000 then
        oz.apply_discount("cart", 10)
    end
end
oz.register_hook("sale.before_complete", "on_sale")
"#;
        let (_dir, plugins_root) = create_plugin_dir("threshold-hook", lua);
        let mgr = PluginManager::new(&plugins_root).unwrap();

        // Below threshold — no discount
        mgr.fire_sale_before_complete(&[line("CHEAP", 1, 100, "IDR")], 100, "IDR", "user-1")
            .unwrap();
        assert!(mgr.drain_pending_discounts().is_empty());

        // Above threshold — discount should fire
        mgr.fire_sale_before_complete(
            &[line("EXPENSIVE", 1, 10000, "IDR")],
            10000,
            "IDR",
            "user-1",
        )
        .unwrap();
        let discounts = mgr.drain_pending_discounts();
        assert_eq!(discounts.len(), 1);
        assert_eq!(discounts[0].percent, 10);
    }

    #[test]
    fn fire_sale_before_complete_with_multiple_lines() {
        let lua = r#"
function count_lines(sale)
    local count = 0
    for i = 1, #sale.lines do
        count = count + 1
    end
    oz.apply_discount("cart", count * 5)
end
oz.register_hook("sale.before_complete", "count_lines")
"#;
        let (_dir, plugins_root) = create_plugin_dir("count-hook", lua);
        let mgr = PluginManager::new(&plugins_root).unwrap();

        mgr.fire_sale_before_complete(
            &[
                line("A", 1, 500, "USD"),
                line("B", 2, 300, "USD"),
                line("C", 1, 1000, "USD"),
            ],
            2100,
            "USD",
            "user-1",
        )
        .unwrap();

        let discounts = mgr.drain_pending_discounts();
        assert_eq!(discounts.len(), 1);
        // 3 lines × 5 = 15
        assert_eq!(discounts[0].percent, 15);
    }

    #[test]
    fn fire_sale_before_complete_preserves_sale_fields() {
        let lua = r#"
function check_sale(sale)
    -- Verify fields then push a discount to signal success
    if sale.total_minor == 5000 and sale.currency == "IDR" and sale.user_id == "cashier-1" then
        oz.apply_discount("cart", 1)
    end
end
oz.register_hook("sale.before_complete", "check_sale")
"#;
        let (_dir, plugins_root) = create_plugin_dir("fields-hook", lua);
        let mgr = PluginManager::new(&plugins_root).unwrap();

        mgr.fire_sale_before_complete(&[line("ITEM", 2, 2500, "IDR")], 5000, "IDR", "cashier-1")
            .unwrap();

        let discounts = mgr.drain_pending_discounts();
        assert_eq!(discounts.len(), 1, "hook should verify sale fields");
    }

    #[test]
    fn fire_sale_before_complete_multiple_hooks_same_event() {
        let lua = r#"
function hook_a(sale)
    oz.apply_discount("cart", 5)
end
function hook_b(sale)
    oz.apply_discount("cart", 10)
end
oz.register_hook("sale.before_complete", "hook_a")
oz.register_hook("sale.before_complete", "hook_b")
"#;
        let (_dir, plugins_root) = create_plugin_dir("multi-hook", lua);
        let mgr = PluginManager::new(&plugins_root).unwrap();

        mgr.fire_sale_before_complete(&[line("X", 1, 100, "USD")], 100, "USD", "u1")
            .unwrap();

        let discounts = mgr.drain_pending_discounts();
        assert_eq!(discounts.len(), 2, "both hooks should fire");
        assert_eq!(discounts[0].percent, 5);
        assert_eq!(discounts[1].percent, 10);
    }

    // ── Delegation method tests ────────────────────────────────────────

    #[test]
    fn validate_order_returns_empty_when_no_hook() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PluginManager::new(dir.path()).unwrap();
        let errors = mgr.validate_order(&[], 0, "USD").unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn apply_discount_returns_none_when_no_hook() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PluginManager::new(dir.path()).unwrap();
        let result = mgr.apply_discount(&[]).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn calc_line_tax_returns_none_when_no_hook() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PluginManager::new(dir.path()).unwrap();
        let result = mgr.calc_line_tax("SKU", 1, 100, "USD").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn validate_order_with_non_empty_lines_no_hook() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PluginManager::new(dir.path()).unwrap();
        let lines = [line("A", 1, 500, "IDR"), line("B", 2, 250, "IDR")];
        let errors = mgr.validate_order(&lines, 1000, "IDR").unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn apply_discount_with_non_empty_lines_no_hook() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PluginManager::new(dir.path()).unwrap();
        let lines = [line("A", 3, 500, "USD")];
        let result = mgr.apply_discount(&lines).unwrap();
        assert!(result.is_none());
    }

    // ── fire_event tests ───────────────────────────────────────────────

    #[test]
    fn fire_event_unregistered_event_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PluginManager::new(dir.path()).unwrap();

        let _lua = mgr.runtime.inner();
        let value = rlua::Value::Nil;
        let result = mgr.fire_event("some.event.never_registered", value);
        assert!(result.is_ok());
        assert!(mgr.drain_pending_discounts().is_empty());
    }

    #[test]
    fn fire_event_with_registered_hook_executes() {
        let lua_script = r#"
function custom_hook(arg)
    oz.apply_discount("custom", 42)
end
oz.register_hook("custom.event", "custom_hook")
"#;
        let (_dir, plugins_root) = create_plugin_dir("custom-event", lua_script);
        let mgr = PluginManager::new(&plugins_root).unwrap();

        let lua = mgr.runtime.inner();
        let value = lua.create_table().unwrap();
        mgr.fire_event("custom.event", rlua::Value::Table(value))
            .unwrap();

        let discounts = mgr.drain_pending_discounts();
        assert_eq!(discounts.len(), 1);
        assert_eq!(discounts[0].target, "custom");
        assert_eq!(discounts[0].percent, 42);
    }

    #[test]
    fn fire_event_registered_but_function_missing_is_ok() {
        let lua_script = r#"
-- Register a hook but don't define the function — manager should warn + continue
oz.register_hook("missing.event", "no_such_function")
"#;
        let (_dir, plugins_root) = create_plugin_dir("missing-func", lua_script);
        let mgr = PluginManager::new(&plugins_root).unwrap();

        let _lua = mgr.runtime.inner();
        let value = rlua::Value::Nil;
        let result = mgr.fire_event("missing.event", value);
        // Should not panic; the missing function is logged and skipped.
        assert!(result.is_ok());
    }
}

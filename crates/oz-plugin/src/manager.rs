use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{Datelike, Timelike};
use oz_lua::{CartLineData, DiscountResult, LuaError, LuaRuntime, TaxOverride};

use crate::error::PluginError;
use crate::loader::PluginRegistry;
use crate::loader::load_plugins;

#[derive(Debug, Clone)]
pub struct PendingDiscount {
    pub target: String,
    pub percent: i64,
}

pub struct PluginManager {
    runtime: LuaRuntime,
    _registry: PluginRegistry,
    hook_names: Arc<Mutex<HashMap<String, Vec<String>>>>,
    pending_discounts: Arc<Mutex<Vec<PendingDiscount>>>,
}

impl PluginManager {
    pub fn new(plugins_dir: &Path) -> Result<Self, PluginError> {
        let registry = load_plugins(plugins_dir)?;
        let runtime = LuaRuntime::new().map_err(|e| PluginError::Lua(e.to_string()))?;

        let hook_names: Arc<Mutex<HashMap<String, Vec<String>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_discounts: Arc<Mutex<Vec<PendingDiscount>>> = Arc::new(Mutex::new(Vec::new()));

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
        })
    }

    pub fn validate_order(
        &self,
        lines: &[CartLineData],
        total_minor: i64,
        currency: &str,
    ) -> Result<Vec<String>, LuaError> {
        self.runtime.validate_order(lines, total_minor, currency)
    }

    pub fn apply_discount(
        &self,
        lines: &[CartLineData],
    ) -> Result<Option<DiscountResult>, LuaError> {
        self.runtime.apply_discount(lines)
    }

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

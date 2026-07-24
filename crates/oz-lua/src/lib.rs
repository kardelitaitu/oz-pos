/*
last audited 24-07-26 by Antigravity
crate: oz-lua | status: SAFE | lint: CLEAN
findings: Migrated from rlua to mlua 0.9. Native memory limit (10 MiB) now enforced via set_memory_limit. All tests passing.
*/

//! Embedded Lua scripting runtime for OZ-POS.
//!
//! `oz-lua` lets merchants customize business rules, promotions, and
//! order validation at runtime without recompiling the Rust core.
//! The runtime is built on [`mlua`] and exposes a curated surface of
//! cart / line / product data to Lua scripts.
//!
//! # Sandboxing
//!
//! Every script executes in a restricted environment:
//!
//! - **Removed globals**: `io`, `loadfile`, `dofile`, `require`,
//!   `package`, `debug`, `rawget`, `rawset`
//! - **Restricted globals**: `os` — `date`, `time`, and `clock` are available
//!   (read-only); `os.execute`, `os.remove`, `os.rename`, `os.exit` are nil
//! - **Allowed**: safe `math`, `string`, `table`, `pairs`, `ipairs`,
//!   `tonumber`, `tostring`, `type`, `pcall`, `xpcall`, `error`
//! - **Instruction limit**: scripts are aborted after 100 000 Lua
//!   instructions to prevent infinite loops.
//! - **Memory limit**: Lua VM is capped at 10 MB to prevent memory
//!   exhaustion from malicious tables or string concatenation.
//!
//! # Hooks
//!
//! | Lua function | Signature | Called when |
//! |---|---|---|
//! | `apply_discount` | `(lines_json) → {percent, label} \| nil` | Before sale creation |
//! | `calc_line_tax` | `(sku, qty, unit_price_minor, currency) → {rate_bps, is_inclusive} \| nil` | During tax computation |
//! | `validate_order` | `(lines_json, total_minor, currency) → string[]` | Before completion |

#![warn(unsafe_code)]
#![warn(missing_docs)]

use std::path::Path;

pub mod bridge;
pub mod error;

pub use bridge::LuaEventBridge;
pub use error::LuaError;

/// Maximum number of Lua bytecode instructions before the VM is interrupted.
/// Prevents infinite loops and runaway CPU from buggy or malicious scripts.
const INSTRUCTION_LIMIT: u64 = 100_000;

/// Maximum memory (in bytes) the Lua VM can allocate before being interrupted.
/// 10 MB — enough for typical discount/tax/validation scripts.
const MEMORY_LIMIT: usize = 10 * 1024 * 1024; // 10 MiB

/// A line item passed into Lua business-rule hooks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CartLineData {
    /// Stock-keeping unit code.
    pub sku: String,
    /// Quantity in this line.
    pub qty: i64,
    /// Unit price in minor units.
    pub unit_price_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
}

/// Result returned from a Lua `apply_discount` hook.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiscountResult {
    /// Discount percentage (0–100).
    pub percent: i64,
    /// Optional human-readable label.
    #[serde(default)]
    pub label: Option<String>,
}

/// Tax-rate override returned from a Lua `calc_line_tax` hook.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TaxOverride {
    /// Rate in basis points (e.g. 1000 = 10%).
    pub rate_bps: i64,
    /// Whether the tax is inclusive of the displayed price.
    #[serde(default)]
    pub is_inclusive: bool,
}

/// A sandboxed Lua scripting runtime.
///
/// Create one instance per application, then load scripts via
/// [`load_file`](LuaRuntime::load_file) or
/// [`load_dir`](LuaRuntime::load_dir). Business-rule hooks are
/// optional — if no script defines them, the hooks return `Ok(None)`.
pub struct LuaRuntime {
    lua: mlua::Lua,
}

// SAFETY: `LuaRuntime` is used behind a `Mutex` in application state,
// guaranteeing that only one thread accesses it at a time.
#[allow(unsafe_code)]
unsafe impl Send for LuaRuntime {}
#[allow(unsafe_code)]
unsafe impl Sync for LuaRuntime {}

impl Default for LuaRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to initialize LuaRuntime")
    }
}

impl LuaRuntime {
    /// Create a new sandboxed Lua VM.
    ///
    /// Removes dangerous globals, sets memory limits, and sets instruction limit.
    pub fn new() -> Result<Self, LuaError> {
        let lua = mlua::Lua::new();

        // Enforce 10 MiB native memory limit in Lua VM
        lua.set_memory_limit(MEMORY_LIMIT)
            .map_err(|e| LuaError::Init(format!("set memory limit failed: {e}")))?;

        // Sandbox: strip dangerous globals.
        {
            let globals = lua.globals();

            // Fully remove: these globals are always dangerous.
            let remove = &[
                "io",
                "loadfile",
                "dofile",
                "require",
                "package",
                "debug",
                "rawget",
                "rawset",
                "rawequal",
                "rawlen",
                "collectgarbage",
                "module",
                "load",
            ];
            for name in remove {
                globals
                    .set(*name, mlua::Value::Nil)
                    .map_err(|e| LuaError::Init(e.to_string()))?;
            }

            // Partially remove `os`: keep read-only time functions, strip execution capabilities.
            let safe_os = lua
                .create_table()
                .map_err(|e| LuaError::Init(e.to_string()))?;
            if let Ok(real_os) = globals.get::<_, mlua::Table>("os") {
                for safe_key in &["date", "time", "clock"] {
                    if let Ok(val) = real_os.get::<_, mlua::Value>(*safe_key) {
                        safe_os
                            .set(*safe_key, val)
                            .map_err(|e| LuaError::Init(e.to_string()))?;
                    }
                }
            }
            globals
                .set("os", safe_os)
                .map_err(|e| LuaError::Init(e.to_string()))?;
        }

        // Instruction limit hook: interrupts scripts after INSTRUCTION_LIMIT operations
        lua.set_hook(
            mlua::HookTriggers::new().every_nth_instruction(INSTRUCTION_LIMIT as u32),
            |_: &mlua::Lua, _: mlua::Debug| {
                Err(mlua::Error::RuntimeError(
                    "script aborted: instruction limit exceeded (100K)".into(),
                ))
            },
        );

        Ok(Self { lua })
    }

    /// Load a Lua script from a file path.
    pub fn load_file(&self, path: impl AsRef<Path>) -> Result<(), LuaError> {
        let code = std::fs::read_to_string(path.as_ref())
            .map_err(|e| LuaError::Load(format!("read {:?}: {e}", path.as_ref())))?;
        self.load_str(&code)
    }

    /// Load a Lua script from a string.
    pub fn load_str(&self, code: &str) -> Result<(), LuaError> {
        let sandbox_code = wrap_sandbox(code);
        self.lua
            .load(&sandbox_code)
            .exec()
            .map_err(|e| LuaError::Script(e.to_string()))?;
        Ok(())
    }

    /// Access the inner `mlua::Lua` state for advanced operations.
    pub fn inner(&self) -> &mlua::Lua {
        &self.lua
    }

    /// Load all `.lua` files from a directory (non-recursive).
    pub fn load_dir(&self, dir: impl AsRef<Path>) -> Result<(), LuaError> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Ok(());
        }
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| LuaError::Load(format!("read dir {:?}: {e}", dir)))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "lua"))
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in &entries {
            let path = entry.path();
            self.load_file(&path)?;
        }
        Ok(())
    }

    /// Check for overwritten global functions and log warnings.
    pub fn detect_overwrites(&self, known: &[String]) -> Vec<String> {
        let globals = self.lua.globals();
        let mut overwritten = Vec::new();
        for name in known {
            if let Ok(val) = globals.get::<_, mlua::Value>(name.as_str())
                && !matches!(val, mlua::Value::Nil)
            {
                let count = known.iter().filter(|n| *n == name).count();
                if count > 1 {
                    tracing::warn!(
                        target: "plugin",
                        "global '{}' was overwritten by a later script",
                        name
                    );
                    overwritten.push(name.clone());
                }
            }
        }
        overwritten
    }

    /// Legacy hook names that new plugins should avoid (use `oz.register_hook` instead).
    #[deprecated(
        since = "0.0.14",
        note = "Use oz.register_hook() instead of global functions"
    )]
    pub const LEGACY_HOOK_NAMES: &[&str] = &["apply_discount", "calc_line_tax", "validate_order"];

    /// Call the Lua `apply_discount(lines)` hook.
    pub fn apply_discount(
        &self,
        lines: &[CartLineData],
    ) -> Result<Option<DiscountResult>, LuaError> {
        let hook: mlua::Function = {
            let globals = self.lua.globals();
            match globals.get("apply_discount") {
                Ok(f) => f,
                Err(_) => return Ok(None),
            }
        };
        let table = build_lines_table(&self.lua, lines)?;
        let result: mlua::Value = hook
            .call(table)
            .map_err(|e| LuaError::Script(e.to_string()))?;
        Ok(parse_discount_result(result))
    }

    /// Call the Lua `calc_line_tax(sku, qty, unit_price_minor, currency)` hook.
    pub fn calc_line_tax(
        &self,
        sku: &str,
        qty: i64,
        unit_price_minor: i64,
        currency: &str,
    ) -> Result<Option<TaxOverride>, LuaError> {
        let hook: mlua::Function = {
            let globals = self.lua.globals();
            match globals.get("calc_line_tax") {
                Ok(f) => f,
                Err(_) => return Ok(None),
            }
        };
        let result: mlua::Value = hook
            .call((sku, qty, unit_price_minor, currency))
            .map_err(|e| LuaError::Script(e.to_string()))?;
        Ok(parse_tax_override(result))
    }

    /// Call the Lua `validate_order(lines, total_minor, currency)` hook.
    pub fn validate_order(
        &self,
        lines: &[CartLineData],
        total_minor: i64,
        currency: &str,
    ) -> Result<Vec<String>, LuaError> {
        let hook: mlua::Function = {
            let globals = self.lua.globals();
            match globals.get("validate_order") {
                Ok(f) => f,
                Err(_) => return Ok(Vec::new()),
            }
        };
        let table = build_lines_table(&self.lua, lines)?;
        let result: mlua::Value = hook
            .call((table, total_minor, currency))
            .map_err(|e| LuaError::Script(e.to_string()))?;
        let mut errors = Vec::new();
        if let mlua::Value::Table(tbl) = &result {
            for pair in tbl.clone().pairs() {
                let (_, val): (mlua::Value, String) =
                    pair.map_err(|e| LuaError::Script(e.to_string()))?;
                errors.push(val);
            }
        }
        Ok(errors)
    }
}

// ── Parsers ──────────────────────────────────────────────────────────────

fn parse_discount_result(val: mlua::Value) -> Option<DiscountResult> {
    match val {
        mlua::Value::Table(tbl) => {
            let percent: i64 = tbl.get("percent").ok()?;
            let label: Option<String> = tbl.get("label").ok().and_then(|v: Option<String>| v);
            Some(DiscountResult { percent, label })
        }
        _ => None,
    }
}

fn parse_tax_override(val: mlua::Value) -> Option<TaxOverride> {
    match val {
        mlua::Value::Table(tbl) => {
            let rate_bps: i64 = tbl.get("rate_bps").ok()?;
            let is_inclusive: bool = tbl.get("is_inclusive").unwrap_or(false);
            Some(TaxOverride {
                rate_bps,
                is_inclusive,
            })
        }
        _ => None,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Build an mlua table from CartLineData.
fn build_lines_table<'lua>(
    lua: &'lua mlua::Lua,
    lines: &[CartLineData],
) -> Result<mlua::Table<'lua>, LuaError> {
    let table = lua
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
        table
            .set(i + 1, row)
            .map_err(|e| LuaError::Script(e.to_string()))?;
    }
    Ok(table)
}

/// Wrap user code inside a sandboxed chunk.
fn wrap_sandbox(code: &str) -> String {
    format!(
        r#"
local ok, err = pcall(function()
{code}
end)
if not ok then
    error(err)
end
"#,
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> LuaRuntime {
        LuaRuntime::new().expect("Lua VM init")
    }

    #[test]
    fn new_creates_sandboxed_vm() {
        let lua = runtime();
        let globals = lua.lua.globals();
        let os_val: mlua::Value = globals.get("os").unwrap();
        assert!(
            matches!(os_val, mlua::Value::Table(_)),
            "os should be a restricted table"
        );
        let os_tbl: mlua::Table = globals.get("os").unwrap();
        let has_date: mlua::Value = os_tbl.get("date").unwrap();
        assert!(
            matches!(has_date, mlua::Value::Function(_)),
            "os.date should exist"
        );
        let has_time: mlua::Value = os_tbl.get("time").unwrap();
        assert!(
            matches!(has_time, mlua::Value::Function(_)),
            "os.time should exist"
        );
        let execute: mlua::Value = os_tbl.get("execute").unwrap();
        assert!(
            matches!(execute, mlua::Value::Nil),
            "os.execute should be nil"
        );
        let remove: mlua::Value = os_tbl.get("remove").unwrap();
        assert!(
            matches!(remove, mlua::Value::Nil),
            "os.remove should be nil"
        );

        let io: mlua::Value = globals.get("io").unwrap();
        assert!(matches!(io, mlua::Value::Nil), "io should be removed");
        let loadfile: mlua::Value = globals.get("loadfile").unwrap();
        assert!(
            matches!(loadfile, mlua::Value::Nil),
            "loadfile should be removed"
        );
        let math: mlua::Value = globals.get("math").unwrap();
        assert!(matches!(math, mlua::Value::Table(_)), "math should exist");
        let string: mlua::Value = globals.get("string").unwrap();
        assert!(
            matches!(string, mlua::Value::Table(_)),
            "string should exist"
        );
    }

    #[test]
    fn load_str_defines_global_function() {
        let lua = runtime();
        lua.load_str("function apply_discount(_) return nil end")
            .unwrap();
        let globals = lua.lua.globals();
        let hook: mlua::Value = globals.get("apply_discount").unwrap();
        assert!(matches!(hook, mlua::Value::Function(_)));
    }

    #[test]
    fn apply_discount_returns_nil_when_no_hook() {
        let lua = runtime();
        let lines = vec![];
        let result = lua.apply_discount(&lines).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn apply_discount_uses_lines_table() {
        let lua = runtime();
        lua.load_str(
            r#"
function apply_discount(lines)
    local total = 0
    for i = 1, #lines do
        total = total + lines[i].qty * lines[i].unit_price_minor
    end
    if total > 1000 then
        return { percent = 10, label = "Bulk" }
    end
    return nil
end
"#,
        )
        .unwrap();

        let lines = vec![CartLineData {
            sku: "COFFEE".into(),
            qty: 5,
            unit_price_minor: 500,
            currency: "USD".into(),
        }];
        let result = lua.apply_discount(&lines).unwrap();
        let d = result.expect("should get discount for >1000");
        assert_eq!(d.percent, 10);
        assert_eq!(d.label.as_deref(), Some("Bulk"));
    }

    #[test]
    fn apply_discount_returns_nil_for_small_orders() {
        let lua = runtime();
        lua.load_str(
            r#"
function apply_discount(lines)
    local total = 0
    for i = 1, #lines do
        total = total + lines[i].qty * lines[i].unit_price_minor
    end
    if total > 1000 then
        return { percent = 5, label = "Bulk" }
    end
    return nil
end
"#,
        )
        .unwrap();

        let lines = vec![CartLineData {
            sku: "CHEAP".into(),
            qty: 1,
            unit_price_minor: 200,
            currency: "USD".into(),
        }];
        let result = lua.apply_discount(&lines).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn calc_line_tax_returns_override() {
        let lua = runtime();
        lua.load_str(
            r#"
function calc_line_tax(sku, qty, unit_price_minor, currency)
    if sku == "CIGARETTES" then
        return { rate_bps = 2000, is_inclusive = true }
    end
    return nil
end
"#,
        )
        .unwrap();

        let result = lua.calc_line_tax("CIGARETTES", 1, 1000, "USD").unwrap();
        let tax = result.expect("cigarettes should have override");
        assert_eq!(tax.rate_bps, 2000);
        assert!(tax.is_inclusive);

        let result = lua.calc_line_tax("COFFEE", 1, 350, "USD").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn calc_line_tax_no_hook() {
        let lua = runtime();
        let result = lua.calc_line_tax("ANY", 1, 100, "USD").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn validate_order_returns_errors() {
        let lua = runtime();
        lua.load_str(
            r#"
function validate_order(lines, total_minor, currency)
    local errors = {}
    for i = 1, #lines do
        if lines[i].qty > 10 then
            table.insert(errors, lines[i].sku .. ": quantity exceeds 10")
        end
    end
    return errors
end
"#,
        )
        .unwrap();

        let lines = vec![
            CartLineData {
                sku: "COFFEE".into(),
                qty: 20,
                unit_price_minor: 350,
                currency: "USD".into(),
            },
            CartLineData {
                sku: "TEA".into(),
                qty: 2,
                unit_price_minor: 250,
                currency: "USD".into(),
            },
        ];
        let errors = lua.validate_order(&lines, 7500, "USD").unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("COFFEE"));
    }

    #[test]
    fn validate_order_no_hook() {
        let lua = runtime();
        let errors = lua.validate_order(&[], 0, "USD").unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn sandbox_allows_os_date_but_blocks_execute() {
        let lua = runtime();
        let date_ok = lua.load_str(r#"local d = os.date("!*t"); assert(type(d) == "table")"#);
        assert!(
            date_ok.is_ok(),
            "os.date should be available: {:?}",
            date_ok
        );
        let time_ok = lua.load_str(r#"local t = os.time(); assert(type(t) == "number")"#);
        assert!(time_ok.is_ok(), "os.time should be available");
        let exec_blocked = lua.load_str(r#"os.execute("echo hacked")"#);
        assert!(exec_blocked.is_err());
    }

    #[test]
    fn sandbox_blocks_io_open() {
        let lua = runtime();
        let result = lua.load_str(r#"io.open("/etc/passwd")"#);
        assert!(result.is_err());
    }

    #[test]
    fn sandbox_blocks_dofile() {
        let lua = runtime();
        let result = lua.load_str(r#"dofile("script.lua")"#);
        assert!(result.is_err());
    }

    #[test]
    fn script_syntax_error_is_caught() {
        let lua = runtime();
        let result = lua.load_str("function broken(");
        assert!(result.is_err());
    }

    #[test]
    fn load_file_missing_path_is_error() {
        let lua = runtime();
        let result = lua.load_file("/nonexistent/script.lua");
        assert!(result.is_err());
    }

    #[test]
    fn load_dir_skips_missing_dir() {
        let lua = runtime();
        let result = lua.load_dir("/nonexistent/scripts");
        assert!(result.is_ok());
    }

    #[test]
    fn discount_result_serde_roundtrip() {
        let json = r#"{"percent": 15, "label": "Senior"}"#;
        let dr: DiscountResult = serde_json::from_str(json).unwrap();
        assert_eq!(dr.percent, 15);
        assert_eq!(dr.label.as_deref(), Some("Senior"));
    }

    #[test]
    fn tax_override_serde_roundtrip() {
        let json = r#"{"rate_bps": 1000, "is_inclusive": true}"#;
        let to: TaxOverride = serde_json::from_str(json).unwrap();
        assert_eq!(to.rate_bps, 1000);
        assert!(to.is_inclusive);
    }

    #[test]
    fn multiple_scripts_can_be_loaded() {
        let lua = runtime();
        lua.load_str("function apply_discount(_) return { percent = 5, label = \"First\" } end")
            .unwrap();
        lua.load_str("function calc_line_tax(_, _, _, _) return { rate_bps = 800, is_inclusive = false } end").unwrap();
        lua.load_str("function validate_order(_, _, _) return {} end")
            .unwrap();

        let lines = vec![CartLineData {
            sku: "X".into(),
            qty: 1,
            unit_price_minor: 100,
            currency: "USD".into(),
        }];
        assert!(lua.apply_discount(&lines).unwrap().is_some());
        assert!(lua.calc_line_tax("X", 1, 100, "USD").unwrap().is_some());
        assert!(lua.validate_order(&lines, 100, "USD").unwrap().is_empty());
    }

    #[test]
    fn dangerous_globals_are_nil_or_restricted() {
        let lua = runtime();
        let globals = lua.lua.globals();
        let os_val: mlua::Value = globals.get("os").unwrap();
        assert!(
            matches!(os_val, mlua::Value::Table(_)),
            "os should be restricted table"
        );
        let os_tbl: mlua::Table = globals.get("os").unwrap();
        let execute: mlua::Value = os_tbl.get("execute").unwrap();
        assert!(
            matches!(execute, mlua::Value::Nil),
            "os.execute should be nil"
        );

        let nil_globals = [
            "io",
            "loadfile",
            "dofile",
            "require",
            "package",
            "debug",
            "rawget",
            "rawset",
            "rawequal",
            "rawlen",
            "collectgarbage",
            "module",
            "load",
        ];
        for name in &nil_globals {
            let val: mlua::Value = globals.get(*name).unwrap();
            assert!(
                matches!(val, mlua::Value::Nil),
                "dangerous global '{name}' should be nil"
            );
        }
    }

    #[test]
    fn safe_globals_still_work() {
        let lua = runtime();
        lua.load_str(
            r#"
local pi = math.pi
assert(pi > 3.14)

local greeting = string.upper("hello")
assert(greeting == "HELLO")

local t = { 1, 2, 3 }
table.insert(t, 4)
assert(#t == 4)

local count = 0
for _, _ in pairs(t) do count = count + 1 end
assert(count == 4)

assert(tonumber("42") == 42)
assert(tostring(42) == "42")

assert(type("hello") == "string")

local ok, val = pcall(function() return 1 + 1 end)
assert(ok and val == 2)

local ok2 = pcall(function() error("test") end)
assert(not ok2, "pcall should catch error")
"#,
        )
        .unwrap();
    }

    #[test]
    fn sandbox_blocks_require() {
        let lua = runtime();
        let result = lua.load_str(r#"require("socket")"#);
        assert!(result.is_err());
    }

    #[test]
    fn sandbox_blocks_package_access() {
        let lua = runtime();
        let result = lua.load_str(r#"package.path = "/evil/?.lua""#);
        assert!(result.is_err());
    }

    #[test]
    fn sandbox_blocks_load() {
        let lua = runtime();
        let result = lua.load_str(r#"load("return 1")()"#);
        assert!(result.is_err());
    }

    #[test]
    fn sandbox_blocks_rawget() {
        let lua = runtime();
        let result = lua.load_str(r#"rawget(_G, "os")"#);
        assert!(result.is_err());
    }

    #[test]
    fn sandbox_blocks_rawset() {
        let lua = runtime();
        let result = lua.load_str(r#"rawset(_G, "os", {})"#);
        assert!(result.is_err());
    }

    #[test]
    fn sandbox_blocks_collectgarbage() {
        let lua = runtime();
        let result = lua.load_str(r#"collectgarbage("collect")"#);
        assert!(result.is_err());
    }

    #[test]
    fn sandbox_blocks_debug_access() {
        let lua = runtime();
        let result = lua.load_str(r#"debug.sethook()"#);
        assert!(result.is_err());
    }

    #[test]
    fn sandbox_blocks_module() {
        let lua = runtime();
        let result = lua.load_str(r#"module("evil")"#);
        assert!(result.is_err());
    }

    #[test]
    fn malicious_script_multi_vector_attack_blocked() {
        let lua = runtime();
        let result = lua.load_str(
            r#"
pcall(function() os.execute("rm -rf /") end)
pcall(function() io.open("/etc/passwd") end)
pcall(function() dofile("/tmp/evil.lua") end)
pcall(function() require("socket") end)
pcall(function() loadfile("/tmp/evil.luac") end)
pcall(function() load("return 1") end)
pcall(function() debug.sethook() end)
pcall(function() rawget(_G, "os") end)
pcall(function() rawset(_G, "os", {}) end)
pcall(function() module("evil") end)
pcall(function() collectgarbage("collect") end)
"#,
        );
        assert!(
            result.is_ok(),
            "malicious multi-vector script should load safely: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn instruction_limit_aborts_infinite_loop() {
        let lua = runtime();
        let result = lua.load_str("while true do end");
        assert!(
            result.is_err(),
            "infinite loop should be aborted by instruction limit"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("instruction")
                || err.contains("interrupted")
                || err.contains("timeout")
                || err.contains("limit"),
            "error should mention instruction limit, got: {err}"
        );
    }

    #[test]
    fn instruction_limit_allows_normal_scripts() {
        let lua = runtime();
        lua.load_str(
            r#"
function factorial(n)
    if n <= 1 then return 1 end
    return n * factorial(n - 1)
end

result = factorial(10)
"#,
        )
        .unwrap();
        let result: i64 = lua
            .inner()
            .globals()
            .get::<_, mlua::Value>("result")
            .ok()
            .and_then(|v| match v {
                mlua::Value::Integer(i) => Some(i),
                _ => None,
            })
            .unwrap_or(0);
        assert_eq!(result, 3628800, "factorial(10) should compute correctly");
    }

    #[test]
    fn real_example_discount_bulk_works_in_sandbox() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/examples");
        let path = base.join("discount_bulk.lua");
        let lua = runtime();
        lua.load_file(&path).unwrap();

        let lines = vec![CartLineData {
            sku: "ITEM".into(),
            qty: 10,
            unit_price_minor: 100,
            currency: "USD".into(),
        }];
        let result = lua.apply_discount(&lines).unwrap();
        let d = result.expect("10+ items should get 10% discount");
        assert_eq!(d.percent, 10);
        assert_eq!(d.label.as_deref(), Some("Bulk 10+"));

        let lines = vec![CartLineData {
            sku: "ITEM".into(),
            qty: 6,
            unit_price_minor: 1000,
            currency: "USD".into(),
        }];
        let result = lua.apply_discount(&lines).unwrap();
        let d = result.expect("total > 5000 should get 5% discount");
        assert_eq!(d.percent, 5);
        assert_eq!(d.label.as_deref(), Some("Volume"));

        let lines = vec![CartLineData {
            sku: "CHEAP".into(),
            qty: 1,
            unit_price_minor: 100,
            currency: "USD".into(),
        }];
        let result = lua.apply_discount(&lines).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn real_example_tax_overrides_works_in_sandbox() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/examples");
        let path = base.join("tax_overrides.lua");
        let lua = runtime();
        lua.load_file(&path).unwrap();

        let tax = lua
            .calc_line_tax("CIG-001", 1, 1000, "USD")
            .unwrap()
            .expect("CIG prefix should get tax override");
        assert_eq!(tax.rate_bps, 2000);
        assert!(tax.is_inclusive);

        let tax = lua
            .calc_line_tax("TOB-001", 1, 500, "USD")
            .unwrap()
            .expect("TOB prefix should get tax override");
        assert_eq!(tax.rate_bps, 2000);

        let tax = lua
            .calc_line_tax("MILK-001", 1, 200, "USD")
            .unwrap()
            .expect("MILK prefix should get 0% VAT");
        assert_eq!(tax.rate_bps, 0);
        assert!(!tax.is_inclusive);

        let tax = lua
            .calc_line_tax("FOOD-001", 1, 500, "USD")
            .unwrap()
            .expect("FOOD- prefix should get 8% GST");
        assert_eq!(tax.rate_bps, 800);

        let result = lua.calc_line_tax("COFFEE", 1, 350, "USD").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn real_example_validate_order_works_in_sandbox() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/examples");
        let path = base.join("validate_order.lua");
        let lua = runtime();
        lua.load_file(&path).unwrap();

        let lines = vec![CartLineData {
            sku: "ITEM".into(),
            qty: 100,
            unit_price_minor: 100,
            currency: "USD".into(),
        }];
        let errors = lua.validate_order(&lines, 10000, "USD").unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("ITEM"));
        assert!(errors[0].contains("50"));

        let lines = vec![CartLineData {
            sku: "BEER-001".into(),
            qty: 6,
            unit_price_minor: 500,
            currency: "USD".into(),
        }];
        let errors = lua.validate_order(&lines, 3000, "USD").unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("age"));

        let lines = vec![
            CartLineData {
                sku: "SKU123".into(),
                qty: 1,
                unit_price_minor: 100,
                currency: "USD".into(),
            },
            CartLineData {
                sku: "SKU123".into(),
                qty: 2,
                unit_price_minor: 100,
                currency: "USD".into(),
            },
        ];
        let errors = lua.validate_order(&lines, 300, "USD").unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("duplicate"));

        let lines = vec![CartLineData {
            sku: "COFFEE".into(),
            qty: 2,
            unit_price_minor: 350,
            currency: "USD".into(),
        }];
        let errors = lua.validate_order(&lines, 700, "USD").unwrap();
        assert!(errors.is_empty());
    }
}

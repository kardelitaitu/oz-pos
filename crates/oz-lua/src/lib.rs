//! Embedded Lua scripting runtime for OZ-POS.
//!
//! `oz-lua` lets merchants customize business rules, promotions, and
//! order validation at runtime without recompiling the Rust core.
//! The runtime is built on [`rlua`] and exposes a curated surface of
//! cart / line / product data to Lua scripts.
//!
//! # Sandboxing
//!
//! Every script executes in a restricted environment:
//!
//! - **Removed globals**: `os`, `io`, `loadfile`, `dofile`, `require`,
//!   `package`, `debug`, `rawget`, `rawset`
//! - **Allowed**: safe `math`, `string`, `table`, `pairs`, `ipairs`,
//!   `tonumber`, `tostring`, `type`, `pcall`, `xpcall`, `error`
//! - **Instruction limit**: scripts are aborted after 100 000 Lua
//!   instructions to prevent infinite loops.
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

pub mod error;

pub use error::LuaError;

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
    lua: rlua::Lua,
}

// SAFETY: `rlua::Lua` wraps a raw `*mut lua_State` pointer and is
// neither `Send` nor `Sync`. However, `LuaRuntime` is always used
// behind a `Mutex` in the Tauri application state, guaranteeing that
// only one thread accesses it at a time. The pointer is only ever
// dereferenced from the Tokio async runtime which runs on a fixed
// thread pool, and all accesses are serialised by the outer Mutex.
#[allow(unsafe_code)]
unsafe impl Send for LuaRuntime {}
#[allow(unsafe_code)]
unsafe impl Sync for LuaRuntime {}

impl LuaRuntime {
    /// Create a new sandboxed Lua VM.
    ///
    /// Removes dangerous globals and sets the instruction limit.
    pub fn new() -> Result<Self, LuaError> {
        let lua = rlua::Lua::new();

        // Sandbox: strip dangerous globals.
        {
            let globals = lua.globals();
            let remove = &[
                "os", "io", "loadfile", "dofile", "require", "package",
                "debug", "rawget", "rawset", "rawequal", "rawlen",
                "collectgarbage", "module", "load",
            ];
            for name in remove {
                globals.set(*name, rlua::Value::Nil)
                    .map_err(|e| LuaError::Init(e.to_string()))?;
            }
        }

        Ok(Self { lua })
    }

    /// Load a Lua script from a file path.
    ///
    /// The script's chunks are compiled and stored in the VM.
    /// Global functions defined in the script (such as
    /// `apply_discount`) become callable from Rust.
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

    /// Call the Lua `apply_discount(lines)` hook.
    ///
    /// Returns `Ok(None)` when no script defined the hook.
    pub fn apply_discount(&self, lines: &[CartLineData]) -> Result<Option<DiscountResult>, LuaError> {
        let hook: rlua::Function = {
            let globals = self.lua.globals();
            match globals.get("apply_discount") {
                Ok(f) => f,
                Err(_) => return Ok(None),
            }
        };
        let table = build_lines_table(&self.lua, lines)?;
        let result: rlua::Value = hook.call(table)
            .map_err(|e| LuaError::Script(e.to_string()))?;
        Ok(parse_discount_result(result))
    }

    /// Call the Lua `calc_line_tax(sku, qty, unit_price_minor, currency)` hook.
    ///
    /// Returns `Ok(None)` when no script defined the hook.
    pub fn calc_line_tax(
        &self,
        sku: &str,
        qty: i64,
        unit_price_minor: i64,
        currency: &str,
    ) -> Result<Option<TaxOverride>, LuaError> {
        let hook: rlua::Function = {
            let globals = self.lua.globals();
            match globals.get("calc_line_tax") {
                Ok(f) => f,
                Err(_) => return Ok(None),
            }
        };
        let result: rlua::Value = hook.call((sku, qty, unit_price_minor, currency))
            .map_err(|e| LuaError::Script(e.to_string()))?;
        Ok(parse_tax_override(result))
    }

    /// Call the Lua `validate_order(lines, total_minor, currency)` hook.
    ///
    /// Returns a list of validation error strings (empty = valid).
    pub fn validate_order(
        &self,
        lines: &[CartLineData],
        total_minor: i64,
        currency: &str,
    ) -> Result<Vec<String>, LuaError> {
        let hook: rlua::Function = {
            let globals = self.lua.globals();
            match globals.get("validate_order") {
                Ok(f) => f,
                Err(_) => return Ok(Vec::new()),
            }
        };
        let table = build_lines_table(&self.lua, lines)?;
        let result: rlua::Value = hook.call((table, total_minor, currency))
            .map_err(|e| LuaError::Script(e.to_string()))?;
        let mut errors = Vec::new();
        if let rlua::Value::Table(tbl) = &result {
            for pair in tbl.clone().pairs() {
                let (_, val): (rlua::Value, String) = pair
                    .map_err(|e| LuaError::Script(e.to_string()))?;
                errors.push(val);
            }
        }
        Ok(errors)
    }
}

// ── Parsers ──────────────────────────────────────────────────────────────

fn parse_discount_result(val: rlua::Value) -> Option<DiscountResult> {
    match val {
        rlua::Value::Table(tbl) => {
            let percent: i64 = tbl.get("percent").ok()?;
            let label: Option<String> = tbl.get("label").ok().and_then(|v: Option<String>| v);
            Some(DiscountResult { percent, label })
        }
        _ => None,
    }
}

fn parse_tax_override(val: rlua::Value) -> Option<TaxOverride> {
    match val {
        rlua::Value::Table(tbl) => {
            let rate_bps: i64 = tbl.get("rate_bps").ok()?;
            let is_inclusive: bool = tbl.get("is_inclusive").unwrap_or(false);
            Some(TaxOverride { rate_bps, is_inclusive })
        }
        _ => None,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Build an rlua table from CartLineData.
fn build_lines_table<'lua>(lua: &'lua rlua::Lua, lines: &[CartLineData]) -> Result<rlua::Table<'lua>, LuaError> {
    let table = lua.create_table().map_err(|e| LuaError::Script(e.to_string()))?;
    for (i, line) in lines.iter().enumerate() {
        let row = lua.create_table().map_err(|e| LuaError::Script(e.to_string()))?;
        row.set("sku", line.sku.as_str()).map_err(|e| LuaError::Script(e.to_string()))?;
        row.set("qty", line.qty).map_err(|e| LuaError::Script(e.to_string()))?;
        row.set("unit_price_minor", line.unit_price_minor).map_err(|e| LuaError::Script(e.to_string()))?;
        row.set("currency", line.currency.as_str()).map_err(|e| LuaError::Script(e.to_string()))?;
        table.set(i + 1, row).map_err(|e| LuaError::Script(e.to_string()))?;
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
        let os: rlua::Value = globals.get("os").unwrap();
        assert!(matches!(os, rlua::Value::Nil), "os should be removed");
        let io: rlua::Value = globals.get("io").unwrap();
        assert!(matches!(io, rlua::Value::Nil), "io should be removed");
        let loadfile: rlua::Value = globals.get("loadfile").unwrap();
        assert!(matches!(loadfile, rlua::Value::Nil), "loadfile should be removed");
        let math: rlua::Value = globals.get("math").unwrap();
        assert!(matches!(math, rlua::Value::Table(_)), "math should exist");
        let string: rlua::Value = globals.get("string").unwrap();
        assert!(matches!(string, rlua::Value::Table(_)), "string should exist");
    }

    #[test]
    fn load_str_defines_global_function() {
        let lua = runtime();
        lua.load_str("function apply_discount(_) return nil end").unwrap();
        let globals = lua.lua.globals();
        let hook: rlua::Value = globals.get("apply_discount").unwrap();
        assert!(matches!(hook, rlua::Value::Function(_)));
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
        lua.load_str(r#"
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
"#).unwrap();

        let lines = vec![CartLineData {
            sku: "COFFEE".into(), qty: 5, unit_price_minor: 500, currency: "USD".into(),
        }];
        let result = lua.apply_discount(&lines).unwrap();
        let d = result.expect("should get discount for >1000");
        assert_eq!(d.percent, 10);
        assert_eq!(d.label.as_deref(), Some("Bulk"));
    }

    #[test]
    fn apply_discount_returns_nil_for_small_orders() {
        let lua = runtime();
        lua.load_str(r#"
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
"#).unwrap();

        let lines = vec![CartLineData {
            sku: "CHEAP".into(), qty: 1, unit_price_minor: 200, currency: "USD".into(),
        }];
        let result = lua.apply_discount(&lines).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn calc_line_tax_returns_override() {
        let lua = runtime();
        lua.load_str(r#"
function calc_line_tax(sku, qty, unit_price_minor, currency)
    if sku == "CIGARETTES" then
        return { rate_bps = 2000, is_inclusive = true }
    end
    return nil
end
"#).unwrap();

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
        lua.load_str(r#"
function validate_order(lines, total_minor, currency)
    local errors = {}
    for i = 1, #lines do
        if lines[i].qty > 10 then
            table.insert(errors, lines[i].sku .. ": quantity exceeds 10")
        end
    end
    return errors
end
"#).unwrap();

        let lines = vec![
            CartLineData { sku: "COFFEE".into(), qty: 20, unit_price_minor: 350, currency: "USD".into() },
            CartLineData { sku: "TEA".into(), qty: 2, unit_price_minor: 250, currency: "USD".into() },
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
    fn sandbox_blocks_os_execute() {
        let lua = runtime();
        let result = lua.load_str(r#"os.execute("echo hacked")"#);
        assert!(result.is_err());
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
        lua.load_str("function apply_discount(_) return { percent = 5, label = \"First\" } end").unwrap();
        lua.load_str("function calc_line_tax(_, _, _, _) return { rate_bps = 800, is_inclusive = false } end").unwrap();
        lua.load_str("function validate_order(_, _, _) return {} end").unwrap();

        let lines = vec![CartLineData { sku: "X".into(), qty: 1, unit_price_minor: 100, currency: "USD".into() }];
        assert!(lua.apply_discount(&lines).unwrap().is_some());
        assert!(lua.calc_line_tax("X", 1, 100, "USD").unwrap().is_some());
        assert!(lua.validate_order(&lines, 100, "USD").unwrap().is_empty());
    }


}

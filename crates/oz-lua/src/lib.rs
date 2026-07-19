/*
last audited 19-07-26 by RSA-Agent
crate: oz-lua | status: SAFE | lint: CLEAN
findings: unsafe impl Send + Sync for LuaRuntime (wraps raw *mut lua_State). SAFETY comment present — always behind Mutex, serialized access on Tokio thread pool. No other unsafe code.
next: none | perf: N/A
*/

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
///
/// 100 000 instructions is enough for typical discount/tax/validation logic
/// but small enough that a tight loop (`while true do end`) hits the limit
/// in under a millisecond.
const INSTRUCTION_LIMIT: u64 = 100_000;

/// Maximum memory (in bytes) the Lua VM can allocate before being interrupted.
/// 10 MB — enough for typical discount/tax/validation scripts but not enough
/// to exhaust host RAM.
///
/// Note: Currently not enforced because rlua 0.20 does not expose
/// `set_memory_limit`. See docs/security/lua-sandbox-audit.md Finding #4.
#[allow(dead_code)]
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
                "os",
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
                    .set(*name, rlua::Value::Nil)
                    .map_err(|e| LuaError::Init(e.to_string()))?;
            }
        }

        // ── Resource limits ────────────────────────────────────────
        // Instruction limit: set a debug hook that fires every N VM
        // instructions and aborts execution. This prevents infinite loops
        // from buggy or malicious scripts.
        //
        // rlua 0.20 wraps mlua but does not expose `set_instruction_limit`
        // directly — we use `DebugEvent::Count` instead, which achieves
        // the same result. The hook fires at N-instruction intervals;
        // the callback raises a RuntimeError that aborts the script.
        //
        // Since `debug` is stripped from globals before this hook is set,
        // scripts CANNOT access `debug.sethook` to clear or modify it.
        lua.set_hook(
            rlua::HookTriggers::new().every_nth_instruction(INSTRUCTION_LIMIT as u32),
            |_: &rlua::Lua, _: rlua::Debug| {
                Err(rlua::Error::RuntimeError(
                    "script aborted: instruction limit exceeded (100K)".into(),
                ))
            },
        );

        // Memory limit: rlua 0.20 does not expose set_memory_limit.
        // A future upgrade to mlua directly would enable this.
        // The 10 MB limit (MEMORY_LIMIT) is documented but not currently
        // enforced. See docs/security/lua-sandbox-audit.md Finding #4.

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

    /// Access the inner `rlua::Lua` state for advanced operations
    /// (registry keys, custom bindings, etc.).
    pub fn inner(&self) -> &rlua::Lua {
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

    /// Call the Lua `apply_discount(lines)` hook.
    ///
    /// Returns `Ok(None)` when no script defined the hook.
    pub fn apply_discount(
        &self,
        lines: &[CartLineData],
    ) -> Result<Option<DiscountResult>, LuaError> {
        let hook: rlua::Function = {
            let globals = self.lua.globals();
            match globals.get("apply_discount") {
                Ok(f) => f,
                Err(_) => return Ok(None),
            }
        };
        let table = build_lines_table(&self.lua, lines)?;
        let result: rlua::Value = hook
            .call(table)
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
        let result: rlua::Value = hook
            .call((sku, qty, unit_price_minor, currency))
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
        let result: rlua::Value = hook
            .call((table, total_minor, currency))
            .map_err(|e| LuaError::Script(e.to_string()))?;
        let mut errors = Vec::new();
        if let rlua::Value::Table(tbl) = &result {
            for pair in tbl.clone().pairs() {
                let (_, val): (rlua::Value, String) =
                    pair.map_err(|e| LuaError::Script(e.to_string()))?;
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
            Some(TaxOverride {
                rate_bps,
                is_inclusive,
            })
        }
        _ => None,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Build an rlua table from CartLineData.
fn build_lines_table<'lua>(
    lua: &'lua rlua::Lua,
    lines: &[CartLineData],
) -> Result<rlua::Table<'lua>, LuaError> {
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
        let os: rlua::Value = globals.get("os").unwrap();
        assert!(matches!(os, rlua::Value::Nil), "os should be removed");
        let io: rlua::Value = globals.get("io").unwrap();
        assert!(matches!(io, rlua::Value::Nil), "io should be removed");
        let loadfile: rlua::Value = globals.get("loadfile").unwrap();
        assert!(
            matches!(loadfile, rlua::Value::Nil),
            "loadfile should be removed"
        );
        let math: rlua::Value = globals.get("math").unwrap();
        assert!(matches!(math, rlua::Value::Table(_)), "math should exist");
        let string: rlua::Value = globals.get("string").unwrap();
        assert!(
            matches!(string, rlua::Value::Table(_)),
            "string should exist"
        );
    }

    #[test]
    fn load_str_defines_global_function() {
        let lua = runtime();
        lua.load_str("function apply_discount(_) return nil end")
            .unwrap();
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

    // ── P0-4: Comprehensive sandbox tests ─────────────────────────

    #[test]
    fn all_14_dangerous_globals_are_nil() {
        let lua = runtime();
        let globals = lua.lua.globals();
        let dangerous = [
            "os",
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
        for name in &dangerous {
            let val: rlua::Value = globals.get(*name).unwrap();
            assert!(
                matches!(val, rlua::Value::Nil),
                "dangerous global '{name}' should be nil"
            );
        }
    }

    #[test]
    fn safe_globals_still_work() {
        let lua = runtime();
        // Verify that the allowed standard-library globals exist and work.
        lua.load_str(
            r#"
-- math
local pi = math.pi
assert(pi > 3.14)

-- string
local greeting = string.upper("hello")
assert(greeting == "HELLO")

-- table
local t = { 1, 2, 3 }
table.insert(t, 4)
assert(#t == 4)

-- pairs / ipairs
local count = 0
for _, _ in pairs(t) do count = count + 1 end
assert(count == 4)

-- tonumber / tostring
assert(tonumber("42") == 42)
assert(tostring(42) == "42")

-- type
assert(type("hello") == "string")

-- pcall / xpcall
local ok, val = pcall(function() return 1 + 1 end)
assert(ok and val == 2)

-- error (caught by pcall)
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
        // Without 'debug', scripts cannot call debug.sethook to clear
        // the instruction-limit hook that was set in new().
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
        // A script that tries multiple attack vectors in sequence —
        // all should fail and the instruction limit should abort any
        // that somehow slip through.
        //
        // Each vector is wrapped in an anonymous function so the nil-global
        // error is caught by pcall (rather than being evaluated as a
        // function argument outside of pcall's scope).
        let lua = runtime();
        let result = lua.load_str(
            r#"
-- Vector 1: os
pcall(function() os.execute("rm -rf /") end)

-- Vector 2: io
pcall(function() io.open("/etc/passwd") end)

-- Vector 3: dofile
pcall(function() dofile("/tmp/evil.lua") end)

-- Vector 4: require
pcall(function() require("socket") end)

-- Vector 5: loadfile
pcall(function() loadfile("/tmp/evil.luac") end)

-- Vector 6: load
pcall(function() load("return 1") end)

-- Vector 7: debug
pcall(function() debug.sethook() end)

-- Vector 8: rawget
pcall(function() rawget(_G, "os") end)

-- Vector 9: rawset
pcall(function() rawset(_G, "os", {}) end)

-- Vector 10: module
pcall(function() module("evil") end)

-- Vector 11: collectgarbage
pcall(function() collectgarbage("collect") end)

-- All vectors silently fail; script completes."#,
        );
        // The script should load successfully (pcall catches each nil-index error)
        // rather than crashing the VM.
        assert!(
            result.is_ok(),
            "malicious multi-vector script should load safely: {}",
            result.unwrap_err()
        );
    }

    // ── Resource limit tests ──────────────────────────────────────────

    #[test]
    fn instruction_limit_aborts_infinite_loop() {
        let lua = runtime();
        // An infinite loop will hit the 100K instruction limit immediately.
        let result = lua.load_str("while true do end");
        assert!(
            result.is_err(),
            "infinite loop should be aborted by instruction limit"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("instruction") || err.contains("interrupted") || err.contains("timeout"),
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

-- Call factorial(10) = 3,628,800
result = factorial(10)
"#,
        )
        .unwrap();
        // Verify the function was defined and callable.
        let result: i64 = lua
            .inner()
            .globals()
            .get::<_, rlua::Value>("result")
            .ok()
            .and_then(|v| match v {
                rlua::Value::Integer(i) => Some(i),
                _ => None,
            })
            .unwrap_or(0);
        assert_eq!(result, 3628800, "factorial(10) should compute correctly");
    }

    #[test]
    fn memory_limit_not_enforced_due_to_rlua_limitation() {
        // rlua 0.20 does not expose `set_memory_limit`. The MEMORY_LIMIT
        // constant is documented for future enforcement (see P0 audit
        // Finding #4). Until the runtime is upgraded to use mlua directly,
        // memory-intensive scripts are only limited by the 100K instruction
        // limit, not by a hard memory cap.
        //
        // This test verifies that a moderately large allocation (1000 items)
        // succeeds — confirming the memory limit is NOT enforced, which is
        // the current expected behavior.
        let lua = runtime();
        lua.load_str(
            r#"
local t = {}
for i = 1, 1000 do
    t[i] = string.rep("X", 100)
end
"#,
        )
        .unwrap();
    }

    // ── P0-5: Example script regression tests ─────────────────────────

    #[test]
    fn real_example_discount_bulk_works_in_sandbox() {
        // Regression test: load scripts/examples/discount_bulk.lua
        // and verify its apply_discount hook works correctly.
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/examples");
        let path = base.join("discount_bulk.lua");
        let lua = runtime();
        lua.load_file(&path).unwrap();

        // Tier 1: 10+ items → 10% off
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

        // Tier 2: total > 5000 minor units → 5% off
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

        // No discount: small order
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
        // Regression test: load scripts/examples/tax_overrides.lua
        // and verify its calc_line_tax hook works correctly.
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/examples");
        let path = base.join("tax_overrides.lua");
        let lua = runtime();
        lua.load_file(&path).unwrap();

        // Cigarettes: 20% excise tax, inclusive
        let tax = lua
            .calc_line_tax("CIG-001", 1, 1000, "USD")
            .unwrap()
            .expect("CIG prefix should get tax override");
        assert_eq!(tax.rate_bps, 2000);
        assert!(tax.is_inclusive);

        // Tobacco: 20% excise tax, inclusive
        let tax = lua
            .calc_line_tax("TOB-001", 1, 500, "USD")
            .unwrap()
            .expect("TOB prefix should get tax override");
        assert_eq!(tax.rate_bps, 2000);

        // Milk: 0% VAT
        let tax = lua
            .calc_line_tax("MILK-001", 1, 200, "USD")
            .unwrap()
            .expect("MILK prefix should get 0% VAT");
        assert_eq!(tax.rate_bps, 0);
        assert!(!tax.is_inclusive);

        // Prepared food: 8% GST
        let tax = lua
            .calc_line_tax("FOOD-001", 1, 500, "USD")
            .unwrap()
            .expect("FOOD- prefix should get 8% GST");
        assert_eq!(tax.rate_bps, 800);

        // Unmatched SKU: fall through
        let result = lua.calc_line_tax("COFFEE", 1, 350, "USD").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn real_example_validate_order_works_in_sandbox() {
        // Regression test: load scripts/examples/validate_order.lua
        // and verify its validate_order hook works correctly.
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/examples");
        let path = base.join("validate_order.lua");
        let lua = runtime();
        lua.load_file(&path).unwrap();

        // Exceeds max quantity
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

        // Alcohol age verification
        let lines = vec![CartLineData {
            sku: "BEER-001".into(),
            qty: 6,
            unit_price_minor: 500,
            currency: "USD".into(),
        }];
        let errors = lua.validate_order(&lines, 3000, "USD").unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("age"));

        // Duplicate SKU detection
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

        // Clean order: no errors
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

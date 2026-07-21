//! Fuzz target for Lua script sandbox — feeds arbitrary byte sequences
//! to `LuaRuntime::load_str()` and verifies the sandbox contains all
//! attacks without panicking or crashing the process.
//!
//! The sandbox strips dangerous globals (os, io, loadfile, etc.) and
//! sets an instruction limit of 100K VM ops. This fuzz target ensures
//! no combination of bytes can bypass the sandbox or cause a panic.
//
// # Safety
//
// This fuzz target is `no_main` and compiled only with cargo-fuzz.
// It does not use `unsafe` directly.
//
// The rlua library internally wraps a raw `*mut lua_State`, but the
// `LuaRuntime` struct in oz-lua is always behind a Mutex in production.
// In fuzz testing there is no concurrency, and rlua's internal locking
// is the only synchronization concern.
//
// The instruction-limit hook uses rlua's `DebugEvent::Count` which is
// safe to set from any thread. No other thread safety considerations
// apply to single-threaded fuzzing.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only fuzz valid UTF-8 strings (Lua source is text).
    if let Ok(s) = std::str::from_utf8(data) {
        // Create a fresh sandboxed Lua VM per input.
        let lua = match oz_lua::LuaRuntime::new() {
            Ok(lua) => lua,
            Err(_) => return, // VM init failure is not a bug we're testing for
        };

        // Load the arbitrary string as Lua code. The sandbox must:
        // - Not panic/crash on any UTF-8 input
        // - Block dangerous globals (os, io, loadfile, etc.)
        // - Abort infinite loops via 100K instruction limit
        let _ = lua.load_str(s);

        // Test that the sandbox is still intact after loading potentially
        // malicious code — dangerous globals must remain nil.
        // Only check if input is short enough to have loaded successfully
        // and not exceeded the instruction limit.
        if s.len() < 500 {
            let globals = lua.inner().globals();
            let dangerous = ["os", "io", "loadfile", "dofile", "require",
                             "package", "debug", "rawget", "rawset",
                             "rawequal", "rawlen", "collectgarbage", "module", "load"];
            for name in &dangerous {
                let val: rlua::Value = match globals.get(*name) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                assert!(
                    matches!(val, rlua::Value::Nil),
                    "dangerous global '{name}' should be nil after malicious input"
                );
            }
        }

        // Also fuzz the apply_discount hook with the current sandbox state.
        // Even if load_str failed, the VM should be in a recoverable state.
        let lines = oz_lua::CartLineData {
            sku: s.chars().take(50).collect(),
            qty: 1,
            unit_price_minor: 100,
            currency: "USD".to_string(),
        };
        let _ = lua.apply_discount(&[lines]);
    }

    // Non-UTF-8 bytes should be handled by `std::str::from_utf8`
    // returning Err — we simply skip them. No crash should occur.
});

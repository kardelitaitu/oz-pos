# oz-lua

Embedded Lua scripting runtime for OZ-POS. Lets merchants customize business rules, promotions, and UI layouts at runtime without recompiling the Rust core.

## Public API

- [`LuaError`](src/error.rs) — `thiserror`-based error for the runtime and script evaluation.

## Planned surface

- A curated `rlua`/`mlua`-backed VM that exposes a safe subset of `oz-core` types.
- Script discovery from `lua_scripts/` directories.
- Per-script resource limits and timeout enforcement.
- A migration path from inline Lua to first-class Rust features.

## Status

Scaffold only. The binding surface lands in a follow-up once `oz-core` and the cart state machine are stable.

See the `rust-backend` skill for the Rust-side error-handling conventions.

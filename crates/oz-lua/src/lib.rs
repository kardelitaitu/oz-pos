//! Embedded Lua scripting runtime for OZ-POS.
//!
//! `oz-lua` lets merchants customize business rules, promotions, and
//! UI layouts at runtime without recompiling the Rust core. The runtime
//! is built on `rlua` (or its successor `mlua`) and exposes a curated
//! surface of `oz-core` types to Lua scripts.
//!
//! This crate is currently a scaffold — the binding surface will be
//! added in a follow-up. The error type and module structure are in
//! place so the `rust-backend` skill's conventions apply.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;

pub use error::LuaError;

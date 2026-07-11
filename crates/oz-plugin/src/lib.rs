#![warn(missing_docs)]

//! Plugin discovery, loading, and sandboxing.
//!
//! Plugins are Lua scripts packaged with a `plugin.toml` manifest.
//! They are loaded from the `plugins/` directory at startup and
//! given access to a sandboxed Lua environment.

/// Database types for plugin persistence.
pub mod db;
/// Plugin error types.
pub mod error;
/// Plugin loading and scanning.
pub mod loader;
/// Runtime plugin manager with Lua sandbox.
pub mod manager;
/// Plugin manifest (`plugin.toml`) deserialization.
pub mod manifest;
/// Plugin package format (.ozp) handling.
pub mod package;

pub use error::PluginError;
pub use loader::{LoadedPlugin, PluginRegistry, load_plugins};
pub use manager::PluginManager;
pub use manifest::PluginManifest;

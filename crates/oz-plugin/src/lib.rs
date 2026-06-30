//! Plugin discovery, loading, and sandboxing.
//!
//! Plugins are Lua scripts packaged with a `plugin.toml` manifest.
//! They are loaded from the `plugins/` directory at startup and
//! given access to a sandboxed Lua environment.

pub mod error;
pub mod loader;
pub mod manifest;

pub use error::PluginError;
pub use loader::{LoadedPlugin, PluginRegistry, load_plugins};
pub use manifest::PluginManifest;

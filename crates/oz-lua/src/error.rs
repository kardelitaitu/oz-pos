//! Error type for the `oz-lua` runtime.

use thiserror::Error;

/// Errors that can originate in the Lua runtime or script evaluation.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LuaError {
    /// The Lua VM failed to start.
    #[error("lua runtime init failed: {0}")]
    Init(String),

    /// A script raised a Lua-level error.
    #[error("lua script error: {0}")]
    Script(String),

    /// A script tried to call a binding that is not exposed.
    #[error("unknown binding: {0}")]
    UnknownBinding(String),

    /// Failed to load a script from disk.
    #[error("lua script load failed: {0}")]
    Load(String),
}

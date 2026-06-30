use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("plugin manifest error: {0}")]
    Manifest(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Lua error: {0}")]
    Lua(String),
    #[error("plugin not found: {0}")]
    NotFound(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

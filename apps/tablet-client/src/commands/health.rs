//! Health-check commands used by the front-end's startup smoke test and
//! the About dialog. No state required.

use serde::Serialize;
use tauri::command;

use crate::error::AppError;

/// Liveness probe. Returns `Ok("pong")` if the Tauri runtime is alive.
#[command]
pub async fn ping() -> Result<String, AppError> {
    Ok("pong".into())
}

/// Build/version information for the About dialog.
#[derive(Debug, Serialize)]
pub struct VersionInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub rust_version: &'static str,
    pub target: &'static str,
}

#[command]
pub async fn version() -> Result<VersionInfo, AppError> {
    Ok(VersionInfo {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        rust_version: env!("CARGO_PKG_RUST_VERSION"),
        target: option_env!("TARGET").unwrap_or("unknown"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ping_returns_pong() {
        assert_eq!(ping().await.unwrap(), "pong");
    }

    #[tokio::test]
    async fn version_has_populated_fields() {
        let v = version().await.unwrap();
        assert!(!v.name.is_empty());
        assert!(!v.version.is_empty());
        assert!(!v.target.is_empty());
    }
}

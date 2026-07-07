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

    // ── VersionInfo struct tests ─────────────────────────────────────

    #[test]
    fn version_info_debug() {
        let v = VersionInfo {
            name: "test-app",
            version: "1.0.0",
            rust_version: "1.80",
            target: "x86_64-linux",
        };
        let debug = format!("{v:?}");
        assert!(debug.contains("test-app"));
        assert!(debug.contains("1.0.0"));
        assert!(debug.contains("x86_64-linux"));
    }

    #[test]
    fn version_info_serde_json() {
        let v = VersionInfo {
            name: "test-app",
            version: "1.0.0",
            rust_version: "1.80",
            target: "x86_64-linux",
        };
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["name"], "test-app");
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["target"], "x86_64-linux");
    }

    #[test]
    fn version_info_field_access() {
        let v = VersionInfo {
            name: "oz-pos-app",
            version: "0.0.3",
            rust_version: "1.80",
            target: "wasm32",
        };
        assert_eq!(v.name, "oz-pos-app");
        assert_eq!(v.version, "0.0.3");
        assert_eq!(v.rust_version, "1.80");
        assert_eq!(v.target, "wasm32");
    }
}

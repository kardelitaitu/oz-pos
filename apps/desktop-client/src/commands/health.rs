//! Health-check commands used by the front-end's startup smoke test and
//! the About dialog. No state required.

use serde::Serialize;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

/// Liveness probe. Returns `Ok("pong")` if the Tauri runtime is alive.
#[tauri::command]
pub async fn ping() -> Result<String, AppError> {
    Ok("pong".into())
}

/// Build/version information for the About dialog.
#[derive(Debug, Serialize)]
pub struct VersionInfo {
    /// Display name.
    pub name: &'static str,
    /// Version.
    pub version: &'static str,
    /// Rust Version.
    pub rust_version: &'static str,
    /// Target.
    pub target: &'static str,
}

#[tauri::command]
/// Version.
pub async fn version() -> Result<VersionInfo, AppError> {
    Ok(VersionInfo {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        rust_version: env!("CARGO_PKG_RUST_VERSION"),
        target: option_env!("TARGET").unwrap_or("unknown"),
    })
}

/// Version info resolved from a session token. ADR #7.
/// Validates the session token and returns the same compile-time version info.
#[tauri::command]
pub async fn version_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<VersionInfo, AppError> {
    let _session = state.resolve_session(&session_token)?;
    Ok(VersionInfo {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        rust_version: env!("CARGO_PKG_RUST_VERSION"),
        target: option_env!("TARGET").unwrap_or("unknown"),
    })
}

/// Get the stable device identifier (hostname) for terminal binding.
///
/// Reads `COMPUTERNAME` on Windows, `HOSTNAME` on Unix, or falls back
/// to `"unknown-device"`. This is used by WorkspaceContext to populate
/// the `terminal_id` field when creating session tokens (ADR #7).
#[tauri::command]
pub async fn get_device_id() -> Result<String, AppError> {
    Ok(std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-device".to_string()))
}

/// Get the local IP address of the machine.
#[tauri::command]
pub async fn get_local_ip() -> Result<String, AppError> {
    use std::net::UdpSocket;
    // A trick to get the local IP address without making actual network requests.
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return Ok("127.0.0.1".into()),
    };
    if let Ok(()) = socket.connect("8.8.8.8:80")
        && let Ok(local_addr) = socket.local_addr()
    {
        return Ok(local_addr.ip().to_string());
    }
    Ok("127.0.0.1".into())
}

/// Session-scoped variant of [`ping`].
#[tauri::command]
pub async fn ping_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let _session = state.resolve_session(&session_token)?;
    ping().await
}

/// Session-scoped variant of [`get_device_id`].
#[tauri::command]
pub async fn get_device_id_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let _session = state.resolve_session(&session_token)?;
    get_device_id().await
}

/// Session-scoped variant of [`get_local_ip`].
#[tauri::command]
pub async fn get_local_ip_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let _session = state.resolve_session(&session_token)?;
    get_local_ip().await
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod tests;

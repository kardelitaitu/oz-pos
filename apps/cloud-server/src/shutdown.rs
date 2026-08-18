//! Graceful shutdown signal for the cloud server.
//!
//! Provides [`shutdown_signal`] which resolves when the process receives
//! SIGTERM (Unix) or Ctrl+C (all platforms). The returned future can be
//! passed directly to `axum::serve(...).with_graceful_shutdown(...)`.

use tracing::{info, warn};

/// Wait for a shutdown signal (SIGTERM or Ctrl+C), then log and return.
///
/// On Unix, listens for both SIGTERM (Docker/K8s stop) and SIGINT (Ctrl+C).
/// On Windows, listens for Ctrl+C only (SIGTERM is not a first-class signal).
///
/// # Example
///
/// ```no_run
/// let app = Router::new();
/// let listener = tokio::net::TcpListener::bind("0.0.0.0:3099").await.unwrap();
/// axum::serve(listener, app)
///     .with_graceful_shutdown(shutdown_signal())
///     .await
///     .unwrap();
/// ```
pub async fn shutdown_signal() {
    // RUST-07: signal-handler installation is a recoverable OS interaction.
    // On failure we log a warning and fall back to a never-resolving future so
    // the process keeps running (and can still be terminated externally)
    // instead of panicking during startup.
    let ctrl_c = async {
        match tokio::signal::ctrl_c().await {
            Ok(()) => info!("received Ctrl+C, starting graceful shutdown"),
            Err(e) => {
                warn!(error = %e, "failed to install Ctrl+C handler; graceful shutdown via Ctrl+C unavailable");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
                info!("received SIGTERM, starting graceful shutdown");
            }
            Err(e) => {
                warn!(error = %e, "failed to install SIGTERM handler; graceful shutdown via SIGTERM unavailable");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

#[cfg(test)] #[path = "shutdown_tests.rs"] mod tests;

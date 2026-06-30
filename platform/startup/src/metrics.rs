//! Optional Prometheus metrics HTTP endpoint.
//!
//! When the `metrics` feature is enabled, starts a lightweight
//! HTTP server that exposes `/metrics` for Prometheus scraping.

#[cfg(feature = "metrics")]
pub mod server {
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    /// Start a minimal HTTP server serving Prometheus metrics on `addr`.
    ///
    /// Returns a join handle — the server runs until the application shuts down.
    pub async fn start_metrics_server(
        addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(addr).await?;
        tracing::info!(addr = %addr, "Prometheus metrics endpoint started");

        loop {
            let (mut stream, peer) = listener.accept().await?;
            tracing::trace!(peer = %peer, "Metrics scrape connection");

            let metrics_body = oz_reporting::gather_metrics();

            // Minimal HTTP response
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                metrics_body.len(),
                metrics_body
            );

            use tokio::io::AsyncWriteExt;
            if let Err(e) = stream.write_all(response.as_bytes()).await {
                tracing::warn!(error = %e, "Failed to write metrics response");
            }
        }
    }
}

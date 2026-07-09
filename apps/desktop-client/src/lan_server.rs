//! LAN event forwarder — a lightweight TCP server that broadcasts domain
//! events to KDS tablet peers on the local network.
//!
//! The server listens on port 9180 for TCP connections from KDS tablets
//! (or any LAN peer). When a domain event is published locally via the
//! event bus (`sale.completed` or `order.course_fired`), it is serialised
//! to JSON and written as a single line (newline-delimited) to every
//! connected peer.
//!
//! # Wire format
//!
//! Each forwarded event is a single line of JSON terminated by `\n`.
//! The event type can be inferred from the field names:
//!
//! - `sale.completed`: contains `sale_id`, `line_items`, `total_minor`, `currency`
//! - `order.course_fired`: contains `sale_id`, `course_id`, `display_number`, `items`
//!
//! # Example
//!
//! ```ignore
//! use crate::lan_server::LanEventForwarder;
//!
//! let forwarder = LanEventForwarder::new();
//! tauri::async_runtime::spawn(forwarder.clone().run());
//!
//! // Subscribe event bus handlers
//! {
//!     let handle = forwarder.handle();
//!     bus.subscribe("sale.completed", Box::new(handle.sale_completed_handler()));
//!     bus.subscribe("order.course_fired", Box::new(handle.course_fired_handler()));
//! }
//! ```

use foundation::contracts::{EventHandler, ModuleResult};
use oz_core::events::{CourseFired, SaleCompleted};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

/// TCP port for the LAN event forwarding server.
const LAN_PORT: u16 = 9180;

/// Maximum number of pending broadcast messages before old ones are
/// dropped (avoids unbounded memory growth for slow peers).
const CHANNEL_CAPACITY: usize = 256;

// ── LanEventForwarder ────────────────────────────────────────────────

/// A lightweight TCP event forwarder that broadcasts domain events to
/// LAN peers (KDS tablets, secondary displays, etc.).
///
/// Clone the handle for passing into `tokio::spawn` or event handlers.
#[derive(Clone)]
pub struct LanEventForwarder {
    tx: broadcast::Sender<String>,
}

/// Handle for registering event bus handlers.
///
/// Obtained via [`LanEventForwarder::handle()`].
#[derive(Clone)]
pub struct LanForwarderHandle {
    tx: broadcast::Sender<String>,
}

impl LanEventForwarder {
    /// Create a new forwarder and start the broadcast channel.
    ///
    /// Does **not** start the TCP listener — call [`run()`](Self::run)
    /// to bind and accept connections.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self { tx }
    }

    /// Return a handle for registering event bus subscribers.
    pub fn handle(&self) -> LanForwarderHandle {
        LanForwarderHandle {
            tx: self.tx.clone(),
        }
    }

    /// Bind the TCP listener and start accepting connections.
    ///
    /// Spawns a tokio task for each accepted connection that reads
    /// from the broadcast channel and writes newline-delimited JSON
    /// to the TCP stream.
    ///
    /// This function runs forever (or until a fatal bind error).
    pub async fn run(self) {
        let addr = format!("0.0.0.0:{LAN_PORT}");
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => {
                tracing::info!(address = %addr, "LAN event forwarder started");
                l
            }
            Err(e) => {
                tracing::error!(address = %addr, error = %e, "failed to bind LAN forwarder");
                return;
            }
        };

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    tracing::debug!(peer = %peer_addr, "LAN peer connected");
                    let rx = self.tx.subscribe();
                    tokio::spawn(handle_peer(stream, peer_addr.to_string(), rx));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "LAN accept failed");
                }
            }
        }
    }

    /// Send an event JSON string to all connected peers.
    ///
    /// This is non-blocking — broadcast messages are queued in the
    /// channel and delivered asynchronously.
    pub fn broadcast(&self, event_json: String) {
        // Ignore the subscriber count; peers that can't keep up will
        // have their receiver lagged and closed automatically.
        let _ = self.tx.send(event_json);
    }
}

impl Default for LanEventForwarder {
    fn default() -> Self {
        Self::new()
    }
}

// ── Peer handler ─────────────────────────────────────────────────────

/// Read events from the broadcast channel and write them to the TCP
/// stream, one line per event. Exits when the stream is closed or the
/// broadcast sender is dropped.
async fn handle_peer(
    mut stream: tokio::net::TcpStream,
    peer_addr: String,
    mut rx: broadcast::Receiver<String>,
) {
    loop {
        match rx.recv().await {
            Ok(msg) => {
                let line = format!("{msg}\n");
                if let Err(e) = stream.write_all(line.as_bytes()).await {
                    tracing::debug!(peer = %peer_addr, error = %e, "LAN peer disconnected");
                    return;
                }
            }
            Err(broadcast::error::RecvError::Lagged(count)) => {
                tracing::warn!(peer = %peer_addr, skipped = count, "LAN peer lagged");
                // Continue — the peer will receive the next event.
            }
            Err(broadcast::error::RecvError::Closed) => {
                tracing::debug!(peer = %peer_addr, "LAN forwarder shutting down");
                return;
            }
        }
    }
}

// ── Event bus handlers ───────────────────────────────────────────────

/// Handlers that bridge domain events to the LAN broadcast channel.
///
/// Register these on the event bus during application setup so that
/// `sale.completed` and `order.course_fired` events are forwarded to
/// all connected LAN peers.
impl LanForwarderHandle {
    /// Create an `EventHandler<SaleCompleted>` that serialises the
    /// event to JSON and broadcasts it to all connected LAN peers.
    pub fn sale_completed_handler(&self) -> SaleCompletedHandler {
        SaleCompletedHandler {
            tx: self.tx.clone(),
        }
    }

    /// Create an `EventHandler<CourseFired>` that serialises the
    /// event to JSON and broadcasts it to all connected LAN peers.
    pub fn course_fired_handler(&self) -> CourseFiredHandler {
        CourseFiredHandler {
            tx: self.tx.clone(),
        }
    }
}

// ── SaleCompletedHandler ─────────────────────────────────────────────

/// Forwards `sale.completed` events to LAN peers as JSON.
pub struct SaleCompletedHandler {
    tx: broadcast::Sender<String>,
}

impl EventHandler<SaleCompleted> for SaleCompletedHandler {
    fn handle(&self, event: &SaleCompleted) -> ModuleResult {
        let json = serde_json::to_string(event)
            .map_err(|e| anyhow::anyhow!("serialising SaleCompleted: {e}"))?;
        let _ = self.tx.send(json);
        Ok(())
    }
}

// ── CourseFiredHandler ───────────────────────────────────────────────

/// Forwards `order.course_fired` events to LAN peers as JSON.
pub struct CourseFiredHandler {
    tx: broadcast::Sender<String>,
}

impl EventHandler<CourseFired> for CourseFiredHandler {
    fn handle(&self, event: &CourseFired) -> ModuleResult {
        let json = serde_json::to_string(event)
            .map_err(|e| anyhow::anyhow!("serialising CourseFired: {e}"))?;
        let _ = self.tx.send(json);
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpStream;
    use tokio::sync::broadcast;

    // ── LanEventForwarder construction ───────────────────────────

    #[test]
    fn forwarder_new_creates_channel() {
        let fwd = LanEventForwarder::new();
        // Broadcast a message and verify it doesn't panic.
        fwd.broadcast("{\"test\":1}".into());
    }

    #[test]
    fn forwarder_handle_is_clone() {
        let fwd = LanEventForwarder::new();
        let h1 = fwd.handle();
        let h2 = fwd.handle();
        // Both handles should be able to create handlers
        let _ = h1.sale_completed_handler();
        let _ = h2.course_fired_handler();
    }

    #[test]
    fn forwarder_default_impl() {
        let fwd: LanEventForwarder = Default::default();
        fwd.broadcast("ping".into());
    }

    // ── SaleCompletedHandler ─────────────────────────────────────

    #[test]
    fn sale_completed_handler_forwards_event() {
        let (tx, mut rx) = broadcast::channel(16);
        let handler = SaleCompletedHandler { tx };

        let event = SaleCompleted {
            sale_id: "sale-1".into(),
            line_items: vec![],
            total_minor: 1000,
            currency: "USD".into(),
            customer_id: None,
        };

        handler.handle(&event).unwrap();

        let received = rx.try_recv().unwrap();
        assert!(received.contains("\"sale-1\""), "JSON should contain sale_id");
    }

    #[test]
    fn sale_completed_handler_with_items() {
        let (tx, mut rx) = broadcast::channel(16);
        let handler = SaleCompletedHandler { tx };

        let event = SaleCompleted {
            sale_id: "sale-2".into(),
            line_items: vec![oz_core::events::SaleCompletedLine {
                sku: "COFFEE".into(),
                qty: 2,
                unit_price_minor: 350,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 700,
            currency: "USD".into(),
            customer_id: Some("cust-1".into()),
        };

        handler.handle(&event).unwrap();

        let received = rx.try_recv().unwrap();
        assert!(received.contains("COFFEE"));
        assert!(received.contains("cust-1"));
        assert!(received.contains("700"));
    }

    // ── CourseFiredHandler ───────────────────────────────────────

    #[test]
    fn course_fired_handler_forwards_event() {
        let (tx, mut rx) = broadcast::channel(16);
        let handler = CourseFiredHandler { tx };

        let event = CourseFired {
            sale_id: "sale-42".into(),
            course_id: "main".into(),
            display_number: Some(101),
            items: vec![oz_core::events::CourseItem {
                sku: "STEAK".into(),
                qty: 2,
                name: "Grilled Steak".into(),
            }],
        };

        handler.handle(&event).unwrap();

        let received = rx.try_recv().unwrap();
        assert!(received.contains("sale-42"));
        assert!(received.contains("main"));
        assert!(received.contains("STEAK"));
        assert!(received.contains("Grilled Steak"));
    }

    #[test]
    fn course_fired_handler_no_display_number() {
        let (tx, mut rx) = broadcast::channel(16);
        let handler = CourseFiredHandler { tx };

        let event = CourseFired {
            sale_id: "sale-3".into(),
            course_id: "drinks".into(),
            display_number: None,
            items: vec![],
        };

        handler.handle(&event).unwrap();

        let received = rx.try_recv().unwrap();
        assert!(received.contains("null"));
    }

    // ── Peer handler (integration-style) ─────────────────────────

    #[tokio::test]
    async fn peer_handles_broadcast_messages() {
        let (tx, rx) = broadcast::channel(16);

        // Bind a listener on a random port for testing.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Accept one connection and spawn the peer handler.
        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_peer(stream, "test-peer".into(), rx).await;
        });

        // Connect a client.
        let mut client = TcpStream::connect(addr).await.unwrap();

        // Send a broadcast message.
        tx.send("{\"event\":\"test\"}".into()).unwrap();

        // Close the broadcast channel to stop the server.
        drop(tx);

        // Read from the client stream.
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut client, &mut buf)
            .await
            .unwrap();

        assert_eq!(buf, b"{\"event\":\"test\"}\n");

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn peer_handles_multiple_messages() {
        let (tx, rx) = broadcast::channel(16);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_peer(stream, "test-peer".into(), rx).await;
        });

        let mut client = TcpStream::connect(addr).await.unwrap();

        tx.send("msg1".into()).unwrap();
        tx.send("msg2".into()).unwrap();
        drop(tx);

        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut client, &mut buf)
            .await
            .unwrap();

        assert_eq!(buf, b"msg1\nmsg2\n");

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn peer_graceful_shutdown() {
        let (tx, rx) = broadcast::channel(16);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_peer(stream, "graceful-test".into(), rx).await;
        });

        let _client = TcpStream::connect(addr).await.unwrap();

        // Drop the sender — this should cause the peer handler to exit via RecvError::Closed.
        drop(tx);

        // The server should exit cleanly (no timeout).
        tokio::time::timeout(std::time::Duration::from_secs(2), server_handle)
            .await
            .expect("peer should have shut down within timeout")
            .unwrap();
    }
}

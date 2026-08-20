//! LAN event forwarder — a lightweight TCP server that broadcasts domain
//! events to KDS tablet peers on the local network.
//!
//! # Features
//!
//! - Broadcasts `sale.completed` and `order.course_fired` events to all
//!   connected LAN peers via newline-delimited JSON over TCP.
//! - Sends a `{"type":"ping"}` heartbeat every 5 seconds to detect
//!   silent disconnections.
//! - When a TCP write fails, buffers the undelivered event in an
//!   in-memory per-peer queue.
//! - When a peer reconnects, automatically flushes buffered events
//!   before entering the normal broadcast loop.
//!
//! # Wire format
//!
//! Each forwarded event is a single line of JSON terminated by `\n`:
//!
//! - `sale.completed`: `{"sale_id":"...","line_items":[...],...}`
//! - `order.course_fired`: `{"sale_id":"...","course_id":"...",...}`
//! - Heartbeat: `{"type":"ping"}`
//!
//! # Example
//!
//! ```no_run
//! use oz_pos_app_lib::lan_server::LanEventForwarder;
//!
//! let forwarder = LanEventForwarder::default();
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use foundation::contracts::{EventHandler, ModuleResult};
use oz_core::events::{CourseFired, SaleCompleted};
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, broadcast};

/// Maximum number of pending broadcast messages before old ones are
/// dropped (avoids unbounded memory growth for slow peers).
const CHANNEL_CAPACITY: usize = 256;

/// Interval between heartbeat pings sent to each peer (seconds).
const HEARTBEAT_INTERVAL_SECS: u64 = 5;

/// Timeout for the PSK handshake (seconds).
const PSK_HANDSHAKE_TIMEOUT_SECS: u64 = 5;

/// First message a peer must send when PSK is configured.
#[derive(Debug, Deserialize)]
struct HelloMsg {
    op: String,
    psk: String,
}

/// A peer request for KDS discovery information.
#[derive(Debug, Deserialize)]
struct DiscoverMsg {
    op: String,
}

// ── LanEventForwarder ────────────────────────────────────────────────

/// A lightweight TCP event forwarder that broadcasts domain events to
/// LAN peers (KDS tablets, secondary displays, etc.).
///
/// Clone the handle for passing into `tokio::spawn` or event handlers.
#[derive(Clone)]
pub struct LanEventForwarder {
    tx: broadcast::Sender<String>,
    /// Per-peer offline buffer. Maps peer address → buffered JSON events
    /// that could not be delivered due to disconnection.
    offline_buffer: Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// TCP bind address (e.g. `"127.0.0.1:9180"` or `"0.0.0.0:9180"`).
    bind_addr: String,
    /// Optional pre-shared key for external bind mode.
    /// When `Some`, peers must send `{"op":"hello","psk":"<value>"}`
    /// as their first message or the connection is dropped.
    psk: Option<Arc<String>>,
    /// Discovery payload returned when a peer sends `{"op":"discover"}`.
    /// Set at construction time; `None` disables discovery responses.
    discovery_payload: Option<Arc<String>>,
}

/// Handle for registering event bus handlers.
///
/// Obtained via [`LanEventForwarder::handle()`].
#[derive(Clone)]
pub struct LanForwarderHandle {
    tx: broadcast::Sender<String>,
}

impl LanEventForwarder {
    /// Create a new forwarder with an empty offline buffer.
    pub fn new(bind_addr: String, psk: Option<String>) -> Self {
        let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            tx,
            offline_buffer: Arc::new(Mutex::new(HashMap::new())),
            bind_addr,
            psk: psk.map(Arc::new),
            discovery_payload: None,
        }
    }

    /// Set the discovery payload for KDS device enrollment.
    ///
    /// When set, a peer can send `{"op":"discover"}` after the
    /// PSK handshake (if any) and receive this JSON payload as a
    /// response. The payload should contain the restaurant POS
    /// identity, active devices, and version information.
    pub fn with_discovery(mut self, payload: String) -> Self {
        self.discovery_payload = Some(Arc::new(payload));
        self
    }

    /// Return a handle for registering event bus subscribers.
    pub fn handle(&self) -> LanForwarderHandle {
        LanForwarderHandle {
            tx: self.tx.clone(),
        }
    }

    /// Bind the TCP listener and start accepting connections.
    ///
    /// Spawns a tokio task for each accepted connection that:
    /// 1. Optionally performs a PSK handshake (for external bind)
    /// 2. Flushes any buffered events for this peer address
    /// 3. Subscribes to the broadcast channel
    /// 4. Sends heartbeat pings every 5s
    /// 5. Buffers events on write failure and exits
    pub async fn run(self) {
        let listener = match TcpListener::bind(&self.bind_addr).await {
            Ok(l) => {
                tracing::info!(address = %self.bind_addr, "LAN event forwarder started");
                l
            }
            Err(e) => {
                tracing::error!(address = %self.bind_addr, error = %e, "failed to bind LAN forwarder");
                return;
            }
        };

        let psk = self.psk.clone();

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let addr = peer_addr.to_string();
                    tracing::debug!(peer = %addr, "LAN peer connected");

                    // Drain buffered events for this peer before subscribing.
                    let initial_events: Vec<String> = self
                        .offline_buffer
                        .lock()
                        .await
                        .remove(&addr)
                        .unwrap_or_default();

                    if !initial_events.is_empty() {
                        tracing::info!(
                            peer = %addr,
                            count = initial_events.len(),
                            "flushing buffered LAN events on reconnection"
                        );
                    }

                    let rx = self.tx.subscribe();
                    let buffer = self.offline_buffer.clone();
                    let psk_clone = psk.clone();
                    let discovery = self.discovery_payload.clone();
                    tokio::spawn(handle_peer(
                        stream,
                        addr,
                        rx,
                        buffer,
                        initial_events,
                        psk_clone,
                        discovery,
                    ));
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
        let _ = self.tx.send(event_json);
    }

    /// Return the number of buffered events across all disconnected peers.
    pub async fn buffered_count(&self) -> usize {
        let buf = self.offline_buffer.lock().await;
        buf.values().map(|v| v.len()).sum()
    }

    /// Return the number of distinct peer addresses with buffered events.
    pub async fn buffered_peer_count(&self) -> usize {
        self.offline_buffer.lock().await.len()
    }
}

impl Default for LanEventForwarder {
    fn default() -> Self {
        Self::new("127.0.0.1:9180".to_string(), None)
    }
}

/// Discovery endpoint response for KDS device enrollment.
#[derive(Debug, serde::Serialize)]
pub struct KdsDiscoverResponse {
    /// The Restaurant POS terminal ID.
    pub restaurant_pos_id: String,
    /// Active KDS devices registered under this POS.
    pub devices: Vec<oz_core::kds::KdsDevice>,
    /// Application version.
    pub version: &'static str,
}

// ── Peer handler ─────────────────────────────────────────────────────

/// Read events from the broadcast channel and write newline-delimited
/// JSON to the TCP stream. Sends a heartbeat ping every 5 seconds.
///
/// When a write fails, the undelivered message is pushed to the offline
/// buffer keyed by `peer_addr` so it can be replayed on reconnection.
async fn handle_peer(
    mut stream: tokio::net::TcpStream,
    peer_addr: String,
    mut rx: broadcast::Receiver<String>,
    offline_buffer: Arc<Mutex<HashMap<String, Vec<String>>>>,
    initial_events: Vec<String>,
    psk: Option<Arc<String>>,
    discovery_payload: Option<Arc<String>>,
) {
    // Phase 0: PSK handshake (only when configured for external bind).
    // The handshake runs inside the spawned task so a slow/malicious
    // peer cannot block the accept loop (DoS protection).
    if let Some(expected_psk) = &psk {
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        match tokio::time::timeout(
            std::time::Duration::from_secs(PSK_HANDSHAKE_TIMEOUT_SECS),
            reader.read_line(&mut line),
        )
        .await
        {
            Ok(Ok(_)) => {
                let hello: Result<HelloMsg, _> = serde_json::from_str(line.trim());
                match hello {
                    Ok(msg) if msg.op == "hello" && msg.psk == **expected_psk => {
                        tracing::debug!(peer = %peer_addr, "LAN PSK handshake accepted");
                    }
                    _ => {
                        tracing::warn!(peer = %peer_addr, "LAN PSK handshake rejected — bad credentials");
                        return;
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::warn!(peer = %peer_addr, error = %e, "LAN PSK handshake failed — read error");
                return;
            }
            Err(_elapsed) => {
                tracing::warn!(peer = %peer_addr, "LAN PSK handshake timed out");
                return;
            }
        }
    }

    // Phase 1: Handle discovery request (if enabled).
    // KDS devices send `{"op":"discover"}` after connecting to learn
    // the Restaurant POS identity and available devices.
    if let Some(ref payload) = discovery_payload {
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        match tokio::time::timeout(
            std::time::Duration::from_secs(PSK_HANDSHAKE_TIMEOUT_SECS),
            reader.read_line(&mut line),
        )
        .await
        {
            Ok(Ok(_)) => {
                if let Ok(msg) = serde_json::from_str::<DiscoverMsg>(line.trim()) {
                    if msg.op == "discover" {
                        let response = format!("{payload}\n");
                        if let Err(e) = stream.write_all(response.as_bytes()).await {
                            tracing::debug!(
                                peer = %peer_addr,
                                error = %e,
                                "failed to send discovery response"
                            );
                            return;
                        }
                        tracing::debug!(peer = %peer_addr, "KDS discovery response sent");
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::debug!(
                    peer = %peer_addr,
                    error = %e,
                    "discovery read failed"
                );
            }
            Err(_elapsed) => {
                // No discovery request — proceed to normal event streaming.
            }
        }
    }

    // Phase 2: Flush any buffered events first.
    for event in initial_events {
        let line = format!("{event}\n");
        if let Err(e) = stream.write_all(line.as_bytes()).await {
            tracing::debug!(
                peer = %peer_addr,
                error = %e,
                "failed to flush buffered events to reconnecting peer"
            );
            // Re-buffer the remaining events for next reconnect attempt.
            offline_buffer
                .lock()
                .await
                .entry(peer_addr.clone())
                .or_default()
                .push(event);
            return;
        }
    }

    // Phase 3: Normal broadcast loop with heartbeat.
    let mut heartbeat =
        tokio::time::interval(std::time::Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
    // Skip the immediate first tick so the heartbeat doesn't fire
    // before initial events are flushed.
    heartbeat.tick().await;

    loop {
        tokio::select! {
            biased;

            msg = rx.recv() => {
                match msg {
                    Ok(msg) => {
                        let line = format!("{msg}\n");
                        if let Err(e) = stream.write_all(line.as_bytes()).await {
                            tracing::debug!(
                                peer = %peer_addr,
                                error = %e,
                                "LAN peer disconnected, event buffered"
                            );
                            // Buffer the event for replay on reconnection.
                            offline_buffer
                                .lock()
                                .await
                                .entry(peer_addr.clone())
                                .or_default()
                                .push(msg);
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!(peer = %peer_addr, skipped = count, "LAN peer lagged");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::debug!(peer = %peer_addr, "LAN forwarder shutting down");
                        return;
                    }
                }
            }

            _ = heartbeat.tick() => {
                if let Err(e) = stream.write_all(b"{\"type\":\"ping\"}\n").await {
                    tracing::debug!(
                        peer = %peer_addr,
                        error = %e,
                        "LAN peer disconnected (heartbeat)"
                    );
                    return;
                }
            }
        }
    }
}

// ── Event bus handlers ───────────────────────────────────────────────

/// Handlers that bridge domain events to the LAN broadcast channel.
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
#[path = "lan_server_tests.rs"]
mod tests;

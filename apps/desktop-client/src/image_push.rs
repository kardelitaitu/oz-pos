//! Desktop image push scheduler (spec 0046b §3.6).
//!
//! Background daemon that drains the local `image_push_queue` to the cloud
//! server's `POST /api/v1/images:batch` endpoint.  Wakes on jittered
//! intervals (60–300 s), peeks up to 16 ready hashes, reads the transcoded
//! WebP files from the app cache, builds length-prefixed binary frames, and
//! POSTs the batch.  Per-hash outcomes (`stored` / `duplicate` / `rejected`)
//! are applied via `Store::mark_push_attempt` — success removes the queue
//! row, failure bumps the attempt counter with AWS full-jitter backoff, and
//! dead-lettering at 8 attempts.

use std::path::PathBuf;
use std::sync::Arc;

use oz_core::Store;
use oz_core::sync_client::SyncConfig;
use tokio::sync::Mutex;

/// ── Constants ──────────────────────────────────────────────────────────

/// Maximum images per batch push.
const BATCH_MAX_IMAGES: usize = 16;

/// Maximum total batch payload in bytes (512 KB).
const BATCH_MAX_BYTES: usize = 512 * 1024;

/// Jitter lower bound (seconds).
const JITTER_MIN: u64 = 60;

/// Jitter upper bound (seconds).
const JITTER_MAX: u64 = 300;

/// ── Types ──────────────────────────────────────────────────────────────

/// The image push scheduler.
///
/// Holds a reference to the store DB, the app cache directory (where WebP
/// files live), and a shared `reqwest::Client` for HTTP calls.
pub struct ImagePushScheduler {
    db: Arc<Mutex<rusqlite::Connection>>,
    cache_dir: PathBuf,
    client: reqwest::Client,
}

impl ImagePushScheduler {
    /// Create a new scheduler.
    ///
    /// `db` is the store's SQLite connection (behind a tokio mutex).
    /// `cache_dir` is the Tauri app cache directory (the `images/` subdir
    /// holds the WebP files).
    pub fn new(db: Arc<Mutex<rusqlite::Connection>>, cache_dir: PathBuf) -> Self {
        Self {
            db,
            cache_dir,
            client: reqwest::Client::new(),
        }
    }

    /// Run the drain loop forever.  On each tick:
    ///
    /// 1. Resolve sync config (server URL + API key) from settings.
    /// 2. Peek up to 16 due hashes from `image_push_queue`.
    /// 3. Read the WebP files from the cache dir.
    /// 4. Build length-prefixed binary frames.
    /// 5. POST to `{server_url}/api/v1/images:batch`.
    /// 6. Apply per-hash outcomes via `Store::mark_push_attempt`.
    pub async fn run(&self) {
        tracing::info!("image push scheduler started");
        // Initial delay so the daemon doesn't hammer the server on boot.
        tokio::time::sleep(std::time::Duration::from_secs(JITTER_MIN)).await;
        loop {
            self.drain_once().await;
            tokio::time::sleep(rand_jitter(JITTER_MIN, JITTER_MAX)).await;
        }
    }

    /// One drain cycle: peek queue → read files → POST → process outcomes.
    async fn drain_once(&self) {
        // Phase 1: brief DB lock — read config + peek push batch.
        let (config, pending) = {
            let db = self.db.lock().await;
            let store = Store::new(&db);
            let config = SyncConfig::from_settings(&store).ok().flatten();
            let pending = store.peek_push_batch(BATCH_MAX_IMAGES).unwrap_or_default();
            (config, pending)
        };

        let Some(config) = config else {
            tracing::trace!("image push: sync not configured, skipping");
            return;
        };
        if pending.is_empty() {
            tracing::trace!("image push: queue empty");
            return;
        }

        let token = match &config.api_key {
            Some(k) if !k.is_empty() => k.clone(),
            _ => {
                tracing::warn!("image push: no API key configured");
                return;
            }
        };

        // Phase 2: read files from the app cache (no DB lock held).
        let mut frames: Vec<u8> = Vec::with_capacity(BATCH_MAX_BYTES.min(8192));
        let mut batch_hashes: Vec<String> = Vec::with_capacity(pending.len());
        let mut total_bytes = 0usize;

        for (hash, _size_bytes, _attempts) in &pending {
            let path = self.cache_dir.join("images").join(format!("{hash}.webp"));
            match tokio::fs::read(&path).await {
                Ok(bytes) => {
                    let frame_len = 4 + bytes.len(); // u32 length prefix + payload
                    if batch_hashes.len() >= BATCH_MAX_IMAGES
                        || total_bytes + frame_len > BATCH_MAX_BYTES
                    {
                        break;
                    }
                    frames.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                    frames.extend_from_slice(&bytes);
                    total_bytes += frame_len;
                    batch_hashes.push(hash.clone());
                }
                Err(e) => {
                    tracing::warn!(hash, error = %e, "image push: missing file, skipping");
                    // File missing — mark as a failure so the queue entry is
                    // either retried (backoff) or eventually dead-lettered.
                    let db = self.db.lock().await;
                    let store = Store::new(&db);
                    let _ = store.mark_push_attempt(hash, false);
                }
            }
        }

        if batch_hashes.is_empty() {
            return;
        }

        // Phase 3: POST the batch (no DB lock).
        let url = format!("{}/api/v1/images:batch", config.server_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .header("content-type", "application/octet-stream")
            .body(frames)
            .send()
            .await;

        let outcomes = match resp {
            Ok(r) if r.status().is_success() => {
                match r.json::<serde_json::Value>().await {
                    Ok(json) => {
                        // Parse per-hash results from the batch response.
                        // Response shape: { results: [{ hash: "..."|null, status: "stored"|"duplicate"|"rejected" }] }
                        let mut map = std::collections::HashMap::new();
                        if let Some(results) = json["results"].as_array() {
                            for item in results {
                                let status = item["status"].as_str().unwrap_or("rejected");
                                let hash = item["hash"].as_str().unwrap_or_default().to_owned();
                                map.insert(hash, status.to_owned());
                            }
                        }
                        map
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "image push: failed to parse batch response");
                        return self.mark_all_failed(&batch_hashes).await;
                    }
                }
            }
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                tracing::warn!(status = %status, body = %body, "image push: batch rejected");
                return self.mark_all_failed(&batch_hashes).await;
            }
            Err(e) => {
                tracing::warn!(error = %e, "image push: network error");
                return self.mark_all_failed(&batch_hashes).await;
            }
        };

        // Phase 4: apply per-hash outcomes (brief DB lock per hash).
        let db = self.db.lock().await;
        let store = Store::new(&db);
        for hash in &batch_hashes {
            let success = match outcomes.get(hash).map(|s| s.as_str()) {
                Some("stored" | "duplicate") => true,
                _ => {
                    if let Some(status) = outcomes.get(hash) {
                        tracing::warn!(hash, status = %status, "image push: rejected");
                    }
                    false
                }
            };
            if let Err(e) = store.mark_push_attempt(hash, success) {
                tracing::error!(hash, error = %e, "image push: mark_push_attempt failed");
            }
        }
    }

    /// Mark all given hashes as failed (network error or server rejection).
    async fn mark_all_failed(&self, hashes: &[String]) {
        let db = self.db.lock().await;
        let store = Store::new(&db);
        for hash in hashes {
            if let Err(e) = store.mark_push_attempt(hash, false) {
                tracing::error!(hash, error = %e, "image push: mark_push_attempt (all-failed) error");
            }
        }
    }
}

/// Generate a uniform random delay in [min, max] seconds.
fn rand_jitter(min: u64, max: u64) -> std::time::Duration {
    use rand::Rng;
    let secs = rand::thread_rng().gen_range(min..=max);
    std::time::Duration::from_secs(secs)
}

#[cfg(test)]
#[path = "image_push_tests.rs"]
mod tests;

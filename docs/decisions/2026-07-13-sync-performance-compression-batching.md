# ADR #10: Sync Performance (Compression & Batching)

**Status:** Proposed
**Date:** 2026-07-13
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** sync, compression, batching, performance, network, low-bandwidth

---

## Context

OZ-POS is designed to support low-end hardware (e.g., Android 9 tablets, Core2Duo PCs) operating in low-bandwidth, unstable, or offline environments. 

Currently, the client-side sync engine (`platform-sync`) retrieves all pending offline changes from the `offline_queue` SQLite table and sends them in a single massive HTTP request to the cloud sync server (`oz-cloud-server`). If a terminal has been offline for a long period, the size of this payload can reach thousands of rows, causing several issues on low-end terminals:
- Memory exhaustion (OOM crashes) during serialization of large JSON payloads.
- High CPU utilization causing blocking or freezes in the main React/Tauri UI thread.
- Frequent HTTP timeouts (due to the 30-second transport limit).

Additionally, the sync payload is currently sent uncompressed, consuming significant cellular or satellite data in remote regions.

---

## Decision

To resolve these network and memory bottlenecks, we will implement three key architectural improvements in the synchronization layer:

### 1. Client-Side Sync Batching
The sync engine will split the list of pending items into smaller chunks (batches of 20 items) before transmission:

```rust
// Process pending changes in batches of 20
for chunk in pending.chunks(20) {
    let results = self.transport.push_items(chunk).await?;
    for (item, outcome) in chunk.iter().zip(results.iter()) {
        // apply outcomes (Accepted / Conflict / Rejected)...
    }
}
```

- **Safety & Progress**: Chunking limits payload sizes to a few kilobytes, ensuring fast request resolution times and visual progress feedback for the user.
- **Transactional Integrity**: Since the local SQLite engine operates via `rusqlite` transactions, each batch can commit its success state independently. In case of a drop midway, progress is preserved.

### 2. Brotli/Gzip Compression
To handle low-bandwidth network connections:
- **Server Middleware**: We will enable Brotli and Gzip decompression/compression middleware on the Axum server using the `tower-http` library's `compression-full` feature.
- **Client Configuration**: The `reqwest` client builder inside the `platform-sync` transport layer will be configured to automatically request and send compressed payloads.

### 3. Backoff with Jitter
To protect server resources from DDoS-like spikes when connection recovers:
- The background daemon will apply exponential backoff (starting at 2s up to 60s) with random jitter (+/- 10-30s) on failed sync cycles.

---

## Consequences

### Positive
- **OOM Prevention**: Low-resource tablets will never serialize more than 20 rows at a time, keeping RAM footprint flat and stable.
- **UI Responsiveness**: Small batch serialization runs asynchronously without locking the Tauri main thread.
- **Data Savings**: Compression reduces sync data payloads by 50-70%, lowering operating costs for multi-store merchants using mobile hotspots.

### Negative
- **Mid-cycle Interruptions**: If the network connection drops halfway through a sync cycle, the first batches will have successfully synced while later ones remain pending. The system must gracefully resume on the next cycle (which is handled natively by checking the synced state of individual rows).

---

## Related
- ADR #4 — Store-First Tenancy & Workspace Tenancy
- ADR #6 — CRDT Delta Ledger & Offline Sync
- `platform/sync/src/lib.rs` — Client sync engine
- `platform/sync/src/transport.rs` — HTTP push/pull transport
- `apps/cloud-server/src/main.rs` — Axum server entrypoint

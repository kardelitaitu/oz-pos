# ADR #10: Sync Performance and Ultra-Low-Cost Server Strategy

**Status:** Proposed
**Date:** 2026-07-13
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** sync, compression, batching, performance, network, low-bandwidth, hosting-cost, sqlite

---

## Context

OZ-POS is designed to support low-end hardware (e.g., Android 9 tablets, Core2Duo PCs) operating in low-bandwidth, unstable, or offline environments. 

Additionally, the project has a strict cost target: **the cloud sync server must cost no more than $2–$3 a month to host** (typically a tiny shared-core VPS with 512MB–1GB RAM and 1 shared vCPU), yet be capable of handling synchronization for thousands of active stores. 

Currently:
- The client-side sync engine (`platform-sync`) retrieves all pending offline changes from the `offline_queue` SQLite table and sends them in a single massive HTTP request to the cloud sync server (`oz-cloud-server`). If a terminal has been offline for a long period, this causes OOM crashes, high CPU spikes, and request timeouts.
- The sync payload is currently sent uncompressed, consuming significant cellular or satellite data.
- High-frequency active polling by thousands of stores would easily overwhelm a low-tier $2 VPS with connection overhead and concurrent database queries.

---

## Decision

To resolve these network and memory bottlenecks and achieve the $2–$3/month hosting target, we will implement the following strategies:

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

### 4. Server-Side SQLite Backend
To bypass the memory footprint of a running PostgreSQL instance (which easily consumes 250MB–500MB of RAM just sitting idle):
- We will deploy the cloud server utilizing the in-process **SQLite backend** (`DbPool::Sqlite`). SQLite runs directly in the Rust binary process, reducing baseline idle RAM footprint to under 30MB.
- **Multi-Tenant Isolation**: Tenant sync logs will be partitioned into separate SQLite database files (e.g. `tenants/tenant_a.db`) to keep file write locks isolated and make tenant backups trivial.

### 5. Event-Driven Sync (No High-Frequency Polling)
To prevent constant server query load:
- Terminals will only trigger a sync cycle **on-event** (e.g. immediately following a completed sale, cash drop, or staff clock-in).
- Active high-frequency background polling is disabled. When idle, terminals will run a lazy sync heartbeat once every **15 to 30 minutes**.

### 6. Single-Threaded Runtime & Concurrency Limits
- **Tokio Runtime**: The server will compile utilizing a single-threaded Tokio execution runtime. For a 1-core $2 VPS, this eliminates context-switching CPU overhead.
- **Axum Guardrails**: Enforce a `ConcurrencyLimitLayer` at the Axum router level to cap concurrent active requests (e.g. max 50). Burst sync requests will be queued or shed gracefully rather than causing OOM crashes.

---

## Consequences

### Positive
- **OOM Prevention**: Low-resource tablets and the $2/month server keep RAM footprints flat, stable, and predictable.
- **UI Responsiveness**: Small batch serialization runs asynchronously without locking the Tauri main thread.
- **Data Savings**: Compression reduces sync data payloads by 50-70%, lowering operating costs for multi-store merchants using mobile hotspots.
- **Resource Protection**: Concurrency limits and jitter protect the shared CPU core from spikes.

### Negative
- **Mid-cycle Interruptions**: If the network connection drops halfway through a sync cycle, the first batches will have successfully synced while later ones remain pending. The system must gracefully resume on the next cycle (which is handled natively by checking the synced state of individual rows).

---

## Related
- ADR #4 — Store-First Tenancy & Workspace Tenancy
- ADR #6 — CRDT Delta Ledger & Offline Sync
- `platform/sync/src/lib.rs` — Client sync engine
- `platform/sync/src/transport.rs` — HTTP push/pull transport
- `apps/cloud-server/src/main.rs` — Axum server entrypoint


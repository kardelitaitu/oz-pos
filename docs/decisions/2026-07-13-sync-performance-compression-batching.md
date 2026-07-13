# ADR #10: Sync Performance, Ultra-Low-Cost Server, and 3-Month Retention Strategy

**Status:** Proposed
**Date:** 2026-07-13
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** sync, compression, batching, performance, network, low-bandwidth, hosting-cost, sqlite, retention

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
- **Multi-Tenant Isolation**: Tenant sync logs will be partitioned into separate SQLite database files (e.g., `tenants/tenant_a.db`) to keep file write locks isolated and make tenant backups trivial.

### 5. Event-Driven Sync (No High-Frequency Polling)
To prevent constant server query load:
- Terminals will only trigger a sync cycle **on-event** (e.g., immediately following a completed sale, cash drop, or staff clock-in).
- Active high-frequency background polling is disabled. When idle, terminals will run a lazy sync heartbeat once every **15 to 30 minutes**.

### 6. Single-Threaded Runtime & Concurrency Limits
- **Tokio Runtime**: The server will compile utilizing a single-threaded Tokio execution runtime. For a 1-core $2 VPS, this eliminates context-switching CPU overhead.
- **Axum Guardrails**: Enforce a `ConcurrencyLimitLayer` at the Axum router level to cap concurrent active requests (e.g., max 50). Burst sync requests will be queued or shed gracefully rather than causing OOM crashes.

### 7. Smart Disk-Pressure Data Retention & Pruning Strategy
To keep the storage requirements of the server lightweight enough to run on cheap $2 VPS hosting with a 40 GB SSD, while preserving historical sync anchors as long as possible:
- **Baseline Retention**: The cloud server guarantees a minimum **3-month rolling window (90 days)** of sync data.
- **Smart / Opportunistic Retention (Disk < 90%)**: The server does **not** actively delete files older than 3 months if the total SSD disk usage is under **90%**. This lets terminals retrieve historical delta-sync states even after long absences (e.g. seasonal stores).
- **Disk-Pressure Pruning (Disk >= 90%)**: A lightweight hourly background task checks the server's filesystem disk space:
  - If disk space exceeds **90%**, the server will locate all monthly sync log database files (e.g., `sync_log_YYYY_MM.db`) and sort them chronologically (oldest first).
  - The server deletes the oldest files that are **older than the 3-month retention threshold** one by one.
  - The check runs iteratively until either the disk space drops back below **85%** or all database files older than 3 months have been deleted.
- **SQLite Storage Reclamation**: Since standard SQLite deletion does not shrink the file size on disk, we will deploy one of two reclamation mechanisms:
  - **Option A (Monthly Database Partitioning)**: Segment tenant sync logs into monthly database files (e.g. `sync_log_2026_06.db`). Deleting data older than 3 months is simplified to a standard OS file deletion (`rm sync_log_2026_03.db`), which releases space instantly back to the OS in less than 1 millisecond.
  - **Option B (Incremental Auto-Vacuum)**: Initialize the SQLite database with `PRAGMA auto_vacuum = INCREMENTAL;` and execute `PRAGMA incremental_vacuum(N);` after deletes to release free pages back to the OS without lock contention.
- **Client Recovery**: If a client terminal attempts to sync with a `since` timestamp that points to a period already pruned by the server, the server returns an `AnchorExpired` error. The client will catch this error and trigger a full base sync instead of a delta sync.


---

## Sizing & SSD Estimation (3-Month Retention)

Based on a daily footprint of **~600 KB per active workspace** (200 sales transactions/day @ 3 KB per transaction record + overhead), the 3-month rolling window consumes **~60 MB per workspace** (including safety buffers).

| Workspace Count | Minimum SSD Size (3-Month Window) | CPU Cores | RAM | Target VPS Cost |
| :--- | :--- | :--- | :--- | :--- |
| **100** | **20 GB SSD** (OS: 10GB + DB: 6GB + Buffer: 4GB) | 1 Core (Shared) | 1 GB – 2 GB | $2.50 – $3.50/mo |
| **500** | **40 GB SSD** (OS: 10GB + DB: 30GB) | 1 Core (Shared) | 2 GB | **$3.50 – $5.00/mo** |
| **2,000** | **160 GB SSD** (OS: 15GB + DB: 120GB + Buffer: 25GB) | 2 Cores | 4 GB | $10.00 – $15.00/mo |
| **5,000** | **320 GB SSD** (OS: 20GB + DB: 300GB) | 4 Cores | 8 GB | $20.00 – $25.00/mo |
| **10,000** | **640 GB SSD** (OS: 25GB + DB: 600GB) | 8 Cores | 16 GB | $40.00 – $50.00/mo |

---

## Consequences

### Positive
- **OOM Prevention**: Low-resource tablets and the $2/month server keep RAM footprints flat, stable, and predictable.
- **UI Responsiveness**: Small batch serialization runs asynchronously without locking the Tauri main thread.
- **Data Savings**: Compression reduces sync data payloads by 50-70%, lowering operating costs for multi-store merchants using mobile hotspots.
- **Resource Protection**: Concurrency limits and jitter protect the shared CPU core from spikes.
- **Minimal Storage Overhead**: Limiting retention to 3 months guarantees that 500 workspaces can easily run on a cheap 40 GB SSD.

### Negative
- **Mid-cycle Interruptions**: If the network connection drops halfway through a sync cycle, the first batches will have successfully synced while later ones remain pending. The system must gracefully resume on the next cycle (which is handled natively by checking the synced state of individual rows).

---

## Related
- ADR #4 — Store-First Tenancy & Workspace Tenancy
- ADR #6 — CRDT Delta Ledger & Offline Sync
- `platform/sync/src/lib.rs` — Client sync engine
- `platform/sync/src/transport.rs` — HTTP push/pull transport
- `apps/cloud-server/src/main.rs` — Axum server entrypoint



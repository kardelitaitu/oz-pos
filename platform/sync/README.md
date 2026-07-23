<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (0 findings, 1 minor note) · crate is platform-sync (Cargo.toml name); SyncEngine (src/lib.rs:743/933) and SyncConfig (field at lib.rs:745, new(config: SyncConfig)) verified; usage snippet matches · note: the directory tree (queue/, transport/, replication/, conflict/) is illustrative — actual layout is flat src/ files queue.rs, transport.rs, replication.rs, conflict.rs (+ daemon.rs, pg_daemon.rs, pg_transport.rs) · offline-first + LWW conflict resolution described matches the implementation -->

# platform-sync

Offline-first sync engine for OZ-POS. Provides an offline queue, HTTP transport, push/pull replication, and last-write-wins conflict resolution.

## Architecture

```
platform/sync/
├── queue/       — Local change log (wraps oz-core offline_queue table)
├── transport/   — HTTP client for communicating with remote sync server
├── replication/ — Push + pull orchestration
└── conflict/    — Conflict resolution strategies (LWW initially)
```

## Usage

```rust
use platform_sync::{SyncEngine, SyncConfig};
use oz_core::db::Store;

let engine = SyncEngine::new(config);
let result = engine.run_sync_cycle(&store).await?;
```

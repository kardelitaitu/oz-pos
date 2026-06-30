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

<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (0 findings, 1 minor note) · crate is platform-sync (Cargo.toml name); SyncEngine (src/lib.rs:743/933) and SyncConfig (field at lib.rs:745, new(config: SyncConfig)) verified; usage snippet matches · note: the directory tree (queue/, transport/, replication/, conflict/) is illustrative — actual layout is flat src/ files queue.rs, transport.rs, replication.rs, conflict.rs (+ daemon.rs, pg_daemon.rs, pg_transport.rs) · offline-first + LWW conflict resolution described matches the implementation -->

# platform-sync

Offline-first sync engine for OZ-POS. Provides an offline queue, HTTP transport, push/pull replication, and last-write-wins conflict resolution.

When a retained pull anchor expires, both `SyncEngine` and the SQLite-backed
`SyncDaemon` fetch the authoritative snapshot, import it transactionally, and
reset the durable pull anchor to the server's `oldest_available` boundary.
This prevents a terminal from re-fetching the same snapshot on every daemon
cycle.

The PostgreSQL-backed `PgSyncDaemon` follows the same recovery contract: it
checks the remote retention watermark, fetches reference data directly from
the PostgreSQL tables when the durable anchor has expired, imports it through
the shared typed importer, and resets the anchor only after a successful
import. PostgreSQL deployments use a dedicated sync database rather than the
HTTP snapshot endpoint.

### Real PostgreSQL integration checks

The crate includes ignored integration tests for the PostgreSQL retention and
snapshot contracts. They use a disposable database and do not run as part of
the normal unit-test suite:

```text
docker run --name oz-pos-pg-sync-tdd --rm -d \
  -e POSTGRES_USER=ozsync -e POSTGRES_PASSWORD=ozsync \
  -e POSTGRES_DB=ozsync -p 127.0.0.1:15432:5432 postgres:16-alpine

PG_SYNC_TEST_URL=postgresql://ozsync:ozsync@127.0.0.1:15432/ozsync \
  cargo test -p platform-sync --test pg_integration -- --ignored --nocapture
```

The harness verifies real PostgreSQL `MIN(created_at)` anchor expiry,
boolean/timestamp decoding in snapshots, and that credential verifier
material is absent from the typed snapshot. The disposable container must be
removed by the caller after the run (`docker stop oz-pos-pg-sync-tdd`).

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

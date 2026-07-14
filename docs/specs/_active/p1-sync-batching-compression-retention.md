# P-1 — Sync batching, compression, backoff & retention

- **Status:** PENDING
- **Phase:** 1 of 3 (Sync Performance Strategy — ADR #10)
- **Parent:** `docs/decisions/2026-07-13-sync-performance-compression-batching.md`
- **Severity:** HIGH
- **Owner:** TBD
- **Est. effort:** 2-3 weeks

## Summary

Eliminate OOM risk on low-end terminals and reduce bandwidth cost by (1) splitting sync payloads into adaptive 64 KB batches, (2) compressing responses with Gzip, (3) applying capped exponential backoff with full-jitter on failures, and (4) pruning the `offline_queue` after 90 days with incremental vacuum.

## Baseline (pre-fix)

- `platform/sync/src/lib.rs`: `SyncEngine::run_sync_cycle()` sends **all** pending items in a single `transport.push_items()` call. A terminal with 500 pending items pushes ~3 MB of JSON in one HTTP request.
- `platform/sync/src/transport.rs`: `reqwest::Client` has no `.gzip(true)`. No compression headers. JSON payloads are sent and received uncompressed.
- `platform/sync/src/daemon.rs`: `SyncDaemon` sleeps a fixed random range (60-120s) on each cycle. No exponential backoff. The `retry_count` field is incremented on failure but **never read**.
- `crates/oz-core/migrations/018_offline_queue.sql`: No `PRAGMA auto_vacuum`. Synced and failed items accumulate indefinitely. There is no cleanup mechanism.

## Acceptance criteria

### Batching
- [ ] `build_batches()` produces chunks ≤ 64 KB serialized JSON each
- [ ] Minimum 1 item per batch (no empty requests)
- [ ] Batches are sent **sequentially** — batch N+1 waits for batch N's response
- [ ] Each batch commits independently via `rusqlite` transaction
- [ ] Server detects batched vs legacy push: array-of-arrays = batched, flat array = legacy
- [ ] Server-side push endpoint accepts both formats and returns per-item outcomes

### Compression
- [ ] `tower-http` enabled with `compression-gzip` feature (not `compression-full`)
- [ ] Server compresses push/pull responses with Gzip level 6
- [ ] `reqwest::ClientBuilder` in `transport.rs` configured with `.gzip(true)`
- [ ] Client advertises `Accept-Encoding: gzip`
- [ ] Client request bodies are NOT re-compressed (≤ 64 KB batches make compression overhead not worth it)

### Backoff
- [ ] Exponential backoff formula: `sleep_ms = rand(0, min(60_000, 2_000 * 2^retries))`
- [ ] `retry_count` field drives the delay. Reset to 0 after a successful sync cycle
- [ ] On-event syncs (manual, sale completed) bypass backoff entirely

### Retention
- [ ] `offline_queue` table DDL updated with `PRAGMA auto_vacuum = INCREMENTAL;` (new migration)
- [ ] Existing production databases altered with `PRAGMA auto_vacuum = 2;`
- [ ] Hourly background task checks disk usage
- [ ] If disk ≥ 90%: cursor-based loop `SELECT id … ORDER BY id LIMIT 500`, `DELETE WHERE id IN (?)`, `PRAGMA incremental_vacuum(50)`
- [ ] Loop repeats until disk < 85% or no more rows ≥ 90 days old
- [ ] Data < 90 days is never deleted, regardless of disk pressure
- [ ] Server response for pull with pruned `since` timestamp: returns `AnchorExpired` error (includes `oldest_available` timestamp for client)
- [ ] Client catches `AnchorExpired`, logs warning with `oldest_available`, and retries on next sync cycle. A full base sync is not attempted in this phase — the snapshot endpoint (Phase 3) will provide that capability.

### Performance targets
- [ ] p95 sync time for 50 items on simulated cellular (200ms RTT): < 2 seconds
- [ ] Server RSS memory under load (100 concurrent syncs): < 100 MB
- [ ] Client RSS memory with 500 pending items: no increase over baseline
- [ ] Compression ratio on pull responses: ≥ 40% size reduction

## Plan

1. **Batching**: Add `build_batches()` to `platform/sync/src/lib.rs`. Modify `run_sync_cycle()` to iterate over batches sequentially. Update `SyncError` types if needed. Update `transport.rs` `push_items()` to accept `&[OfflineQueueItem]` (already does — no signature change needed).
2. **Server batch detection**: In `apps/cloud-server/src/sync_api.rs`, wrap push handler to inspect JSON root: array-of-arrays → process each batch independently; flat array → legacy path.
3. **Compression — server**: Add `compression-gzip` to `tower-http` feature list in workspace `Cargo.toml`. Add `.layer(CompressionLayer::new().gzip(true).quality(6))` to Axum router in `apps/cloud-server/src/main.rs`.
4. **Compression — client**: Add `.gzip(true)` to `reqwest::ClientBuilder` in `platform/sync/src/transport.rs`.
5. **Backoff**: Modify `platform/sync/src/daemon.rs` `run()` loop to track failure count, apply exponential backoff formula, reset on success. Keep existing on-event trigger path that bypasses backoff.
6. **Retention migration**: Create `crates/oz-core/migrations/XXX_offline_queue_autovacuum.sql` enabling `PRAGMA auto_vacuum = INCREMENTAL;`. Register in `migrations.rs`.
7. **Pruning background task**: Add a new file `apps/cloud-server/src/prune.rs` with a `start_prune_loop(db: Arc<tokio::sync::Mutex<rusqlite::Connection>>)` function that runs on `tokio::time::interval(Duration::from_secs(3600))`. Implements the cursor-based DELETE loop + `incremental_vacuum`. Spawn this task in `main.rs` after the Axum server starts.
8. **AnchorExpired error**: Define `SyncErrorCode::AnchorExpired` variant in shared crate with `oldest_available: Option<DateTime<Utc>>` field. Return from pull handler when `since` < oldest retained row. The `retry_after_secs` field is reserved for Phase 3 and excluded from this phase's struct. On the client side, add catch logic in `run_sync_cycle()` that logs a warning and returns early (instead of crashing), so the next scheduled cycle retries naturally.

## Verification

| Check | Expected |
|-------|----------|
| `cargo build -p platform-sync -p oz-cloud-server --lib --tests` | exit 0 |
| `cargo clippy -p platform-sync -p oz-cloud-server --lib --tests -- -D warnings` | 0 warnings |
| `cargo test -p platform-sync` | all passing |
| `cargo test -p oz-cloud-server` | all passing |
| Manual: 500 fake pending items, measure RSS before/after | No increase |
| Manual: `curl -v -H "Accept-Encoding: gzip" …` | Response has `Content-Encoding: gzip` |
| Manual: populate offline_queue with 1000 items, set 90% disk threshold | Items > 90 days deleted, vacuum runs |

## Residual / follow-ups

- **Client pull endpoint**: This phase does not add pull pagination. Pull responses may still be large for terminals offline > 30 days. Phase 3 addresses this with cursor-based pagination.
- **Tiered heartbeat**: The existing random 60-120s daemon interval is kept as-is. Phase 3 introduces server-driven tiered heartbeat.
- **Priority tiers**: All items pushed in arrival order. Phase 2 adds priority-based reordering.
- **Concurrency limits**: No per-route-group limits in this phase. Phase 2 adds them.
- **Snapshot endpoint**: Not included. Phase 3 adds it.

## References

- `docs/decisions/2026-07-13-sync-performance-compression-batching.md`
- `platform/sync/src/lib.rs`
- `platform/sync/src/transport.rs`
- `platform/sync/src/daemon.rs`
- `apps/cloud-server/src/sync_api.rs`
- `apps/cloud-server/src/main.rs`
- `crates/oz-core/migrations/018_offline_queue.sql`

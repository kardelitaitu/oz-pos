# Sync Module Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** Sync module — offline queue, HTTP transport, replication, conflict resolution, background daemons, tenant scope, cloud endpoints, settings UI, and tests
> **Status:** ✅ **FULLY REMEDIATED** — all 12 findings closed (SYNC-01→SYNC-12)
> **Remediation commits:** `a1ea01e7` (SYNC-01), `b722740f` (SYNC-02+05), `5229e296` (SYNC-03+04), `85e323c7` (SYNC-06), `5633e790` (SYNC-07/08/09/10/11), `178abfbf` (SYNC-12)
> **Production code changed:** Yes — see commit chain above

## Scope

This audit evaluates the Sync module against the universal checklist in `audit/AUDIT_JULY_2026.md`: functionality and state management, offline and recovery behaviour, conflict resolution, authorization and tenant boundaries, UI loading/error states, accessibility and localization, theming, performance, and test coverage.

Inspected areas:

- `platform/sync/src/{lib,queue,transport,replication,conflict,daemon,pg_daemon,pg_transport}.rs`
- `platform/sync/tests/integration_test.rs`
- `platform/sync/README.md`
- `crates/oz-core/src/{sync_client,offline}.rs`
- `crates/oz-core/src/db/offline.rs`
- `apps/desktop-client/src/commands/{sync,offline}.rs`
- `apps/cloud-server/src/sync_api.rs`
- `ui/src/api/offline.ts`
- `ui/src/hooks/{useCloudSync,useSyncConnection}.ts`
- `ui/src/frontend/shell/StatusBar.tsx`
- `ui/src/contexts/SettingsContext.tsx`
- `ui/src/__tests__/{useCloudSync,SyncSection}.test.tsx`
- ADR #6 (CRDT Delta Ledger & Offline Sync) and ADR #21 (Sync Conflict Resolution Strategy)

## Architecture summary

The codebase contains two overlapping sync execution paths:

1. `platform_sync::SyncEngine::run_sync_cycle()` performs a health check, priority/size-batched push, conflict dispatch, paginated pull, and snapshot recovery after an expired anchor.
2. `platform_sync::SyncDaemon::run_tick()` periodically reads local settings and pending items, pushes them, applies outcomes, then pulls remote data. It uses the same transport and queue primitives but does not share all of the `SyncEngine`'s safeguards.

The desktop commands provide immediate push (`sync_run`), destructive snapshot import (`sync_pull`), connection/token helpers, and offline queue inspection. The cloud server exposes authenticated, rate-limited `/api/sync/*` endpoints with tenant filtering and cursor pagination. The UI has a shared `useCloudSync` hook, a connection poller, Settings integration, and a StatusBar indicator.

## Findings

### SYNC-01 — Background daemon replays the remote queue on every cycle (P0 if daemon-enabled; latent P1 otherwise)

**Evidence:** `platform/sync/src/daemon.rs::run_tick()` calls `transport.pull_updates(None, None)` on every configured cycle. It does not persist or pass a last-sync anchor or cursor. `apps/cloud-server/src/sync_api.rs::pull_handler()` returns all tenant rows when `since` is absent. The daemon then calls `SyncQueue::apply_remote()` for every returned item.

`SyncQueue::apply_remote()` applies actions as mutations: `complete_sale` deducts stock, `stock.adjusted` changes stock, and `product.created` creates a product. There is no remote-item receipt table or idempotency check before applying an item.

**Impact:** Every daemon cycle can reapply previously pulled stock/sale actions. A stock adjustment or sale that is returned repeatedly can decrement inventory repeatedly, producing silent data corruption. This is a production-impacting issue if the background daemon is enabled for a store.

**Recommendation:** Persist a per-tenant/per-store pull cursor or authoritative server anchor. Pass it to every pull, advance it only after the corresponding page is applied successfully, and use a durable remote-item ID ledger/unique constraint so replay is harmless. Add a regression test that runs two daemon ticks against the same remote item and verifies the local mutation occurs once.

**Status:** ✅ **Remediated** (`a1ea01e7`) — durable pull anchor (`sync_pull_state` single-row table) + remote-item idempotency ledger (`sync_applied_items` with `INSERT OR IGNORE`); two-cycle replay regression test added

### SYNC-02 — Daemon conflict handling bypasses the ADR #21 resolver

**Evidence:** `platform/sync/src/lib.rs::SyncEngine::run_sync_cycle()` dispatches conflicts through `conflict::resolve_conflict()`, which distinguishes version LWW, sale state ordering, and stock CRDT handling. In contrast, `platform/sync/src/daemon.rs::run_tick()` handles `PushOutcome::Conflict` by marking the local item synced and re-enqueuing the remote action/payload directly, with the comment `LWW: remote wins`.

**Impact:** The manual/immediate engine and the background daemon can resolve the same conflict differently. A higher local product version, an advanced sale state, or a stock delta merge can be discarded when the daemon is the path that processes the conflict.

**Recommendation:** Extract one conflict-application service used by both paths. It must persist the resolution result, preserve both source items for auditability, and apply the same ADR #21 strategy regardless of trigger.

**Status:** ✅ **Remediated** (`b722740f`) — shared conflict service + consumable CRDT merge envelope used by both the engine and the daemon

### SYNC-03 — `sync_pull` IPC contract is incomplete in the front end

**Evidence:** `apps/desktop-client/src/commands/sync.rs::sync_pull()` requires a `SyncPullArgs` value with `confirm_destructive: bool` and rejects false. `ui/src/api/offline.ts::syncPull()` invokes `loggedInvoke('sync_pull')` without any argument. `useCloudSync.pullFromServer()` also calls `syncPull()` without an explicit confirmation payload, despite its comment saying the caller should have shown a confirmation dialog.

**Impact:** The UI pull action cannot satisfy the command's required argument contract. Depending on Tauri argument decoding, the operation fails before the server request or is unavailable from the UI. The visible confirmation intent is not enforced end-to-end.

**Recommendation:** Change the typed API to `syncPull(confirmDestructive: boolean)` or, preferably, `syncPull({ confirmDestructive: true })`; make the confirmation dialog and command payload one explicit flow. Add a UI/API contract test asserting the exact IPC arguments and a command test that rejects false/missing consent.

**Status:** ✅ **Remediated** (`5229e296`) — typed `SyncPullArgs { confirmDestructive }` contract, confirm-dialog flow end-to-end, contract + consent-rejection tests

### SYNC-04 — Manual offline retry marks items synced without performing cloud sync

**Evidence:** `apps/desktop-client/src/commands/offline.rs::retry_offline_sync()` describes itself as a placeholder and loops over every pending item, calling `store.mark_offline_synced()` directly. It does not contact `sync_run`, validate the action, or apply a server outcome.

**Impact:** Any caller of `retry_offline_sync` can make unsent transactions disappear from the pending queue and appear successful. If this command is exposed through a UI or future IPC caller, it creates false delivery guarantees and possible loss of sales or inventory events.

**Recommendation:** Remove the placeholder command from production registration until it delegates to the real sync pipeline, or make it call the authenticated sync service and mark each item only after an accepted outcome. Add tests for network failure, rejection, conflict, and successful retry.

**Status:** ✅ **Remediated** (`5229e296`) — `retry_offline_sync` delegates to the real pipeline (config → async push → per-item outcome); network/rejection/conflict/success tests added

### SYNC-05 — CRDT conflict merge payload cannot be consumed by the queue applier

**Evidence:** `platform/sync/src/conflict.rs::resolve_stock_crdt()` creates a new winner whose payload is shaped as `{ local: ..., remote: ..., merge_type: "crdt_delta" }`. `SyncQueue::apply_resolution()` enqueues that winner as a normal queue item. The `stock.adjusted` branch of `SyncQueue::apply_remote()` deserializes the payload into `StockAdjustmentPayload { sku, delta }`, which requires top-level `sku` and `delta`; the merged payload has neither. The `stock.movement` branch likewise expects a flat movement row.

**Impact:** The local `apply_resolution` step can mark the conflict resolved and enqueue the merged event, but the eventual reapplication/replication path cannot consume that event through the normal remote dispatcher. The intended “both deltas are preserved” property is therefore not end-to-end true.

**Recommendation:** Represent a CRDT merge as two idempotent ledger events, or add an explicit `crdt_delta` action/decoder that validates and applies both nested deltas transactionally. Test the complete path from `resolve_conflict()` through `apply_resolution()` and eventual `apply_remote()`.

**Status:** ✅ **Remediated** (`b722740f`) — CRDT merge represented as two idempotent ledger events executable through the normal remote dispatcher

### SYNC-06 — Snapshot endpoint exports password/PIN hashes to sync clients

**Evidence:** `apps/cloud-server/src/sync_api.rs::snapshot_handler()` includes `pin_hash` in every serialized user row. `crates/oz-core/src/sync_client.rs` defines `SnapshotUser` with `pin_hash` and `apply_snapshot()` writes it into the local users table.

**Impact:** A sync token with snapshot access receives credential-verifier material for all users in the tenant. A compromised terminal, log, debug capture, or overly broad operator token increases the blast radius of credential theft. Synchronization of user authentication hashes is not necessary for product/tax/reference-data synchronization.

**Recommendation:** Remove `pin_hash` from the snapshot contract and prohibit user credential replication. If user metadata must sync, return only the minimum non-secret fields and use a separate, tightly authorized identity-management flow for credential changes. Add a contract test asserting sensitive fields never appear in snapshot JSON.

**Status:** ✅ **Remediated** (`85e323c7`) — `pin_hash` removed from the snapshot contract on all three paths (server, client, platform-sync); `deny_unknown_fields` + placeholder on insert, existing hash preserved on conflict; raw-bytes sensitive-field test; `verify_pin` fails closed

### SYNC-07 — Local queue tenancy is not consistently bound to the authenticated store/session

**Evidence:** `oz_core::offline::Store::enqueue_offline()` uses tenant `"default"`; `list_pending_offline()`, `list_all_offline()`, and `pending_offline_count()` are unscoped variants. The scoped `list_pending_offline_for_tenant()` exists but is not used by the desktop offline commands or `SyncQueue`. Server-side push replaces the incoming item tenant with the JWT tenant, but client-side queue reads and writes do not derive a tenant from the session.

**Impact:** In a process that can access more than one store or tenant database, an unscoped sync operation can mix pending events or display counts from the wrong scope. Store-separated SQLite files reduce the practical blast radius but do not make the API contract safe by construction.

**Recommendation:** Require a resolved tenant/store context for enqueue, list, count, mark, and delete operations. Keep the server JWT tenant authoritative and add tests that exercise two tenant IDs through the client queue boundary.

**Status:** ✅ **Remediated** (`5633e790`) — tenant-scoped queue variants (mark synced strict NotFound, mark failed / delete / count scoped) + four two-tenant boundary tests

### SYNC-08 — The Settings sync connection test is simulated, not a connectivity test

**Evidence:** `ui/src/hooks/useCloudSync.ts::testConnection()` waits `SIMULATED_LATENCY_TEST_MS`, then treats any non-empty `serverURL` as reachable. It does not call the typed `testSyncConnection()` API. The separate `useSyncConnection` hook does call the API, so the Settings action and StatusBar indicator use different semantics.

**Impact:** Settings can report “Connection test passed” for an invalid URL, offline server, or unreachable host. Operators may save a configuration believing it was verified when it was never contacted.

**Recommendation:** Replace the simulation with `testSyncConnection(serverURL.trim())`, surface the returned status/latency, and keep the same timeout/error taxonomy as the daemon. Add tests for success, timeout, invalid URL, and server rejection.

**Status:** ✅ **Remediated** (`5633e790`) — `useCloudSync.testConnection` now calls the real `test_sync_connection` IPC (shared with the StatusBar poller) and surfaces the server status; success/timeout/unreachable/exception tests added

### SYNC-09 — Snapshot and pull error handling can present an empty success

**Evidence:** `snapshot_handler()` returns JSON error objects through `axum::Json<serde_json::Value>` rather than an HTTP error status. `fetch_snapshot_from_server()` deserializes a successful response into `Snapshot` with default empty arrays. `useCloudSync.pullFromServer()` reports an informational “snapshot was empty” message when all counts are zero.

**Impact:** A server-side snapshot query failure can be interpreted as a valid empty snapshot, masking an operational or schema error and giving the operator false confidence that local data is synchronized.

**Recommendation:** Return appropriate non-2xx status codes and a typed error envelope. Reject an error-shaped or structurally incomplete snapshot before applying it, and distinguish “valid empty snapshot” from “request failed” in the UI.

**Status:** ✅ **Remediated** (`5633e790`) — snapshot handler returns non-2xx with an error envelope on query failure; cache path returns Ok; client rejects non-success before applying

### SYNC-10 — Pull row decode failures are silently discarded by the cloud endpoint

**Evidence:** In `apps/cloud-server/src/sync_api.rs::pull_handler()`, SQLite rows are converted with `rows.filter_map(|r| r.ok()).collect()`. A malformed row or schema mismatch is dropped without returning an error or metric, while the response still succeeds.

**Impact:** A client can receive an apparently complete page that silently omits one or more changes. The omission is especially difficult to diagnose because cursors advance based on the returned page.

**Recommendation:** Collect row conversion errors explicitly, log/metric the affected tenant and cursor, and return a 5xx response rather than silently truncating data. Add a test with a malformed row/schema fixture.

**Status:** ✅ **Remediated** (`5633e790`) — pull row decode failures return 500 + `sync_pull_row_decode_failures_total` metric instead of silent `filter_map` drops; snapshot queries fail loudly too; malformed-row test added

### SYNC-11 — Transport and UI contracts contain stale or inconsistent shapes

**Evidence:** `ui/src/api/offline.ts` defines `OfflineQueueItemDto` without `payload`, while `apps/desktop-client/src/commands/offline.rs::OfflineQueueItemDto` serializes `payload`. The same file defines `SyncResult` with `synced`/`failed`, while the Rust offline retry command returns `syncedCount`, `failedCount`, and `totalCount`. The Sync API also exposes both `syncRun()` and the separate placeholder `retryOfflineSync()` paths.

**Impact:** TypeScript callers can silently lose fields or misread responses, and future code may select the placeholder path instead of the real cloud sync path. This increases integration regressions around offline recovery.

**Recommendation:** Generate or contract-test DTOs from the Rust command schema, remove unused duplicate result types, and expose one canonical retry operation. Add a compile-time/API contract test covering every registered offline/sync command.

**Status:** ✅ **Remediated** (`5633e790`) — `OfflineQueueItemDto` TS type matches the Rust serializer (`payload` restored); `SyncResult` DTO aligned (`syncedCount`/`failedCount`/`totalCount`); IPC contract tests cover every offline/sync command

### SYNC-12 — Sync UI has accessibility and localization drift in shared status surfaces

**Evidence:** `ui/src/frontend/shell/StatusBar.tsx` contains hardcoded user-visible strings such as `Application status`, `OZ-POS Enterprise v0.0.24`, `Sync`, `Stripe`, and `Proprietary License`; its sync conflict warning uses an inline color. `useSyncConnection.ts` also stores hardcoded labels such as `Checking…` and `Disconnected` rather than receiving localized strings.

**Impact:** Screen readers and non-English operators receive incomplete localization, while theme contrast depends on the inline fallback rather than the shared token system.

**Recommendation:** Move all visible labels and ARIA names to Fluent keys with fallbacks, inject localization into the connection hook or localize at the render boundary, and use a semantic CSS class/token for the warning icon.

**Status:** ✅ **Remediated** (`178abfbf`) — all StatusBar labels + ARIA through Fluent keys with en/id parity; conflict icon uses a semantic token class; `useSyncConnection` no longer fabricates hardcoded labels (render-boundary localization)

## Positive controls observed

- Cloud sync routes are wrapped in JWT authentication and per-tenant rate limiting.
- Server push and pull queries use the tenant ID from JWT claims rather than trusting the incoming item tenant.
- Pull pagination has a stable `(created_at, id)` cursor and a 500-row page cap.
- The transport has request timeouts, classified connection errors, migration redirects, and explicit anchor-expiry handling.
- `SyncEngine` batches by byte size and priority, uses a health check, and has snapshot recovery logic.
- Database snapshot imports use transactions and tests cover rollback on a foreign-key failure.
- The daemon prevents duplicate starts, supports graceful shutdown, and records status/backoff metadata.
- The focused unit suite covers resolver strategies, queue operations, transport serialization, daemon lifecycle, batching, and snapshot import behaviour.

## Test and validation results

Commands run during this audit:

```text
cargo test -p platform-sync --lib
cargo test -p platform-sync --test integration_test
```

Results:

- `platform-sync` library tests: **212 passed, 0 failed**
- `platform-sync` integration tests: **19 ignored, 0 failed, 0 executed** (the integration tests are gated behind the `slow-tests` feature)
- The report content and heading structure were reviewed. The report is untracked, so ordinary `git diff --check` does not inspect it; Markdown hard-break spacing is intentionally retained in the quoted metadata lines.

The integration suite's ignored status is important: it is not evidence that the HTTP push/pull, conflict, retry, tenant, and throughput scenarios pass in this run. A CI or scheduled job should execute the suite with `--features slow-tests` and publish the result.

## Recommended remediation order

1. **SYNC-01:** Add durable pull anchors and idempotent remote application before enabling daemon sync broadly. — ✅ done
2. **SYNC-02 and SYNC-05:** Unify conflict handling and make CRDT merges executable end-to-end. — ✅ done
3. **SYNC-03 and SYNC-04:** Repair the destructive-pull contract and remove the placeholder retry path. — ✅ done
4. **SYNC-06:** Remove credential hashes from snapshots and add a sensitive-field contract test. — ✅ done
5. **SYNC-07 through SYNC-11:** Close scope, error, API-shape, and observability gaps. — ✅ done
6. **SYNC-12:** Complete the shared status-bar i18n and token cleanup. — ✅ done
7. Run the full integration suite with `slow-tests`, then add a two-cycle replay test as a release gate. — ⚠️ remaining: the `slow-tests`-gated integration suite (19 tests) is not run by the default CI; a scheduled/CI job should execute `cargo test -p platform-sync --features slow-tests` and publish the result.

## Audit status

✅ **FULLY REMEDIATED** — all 12 findings closed across 6 commits:

| Finding | Severity | Commit |
|---------|----------|--------|
| SYNC-01 durable pull anchor + idempotency | P0/P1 | `a1ea01e7` |
| SYNC-02 shared conflict service | P1 | `b722740f` |
| SYNC-03 pull consent contract | P1 | `5229e296` |
| SYNC-04 real offline retry | P1 | `5229e296` |
| SYNC-05 CRDT merge executability | P1 | `b722740f` |
| SYNC-06 no credential hashes in snapshots | P1 | `85e323c7` |
| SYNC-07 client queue tenancy | P2 | `5633e790` |
| SYNC-08 real connection test | P2 | `5633e790` |
| SYNC-09 snapshot error status | P2 | `5633e790` |
| SYNC-10 no silent row drops | P2 | `5633e790` |
| SYNC-11 DTO contract shapes | P2 | `5633e790` |
| SYNC-12 status-bar i18n | P3 | `178abfbf` |

**Residual (documented, not blocking):** the `slow-tests`-gated integration suite should be wired into a scheduled/CI job as a release gate.

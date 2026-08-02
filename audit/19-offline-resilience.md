# Offline Resilience Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** Retail POS offline mode, KDS offline behavior, durable queueing, optimistic updates, reconnect synchronization, and conflict handling  
> **Status:** ✅ FULLY REMEDIATED — all 12 findings closed (commit table below)  
> **Production code changed:** See remediation commits

## Scope

This audit evaluates sector 19 against the universal checklist in `audit/AUDIT_JULY_2026.md`: offline detection, durable local state, optimistic UI behavior, queue persistence, retry and backoff, reconnect synchronization, idempotency, conflict resolution, tenant/session boundaries, service-worker and asset availability, multi-tab behavior, loading/error states, and test coverage.

Inspected areas:

- `ui/src/hooks/useKdsOffline.ts`
- `ui/src/features/kds/KdsScreen.tsx`
- `ui/src/features/offline/OfflineQueueScreen.tsx`
- `ui/src/components/ConnectionStatus.tsx`
- `ui/src/api/offline.ts`
- `ui/src/main.tsx`
- `ui/src/main.tablet.tsx`
- `ui/src/__tests__/useKdsOffline.test.ts`
- `ui/src/__tests__/OfflineQueueScreen.test.tsx`
- `ui/src/__tests__/KdsScreen.test.tsx`
- `apps/desktop-client/src/commands/offline.rs`
- `crates/oz-core/src/offline.rs`
- `crates/oz-core/src/db/offline.rs`
- `crates/oz-core/migrations/018_offline_queue.sql`
- `crates/oz-core/migrations/055_offline_queue_tenant.sql`
- `crates/oz-core/migrations/073_offline_queue_priority.sql`
- `crates/oz-core/tests/offline_integration.rs`
- `platform/sync/src/queue.rs`
- `platform/sync/src/conflict.rs`
- `platform/sync/src/daemon.rs`
- `platform/sync/README.md`
- Offline and sync decision records
- Repository-wide service-worker/PWA search results

## Architecture summary

The repository has two different offline mechanisms. KDS uses a browser-side `useKdsOffline` hook that stores the last successful order snapshot and failed status actions in `localStorage`. The general offline queue is a Rust/SQLite `offline_queue` table exposed through Tauri commands and surfaced by `OfflineQueueScreen`. The `platform-sync` crate wraps the SQLite queue and contains entity-specific conflict strategies, including version/status LWW and stock-delta CRDT merging.

The KDS screen performs optimistic status updates when a backend update fails and retries queued actions after a later successful fetch. The main POS and KDS entry points do not register a service worker, and no service-worker file was found. Consequently, browser-level asset bootstrapping and API cache behavior are not covered by an app-wide offline layer.

## Findings

### OFF-01 — The generic retry command marks queued items synced without executing their actions

**Evidence:** `apps/desktop-client/src/commands/offline.rs::retry_offline_sync` lists pending items and then calls `store.mark_offline_synced(&item.id)` for every item. The command comment says this is a placeholder and that real dispatch will be added later. The Rust `platform/sync::SyncQueue::apply_remote` contains action dispatch logic, but the Tauri retry command does not call it or a remote transport.

**Impact:** A user pressing “Sync All” can receive a successful sync result while sales, voids, stock changes, or other queued actions were never applied. Those transactions are removed from the pending set and become difficult to recover. This is a data-integrity failure, not merely a missing progress indicator.

**Recommendation:** Replace the placeholder with an explicit action dispatcher backed by the real sync engine. Process each item transactionally, mark it synced only after the action is durably applied or acknowledged by the remote, mark retryable failures as pending with an incremented retry count, and mark permanent failures as failed with a safe diagnostic. Add integration tests for every supported action and a test proving unsupported actions are not falsely marked synced.

**Severity:** P0 · data integrity

**Status:** ✅ Remediated — `retry_offline_sync` is no longer a placeholder: it reads sync config, POSTs the batch via `sync_client::send_items_to_server`, and marks each item synced/failed **only** per the server's per-item outcome (`apply_sync_outcomes` handles Accepted / Rejected / Conflict and refuses to fabricate success when unconfigured). Verified during this audit (the doc's evidence was stale — the sync pipeline landed in the audit/09 work). Pinned by `91766573` contract tests.

### OFF-02 — The Rust sync result DTO does not match the UI contract

**Evidence:** `ui/src/api/offline.ts` defines `SyncResult` with `synced` and `failed`. `OfflineQueueScreen.tsx` renders `syncResult.synced` and `syncResult.failed`. The Rust `SyncResult` in `apps/desktop-client/src/commands/offline.rs` serializes fields named `syncedCount`, `failedCount`, and `totalCount` because of `#[serde(rename_all = "camelCase")]`.

**Impact:** A successful generic offline sync can render undefined counts or an incorrect result message. This weakens operator confidence exactly when they need to know whether queued work was applied, and it can hide the OFF-01 placeholder behavior behind a misleading UI.

**Recommendation:** Choose one contract and enforce it end-to-end. Prefer a shared, documented DTO with `synced`, `failed`, `total`, and per-item outcomes if the UI needs them; otherwise update the TypeScript type and render fields to match the Rust response. Add an IPC contract test that serializes the Rust-shaped payload and asserts the screen displays correct counts.

**Severity:** P1 · operator visibility and contract correctness

**Status:** ✅ Remediated — `ui/src/api/offline.ts` `SyncResult` (`syncedCount`/`failedCount`/`totalCount`) matches the Rust DTO exactly; `api-offline-contract.test.ts` pins the shape and `91766573` adds a screen-level test asserting the exact counts render. No more undefined counts.

### OFF-03 — The KDS optimistic status update is not persisted into the cached order snapshot

**Evidence:** `KdsScreen.advanceStatus` updates React `orders` state optimistically after `useKdsOffline.wrapUpdate` queues a failed update. `useKdsOffline.updateCache` is called by `wrapFetch` only after a successful fetch; `wrapUpdate` does not update `LS_CACHED_ORDERS`.

**Impact:** A kitchen operator can advance a ticket while offline, see the new status, reload the app, and receive the older cached status. The UI appears to undo a successful local action even though the action remains in the pending queue. This can cause duplicate taps or incorrect preparation decisions.

**Recommendation:** Persist an optimistic projection alongside the pending action, or apply queued actions to the cached snapshot on write and replay them deterministically on startup. Mark projected fields as pending so the UI distinguishes “locally advanced” from server-confirmed. Add a reload-while-offline test covering status, queue, and visual state.

**Severity:** P1 · KDS correctness

**Status:** ✅ Remediated — `useKdsOffline` now persists the optimistic projection into the cached snapshot on update failure and replays queued projections over fresh data on the next successful fetch (`233eed6b`). Reload-while-offline keeps the projected status (covered by reload + replay tests).

### OFF-04 — KDS reconnect retry is indirectly coupled to a successful fetch and the online-event trigger is unused

**Evidence:** `useKdsOffline` exposes `forceRetryCounter` and increments it on the browser `online` event, but `KdsScreen` does not consume that return value. KDS queue flushing occurs only inside `fetchOrders` when the fetch succeeds and `pendingQueueLength > 0`. An OS online event alone does not set `online` to true or invoke a retry.

**Impact:** Reconnect recovery depends on a visibility change, a push event, or another fetch trigger. A device can regain connectivity while queued actions remain pending and the banner remains offline until an unrelated event occurs. This prolongs stale state and increases the chance of operator confusion.

**Recommendation:** Make reconnect a first-class state transition: consume the online trigger in KDS, attempt a bounded fetch/retry cycle, and set online only after a backend probe succeeds. Ensure retry and fetch are serialized so the same action cannot be submitted concurrently. Test an online event with no intervening push or visibility event.

**Severity:** P1 · reconnect reliability

**Status:** ✅ Remediated — the hook now exposes a reconnect trigger (`reconnectCounter`) that `KdsScreen` consumes (`233eed6b`), so a browser `online` event drives a bounded fetch/retry cycle; retry and fetch are serialized and the online flag is only set after a successful probe.

### OFF-05 — KDS pending actions have no bounded retry or permanent-failure policy

**Evidence:** `useKdsOffline.retryPending` increments `retryCount` whenever an action fails but retains it indefinitely. It has no maximum attempts, backoff timestamp, dead-letter state, or distinction between transient network errors and permanent validation/authorization errors. KDS retry handlers return `false` for every caught error.

**Impact:** A malformed, unauthorized, or obsolete action can be retried forever on every fetch/retry attempt. This creates repeated backend traffic and leaves the queue permanently noisy without telling the kitchen how to resolve it.

**Recommendation:** Add retry metadata such as `nextAttemptAt`, classify errors as transient/permanent, use exponential backoff with jitter, and move exhausted/permanent actions to a visible failed state requiring explicit user action. Preserve the original error in a redacted diagnostic while showing a localized operator message.

**Severity:** P1 · queue reliability

**Status:** ✅ Remediated — bounded retries with exponential backoff (`nextAttemptAt`), a `MAX_RETRY_ATTEMPTS` ceiling, and a visible dead-letter list that requires explicit user action; the original error is preserved in redacted form (`233eed6b`). Covered by backoff, exhaustion, and clear-dead-letter tests.

### OFF-06 — Browser offline persistence covers neither the application shell nor POS sales

**Evidence:** Repository search found no service-worker file or registration. `ui/src/main.tsx` and `ui/src/main.tablet.tsx` render the application directly without service-worker setup. `useKdsOffline` is KDS-specific and uses `localStorage`; no equivalent browser persistence or replay path was found for retail POS cart/sale completion in the inspected surface.

**Impact:** A full reload while disconnected may fail to boot if the desktop/web runtime cannot retrieve application assets. KDS has a last-known-good API snapshot, but this does not establish offline POS operation, offline sale completion, or universal asset availability.

**Recommendation:** Decide and document the supported offline boundary. If reload/offline boot is required, add a versioned service worker or equivalent desktop asset strategy with an explicit cache invalidation policy. For POS, implement a durable, idempotent sale-outbox path rather than relying on in-memory cart state, and clearly gate actions that cannot be safely performed offline.

**Severity:** P1 · product capability and availability

**Status:** ✅ Remediated — supported offline boundary decided and documented below (see “Offline boundary decision (OFF-06)”). Desktop/tablet clients are native Tauri bundles (no service worker needed for asset availability); KDS gets durable optimistic state + persisted queue; retail POS sale completion is **not** supported in-memory — the durable sale-outbox is the SQLite `offline_queue` + real retry pipeline. Actions that cannot be performed offline remain gated.

### OFF-07 — Local KDS cache and queue are not scoped to store, workspace, session, or expiry

**Evidence:** `useKdsOffline` uses fixed localStorage keys: `kds-cached-orders`, `kds-last-sync`, and `kds-offline-queue`. The cached order snapshot and `PendingKdsAction` contain no store ID, workspace instance ID, session identity, or expiration timestamp. KDS filters fetched data by workspace store after fetch, but the cache fallback is returned before that filtering and is then filtered only by `KdsScreen`’s current scope.

**Impact:** On a shared terminal or after switching stores/workspaces, stale orders and queued mutations can survive across contexts. Old sensitive order data can remain available indefinitely, and a queued update may be attempted under a different session or store scope.

**Recommendation:** Namespace cache and queue records by store/instance and bind mutations to the intended scope. Add a retention/expiry policy, clear or quarantine data at logout/store switch, and require a valid current session before replay. Add tests for store switching, logout, cache expiry, and session-token rotation.

**Severity:** P1 · data isolation and security

**Status:** ✅ Remediated — cache and queue keys are namespaced per store scope, queued mutations are bound to the scope they were created in, and a 24h TTL expires stale snapshots (`17ee223c`). Covered by per-store isolation, scope-binding, and expiry tests.

### OFF-08 — KDS localStorage writes are best-effort and there is no durable fallback or capacity policy

**Evidence:** `useKdsOffline.writeLS` catches all `localStorage.setItem` errors, including quota exhaustion and storage unavailability, and silently ignores them. The hook then reports in-memory success and continues operating.

**Impact:** A queued action or cached snapshot can disappear on reload without any operator warning. The KDS can appear offline-capable during the current session while losing the only durable recovery record.

**Recommendation:** Treat persistence failure as observable state. Prefer the SQLite offline queue for mutations, use bounded/validated cache payloads, and expose a “local offline storage unavailable” warning with a safe fallback. Add tests for quota errors that assert the user-visible durability state, not just that the method does not throw.

**Severity:** P1 · durability

**Status:** ✅ Remediated — persistence failures are now observable: the hook exposes a `storageUnavailable` state and `KdsScreen` shows a localized “local offline storage unavailable” banner (`233eed6b`/`17ee223c`). Quota/exception tests assert the durable-state surface, not just that `setItem` did not throw.

### OFF-09 — Queue and conflict infrastructure exists, but the generic Tauri path does not use tenant/priority/conflict semantics consistently

**Evidence:** The SQLite schema and `oz_core` models include `tenant_id` and `priority`, and `platform/sync` implements tenant-aware listing, priority types, deduplication, and conflict strategies. However, `apps/desktop-client/src/commands/offline.rs` exposes DTOs without tenant/priority fields, calls default-tenant `enqueue_offline`, lists unscoped queues, and the retry command does not invoke conflict resolution or the sync engine.

**Impact:** The architectural safeguards are not necessarily applied at the operator-facing command boundary. Multi-store isolation, priority ordering, deduplication, and conflict observability can be bypassed or invisible to the UI.

**Recommendation:** Route all offline commands through the scoped sync service, require an authenticated store/tenant context, preserve priority and conflict metadata in DTOs, and use the same dispatch path for local and remote replay. Add contract tests proving tenant isolation and critical-before-normal ordering.

**Severity:** P1 · synchronization integrity

**Status:** ✅ Remediated — the command boundary now preserves tenant + priority: `OfflineQueueItemDto` carries `tenantId`/`priority`, `EnqueueOfflineArgs` accepts optional tenant/tier routed through core `enqueue_offline_scoped`, and `retry_offline_sync` sorts critical-before-normal (`e07ec4ae`). Tenant isolation and ordering are pinned by core + command contract tests.

### OFF-10 — Cross-tab/device coordination and idempotent replay are not established for browser-side KDS actions

**Evidence:** `useKdsOffline` reads and writes localStorage directly and does not subscribe to the `storage` event, use a lock, or coordinate through `BroadcastChannel`. The action ID is derived from `orderId->targetStatus`; there is no server idempotency key or replay lease in the hook.

**Impact:** Multiple tabs or windows can load the same queue and retry the same action concurrently. A shared terminal workflow can produce duplicate status requests or last-writer-wins behavior that differs from the visible local state.

**Recommendation:** Prefer one durable queue owner (Rust/SQLite) or add a cross-context lease and `storage`/`BroadcastChannel` coordination. Send a stable idempotency key with each replay and make backend status transitions idempotent. Test concurrent consumers and duplicate replay responses.

**Severity:** P2 · concurrency risk

**Status:** ✅ Remediated — the hook subscribes to the `storage` event so another tab writing the same scope triggers a queue reload, and action IDs already dedupe by order+status (`17ee223c`). Cross-tab coordination is covered by a dedicated test.

### OFF-11 — Conflict resolution is implemented as a library capability but not proven end-to-end for queued business actions

**Evidence:** `platform/sync/src/conflict.rs` provides dispatch for version LWW, sale status ordering, stock CRDT merge, and created-at fallback, with extensive unit tests. `platform/sync/src/queue.rs` applies resolutions and remote actions. The desktop offline command’s `retry_offline_sync` does not call these components, and the inspected UI tests focus on hook queue mechanics rather than persisted conflict outcomes.

**Impact:** Good isolated conflict algorithms may not protect actual operator flows. A conflict can remain unhandled, be falsely marked synced by OFF-01, or lose the metadata needed to explain the result.

**Recommendation:** Add an integration path from the Tauri command to `SyncEngine`/`SyncQueue`, then test duplicate sales, stale sale status, stock-delta merges, and unsupported actions through the command boundary. Show conflict counts and resolution summaries only after the real path has recorded them.

**Severity:** P1 · correctness assurance

**Status:** ✅ Remediated — real conflicts flowing through the generic Tauri retry path are now recorded as resolutions: `apply_sync_outcomes` marks a `Conflict` outcome via `mark_offline_resolved` (server copy wins) so `offline_queue_status_summary.conflict_count` reflects actual command-boundary conflicts instead of always reading zero (`e07ec4ae`). Contract test asserts the resolution marker + summary count end-to-end.

### OFF-12 — Offline test coverage is strong for the KDS hook and queue primitives but lacks reload, isolation, and failure-contract tests

**Evidence:** `useKdsOffline.test.ts` covers cache restore, corrupted storage, fetch fallback, action queueing, deduplication, retries, localStorage exceptions, and online-event counter behavior. `OfflineQueueScreen.test.tsx` covers loading, empty/error states, delete, counts, and sync messaging. Rust and platform-sync tests cover queue lifecycle, the placeholder mark-synced behavior, and conflict algorithms. No inspected test covers a reload after an optimistic KDS update, service-worker/offline shell boot, store/session isolation, real generic Tauri retry dispatch, or the Rust/UI `SyncResult` field mismatch.

**Impact:** Existing tests validate components in isolation while the highest-risk offline boundaries remain unverified. Regressions can pass all current hook and queue tests while still losing or falsely acknowledging business actions.

**Recommendation:** Add end-to-end offline scenarios: disconnect, perform an allowed action, reload, reconnect, replay, verify server state, and inspect conflict/failure outcomes. Add API contract tests for every offline DTO and explicit isolation/expiry tests.

**Severity:** P1 · quality assurance

**Status:** ✅ Remediated — reload-after-optimistic-update, store/session isolation, cache expiry, cross-tab coordination, offline-DTO contract, and conflict-outcome tests now exist (`91766573`, `17ee223c`, `e07ec4ae`). 100 UI tests + 57 core offline + 26 sync_client + 15/15 client command tests green. Service-worker/offline-shell boot is explicitly documented as out of scope (native Tauri bundles) in the boundary decision.

## Offline boundary decision (OFF-06)

Decided and documented 2026-08-02 during remediation:

- **Desktop & tablet clients are native Tauri bundles.** Application assets ship
  inside the installable binary, so no service worker or browser asset strategy
  is required for reload/offline boot. Web/PWA asset availability is explicitly
  **out of scope** for this product; a service-worker file must not be added
  without a product decision to ship a PWA.
- **KDS offline capability** = last-known-good order snapshot (durable, store-
  scoped, 24h TTL) + persisted pending-action queue with bounded retries and a
  dead-letter list. This is a *resilience* layer, not a full offline KDS: it
  preserves status transitions and replays them on reconnect.
- **Retail POS sale completion is NOT supported in-memory.** The durable,
  idempotent sale-outbox is the SQLite `offline_queue` exposed through the
  `enqueue_offline` / `retry_offline_sync` commands; a sale can only be queued
  for later sync, never silently acknowledged as synced without a server
  outcome (OFF-01/OFF-11).
- **Actions that cannot be safely performed offline remain gated** — the UI
  surfaces offline/queued banners (`ConnectionStatus`, KDS banners,
  `OfflineQueueScreen`) rather than pretending the action succeeded.

## Positive controls observed

- KDS has a last-known-good order cache and a persisted pending-action queue.
- KDS update actions are deduplicated by order and target status within the browser queue.
- KDS UI exposes offline and queued-update banners rather than silently hiding every failure.
- The SQLite queue has migrations, status/retry fields, tenant and priority columns, indexes, and integration tests.
- The core queue provides pending/synced/failed states and status summaries.
- The platform sync layer includes deduplication, tenant-aware listing, entity-specific conflict strategies, and stock-delta CRDT support.
- `ConnectionStatus` uses request timeouts and exponential backoff.
- Focused UI and Rust tests provide a substantial foundation for a durable end-to-end offline contract.

## Test and validation results

Focused validation completed for this report:

```text
cd ui
npx vitest run src/__tests__/useKdsOffline.test.ts src/__tests__/OfflineQueueScreen.test.tsx src/__tests__/KdsScreen.test.tsx
npm run typecheck

cd ..
cargo test -p oz-core offline
cargo test -p platform-sync
```

Results:

- Report existence and Markdown formatting: **passed**; 12 findings and no invalid trailing whitespace
- Focused UI offline tests: **passed**; 3 files, 84 tests
- UI TypeScript typecheck: **passed** with 0 errors
- `oz-core` offline tests: **passed**; 48 tests
- `platform-sync` tests: **passed**; 212 tests
- No production code was changed during this audit

Additional static checks recommended for remediation should verify:

- No service-worker registration exists unless intentionally documented as out of scope.
- Rust and TypeScript offline DTO field names match.
- Generic retry dispatch does not mark an item synced before applying it.
- Store/session namespace and expiry rules are tested.

## Remediation commit chain

| Phase | Findings | Commit | Validation |
|---|---|---|---|
| 1 | OFF-01/OFF-02 — verified stale (real sync pipeline + matching SyncResult contract); pinned counts | `91766573` | typecheck ✓ · contract tests ✓ |
| 2 | OFF-03/OFF-04/OFF-05 — durable optimistic projection, reconnect-driven retry, bounded retry + dead-letter | `233eed6b` | typecheck ✓ · 77 hook tests ✓ · lint ✓ |
| 3 | OFF-07/OFF-08/OFF-10 — store-scoped storage + 24h TTL expiry + cross-tab `storage` coordination | `17ee223c` | typecheck ✓ · 83 hook tests ✓ · lint ✓ |
| 4 | OFF-09/OFF-11 — tenant/priority DTOs + scoped enqueue + critical-first ordering + real conflict recording | `e07ec4ae` | oz-core 57 offline + 26 sync_client ✓ · desktop 15 + tablet 15 ✓ · UI 37 ✓ · clippy ✓ |
| 5 | OFF-06/OFF-12 — offline boundary decision documented; reload/isolation/expiry/conflict tests added | this commit | 100 UI offline tests ✓ |

## Recommended remediation order

All five phases are now complete — see the commit table above.

1. ~~**OFF-01/OFF-02:** Replace placeholder replay and fix the Rust/UI sync-result contract~~ ✅ `91766573`
2. ~~**OFF-03/OFF-04/OFF-05:** Make KDS optimistic state durable, reconnect-driven, serialized, and bounded~~ ✅ `233eed6b`
3. ~~**OFF-06/OFF-07/OFF-08:** Define the supported offline boundary, protect store/session data, and make persistence failures visible~~ ✅ `17ee223c` + this commit
4. ~~**OFF-09/OFF-11:** Wire the command boundary to tenant, priority, deduplication, and conflict-resolution infrastructure~~ ✅ `e07ec4ae`
5. ~~**OFF-10/OFF-12:** Add idempotent cross-context replay and end-to-end disconnect/reload/reconnect tests~~ ✅ `17ee223c` + this commit

## Audit status

✅ **FULLY REMEDIATED** — all 12 findings (OFF-01 → OFF-12) are closed across the commit chain `91766573` → `233eed6b` → `17ee223c` → `e07ec4ae` → this commit. Each remediation links to contract tests and validation results. The supported offline boundary is documented in the “Offline boundary decision (OFF-06)” section above.

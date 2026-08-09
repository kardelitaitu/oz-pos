# Phase 4 — Local Sync End-to-End Verification

**Date:** 2026-08-09
**Scope:** Docker sync API, debug Tauri process, and persisted sync command
**Status:** Verified with one live-client boundary documented

## Verification performed

The existing local stack was not restarted because it is shared with the running debug client.

- `GET /api/v1/health` returned HTTP 200.
- `GET /health` reported SQLite connected and zero pending queue items.
- `POST /api/v1/tokens` issued a short-lived diagnostic token without recording its value.
- Authenticated `GET /api/sync/status` returned HTTP 200.
- Authenticated empty `POST /api/sync/push` returned `results: []`.
- Authenticated empty `POST /api/sync/pull` returned an empty page with `next_cursor: null`.
- Unauthenticated `/api/sync/status` correctly returned HTTP 401.
- A debug `oz-pos-app.exe` process and the Vite dev server remained running on the existing local listeners.
- The new desktop regression test passed: persisted URL, API key, and enabled settings are consumed by the real `sync_run` command, which reports a successful empty-queue result.

## Regression contract

The test protects the bootstrap boundary after auto-provisioning: the command reads the Tauri settings database rather than relying on a UI copy of the configuration, and it returns an explicit `SyncAttemptResult` instead of silently no-oping.

## Remaining boundary

The live Tauri process was not driven through a native IPC call to create a non-empty local queue item. The available safe probes verified the server and command contracts without mutating the shared server queue. A future harness should launch an isolated Tauri profile or inject a test AppState, enqueue one local item, run `sync_run`, and assert the server-side accepted outcome.

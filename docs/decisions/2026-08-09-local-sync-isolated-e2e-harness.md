# Isolated Local Sync End-to-End Harness

**Date:** 2026-08-09
**Scope:** Tauri `sync_run` command and HTTP push boundary
**Status:** Implemented

## Coverage

The desktop sync test now creates an isolated in-memory AppState and a temporary loopback HTTP server. It:

1. Persists URL, API key, and enabled settings.
2. Enqueues one pending offline item.
3. Invokes the real `sync_run` command.
4. Returns a server `accepted` outcome from `POST /api/sync/push`.
5. Verifies the bearer header and request path.
6. Verifies the local queue item transitions to `synced` and the command reports one successful item.

The test does not depend on Docker, the running Tauri process, a fixed port, or shared local databases. The server binds to an ephemeral loopback port and is shut down when the test task completes.

## Verification

- Isolated desktop harness: **2 sync-run tests passed** (empty persisted queue and one accepted item).
- Rust formatting passed.
- The existing `save_topology_json` dead-code warning remains unrelated to this harness.
- Temporary Cargo target artifacts were removed after verification.

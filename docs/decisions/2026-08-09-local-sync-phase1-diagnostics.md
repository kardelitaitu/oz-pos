# Phase 1 — Local Docker and Dev Tauri Sync Diagnostics

**Date:** 2026-08-09
**Scope:** Local Docker sync server and debug Tauri bootstrap
**Status:** Diagnostic complete; no server-side failure reproduced

## Evidence

- Docker Compose services are running and healthy:
  - `pos-cloud-server` on port `3099`
  - `license-server` on port `8080`
  - `redis` on port `6379`
- `GET http://localhost:3099/api/v1/health` returned HTTP `200` with `{"status":"ok","version":"0.0.25"}`.
- `GET http://localhost:3099/health` returned HTTP `200`, with SQLite connected and zero pending sync items.
- `POST http://localhost:3099/api/v1/tokens` succeeds when the required `label` field is supplied. An empty JSON object correctly returns `422`; this is request validation, not a Docker failure.
- An issued token successfully authenticated:
  - `GET /api/sync/status` returned HTTP `200`.
  - `POST /api/sync/push` with an empty batch returned HTTP `200` and `results: []`.
  - `POST /api/sync/pull` with no cursor returned HTTP `200` and an empty page.
  - The unauthenticated status control returned HTTP `401` as expected.
- A debug `oz-pos-app.exe` process and the Vite dev server are running locally; port `1420` is listening for the dev frontend.
- Recent `pos-cloud-server` logs contain no startup, migration, database, or request errors. The containers have been running since their last healthy startup.

## Conclusion

The first failing seam is **not** Docker health, token issuance, authentication, or the sync HTTP API. The remaining unverified seam is inside the running Tauri client: whether debug auto-provisioning persisted `http://localhost:3099`, the issued API key, and `enabled = true`, and whether `sync_run` is being triggered and reporting its result.

The token value was not recorded. No credentials or secrets are included in this diagnostic.

## Next phase

Inspect the Tauri-side settings and daemon lifecycle without restarting the existing process. Capture the result of `get_sync_settings`, `pending_sync_count`, and one explicit `sync_run`; then repair only the first failing client-side seam.

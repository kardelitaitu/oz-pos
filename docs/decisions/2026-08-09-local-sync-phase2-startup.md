# Phase 2 — Deterministic Local Sync Startup

**Date:** 2026-08-09
**Scope:** `scripts/start-local-sync.bat` and local Docker Compose startup
**Status:** Implemented

## Problem

The local launcher reported success immediately after `docker compose up -d --build`. That only proved Compose accepted the request; it did not prove that the cloud server had passed its healthcheck or that the token endpoint required by the debug Tauri bootstrap was usable. PostgreSQL mode also passed `--profile pg` without merging `docker-compose.pg.yml`, so it did not select the PostgreSQL override.

## Changes

- Validate the merged Compose configuration before starting containers. Missing `OZ_API_SECRET` or `OZ_LICENSE_PRIVATE_KEY` now fails before a misleading startup-success message; PostgreSQL mode also validates `PG_PASSWORD` through the explicit override.
- Keep the default path on SQLite with the existing `docker compose up` command.
- Merge `docker-compose.yml`, `docker-compose.override.yml`, and `docker-compose.pg.yml` explicitly for `--pg`.
- Wait for both `/api/v1/health` and `POST /api/v1/tokens` to succeed before reporting readiness. The readiness token response is discarded and no credential is printed.
- Include the selected API port in readiness and success messages when `OZ_API_PORT` is supplied in the process environment.

## Verification

- Existing local Compose configuration passes `docker compose config --quiet`.
- The running stack passed health, token issuance, authenticated sync status, push, and pull probes during Phase 1.
- The launcher remains Windows-only and was verified by static inspection; it was not executed because execution would rebuild or restart the shared Docker stack.

## Follow-up

Phase 3 should expose the Tauri-side persisted sync settings and explicit `sync_run` result so a healthy server cannot appear idle when client bootstrap or daemon scheduling is the actual failure.

# Phase 3 — Tauri Sync Diagnostics

**Date:** 2026-08-09
**Scope:** Debug Tauri sync invocation and UI hydration
**Status:** Implemented

## Problem

The shared `useCloudSync` hook refused to call the real `sync_run` command when its localStorage copy of the server URL was empty. Debug auto-provisioning writes the URL and API key to the Tauri settings database, so this stale UI guard could make a working persisted configuration appear idle and hide the backend's explicit diagnostic result.

## Decision

`sync_run` is the authority for whether sync is configured, enabled, authenticated, and reachable. The hook now always delegates to `sync_run` when a run is requested. It continues to display the returned `synced`, `failed`, and `error` fields and refreshes the authoritative pending count. The localStorage URL remains UI hydration state and is still used for connection probes and destructive pulls, which require a user-supplied candidate URL.

## Regression coverage

The hook test now proves that an empty localStorage URL still invokes `sync_run`, accepts a successful persisted-backend result, updates the status to online, and emits the success toast.

This keeps the repair scoped: no credentials are exposed, no database schema changes are required, and the existing Tauri command contract remains unchanged.

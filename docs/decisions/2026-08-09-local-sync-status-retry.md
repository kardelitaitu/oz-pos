# Local Sync Status Retry

**Date:** 2026-08-09
**Scope:** Status-bar sync connectivity indicator
**Status:** Implemented

## Problem

The Docker sync server can become healthy after the debug Tauri window and status-bar hook start. The hook performed one immediate health check and then waited 60 seconds for the next poll, leaving the indicator red even after the server and auto-provisioning became ready.

## Decision

Keep the 60-second poll interval while connected, but retry every 5 seconds after a failed or disconnected check. This lets a startup race recover without restarting the app or Docker and does not change the backend sync contract.

The retry remains presentation-only: it calls the existing `test_sync_connection` command, never handles credentials, and stops cleanly on unmount.

## Regression coverage

The hook test simulates an initial `No server URL configured` result followed by a successful health check. It verifies the second check occurs after 5 seconds and the indicator transitions to connected with the reported latency.

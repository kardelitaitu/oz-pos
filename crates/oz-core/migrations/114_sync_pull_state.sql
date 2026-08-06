-- ── 114: Durable sync pull anchor + remote-item idempotency ledger (audit/09 SYNC-01) ──
-- The background sync daemon previously called pull_updates(None, None) on every
-- cycle, so the server returned the ENTIRE tenant queue and every cycle re-applied
-- previously-pulled stock/sale actions (double stock deduction = silent corruption).
--
-- This migration adds two tables:
--   1. sync_pull_state  — single-row store for the client's durable pull anchor
--      (`since` timestamp) and pagination `cursor`. The daemon persists this only
--      AFTER a page is applied successfully, so a crash mid-pull replays safely.
--   2. sync_applied_items — durable remote-item receipt ledger. Every remote item
--      id applied locally is recorded here; apply_remote skips rows already present,
--      so even a stale anchor or a server that ignores `since` cannot double-apply.
CREATE TABLE IF NOT EXISTS sync_pull_state (
    id         INTEGER PRIMARY KEY CHECK (id = 1),   -- single-row guard
    since      TEXT,                                  -- ISO-8601 anchor timestamp
    cursor     TEXT                                   -- opaque pagination cursor (P-3)
);

CREATE TABLE IF NOT EXISTS sync_applied_items (
    item_id    TEXT PRIMARY KEY,                     -- remote offline_queue item id
    action     TEXT NOT NULL,                        -- action applied (for diagnostics)
    applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_sync_applied_items_applied_at
    ON sync_applied_items(applied_at);

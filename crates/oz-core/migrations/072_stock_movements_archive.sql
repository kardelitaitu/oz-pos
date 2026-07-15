-- 072_stock_movements_archive.sql — Stock Movements Archive (ADR #6 Q4)
--
-- Creates the archive table for delta ledger pruning. Rows older than 90
-- days are copied here before being replaced by a consolidated rollup row
-- in the live `stock_movements` table.
--
-- The archive table has the same schema as stock_movements but no indexes
-- — it's append-only and queried infrequently (audit queries).
--
-- Also enables PRAGMA auto_vacuum = INCREMENTAL to reclaim disk space
-- after pruning DELETE operations.

CREATE TABLE IF NOT EXISTS stock_movements_archive (
    id                  TEXT PRIMARY KEY,
    item_id             TEXT NOT NULL,
    delta               INTEGER NOT NULL,
    reason              TEXT,
    source_terminal_id  TEXT,
    source_user_id      TEXT,
    store_id            TEXT NOT NULL DEFAULT '',
    created_at          TEXT NOT NULL
);

-- Enable incremental vacuum for the database. For EXISTING databases
-- upgrading through this migration, a one-time `VACUUM` must be run
-- outside of any transaction after the migration completes. New
-- databases (fresh installs) get incremental vacuum from the start
-- and do not need a separate VACUUM.
PRAGMA auto_vacuum = INCREMENTAL;

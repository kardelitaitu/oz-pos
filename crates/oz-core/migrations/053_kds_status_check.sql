-- ============================================================
-- Migration 053: Enforce valid KDS order statuses via CHECK
-- constraint on the kds_orders table.
--
-- SQLite does not support ALTER TABLE ADD CONSTRAINT, so we
-- recreate the table with the CHECK constraint, copy existing
-- data, and swap.
-- ============================================================

-- Create a new table with the CHECK constraint on status.
CREATE TABLE IF NOT EXISTS kds_orders_v053 (
    id              TEXT PRIMARY KEY,
    sale_id         TEXT NOT NULL UNIQUE REFERENCES sales(id),
    -- Valid states: pending (received, not started), preparing,
    -- ready (cooked, awaiting pickup), served (delivered),
    -- cancelled (voided by kitchen or POS).
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'preparing', 'ready', 'served', 'cancelled')),
    items_summary   TEXT NOT NULL DEFAULT '',
    item_count      INTEGER NOT NULL DEFAULT 0,
    display_number  INTEGER,
    received_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    started_at      TEXT,
    ready_at        TEXT,
    served_at       TEXT,
    prep_time_seconds INTEGER DEFAULT 0,
    notes           TEXT NOT NULL DEFAULT ''
);

-- Copy existing data. If any row has an invalid status the
-- INSERT will fail — but application-level validation via
-- KdsStatus::from_str() guarantees data integrity.
INSERT INTO kds_orders_v053 (
    id, sale_id, status, items_summary, item_count, display_number,
    received_at, started_at, ready_at, served_at, prep_time_seconds, notes
)
SELECT
    id, sale_id, status, items_summary, item_count, display_number,
    received_at, started_at, ready_at, served_at, prep_time_seconds, notes
FROM kds_orders;

-- Swap tables.
DROP TABLE kds_orders;
ALTER TABLE kds_orders_v053 RENAME TO kds_orders;

-- Recreate the daily counter table (unchanged, included for
-- completeness in the migration bundle).
CREATE TABLE IF NOT EXISTS kds_daily_counters (
    date        TEXT PRIMARY KEY,
    counter     INTEGER NOT NULL DEFAULT 0
);

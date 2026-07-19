-- 089_stock_summary_composite_pk.sql
-- ADR #18 §2c: `stock_summary` composite-PK rebuild — promotes
-- PRIMARY KEY from (item_id) to (item_id, location_id). This is the
-- heavy Phase 1 step that completes the location-scoped stock ledger:
-- without it, queries like `SELECT qty FROM stock_summary` returns
-- aggregated cross-location totals instead of per-location values.
--
-- Same PRAGMA + RENAME + CREATE + INSERT...SELECT + DROP + recreate-index
-- pattern as migration 081 (stock_transfers rebuild) and §2a's verbatim
-- ADR form. The on-delete chain note from §9c applies identically here:
--
--   ON DELETE RESTRICT on stock_summary.location_id chains through to
--   inventory_locations. A location with summary rows cannot be hard-
--   deleted. The 'default' and 'transit' canonical UUIDs (migration 078)
--   are seeded is_active=1 forever; legacy data is safe.
--
-- Why this is independent from §2a inventory rebuild (still deferred):
--
--   Migration 079 left `inventory` in ADD COLUMN form (one row per
--   (product, location) — well, actually one row per product × DEFAULT
--   location because the canonical UUID default applies to all legacy
--   product rows). §2a's full composite-PK rebuild is a separate
--   follow-up migration that requires Rust-side query refactor
--   (`list_products` GROUP BY — per ADR §2a's callout). That's deferred.
--
--   §2c (this migration) rebuilds `stock_summary` independently — the
--   CRDT delta ledger's materialised aggregate. The two can be
--   sequenced independently.
--
-- Insertion backfill: stock_summary_old has NO location_id column
-- (created in migration 063 prior to ADR §1). Every pre-migration row
-- maps to the canonical default-location UUID via literal projection
-- in the INSERT...SELECT.

PRAGMA foreign_keys = OFF;

ALTER TABLE stock_summary RENAME TO stock_summary_old;

CREATE TABLE stock_summary (
    item_id     TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    location_id TEXT NOT NULL DEFAULT '01926b3a-0000-7000-8000-000000000001'
                REFERENCES inventory_locations(id) ON DELETE RESTRICT,
    qty         INTEGER NOT NULL DEFAULT 0,
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (item_id, location_id)
);

INSERT INTO stock_summary (item_id, location_id, qty, updated_at)
SELECT item_id,
       '01926b3a-0000-7000-8000-000000000001',    -- canonical default-location UUID (§13-36)
       qty,
       updated_at
FROM stock_summary_old;

DROP TABLE stock_summary_old;

-- Per-location index — non-overlapping with the implicit (item_id, location_id)
-- composite-PK index. ADR §13 finding 35 pattern: per-location audit queries
-- (`SELECT qty FROM stock_summary WHERE location_id = ?`) need this to skip
-- the cross-product scan.
CREATE INDEX IF NOT EXISTS idx_stock_summary_location
    ON stock_summary(location_id);

PRAGMA foreign_keys = ON;

-- 081_stock_transfers_received_partial.sql
-- ADR #18 §2d + §7: rebuild stock_transfers with location FK columns and
-- extended CHECK constraint to include 'received_partial' (post-decision
-- review §13 finding 34 — Critical severity).
--
-- This rebuild addresses three realities at once:
--
--   1. The migration 047 CHECK constraint
--      CHECK (status IN ('draft','pending','in_transit','received','cancelled'))
--      crashes on any INSERT with status = 'received_partial' (§7 step 6
--      describes partial receipt as a real-world flow).
--   2. The legacy free-text `source_location`/`destination_location` columns
--      become `_old` for backward-compatibility audit; new code never reads
--      them and downstream readers MUST use the FK columns (§2d).
--   3. New NOT NULL FK columns `source_location_id`/`destination_location_id`
--      point at `inventory_locations` with `ON DELETE RESTRICT` so a
--      location with in-flight transfers cannot be hard-deleted.
--
-- REBUILD PATTERN — POST v0.0.10 fix (criterion 19-1 batch):
--
-- The pre-v0.0.10 version of this migration used the classic SQLite
-- pattern `ALTER TABLE stock_transfers RENAME TO stock_transfers_old` →
-- `CREATE TABLE stock_transfers (...)` → `INSERT ... FROM stock_transfers_old`
-- → `DROP TABLE stock_transfers_old`. That pattern triggers SQLite's
-- implicit FK-reference renaming for the duration of the rename — every
-- `REFERENCES stock_transfers(id)` text in `sqlite_master` is silently
-- rewritten to `REFERENCES stock_transfers_old(id)`. After the DROP
-- `stock_transfer_lines.transfer_id` was left pointing at a non-existent
-- table name; subsequent `INSERT INTO stock_transfer_lines` panicked
-- at runtime with "no such table: main.stock_transfers_old".
--
-- The safe pattern: `CREATE TABLE stock_transfers_new (...)` →
-- `INSERT…SELECT FROM stock_transfers` → `DROP TABLE stock_transfers`
-- → `ALTER TABLE stock_transfers_new RENAME TO stock_transfers`. The
-- `RENAME INTO stock_transfers` step is on a NEW table that has never
-- had a different name, so SQLite has no FK-rewriting work to do, and
-- the `stock_transfer_lines.REFERENCES stock_transfers(id)` text is
-- untouched. See https://sqlite.org/lang_altertable.html#otheralter
-- for the documented limitation.
--
-- On the FK-chain concern:
--   * `stock_transfer_lines.transfer_id REFERENCES stock_transfers(id) ON DELETE CASCADE`
--     survives the rebuild because the new `stock_transfers.id` keeps the
--     same type (TEXT) and the same PRIMARY KEY constraint, and the FK
--     reference text in `sqlite_master` is never rewritten.
--   * `PRAGMA foreign_keys = OFF` is set around the rebuild so the
--     DROP TABLE of `stock_transfers` does not trigger cascades from
--     `stock_transfer_lines` (which FK-references the original table).
--   * `PRAGMA foreign_keys = ON` is restored at the end so subsequent
--     migration runs (and the production database) are back to FK-enforcing.
--
-- Insertion backfill: the ADR §7 form uses
--   `COALESCE(source_location_id, '01926b3a-…-001')` for the FK columns,
-- but the original `stock_transfers` table has NO `source_location_id`
-- column — that column is being created in this migration. The correct
-- backfill is to project the literal canonical default UUID into every
-- copied row. (See ADR §13 acceptance criterion 36 for the UUID-propagation
-- rationale.)

PRAGMA foreign_keys = OFF;

CREATE TABLE stock_transfers_new (
    id                     TEXT PRIMARY KEY,
    transfer_number        TEXT NOT NULL UNIQUE,
    status                 TEXT NOT NULL DEFAULT 'draft'
                           CHECK (status IN (
                               'draft',
                               'pending',
                               'in_transit',
                               'received',
                               'received_partial',
                               'cancelled'
                           )),
    source_location_old           TEXT,
    destination_location_old      TEXT,
    source_location_id        TEXT NOT NULL DEFAULT '01926b3a-0000-7000-8000-000000000001'
                           REFERENCES inventory_locations(id) ON DELETE RESTRICT,
    destination_location_id   TEXT NOT NULL DEFAULT '01926b3a-0000-7000-8000-000000000001'
                           REFERENCES inventory_locations(id) ON DELETE RESTRICT,
    source_terminal_id        TEXT REFERENCES terminals(id),
    destination_terminal_id   TEXT REFERENCES terminals(id),
    notes         TEXT NOT NULL DEFAULT '',
    created_by    TEXT NOT NULL REFERENCES users(id),
    received_by   TEXT REFERENCES users(id),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    sent_at       TEXT,
    received_at   TEXT,
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO stock_transfers_new (
    id, transfer_number, status,
    source_location_old, destination_location_old,
    source_location_id, destination_location_id,
    source_terminal_id, destination_terminal_id,
    notes, created_by, received_by,
    created_at, sent_at, received_at, updated_at
)
SELECT
    id, transfer_number, status,
    source_location, destination_location,    -- legacy text cols → _old audit-trail
    '01926b3a-0000-7000-8000-000000000001',   -- canonical default-location UUID (ADR §13-36)
    '01926b3a-0000-7000-8000-000000000001',
    source_terminal_id, destination_terminal_id,
    notes, created_by, received_by,
    created_at, sent_at, received_at, updated_at
FROM stock_transfers;

DROP TABLE stock_transfers;

ALTER TABLE stock_transfers_new RENAME TO stock_transfers;

-- Recreate migration 047's indexes that were lost in DROP TABLE.
-- (Pushing these into the CREATE TABLE body would also work; we keep them
-- separate so the index set is auditable in one place per migration.)
CREATE INDEX IF NOT EXISTS idx_stock_transfers_status
    ON stock_transfers(status);
CREATE INDEX IF NOT EXISTS idx_stock_transfers_created
    ON stock_transfers(created_at);

-- Per-location FK indexes — non-overlapping with the existing status/created_at
-- indexes above. These serve §7 audit queries ("all transfers from
-- Warehouse A today") which the ADR §13 acceptance criteria rely on.
CREATE INDEX IF NOT EXISTS idx_stock_transfers_source_location
    ON stock_transfers(source_location_id, created_at);
CREATE INDEX IF NOT EXISTS idx_stock_transfers_destination_location
    ON stock_transfers(destination_location_id, created_at);

PRAGMA foreign_keys = ON;

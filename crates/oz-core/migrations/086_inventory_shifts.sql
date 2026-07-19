-- 086_inventory_shifts.sql
-- ADR #18 §9d: Inventory Shifts — Accountability Window.
--
-- Per the ADR: "An inventory shift is bound to a specific location via
-- `location_id NOT NULL`. If a worker needs to work at a different location
-- (e.g., switching from Warehouse A to Warehouse B via the location picker),
-- they end their current shift and start a new one. This keeps the audit
-- trail clean — every transaction during a shift happened at a known location."
--
-- Bundles the new table + ALTER inventory_transactions.inventory_shift_id FK
-- per the migration 084 pattern (sibling table + ALTER). The FK on
-- inventory_transactions is NULLABLE per the ADR §9d "Transaction without
-- active shift" rule:
--
--   "The `inventory_shift_id` FK is nullable — if no active shift exists when
--    a transaction is created, the field remains `NULL`. The transaction is
--    still recorded in `stock_movements` and `inventory_transactions`, but
--    it won't be attributable to a shift."
--
-- Partial unique index enforcing "one active shift per (user, location)":
--
--   "A single inventory user may have at most ONE ACTIVE SHIFT PER LOCATION.
--    Cross-location active shifts ARE allowed (worker hops Warehouse A ↔ B
--    without ending either shift, per Section 9d 'one shift = one location')."
--
-- The partial index is `(user_id, location_id) WHERE status = 'active'` —
-- this preserves §9d's location-switching narrative. The §13 finding 32
-- post-decision-review amended the v1 (user_id)-only form which would have
-- contradicted §9d's "one shift = one location" invariant.
--
-- Why a separate `inventory_shifts` table (not reusing cashier `shifts`
-- from migration 021)? Cashier shifts carry cash-specific columns
-- (opening_balance_minor, closing_balance_minor, cash_difference_minor,
-- total_cash_minor, etc.) that are meaningless for warehouse workers.
-- A separate table keeps the domain clean and avoids NULL-filled rows.
-- An inventory worker can do inventory work without being on a POS terminal.

CREATE TABLE IF NOT EXISTS inventory_shifts (
    id          TEXT PRIMARY KEY,                              -- UUID v7
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    location_id TEXT NOT NULL REFERENCES inventory_locations(id) ON DELETE RESTRICT,
    terminal_id TEXT REFERENCES terminals(id),                -- nullable; terminal that opened the shift
    started_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ended_at    TEXT,                                          -- nullable; non-null when status = 'ended'
    status      TEXT NOT NULL DEFAULT 'active'
                CHECK (status IN ('active', 'ended')),
    notes       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Per-user shift history (dashboard "Budi's last 5 shifts" query).
-- Non-overlapping with the per-location index below.
CREATE INDEX IF NOT EXISTS idx_inv_shifts_user
    ON inventory_shifts(user_id, started_at);

-- Per-location shift log ("who was on duty at Warehouse A today" — the
-- §9d example query). Distinct leading column from user-starting.
CREATE INDEX IF NOT EXISTS idx_inv_shifts_location
    ON inventory_shifts(location_id, started_at);

-- Status filter for the §9d "active shifts lookup" pattern
-- `WHERE status = 'active' AND user_id = ? AND location_id = ?`.
-- Also speeds up shift-list dashboard filters "show only ended".
-- Distinct leading column from the compound user/location indexes.
CREATE INDEX IF NOT EXISTS idx_inv_shifts_status
    ON inventory_shifts(status);

-- §9d single-shift-per-location invariant (database-level enforcement).
-- The partial index ensures at most ONE row with status='active' exists
-- for any (user_id, location_id) pair. The "exactly one active" half of
-- the invariant (there MUST be at least one active if work is happening)
-- remains application-layer via the shift-open command. SQLite does not
-- enforce "at least one" without triggers; this index enforces "at most
-- one", which is the half that's blocking on FK inserts.
CREATE UNIQUE INDEX IF NOT EXISTS idx_inv_shifts_active_per_user_location
    ON inventory_shifts(user_id, location_id) WHERE status = 'active';

-- ── Link to inventory_transactions (§9d second half) ─────────────────
-- Nullable FK per the §9d text. Existing inventory_transactions rows
-- (migration 084) get NULL automatically since no DEFAULT is specified.
-- The application-layer shift-open prompt must enforce "no shifts without
-- inventory_shift_id set on subsequent transactions" via the inventory
-- workspace UI (a persistent ⚠ "No active shift" banner emerges when
-- the user dismisses the start-shift prompt until they end the shift).
ALTER TABLE inventory_transactions ADD COLUMN inventory_shift_id TEXT
    REFERENCES inventory_shifts(id);

-- Lookup all transactions for a single shift ("During this shift at
-- Warehouse A: 5 receives, 2 transfer-outs, 1 stock count" — §9d summary).
-- Partial index WHERE NOT NULL because early sessions before §9d had no
-- shift linkage; sparse initial population until §3 Rust API lands.
-- Non-overlapping with the 084 indexes on inventory_transactions
-- (location_id, staff_id, transfer_id).
CREATE INDEX IF NOT EXISTS idx_inv_tx_shift
    ON inventory_transactions(inventory_shift_id)
    WHERE inventory_shift_id IS NOT NULL;

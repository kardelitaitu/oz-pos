-- 084_inventory_transaction_audit.sql
-- ADR #18 §9a + §9b: Staff audit trail — every inventory operation grouped
-- as an `inventory_transaction` (a session) with `inventory_transaction_lines`
-- (the SKUs + qtys that moved in that session).
--
-- Bundled into one migration following the established pattern from
-- migration 047 (which combines stock_transfers + stock_transfer_lines as
-- sibling tables). §9a's table must exist before §9b's FK references it.
--
-- The audit trail answers: "which staff input incoming items to Warehouse A
-- on July 18?" and "show me everything Budi received in that shipment."
-- Links:
--   `inventory_transaction_lines.transaction_id` → `inventory_transactions.id` ON DELETE CASCADE
--   `inventory_transactions.location_id`       → `inventory_locations.id`    ON DELETE RESTRICT
--   `inventory_transactions.staff_id`          → `users.id`                  ON DELETE RESTRICT
--   `inventory_transactions.transfer_id`       → `stock_transfers.id`        (nullable)
--   `inventory_transactions.purchase_order_id` → `purchase_orders.id`        (nullable)
--
-- §9c (added by follow-up migration) will link `stock_movements` rows
-- back to the inventory_transaction that triggered them via
-- inventory_movements.inventory_transaction_id. Until that lands, the
-- trail is "session-level" only — session exists but individual ledger
-- deltas don't yet trace back to it. §9d (also follow-up) will add a
-- nullable inventory_shift_id FK that lets the audit query "who was on
-- duty at Warehouse A when these items were received?"

-- ── §9a inventory_transactions ──────────────────────────────────────────

CREATE TABLE IF NOT EXISTS inventory_transactions (
    id                TEXT PRIMARY KEY,                              -- UUID v7
    type              TEXT NOT NULL CHECK (type IN (
                          'receive',        -- goods received (from supplier or PO)
                          'transfer-out',   -- goods sent to another location
                          'transfer-in',    -- goods received from another location
                          'adjust',         -- manual stock correction
                          'count'           -- stock take / physical count
                      )),
    location_id       TEXT NOT NULL REFERENCES inventory_locations(id) ON DELETE RESTRICT,
    staff_id          TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    transfer_id       TEXT REFERENCES stock_transfers(id),            -- nullable; set for transfer types
    purchase_order_id TEXT REFERENCES purchase_orders(id),            -- nullable; set for PO receiving
    notes             TEXT NOT NULL DEFAULT '',
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Per-location audit queries: "everything that happened at Warehouse A
-- today" — index covers WHERE location_id = ? ORDER BY created_at DESC.
CREATE INDEX IF NOT EXISTS idx_inv_tx_location
    ON inventory_transactions(location_id, created_at);

-- Per-staff audit queries: "show me everything Budi did today" — the
-- dashboard's primary lookup. Non-overlapping with the location index.
CREATE INDEX IF NOT EXISTS idx_inv_tx_staff
    ON inventory_transactions(staff_id, created_at);

-- Optional transfer-id lookup: "find the audit session for transfer X".
-- Sparse index — most transactions are NOT transfer-related — but
-- cheap because the FK column has low-cardinality coverage.
CREATE INDEX IF NOT EXISTS idx_inv_tx_transfer
    ON inventory_transactions(transfer_id) WHERE transfer_id IS NOT NULL;

-- ── §9b inventory_transaction_lines ─────────────────────────────────────

CREATE TABLE IF NOT EXISTS inventory_transaction_lines (
    id               TEXT PRIMARY KEY,                               -- UUID v7
    transaction_id   TEXT NOT NULL REFERENCES inventory_transactions(id) ON DELETE CASCADE,
    sku              TEXT NOT NULL,
    product_name     TEXT NOT NULL DEFAULT '',
    qty              INTEGER NOT NULL CHECK (qty > 0),
    barcode_scanned  TEXT,                                           -- nullable; the barcode actually scanned
    sort_order       INTEGER NOT NULL DEFAULT 0
);

-- Lookup all lines for a session — the JOIN column for §9 query examples
-- like "show me everything Budi did today grouped by session".
CREATE INDEX IF NOT EXISTS idx_inv_tx_lines_tx
    ON inventory_transaction_lines(transaction_id);

-- Optional barcode traceability — when a scan happened, this column
-- preserves the literal scanned code so audit can match it to the SKU
-- that was filed (helpful when SKU resolution disagreed with the scan).
-- Cardinality is sparse (NULL most of the time); the implicit FK to
-- a barcode-resolution view is deferred to Phase 2.

-- 046_stock_transfers.sql — Stock transfers between terminals/stores.
--
-- Tracks inventory transfers between locations and terminals, including
-- the state machine: draft → pending → in_transit → received / cancelled.

CREATE TABLE IF NOT EXISTS stock_transfers (
    id            TEXT PRIMARY KEY,
    transfer_number TEXT NOT NULL UNIQUE,
    status        TEXT NOT NULL DEFAULT 'draft'
                  CHECK (status IN ('draft','pending','in_transit','received','cancelled')),
    source_location      TEXT,
    destination_location TEXT,
    source_terminal_id      TEXT REFERENCES terminals(id),
    destination_terminal_id TEXT REFERENCES terminals(id),
    notes         TEXT NOT NULL DEFAULT '',
    created_by    TEXT NOT NULL REFERENCES users(id),
    received_by   TEXT REFERENCES users(id),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    sent_at       TEXT,
    received_at   TEXT,
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_stock_transfers_status   ON stock_transfers(status);
CREATE INDEX IF NOT EXISTS idx_stock_transfers_created  ON stock_transfers(created_at);

CREATE TABLE IF NOT EXISTS stock_transfer_lines (
    id           TEXT PRIMARY KEY,
    transfer_id  TEXT NOT NULL REFERENCES stock_transfers(id) ON DELETE CASCADE,
    sku          TEXT NOT NULL,
    product_name TEXT NOT NULL DEFAULT '',
    qty          INTEGER NOT NULL CHECK (qty > 0),
    received_qty INTEGER NOT NULL DEFAULT 0 CHECK (received_qty >= 0)
);

CREATE INDEX IF NOT EXISTS idx_stock_transfer_lines_transfer_id
    ON stock_transfer_lines(transfer_id);

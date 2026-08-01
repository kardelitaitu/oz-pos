-- 112_stock_transfer_actor_ids.sql — decouple transfer audit actors from local auth FKs.
--
-- Session authentication is resolved from the global identity database, while
-- stock transfers live in per-store databases. The old created_by/received_by
-- REFERENCES users(id) constraints made a valid global user require a fake
-- local users/roles row before a transfer could be written. That duplicated
-- authentication data and could create misleading local identities.
--
-- Keep the actor IDs as immutable text audit references. Commands derive them
-- from the authenticated session; no client-supplied actor value is accepted.
-- SQLite requires a table rebuild to remove the two foreign-key clauses.

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
    source_location_old    TEXT,
    destination_location_old TEXT,
    source_location_id     TEXT NOT NULL DEFAULT '01926b3a-0000-7000-8000-000000000001'
                           REFERENCES inventory_locations(id) ON DELETE RESTRICT,
    destination_location_id TEXT NOT NULL DEFAULT '01926b3a-0000-7000-8000-000000000001'
                           REFERENCES inventory_locations(id) ON DELETE RESTRICT,
    source_terminal_id     TEXT REFERENCES terminals(id),
    destination_terminal_id TEXT REFERENCES terminals(id),
    notes                  TEXT NOT NULL DEFAULT '',
    created_by             TEXT NOT NULL,
    received_by            TEXT,
    created_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    sent_at                TEXT,
    received_at            TEXT,
    updated_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
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
    source_location_old, destination_location_old,
    source_location_id, destination_location_id,
    source_terminal_id, destination_terminal_id,
    notes, created_by, received_by,
    created_at, sent_at, received_at, updated_at
FROM stock_transfers;

DROP TABLE stock_transfers;
ALTER TABLE stock_transfers_new RENAME TO stock_transfers;

CREATE INDEX IF NOT EXISTS idx_stock_transfers_status
    ON stock_transfers(status);
CREATE INDEX IF NOT EXISTS idx_stock_transfers_created
    ON stock_transfers(created_at);
CREATE INDEX IF NOT EXISTS idx_stock_transfers_source_location
    ON stock_transfers(source_location_id, created_at);
CREATE INDEX IF NOT EXISTS idx_stock_transfers_destination_location
    ON stock_transfers(destination_location_id, created_at);

PRAGMA foreign_keys = ON;

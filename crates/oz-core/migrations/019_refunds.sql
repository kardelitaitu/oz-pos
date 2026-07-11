-- 019_refunds.sql
-- Refund/return tracking for completed sales.
-- Supports partial refunds — multiple refunds can reference the same sale.
-- Stock is NOT automatically restored; the inventory team handles that
-- via the existing inventory adjustment flow.

CREATE TABLE IF NOT EXISTS refunds (
    id              TEXT PRIMARY KEY,
    sale_id         TEXT NOT NULL REFERENCES sales(id) ON DELETE RESTRICT,
    total_minor     INTEGER NOT NULL CHECK (total_minor >= 0),
    currency        TEXT NOT NULL,
    reason          TEXT NOT NULL DEFAULT '',
    note            TEXT NOT NULL DEFAULT '',
    processed_by    TEXT NOT NULL,          -- user_id who processed the refund
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS refund_lines (
    id              TEXT PRIMARY KEY,
    refund_id       TEXT NOT NULL REFERENCES refunds(id) ON DELETE CASCADE,
    sale_line_id    TEXT NOT NULL,          -- FK to sale_lines.id (logical ref, no CASCADE)
    sku             TEXT NOT NULL,
    qty             INTEGER NOT NULL CHECK (qty > 0),
    unit_minor      INTEGER NOT NULL CHECK (unit_minor >= 0),
    line_minor      INTEGER NOT NULL CHECK (line_minor >= 0),
    currency        TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_refunds_sale_id ON refunds(sale_id);

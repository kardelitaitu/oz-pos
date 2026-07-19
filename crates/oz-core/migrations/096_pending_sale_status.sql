-- 096_pending_sale_status.sql — ADR-20: add 'pending' status and expiry
-- columns to the sales table for the three-phase sale lifecycle.
--
-- Current valid statuses (accumulated): 'active', 'completed', 'voided',
-- 'refunded'. SQLite cannot ALTER a CHECK constraint, so we rebuild.
-- Adds 'pending' to the CHECK constraint and new columns for the
-- stock-reservation-before-payment-capture flow.
--
-- New columns:
--   pending_expires_at  — ISO-8601 timestamp of when this pending sale
--                         auto-voids (set to NOW + 30 min).
--   payment_reference   — gateway transaction reference (nullable).
--   captured_at         — ISO-8601 timestamp of payment capture (nullable).
--
-- Columns already present from migration 008 (reused):
--   payment_method      — already exists, used by finalize_sale too.

PRAGMA foreign_keys = OFF;

CREATE TABLE sales_new (
    id                  TEXT PRIMARY KEY,
    total_minor         INTEGER NOT NULL,
    currency            TEXT NOT NULL,
    line_count          INTEGER NOT NULL CHECK (line_count >= 0),
    status              TEXT NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'pending', 'completed', 'voided', 'refunded')),
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    payment_method      TEXT,
    tendered_minor      INTEGER,
    discount_percent    INTEGER NOT NULL DEFAULT 0,
    discount_label      TEXT,
    user_id             TEXT,
    subtotal_minor      INTEGER NOT NULL DEFAULT 0,
    tax_total_minor     INTEGER NOT NULL DEFAULT 0,
    customer_id         TEXT REFERENCES customers(id),
    version             INTEGER NOT NULL DEFAULT 1,
    store_id            TEXT,
    deduction_locations TEXT,
    pending_expires_at  TEXT,
    payment_reference   TEXT,
    captured_at         TEXT
);

INSERT INTO sales_new (
    id, total_minor, currency, line_count, status, created_at, updated_at,
    payment_method, tendered_minor, discount_percent, discount_label,
    user_id, subtotal_minor, tax_total_minor, customer_id, version,
    store_id, deduction_locations,
    pending_expires_at, payment_reference, captured_at
)
SELECT
    id, total_minor, currency, line_count, status, created_at, updated_at,
    payment_method, tendered_minor, discount_percent, discount_label,
    user_id, subtotal_minor, tax_total_minor, customer_id, version,
    store_id, deduction_locations,
    NULL AS pending_expires_at,
    NULL AS payment_reference,
    NULL AS captured_at
FROM sales;

DROP TABLE sales;

ALTER TABLE sales_new RENAME TO sales;

CREATE INDEX IF NOT EXISTS idx_sales_created_at ON sales(created_at);
CREATE INDEX IF NOT EXISTS idx_sales_store_status
    ON sales(store_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sales_pending_expires ON sales(pending_expires_at)
    WHERE status = 'pending';

PRAGMA foreign_keys = ON;

-- 009_tax_rates.sql — tax rate configuration.
--
-- Stores named tax rates (e.g. "VAT 20%", "Sales Tax 8.25%") that
-- can be applied to products. Future migrations may add a junction
-- table for per-product tax assignments.

CREATE TABLE IF NOT EXISTS tax_rates (
    id          TEXT PRIMARY KEY,                            -- UUID v4
    name        TEXT NOT NULL,                               -- e.g. "Sales Tax"
    rate_bps    INTEGER NOT NULL CHECK(rate_bps >= 0),       -- basis points (e.g. 825 = 8.25%)
    is_default  INTEGER NOT NULL DEFAULT 0,                  -- 1 if this is the default rate
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_tax_rates_name ON tax_rates(name);

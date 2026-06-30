-- 006_currencies.sql — ISO-4217 currency and exchange rate tables.
--
-- This migration adds the `currencies` table for ISO-4217 metadata and
-- the `exchange_rate` table for multi-currency support. The `currencies`
-- table is seeded with common currencies via `oz-cli init-db`.

CREATE TABLE IF NOT EXISTS currencies (
    code            TEXT PRIMARY KEY,          -- ISO-4217 alpha-3, e.g. "USD"
    numeric_code    TEXT NOT NULL,             -- ISO-4217 numeric, e.g. "840"
    name            TEXT NOT NULL,             -- Display name, e.g. "US Dollar"
    minor_exponent  INTEGER NOT NULL DEFAULT 2, -- Decimal places, e.g. 2 for USD
    symbol          TEXT NOT NULL DEFAULT '',  -- Currency symbol, e.g. "$"
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_currencies_code ON currencies(code);

CREATE TABLE IF NOT EXISTS exchange_rates (
    id              TEXT PRIMARY KEY,
    from_currency   TEXT NOT NULL REFERENCES currencies(code),
    to_currency     TEXT NOT NULL REFERENCES currencies(code),
    rate            REAL NOT NULL,            -- Conversion rate (from * rate = to)
    source          TEXT NOT NULL DEFAULT 'manual', -- manual, api, etc.
    effective_date  TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (from_currency, to_currency, effective_date)
);

CREATE INDEX IF NOT EXISTS idx_exchange_rates_from ON exchange_rates(from_currency);
CREATE INDEX IF NOT EXISTS idx_exchange_rates_to   ON exchange_rates(to_currency);

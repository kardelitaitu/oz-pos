-- 021_shifts.sql — Shift management for cash reconciliation.
--
-- Tracks the open/close lifecycle of a cashier shift with opening and
-- closing cash balances, calculated expected cash from sales, and
-- breakdowns of total sales by payment method.

CREATE TABLE IF NOT EXISTS shifts (
    id                    TEXT PRIMARY KEY,
    user_id               TEXT NOT NULL REFERENCES users(id),
    terminal_id           TEXT REFERENCES terminals(id),
    opened_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    closed_at             TEXT,
    opening_balance_minor INTEGER NOT NULL DEFAULT 0,
    closing_balance_minor INTEGER,                     -- counted cash at close
    expected_cash_minor   INTEGER,                     -- opening + cash sales - cash payouts
    cash_difference_minor INTEGER,                     -- closing - expected (positive = over, negative = short)
    total_sales_minor     INTEGER NOT NULL DEFAULT 0,  -- total sales amount during shift
    total_cash_minor      INTEGER NOT NULL DEFAULT 0,  -- cash sales amount
    total_card_minor      INTEGER NOT NULL DEFAULT 0,  -- card sales amount
    total_other_minor     INTEGER NOT NULL DEFAULT 0,  -- other payment method sales
    total_voids_minor     INTEGER NOT NULL DEFAULT 0,  -- voided amount
    total_refunds_minor   INTEGER NOT NULL DEFAULT 0,  -- refunded amount
    notes                 TEXT NOT NULL DEFAULT '',
    status                TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'closed')),
    created_at            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_shifts_user_id ON shifts(user_id);
CREATE INDEX IF NOT EXISTS idx_shifts_status ON shifts(status);
CREATE INDEX IF NOT EXISTS idx_shifts_opened_at ON shifts(opened_at);

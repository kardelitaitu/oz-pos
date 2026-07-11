-- 023_cash_payouts.sql — Mid-shift cash removals (safe drops).
--
-- Tracks cash removed from the drawer during a shift (e.g. for bank drops,
-- manager pickups). These payouts reduce the expected cash calculation
-- at shift close: expected = opening + cash_sales - total_payouts.

CREATE TABLE IF NOT EXISTS cash_payouts (
    id          TEXT PRIMARY KEY,
    shift_id    TEXT NOT NULL REFERENCES shifts(id),
    amount_minor INTEGER NOT NULL CHECK(amount_minor > 0),
    reason      TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_cash_payouts_shift_id ON cash_payouts(shift_id);

-- Add total_payouts_minor column to shifts for storing aggregated payout total.
ALTER TABLE shifts ADD COLUMN total_payouts_minor INTEGER NOT NULL DEFAULT 0;

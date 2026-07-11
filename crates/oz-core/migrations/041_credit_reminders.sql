-- Add credit settlement tracking to the payments table.
-- `settled_at` and `settled_by` track when an admin settles a credit sale.

ALTER TABLE payments ADD COLUMN settled_at TEXT;
ALTER TABLE payments ADD COLUMN settled_by TEXT;

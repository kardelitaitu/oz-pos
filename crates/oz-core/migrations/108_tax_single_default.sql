-- 108_tax_single_default.sql — enforce at most one default tax rate (TAX-02).
--
-- The application already clears the previous default inside a transaction
-- (create/update), but the database previously had no invariant enforcing it.
-- This migration:
--   1. Normalises any legacy data that already violates the invariant (a
--      pre-transaction bug could leave two rows flagged default), keeping the
--      OLDEST default and clearing the rest.
--   2. Adds a partial unique index so SQLite itself rejects a second
--      is_default = 1 row, closing the concurrency/failure window.

UPDATE tax_rates
SET is_default = 0
WHERE is_default = 1
  AND id NOT IN (
    SELECT id FROM tax_rates
    WHERE is_default = 1
    ORDER BY created_at ASC
    LIMIT 1
  );

CREATE UNIQUE INDEX IF NOT EXISTS idx_tax_rates_single_default
  ON tax_rates(is_default)
  WHERE is_default = 1;

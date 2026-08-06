-- 109_tax_soft_delete.sql — soft-delete flag for tax rates (TAX-03).
--
-- Previously, deleting a tax rate issued a hard `DELETE`, which relied on
-- `ON DELETE CASCADE` for the product_taxes/category_taxes junction rows and
-- silently removed the rate's id from historical `sale_lines.tax_rate_id`
-- (which has no cascade). TAX-03 replaces this with a deliberate policy:
--
--   1. `is_active = 0` archives a rate instead of deleting it, so historical
--      sale lines keep a resolvable (though hidden) rate linkage for audit.
--   2. Archiving a rate that is still referenced by historical sales is
--      BLOCKED at the application layer (structured error) — receipts and
--      audit trails must keep their rate linkage.
--   3. Product/category junction rows are removed in the same transaction,
--      preserving the old cascade's "no dangling assignments" guarantee.
--
-- The flag defaults to 1, so pre-109 rows (all active) and future INSERTs
-- need no code change.

ALTER TABLE tax_rates ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;

CREATE INDEX IF NOT EXISTS idx_tax_rates_active ON tax_rates(is_active);

-- 132_product_attributes.sql — retail merchandising attributes (ADR #36 D1).
--
-- Adds brand, rack position code, notes, unit of measure (UOM), active status,
-- and the default supplier to `products`. `cost_minor` already exists
-- (migration 054) and is not re-migrated.
--
-- Semantics:
--   brand / rack_location / notes / unit  — free text, nullable (normalized
--     lookup tables are a later decision, same pattern as brand).
--   is_active                              — 1 = sellable/visible, 0 = retired.
--     Matches product_variants.is_active: hide without deleting, preserving
--     sales history.
--   default_supplier_id                    — nullable FK to suppliers (046);
--     the preferred supplier for prefill/reorder. Suppliers are local
--     purchasing data, so this column never syncs (ADR #36 D2).
--
-- Plain ADD COLUMN is safe: migration 117 already rebuilt `products` with the
-- full prior column set, and nullable/defaulted columns do not affect the
-- fresh-install-vs-upgrade fingerprint test.

ALTER TABLE products ADD COLUMN brand TEXT;
ALTER TABLE products ADD COLUMN rack_location TEXT;
ALTER TABLE products ADD COLUMN notes TEXT;
ALTER TABLE products ADD COLUMN unit TEXT;
ALTER TABLE products ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;
ALTER TABLE products ADD COLUMN default_supplier_id TEXT REFERENCES suppliers(id);

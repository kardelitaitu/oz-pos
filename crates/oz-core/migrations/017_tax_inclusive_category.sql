-- 017_tax_inclusive_category.sql — Inclusive/exclusive tax + per-category rates.
--
-- Adds the `is_inclusive` flag to each tax rate (1 = tax-inclusive pricing,
-- 0 = tax-exclusive pricing), and introduces a junction table for
-- category-level tax rate assignments as a convenience override.

ALTER TABLE tax_rates ADD COLUMN is_inclusive INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS category_taxes (
    category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    tax_rate_id TEXT NOT NULL REFERENCES tax_rates(id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (category_id, tax_rate_id)
);

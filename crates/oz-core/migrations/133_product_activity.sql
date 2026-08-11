-- 133_product_activity.sql — popularity signal ledger + materialized score (ADR #37 D3).
--
-- product_activity records acted-upon searches and product edit events. The
-- sales signal needs no rows: sale_lines is already the durable, synced
-- ledger. The materialized products.popularity_score is recomputed from this
-- ledger plus sale_lines by the formula in code (crates/oz-core popularity
-- module) — the ledger keeps history so the formula can be retuned later
-- without a migration.
--
-- Local-only: neither the ledger nor the score is ever synced (ADR #37 D4).

CREATE TABLE IF NOT EXISTS product_activity (
    id         TEXT PRIMARY KEY,
    sku        TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (event_type IN ('search', 'edit')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_product_activity_sku ON product_activity(sku);

ALTER TABLE products ADD COLUMN popularity_score REAL NOT NULL DEFAULT 0;

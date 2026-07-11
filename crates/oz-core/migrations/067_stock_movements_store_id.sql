-- 067_stock_movements_store_id.sql — Cross-Store Delta Routing (ADR #6)
--
-- Adds a `store_id` column to the `stock_movements` delta ledger so
-- the sync layer can route deltas to the correct store database.
--
-- In per-store SQLite databases, the DB file IS the scope, so the
-- column serves as a sanity check and sync routing key. Defaults to
-- empty string for existing rows (backward compatible).

ALTER TABLE stock_movements ADD COLUMN store_id TEXT NOT NULL DEFAULT '';

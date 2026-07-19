-- 085_stock_movements_inventory_transaction_fk.sql
-- ADR #18 §9c: link stock_movements deltas (and their archive mirror) to
-- the inventory_transaction session that triggered them.
--
-- Schema follows ADR §9c verbatim:
--
--   ALTER TABLE stock_movements ADD COLUMN inventory_transaction_id TEXT
--       REFERENCES inventory_transactions(id) ON DELETE RESTRICT;
--
-- NULLABLE COLUMNS (no DEFAULT clause, no NOT NULL):
--
--   * Existing rows in stock_movements predate §9a (migration 084) — they
--     have no associated audit session, so the column reads NULL. This
--     matches "before audit existed, there was nothing to audit" semantics.
--   * Legacy migration-seed rows from migration 063 (`reason = 'migration-seed'`)
--     also have NULL — they were created by the migration runner, not by
--     a staff-triggered session.
--   * Forward-compat: when the §3 Rust API lands
--     (`adjust_stock_at_location_with_reason_and_tx_id`), it populates this
--     column on every delta. Pre-§3 callers continue to write NULL.
--
-- ── On-delete chain note (added by post-decision review §13 consequences) ──
-- ON DELETE RESTRICT on stock_movements.inventory_transaction_id chains
-- transitively through to inventory_transactions.staff_id, which already
-- FK-references users(id) ON DELETE RESTRICT. Net effect:
--
--   A user with audit history cannot be hard-deleted from `users` without
--   first clearing ALL their inventory_transactions (with their
--   stock_movements cascade). The de-facto policy is therefore:
--
--   * De-provisioning a former staff member MUST soft-delete
--     (`UPDATE users SET is_active = 0`) rather than `DELETE FROM users`.
--   * The shift-open command must enforce `is_active = 1` when looking
--     up staff so pending shifts can't be opened by deactivated accounts.
--
-- This is the right policy for an audit-conscious system: hard-deleting a
-- staff record silently destroys the lineage of every receipt, transfer,
-- adjustment, and count they've ever logged. Soft-delete preserves the
-- audit trail indefinitely.
--
-- ── Archive mirror ───────────────────────────────────────────────────────
-- stock_movements_archive mirrors stock_movements' schema (same rows,
-- same columns) so audit queries against pruned data retain full session
-- linkage. The same column gets added to the mirror table.

ALTER TABLE stock_movements ADD COLUMN inventory_transaction_id TEXT
    REFERENCES inventory_transactions(id) ON DELETE RESTRICT;

ALTER TABLE stock_movements_archive ADD COLUMN inventory_transaction_id TEXT
    REFERENCES inventory_transactions(id) ON DELETE RESTRICT;

-- ── Indexes ──────────────────────────────────────────────────────────────
-- Partial index on (inventory_transaction_id) WHERE NOT NULL — the column
-- is sparsely populated (NULL for legacy rows and pre-§3 callers), so a
-- partial index avoids indexing the NULL rows. Leading column is
-- inventory_transaction_id for the §9c query pattern
-- "show all stock_movements for transaction X".
-- Non-overlapping with migration 063's per-item index
-- `idx_stock_movements_item ON (item_id, created_at)` and migration 080's
-- per-location index `idx_stock_movements_location_created (location_id, created_at)`;
-- each serves a distinct query axis.
CREATE INDEX IF NOT EXISTS idx_stock_movements_inventory_transaction_id
    ON stock_movements(inventory_transaction_id) WHERE inventory_transaction_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_stock_movements_archive_inventory_transaction_id
    ON stock_movements_archive(inventory_transaction_id) WHERE inventory_transaction_id IS NOT NULL;

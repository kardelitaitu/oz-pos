-- 092_rebuild_stock_summary_group_by_location.sql
-- ADR-19 §15 criterion 19-1: SQL-layer mirror of the Rust
-- `rebuild_stock_summary()` function (crates/oz-core/src/db/products.rs).
--
-- Migration 089 established the composite PRIMARY KEY
-- `(item_id, location_id)` on `stock_summary` via a 1:1 copy of
-- `stock_summary_old` → `stock_summary` with the canonical default UUID
-- projected into `location_id`. That backfill is mechanically correct
-- because pre-089 stock_summary rows all descended from the global pool
-- (single location_id value: the canonical default UUID).
--
-- However, the *runtime* rebuild performed by rebuild_stock_summary()
-- uses `GROUP BY item_id, location_id` — explicit aggregation across
-- both columns. This migration is the SQL-layer mirror: it performs an
-- authoritative delete-and-rebuild of stock_summary by aggregating the
-- delta ledger with the same GROUP BY, so a fresh install passes the
-- §9e-iii low_stock_alerts_at_location per-location vector query even
-- before any Rust code runs.
--
-- Why an EXTRA migration (vs. inline 089 GROUP BY):
--
--   * Migration 089 owns the schema rebuild — its purpose is the
--     RENAME → CREATE → INSERT…SELECT → DROP pattern. Adding explicit
--     aggregation there would couple the schema change to live data,
--     which is dangerous because migrations are replayed on production
--     databases with arbitrary delta state.
--
--   * This 092 migration separates the explicit aggregation step into
--     its own idempotent unit. A fresh install applies 089 then 092
--     and ends in the per-location aggregated state. A production
--     install with existing stock_movements also applies 092 cleanly
--     (DELETE+INSERT with SUM is unconditionally correct).
--
--   * Idempotent: re-running 092 on a snapshot DB that has already
--     aggregated stock_summary produces the same end state (all
--     deduped via the composite PK upsert). No silent drift.
--
--   * Test surface is unaffected: 089's downstream tests (low_stock_alerts
--     per-location and rebuild_stock_summary's own cargo tests) both
--     pass after 092 because they read from `stock_summary` directly.

-- ─── MUST RUN INSIDE A TRANSACTION ──────────────────────────────────────
-- This migration's DELETE-FROM-Stock-Summary + INSERT-INTO-Stock-Summary
-- sequence leaves stock_summary EMPTY in the gap between the two
-- statements. The migration framework's `cached_sql()` builder wraps each
-- migration in BEGIN/COMMIT (see crates/oz-core/src/migrations.rs), so
-- during a normal `migrations::run()` call the gap is invisible to other
-- connections. **If a sysadmin runs this SQL outside the migration
-- runner (e.g., ad-hoc `sqlite3` shell session or manual admin script),
-- reads of stock_summary from other connections will see qty=0 between
-- the DELETE and INSERT.** Always run via the migration runner.

-- 1. Drop the post-089 1:1 copies so the rebuilt aggregate will land
--    cleanly on the composite PRIMARY KEY (item_id, location_id).
DELETE FROM stock_summary;

-- 2. Insert aggregated (item_id, location_id, SUM(delta)) groups. Every
--    stock_movements row has a location_id post-080 (NOT NULL DEFAULT
--    canonical UUID) so the GROUP BY produces per-location tuples —
--    including a (item_id, canonical-UUID) tuple for any pre-080 row's
--    original location (preserving 089's "all legacy stock funneled to
--    canonical default" invariant).
INSERT INTO stock_summary (item_id, location_id, qty, updated_at)
SELECT
    item_id,
    location_id,
    SUM(delta),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM stock_movements
GROUP BY item_id, location_id;

-- 3. Mirror the Rust rebuild_stock_summary()'s inventory zero-out step.
--    The runtime rebuild zeroes `inventory.qty` for products whose
--    `stock_movements` SUM(delta) ≤ 0 (over-sold items, fully-sold items).
--    The migration must reproduce this at the SQL layer or a fresh install
--    would have inventory rows showing positive qty for products whose
--    ledger has been fully consumed — silent drift between the two
--    layers. Builds the HAVING-clause subquery defensively (no rows on a
--    pristine fresh install where SUM(delta) is always ≥ 0).
UPDATE inventory
   SET qty = 0,
       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
 WHERE product_id IN (
       SELECT item_id
         FROM stock_movements
        GROUP BY item_id
       HAVING SUM(delta) <= 0
   );

-- 4. Index refresh — recreate the per-location index from 089 to ensure
--    the query planner uses it after the rebuild. (Already created by
--    089; the IF NOT EXISTS is defensive against earlier 089 failures
--    that may have left the table without the index.)
CREATE INDEX IF NOT EXISTS idx_stock_summary_location
    ON stock_summary(location_id);

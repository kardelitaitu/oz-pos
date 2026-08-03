-- 116_setting_updated_unique_version.sql
-- DB-08: enforce per-(key, terminal_id) version uniqueness at the DB layer.
--
-- The setting_updated delta ledger (migration 100) documents version
-- allocation as MAX(version) + 1 in application code. Nothing at the
-- database layer prevented two concurrent writers from inserting the same
-- version for the same (key, terminal_id) pair — duplicate versions were
-- silently accepted. This migration:
--
--   1. Collapses any existing duplicate (key, terminal_id, version) rows,
--      keeping the highest `id` (the most recently written row) per group.
--   2. Adds a UNIQUE index so future duplicate versions fail closed at the
--      database boundary instead of corrupting the delta ledger.
--
-- The application writer (platform-core settings `write_delta` /
-- `write_delta_on_tx`) computes MAX(version) + 1 inside a savepoint/
-- transaction; the UNIQUE index turns the concurrent-writer race into an
-- explicit constraint error rather than a silent duplicate. `set_tracked`
-- already treats delta-write failures as non-fatal (logged), so a contended
-- write surfaces as a logged conflict, never a corrupted ledger.

-- 1. Collapse existing duplicates: keep the newest row per version group.
DELETE FROM setting_updated
WHERE id NOT IN (
    SELECT MAX(id) FROM setting_updated GROUP BY key, terminal_id, version
);

-- 2. Fail closed on future duplicate versions.
CREATE UNIQUE INDEX IF NOT EXISTS idx_setting_updated_unique_version
    ON setting_updated(key, terminal_id, version);

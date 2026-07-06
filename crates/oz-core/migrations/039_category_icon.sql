-- Migration 039: add icon column to categories
--
-- Stores a short icon identifier (e.g. "dots-1", "dots-2", "dots-3") for each
-- category. Existing rows default to an empty string which the UI renders as
-- "no icon" (falls back gracefully).

ALTER TABLE categories ADD COLUMN icon TEXT NOT NULL DEFAULT '';

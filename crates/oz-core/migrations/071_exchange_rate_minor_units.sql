-- 071_exchange_rate_minor_units.sql
--
-- Migrate `exchange_rates.rate` from REAL (`f64`) to INTEGER fixed-point
-- at 6 decimal places (`rate_millionths`). Closes audit finding C-1
-- referenced in `docs/specs/_active/2026-07-12-desktop-app-audit.md`.
--
-- Why: exchange rates feed every multi-currency checkout multiplier; an
-- `f64` source contaminates every downstream Money calculation and the
-- `<= 0` validation is sign-unstable near zero. Integer fixed-point is
-- the same correctness posture the rest of OZ-POS already enforces via
-- `Money.minor_units: i64`.
--
-- Scale: 1_000_000 = `rate_millionths = rate * 1_000_000`. The factor 6 covers
-- every fixture in the existing test suite including the extreme
-- 0.00025 (JPY→KWD) case; the largest single-leg rate observed in tests is
-- ~150 (USD→JPY), which sits comfortably inside i64 (max 9.2e18).
--
-- Sequence:
--   1. ADD COLUMN with DEFAULT 0 so existing rows remain valid
--      (SQLite drops the DEFAULT after the UPDATE for new rows unless
--      preserved; we preserve it for safety against partial migrations).
--   2. Backfill from the legacy `rate REAL` column using ROUND(rate * 1e6)
--      to avoid fp→integer off-by-one errors.
--   3. DROP the legacy column. Application code is updated in the same
--      atomic commit so the column is never queried post-migration.
--
-- Rollback path: revert application code to read/write `rate` f64, then
-- ALTER TABLE exchange_rates ADD COLUMN rate REAL; UPDATE ... rate=rate_millionths/1e6; DROP COLUMN rate_millionths.

ALTER TABLE exchange_rates ADD COLUMN rate_millionths INTEGER NOT NULL DEFAULT 0;

UPDATE exchange_rates
   SET rate_millionths = CAST(ROUND(rate * 1000000) AS INTEGER);

ALTER TABLE exchange_rates DROP COLUMN rate;

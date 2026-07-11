-- 065_version_optimistic.sql — Optimistic concurrency version columns (ADR #6).
--
-- Each row in `products` and `sales` gets a monotonically-increasing
-- `version` integer. Writers increment it on UPDATE; readers compare it
-- before writing. If two writers race, one gets 0 rows affected and
-- retries or returns a conflict error.

-- products: add version column (existing rows default to 1).
ALTER TABLE products ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

-- sales: add version column (existing rows default to 1).
ALTER TABLE sales ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

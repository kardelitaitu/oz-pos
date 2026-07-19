-- 097_payment_idempotency_keys.sql — ADR-20 §5: idempotency key support
-- for payment gateway requests to prevent duplicate charges on retry.
--
-- Generates a UUIDv7 idempotency key before every payment gateway request.
-- The key is stored with the payment record so retries with the same key
-- are detected and prevented at the database layer.
--
-- SQLite allows NULLs in UNIQUE indexes (multiple NULLs are not equal),
-- so the index only enforces uniqueness for non-NULL keys.

ALTER TABLE payments ADD COLUMN idempotency_key TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_payments_idempotency_key ON payments(idempotency_key);

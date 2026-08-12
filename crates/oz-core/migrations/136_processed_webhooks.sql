-- 136_processed_webhooks.sql
-- Tracks webhook event IDs to prevent duplicate processing.
-- Stripe (and other providers) may redeliver webhooks; this table
-- ensures idempotent handling by recording each event ID after
-- successful processing.
CREATE TABLE IF NOT EXISTS processed_webhooks (
    event_id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,   -- 'stripe' or 'square'
    received_at TEXT NOT NULL DEFAULT (datetime('now')),
    event_type TEXT           -- e.g. 'payment_intent.succeeded'
);

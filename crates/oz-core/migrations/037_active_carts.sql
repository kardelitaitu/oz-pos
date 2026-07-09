-- 037_active_carts.sql
-- Persist active (in-progress) carts so they survive restarts.
--
-- Each cart is serialised as JSON in `cart_data` using the existing
-- `Cart` serde impl from the `foundation` crate.  This keeps the
-- persistence layer simple — no need to replicate the in-memory cart
-- schema in SQL.

CREATE TABLE IF NOT EXISTS active_carts (
    id              TEXT PRIMARY KEY NOT NULL,
    cart_data       TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_active_carts_updated_at ON active_carts(updated_at DESC);

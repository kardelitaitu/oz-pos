-- 087_stock_thresholds_alerts.sql
-- ADR #18 §9e: Configurable per-product / per-location stock-threshold
-- alerts with acknowledgment lifecycle. Bundles §9e-i (configuration) +
-- §9e-ii (triggered events) per the migration 084/086 pattern (sibling
-- tables that reference each other ship atomically).
--
-- Why two tables in one migration:
--
--   * §9e-i `stock_thresholds` defines WHERE alerts FIRE (config)
--   * §9e-ii `stock_alert_events` records WHEN alerts FIRED, WHEN they
--     got ACKNOWLEDGED, and WHEN they got RESOLVED (lifecycle).
--   * §9e-ii.alert_events.threshold_id → §9e-i.thresholds.id
--     with ON DELETE CASCADE. If a threshold is deleted, the events
--     that were calibrated by it decay with it.
--   * §9e-ii.alert_events.status drives "is this alert actionable?"
--     `active` shows on the badge; `acknowledged` is dashboard-visible
--     but stop nagging; `resolved` is archived.
--
-- ── §9e-i stock_thresholds ──────────────────────────────────────────
--
-- Scoping rules (per ADR §9e-i text):
--   (product_id, location_id) = ('CHA-001', 'wh-a')  → CHA-001 at Warehouse A
--   (product_id, location_id) = ('CHA-001', NULL)     → CHA-001 at any location
--   (product_id, location_id) = (NULL,    'wh-a')     → NOT allowed (always product-scoped)
--   No row at all                                         → system default threshold applies

CREATE TABLE IF NOT EXISTS stock_thresholds (
    id          TEXT PRIMARY KEY,                              -- UUID v7
    product_id  TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    location_id TEXT REFERENCES inventory_locations(id) ON DELETE CASCADE,
    threshold   INTEGER NOT NULL CHECK (threshold >= 0),
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(product_id, location_id)
);

-- Per-product threshold lookup: "what's CHA-001 calibrated to?"
CREATE INDEX IF NOT EXISTS idx_stock_thresholds_product
    ON stock_thresholds(product_id);

-- Per-location threshold lookup: "show me all thresholds at Warehouse A."
CREATE INDEX IF NOT EXISTS idx_stock_thresholds_location
    ON stock_thresholds(location_id);

-- SQLite's UNIQUE(product_id, location_id) constraint treats NULLs as
-- DISTINCT — which means without this partial index, two rows of
-- `(product_id='CHA-001', location_id=NULL)` would be permitted when
-- only one global threshold per product is intended per ADR §9e-i.
-- The WHERE clause excludes NULL location_id rows from the implicit
-- duplicate-allowed behavior.
CREATE UNIQUE INDEX IF NOT EXISTS idx_stock_thresholds_global
    ON stock_thresholds(product_id) WHERE location_id IS NULL;

-- ── §9e-ii stock_alert_events ───────────────────────────────────────
--
-- Captured `current_qty` and `threshold` AT the moment of trigger — they
-- must NOT be queried live (the JOIN would lie if either changed since
-- trigger). The lifecycle:
--
--   1. Stock drops below threshold (synchronous check inside the same
--      adjust_stock transaction per ADR §9e-ii "alert check is synchronous").
--   2. Check: any `active` event for this threshold_id? If yes, skip
--      (don't spam duplicates). If no, INSERT new event with status='active'.
--   3. Staff opens alert panel, acknowledges:
--      UPDATE stock_alert_events
--         SET status = 'acknowledged', acknowledged_at = now, acknowledged_by = user
--       WHERE id = ?.
--   4. Stock recovers above threshold (qty >= threshold) in a subsequent
--      adjust_stock: UPDATE status='resolved', resolved_at=now.

CREATE TABLE IF NOT EXISTS stock_alert_events (
    id              TEXT PRIMARY KEY,
    threshold_id    TEXT NOT NULL REFERENCES stock_thresholds(id) ON DELETE CASCADE,
    product_id      TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    location_id     TEXT REFERENCES inventory_locations(id) ON DELETE CASCADE,
    current_qty     INTEGER NOT NULL,                         -- qty at trigger time
    threshold       INTEGER NOT NULL,                         -- threshold value that was breached
    status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'acknowledged', 'resolved')),
    triggered_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    acknowledged_at TEXT,
    acknowledged_by TEXT REFERENCES users(id),
    resolved_at     TEXT
);

-- Active-alert dashboard query: "show me all active alerts ordered by
-- triggered_at DESC" — the §9e-iii example query.
-- Non-overlapping with the per-product index below.
CREATE INDEX IF NOT EXISTS idx_stock_alert_events_status
    ON stock_alert_events(status, triggered_at);

-- Per-product alert history: "all alerts ever fired for CHA-001 at
-- Warehouse A" — the resolution audit query. Includes location_id so
-- cross-location alert trail is queryable.
CREATE INDEX IF NOT EXISTS idx_stock_alert_events_product
    ON stock_alert_events(product_id, location_id);

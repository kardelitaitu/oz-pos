-- 105_kds_line_items.sql
-- KDS line items — structured per-item data for kitchen tickets.
--
-- Replaces the flat items_summary string on kds_orders with real rows
-- that carry per-item course, modifier, and status information.
--
-- Each row represents a single product line (sku + qty) within a KDS
-- ticket. Multiple rows with the same display_name and qty=1 represent
-- individual items on the ticket.
--
-- The course column enables course-grouped display (appetizer → main
-- → dessert) and future course-firing workflows.
-- The modifiers_json column carries per-item modifier choices.
-- The item_status column enables per-item status tracking (TODO 3e).

CREATE TABLE IF NOT EXISTS kds_line_items (
    id              TEXT PRIMARY KEY,                              -- UUIDv7
    kds_order_id    TEXT NOT NULL REFERENCES kds_orders(id) ON DELETE CASCADE,
    sku             TEXT NOT NULL,
    display_name    TEXT NOT NULL,                                 -- product name at creation time
    qty             INTEGER NOT NULL CHECK(qty > 0),
    course          TEXT,                                          -- NULL | "appetizer" | "main" | "dessert" | "beverage"
    modifiers_json  TEXT,                                          -- NULL | JSON array of { name, choice, price_minor }
    line_position   INTEGER NOT NULL DEFAULT 0,
    item_status     TEXT NOT NULL DEFAULT 'pending'
                    CHECK(item_status IN ('pending','preparing','ready','served','cancelled')),
    started_at      TEXT,
    ready_at        TEXT,
    served_at       TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_kds_line_items_order ON kds_line_items(kds_order_id, line_position);
CREATE INDEX IF NOT EXISTS idx_kds_line_items_status ON kds_line_items(kds_order_id, item_status);

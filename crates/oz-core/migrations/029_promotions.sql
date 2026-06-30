-- Promotions engine tables.

CREATE TABLE IF NOT EXISTS promotions (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    -- 'percentage', 'fixed_amount', 'buy_x_get_y'
    promo_type  TEXT NOT NULL,
    -- For percentage: value is the percent (e.g., 10 = 10% off)
    -- For fixed_amount: value is in minor units
    -- For buy_x_get_y: value is the discount % on the free item
    value_minor INTEGER NOT NULL DEFAULT 0,
    -- Buy-X-get-Y: minimum quantity the customer must buy
    min_qty     INTEGER,
    -- Buy-X-get-Y: product SKU that must be purchased
    trigger_sku TEXT,
    -- Buy-X-get-Y: product SKU that gets the discount (empty = same as trigger)
    reward_sku  TEXT,
    -- Buy-X-get-Y: how many reward items the customer gets
    reward_qty  INTEGER DEFAULT 1,
    -- Time-limited: optional start/end
    starts_at   TEXT,
    ends_at     TEXT,
    -- Minimum order total in minor units for the promotion to apply
    min_order_minor INTEGER DEFAULT 0,
    -- Which product category this applies to (empty = all products)
    category_id TEXT,
    -- Whether this promotion is currently active
    active      INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS promotion_applications (
    id          TEXT PRIMARY KEY,
    promotion_id TEXT NOT NULL REFERENCES promotions(id),
    sale_id     TEXT NOT NULL REFERENCES sales(id),
    discount_minor INTEGER NOT NULL,
    description TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

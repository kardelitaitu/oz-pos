-- Loyalty program: points, tiers, redemption.

-- Tiers define the name and earning multiplier.
CREATE TABLE IF NOT EXISTS loyalty_tiers (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    min_points  INTEGER NOT NULL DEFAULT 0,
    points_per_unit INTEGER NOT NULL DEFAULT 10,
    earn_multiplier REAL NOT NULL DEFAULT 1.0,
    colour      TEXT NOT NULL DEFAULT '#6b7280',
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Customer loyalty account: points balance + current tier.
CREATE TABLE IF NOT EXISTS loyalty_accounts (
    id          TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL UNIQUE REFERENCES customers(id),
    points      INTEGER NOT NULL DEFAULT 0,
    lifetime_points INTEGER NOT NULL DEFAULT 0,
    tier_id     TEXT REFERENCES loyalty_tiers(id),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Points transactions (earn, redeem, adjust).
CREATE TABLE IF NOT EXISTS loyalty_transactions (
    id          TEXT PRIMARY KEY,
    account_id  TEXT NOT NULL REFERENCES loyalty_accounts(id),
    sale_id     TEXT REFERENCES sales(id),
    points      INTEGER NOT NULL,
    txn_type    TEXT NOT NULL,
    description TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Seed default tiers: Bronze, Silver, Gold, Platinum.
INSERT OR IGNORE INTO loyalty_tiers (id, name, min_points, points_per_unit, earn_multiplier, colour, sort_order) VALUES
    ('tier-bronze',   'Bronze',   0,    10, 1.0, '#cd7f32', 1),
    ('tier-silver',   'Silver',   100,  10, 1.25, '#c0c0c0', 2),
    ('tier-gold',     'Gold',     500,  10, 1.5, '#ffd700', 3),
    ('tier-platinum', 'Platinum', 2000, 10, 2.0, '#e5e4e2', 4);

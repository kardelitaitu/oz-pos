-- 052_order_modifiers.sql — Order modifiers for restaurant menu items.
--
-- Modifiers allow customising menu items (e.g. "Extra Cheese", "No Onions",
-- "Well Done") with optional price adjustments and selection limits.
--
-- Each product can have multiple modifier groups attached, and each group
-- contains one or more modifier options. The POS UI enforces min/max
-- selections per group (e.g. "Choose up to 2 sides").

-- Modifier groups define a category of options (e.g. "Side Dish", "Doneness").
CREATE TABLE IF NOT EXISTS modifier_groups (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    min_selections INTEGER NOT NULL DEFAULT 0 CHECK (min_selections >= 0),
    max_selections INTEGER NOT NULL DEFAULT 1 CHECK (max_selections >= 1),
    sort_order     INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (max_selections >= min_selections)
);

-- Individual modifier options within a group (e.g. "Fries", "Salad" in "Side Dish").
CREATE TABLE IF NOT EXISTS modifiers (
    id             TEXT PRIMARY KEY,
    group_id       TEXT NOT NULL REFERENCES modifier_groups(id) ON DELETE CASCADE,
    name           TEXT NOT NULL,
    price_minor    INTEGER NOT NULL DEFAULT 0 CHECK (price_minor >= 0),
    sort_order     INTEGER NOT NULL DEFAULT 0,
    is_default     INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Links products to their available modifier groups.
CREATE TABLE IF NOT EXISTS product_modifier_groups (
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    group_id   TEXT NOT NULL REFERENCES modifier_groups(id) ON DELETE CASCADE,
    PRIMARY KEY (product_id, group_id)
);

CREATE INDEX IF NOT EXISTS idx_modifiers_group_id ON modifiers(group_id);
CREATE INDEX IF NOT EXISTS idx_product_modifier_groups_product ON product_modifier_groups(product_id);

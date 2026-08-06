-- 117_scoping_store_id_fk.sql
-- DB-04 end-state: store_id referential integrity (ADR #4 Phase 2 close-out)
--
-- Migration 069 added nullable `store_id` to products, sales, sale_lines, and
-- customers, but nothing at the database layer tied a non-NULL store_id to the
-- store catalog (store_profiles). A call bypassing the store repository — or a
-- legacy row carrying a store_id for a store that was later removed — could
-- reference a store that does not exist. Indexes improved lookup but did not
-- enforce the ownership invariant described by the schema comments.
--
-- This migration:
--
--   1. Quarantines orphaned store_ids: any non-NULL store_id that does not
--      exist in store_profiles is reset to NULL — the documented
--      "unscoped / legacy / global shared" sentinel from migration 069. The
--      reset happens inside the copy (CASE WHEN ... IN (SELECT id ...)), so
--      there is no window where an orphaned reference survives.
--   2. Rebuilds products, sales, sale_lines, and customers with
--      `store_id TEXT REFERENCES store_profiles(id) ON DELETE SET NULL
--      ON UPDATE CASCADE`, so every non-NULL store_id must reference a real
--      store profile from now on.
--   3. Explicitly preserves NULL = global. Filesystem-level per-store database
--      isolation is the primary scoping mechanism (see
--      docs/decisions/2026-07-10-workspace-type-instance-design.md — data is
--      isolated by the store-level database switch, not by query-level
--      WHERE store_id clauses); the 069 columns are the soft-scoping layer for
--      shared/cloud databases. Forcing NOT NULL here would break the per-store
--      DB model, where only the 'default' profile is seeded. NULL therefore
--      remains valid and is the explicit "global shared" state every caller
--      must honor (pinned by the migration_069_* tests).
--
-- ON DELETE SET NULL (not RESTRICT, not CASCADE): deleting a store profile
-- never blocks on historical domain rows and never destroys them — scoped rows
-- revert to the explicit global sentinel. This matches the existing
-- delete_store_profile flow, which is only blocked for the primary store.
--
-- The runner (DB-05) disables foreign_keys at the connection level outside the
-- migration transaction, so the DROP + RENAME rebuild below is safe even with
-- populated child tables (the same pattern as migrations 066/089/092/096).

-- ══════════════════════════════════════════════════════════════════
-- 1. Create replacement tables with the store_id FK
-- ══════════════════════════════════════════════════════════════════

CREATE TABLE customers_new (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    email           TEXT,
    phone           TEXT,
    loyalty_points  INTEGER NOT NULL DEFAULT 0,
    total_spent_minor INTEGER NOT NULL DEFAULT 0,
    currency        TEXT NOT NULL DEFAULT 'USD',
    notes           TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    store_id        TEXT REFERENCES store_profiles(id) ON DELETE SET NULL ON UPDATE CASCADE
);

CREATE TABLE products_new (
    id          TEXT PRIMARY KEY,
    sku         TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    price_minor INTEGER NOT NULL CHECK (price_minor >= 0),
    currency    TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    category_id TEXT REFERENCES categories(id),
    barcode     TEXT,
    price_updated_at TEXT DEFAULT '',
    track_serial INTEGER NOT NULL DEFAULT 0,
    product_type TEXT NOT NULL DEFAULT 'retail',
    cost_minor  INTEGER NOT NULL DEFAULT 0,
    version     INTEGER NOT NULL DEFAULT 1,
    store_id    TEXT REFERENCES store_profiles(id) ON DELETE SET NULL ON UPDATE CASCADE,
    tenant_id   TEXT NOT NULL DEFAULT 'default',
    kitchen_zone TEXT
);

CREATE TABLE sales_new (
    id                  TEXT PRIMARY KEY,
    total_minor         INTEGER NOT NULL,
    currency            TEXT NOT NULL,
    line_count          INTEGER NOT NULL CHECK (line_count >= 0),
    status              TEXT NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'pending', 'completed', 'voided', 'refunded')),
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    payment_method      TEXT,
    tendered_minor      INTEGER,
    discount_percent    INTEGER NOT NULL DEFAULT 0,
    discount_label      TEXT,
    user_id             TEXT,
    subtotal_minor      INTEGER NOT NULL DEFAULT 0,
    tax_total_minor     INTEGER NOT NULL DEFAULT 0,
    customer_id         TEXT REFERENCES customers(id),
    version             INTEGER NOT NULL DEFAULT 1,
    store_id            TEXT REFERENCES store_profiles(id) ON DELETE SET NULL ON UPDATE CASCADE,
    deduction_locations TEXT,
    pending_expires_at  TEXT,
    payment_reference   TEXT,
    captured_at         TEXT
);

CREATE TABLE sale_lines_new (
    id            TEXT PRIMARY KEY,
    sale_id       TEXT NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
    sku           TEXT NOT NULL,
    qty           INTEGER NOT NULL CHECK (qty > 0),
    unit_minor    INTEGER NOT NULL,
    line_minor    INTEGER NOT NULL,
    currency      TEXT NOT NULL,
    line_position INTEGER NOT NULL,
    tax_minor     INTEGER NOT NULL DEFAULT 0,
    tax_rate_id   TEXT REFERENCES tax_rates(id),
    serial_number TEXT,
    store_id      TEXT REFERENCES store_profiles(id) ON DELETE SET NULL ON UPDATE CASCADE,
    course        TEXT,
    modifiers_json TEXT,
    tax_breakdown_json TEXT,
    UNIQUE (sale_id, line_position)
);

-- ══════════════════════════════════════════════════════════════════
-- 2. Copy data — quarantining orphaned store_ids to NULL
-- ══════════════════════════════════════════════════════════════════

INSERT INTO customers_new (id, name, email, phone, loyalty_points, total_spent_minor, currency, notes, created_at, updated_at, store_id)
SELECT id, name, email, phone, loyalty_points, total_spent_minor, currency, notes, created_at, updated_at,
       CASE WHEN store_id IN (SELECT id FROM store_profiles) THEN store_id END
FROM customers;

INSERT INTO products_new (id, sku, name, price_minor, currency, created_at, updated_at, category_id, barcode, price_updated_at, track_serial, product_type, cost_minor, version, store_id, tenant_id, kitchen_zone)
SELECT id, sku, name, price_minor, currency, created_at, updated_at, category_id, barcode, price_updated_at, track_serial, product_type, cost_minor, version,
       CASE WHEN store_id IN (SELECT id FROM store_profiles) THEN store_id END,
       tenant_id, kitchen_zone
FROM products;

INSERT INTO sales_new (id, total_minor, currency, line_count, status, created_at, updated_at, payment_method, tendered_minor, discount_percent, discount_label, user_id, subtotal_minor, tax_total_minor, customer_id, version, store_id, deduction_locations, pending_expires_at, payment_reference, captured_at)
SELECT id, total_minor, currency, line_count, status, created_at, updated_at, payment_method, tendered_minor, discount_percent, discount_label, user_id, subtotal_minor, tax_total_minor, customer_id, version,
       CASE WHEN store_id IN (SELECT id FROM store_profiles) THEN store_id END,
       deduction_locations, pending_expires_at, payment_reference, captured_at
FROM sales;

INSERT INTO sale_lines_new (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position, tax_minor, tax_rate_id, serial_number, store_id, course, modifiers_json, tax_breakdown_json)
SELECT id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position, tax_minor, tax_rate_id, serial_number,
       CASE WHEN store_id IN (SELECT id FROM store_profiles) THEN store_id END,
       course, modifiers_json, tax_breakdown_json
FROM sale_lines;

-- ══════════════════════════════════════════════════════════════════
-- 3. Drop old tables (leaf tables first; FK enforcement is OFF here,
--    courtesy of the runner's DB-05 isolation, so the drops are safe
--    even though sales/products are referenced by many child tables)
-- ══════════════════════════════════════════════════════════════════

DROP TABLE sale_lines;
DROP TABLE sales;
DROP TABLE products;
DROP TABLE customers;

-- ══════════════════════════════════════════════════════════════════
-- 4. Rename new tables to final names. SQLite's RENAME automatically
--    rewrites FK references in other tables to point at the renamed
--    table, so the child-table FKs (inventory → products, sale_lines →
--    sales, etc.) keep working.
-- ══════════════════════════════════════════════════════════════════

ALTER TABLE sale_lines_new RENAME TO sale_lines;
ALTER TABLE sales_new RENAME TO sales;
ALTER TABLE products_new RENAME TO products;
ALTER TABLE customers_new RENAME TO customers;

-- ══════════════════════════════════════════════════════════════════
-- 5. Re-create indexes (all pre-existing indexes + the ADR #4 scoping
--    indexes from migration 069)
-- ══════════════════════════════════════════════════════════════════

-- products
CREATE INDEX IF NOT EXISTS idx_products_sku ON products(sku);
CREATE INDEX IF NOT EXISTS idx_products_category_id ON products(category_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_products_barcode ON products(barcode);
CREATE INDEX IF NOT EXISTS idx_products_store_category ON products(store_id, category_id);
CREATE INDEX IF NOT EXISTS idx_products_tenant ON products(tenant_id);

-- sales
CREATE INDEX IF NOT EXISTS idx_sales_created_at ON sales(created_at);
CREATE INDEX IF NOT EXISTS idx_sales_store_status ON sales(store_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sales_pending_expires ON sales(pending_expires_at) WHERE status = 'pending';

-- sale_lines
CREATE INDEX IF NOT EXISTS idx_sale_lines_sale_id ON sale_lines(sale_id);
CREATE INDEX IF NOT EXISTS idx_sale_lines_sku ON sale_lines(sku);
CREATE INDEX IF NOT EXISTS idx_sale_lines_store_sale ON sale_lines(store_id, sale_id);

-- customers
CREATE INDEX IF NOT EXISTS idx_customers_name ON customers(name);
CREATE INDEX IF NOT EXISTS idx_customers_email ON customers(email);
CREATE INDEX IF NOT EXISTS idx_customers_phone ON customers(phone);
CREATE INDEX IF NOT EXISTS idx_customers_store ON customers(store_id);

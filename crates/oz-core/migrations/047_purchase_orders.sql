CREATE TABLE IF NOT EXISTS purchase_orders (
    id              TEXT PRIMARY KEY NOT NULL,
    po_number       TEXT NOT NULL,
    supplier_id     TEXT NOT NULL REFERENCES suppliers(id),
    status          TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'pending', 'approved', 'received', 'cancelled')),
    order_date      TEXT NOT NULL,
    expected_date   TEXT NOT NULL DEFAULT '',
    received_date   TEXT,
    subtotal_minor  INTEGER NOT NULL DEFAULT 0,
    tax_minor       INTEGER NOT NULL DEFAULT 0,
    total_minor     INTEGER NOT NULL DEFAULT 0,
    notes           TEXT NOT NULL DEFAULT '',
    created_by      TEXT REFERENCES users(id),
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_purchase_orders_po_number ON purchase_orders(po_number);

CREATE TABLE IF NOT EXISTS purchase_order_lines (
    id                TEXT PRIMARY KEY NOT NULL,
    po_id             TEXT NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    sku               TEXT NOT NULL DEFAULT '',
    product_name      TEXT NOT NULL DEFAULT '',
    qty               INTEGER NOT NULL DEFAULT 0,
    unit_cost_minor   INTEGER NOT NULL DEFAULT 0,
    line_total_minor  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_po_lines_po_id ON purchase_order_lines(po_id);

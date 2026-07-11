CREATE TABLE IF NOT EXISTS suppliers (
    id          TEXT PRIMARY KEY NOT NULL,
    code        TEXT NOT NULL,
    name        TEXT NOT NULL,
    contact_person TEXT NOT NULL DEFAULT '',
    phone       TEXT NOT NULL DEFAULT '',
    email       TEXT NOT NULL DEFAULT '',
    address     TEXT NOT NULL DEFAULT '',
    tax_id      TEXT NOT NULL DEFAULT '',
    payment_terms TEXT NOT NULL DEFAULT '',
    notes       TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'inactive')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_suppliers_code ON suppliers(code);

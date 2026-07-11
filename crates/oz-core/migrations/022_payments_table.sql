-- Create a separate payments table to support split payments
-- (multiple payment methods per sale).

CREATE TABLE IF NOT EXISTS payments (
    id          TEXT PRIMARY KEY,
    sale_id     TEXT NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
    method      TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    currency    TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_payments_sale_id ON payments(sale_id);

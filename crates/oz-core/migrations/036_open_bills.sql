-- 036_open_bills.sql
-- Extend held_carts to support "Open Bill" (customer tab without payment).
-- bill_type = 'hold' | 'open_bill'
-- customer_name is set when bill_type = 'open_bill'

ALTER TABLE held_carts ADD COLUMN bill_type TEXT NOT NULL DEFAULT 'hold';
ALTER TABLE held_carts ADD COLUMN customer_name TEXT;

CREATE INDEX IF NOT EXISTS idx_held_carts_bill_type ON held_carts(bill_type);

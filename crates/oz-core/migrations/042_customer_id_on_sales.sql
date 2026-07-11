ALTER TABLE sales ADD COLUMN customer_id TEXT REFERENCES customers(id);

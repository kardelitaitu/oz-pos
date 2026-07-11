-- OZ-POS Discount Support
--
-- Adds cart-level discount fields to the sales table.
-- discount_percent: percentage (0-100), 0 means no discount.
-- discount_label: human-readable label (e.g. "Senior 10%", "VIP 15%").

ALTER TABLE sales ADD COLUMN discount_percent INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sales ADD COLUMN discount_label TEXT;
